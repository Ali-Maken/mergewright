//! The rewrite editor (`R`, m7-power-tools/08, note R1, keymap §2.7): his
//! branch as rows, newest first; git's letters mark them; the AFTER plan
//! and the squashed message show before `y`; `y` runs the rewrite that was
//! pictured.
//!
//! The life of one editor:
//! 1. `R` ([`open_rewrite`]) → `opening`: a read (`Command::OpenRewrite`),
//!    `reading feature-a…` on the status line, no overlay yet.
//! 2. Its answer ([`opened`]) → `editor`: the overlay (`GraphOverlay::Rewrite`)
//!    and its first AFTER plan (`Command::PlanPreview`).
//! 3. Every edit that changes the params replans (`after`); the last edit's
//!    plan is the one kept, older answers are dropped by id.
//! 4. `y` → `launched`: the write runs through `operate`; a refusal brings
//!    the editor back with the words ([`write_answered`]), anything else
//!    ends it.
//!
//! Every job the editor starts is routed back here by id ([`owns`]) before
//! the handlers that share its products (the preview, the detail pane, the
//! commit editor's `$EDITOR`), so none of them ever sees the editor's
//! answers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::diff::FileList;
use graft_ops::rewrite::{
    RewriteParams, SquashGroup, TodoAction, TodoLine, TodoProblem, squash_groups,
};
use graft_ui::text_input::TextField;

use crate::jobs::operate::Operated;
use crate::jobs::rewrite_open::{BaseRow, OpenedRewrite, RewriteOpened};

use super::detail::Loading;
use super::events::Command;
use super::preview::{Planned, PreviewFailure};
use super::state::{AppState, Status};

mod after;
mod keys;
#[cfg(test)]
pub(crate) mod tests;
mod words;

pub(crate) use keys::{rewrite_key, rewrite_mouse};

/// The editor's place in [`AppState`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RewriteState {
    /// The open read in flight.
    pub(crate) opening: Option<JobId>,
    /// The row `R` was pressed on, for the cursor once the read answers.
    opening_from: Option<Oid>,
    /// The editor on screen: `Some` exactly while `GraphOverlay::Rewrite`.
    pub(crate) editor: Option<Box<RewriteEditor>>,
    /// After `y`, until the write answers (ruling 6).
    pub(crate) launched: Option<Box<RewriteEditor>>,
    /// Jobs a closed or superseded editor left in flight: their answers
    /// (or their cancellation) still arrive, and are dropped here rather
    /// than reaching a handler that shares their product.
    stale: HashSet<JobId>,
}

/// One open editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RewriteEditor {
    /// What `plan` gave: `head`, `onto`, `line`, `made` come from here.
    opened: RewriteParams,
    /// Every row's whole message, by commit.
    whole: HashMap<Oid, String>,
    /// The row under his.
    base: BaseRow,
    /// His rows, oldest first, as git runs them; the screen shows them
    /// reversed.
    todo: Vec<TodoLine>,
    /// Rows `plan` marked at open (a `fixup!`/`squash!` subject) that he has
    /// not re-marked since: `marked by its name`.
    by_name: HashSet<Oid>,
    /// Rows he moved: `↕ moved`.
    moved: HashSet<Oid>,
    /// His message edits, by commit: a reword row's message, or a squash
    /// group's (keyed by its kept row). By commit, not todo index: a move
    /// changes indices.
    edits: HashMap<Oid, String>,
    /// The cursor, as a screen row: 0 is the newest, `todo.len()` the base.
    cursor: usize,
    /// The one-line message field under a row, while `r` has it open.
    field: Option<Field>,
    /// The AFTER plan.
    after: After,
    /// What the right-hand side shows.
    right: Right,
    /// Below 120 columns: `Tab` shows the AFTER side instead of the rows.
    narrow_shows_right: bool,
    /// A refusal in words, on the row above the bar, until the next key.
    notice: Option<String>,
    /// A left button went down on a row and has not come up: a drag moves
    /// that row.
    dragging: bool,
}

/// `r`'s message field.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Field {
    /// The commit whose message it edits.
    commit: Oid,
    /// The subject line being typed.
    text: TextField,
    /// `Ctrl+E`'s `$EDITOR` handover, while the terminal is away.
    handover: Option<JobId>,
}

