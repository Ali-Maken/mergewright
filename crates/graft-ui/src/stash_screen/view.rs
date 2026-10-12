//! The stash overlay's input types and the overlay struct the widget draws (m8/21b-1 split from `stash_screen.rs`).

use super::{FieldView, FileListView, Keymap, SanitizedText, Theme};

/// One stash entry, as the overlay draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashRowView {
    /// `stash@{0}`, git's own name for the entry.
    pub name: SanitizedText,
    /// The message git recorded, already quoted by the builder.
    pub message: SanitizedText,
    /// How long ago, in words: `2h ago`.
    pub age: SanitizedText,
    /// How many paths the stash touched.
    pub files: usize,
}

/// The save prompt (`s`): a message line and the scope toggle.
#[derive(Debug)]
pub struct StashPromptView<'a> {
    /// What the prompt is about: `Stash the working tree`, or the refusal's
    /// own title on a clean tree.
    pub title: SanitizedText,
    /// The message field, always focused — it is the only text here.
    pub message: FieldView<'a>,
    /// `true` when `--staged` is chosen; `false` is `-u`, the default.
    pub staged_only: bool,
    /// Why `Enter` would not save. Drawn under the options row; `None` when
    /// the save may go ahead.
    pub refusal: Option<SanitizedText>,
}

/// What draws instead of, or with, the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StashStatus {
    /// The read is in flight.
    Loading,
    /// The list is there and empty.
    Empty,
    /// The list could not be read; the line is git's own.
    Failed(SanitizedText),
    /// There are rows.
    Ready,
}

/// Everything the overlay draws.
#[derive(Debug)]
pub struct StashView<'a> {
    /// Newest first, `stash@{0}` at the top.
    pub rows: Vec<StashRowView>,
    /// The row under the cursor, an index into `rows`.
    pub selected: usize,
    /// The selected stash's files, once `Space` has asked for them.
    pub preview: Option<&'a FileListView>,
    /// The save prompt, while it is open. Modal within the overlay.
    pub prompt: Option<StashPromptView<'a>>,
    /// What to draw when there are no rows.
    pub status: StashStatus,
    /// What is happening or just happened — a running write, a refusal, a
    /// finished apply. The overlay covers the graph screen's own footer, so
    /// without this line a write started from here would have no
    /// acknowledgement at all (m3-working-tree-undo-and-stash/06).
    pub notice: Option<SanitizedText>,
    /// Is `notice` a failure? Drawn in `Warning` rather than muted.
    pub notice_is_failure: bool,
    /// A box (the put-back preview) is open over the list and holds every
    /// key: the list's bar row is left blank, as the graph's is under a
    /// preview (review 86, M2).
    pub box_open: bool,
    /// A write is running and `Esc` stops it before the list sees the key
    /// (graft's `update::remote::esc_stops_write`): the bar reads `esc
    /// cancels` … `S close`, never `S/esc close` (m8/59 Major A).
    pub esc_cancels: bool,
}

/// The overlay itself. Carries the theme like every other screen; the
/// caller paints the surface underneath.
#[derive(Debug)]
pub struct StashScreen<'a> {
    /// What to draw.
    pub view: &'a StashView<'a>,
    /// Resolved colours.
    pub theme: &'a Theme,
    /// His keymap: the footer's keys are spelled from it.
    pub keys: &'a Keymap,
}

/// `3 files` / `1 file`, because "1 files" reads as a bug in the counter.
#[must_use]
pub fn files_label(files: usize) -> String {
    if files == 1 {
        "1 file".to_owned()
    } else {
        format!("{files} files")
    }
}
