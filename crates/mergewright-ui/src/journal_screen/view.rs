//! The journal screen's view types: the rows, the status row and the whole view the app hands it.

use super::SanitizedText;
use crate::keymap::{ActionId, Keymap};
use crate::theme::Theme;
use ratatui::style::Style;
use ratatui::text::Line;

/// What a row is to `u` and `U`, in words rather than colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowMark {
    /// An ordinary entry: in force, but not the one a key would act on.
    #[default]
    None,
    /// `u` takes this one back.
    NextUndo,
    /// `U` puts this one back.
    NextRedo,
    /// A previous run died between the entry's two journal lines, so what
    /// it did is not known from the journal alone.
    Interrupted,
    /// A later entry has already reversed this one.
    Undone,
    /// It stopped before writing anything — a refused pull, a rejected
    /// push, a git failure: `nothing written`, the one fact they share; the
    /// entry does not record why it stopped (m8/20b R8).
    NothingWritten,
}

impl RowMark {
    /// The mark as it is drawn, at the right of the row: `← u`, `← U`,
    /// the keys spelled from `keys` (`← next undo` when he left it
    /// unbound). Empty for [`RowMark::None`], which is most rows.
    #[must_use]
    pub fn label(self, keys: &Keymap) -> String {
        let next = |action: &'static str, word: &str| match keys.spell_keys(&[ActionId(action)]) {
            Some(key) => format!("\u{2190} {key}"),
            None => format!("\u{2190} next {word}"),
        };
        match self {
            RowMark::None => String::new(),
            RowMark::NextUndo => next("app.undo", "undo"),
            RowMark::NextRedo => next("app.redo", "redo"),
            RowMark::Interrupted => "interrupted".to_owned(),
            RowMark::Undone => "undone".to_owned(),
            RowMark::NothingWritten => "nothing written".to_owned(),
        }
    }
}

/// Whose row it is (J1's distinctions: glyph, source word, indent).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowKind {
    /// `●` — a journal entry: done in Mergewright, `u` can undo it.
    #[default]
    Mergewright,
    /// `$` — a step the shell took, from git's reflog: `u` never touches it.
    Outside,
    /// `└ ✗ lost` — a commit no branch holds, under the step that lost it.
    Lost,
    /// `└ ✗ lost` — an edit an undo replaced, kept only in its backup.
    LostEdit,
    /// `└ ⇢ replaced` — a commit an undoable step moved a branch off: `u`
    /// brings it back (usability Q1).
    Replaced,
    /// `└ ⇢ deleted` — a commit an undoable step deleted the branch of: `u`
    /// brings it back.
    Deleted,
}

/// A row's word in the source column (ADR-0057 §10): `Mergewright` rows name the
/// product from the names table, `Outside` rows say `outside` — no product
/// word, so no later rename cuts it to fit. The `✗`/`⇢` rows draw no source
/// word: their own text stands where it would be.
#[must_use]
pub fn source_word(kind: RowKind) -> &'static str {
    match kind {
        RowKind::Mergewright => mergewright_core::on_disk::Generation::CURRENT.title(),
        RowKind::Outside => "outside",
        RowKind::Lost | RowKind::LostEdit | RowKind::Replaced | RowKind::Deleted => "",
    }
}

/// One row, as the browser draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRowView {
    /// How long ago, already in words: `2m ago`. Empty on a `✗` row.
    pub age: SanitizedText,
    /// What happened: `commit "Add OAuth provider"`, `reset main 1 commit
    /// back`, or a `✗` row's whole line.
    pub description: SanitizedText,
    /// A `●` row's second column — what it moved and whether a backup was
    /// kept: `1 ref · backup saved`. Empty on other rows.
    pub summary: SanitizedText,
    /// What this row is to `u`/`U`.
    pub mark: RowMark,
    /// The Command Log: every `git` line the operation ran, shown while the
    /// row is expanded.
    pub commands: Vec<SanitizedText>,
    /// Why an `Aborted` entry failed, in full, shown while the row is
    /// expanded; `None` for an entry that finished.
    pub failure: Option<SanitizedText>,
    /// The refs a fetch (or a pull's fetch) moved, one line each
    /// (`origin/main  a1b2c3d → e4f5a6b`, then `+N more`), drawn while the
    /// row is expanded under [`super::moved_heading`] (ADR-0062). Empty →
    /// nothing drawn.
    pub moved: Vec<SanitizedText>,
    /// `Space` opened this row's log.
    pub expanded: bool,
    /// Whose row it is.
    pub kind: RowKind,
    /// A `$` row's own reflog subject, drawn as `git: "<subject>"`: in its
    /// column at 180 columns and wider, else under the row while it is
    /// selected. Sanitised by the caller (the other fields are too).
    pub git_subject: Option<SanitizedText>,
    /// A `✗` row's event: the index of the row it sits under.
    pub under: Option<usize>,
}