/// The AFTER plan's state.
#[derive(Clone, Debug, PartialEq, Eq)]
enum After {
    /// Asked for `asked`; only `job`'s answer is wanted.
    Pending { job: JobId, asked: RewriteParams },
    /// Landed for `asked`.
    Ready {
        planned: Arc<Planned>,
        asked: RewriteParams,
    },
    /// The read failed, in words.
    Failed(String),
    /// The plan's `check` refused, in the one wording (H4).
    Refused(String),
    /// The todo cannot run; nothing was planned.
    Invalid(TodoProblem),
}

/// The right-hand side.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Right {
    /// The AFTER block.
    After,
    /// `⏎`'s file list of one row's changes.
    Changes {
        commit: Oid,
        files: Loading<Arc<FileList>>,
    },
}

/// What the view (`view::rewrite_editor`) reads to draw the editor: everything on screen, in the
/// screen's order, and nothing it has to work out again.
#[derive(Debug)]
pub(crate) struct ViewFacts<'a> {
    /// His branch's short name (`feature-a`).
    pub branch: &'a str,
    /// How many commits since it left the base, as opened.
    pub since: usize,
    /// His rows, newest first.
    pub rows: Vec<RowFacts<'a>>,
    /// The row under his.
    pub base: &'a BaseRow,
    /// The cursor: an index into `rows`, or `rows.len()` on the base row.
    pub cursor: usize,
    /// `r`'s field: its screen row, text and caret (a char index).
    pub field: Option<(usize, &'a str, usize)>,
    /// One block per squash group or reword, newest kept row first: what
    /// the rewrite will write.
    pub messages: Vec<MessageFacts>,
    /// The AFTER side.
    pub after: AfterFacts<'a>,
    /// `Some` while `⏎` shows a row's changes instead of AFTER.
    pub changes: Option<(Oid, &'a Loading<Arc<FileList>>)>,
    /// Below 120 columns: the AFTER side is up instead of the rows.
    pub narrow_shows_right: bool,
    /// A refusal in words, above the bar.
    pub notice: Option<&'a str>,
    /// The `$EDITOR` handover is out: the terminal is away.
    pub handing_over: bool,
}

/// One of his rows.
#[derive(Debug)]
pub(crate) struct RowFacts<'a> {
    /// Its action now.
    pub action: TodoAction,
    /// Its commit.
    pub commit: Oid,
    /// Its subject as it stands (a reword's edited subject).
    pub subject: &'a str,
    /// `↕ moved`.
    pub moved: bool,
    /// `marked by its name`.
    pub by_name: bool,
    /// Its commit is the kept row of a group: `n in one` counts it and the
    /// melded rows.
    pub keeps: usize,
    /// What is wrong with it (`nothing below to meld into`), drawn `✗`.
    pub problem: Option<String>,
}

/// One message the rewrite will write.
#[derive(Debug)]
pub(crate) struct MessageFacts {
    /// The kept (or reworded) row.
    pub kept: Oid,
    /// Its subject, for the label's `r on "<subject>"`.
    pub kept_subject: String,
    /// A squash group's (`MESSAGE of the squashed commit`) or a reword's.
    pub squashed: bool,
    /// The whole message.
    pub text: String,
}

