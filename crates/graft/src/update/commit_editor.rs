//! The commit editor's state and its keys (`c`, `a`; DESIGN.md §2.10, design
//! note "Commit editor", m3-working-tree-undo-and-stash/05).
//!
//! Pure, like the rest of `update`: no git, no clock, no file. The two things
//! it needs from outside — `commit.template`'s contents and, for an amend,
//! `HEAD`'s message — arrive as products of jobs it asked for.
//!
//! Seam rule (`crates/graft/tests/it/seam_rule.rs`): committing means
//! building a [`Operation::Commit`] or [`Operation::Amend`] and handing it to
//! [`Command::Operate`]. What that runs, what it journals and what it snapshots
//! is `graft_ops`'.
//!
//! **The refusal preview is a preview, not the decision.** It is computed
//! here from the working-tree read and the fields, so the button can be
//! dimmed with a reason beside it before anything is pressed; the real
//! refusal is `Operation::check`'s, on facts read at the moment of the write.
//! A preview that says nothing and a `check` that refuses is a normal race
//! (the tree changed under us), and the refusal then lands on the status
//! line like any other write failure.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_git::state::InProgress;
use graft_git::status::WorkingTreeStatus;
use graft_ops::operation::{CommitParams, Operation};
use graft_ui::commit_editor::Field;
use graft_ui::text_input::{TextArea, TextField};

use super::events::Command;
use super::state::{AppState, Status};

mod edit_stop;
mod keys;
mod refusal;
pub(crate) use edit_stop::{amend_at_edit_stop, at_edit_stop};
pub use keys::{committed, editor_key, paste};
pub(crate) use refusal::pane_c_acts;
pub use refusal::{Begun, OperationRefusal, live_operation, operation_refusal, refusal_tabs};

/// The trailer `Ctrl+Shift+C` starts; the user types the name and address.
const CO_AUTHOR: &str = "Co-authored-by: ";

/// Everything the editor holds between keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitEditor {
    /// The subject line.
    pub subject: TextField,
    /// The body.
    pub body: TextArea,
    /// What has focus.
    pub focus: Field,
    /// `--amend`.
    pub amend: bool,
    /// `--signoff`.
    pub sign_off: bool,
    /// `--no-verify`.
    pub no_verify: bool,
    /// `--allow-empty`.
    pub allow_empty: bool,
    /// `HEAD`'s short SHA, for the `AMEND a1b2c3d` title. Set when `a`
    /// opened the editor.
    pub head_short: Option<String>,
    /// `commit.template` has already been asked for (or applied), so
    /// reopening does not ask again.
    pub template_applied: bool,
    /// This editor was picked back up from a draft `Esc` kept, which the
    /// title says so the user knows why there is text in it.
    pub from_draft: bool,
    /// `Commit` was pressed with an empty subject and nothing has been
    /// typed in the subject since. Only then is `subject empty` drawn: an
    /// empty subject on open is the start of every commit, not a mistake
    /// (m7-power-tools/15, :776). Set by a refused commit, cleared by an
    /// edit to the subject.
    pub subject_refused: bool,
    /// Opened by `a` on `HEAD` with nothing staged and branches built on
    /// his (m8-polish-and-ship/14): `⏎` plans a reword that the line
    /// follows, previewed, instead of a plain amend.
    pub reword_line: bool,
}

