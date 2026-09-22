//! A conflicted file as data: git's markers parsed byte-exactly, a side
//! taken per block, the result written back without clobbering, the three
//! index stages read, and git's own `diff3` output reproduced so a block can
//! be given its base (m6-conflicts/01).
//!
//! What a marker is, decided once here (the design note's G1):
//!
//! - **Exactly seven** marker characters at column 0, then a space and a
//!   label: `<<<<<<< `, `||||||| `, `>>>>>>> `; or `=======` alone, which is
//!   the label-less form git writes. The line's terminator (`\n` or `\r\n`)
//!   is never part of the label and is kept on the block, so `render` gives
//!   the bytes back exactly — CRLF, mixed endings, a missing final newline.
//!   A marker of any other size (`conflict-marker-size` ≠ 7) is not a marker:
//!   the file parses as plain text with no blocks.
//! - git writes **one** separator per block and never escapes content, so the
//!   first `=======` after `<<<<<<<` is the separator and a `=======` after it
//!   (a Markdown setext underline, say) is theirs' content. The same holds
//!   for a second `|||||||`. Checked against `git merge-file` on 2026-09-22:
//!   it prints the underline verbatim inside theirs' side and `checkout
//!   --theirs` gives it back as content. The one shape this cannot tell apart
//!   is a `=======` line in *ours'* content: it is read as the separator.
//! - A `<<<<<<<` inside an open block is a [`ParseErrorKind::StrayMarker`]
//!   (nested markers mean a botched resolution was committed), as is a
//!   `=======`, `|||||||` or `>>>>>>>` outside any block, or a `>>>>>>>`
//!   before the separator. A block still open at the end of the file is
//!   [`ParseErrorKind::Unterminated`], reported at its `<<<<<<<` line.
//! - `zdiff3` is `diff3` to the parser: a `|||||||` section is a base.
//!
//! What git does at the edges, measured on git 2.54 (2026-09-22): a side that
//! lacks a final newline gets one added *inside* the block, so a conflict
//! file ends in a partial line only when its last plain line does; a file
//! whose lines end in CRLF gets CRLF-terminated markers.

use std::fmt;
use std::io::{self, Read, Write};
use std::path::Path;

use graft_core::oid::Oid;
use sha1::{Digest, Sha1};

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError};

/// One line of the file, its terminator included when it has one, so that
/// `render` is byte-exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line(pub Vec<u8>);

/// How a marker line ended: nothing (the last line of the file), `\n`, or
/// `\r\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminator {
    /// The file ended right after the marker.
    None,
    /// `\n`.
    Lf,
    /// `\r\n`.
    CrLf,
}

impl Terminator {
    /// The bytes this terminator renders as.
    #[must_use]
    pub fn as_bytes(self) -> &'static [u8] {
        match self {
            Terminator::None => b"",
            Terminator::Lf => b"\n",
            Terminator::CrLf => b"\r\n",
        }
    }

    /// Splits a raw line into its content and its terminator.
    fn split(raw: &[u8]) -> (&[u8], Terminator) {
        match raw {
            [content @ .., b'\r', b'\n'] => (content, Terminator::CrLf),
            [content @ .., b'\n'] => (content, Terminator::Lf),
            content => (content, Terminator::None),
        }
    }
}

/// The terminator each of a block's marker lines was written with, so that
/// a block renders byte-for-byte as git wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkerEnds {
    /// After `<<<<<<< label`.
    pub start: Terminator,
    /// After `||||||| label`; meaningless when the block has no base.
    pub base: Terminator,
    /// After `=======`.
    pub separator: Terminator,
    /// After `>>>>>>> label`.
    pub end: Terminator,
}

impl MarkerEnds {
    /// Every marker ended by `\n`: what a hand-built block uses.
    pub const LF: MarkerEnds = MarkerEnds {
        start: Terminator::Lf,
        base: Terminator::Lf,
        separator: Terminator::Lf,
        end: Terminator::Lf,
    };
}