/// The AFTER side's state.
#[derive(Debug)]
pub(crate) enum AfterFacts<'a> {
    /// `checking…`.
    Checking,
    /// The read failed.
    Failed(&'a str),
    /// The plan refused.
    Refused(&'a str),
    /// The todo cannot run: the problem's words.
    Invalid(String),
    /// The plan: its ghosts, forecast and `published`.
    Ready(&'a Planned),
}

impl RewriteEditor {
    fn new(opened: OpenedRewrite, from: Oid) -> Self {
        let OpenedRewrite {
            params,
            whole,
            base,
        } = opened;
        let todo = params.todo.clone();
        let by_name = todo
            .iter()
            .filter(|row| row.action != TodoAction::Pick)
            .map(|row| row.commit)
            .collect();
        let cursor = todo
            .iter()
            .rposition(|row| row.commit == from)
            .map_or(0, |index| todo.len() - 1 - index);
        RewriteEditor {
            opened: params,
            whole,
            base,
            todo,
            by_name,
            moved: HashSet::new(),
            edits: HashMap::new(),
            cursor,
            field: None,
            after: After::Invalid(TodoProblem::EverythingDropped),
            right: Right::After,
            narrow_shows_right: false,
            notice: None,
            dragging: false,
        }
    }

    /// What `y` runs and the AFTER plan is asked for (ruling 8): `plan`'s
    /// head, onto, line and made, his todo, and `messages` rebuilt by todo
    /// index from his edits (kept by commit):
    /// - each squash group: his edit of its kept row, else — when a
    ///   `squash` row melds in — `default_message`; a fixup-only group
    ///   keeps its kept row's message, as git does and as `plan` left it;
    /// - each `reword` row: his edit.
    ///
    /// # Errors
    /// `validate`'s problem: nothing is planned or run.
    pub(crate) fn params(&self) -> Result<RewriteParams, TodoProblem> {
        graft_ops::rewrite::validate(&self.todo)?;
        let mut messages = std::collections::BTreeMap::new();
        for group in squash_groups(&self.todo) {
            if let Some(message) = self.group_message(&group) {
                messages.insert(group.kept, message);
            }
        }
        for (index, row) in self.todo.iter().enumerate() {
            if row.action == TodoAction::Reword
                && let Some(edit) = self.edits.get(&row.commit)
            {
                messages.entry(index).or_insert_with(|| edit.clone());
            }
        }
        Ok(RewriteParams {
            todo: self.todo.clone(),
            messages,
            ..self.opened.clone()
        })
    }

    /// A squash group's message, when the rewrite writes one.
    fn group_message(&self, group: &SquashGroup) -> Option<String> {
        let kept = self.todo.get(group.kept)?;
        if let Some(edit) = self.edits.get(&kept.commit) {
            return Some(edit.clone());
        }
        let squashes = group
            .melded
            .iter()
            .any(|&row| self.todo.get(row).map(|line| line.action) == Some(TodoAction::Squash));
        squashes.then(|| self.default_message(group))
    }

    /// `default_message` over the whole messages read at open.
    fn default_message(&self, group: &SquashGroup) -> String {
        // `default_message` asks only for the group's own rows, which are
        // todo indices by construction (`squash_groups`); an index past the
        // end cannot be asked, so the empty string is never a message.
        let message_of = |row: usize| {
            self.todo
                .get(row)
                .map_or_else(String::new, |line| self.whole_of(line))
        };
        graft_ops::rewrite::default_message(group, &message_of, &self.todo)
    }

    /// A row's whole message as read at open, else its subject: the job
    /// reads every row's, so the fallback is for a row it could not have
    /// missed.
    fn whole_of(&self, line: &TodoLine) -> String {
        self.whole
            .get(&line.commit)
            .cloned()
            .unwrap_or_else(|| format!("{}\n", line.subject))
    }

    /// Screen row → todo index; `None` for the base row.
    fn todo_index(&self, screen: usize) -> Option<usize> {
        (screen < self.todo.len()).then(|| self.todo.len() - 1 - screen)
    }

    /// The todo index of the cursor's row, `None` on the base.
    fn cursor_row(&self) -> Option<usize> {
        self.todo_index(self.cursor)
    }

    /// The kept row a melding row at `index` melds into.
    fn melds_into(&self, index: usize) -> Option<usize> {
        squash_groups(&self.todo)
            .into_iter()
            .find(|group| group.melded.contains(&index))
            .map(|group| group.kept)
    }

    /// The group whose kept row is `index`, if one melds a row into it.
    fn group_of(&self, index: usize) -> Option<SquashGroup> {
        squash_groups(&self.todo)
            .into_iter()
            .find(|group| group.kept == index)
    }

    /// Every job this editor has in flight.
    fn jobs(&self) -> impl Iterator<Item = JobId> + '_ {
        let after = match &self.after {
            After::Pending { job, .. } => Some(*job),
            _ => None,
        };
        let files = match &self.right {
            Right::Changes {
                files: Loading::Pending(job),
                ..
            } => Some(*job),
            _ => None,
        };
        let handover = self.field.as_ref().and_then(|field| field.handover);
        after.into_iter().chain(files).chain(handover)
    }

    /// Everything the view draws (`view::rewrite_editor`).
    pub(crate) fn view_facts(&self) -> ViewFacts<'_> {
        words::view_facts(self)
    }
}

