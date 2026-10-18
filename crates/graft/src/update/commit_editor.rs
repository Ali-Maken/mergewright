//! The commit screen's state and its keys (`c`, `a`; ease-pass/08, the
//! commit mock; DESIGN.md §2.10).
//!
//! Pure, like the rest of `update`: no git, no clock, no file. What it needs
//! from outside — `commit.template`, `HEAD`'s message for an amend, every
//! change's line counts and the diff of the file under the cursor — arrives
//! as products of jobs it asked for.
//!
//! Seam rule (`crates/graft/tests/it/seam_rule.rs`): committing means
//! building a [`Operation::Commit`] or [`Operation::Amend`] and handing it to
//! [`Command::Operate`]. What that runs, what it journals and what it snapshots
//! is `graft_ops`'.
//!
//! **The refusal is a preview, not the decision.** `⏎` is refused here when
//! there is no message or no file going in, so the bar says why before
//! anything is pressed; the real refusal is `Operation::check`'s, on facts
//! read at the moment of the write, and lands on the status row like any
//! other write failure.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::state::InProgress;
use graft_ops::operation::{CommitParams, FileChoice, Operation};
use graft_ui::commit_editor::{CommitFocus, Refusal};
use graft_ui::text_input::{TextArea, TextField};

use super::events::Command;
use super::state::{AppState, Status};

mod done_row;
mod drafts;
mod edit_stop;
mod files;
mod keys;
mod landing;
mod mouse;
mod open;
mod options;
mod refusal;
mod returned;
pub(crate) use done_row::{done_row, done_shown};
pub use drafts::{Draft, Drafts, WayIn, give_back, redo_took_back};
pub(crate) use edit_stop::{amend_at_edit_stop, at_edit_stop};
pub use files::{
    FileDiffState, commit_files_arrived, diff_arrived, include_of, shown_section, tree_read,
};
pub(crate) use files::{claims_diff, counted, display_order, kind_of};
pub(crate) use keys::toggle_from_palette;
pub use keys::{committed, editor_key, paste};
pub use landing::{LastCommit, commit_landed, landed_bar, push_words, redo_landed, undo_landed};
pub(crate) use mouse::mouse;
pub use open::{head_message_arrived, open};
pub(crate) use open::{head_oid, short_sha};
pub use options::CommitOption;
pub(crate) use refusal::pane_c_acts;
pub use refusal::{Begun, OperationRefusal, live_operation, operation_refusal, refusal_way};
pub use returned::{editor_returned, template_arrived};

/// The trailer `Ctrl+Shift+C` starts; the user types the name and address.
const CO_AUTHOR: &str = "Co-authored-by: ";

/// Everything the screen holds between keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitEditor {
    /// The subject line.
    pub subject: TextField,
    /// The body.
    pub body: TextArea,
    /// What has focus.
    pub focus: CommitFocus,
    /// `c` or `a`: which slot `esc` keeps the words in, and whether an
    /// untouched amend takes the index as it is (08s M1, M2).
    pub way_in: WayIn,
    /// `--amend`.
    pub amend: bool,
    /// `--signoff`.
    pub sign_off: bool,
    /// `--no-verify`.
    pub no_verify: bool,
    /// `--allow-empty`.
    pub allow_empty: bool,
    /// `<sha7> <subject>` of the commit an amend rewrites, for the header.
    /// Set when `a` opened the screen.
    pub head_short: Option<String>,
    /// What `a` prefilled (`HEAD`'s message): `esc` keeps the words only
    /// when they differ from it (08s M1).
    pub prefilled: Option<Box<Draft>>,
    /// The commit `a` opened on: a draft `esc` keeps from here resumes on
    /// the next `a` only while `HEAD` is still this commit.
    pub amending: Option<Oid>,
    /// `commit.template` has already been asked for (or applied), so
    /// reopening does not ask again.
    pub template_applied: bool,
    /// This screen was picked back up from a draft `esc` kept.
    pub from_draft: bool,
    /// Opened by `a` on `HEAD` with nothing staged and branches built on
    /// his (m8-polish-and-ship/14): `⏎` plans a reword that the line
    /// follows, previewed, instead of a plain amend.
    pub reword_line: bool,
    /// Every changed file and whether it goes in, in the status' order.
    /// Worked out fresh on every open (C6), never carried in a draft.
    pub files: Vec<FileChoice>,
    /// He put a file in or left one out with `space`. An amend he never
    /// touched takes the index as it is ([`include_of`]).
    pub touched: bool,
    /// Index into `files` of the file under the cursor.
    pub file_cursor: usize,
    /// The line counts: asked for on open, `None` until they land.
    pub counts: Option<std::sync::Arc<graft_git::diff::FileList>>,
    /// The line-count read in flight; only its answer is taken.
    pub counts_job: Option<JobId>,
    /// The diff of the file under the cursor.
    pub diff: FileDiffState,
}