/// One conflict block: everything between `<<<<<<<` and `>>>>>>>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The label after `<<<<<<< ` (`HEAD`, a branch, `3f2a1b (subject)`).
    pub ours_label: Vec<u8>,
    /// Our side's lines.
    pub ours: Vec<Line>,
    /// The base label and lines, when the file was written in `diff3` or
    /// `zdiff3` style or [`attach_base`] reproduced them.
    pub base: Option<(Vec<u8>, Vec<Line>)>,
    /// The label after `>>>>>>> `.
    pub theirs_label: Vec<u8>,
    /// Their side's lines.
    pub theirs: Vec<Line>,
    /// How each marker line ended.
    pub ends: MarkerEnds,
}

impl Block {
    /// The lines a take leaves behind.
    fn taken(&self, take: Take) -> impl Iterator<Item = &Line> {
        let (ours, theirs) = match take {
            Take::Ours => (&self.ours[..], &[][..]),
            Take::Theirs => (&[][..], &self.theirs[..]),
            Take::Both => (&self.ours[..], &self.theirs[..]),
        };
        ours.iter().chain(theirs)
    }
}

/// Which side a block keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// Our lines only.
    Ours,
    /// Their lines only.
    Theirs,
    /// Ours then theirs — git's `smerge` order.
    Both,
}

/// One run of the file: plain lines, an open conflict, or a block a take has
/// collapsed to its chosen lines (the block is kept so the take can change).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Region {
    /// Lines outside any block.
    Plain(Vec<Line>),
    /// A block still open.
    Conflict(Block),
    /// A block that renders as the taken lines only.
    Resolved {
        /// The block as parsed, intact.
        block: Block,
        /// What it renders as.
        take: Take,
    },
}

impl Region {
    /// How many lines the region renders.
    fn line_count(&self) -> usize {
        match self {
            Region::Plain(lines) => lines.len(),
            Region::Conflict(block) => {
                let base = block.base.as_ref().map_or(0, |(_, lines)| lines.len() + 1);
                3 + block.ours.len() + base + block.theirs.len()
            }
            Region::Resolved { block, take } => block.taken(*take).count(),
        }
    }
}

/// A parsed conflict file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictFile {
    /// The file, in order.
    pub regions: Vec<Region>,
    /// git's blob id of the bytes as parsed (see [`blob_id`]).
    pub fingerprint: Oid,
}

/// Where one block is and what state it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockAt {
    /// The block's index, counting `Conflict` and `Resolved` regions in order.
    pub index: usize,
    /// Open, or taken.
    pub state: BlockState,
    /// The 1-based line the block starts on in `render()` — its `<<<<<<<`
    /// line when open, its first taken line once resolved (or the line the
    /// next region starts on, when the take kept nothing).
    pub first_line: usize,
}

/// Whether a block has been taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockState {
    /// Still a conflict.
    Open,
    /// Taken, this way.
    Took(Take),
}

/// Why bytes are not a conflict file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The 1-based line the error is at.
    pub line: usize,
    /// What is wrong there.
    pub what: ParseErrorKind,
}

/// The two ways a marker file can be malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A block's `<<<<<<<` (the line reported) has no `>>>>>>>` before the
    /// end of the file.
    Unterminated,
    /// A marker where none can be: `=======`, `|||||||` or `>>>>>>>` outside
    /// a block, `<<<<<<<` inside one, or `>>>>>>>` before the separator.
    StrayMarker,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.what {
            ParseErrorKind::Unterminated => {
                write!(f, "line {}: conflict block is never closed", self.line)
            }
            ParseErrorKind::StrayMarker => {
                write!(f, "line {}: conflict marker out of place", self.line)
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// A block index past the last block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoSuchBlock(pub usize);

impl fmt::Display for NoSuchBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no conflict block {}", self.0)
    }
}

