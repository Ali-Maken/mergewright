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
use graft_ops::operation::RebaseParams;
use graft_ops::rewrite::{
    RewriteParams, RewriteRequest, SquashGroup, TodoAction, TodoLine, TodoProblem, squash_groups,
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
mod open;
mod routing;
#[cfg(test)]
pub(crate) mod tests;
mod words;

use open::cancel_opening;
#[cfg(test)]
pub(crate) use open::opens_it;
pub(crate) use open::{field_open, open_rewrite, open_rewrite_from_rebase, opened};
pub(crate) use routing::{
    after_planned, files_listed, job_ended, message_returned, owns, write_answered,
};
pub(crate) use words::{AfterFacts, RowFacts, ViewFacts};
#[cfg(test)]
pub(crate) use words::{melds_into, nothing_kept_yet};

pub(crate) use keys::{rewrite_key, rewrite_mouse};

/// The editor's place in [`AppState`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RewriteState {
    /// The open read in flight.
    pub(crate) opening: Option<JobId>,
    /// Where `R` was pressed, for the cursor and the marks' reason once the
    /// read answers.
    opening_from: Option<OpenedFrom>,
    /// The editor on screen: `Some` exactly while `GraphOverlay::Rewrite`.
    pub(crate) editor: Option<Box<RewriteEditor>>,
    /// After `y`, until the write answers (ruling 6).
    pub(crate) launched: Option<Box<RewriteEditor>>,
    /// Jobs a closed or superseded editor left in flight: their answers
    /// (or their cancellation) still arrive, and are dropped here rather
    /// than reaching a handler that shares their product.
    stale: HashSet<JobId>,
}

/// Where the editor was opened from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenedFrom {
    /// `R` on the graph, on this row: the cursor opens on it.
    Row(Oid),
    /// `R` in a rebase preview (rebase-preview-keeps-drops): the rows are
    /// that rebase's, the suggested drops pre-marked; the cursor opens on
    /// the newest of them.
    Rebase,
}

/// Why a row came up already marked, before any key of his.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkedBy {
    /// A `fixup!`/`squash!` subject (autosquash): `marked by its name`.
    Name,
    /// A rebase preview's suggested drop, already in the base:
    /// `already in <base> · p keeps it`.
    AlreadyIn,
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
    /// Rows that came up marked (a `fixup!`/`squash!` subject, or a rebase's
    /// suggested drop) that he has not re-marked since, and why. Any mark
    /// of his removes the row: the words are for marks he did not make.
    marked_by: HashMap<Oid, MarkedBy>,
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
    /// A todo that can run, not yet asked for: drawn as checking. Only
    /// between `new` and `opened`'s first `replan` (30n m5), so the editor
    /// never claims a state it did not read.
    NotAsked,
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

impl RewriteEditor {
    fn new(opened: OpenedRewrite, from: OpenedFrom) -> Self {
        let OpenedRewrite {
            params,
            whole,
            base,
        } = opened;
        let todo = params.todo.clone();
        let reason = match from {
            OpenedFrom::Row(_) => MarkedBy::Name,
            OpenedFrom::Rebase => MarkedBy::AlreadyIn,
        };
        let marked_by = todo
            .iter()
            .filter(|row| row.action != TodoAction::Pick)
            .map(|row| (row.commit, reason))
            .collect();
        let at = match from {
            OpenedFrom::Row(from) => todo.iter().rposition(|row| row.commit == from),
            // The newest suggested drop: `p` keeps it in one key.
            OpenedFrom::Rebase => todo.iter().rposition(|row| row.action == TodoAction::Drop),
        };
        let cursor = at.map_or(0, |index| todo.len() - 1 - index);
        // What its own todo says: its problem, else not yet asked.
        let after = match graft_ops::rewrite::validate(&todo) {
            Ok(()) => After::NotAsked,
            Err(problem) => After::Invalid(problem),
        };
        RewriteEditor {
            opened: params,
            whole,
            base,
            todo,
            marked_by,
            moved: HashSet::new(),
            edits: HashMap::new(),
            cursor,
            field: None,
            after,
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
    /// `keys` spells the keys its words name (m8/10i).
    pub(crate) fn view_facts(&self, keys: &graft_ui::keymap::Keymap) -> ViewFacts<'_> {
        words::view_facts(self, keys)
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
