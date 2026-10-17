//! The commit screen (ease-pass/08, commit mock
//! `prototypes/ease-mocks/mergewright-commit-screen.html`): a header line
//! saying where the commit goes, a message box (subject, then three body
//! rows), one sentence saying what goes in, then the files going in and
//! staying out beside the diff of the one under the cursor, and a bar of
//! keys for whichever part has focus.
//!
//! Owns no state and reads no repository: [`crate::text_input`] holds the
//! text, `graft::update::commit_editor` holds the rest, and
//! `graft::view::commit_editor` turns that into the [`CommitScreenView`]
//! this draws over the whole frame.
//!
//! Every fact the user needs is in **characters**, never in colour or
//! weight alone: which part has focus (`▸`), which file the cursor is on
//! (`▸` with the files focused, `›` without), whether a file goes in (`✓`)
//! or stays out (`·`, `!` for a secrets file). A capture records the
//! character grid and nothing else (CLAUDE.md rules 5 and 6).

use graft_term::SanitizedText;

use crate::detail::BodyView;
use crate::keymap::Keymap;
use crate::text_input::{AreaView, FieldView};
use crate::theme::Theme;

mod draw;
mod words;

pub use words::{bar, summary_line};

/// Characters the subject is meant to stay inside: the counter reads
/// `n/72` and turns `Warning` past it (C2).
pub const SUBJECT_LIMIT: usize = 72;

/// Characters a body line is meant to stay inside; its `┊` guide's column.
pub const BODY_LIMIT: u16 = 72;

/// The file list's width beside the diff (design note "New UI": `files 44 |
/// diff`).
pub const FILES_WIDTH: u16 = 44;

/// Below this many columns the diff is hidden and the files take the width.
pub const DIFF_COLUMNS: u16 = 100;

/// What has focus. `⇥` walks subject → body → files → subject (C11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitFocus {
    /// The one-line subject: typing goes here, `⏎` commits.
    Subject,
    /// The body: `⏎` is a newline.
    Body,
    /// The file list: `space` puts a file in or leaves it out.
    Files,
}

impl CommitFocus {
    /// The next part `⇥` moves to.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            CommitFocus::Subject => CommitFocus::Body,
            CommitFocus::Body => CommitFocus::Files,
            CommitFocus::Files => CommitFocus::Subject,
        }
    }

    /// The previous part, for `⇧⇥`.
    #[must_use]
    pub fn previous(self) -> Self {
        match self {
            CommitFocus::Subject => CommitFocus::Files,
            CommitFocus::Body => CommitFocus::Subject,
            CommitFocus::Files => CommitFocus::Body,
        }
    }

    /// The subject or the body: a key there is typed (`?` included).
    #[must_use]
    pub fn types(self) -> bool {
        matches!(self, CommitFocus::Subject | CommitFocus::Body)
    }
}

/// Which heading a file sits under (C5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileGroup {
    /// `Going in (n)`, marked `✓`.
    GoingIn,
    /// `Staying out (n)`, dim, marked `·` (or `!` for a secrets file).
    StayingOut,
}

/// One changed file as the list draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitFileRow {
    /// The path, as the status lists it.
    pub path: SanitizedText,
    /// The one-letter change (`M`, `A`, `D`, `R`), as the mock's second
    /// column shows it.
    pub kind: char,
    /// Which heading it is under.
    pub group: FileGroup,
    /// Lines added, once the line counts are read; `None` before then and
    /// for a binary file.
    pub added: Option<u32>,
    /// Lines removed, as `added`.
    pub removed: Option<u32>,
    /// A new file whose name looks like keys or credentials (C7): left out
    /// on its own, marked `!` while it stays out.
    pub secret: bool,
}

/// Where the commit goes, as the summary line names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Onto {
    /// A new commit on this branch (or `HEAD`'s short SHA when detached).
    NewCommit(SanitizedText),
    /// Into the last commit (`a`).
    Amend,
}

/// The one sentence above the files (C4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Summary {
    /// Something goes in: `3 files go in, +60 −3, as a new commit on
    /// wallet-release. 1 stays out.`
    Goes {
        /// Files going in.
        files: u32,
        /// `(added, removed)` over the files going in; `None` until every
        /// one of them has its counts.
        lines: Option<(u32, u32)>,
        /// Where they go.
        onto: Onto,
        /// Files staying out.
        out: u32,
    },
    /// An amend with no file going in: only the message changes.
    MessageOnly {
        /// Files staying out.
        out: u32,
    },
    /// Nothing goes in: `Nothing goes in. Put a file in with space.`
    Nothing,
}

/// Why `⏎` would do nothing (C9). The bar's `⏎` slot says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The subject is empty: `type a message first`.
    NoMessage,
    /// No file goes in: `put a file in first`.
    NoFile,
}

/// The header line: where the commit goes, the push state, the options on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitHeader {
    /// The branch `HEAD` is on (or `HEAD`'s short SHA when detached).
    pub branch: SanitizedText,
    /// `<sha7> <subject>` of the commit an amend rewrites; `None` for a
    /// new commit.
    pub amending: Option<SanitizedText>,
    /// `not pushed yet` / `N commits to push`; `None` when there is nothing
    /// to say (in sync, detached, not read yet) — never a word that reads
    /// as a problem when nothing is wrong (CLAUDE.md status rule).
    pub push: Option<SanitizedText>,
    /// The options that are on, in their order (`signing off`, `skipping
    /// hooks`, `empty allowed`); empty when all are off (Q8 a).
    pub options: Vec<&'static str>,
}

/// Everything the commit screen draws.
#[derive(Clone, Debug)]
pub struct CommitScreenView<'a> {
    /// The header line.
    pub header: CommitHeader,
    /// The subject: its text, cursor and `n/72` counter (`label` unused).
    pub subject: FieldView<'a>,
    /// The body: its lines and cursor (`label` unused).
    pub body: AreaView<'a>,
    /// The sentence above the files.
    pub summary: Summary,
    /// Every changed file, going in first, then staying out.
    pub files: Vec<CommitFileRow>,
    /// Index into `files` of the file under the cursor.
    pub cursor: usize,
    /// What has focus.
    pub focus: CommitFocus,
    /// The diff of the file under the cursor.
    pub diff: BodyView,
    /// Why `⏎` would do nothing, if it would.
    pub refusal: Option<Refusal>,
    /// Why the last write failed (a hook, a refusal), while it is worth
    /// saying: it takes the bar's row until `esc` dismisses it.
    pub status: Option<SanitizedText>,
    /// A commit is running: the bar says so, and every key but `esc` is
    /// ignored.
    pub busy: bool,
    /// A merge, rebase or other operation is live (it began while the
    /// screen was up): where to finish it, in place of the bar, since `⏎`
    /// is refused until it is done (m6-conflicts/48).
    pub live: Option<SanitizedText>,
}

/// [`CommitScreenView`] drawn over the area it is given: the whole frame.
#[derive(Debug)]
pub struct CommitScreen<'a> {
    /// What to draw.
    pub view: &'a CommitScreenView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// The keys the bar spells. The screen is a text field, so its keys
    /// are fixed (ADR-0055): the caller hands the shipped table.
    pub keys: &'a Keymap,
}

#[cfg(test)]
mod tests;