impl std::error::Error for NoSuchBlock {}

/// What one line is to the parser.
#[derive(Clone, Copy)]
enum Marker<'a> {
    Start(&'a [u8]),
    Base(&'a [u8]),
    Separator,
    End(&'a [u8]),
}

impl<'a> Marker<'a> {
    /// Classifies a line's content (terminator already split off).
    fn of(content: &'a [u8]) -> Option<Self> {
        if content == b"=======" {
            return Some(Marker::Separator);
        }
        if let Some(label) = content.strip_prefix(b"<<<<<<< ") {
            return Some(Marker::Start(label));
        }
        if let Some(label) = content.strip_prefix(b"||||||| ") {
            return Some(Marker::Base(label));
        }
        if let Some(label) = content.strip_prefix(b">>>>>>> ") {
            return Some(Marker::End(label));
        }
        None
    }
}

/// Which part of a block the parser is inside.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Ours,
    Base,
    Theirs,
}

/// A block being read.
struct Open {
    block: Block,
    section: Section,
    /// The 1-based line of its `<<<<<<<`, for `Unterminated`.
    started_at: usize,
}

/// The parser's state between lines.
#[derive(Default)]
struct Parser {
    regions: Vec<Region>,
    /// Plain lines not yet closed into a region.
    plain: Vec<Line>,
    open: Option<Open>,
}

impl Parser {
    /// Reads one raw line (terminator included) at 1-based `number`.
    fn line(&mut self, number: usize, raw: &[u8]) -> Result<(), ParseError> {
        let (content, end) = Terminator::split(raw);
        let marker = Marker::of(content);
        let stray = ParseError {
            line: number,
            what: ParseErrorKind::StrayMarker,
        };
        let Some(current) = self.open.as_mut() else {
            return self.outside(number, raw, marker, end, stray);
        };
        match (current.section, marker) {
            (_, Some(Marker::Start(_))) | (Section::Ours | Section::Base, Some(Marker::End(_))) => {
                Err(stray)
            }
            (Section::Ours, Some(Marker::Base(label))) => {
                current.block.base = Some((label.to_vec(), Vec::new()));
                current.block.ends.base = end;
                current.section = Section::Base;
                Ok(())
            }
            (Section::Ours | Section::Base, Some(Marker::Separator)) => {
                current.block.ends.separator = end;
                current.section = Section::Theirs;
                Ok(())
            }
            (Section::Theirs, Some(Marker::End(label))) => {
                let Some(mut done) = self.open.take() else {
                    unreachable!("a block is open on this branch")
                };
                done.block.theirs_label = label.to_vec();
                done.block.ends.end = end;
                self.regions.push(Region::Conflict(done.block));
                Ok(())
            }
            // A second `|||||||` or `=======`, or a `|||||||` after the
            // separator, is content: git writes one of each and never
            // escapes what it copies.
            (Section::Ours, None) => {
                current.block.ours.push(Line(raw.to_vec()));
                Ok(())
            }
            (Section::Base, None | Some(Marker::Base(_))) => {
                if let Some((_, lines)) = current.block.base.as_mut() {
                    lines.push(Line(raw.to_vec()));
                }
                Ok(())
            }
            (Section::Theirs, None | Some(Marker::Base(_) | Marker::Separator)) => {
                current.block.theirs.push(Line(raw.to_vec()));
                Ok(())
            }
        }
    }

    /// A line with no block open: plain text, or the start of one.
    fn outside(
        &mut self,
        number: usize,
        raw: &[u8],
        marker: Option<Marker<'_>>,
        end: Terminator,
        stray: ParseError,
    ) -> Result<(), ParseError> {
        match marker {
            Some(Marker::Start(label)) => {
                if !self.plain.is_empty() {
                    self.regions
                        .push(Region::Plain(std::mem::take(&mut self.plain)));
                }
                self.open = Some(Open {
                    block: Block {
                        ours_label: label.to_vec(),
                        ours: Vec::new(),
                        base: None,
                        theirs_label: Vec::new(),
                        theirs: Vec::new(),
                        ends: MarkerEnds {
                            start: end,
                            base: Terminator::None,
                            separator: Terminator::None,
                            end: Terminator::None,
                        },
                    },
                    section: Section::Ours,
                    started_at: number,
                });
                Ok(())
            }
            Some(_) => Err(stray),
            None => {
                self.plain.push(Line(raw.to_vec()));
                Ok(())
            }
        }
    }

    /// The end of the input.
    fn finish(mut self) -> Result<Vec<Region>, ParseError> {
        if let Some(unfinished) = self.open {
            return Err(ParseError {
                line: unfinished.started_at,
                what: ParseErrorKind::Unterminated,
            });
        }
        if !self.plain.is_empty() {
            self.regions.push(Region::Plain(self.plain));
        }
        Ok(self.regions)
    }
}

/// Parses a working-tree file with git's conflict markers in it.
///
/// Accepts a file with no markers at all (one `Plain` region, or none for an
/// empty file). `render` gives back exactly `bytes` for every input this
/// accepts.
pub fn parse(bytes: &[u8]) -> Result<ConflictFile, ParseError> {
    let fingerprint = blob_id(bytes);
    let mut parser = Parser::default();
    for (index, raw) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
        parser.line(index + 1, raw)?;
    }
    Ok(ConflictFile {
        regions: parser.finish()?,
        fingerprint,
    })
}

