//! `a` on an older commit, and absorb from the `.` menu: put the staged
//! changes into the commit they belong to (m7-power-tools/09, note R2).
//!
//! Three routes, all ending in the Ghost Preview and `y`:
//! - `a` on a commit row of his branch that is not `HEAD` → plan
//!   `AmendInto { target }` off-thread → the "put it in" preview.
//! - `a` on `HEAD`'s row (or the uncommitted row, or the changes pane):
//!   today's amend, unless another local branch exists — then a job asks
//!   whether anything is built on his branch first. Nothing is → today's
//!   amend; something is → the preview, which carries it along.
//! - `.` → absorb on the uncommitted row → plan `Absorb` → its preview.
//!
//! The words (box title, extra rows, landing) are worked out by the plan
//! job and kept in [`AmendInto`] until the landing, so an `R`-editor
//! rewrite that happens to run later never picks them up: [`landed`] takes
//! them only for the very operation that was planned.

use graft_ops::operation::{Operation, Refusal};
use graft_ops::rewrite::{RewriteParams, RewriteRequest};
use graft_ui::graph_screen::RowTag;

use crate::jobs::rewrite::{BoxWords, DoneWords, PlannedPutIn, RewritePlanned};
use graft_core::oid::Oid;

use super::events::{Command, Product};
use super::preview::{PreviewSubject, open_preview};
use super::state::{AppState, Status};
use graft_core::job::JobId;

/// What a put-in is waiting on, and what it planned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AmendInto {
    /// The plan job in flight, and the key that asked (`a` or `.`): a
    /// second `a` or absorb starts nothing.
    pending: Option<(JobId, &'static str)>,
    /// The plan the preview on screen draws, until it lands or closes.
    asked: Option<Asked>,
    /// The read that finds the changed commits' new rows after a landing.
    tagging: Option<JobId>,
    /// The `HEAD`-message read asked for a reword with a line built on it
    /// (`RewritePlanned::RewordHead`): the editor it opens plans a rewrite.
    reword_read: Option<JobId>,
}

/// A planned put-in, kept from the plan's answer to its landing.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Asked {
    params: RewriteParams,
    offsets: Vec<u32>,
    words: DoneWords,
    shown: BoxWords,
    /// Tracked files had unstaged edits when the plan answered — the tree
    /// the preview's `your edits: N files set aside` row counts too.
    unstaged: bool,
}

impl AmendInto {
    /// The box words for `op`, when `op` is the put-in this planned.
    #[must_use]
    pub(crate) fn shown_for(&self, op: &Operation) -> Option<&BoxWords> {
        let asked = self.asked.as_ref()?;
        is_put_in(op, &asked.params).then_some(&asked.shown)
    }

    /// A plan in flight, for tests that need `a` waiting on one.
    #[cfg(test)]
    pub(crate) fn pending_for_test(job: JobId) -> Self {
        Self {
            pending: Some((job, "a")),
            ..Self::default()
        }
    }

    /// Whether the `HEAD`-message read `id` was asked for a reword with a
    /// line built on it; taken, so it answers once.
    pub(crate) fn reword_read_answered(&mut self, id: JobId) -> bool {
        self.reword_read.take_if(|asked| *asked == id).is_some()
    }

    /// Whether a plan is being worked out.
    #[must_use]
    pub fn checking(&self) -> bool {
        self.pending.is_some()
    }
}

fn is_put_in(op: &Operation, params: &RewriteParams) -> bool {
    matches!(op, Operation::Rewrite(launched) if launched == params)
}

/// The landing's put-back clause for a put-in: the staged edits went into
/// commits, so only the unstaged ones are "back" (09b Q3).
const UNSTAGED_BACK: &str = "your unstaged edits are back";

const FINDING_HOMES: &str = "checking where your staged changes go\u{2026}";

/// `a` in the graph or the changes pane.
pub(crate) fn a_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if refused_by_live_operation(state) {
        return;
    }
    if waiting_or_editing(state) {
        return;
    }
    match older_target(state) {
        Some(target) => put_into_older(state, commands, target, next_id),
        None => head_path(state, commands, next_id),
    }
}

/// `a`'s cheap blocks, all three (m7-power-tools/40): an operation is live
/// (P14), an amend is already pending, or the commit editor is open. The
/// bar's `a amend` reads this; [`a_key`] runs the same checks, in order, the
/// live operation first so it can say why.
pub(crate) fn amend_blocked(state: &AppState) -> bool {
    super::commit_editor::live_operation(state).is_some() || waiting_or_editing(state)
}

/// An amend is pending or the commit editor is open: `a` does nothing.
fn waiting_or_editing(state: &AppState) -> bool {
    state.amend_into.pending.is_some() || state.commit_editor.is_some()
}

