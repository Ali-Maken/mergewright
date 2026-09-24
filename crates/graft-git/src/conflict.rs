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

use graft_core::oid::Oid;
use sha1::{Digest, Sha1};

mod base;
mod disk;
mod document;
mod parser;

pub use base::{BaseMismatch, attach_base};
pub use disk::{Diff3Error, ReadError, WriteError, diff3_file, read, stage_bytes, write_back};
pub use parser::parse;

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

#[cfg(test)]
mod tests;