/// git's blob id of `bytes`: `sha1("blob <len>\0" + bytes)`, computed here
/// with no git process, so a fingerprint and git's own index agree on what a
/// file's content is called (SHA-1 repositories; a SHA-256 repository's index
/// names blobs differently, but every comparison Graft makes is between two
/// of these).
///
/// Plain SHA-1, without gix's collision-attack detection: the one input that
/// detection rejects is one git refuses to hash at all, so there is no id to
/// disagree with.
#[must_use]
pub fn blob_id(bytes: &[u8]) -> Oid {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    Oid::from_sha1(hasher.finalize().into())
}

impl ConflictFile {
    /// The file's bytes: plain lines as they are, open blocks with their
    /// markers, resolved blocks as their taken lines.
    #[must_use]
    pub fn render(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for region in &self.regions {
            match region {
                Region::Plain(lines) => {
                    for line in lines {
                        out.extend_from_slice(&line.0);
                    }
                }
                Region::Conflict(block) => {
                    out.extend_from_slice(b"<<<<<<< ");
                    out.extend_from_slice(&block.ours_label);
                    out.extend_from_slice(block.ends.start.as_bytes());
                    for line in &block.ours {
                        out.extend_from_slice(&line.0);
                    }
                    if let Some((label, lines)) = &block.base {
                        out.extend_from_slice(b"||||||| ");
                        out.extend_from_slice(label);
                        out.extend_from_slice(block.ends.base.as_bytes());
                        for line in lines {
                            out.extend_from_slice(&line.0);
                        }
                    }
                    out.extend_from_slice(b"=======");
                    out.extend_from_slice(block.ends.separator.as_bytes());
                    for line in &block.theirs {
                        out.extend_from_slice(&line.0);
                    }
                    out.extend_from_slice(b">>>>>>> ");
                    out.extend_from_slice(&block.theirs_label);
                    out.extend_from_slice(block.ends.end.as_bytes());
                }
                Region::Resolved { block, take } => {
                    for line in block.taken(*take) {
                        out.extend_from_slice(&line.0);
                    }
                }
            }
        }
        out
    }