/// Is the editor's `r` field open (`AppState::text_field_open`)?
pub(crate) fn field_open(state: &AppState) -> bool {
    state
        .rewrite
        .editor
        .as_ref()
        .is_some_and(|editor| editor.field.is_some())
}

// ── opening ────────────────────────────────────────────────────────────

/// `R` on the graph: read what the editor opens on, for the cursor's row
/// (`HEAD`'s on the uncommitted row). A second `R` while reading starts
/// nothing; a write running or git part-way through refuses in the preview's
/// words (one wording, H4).
pub(crate) fn open_rewrite(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.opening.is_some() || state.rewrite.editor.is_some() {
        return;
    }
    if let Some(reason) = super::preview::preview_refusal(state) {
        state.status = Status::Sticky { reason, keys: None };
        return;
    }
    let Some(from) = cursor_commit(state) else {
        state.status =
            Status::Waiting("still reading the refs \u{2014} try again in a moment".to_owned());
        return;
    };
    let id = next_id();
    state.rewrite.opening = Some(id);
    state.rewrite.opening_from = Some(from);
    state.status = Status::Waiting(format!("reading {}\u{2026}", branch_name(state)));
    commands.push(Command::OpenRewrite { id, from });
}

/// The commit `R` opens on: the cursor's row, or `HEAD` on the uncommitted
/// row. `None` while the history or the refs it needs are unread.
fn cursor_commit(state: &AppState) -> Option<Oid> {
    if state.on_working_tree {
        return state.refs.as_deref()?.head();
    }
    Some(state.history.as_ref()?.selected()?.commit.id)
}

/// His branch's short name, for `reading feature-a…`.
fn branch_name(state: &AppState) -> String {
    state
        .refs
        .as_deref()
        .and_then(super::preview::current_branch)
        .map_or_else(
            || "your branch".to_owned(),
            |tip| super::preview::short_name_of(&tip.name),
        )
}

/// `Esc` while the read is out: cancel it and clear its line. `true` when
/// there was one.
fn cancel_opening(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    let Some(id) = state.rewrite.opening.take() else {
        return false;
    };
    state.rewrite.opening_from = None;
    commands.push(Command::CancelJob(id));
    state.rewrite.stale.insert(id);
    if matches!(state.status, Status::Waiting(_)) {
        state.status = Status::Ready;
    }
    true
}

/// The open read answered: the overlay, a refusal on the status line, or
/// the failure.
pub(crate) fn opened(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<RewriteOpened, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.stale.remove(&id) || state.rewrite.opening != Some(id) {
        return;
    }
    state.rewrite.opening = None;
    let from = state.rewrite.opening_from.take();
    match answer {
        Ok(RewriteOpened::Editor(opened)) => {
            if matches!(state.status, Status::Waiting(_)) {
                state.status = Status::Ready;
            }
            let from = from.unwrap_or(opened.params.head.oid);
            let mut editor = Box::new(RewriteEditor::new(*opened, from));
            after::replan(state, &mut editor, commands, next_id);
            state.rewrite.editor = Some(editor);
        }
        Ok(RewriteOpened::Refused(refusal)) => {
            state.status = Status::sticky(refusal.explain());
        }
        Err(why) => {
            state.status = Status::Failed(format!("couldn't open the rewrite editor: {why}"));
        }
    }
}

// ── routing ────────────────────────────────────────────────────────────

/// Is `id` the open read, the AFTER plan, the `⏎` file list or the
/// `$EDITOR` handover of the live editor — or one a closed or superseded
/// editor left in flight, whose answer is dropped here?
pub(crate) fn owns(state: &AppState, id: JobId) -> bool {
    let rewrite = &state.rewrite;
    rewrite.opening == Some(id)
        || rewrite.stale.contains(&id)
        || rewrite
            .editor
            .as_ref()
            .is_some_and(|editor| editor.jobs().any(|job| job == id))
}

