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

/// The trailer `Ctrl+Shift+C` starts; the user types the name and address.
const CO_AUTHOR: &str = "Co-authored-by: ";

/// Who began the operation git is part-way through — which decides whether
/// the operation banner's `c` and `a` can end it (m6-07 F12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Begun {
    /// Graft did: its journal entry is live, and the banner's keys end it.
    InGraft,
    /// The shell did: no entry of Graft's is live. The banner's `c` finishes
    /// a merge (as its own `FinishMerge` entry, m6-conflicts/20); every other
    /// key, and `c` on any other kind, refuses it (`Refusal::NotInProgress`)
    /// — that is finished in the shell.
    InShell,
}

/// The operation git is part-way through and who began it, as
/// [`CommitEditor::refusal`] needs it; `None` when nothing is in progress.
///
/// "Graft began it" is `AppState::live_entries` being non-empty — the last
/// journal read's `Pending` entries git is still inside, the same fact the
/// banner's `c`/`a` look up in `graft_ops` before they run. Until the first
/// journal read lands it is empty, so an operation a previous run of Graft
/// left is worded as the shell's for that moment: the refusal then names
/// git's own command, which still works, rather than a key that might not.
#[must_use]
pub fn live_operation(state: &AppState) -> Option<(InProgress, Begun)> {
    let kind = state.in_progress.as_ref()?.kind;
    let begun = if state.live_entries.is_empty() {
        Begun::InShell
    } else {
        Begun::InGraft
    };
    Some((kind, begun))
}

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
    ) -> Option<String> {
        // Finishing a merge, rebase, cherry-pick or revert Graft began is
        // the operation banner's, from M5 (ADR-0034's shell command was the
        // interim while there was nothing in the app to name). The keys are
        // the banner's own (`docs/design/02-keymap.md` §2.6a); `Esc` first,
        // because this editor is modal and holds every key while it is open.
        if let Some((kind, begun)) = in_progress {
            return Some(match (kind, begun) {
                // A bisect draws no banner — that is M7's — so there are no
                // keys to name and git's own command is still the way out.
                (InProgress::Bisect, _) => format!(
                    "{} in progress — {} first",
                    kind.name(),
                    kind.continue_command()
                ),
                // One begun in the shell has no entry of Graft's. The
                // banner's `a` refuses it (`resume_target`), and so does
                // its `c` on a rebase, pick or revert (M7). Naming them
                // offered keys that do nothing (m6-07 F12), so the way out
                // is git's own command, as for a bisect.
                //
                // A merge's `c` finishes it since m6-conflicts/20, but
                // `esc, then c` from here does not reach the banner: after
                // `Esc` the pane has the keys and `c` reopens this editor
                // (`update::keys::graph_key`, `is_commit_key`). Until that
                // is decided (ticket 20 Part B NOTES) the words stay git's
                // command, which is still true.
                (_, Begun::InShell) => format!(
                    "{} begun in the shell — finish it there: {}",
                    kind.name(),
                    kind.continue_command()
                ),
                (_, Begun::InGraft) => format!(
                    "{} in progress — esc, then c continue  a abort",
                    kind.name()
                ),
            });
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
        if self.subject.text().trim().is_empty() {
            return Some("subject empty".to_owned());
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

/// `HEAD`'s message came back: open the amend editor on it, or say why not.
pub fn head_message_arrived(state: &mut AppState, id: JobId, message: Result<String, String>) {
    if state.head_message != Some(id) {
        return;
    }
    state.head_message = None;
    match message {
        Ok(message) => {
            let short = head_short(state);
            state.commit_editor = Some(CommitEditor::amend(short, &message));
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

/// The editor's keys while it is open. `None` means "not the editor's" — and
/// the editor claims every key but `Ctrl+C`, which still quits (DESIGN.md
/// §2.2: a terminal application that ignores it feels broken).
pub fn editor_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Option<Vec<Command>> {
    state.commit_editor.as_ref()?;
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return None;
    }
    // While the commit runs the screen is not the user's to change: the text
    // on it is what is being written. `Esc` cancels the job.
    if state.operating.is_some() {
        let mut commands = Vec::new();
        if key.code == KeyCode::Escape {
            // The same stop every write gets (m4-remotes/05): the line says
            // `cancelling`, and the cancelled answer reads `cancelled`.
            let _stopping: bool = super::remote::stop_write(state, &mut commands);
        }
        return Some(commands);
    }
    Some(open_editor_key(state, key, next_id))
}

/// [`editor_key`] once the editor is known open and idle.
fn open_editor_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // Whole-editor keys first: they mean the same wherever the focus is.
    if key.modifiers.control {
        return control_key(state, key, next_id);
    }
    match key.code {
        KeyCode::Escape => {
            // The same rule the graph screen follows (`keys.rs`): `Esc`
            // dismisses a failure before it means anything else. The editor
            // draws the reason on its footer, so without this the only way
            // to read a rejected commit's message and then get rid of it
            // would be to close the editor (m3/05 review, Major 1).
            if matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
                state.status = Status::Ready;
                return Vec::new();
            }
            close(state);
            return Vec::new();
        }
        KeyCode::Tab => {
            with_editor(state, |editor| editor.focus = editor.focus.next());
            return Vec::new();
        }
        KeyCode::BackTab => {
            with_editor(state, |editor| editor.focus = editor.focus.previous());
            return Vec::new();
        }
        _ => {}
    }
    let focus = state
        .commit_editor
        .as_ref()
        .map_or(Field::Subject, |editor| editor.focus);
    match focus {
        Field::Subject => subject_key(state, key),
        Field::Body => body_key(state, key),
        Field::CommitButton => button_key(state, key, next_id),
        option => option_key(state, key, option),
    }
}

/// `Ctrl`-held keys: they mean the same wherever the focus is.
fn control_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match key.code {
        // ADR-0034: `Ctrl+Enter` is indistinguishable from `Enter` outside
        // the kitty keyboard protocol, so it is a shortcut for the button
        // and never the only way in.
        KeyCode::Enter => return commit(state, next_id),
        KeyCode::Char('s') => {
            with_editor(state, |editor| editor.sign_off = !editor.sign_off);
        }
        // Shift is already folded into the character, so this is `Ctrl+Shift+C`.
        KeyCode::Char('C') => {
            with_editor(state, |editor| {
                editor.body.append_line(CO_AUTHOR);
                editor.focus = Field::Body;
            });
        }
        KeyCode::Char('e') => return edit_outside(state, next_id),
        KeyCode::Left => with_editor(state, |editor| match editor.focus {
            Field::Body => editor.body.word_left(),
            _ => editor.subject.word_left(),
        }),
        KeyCode::Right => with_editor(state, |editor| match editor.focus {
            Field::Body => editor.body.word_right(),
            _ => editor.subject.word_right(),
        }),
        _ => {}
    }
    Vec::new()
}

/// `Ctrl+E`: hand the message to `$EDITOR`. Which file it is written to is
/// the loop's to decide — `update` knows no repository directory — so the
/// command carries only the text.
fn edit_outside(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let Some(editor) = state.commit_editor.as_ref() else {
        return Vec::new();
    };
    let id = next_id();
    let message = editor.message();
    state.external_editor = Some(id);
    vec![Command::EditCommitMessage { id, message }]
}

/// Run `f` over the open editor; a no-op when there is none.
/// A bracketed paste, into whichever text field has focus.
///
/// `true` when the editor took it, which it does whenever it is open: the
/// editor is modal, so a paste arriving while it is up is meant for it even
/// when the focus is on a toggle and there is nowhere to put the text.
///
/// The fields do the deciding about newlines — the subject drops them
/// because it is one line, the body keeps them — so pasting a whole commit
/// message into the body arrives with its paragraphs intact.
///
/// Graft turns bracketed paste *on*, so before this the user's paste into
/// the editor did nothing whatsoever (m3/05 review, Major 3).
pub fn paste(state: &mut AppState, text: &str) -> bool {
    if state.commit_editor.is_none() || state.operating.is_some() {
        return false;
    }
    with_editor(state, |editor| match editor.focus {
        Field::Subject => editor.subject.insert_str(text),
        Field::Body => editor.body.insert_str(text),
        // A toggle or the button has focus: there is no text field to put
        // this in. Claimed all the same, so it cannot fall through to a
        // screen the user cannot see.
        Field::Amend
        | Field::SignOff
        | Field::NoVerify
        | Field::AllowEmpty
        | Field::CommitButton => {}
    });
    true
}

fn with_editor(state: &mut AppState, f: impl FnOnce(&mut CommitEditor)) {
    if let Some(editor) = state.commit_editor.as_mut() {
        f(editor);
    }
}

/// `Esc`: close, keeping a non-empty draft for the next `c` this session.
fn close(state: &mut AppState) {
    // Whatever `$EDITOR` is about to hand back belongs to an editor that is
    // no longer on screen.
    state.external_editor = None;
    let Some(editor) = state.commit_editor.take() else {
        return;
    };
    state.commit_draft = (!editor.is_empty()).then_some(editor);
}

/// The editor closed because its commit landed: the draft goes too, or the
/// next `c` would reopen a message that is now a commit.
pub fn committed(state: &mut AppState) {
    state.commit_editor = None;
    state.commit_draft = None;
    state.external_editor = None;
}

/// Keys while the subject has focus. `Enter` moves to the body rather than
/// inserting a newline: a subject is one line by definition.
fn subject_key(state: &mut AppState, key: &KeyPress) -> Vec<Command> {
    with_editor(state, |editor| match key.code {
        // `Enter` and `Down` both leave the subject, because a subject is one
        // line by definition and there is nothing below it but the body.
        KeyCode::Enter | KeyCode::Down => editor.focus = Field::Body,
        KeyCode::Char(character) => editor.subject.insert(character),
        KeyCode::Backspace => editor.subject.backspace(),
        KeyCode::Delete => editor.subject.delete(),
        KeyCode::Left => editor.subject.left(),
        KeyCode::Right => editor.subject.right(),
        KeyCode::Home => editor.subject.home(),
        KeyCode::End => editor.subject.end(),
        _ => {}
    });
    Vec::new()
}

/// Keys while the body has focus.
fn body_key(state: &mut AppState, key: &KeyPress) -> Vec<Command> {
    with_editor(state, |editor| match key.code {
        KeyCode::Enter => editor.body.newline(),
        KeyCode::Char(character) => editor.body.insert(character),
        KeyCode::Backspace => editor.body.backspace(),
        KeyCode::Delete => editor.body.delete(),
        KeyCode::Left => editor.body.left(),
        KeyCode::Right => editor.body.right(),
        KeyCode::Up => {
            if editor.body.cursor().0 == 0 {
                editor.focus = Field::Subject;
            } else {
                editor.body.up();
            }
        }
        KeyCode::Down => editor.body.down(),
        KeyCode::Home => editor.body.home(),
        KeyCode::End => editor.body.end(),
        _ => {}
    });
    Vec::new()
}

/// Keys while one of the four toggles has focus.
fn option_key(state: &mut AppState, key: &KeyPress, option: Field) -> Vec<Command> {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Enter => {
            with_editor(state, |editor| {
                if let Some(flag) = editor.option_mut(option) {
                    *flag = !*flag;
                }
            });
        }
        KeyCode::Left | KeyCode::Right => {
            let forward = key.code == KeyCode::Right;
            with_editor(state, |editor| {
                if let Some(next) = option.sibling_option(forward) {
                    editor.focus = next;
                }
            });
        }
        KeyCode::Up => with_editor(state, |editor| editor.focus = Field::Body),
        KeyCode::Down => with_editor(state, |editor| editor.focus = Field::CommitButton),
        _ => {}
    }
    Vec::new()
}

/// Keys while `[ Commit ]` has focus.
fn button_key(
    state: &mut AppState,
    key: &KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match key.code {
        KeyCode::Enter | KeyCode::Char(' ') => commit(state, next_id),
        KeyCode::Up => {
            with_editor(state, |editor| editor.focus = Field::Amend);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Dispatch the commit, unless the preview already knows it would be
/// refused — in which case the reason is already beside the button and on the
/// title row, and pressing it again says nothing new.
fn commit(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let status = state.working_tree.as_deref();
    let in_progress = live_operation(state);
    let Some(editor) = state.commit_editor.as_ref() else {
        return Vec::new();
    };
    if editor.refusal(status, in_progress).is_some() {
        return Vec::new();
    }
    let operation = editor.operation();
    let mut commands = Vec::new();
    super::operate::launch(state, &mut commands, operation, next_id);
    commands
}

#[cfg(test)]
mod tests;