/// P14 first: an operation is live, and `a` (or absorb) answers as `a`
/// always has (`commit_editor.rs`, m6-conflicts/48). True when refused.
fn refused_by_live_operation(state: &mut AppState) -> bool {
    let Some(live) = super::commit_editor::live_operation(state) else {
        return false;
    };
    let tabs = super::commit_editor::refusal_tabs(state);
    state.status = super::commit_editor::operation_refusal(live, tabs).status();
    true
}

/// The commit under the cursor, when it is a history row that is not
/// `HEAD`'s. `None` sends `a` down the `HEAD` path (ruling 1) — also while
/// refs are unread, when `HEAD`'s row cannot be told from an older one.
fn older_target(state: &AppState) -> Option<Oid> {
    if state.on_working_tree || state.on_changes_pane() {
        return None;
    }
    let row = state.history.as_ref()?.selected()?;
    let head = state.refs.as_deref()?.head();
    (Some(row.commit.id) != head).then_some(row.commit.id)
}

fn put_into_older(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    target: Oid,
    next_id: &mut impl FnMut() -> JobId,
) {
    // Checked here, from what the screen already holds, before any job:
    // a plan of nothing is not worth a spawn.
    let staged = state
        .working_tree
        .as_deref()
        .is_some_and(|tree| tree.staged().next().is_some());
    if !staged {
        state.status = Status::sticky(Refusal::NothingStagedFor { target }.explain());
        return;
    }
    start(
        state,
        commands,
        (RewriteRequest::AmendInto { target }, "a"),
        None,
        FINDING_HOMES.to_owned(),
        next_id,
    );
}

/// `a` on `HEAD`: today's amend unless something may be built on it.
fn head_path(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let refs = state.refs.as_deref();
    let branch = refs.and_then(super::preview::current_branch);
    let others = refs.is_some_and(|refs| refs.branches().any(|(_, _, current, _)| !current));
    // A dependent is a local branch: with no other one, none can exist,
    // and neither can one while `HEAD` is detached (`plan` needs his
    // branch). Refs not yet read: today's amend, as before 09.
    let (Some(branch), true) = (branch, others) else {
        commands.extend(super::commit_editor::open(state, true, next_id));
        return;
    };
    let name = branch
        .name
        .strip_prefix("refs/heads/")
        .unwrap_or(&branch.name)
        .to_owned();
    start(
        state,
        commands,
        (RewriteRequest::AmendInto { target: branch.oid }, "a"),
        Some(branch),
        format!("checking what's built on {name}\u{2026}"),
        next_id,
    );
}

/// The reword editor's `⏎` (m8-polish-and-ship/14): plan `HEAD`'s new
/// `message` with the line built on it, then the preview, as an amend with
/// dependents does. No `head`: the line was already found, and a line gone
/// since is the plan's `PlainAmend` refusal, in words.
pub(crate) fn reword_confirmed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    message: String,
    next_id: &mut impl FnMut() -> JobId,
) {
    start(
        state,
        commands,
        (RewriteRequest::RewordHead { message }, "a"),
        None,
        "planning the reword\u{2026}".to_owned(),
        next_id,
    );
}

/// The `.` menu's absorb item on the uncommitted row.
pub(crate) fn absorb_chosen(
    state: &mut AppState,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    if refused_by_live_operation(state) {
        return commands;
    }
    if state.amend_into.pending.is_none() {
        start(
            state,
            &mut commands,
            (RewriteRequest::Absorb, "."),
            None,
            FINDING_HOMES.to_owned(),
            next_id,
        );
    }
    commands
}