    /// Takes a side for the `block`-th block (counting `Conflict` and
    /// `Resolved` regions in order). Re-taking a resolved block replaces its
    /// take.
    pub fn take(&mut self, block: usize, take: Take) -> Result<(), NoSuchBlock> {
        let region = self
            .regions
            .iter_mut()
            .filter(|region| !matches!(region, Region::Plain(_)))
            .nth(block)
            .ok_or(NoSuchBlock(block))?;
        // A momentary placeholder so the block can move out of the old
        // region and into the new one without a clone.
        let previous = std::mem::replace(region, Region::Plain(Vec::new()));
        let inner = match previous {
            Region::Conflict(inner) | Region::Resolved { block: inner, .. } => inner,
            Region::Plain(_) => unreachable!("plain regions were filtered out"),
        };
        *region = Region::Resolved { block: inner, take };
        Ok(())
    }

    /// Every block, open or taken, with the line it starts on.
    #[must_use]
    pub fn blocks(&self) -> Vec<BlockAt> {
        let mut found = Vec::new();
        let mut line = 1usize;
        for region in &self.regions {
            let state = match region {
                Region::Plain(_) => None,
                Region::Conflict(_) => Some(BlockState::Open),
                Region::Resolved { take, .. } => Some(BlockState::Took(*take)),
            };
            if let Some(state) = state {
                found.push(BlockAt {
                    index: found.len(),
                    state,
                    first_line: line,
                });
            }
            line += region.line_count();
        }
        found
    }

    /// How many blocks are still open.
    #[must_use]
    pub fn open(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| matches!(region, Region::Conflict(_)))
            .count()
    }

    /// The blocks in order, whatever their state.
    fn each_block(&self) -> impl Iterator<Item = &Block> {
        self.regions.iter().filter_map(|region| match region {
            Region::Plain(_) => None,
            Region::Conflict(block) | Region::Resolved { block, .. } => Some(block),
        })
    }
}

/// git's own rule for a binary file: a NUL in the first 8 KiB.
const BINARY_PROBE: usize = 8 * 1024;

/// The largest file `read` will parse.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Why a file could not be read as a conflict file.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The file could not be opened or read.
    #[error("could not read the file: {0}")]
    Io(#[from] io::Error),
    /// A NUL in the first 8 KiB: git calls it binary and writes no markers.
    #[error("the file is binary")]
    Binary,
    /// Larger than the resolver handles.
    #[error("the file is {bytes} bytes; the resolver handles up to {MAX_BYTES}")]
    TooLarge {
        /// The file's size.
        bytes: u64,
    },
    /// The bytes are not a well-formed marker file.
    #[error("{0}")]
    Parse(#[from] ParseError),
}

/// Reads and parses a working-tree file.
pub fn read(path: &Path) -> Result<ConflictFile, ReadError> {
    let mut file = std::fs::File::open(path)?;
    let bytes = file.metadata()?.len();
    if bytes > MAX_BYTES {
        return Err(ReadError::TooLarge { bytes });
    }
    // A capacity hint only: `bytes` is at most 8 MiB here, so the conversion
    // cannot fail on any target this builds for, and a 0 would merely grow.
    let mut content = Vec::with_capacity(usize::try_from(bytes).unwrap_or(0));
    file.read_to_end(&mut content)?;
    let probe = &content[..content.len().min(BINARY_PROBE)];
    if probe.contains(&0) {
        return Err(ReadError::Binary);
    }
    Ok(parse(&content)?)
}

/// Why a file could not be written back.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The file or its temporary could not be written or renamed.
    #[error("could not write the file: {0}")]
    Io(#[from] io::Error),
    /// The file's bytes are no longer the ones the caller parsed.
    #[error("the file changed on disk since it was read")]
    Changed {
        /// The blob id of what is there now.
        now: Oid,
    },
}

/// Writes `file`'s rendering over `path`, refusing when the file's current
/// blob id is not `expected`. The bytes go to a temporary in the same
/// directory with the file's mode bits, then `rename` over the path, so a
/// crash mid-write leaves the old file whole. Returns the new fingerprint.
pub fn write_back(path: &Path, file: &ConflictFile, expected: Oid) -> Result<Oid, WriteError> {
    let current = std::fs::read(path)?;
    let now = blob_id(&current);
    if now != expected {
        return Err(WriteError::Changed { now });
    }
    let mode = std::fs::metadata(path)?.permissions();
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path has no parent directory",
        )
    })?;
    let rendered = file.render();
    let mut temporary = tempfile::Builder::new()
        .prefix(".graft-")
        .tempfile_in(directory)?;
    temporary.write_all(&rendered)?;
    temporary.as_file().sync_all()?;
    temporary.as_file().set_permissions(mode)?;
    temporary.persist(path).map_err(|failed| failed.error)?;
    Ok(blob_id(&rendered))
}

