//! A conflicted file as data: git's markers parsed byte-exactly, a side
//! taken per block, the result written back without clobbering, the three
//! index stages read, and git's own `diff3` output reproduced so a block can
//! be given its base (m6-conflicts/01).
//!
//! What a marker is, decided once here (the design note's G1):
//!
//! - **Exactly seven** marker characters at column 0, then either a space and
//!   a label (`<<<<<<< HEAD`) or nothing at all (`<<<<<<<`, the bare form
//!   git's `rerere` writes into a preimage and `xdl_merge` writes whenever a
//!   label is NULL — measured on git 2.54, 2026-09-22). A label is therefore
//!   [`Option`]: `None` renders `<<<<<<<`, `Some(b"")` renders `<<<<<<< `,
//!   and the two stay byte-distinct. `=======` is the separator and never
//!   carries a label. The line's terminator (`\n` or `\r\n`) is never part of
//!   the label and is kept on the block, so `render` gives the bytes back
//!   exactly — CRLF, mixed endings, a missing final newline.
//! - A file git wrote with a bigger `conflict-marker-size` is **refused, not
//!   read as plain text**: a run of **eight or more** `<` at column 0 outside
//!   any block, bare or followed by a space, is
//!   [`ParseErrorKind::MarkerSize`]. That opener is the one line of such a
//!   file that cannot be mistaken for prose, and it always comes first, so
//!   checking it alone catches the file. `=======`-and-longer runs are *not*
//!   checked: a Markdown setext underline is exactly that, and is the very
//!   reason a project sets the attribute. Sizes **below** seven are reachable
//!   too (`conflict-marker-size=3` gives `<<< HEAD`, measured) and are not
//!   caught here — `>>> ` at column 0 is a Python prompt. Reading the
//!   attribute per path is the real fix; see the ticket's NOTES.
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

/// A block's base section: the label on its `|||||||` line and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Base {
    /// The label after `||||||| `; `None` for the bare `|||||||` form.
    pub label: Option<Vec<u8>>,
    /// The base's lines.
    pub lines: Vec<Line>,
}

/// A base [`attach_base`] worked out from the index stages, which the file on
/// disk does not have. **Never rendered** and never counted into a line
/// number: attaching one cannot change what [`ConflictFile::write_back`]
/// would write or what [`BlockAt::first_line`] means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reproduced {
    /// The base lines, and the label git's `--diff3` output carried.
    pub base: Base,
    /// `true` when these lines are the base of a *region* covering several of
    /// the file's blocks, not of this block alone: git's zealous refinement
    /// splits one conflict whose sides share interior lines into several
    /// blocks, and git itself cannot say which base lines belong to which
    /// (that is why `--diff3` refuses to refine). A caller showing this must
    /// say so rather than imply the lines are this block's own.
    pub shared: bool,
}

/// One conflict block: everything between `<<<<<<<` and `>>>>>>>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The label after `<<<<<<< ` (`HEAD`, a branch, `3f2a1b (subject)`);
    /// `None` for the bare `<<<<<<<` form.
    pub ours_label: Option<Vec<u8>>,
    /// Our side's lines.
    pub ours: Vec<Line>,
    /// The base the file on disk has, written in `diff3` or `zdiff3` style.
    /// Part of the file's bytes, so it renders.
    pub base: Option<Base>,
    /// The label after `>>>>>>> `; `None` for the bare `>>>>>>>` form.
    pub theirs_label: Option<Vec<u8>>,
    /// Their side's lines.
    pub theirs: Vec<Line>,
    /// How each marker line ended.
    pub ends: MarkerEnds,
    /// A base [`attach_base`] reproduced; not part of the file's bytes.
    pub reproduced: Option<Reproduced>,
}

