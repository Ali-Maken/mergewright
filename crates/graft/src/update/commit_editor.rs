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
use graft_ui::keymap::ActionId;
use graft_ui::text_input::{TextArea, TextField};

use super::events::Command;
use super::state::{AppState, Status};

mod edit_stop;
mod files;
mod keys;
mod landing;
mod refusal;
mod returned;
pub(crate) use edit_stop::{amend_at_edit_stop, at_edit_stop};
pub use files::{FileDiffState, commit_files_arrived, diff_arrived, include_of, tree_read};
pub(crate) use files::{claims_diff, counted, display_order, kind_of};
pub(crate) use keys::toggle_from_palette;
pub use keys::{committed, editor_key, paste};
pub use landing::{Draft, LastCommit, commit_landed, landed_bar, push_words, undo_landed};
pub(crate) use refusal::pane_c_acts;
pub use refusal::{Begun, OperationRefusal, live_operation, operation_refusal, refusal_way};
pub use returned::{editor_returned, template_arrived};

/// The trailer `Ctrl+Shift+C` starts; the user types the name and address.
const CO_AUTHOR: &str = "Co-authored-by: ";

/// One of the four options, off the screen and never hidden: each is a flag
/// on [`CommitEditor`], named in the header while on (Q8 a).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommitOption {
    /// `--amend`: rewrite the last commit.
    Amend,
    /// `--signoff` (`⌃S`).
    SignOff,
    /// `--no-verify`: skip the commit hooks.
    NoVerify,
    /// `--allow-empty`: a commit with nothing going in.
    AllowEmpty,
}

impl CommitOption {
    /// The one id → option map: the palette's three keyless
    /// `commit.toggle_*` rows (ease-pass/03b, review 03 Minor 5). Each id is
    /// spelled here and nowhere else in the screen's code.
    #[must_use]
    pub fn of_id(id: &ActionId) -> Option<CommitOption> {
        match id.0 {
            "commit.toggle_amend" => Some(CommitOption::Amend),
            "commit.toggle_no_verify" => Some(CommitOption::NoVerify),
            "commit.toggle_allow_empty" => Some(CommitOption::AllowEmpty),
            _ => None,
        }
    }

    /// The header's word while on; `None` for `Amend` (the header says
    /// `amending …` instead).
    #[must_use]
    pub fn header_word(self) -> Option<&'static str> {
        match self {
            CommitOption::Amend => None,
            CommitOption::SignOff => Some("signing off"),
            CommitOption::NoVerify => Some("skipping hooks"),
            CommitOption::AllowEmpty => Some("empty allowed"),
        }
    }

    /// The header's order: signing off, skipping hooks, empty allowed.
    pub const HEADER_ORDER: [CommitOption; 3] = [
        CommitOption::SignOff,
        CommitOption::NoVerify,
        CommitOption::AllowEmpty,
    ];
}