/// One index stage of a path: its mode and blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stage {
    mode: u32,
    blob: Oid,
}

/// The three stages of an unmerged path, `git ls-files -u`; `None` where a
/// stage is missing (a whole-file shape: added or deleted on one side).
fn stages(repo: &RepoHandle, path: &Path) -> Result<[Option<Stage>; 3], RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("--literal-pathspecs"),
            GitArg::Flag("ls-files"),
            GitArg::Flag("-u"),
            GitArg::Flag("-z"),
            GitArg::Separator,
            GitArg::Path(path_bytes(path)),
        ])
        .run(&())?;
    let mut found = [None; 3];
    for record in output.stdout.split(|&b| b == 0) {
        if record.is_empty() {
            continue;
        }
        let malformed = || RunError::Unparsable(format!("ls-files -u record {record:?}"));
        let tab = record
            .iter()
            .position(|&b| b == b'\t')
            .ok_or_else(malformed)?;
        let header = std::str::from_utf8(&record[..tab]).map_err(|_| malformed())?;
        let mut fields = header.split(' ');
        let (Some(mode), Some(blob), Some(stage), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(malformed());
        };
        let mode = u32::from_str_radix(mode, 8).map_err(|_| malformed())?;
        let blob = Oid::from_hex(blob).map_err(|_| malformed())?;
        let slot = match stage {
            "1" => 0,
            "2" => 1,
            "3" => 2,
            _ => return Err(malformed()),
        };
        found[slot] = Some(Stage { mode, blob });
    }
    Ok(found)
}

/// One stage's content, `git show :N:<path>` in effect: the blob `ls-files
/// -u` names for that stage. `Ok(None)` when the stage is missing — the
/// path was added or deleted on that side — rather than a git error to
/// parse.
pub fn stage_bytes(repo: &RepoHandle, path: &Path, stage: u8) -> Result<Option<Vec<u8>>, RunError> {
    let slot = match stage {
        1..=3 => usize::from(stage - 1),
        other => {
            return Err(RunError::Unparsable(format!(
                "stage {other} is not one of 1, 2, 3"
            )));
        }
    };
    let Some(found) = stages(repo, path)?[slot] else {
        return Ok(None);
    };
    blob_bytes(repo, found.blob).map(Some)
}

/// A blob's content, `git cat-file blob <id>`.
fn blob_bytes(repo: &RepoHandle, blob: Oid) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("blob"),
            GitArg::Oid(blob),
        ])
        .run(&())?;
    Ok(output.stdout)
}

