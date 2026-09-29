//! The rewrite editor's view types: what the app hands the widget to draw.

use super::SanitizedText;

/// Everything the editor draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteEditorView {
    /// `REWRITE feature-a · 5 commits since it left main · newest first ·
    /// nothing written until y`.
    pub title: SanitizedText,
    /// His rows newest first, then the base row last.
    pub rows: Vec<EditorRow>,
    /// Index into `rows`.
    pub cursor: usize,
    /// `r`'s message field, drawn on the line under its row.
    pub field: Option<FieldView>,
    /// The message the rewrite will write, when a squash group or a reword
    /// exists.
    pub message: Option<MessageBlock>,
    /// The AFTER plan or `⏎`'s file list.
    pub right: RightSide,
    /// Below [`NARROW_BELOW`]: `Tab` has swapped the rows for the right side.
    pub narrow_shows_right: bool,
    /// A refusal in words, on the area's last row.
    pub notice: Option<SanitizedText>,
    /// The ASCII renderer tier: every glyph has an ASCII stand-in.
    pub ascii: bool,
}

/// A row's action, which picks its word and glyph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowMark {
    /// `pick`.
    Pick,
    /// `reword ✎`.
    Reword,
    /// `edit ⏸`.
    Edit,
    /// `squash ↓`.
    Squash,
    /// `fixup ↓`.
    Fixup,
    /// `drop ✗`, subject muted.
    Drop,
    /// The row under his: `─ <label>`, not editable.
    Base {
        /// The upstream's or the default branch's short name, or a sha.
        label: SanitizedText,
    },
}

/// One row on the left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRow {
    /// Its action.
    pub mark: RowMark,
    /// Its short sha.
    pub short: String,
    /// Its subject as it stands.
    pub subject: SanitizedText,
    /// What the row says about itself (`melds into the row below · marked by
    /// its name`, `✗ nothing below to meld into`, `stays`).
    pub note: Option<SanitizedText>,
    /// `↕ moved`, after the note.
    pub moved: bool,
}

/// `r`'s one-line field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldView {
    /// The row it edits (index into `rows`); the field is the line under it.
    pub row: usize,
    /// The subject being typed.
    pub text: SanitizedText,
    /// The caret, a character index.
    pub caret: usize,
}

/// The message block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageBlock {
    /// `MESSAGE of the squashed commit · r on "<kept subject>" changes it`.
    pub label: SanitizedText,
    /// The message, one entry per line, each drawn behind `┊`.
    pub lines: Vec<SanitizedText>,
}

/// What the right-hand side shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RightSide {
    /// The AFTER plan.
    After(AfterView),
    /// `⏎`: one row's changes.
    Changes {
        /// The row's short sha.
        short: String,
        /// Its files.
        files: ChangesView,
    },
}

/// The AFTER plan's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfterView {
    /// `checking…` in the header.
    Checking,
    /// The plan could not be read, in words.
    Failed(SanitizedText),
    /// The plan (or the todo) refuses, in words; `y` gives the same.
    Refused(SanitizedText),
    /// The plan landed.
    Ready {
        /// The branches after `y`, top down, base last.
        rows: Vec<AfterRow>,
        /// `built on yours: …`, `on origin: …`, `no conflicts expected`.
        facts: Vec<SanitizedText>,
    },
}

/// One AFTER row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfterRow {
    /// The branch label, on the first row of each branch's run.
    pub branch: Option<SanitizedText>,
    /// New (and whether his new tip) or unchanged.
    pub mark: AfterMark,
    /// The commit's subject.
    pub subject: SanitizedText,
    /// `built on yours · follows`, `3 in one`, `unchanged`, ….
    pub note: Option<SanitizedText>,
}

/// An AFTER row's mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfterMark {
    /// His branch's new tip: `● new`.
    NewTip,
    /// A new commit: `○ new`.
    New,
    /// Kept as it is, by its short sha.
    Unchanged {
        /// The short sha.
        short: String,
    },
}

/// `⏎`'s file list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangesView {
    /// Asked for, not arrived.
    Reading,
    /// The read failed, in words.
    Failed(SanitizedText),
    /// Each file: path, and lines added and removed — `None` for a binary
    /// file, which git counts no lines for (drawn `binary`).
    Files(Vec<(SanitizedText, Option<(u32, u32)>)>),
}