impl Block {
    /// The base to show for this block: the one the file itself carries, else
    /// the one [`attach_base`] reproduced. `shared` is `true` only for a
    /// reproduced base covering several blocks.
    #[must_use]
    pub fn shown_base(&self) -> Option<(&Base, bool)> {
        match (&self.base, &self.reproduced) {
            (Some(base), _) => Some((base, false)),
            (None, Some(reproduced)) => Some((&reproduced.base, reproduced.shared)),
            (None, None) => None,
        }
    }

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

/// Which reading of a conflict file a line number counts in: one side's
/// version, or the result being built. Each numbers its lines 1..=n.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    /// Plain lines and every block's ours lines.
    Ours,
    /// Plain lines and every block's theirs lines.
    Theirs,
    /// Plain lines, taken lines, and each open block's ours lines.
    Result,
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
            // `reproduced` is deliberately absent: it is not in the file.
            Region::Conflict(block) => {
                let base = block.base.as_ref().map_or(0, |base| base.lines.len() + 1);
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

/// The ways a marker file can be one this parser will not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A block's `<<<<<<<` (the line reported) has no `>>>>>>>` before the
    /// end of the file.
    Unterminated,
    /// A marker where none can be: `=======`, `|||||||` or `>>>>>>>` outside
    /// a block, `<<<<<<<` inside one, or `>>>>>>>` before the separator.
    StrayMarker,
    /// The line opens a conflict with markers longer than seven characters:
    /// the path's `conflict-marker-size` is not git's default and this parser
    /// cannot read the file. Refusing is the point — the file *is*
    /// conflicted, and calling it plain text would let its markers be staged
    /// and committed with nothing able to object.
    MarkerSize,
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
            ParseErrorKind::MarkerSize => write!(
                f,
                "line {}: this file's conflict markers are longer than seven characters \
                 (conflict-marker-size); Graft cannot resolve it yet",
                self.line
            ),
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

/// What one line is to the parser. A label is `None` for the bare marker
/// form (`<<<<<<<` with nothing after it).
#[derive(Clone, Copy)]
enum Marker<'a> {
    Start(Option<&'a [u8]>),
    Base(Option<&'a [u8]>),
    Separator,
    End(Option<&'a [u8]>),
}

/// How a line reads before the parser's state is consulted.
#[derive(Clone, Copy)]
enum LineKind<'a> {
    /// Not a marker.
    Plain,
    /// A marker of git's default size.
    Marker(Marker<'a>),
    /// A conflict opened with markers longer than seven characters.
    Oversize,
}

/// The marker length this parser reads: git's default `conflict-marker-size`.
const MARKER_SIZE: usize = 7;

impl<'a> Marker<'a> {
    /// Classifies a line's content (terminator already split off).
    fn of(content: &'a [u8]) -> LineKind<'a> {
        if content == b"=======" {
            return LineKind::Marker(Marker::Separator);
        }
        let build: fn(Option<&'a [u8]>) -> Marker<'a> = match content.first() {
            Some(b'<') => Marker::Start,
            Some(b'|') => Marker::Base,
            Some(b'>') => Marker::End,
            _ => return LineKind::Plain,
        };
        let head = content[0];
        let run = content.iter().take_while(|&&byte| byte == head).count();
        // Either a space and a label, or nothing: `<<<<<<< HEAD`, `<<<<<<<`.
        let label = match &content[run..] {
            [] => None,
            [b' ', label @ ..] => Some(label),
            _ => return LineKind::Plain,
        };
        if run == MARKER_SIZE {
            return LineKind::Marker(build(label));
        }
        // Only the opener is checked for an oversize run, and only outside a
        // block (see `Parser::outside`): see the module doc for why `=`, `|`
        // and `>` runs stay content.
        if run > MARKER_SIZE && head == b'<' {
            return LineKind::Oversize;
        }
        LineKind::Plain
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
        let kind = Marker::of(content);
        let stray = ParseError {
            line: number,
            what: ParseErrorKind::StrayMarker,
        };
        let Some(current) = self.open.as_mut() else {
            return self.outside(number, raw, kind, end, stray);
        };
        // Inside a block an oversize `<` run is content: a file this parser
        // got inside of has seven-character markers, so a longer run is a
        // line of a side, not a marker.
        let marker = match kind {
            LineKind::Marker(marker) => Some(marker),
            LineKind::Plain | LineKind::Oversize => None,
        };
        match (current.section, marker) {
            (_, Some(Marker::Start(_))) | (Section::Ours | Section::Base, Some(Marker::End(_))) => {
                Err(stray)
            }
            (Section::Ours, Some(Marker::Base(label))) => {
                current.block.base = Some(Base {
                    label: label.map(<[u8]>::to_vec),
                    lines: Vec::new(),
                });
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
                done.block.theirs_label = label.map(<[u8]>::to_vec);
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
                if let Some(base) = current.block.base.as_mut() {
                    base.lines.push(Line(raw.to_vec()));
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
        kind: LineKind<'_>,
        end: Terminator,
        stray: ParseError,
    ) -> Result<(), ParseError> {
        match kind {
            LineKind::Marker(Marker::Start(label)) => {
                if !self.plain.is_empty() {
                    self.regions
                        .push(Region::Plain(std::mem::take(&mut self.plain)));
                }
                self.open = Some(Open {
                    block: Block {
                        ours_label: label.map(<[u8]>::to_vec),
                        ours: Vec::new(),
                        base: None,
                        theirs_label: None,
                        theirs: Vec::new(),
                        ends: MarkerEnds {
                            start: end,
                            base: Terminator::None,
                            separator: Terminator::None,
                            end: Terminator::None,
                        },
                        reproduced: None,
                    },
                    section: Section::Ours,
                    started_at: number,
                });
                Ok(())
            }
            LineKind::Marker(_) => Err(stray),
            LineKind::Oversize => Err(ParseError {
                line: number,
                what: ParseErrorKind::MarkerSize,
            }),
            LineKind::Plain => {
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

/// Writes a marker and its label, terminator not included: `<<<<<<< HEAD`
/// for `Some(b"HEAD")`, `<<<<<<< ` for `Some(b"")`, `<<<<<<<` for `None`.
fn marker_line(out: &mut Vec<u8>, marker: &[u8], label: Option<&[u8]>) {
    out.extend_from_slice(marker);
    if let Some(label) = label {
        out.push(b' ');
        out.extend_from_slice(label);
    }
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
                // `reproduced` is deliberately absent: it is not in the file,
                // so attaching one cannot change what gets written back.
                Region::Conflict(block) => {
                    marker_line(&mut out, b"<<<<<<<", block.ours_label.as_deref());
                    out.extend_from_slice(block.ends.start.as_bytes());
                    for line in &block.ours {
                        out.extend_from_slice(&line.0);
                    }
                    if let Some(base) = &block.base {
                        marker_line(&mut out, b"|||||||", base.label.as_deref());
                        out.extend_from_slice(block.ends.base.as_bytes());
                        for line in &base.lines {
                            out.extend_from_slice(&line.0);
                        }
                    }
                    out.extend_from_slice(b"=======");
                    out.extend_from_slice(block.ends.separator.as_bytes());
                    for line in &block.theirs {
                        out.extend_from_slice(&line.0);
                    }
                    marker_line(&mut out, b">>>>>>>", block.theirs_label.as_deref());
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

    /// One entry per region: the 1-based line that region starts on when the
    /// file is read as projection `p`. Every projection numbers its own lines
    /// contiguously from 1, so the entries never skip a number.
    ///
    /// - [`Projection::Ours`] / [`Projection::Theirs`]: plain lines, plus
    ///   that side's lines of every block, open or taken.
    /// - [`Projection::Result`]: plain lines, a taken block's taken lines,
    ///   and an open block's ours lines (drawn as a placeholder).
    ///
    /// An empty region gets the number the next line would have.
    #[must_use]
    pub fn first_lines(&self, p: Projection) -> Vec<usize> {
        let mut line = 1usize;
        self.regions
            .iter()
            .map(|region| {
                let first = line;
                line += match (region, p) {
                    (Region::Plain(lines), _) => lines.len(),
                    // An open block's result is its ours lines (a placeholder).
                    (
                        Region::Conflict(block) | Region::Resolved { block, .. },
                        Projection::Ours,
                    )
                    | (Region::Conflict(block), Projection::Result) => block.ours.len(),
                    (
                        Region::Conflict(block) | Region::Resolved { block, .. },
                        Projection::Theirs,
                    ) => block.theirs.len(),
                    (Region::Resolved { block, take }, Projection::Result) => {
                        block.taken(*take).count()
                    }
                };
                first
            })
            .collect()
    }

    /// How many blocks are still open.
    #[must_use]
    pub fn open(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| matches!(region, Region::Conflict(_)))
            .count()
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

/// One stage's content **raw**, as the object database holds it: `git show
/// :N:<path>` in effect, the blob `ls-files -u` names for that stage, with no
/// `.gitattributes` conversion applied. That is what a whole-file view wants
/// — the committed bytes — and it is *not* what the working tree holds under
/// `eol=crlf`, `core.autocrlf` or a smudge filter; [`diff3_file`] uses the
/// converted form ([`filtered_bytes`]) for that reason.
///
/// `Ok(None)` when the stage is missing — the path was added or deleted on
/// that side — rather than a git error to parse.
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

/// A blob's content **raw**, `git cat-file blob <id>`: the object's own
/// bytes, no `.gitattributes` conversion.
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

/// A blob's content **as it would sit in the working tree at `path`**: `git
/// cat-file --filters --path=<path> <id>`, so the path's `.gitattributes`
/// apply — `text`/`eol=crlf`/`core.autocrlf` line endings, `ident`, a smudge
/// filter.
///
/// Why [`diff3_file`] needs this and not [`blob_bytes`]: a repository with
/// `eol=crlf` stores LF blobs and checks out CRLF, so merging the raw stages
/// reproduces LF sides that match no line of the CRLF file on disk, and
/// [`attach_base`] would call a pristine file edited. Measured on git 2.54,
/// 2026-09-22: with `f.txt text eol=crlf`, the stop writes CRLF throughout
/// (markers included) while `git show :2:f.txt` is LF; `merge-file --diff3`
/// over the filtered copies prints CRLF sides and CRLF markers, matching the
/// working tree line for line.
fn filtered_bytes(repo: &RepoHandle, path: &Path, blob: Oid) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("cat-file"),
            GitArg::Flag("--filters"),
            GitArg::Glued("--path=", path_bytes(path)),
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
/// written to a scratch directory **in their working-tree form** (see
/// [`filtered_bytes`]) and merged with `git merge-file -p --diff3`, whose
/// output is what git would have written to the working tree under the
/// `diff3` conflict style — same engine, same hunks. `None` when a stage is
/// missing, since a whole-file shape has no blocks.
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
        // Filtered, not raw: the sides have to equal the working tree's
        // lines byte for byte for `attach_base` to match them.
        std::fs::write(
            scratch.path().join(name),
            filtered_bytes(repo, path, stage.blob)?,
        )?;
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

/// A block whose `(ours, theirs)` lines no run of git's own output has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseMismatch {
    /// The first block (by [`ConflictFile::blocks`] index) with no match.
    pub block: usize,
}

impl fmt::Display for BaseMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never "after edits": a pristine file reaches here too when its
        // shape is one this alignment does not model.
        write!(
            f,
            "block {} matches nothing in git's own merge of the three stages, so it has no base",
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

/// How many lines a run of regions has on one side.
fn side_count(run: &[Region], side: Side) -> usize {
    side_lines(run, side).count()
}

/// Where each conflict (open or taken) sits in a region list.
fn conflict_positions(regions: &[Region]) -> Vec<usize> {
    regions
        .iter()
        .enumerate()
        .filter(|(_, region)| !matches!(region, Region::Plain(_)))
        .map(|(at, _)| at)
        .collect()
}

/// The base of a run of reproduced regions: the base lines of each block with
/// the plain lines between them kept in place, labelled as the run's first
/// block was. `None` when a block in the run carries no base at all —
/// `merge-file --diff3` always writes one, so this is defensive.
fn run_base(run: &[Region]) -> Option<Base> {
    let mut lines = Vec::new();
    let mut label: Option<Option<Vec<u8>>> = None;
    for region in run {
        match region {
            Region::Plain(plain) => lines.extend(plain.iter().cloned()),
            Region::Conflict(block) | Region::Resolved { block, .. } => {
                let base = block.base.as_ref()?;
                lines.extend(base.lines.iter().cloned());
                if label.is_none() {
                    label = Some(base.label.clone());
                }
            }
        }
    }
    Some(Base {
        label: label?,
        lines,
    })
}

/// One file run matched to one reproduced run.
struct Aligned {
    /// How many of the file's blocks the match covers: 1 normally, more when
    /// git split one reproduced conflict into several.
    file_blocks: usize,
    /// The reproduced base for them.
    base: Option<Base>,
    /// The reproduced conflict number to resume the search at.
    next: usize,
}

/// Matches the file's blocks from `from_block` against the reproduced
/// conflicts from `from_repro`: the shortest run of reproduced regions
/// (growing first) that some run of file regions is byte-equal to on both
/// sides. Runs on either side start and end on a conflict and carry the plain
/// lines in between, so growing a run is exactly what git's joining and
/// splitting do to one region of the file.
fn find_alignment(
    file: &[Region],
    file_at: &[usize],
    from_block: usize,
    reproduced: &[Region],
    repro_at: &[usize],
    from_repro: usize,
) -> Option<Aligned> {
    let file_from = file_at[from_block];
    let rest = &file[file_from..];
    let (most_ours, most_theirs) = (side_count(rest, Side::Ours), side_count(rest, Side::Theirs));

    for (j, &repro_from) in repro_at.iter().enumerate().skip(from_repro) {
        for (k, &repro_to) in repro_at.iter().enumerate().skip(j) {
            let repro_run = &reproduced[repro_from..=repro_to];
            let (want_ours, want_theirs) = (
                side_count(repro_run, Side::Ours),
                side_count(repro_run, Side::Theirs),
            );
            // Both sides only grow with `k`: past what the rest of the file
            // could supply, this start is hopeless.
            if want_ours > most_ours || want_theirs > most_theirs {
                break;
            }
            for (p, &file_to) in file_at.iter().enumerate().skip(from_block) {
                let file_run = &file[file_from..=file_to];
                if side_count(file_run, Side::Ours) > want_ours
                    || side_count(file_run, Side::Theirs) > want_theirs
                {
                    break;
                }
                if side_lines(file_run, Side::Ours).eq(side_lines(repro_run, Side::Ours))
                    && side_lines(file_run, Side::Theirs).eq(side_lines(repro_run, Side::Theirs))
                {
                    return Some(Aligned {
                        file_blocks: p - from_block + 1,
                        base: run_base(repro_run),
                        next: k + 1,
                    });
                }
            }
        }
    }
    None
}

/// Gives each of `file`'s blocks the base git's `diff3` output has for it, as
/// [`Block::reproduced`], walking both in order and comparing content only.
///
/// **Nothing this writes is rendered**: `reproduced` is not part of the
/// file's bytes, so [`ConflictFile::render`], [`write_back`] and the line
/// numbers in [`ConflictFile::blocks`] are the same before and after. A block
/// the file itself gave a base (a `diff3`/`zdiff3` file) keeps that base
/// untouched, labels included.
///
/// git's `xdl_merge` at its default level both **joins** and **splits**, and
/// `--diff3` does neither (measured on git 2.54, 2026-09-22):
///
/// - Conflicts fewer than four unchanged lines apart are joined into one
///   block, so a file block can be *several* reproduced blocks with the plain
///   lines between them.
/// - A conflict whose two sides share interior lines is refined into several
///   blocks, so a file block can be a *fraction* of one reproduced block:
///   base `1…7` against ours `A b c d e f B` and theirs `X b c d e f Y` is
///   two blocks on disk (`A`/`X`, five plain lines, `B`/`Y`) and one block
///   with seven-line sides under `--diff3`.
///
/// So the match is between *runs*: the shortest run of reproduced regions
/// that some run of file regions is byte-equal to on both sides, each run
/// starting and ending on a conflict and carrying the plain lines between.
/// Where git split, every file block in the run is given the whole run's base
/// with [`Reproduced::shared`] set — git itself cannot say which of those
/// base lines belongs to which block, which is why `--diff3` refuses to
/// refine, so claiming otherwise would be a guess.
///
/// A reproduced conflict the file no longer has is skipped — a take written
/// to disk removes its block from the file while the stages, and so `diff3`,
/// still have it — but a file block is never given a base by position: the
/// whole call fails with the first block that matches nothing, and attaches
/// nothing. After a hand edit inside a block the answer is a mismatch, not a
/// guess. Returns how many blocks were given a base.
pub fn attach_base(file: &mut ConflictFile, diff3: &ConflictFile) -> Result<usize, BaseMismatch> {
    let file_at = conflict_positions(&file.regions);
    let repro_at = conflict_positions(&diff3.regions);

    // Decide first, attach second: a mismatch on the third block must leave
    // the first two as they were.
    let mut found: Vec<Option<Reproduced>> = Vec::with_capacity(file_at.len());
    let mut cursor = 0usize;
    while found.len() < file_at.len() {
        let index = found.len();
        let hit = find_alignment(
            &file.regions,
            &file_at,
            index,
            &diff3.regions,
            &repro_at,
            cursor,
        )
        .ok_or(BaseMismatch { block: index })?;
        let shared = hit.file_blocks > 1;
        for _ in 0..hit.file_blocks {
            found.push(hit.base.clone().map(|base| Reproduced { base, shared }));
        }
        cursor = hit.next;
    }

    let mut given = 0usize;
    let blocks = file.regions.iter_mut().filter_map(|region| match region {
        Region::Plain(_) => None,
        Region::Conflict(block) | Region::Resolved { block, .. } => Some(block),
    });
    for (block, reproduced) in blocks.zip(found) {
        if reproduced.is_some() {
            given += 1;
        }
        block.reproduced = reproduced;
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