/// Why git's `diff3` blocks could not be reproduced.
#[derive(Debug, thiserror::Error)]
pub enum Diff3Error {
    /// A stage could not be read.
    #[error(transparent)]
    Run(#[from] RunError),
    /// The scratch copies of the stages could not be written.
    #[error("could not write the stages to a scratch directory: {0}")]
    Io(#[from] io::Error),
    /// `merge-file` itself failed (its exit code past the conflict count).
    #[error("git merge-file failed: {stderr_first_line}")]
    MergeFile {
        /// The exit code; `None` when git was killed.
        status: Option<i32>,
        /// What it said.
        stderr_first_line: String,
    },
    /// `merge-file`'s output did not parse — a marker rule this module has
    /// wrong, not a repository state.
    #[error("git merge-file printed something this parser cannot read: {0}")]
    Parse(#[from] ParseError),
}

/// `merge-file` exits with the conflict count, capped here; above it, or
/// killed, something went wrong.
const MERGE_FILE_MAX_CONFLICTS: i32 = 127;

/// git's own conflict file for the path, base included: the three stages
/// written to a scratch directory and merged with `git merge-file -p
/// --diff3`, whose output is what git would have written to the working
/// tree under the `diff3` conflict style — same engine, same hunks. `None`
/// when a stage is missing, since a whole-file shape has no blocks.
///
/// The scratch directory is a `tempfile` directory, not one under `.git/`:
/// nothing is left in the user's repository if this process dies mid-way,
/// and a read-only checkout (the sealed test corpus among them) works too.
/// The labels on the returned blocks are `ours`, `base`, `theirs`;
/// [`attach_base`] compares content, never labels.
pub fn diff3_file(repo: &RepoHandle, path: &Path) -> Result<Option<ConflictFile>, Diff3Error> {
    let [Some(base), Some(ours), Some(theirs)] = stages(repo, path)? else {
        return Ok(None);
    };
    let scratch = tempfile::Builder::new().prefix("graft-diff3-").tempdir()?;
    for (name, stage) in [("ours", ours), ("base", base), ("theirs", theirs)] {
        std::fs::write(scratch.path().join(name), blob_bytes(repo, stage.blob)?)?;
    }
    let raw = GitInvocation::in_directory(scratch.path())
        .args([
            GitArg::Flag("merge-file"),
            GitArg::Flag("-p"),
            GitArg::Flag("--diff3"),
            GitArg::Flag("-L"),
            GitArg::Flag("ours"),
            GitArg::Flag("-L"),
            GitArg::Flag("base"),
            GitArg::Flag("-L"),
            GitArg::Flag("theirs"),
            GitArg::Flag("ours"),
            GitArg::Flag("base"),
            GitArg::Flag("theirs"),
        ])
        .run_raw(&())?;
    match raw.code {
        Some(conflicts) if (0..=MERGE_FILE_MAX_CONFLICTS).contains(&conflicts) => {}
        status => {
            return Err(Diff3Error::MergeFile {
                status,
                stderr_first_line: first_line(&raw.output.stderr),
            });
        }
    }
    Ok(Some(parse(&raw.output.stdout)?))
}

/// The first non-empty line of some git stderr, lossily.
fn first_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    match text.lines().map(str::trim).find(|line| !line.is_empty()) {
        Some(line) => line.to_owned(),
        // git said nothing: not a failure to read, just nothing to show.
        None => String::new(),
    }
}

/// A block whose `(ours, theirs)` lines no reproduced block has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseMismatch {
    /// The first block (by [`ConflictFile::blocks`] index) with no match.
    pub block: usize,
}

impl fmt::Display for BaseMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "block {} no longer matches what git produced; base unavailable after edits",
            self.block
        )
    }
}

impl std::error::Error for BaseMismatch {}

/// Which side of a block a run is read on.
#[derive(Clone, Copy)]
enum Side {
    Ours,
    Theirs,
}

/// The lines a run of reproduced regions has on one side: plain lines as
/// they are, each block's lines for that side.
fn side_lines(run: &[Region], side: Side) -> impl Iterator<Item = &Line> {
    run.iter().flat_map(move |region| match region {
        Region::Plain(lines) => lines.iter(),
        Region::Conflict(block) | Region::Resolved { block, .. } => match side {
            Side::Ours => block.ours.iter(),
            Side::Theirs => block.theirs.iter(),
        },
    })
}

/// What one file block was matched to: the base lines of its run, and the
/// base label and marker ending of the run's first block. `None` when a
/// block in the run carries no base — `merge-file --diff3` always writes
/// one, so this is defensive, not expected.
type RunBase = Option<(Vec<u8>, Vec<Line>, Terminator)>;