impl Default for CommitEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitEditor {
    /// An empty editor, focus on the subject.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subject: TextField::new(""),
            body: TextArea::new(""),
            focus: Field::Subject,
            amend: false,
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            head_short: None,
            template_applied: false,
            from_draft: false,
            subject_refused: false,
            reword_line: false,
        }
    }

    /// An editor amending `HEAD`, prefilled with its message.
    #[must_use]
    pub fn amend(head_short: String, message: &str) -> Self {
        let mut editor = Self::new();
        editor.amend = true;
        editor.head_short = Some(head_short);
        // Whatever the old message was, it is not a template to fetch.
        editor.template_applied = true;
        editor.load_message(message);
        editor
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
        self.focus = Field::Subject;
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

    /// What the write is given.
    #[must_use]
    pub fn params(&self) -> CommitParams {
        CommitParams {
            message: self.message(),
            sign_off: self.sign_off,
            no_verify: self.no_verify,
            allow_empty: self.allow_empty,
        }
    }

    /// The operation `[ Commit ]` dispatches.
    #[must_use]
    pub fn operation(&self) -> Operation {
        if self.amend {
            Operation::Amend(self.params())
        } else {
            Operation::Commit(self.params())
        }
    }

    /// Why `[ Commit ]` would do nothing, in the words the title row and the
    /// button's neighbour both use — or `None` when it would commit.
    ///
    /// `status` is `None` until the first working-tree read lands, which is
    /// a reason of its own rather than an assumption either way.
    #[must_use]
    pub fn refusal(
        &self,
        status: Option<&WorkingTreeStatus>,
        in_progress: Option<(InProgress, Begun)>,
        tabs: u8,
    ) -> Option<String> {
        self.refusal_before_subject(status, in_progress, tabs)
            .or_else(|| self.subject_refusal())
    }

    fn subject_refusal(&self) -> Option<String> {
        self.subject
            .text()
            .trim()
            .is_empty()
            .then(|| "subject empty".to_owned())
    }

    /// [`Self::refusal`] as the editor draws it: `subject empty` only once
    /// `Commit` was pressed with it ([`Self::subject_refused`]). The gate
    /// stays [`Self::refusal`] — an empty subject never commits.
    #[must_use]
    pub fn shown_refusal(
        &self,
        status: Option<&WorkingTreeStatus>,
        in_progress: Option<(InProgress, Begun)>,
        tabs: u8,
    ) -> Option<String> {
        self.refusal_before_subject(status, in_progress, tabs)
            .or_else(|| {
                self.subject_refused
                    .then(|| self.subject_refusal())
                    .flatten()
            })
    }

    /// Every refusal but the subject's, in the order they are reported.
    fn refusal_before_subject(
        &self,
        status: Option<&WorkingTreeStatus>,
        in_progress: Option<(InProgress, Begun)>,
        tabs: u8,
    ) -> Option<String> {
        // Finishing an operation is the banner's (m6-conflicts/39, 48);
        // the words are `operation_refusal`'s, shared with the pane.
        if let Some(live) = in_progress {
            return Some(operation_refusal(live, tabs).editor_words());
        }
        // An amend with nothing staged is a reword, which is allowed
        // (`Operation::check`).
        if !self.amend && !self.allow_empty {
            match status {
                None => return Some("working tree not read yet".to_owned()),
                Some(status) if status.staged().next().is_none() => {
                    return Some("nothing staged".to_owned());
                }
                Some(_) => {}
            }
        }
        None
    }

    /// Nothing typed: an `Esc` here throws nothing away.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subject.is_empty() && self.body.is_empty()
    }

    /// The option `field` names, as a mutable flag. `None` for the subject,
    /// the body and the button.
    fn option_mut(&mut self, field: Field) -> Option<&mut bool> {
        match field {
            Field::Amend => Some(&mut self.amend),
            Field::SignOff => Some(&mut self.sign_off),
            Field::NoVerify => Some(&mut self.no_verify),
            Field::AllowEmpty => Some(&mut self.allow_empty),
            Field::Subject | Field::Body | Field::CommitButton => None,
        }
    }

    /// The four toggles and their state, in the row's order.
    #[must_use]
    pub fn options(&self) -> [(Field, bool); 4] {
        [
            (Field::Amend, self.amend),
            (Field::SignOff, self.sign_off),
            (Field::NoVerify, self.no_verify),
            (Field::AllowEmpty, self.allow_empty),
        ]
    }
}