fn start(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    (request, opened_by): (RewriteRequest, &'static str),
    head: Option<graft_ops::operation::Tip>,
    waiting: String,
    next_id: &mut impl FnMut() -> JobId,
) {
    let id = next_id();
    state.amend_into.pending = Some((id, opened_by));
    state.amend_into.asked = None;
    state.status = Status::Waiting(waiting);
    commands.push(Command::PlanRewrite { id, request, head });
}

/// A plan or tag read answered.
pub(crate) fn answer(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    product: Product,
) {
    let mut local_jobs = state.local_jobs;
    let mut next_id = || local_jobs.next_id();
    match product {
        Product::RewritePlanned(answer) => planned(state, commands, id, answer, &mut next_id),
        Product::RowTagsResolved(answer) => tags_resolved(state, id, answer),
        // `update::answer` routes only these two here.
        _ => {}
    }
    state.local_jobs = local_jobs;
}

/// The plan came back: the preview, a refusal in words, or today's amend.
pub(crate) fn planned(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<RewritePlanned, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some((_, opened_by)) = state.amend_into.pending.take_if(|(job, _)| *job == id) else {
        return;
    };
    match answer {
        Ok(RewritePlanned::Plan(put_in)) => {
            let PlannedPutIn {
                params,
                offsets,
                words,
                shown,
            } = *put_in;
            if matches!(state.status, Status::Waiting(_)) {
                state.status = Status::Ready;
            }
            let op = Operation::Rewrite(params.clone());
            let unstaged = has_unstaged(state);
            state.amend_into.asked = Some(Asked {
                params,
                offsets,
                words,
                shown,
                unstaged,
            });
            let opened = open_preview(
                state,
                commands,
                PreviewSubject::Operation {
                    op: Box::new(op),
                    opened_by,
                },
                false,
                next_id,
            );
            if !opened {
                state.amend_into.asked = None;
            }
        }
        Ok(RewritePlanned::Refused(refusal)) => {
            state.status = Status::sticky(refusal.explain());
        }
        Ok(RewritePlanned::NothingBuiltOnHead) => {
            if matches!(state.status, Status::Waiting(_)) {
                state.status = Status::Ready;
            }
            commands.extend(super::commit_editor::open(state, true, next_id));
        }
        Ok(RewritePlanned::RewordHead) => {
            if matches!(state.status, Status::Waiting(_)) {
                state.status = Status::Ready;
            }
            commands.extend(super::commit_editor::open(state, true, next_id));
            state.amend_into.reword_read = state.head_message;
        }
        Err(why) => {
            state.status = Status::sticky(format!("couldn't plan the put-in: {why}"));
        }
    }
}

/// An operation landed (`operate::operated`'s `Done`). When it is the
/// put-in planned here: its own words, and a read for the rows to tag.
pub(crate) fn landed(
    state: &mut AppState,
    op: Option<&Operation>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(op) = op else {
        return;
    };
    let Some(asked) = state
        .amend_into
        .asked
        .take_if(|asked| is_put_in(op, &asked.params))
    else {
        return;
    };
    // The note's words, composed with the put-back's (09b Q3). A put-in
    // always sets edits aside — it has staged ones (`Operation::sets_aside`)
    // — so `done_status`'s plain `your edits are back` would claim back the
    // staged edits that are now in commits. What came back is his unstaged
    // edits, when the planned tree had any. A `Done` means the put-back went
    // through clean; a clash lands elsewhere.
    let unstaged_back = asked.unstaged;
    let also = join(
        asked.words.also.as_deref(),
        unstaged_back.then_some(UNSTAGED_BACK),
    );
    state.status = Status::Done {
        what: asked.words.what.clone(),
        also: (!also.is_empty()).then_some(also),
        keys: Some("u undoes".to_owned()),
    };
    if asked.offsets.is_empty() {
        return;
    }
    let id = next_id();
    state.amend_into.tagging = Some(id);
    commands.push(Command::ResolveRowTags(id, asked.offsets));
}

/// Any tracked file with edits not staged (untracked files are never set
/// aside: they stay where they are).
fn has_unstaged(state: &AppState) -> bool {
    state.working_tree.as_deref().is_some_and(|tree| {
        tree.entries
            .iter()
            .any(|entry| entry.unstaged.is_some() && !entry.untracked)
    })
}

fn join(first: Option<&str>, second: Option<&str>) -> String {
    [first, second]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// The changed commits' new rows came back: tag them, or say why not.
pub(crate) fn tags_resolved(state: &mut AppState, id: JobId, answer: Result<Vec<Oid>, String>) {
    if state.amend_into.tagging.take_if(|job| *job == id).is_none() {
        return;
    }
    match answer {
        Ok(changed) => {
            for commit in changed {
                state.row_tags.set(commit, RowTag::Changed);
            }
        }
        Err(why) => {
            // Rule 3: the landing stands, and the missing tag is said
            // beside it rather than left out silently.
            let missing = format!("couldn't mark the changed commit: {why}");
            state.status = match std::mem::replace(&mut state.status, Status::Ready) {
                Status::Done { what, also, keys } => Status::Done {
                    what,
                    also: Some(join(also.as_deref(), Some(&missing))),
                    keys,
                },
                // The next key already replaced the landing: the failure
                // still has to be seen.
                _ => Status::sticky(missing),
            };
        }
    }
}

/// A write started (`operate::claim_write`): a tag read still out names
/// rows of the history before it, so its answer must tag nothing (review
/// m7-09-to-21c, Minor 2). The tags already drawn go with `row_tags`.
pub(crate) fn write_claimed(state: &mut AppState) {
    state.amend_into.tagging = None;
}

/// A job ended without an answer (cancelled or failed to start): a plan
/// in flight is over, and so is a tag read.
pub(crate) fn job_ended(state: &mut AppState, id: JobId) {
    if state
        .amend_into
        .pending
        .take_if(|(job, _)| *job == id)
        .is_some()
        && matches!(state.status, Status::Waiting(_))
    {
        state.status = Status::Ready;
    }
    if state.amend_into.tagging == Some(id) {
        state.amend_into.tagging = None;
    }
}

/// The preview closed without running: its words go with it.
pub(crate) fn forget(state: &mut AppState) {
    state.amend_into.asked = None;
}

#[cfg(test)]
mod tests;