fn run_base(run: &[Region]) -> RunBase {
    let mut lines = Vec::new();
    let mut first: Option<(Vec<u8>, Terminator)> = None;
    for region in run {
        match region {
            Region::Plain(plain) => lines.extend(plain.iter().cloned()),
            Region::Conflict(block) | Region::Resolved { block, .. } => {
                let (label, base) = block.base.as_ref()?;
                lines.extend(base.iter().cloned());
                first.get_or_insert_with(|| (label.clone(), block.ends.base));
            }
        }
    }
    let (label, ending) = first?;
    Some((label, lines, ending))
}

/// The shortest run of reproduced regions, starting at or after conflict
/// number `cursor` and ending on a conflict, whose two sides are exactly
/// `block`'s. Returns its base and the conflict number after it.
fn find_run(
    reproduced: &[Region],
    conflicts: &[usize],
    cursor: usize,
    block: &Block,
) -> Option<(RunBase, usize)> {
    for (j, &from) in conflicts.iter().enumerate().skip(cursor) {
        for (k, &to) in conflicts.iter().enumerate().skip(j) {
            let run = &reproduced[from..=to];
            // Both sides only grow with `k`: past the block's length, stop.
            if side_lines(run, Side::Ours).count() > block.ours.len()
                || side_lines(run, Side::Theirs).count() > block.theirs.len()
            {
                break;
            }
            if side_lines(run, Side::Ours).eq(block.ours.iter())
                && side_lines(run, Side::Theirs).eq(block.theirs.iter())
            {
                return Some((run_base(run), k + 1));
            }
        }
    }
    None
}

/// Gives each of `file`'s blocks the base git's `diff3` output has for it,
/// walking both in order and comparing content only.
///
/// git's default (`merge`) conflict style joins conflicts that fewer than
/// four unchanged lines separate into one block; `diff3` style never joins
/// them (measured on git 2.54, 2026-09-22). So a file block is never
/// assumed to be one reproduced block: it is the shortest run of reproduced
/// regions — a conflict, or several with the plain lines between — whose
/// ours-side lines and theirs-side lines are byte-equal to the block's, and
/// its base is that run's base lines with the plain lines kept in place.
/// A single block matches a run of one.
///
/// A reproduced conflict the file no longer has is skipped — a take written
/// to disk removes its block from the file while the stages, and so `diff3`,
/// still have it — but a file block is never given a base by position: it
/// matches the next run with exactly its lines, or the whole call fails with
/// the first block that matches nothing and attaches nothing. After a hand
/// edit inside a block the answer is a mismatch, not a guess. Returns how
/// many blocks were given a base.
pub fn attach_base(file: &mut ConflictFile, diff3: &ConflictFile) -> Result<usize, BaseMismatch> {
    let conflicts: Vec<usize> = diff3
        .regions
        .iter()
        .enumerate()
        .filter(|(_, region)| !matches!(region, Region::Plain(_)))
        .map(|(at, _)| at)
        .collect();

    // Decide first, attach second: a mismatch on the third block must leave
    // the first two as they were.
    let mut cursor = 0usize;
    let mut found = Vec::new();
    for (index, block) in file.each_block().enumerate() {
        let (base, next) = find_run(&diff3.regions, &conflicts, cursor, block)
            .ok_or(BaseMismatch { block: index })?;
        found.push(base);
        cursor = next;
    }

    let mut given = 0usize;
    let blocks = file.regions.iter_mut().filter_map(|region| match region {
        Region::Plain(_) => None,
        Region::Conflict(block) | Region::Resolved { block, .. } => Some(block),
    });
    for (block, base) in blocks.zip(found) {
        if let Some((label, lines, ending)) = base {
            block.base = Some((label, lines));
            block.ends.base = ending;
            given += 1;
        }
    }
    Ok(given)
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(test)]
mod tests;