impl Default for CommitEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitEditor {
    /// An empty screen, focus on the subject, nothing chosen yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subject: TextField::new(""),
            body: TextArea::new(""),
            focus: CommitFocus::Subject,
            way_in: WayIn::Commit,
            amend: false,
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            head_short: None,
            prefilled: None,
            amending: None,
            template_applied: false,
            from_draft: false,
            reword_line: false,
            files: Vec::new(),
            touched: false,
            file_cursor: 0,
            counts: None,
            counts_job: None,
            diff: FileDiffState::default(),
        }
    }

    /// A screen amending `HEAD`, prefilled with its message. `head_short` is
    /// `<sha7>`; the header adds the message's subject.
    #[must_use]
    pub fn amend(head_short: String, message: &str) -> Self {
        let mut editor = Self::new();
        editor.way_in = WayIn::Amend;
        editor.amend = true;
        let subject = message.lines().next().unwrap_or_default().trim();
        editor.head_short = Some(if subject.is_empty() {
            head_short
        } else {
            format!("{head_short} {subject}")
        });
        // Whatever the old message was, it is not a template to fetch.
        editor.template_applied = true;
        editor.load_message(message);
        editor.prefilled = Some(Box::new(editor.draft()));
        editor
    }

    /// A screen resuming `draft`'s words (C15). Options start off.
    #[must_use]
    pub fn from_draft(draft: &Draft) -> Self {
        let mut editor = Self::new();
        editor.subject = TextField::new(&draft.subject);
        editor.body = TextArea::new(&draft.body);
        editor.amending = draft.amending;
        editor.from_draft = true;
        editor
    }

    /// Words typed over what `a` prefilled; on a `c` screen, any words.
    #[must_use]
    pub fn words_typed(&self) -> bool {
        match self.prefilled.as_deref() {
            Some(prefilled) => {
                prefilled.subject != self.subject.text() || prefilled.body != self.body.text()
            }
            None => !self.is_empty(),
        }
    }

    /// The words `esc` keeps.
    #[must_use]
    pub fn draft(&self) -> Draft {
        Draft {
            subject: self.subject.text().to_owned(),
            body: self.body.text(),
            amending: self.amending,
        }
    }

    /// Replace subject and body with `text`, split the way git splits a
    /// commit message: the first line is the subject, one blank line is the
    /// separator, the rest is the body. Comment lines are already gone by
    /// the time this is called.
    pub fn load_message(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n");
        let (subject, body) = match text.split_once('\n') {
            Some((first, rest)) => (first, rest.trim_start_matches('\n')),
            None => (text.as_str(), ""),
        };
        self.subject = TextField::new(subject.trim_end());
        self.body = TextArea::new(body.trim_end_matches('\n'));
        self.focus = CommitFocus::Subject;
    }

    /// The whole message, the way git wants it on stdin: subject, a blank
    /// line, then the body, with a trailing newline. No body, no blank line.
    #[must_use]
    pub fn message(&self) -> String {
        let subject = self.subject.text().trim();
        let body = self.body.text();
        let body = body.trim_end();
        if body.is_empty() {
            format!("{subject}\n")
        } else {
            format!("{subject}\n\n{body}\n")
        }
    }

    /// What the write is given: the rows going in (C6, C9), or the index as
    /// it is for an `a` screen still amending, rows untouched (08s M2).
    #[must_use]
    pub fn params(&self) -> CommitParams {
        CommitParams {
            message: self.message(),
            sign_off: self.sign_off,
            no_verify: self.no_verify,
            allow_empty: self.allow_empty,
            include: include_of(&self.files, self.way_in, self.amend, self.touched),
        }
    }

    /// The operation `⏎` dispatches.
    #[must_use]
    pub fn operation(&self) -> Operation {
        if self.amend {
            Operation::Amend(self.params())
        } else {
            Operation::Commit(self.params())
        }
    }

    /// How many files go in.
    #[must_use]
    pub fn going_in(&self) -> usize {
        self.files.iter().filter(|file| file.goes_in).count()
    }

    /// Why `⏎` would do nothing (C9): no message first, then no file — an
    /// `a` amend he never touched may change only its message (with amend
    /// turned on, `c` still needs a file: a message-only amend is `a`'s),
    /// and `--allow-empty` lets nothing go in.
    #[must_use]
    pub fn refusal(&self) -> Option<Refusal> {
        if self.subject.text().trim().is_empty() {
            return Some(Refusal::NoMessage);
        }
        if self.going_in() == 0 && !self.message_only() && !self.allow_empty {
            return Some(Refusal::NoFile);
        }
        None
    }

    /// `⏎` takes the index as it is: an `a` screen, still amending, its
    /// rows untouched (08s M2).
    #[must_use]
    pub fn message_only(&self) -> bool {
        self.way_in == WayIn::Amend && self.amend && !self.touched
    }

    /// Nothing typed: an `esc` here throws nothing away.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subject.is_empty() && self.body.is_empty()
    }

    /// The flag `option` names.
    pub fn option_mut(&mut self, option: CommitOption) -> &mut bool {
        match option {
            CommitOption::Amend => &mut self.amend,
            CommitOption::SignOff => &mut self.sign_off,
            CommitOption::NoVerify => &mut self.no_verify,
            CommitOption::AllowEmpty => &mut self.allow_empty,
        }
    }

    /// Whether `option` is on.
    #[must_use]
    pub fn option(&self, option: CommitOption) -> bool {
        match option {
            CommitOption::Amend => self.amend,
            CommitOption::SignOff => self.sign_off,
            CommitOption::NoVerify => self.no_verify,
            CommitOption::AllowEmpty => self.allow_empty,
        }
    }
}

/// The words the screen shows in place of its bar while an operation is
/// live (it began while the screen was up): where to finish it, spelled
/// with the shipped keys, since the screen is a text field (10b W4).
/// `None` while nothing is live.
#[must_use]
pub fn live_words(state: &AppState) -> Option<String> {
    let live = live_operation(state)?;
    Some(
        operation_refusal(
            live,
            refusal_way(state),
            graft_ui::keymap::Keymap::shipped(),
        )
        .editor_words(),
    )
}

#[cfg(test)]
mod draft_tests;
#[cfg(test)]
mod landing_tests;
#[cfg(test)]
mod mask_tests;
#[cfg(test)]
mod screen_tests;
#[cfg(test)]
pub(crate) mod tests;