/// The AFTER plan answered. Only the live editor's current plan is kept.
pub(crate) fn after_planned(
    state: &mut AppState,
    id: JobId,
    answer: Result<Arc<Planned>, PreviewFailure>,
) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    let After::Pending { job, asked } = &editor.after else {
        return;
    };
    if *job != id {
        return;
    }
    let asked = asked.clone();
    editor.after = match answer {
        Ok(planned) => After::Ready { planned, asked },
        Err(PreviewFailure::Refused(refusal)) => {
            let op = graft_ops::operation::Operation::Rewrite(asked);
            After::Refused(words::status_words(&super::operate::refusal_status(
                &op, &refusal,
            )))
        }
        Err(PreviewFailure::Failed(why)) => After::Failed(why),
    };
}

/// `⏎`'s file list answered.
pub(crate) fn files_listed(state: &mut AppState, id: JobId, answer: Result<Arc<FileList>, String>) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    if let Right::Changes { files, .. } = &mut editor.right
        && *files == Loading::Pending(id)
    {
        *files = match answer {
            Ok(list) => Loading::Ready(list),
            Err(why) => Loading::Failed(why),
        };
    }
}

/// `$EDITOR` came back with the whole message (`Ctrl+E` in the field): it
/// becomes the row's message, as `⏎` in the field would, and replans.
pub(crate) fn message_returned(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<String, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    let Some(mut editor) = state.rewrite.editor.take() else {
        return;
    };
    let ours = editor
        .field
        .as_ref()
        .is_some_and(|field| field.handover == Some(id));
    if ours {
        match answer {
            Ok(text) => {
                let message = format!("{}\n", text.trim_end_matches('\n'));
                keys::keep_message(state, &mut editor, message, commands, next_id);
            }
            Err(why) => {
                if let Some(field) = editor.field.as_mut() {
                    field.handover = None;
                }
                editor.notice = Some(why);
            }
        }
    }
    state.rewrite.editor = Some(editor);
}

/// A job ended with no answer (cancelled, or failed as a job): whatever
/// waited on it stops waiting, in words where it was the user's.
pub(crate) fn job_ended(state: &mut AppState, id: JobId, why: &str) {
    if state.rewrite.stale.remove(&id) {
        return;
    }
    if state.rewrite.opening == Some(id) {
        state.rewrite.opening = None;
        state.rewrite.opening_from = None;
        if matches!(state.status, Status::Waiting(_)) {
            state.status = Status::Ready;
        }
        return;
    }
    let Some(editor) = state.rewrite.editor.as_mut() else {
        return;
    };
    if matches!(&editor.after, After::Pending { job, .. } if *job == id) {
        editor.after = After::Failed(why.to_owned());
    }
    if let Right::Changes { files, .. } = &mut editor.right
        && *files == Loading::Pending(id)
    {
        *files = Loading::Failed(why.to_owned());
    }
    if let Some(field) = editor.field.as_mut()
        && field.handover == Some(id)
    {
        field.handover = None;
        editor.notice = Some(why.to_owned());
    }
}

/// The write `y` launched answered (`operate::operated`, first): a refusal
/// brings the editor back with the words on its notice row (R1 "Refused at
/// apply: stays in the editor"); anything else — done, stopped, failed —
/// ends it.
pub(crate) fn write_answered(state: &mut AppState, answer: &Operated) {
    let Some(mut editor) = state.rewrite.launched.take() else {
        return;
    };
    if let Operated::Failed {
        refused: Some(refusal),
        ..
    } = answer
    {
        let op = match &editor.after {
            After::Ready { asked, .. } => graft_ops::operation::Operation::Rewrite(asked.clone()),
            _ => graft_ops::operation::Operation::Rewrite(editor.opened.clone()),
        };
        editor.notice = Some(words::status_words(&super::operate::refusal_status(
            &op, refusal,
        )));
        state.rewrite.editor = Some(editor);
    }
}

/// Close the editor: nothing written; its jobs cancelled, their answers
/// dropped when they come.
fn close(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(editor) = state.rewrite.editor.take() else {
        return;
    };
    for job in editor.jobs() {
        // The `$EDITOR` handover is the loop's, not a job: nothing to
        // cancel, but its answer is still dropped.
        if editor.field.as_ref().and_then(|field| field.handover) != Some(job) {
            commands.push(Command::CancelJob(job));
        }
        state.rewrite.stale.insert(job);
    }
}