impl Default for JournalRowView {
    /// An empty `●` row: the base the caller fills in.
    fn default() -> Self {
        JournalRowView {
            age: mergewright_term::sanitize::sanitize(""),
            description: mergewright_term::sanitize::sanitize(""),
            summary: mergewright_term::sanitize::sanitize(""),
            mark: RowMark::None,
            commands: Vec::new(),
            failure: None,
            moved: Vec::new(),
            expanded: false,
            kind: RowKind::Mergewright,
            git_subject: None,
            under: None,
        }
    }
}

/// What draws instead of, or with, the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalStatus {
    /// The read is in flight.
    Loading,
    /// The journal is there and empty: nothing has been written yet.
    Empty,
    /// The journal could not be read; the line is git's or the parser's own.
    Failed(SanitizedText),
    /// There are rows.
    Ready,
}

/// J's status row: a refusal or a notice, drawn above the legend only while
/// there is one (the status rule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalStatusRow {
    /// The line, already fitted by the caller.
    pub words: String,
    /// A refusal or a failure, drawn as a warning; else a notice, muted.
    pub problem: bool,
    /// The problem is a failure (`Status::Failed`): drawn `⊗ <words>`, the mark
    /// the renderer's. A refusal or a sticky line is a problem, not a failure.
    pub failure: bool,
    /// The ASCII tier: the failure mark is `(x)`.
    pub ascii: bool,
}

impl JournalStatusRow {
    /// A refusal: a problem, not a failure, so no mark.
    #[must_use]
    pub fn refusal(words: &str) -> Self {
        Self {
            words: words.to_owned(),
            problem: true,
            failure: false,
            ascii: false,
        }
    }

    /// The row as drawn: `ink` for the words, and for a failure the
    /// renderer's mark before them (`crate::failure`).
    pub(super) fn line(&self, theme: &Theme, ink: Style) -> Line<'static> {
        if self.failure {
            crate::failure::line(&self.words, self.ascii, theme, ink)
        } else {
            Line::styled(self.words.clone(), ink)
        }
    }
}

/// J's kept `/` find, for the title's tail (m7-power-tools/30o-c): drawn as
/// blame's `FindTitle` is, ` · find "<query>" <at> of <of>`, or
/// ` · find "<query>" · no match`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalFindView {
    /// What was found, sanitised.
    pub query: SanitizedText,
    /// Which hit the cursor is at, 1-based; `None` when nothing matched.
    pub at: Option<usize>,
    /// How many rows it matched.
    pub of: usize,
}

/// Everything the browser draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalView {
    /// Which repository this journal belongs to.
    pub location: SanitizedText,
    /// Newest first; `✗` rows right after their event.
    pub rows: Vec<JournalRowView>,
    /// The row under the cursor, an index into `rows`.
    pub selected: usize,
    /// What to draw when there are no rows.
    pub status: JournalStatus,
    /// After the title, while the lost-commit scan is out or when it could
    /// not finish: `finding lost commits…`. `None` once it is done.
    pub title_suffix: Option<String>,
    /// The refusal or notice to show, if any.
    pub status_row: Option<JournalStatusRow>,
    /// What `⏎` does on the selected row, the verb alone (`undo this`; the
    /// widget spells `journal.revert`'s key in front of it from his
    /// keymap), or `None` when the row alone cannot name it.
    pub enter_words: Option<String>,
    /// `b` has a commit to name on the selected row: the bar offers
    /// `b new branch here` only then (graft's `row_commit`, the function
    /// `b` itself reads).
    pub branch_here: bool,
    /// A box is docked over the browser and names the keys: the bar is
    /// left blank, as the stash list's is under its box.
    pub box_open: bool,
    /// The find `⏎` kept: the title's tail, and `n N match` on the bar.
    pub find: Option<JournalFindView>,
    /// The `/` prompt while he types, drawn in place of the bar (blame's).
    pub find_prompt: Option<SanitizedText>,
    /// A write is running and `Esc` stops it before J sees the key
    /// (graft's `update::remote::esc_stops_write`): the bar then reads
    /// `J close` and `esc cancels`, never `J/esc close` (m8/59 Major A).
    pub esc_cancels: bool,
}