/// Open the editor: `c` plain, `a` for an amend.
///
/// `a` does not open anything yet — it asks for `HEAD`'s message first and
/// the editor opens on the answer. The alternative (open now, drop the text
/// in when it lands, queue keys meanwhile) buys a few milliseconds and costs
/// a queue, a race between the queued keys and the prefill, and a screen that
/// changes under the user's hands; one read of one commit is not worth that.
///
/// `c` reopens whatever draft `Esc` kept, and asks for `commit.template` only
/// when the body is empty — a template over the user's own words would be
/// worse than no template at all.
pub fn open(
    state: &mut AppState,
    amend: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.commit_editor.is_some() {
        return Vec::new();
    }
    // An operation is live: the editor would open only to refuse, so the
    // pane answers in words instead (m6-conflicts/48). Nothing is read and
    // the draft stays where `Esc` left it.
    if let Some(live) = live_operation(state) {
        state.status = operation_refusal(live, refusal_tabs(state)).status();
        return Vec::new();
    }
    if amend {
        let id = next_id();
        state.head_message = Some(id);
        return vec![Command::ReadHeadMessage(id)];
    }
    let mut editor = match state.commit_draft.take() {
        Some(mut draft) => {
            draft.from_draft = true;
            draft
        }
        None => CommitEditor::new(),
    };
    let mut commands = Vec::new();
    if !editor.template_applied && editor.body.is_empty() {
        let id = next_id();
        editor.template_applied = true;
        state.commit_template = Some(id);
        commands.push(Command::ReadCommitTemplate(id));
    }
    state.commit_editor = Some(editor);
    commands
}

/// `HEAD`'s message came back: open the amend editor on it, or say why not
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
                    sign_off: false,
                    no_verify: false,
                    allow_empty: false,
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
        state.status = operation_refusal(live, refusal_tabs(state)).status();
        return;
    }
    match message {
        Ok(message) => {
            let short = head_short(state);
            let mut editor = CommitEditor::amend(short, &message);
            editor.reword_line = state.amend_into.reword_read_answered(id);
            state.commit_editor = Some(editor);
        }
        // Not swallowed: `a` did nothing, and the user is told why — an
        // unborn `HEAD` is the common reason.
        Err(why) => state.status = Status::sticky(format!("amend: {why}")),
    }
}

/// `HEAD`'s short SHA for the `AMEND a1b2c3d` title, from the refs the badges
/// already read. Unknown (no refs yet) reads as `HEAD`, which is true and
/// says nothing false.
fn head_short(state: &AppState) -> String {
    state
        .refs
        .as_deref()
        .and_then(crate::refs::RefBadges::head)
        .map_or_else(
            || "HEAD".to_owned(),
            |oid| oid.to_string().chars().take(7).collect(),
        )
}

/// `commit.template` came back. Only fills an empty body: between asking and
/// answering the user may already have typed.
pub fn template_arrived(state: &mut AppState, id: JobId, template: Result<Option<String>, String>) {
    if state.commit_template != Some(id) {
        return;
    }
    state.commit_template = None;
    let template = match template {
        Ok(template) => template,
        // The editor is already open and usable; the template's failure is
        // reported rather than dropped, and the body stays empty.
        Err(why) => {
            state.status = Status::Failed(format!("commit.template: {why}"));
            return;
        }
    };
    let Some(text) = template else {
        return;
    };
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    if !editor.body.is_empty() {
        return;
    }
    let body = strip_comments(&text);
    if !body.trim().is_empty() {
        editor.body = TextArea::new(body.trim_end_matches('\n'));
    }
}

/// `$EDITOR` closed: the file it wrote, or why Graft could not use it.
///
/// Dropped when the editor it belongs to is gone — `Esc` while `$EDITOR` was
/// open, or a second `Ctrl+E` — so a message never lands in a field the user
/// has moved on from.
pub fn editor_returned(state: &mut AppState, id: JobId, result: Result<String, String>) {
    if state.external_editor != Some(id) {
        return;
    }
    state.external_editor = None;
    let Some(editor) = state.commit_editor.as_mut() else {
        return;
    };
    match result {
        Ok(text) => editor.load_message(strip_comments(&text).trim_end_matches('\n')),
        // The old text is still in the fields: nothing was thrown away, and
        // the reason says so.
        Err(why) => state.status = Status::sticky(why),
    }
}

/// Drop the lines git itself would strip from a message file: those starting
/// with `#`, once leading whitespace is ignored the way git's own
/// `--cleanup=strip` does not.
///
/// Kept simple on purpose: git strips a line whose **first** character is the
/// comment character. A line indented before its `#` is content to git, and
/// so it is here too.
fn strip_comments(text: &str) -> String {
    let kept: Vec<&str> = text
        .split('\n')
        .filter(|line| !line.starts_with('#'))
        .collect();
    kept.join("\n")
}

#[cfg(test)]
mod tests;