/// Everything the screen holds between keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitEditor {
    /// The subject line.
    pub subject: TextField,
    /// The body.
    pub body: TextArea,
    /// What has focus.
    pub focus: CommitFocus,
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
            amend: false,
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            head_short: None,
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

    /// What the write is given: the files going in (C6, C9), or the index
    /// as it is for an amend he never touched.
    #[must_use]
    pub fn params(&self) -> CommitParams {
        CommitParams {
            message: self.message(),
            sign_off: self.sign_off,
            no_verify: self.no_verify,
            allow_empty: self.allow_empty,
            // A plain commit always takes what the screen shows going in.
            include: include_of(&self.files, self.touched || !self.amend),
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
    /// amend he never touched may change only its message, and
    /// `--allow-empty` lets nothing go in.
    #[must_use]
    pub fn refusal(&self) -> Option<Refusal> {
        if self.subject.text().trim().is_empty() {
            return Some(Refusal::NoMessage);
        }
        let message_only = self.amend && !self.touched;
        if self.going_in() == 0 && !message_only && !self.allow_empty {
            return Some(Refusal::NoFile);
        }
        None
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

/// Open the screen: `c` plain, `a` for an amend.
///
/// `a` does not open anything yet — it asks for `HEAD`'s message first and
/// the screen opens on the answer. One read of one commit is not worth a
/// screen that changes under the user's hands.
///
/// `c` resumes a draft `esc` kept for a new commit, and asks for
/// `commit.template` only when the body is empty. Either way the commit he
/// just made stops being the one `u` gives back as a message
/// ([`LastCommit`]).
pub fn open(
    state: &mut AppState,
    amend: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.commit_editor.is_some() {
        return Vec::new();
    }
    // An operation is live: the screen would open only to refuse, so the
    // pane answers in words instead (m6-conflicts/48). Nothing is read and
    // the draft stays where `esc` left it.
    if let Some(live) = live_operation(state) {
        state.status = operation_refusal(live, refusal_way(state), &state.keymap).status();
        return Vec::new();
    }
    state.last_commit = None;
    if amend {
        let id = next_id();
        state.head_message = Some(id);
        return vec![Command::ReadHeadMessage(id)];
    }
    let mut editor = match state.commit_draft.take() {
        Some(draft) if draft.amending.is_none() => CommitEditor::from_draft(&draft),
        // A draft kept from an amend waits for the next `a`.
        kept => {
            state.commit_draft = kept;
            CommitEditor::new()
        }
    };
    let mut commands = Vec::new();
    if !editor.template_applied && editor.body.is_empty() && !editor.from_draft {
        let id = next_id();
        editor.template_applied = true;
        state.commit_template = Some(id);
        commands.push(Command::ReadCommitTemplate(id));
    }
    show(state, &mut commands, editor, next_id);
    commands
}

/// Put `editor` on screen with its files chosen fresh (C6) and their counts
/// and the first file's diff asked for.
fn show(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mut editor: CommitEditor,
    next_id: &mut impl FnMut() -> JobId,
) {
    files::choose(&mut editor, state.working_tree.as_deref());
    let id = next_id();
    editor.counts_job = Some(id);
    commands.push(Command::ReadCommitFiles(id));
    state.commit_editor = Some(Box::new(editor));
    files::ask_diff(state, commands, next_id);
}

/// `HEAD`'s message came back: open the amend screen on it, or say why not
/// — or, for `a` at an `edit` stop, amend with it at once (07b).
pub fn head_message_arrived(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    message: Result<String, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.head_message != Some(id) {
        return;
    }
    state.head_message = None;
    let edit_amend = std::mem::take(&mut state.edit_amend);
    // P14's exception: at an `edit` stop the amend is the step itself
    // (`graft_ops`' `check` holds the same rule against a fresh read).
    if edit_amend && at_edit_stop(state) {
        match message {
            Ok(message) => {
                let amend = Operation::Amend(CommitParams {
                    message,
                    ..CommitParams::default()
                });
                let launched = super::operate::launch(state, commands, amend, next_id);
                super::operate::say_if_refused(state, launched);
            }
            Err(why) => state.status = Status::sticky(format!("amend: {why}")),
        }
        return;
    }
    // An operation became live while the read was in flight: refuse as
    // `open` would have, whatever the read said.
    if let Some(live) = live_operation(state) {
        state.status = operation_refusal(live, refusal_way(state), &state.keymap).status();
        return;
    }
    match message {
        Ok(message) => {
            let head = head_oid(state);
            let short = head.map_or_else(
                || "HEAD".to_owned(),
                |oid| oid.to_string().chars().take(7).collect(),
            );
            let reword_line = state.amend_into.reword_read_answered(id);
            // A reword's text he already typed for this `HEAD` comes back
            // (review m8-14 Major 2); `HEAD`'s message otherwise.
            let kept = reword_line
                .then(|| super::amend_into::kept_reword(state))
                .flatten();
            let mut editor = if let Some(mut kept) = kept {
                kept.from_draft = true;
                kept
            } else {
                let mut editor = CommitEditor::amend(short, &message);
                editor.reword_line = reword_line;
                // A draft `esc` kept from an amend of this same `HEAD`
                // comes back over its message (C15).
                if let Some(draft) = state.commit_draft.take() {
                    if draft.amending.is_some() && draft.amending == head {
                        editor.subject = TextField::new(&draft.subject);
                        editor.body = TextArea::new(&draft.body);
                        editor.from_draft = true;
                    } else {
                        state.commit_draft = Some(draft);
                    }
                }
                editor
            };
            editor.amending = head;
            show(state, commands, editor, next_id);
        }
        // Not swallowed: `a` did nothing, and the user is told why — an
        // unborn `HEAD` is the common reason.
        Err(why) => state.status = Status::sticky(format!("amend: {why}")),
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

/// `HEAD`'s commit, from the refs the badges already read; `None` before
/// the first refs answer.
fn head_oid(state: &AppState) -> Option<Oid> {
    state.refs.as_deref().and_then(crate::refs::RefBadges::head)
}

#[cfg(test)]
mod screen_tests;
#[cfg(test)]
pub(crate) mod tests;
