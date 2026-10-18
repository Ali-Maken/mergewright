//! The commit-graph offer's own share of [`super::update`] (m1-the-graph/54,
//! ADR-0008): deciding whether to offer, and the `B`/`X` state machine.
//!
//! The offer shows only after this run's topological sort took
//! [`SLOW_SORT`] or more (m7-power-tools/15, :778): a file that speeds up a
//! sort nobody waited for is a question with no point. The repository's
//! facts alone never offer; they only take an offer away.

use core::time::Duration;

use crate::repository::CommitGraphFact;
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::job::JobId;

use super::events::{AppEvent, Command, CommitGraphOffer, Product};
use super::state::{AppState, Status};

/// A sort this slow is one the user waited for.
pub(crate) const SLOW_SORT: Duration = Duration::from_millis(300);

/// Whether a sort that took `sort_took`, in a repository whose commit-graph
/// is `fact`, is worth offering a file for. Pure, so tests feed the time.
pub(crate) fn offer_after_sort(fact: &CommitGraphFact, sort_took: Duration) -> bool {
    matches!(fact, CommitGraphFact::Absent { declined: false }) && sort_took >= SLOW_SORT
}

/// This run's sort was taken up after `took`: offer, if it was slow and
/// nothing has happened to the offer yet. A sort that lands before the
/// repository's facts offers nothing — there is no fact to judge by.
pub(super) fn sort_landed(next: &mut AppState, took: Duration) {
    if next.commit_graph != CommitGraphOffer::NotNeeded {
        return;
    }
    let Some(facts) = &next.repository else {
        return;
    };
    if offer_after_sort(&facts.commit_graph, took) {
        next.commit_graph = if next.harness.offer_commit_graph {
            CommitGraphOffer::Offered
        } else {
            CommitGraphOffer::Suppressed
        };
    }
}

/// The offer's `B`/`X` answer: it can be acted on, and the graph has the
/// keyboard. In the detail pane `B` is blame (keymap §2.5,
/// m7-power-tools/36), so there the offer row is drawn without its keys.
/// The one predicate the key path and the row both read.
pub(crate) fn offer_keys_live(state: &AppState) -> bool {
    state.commit_graph_actionable() && state.graph_has_focus()
}

/// The repository's facts are in: take the offer away if the facts say
/// there is nothing to offer. Only from a state nothing has happened in yet
/// — a refresh never re-offers after a write, a failure or a decline this
/// run. Offering is [`sort_landed`]'s.
pub(super) fn decide_commit_graph_offer(next: &mut AppState) {
    if !matches!(
        next.commit_graph,
        CommitGraphOffer::NotNeeded | CommitGraphOffer::Offered
    ) {
        return;
    }
    let Some(facts) = &next.repository else {
        return;
    };
    next.commit_graph = match &facts.commit_graph {
        CommitGraphFact::Present { .. }
        | CommitGraphFact::Shallow
        | CommitGraphFact::Absent { declined: true } => CommitGraphOffer::NotNeeded,
        CommitGraphFact::Unreadable(why) => {
            next.status = Status::Failed(why.clone());
            CommitGraphOffer::NotNeeded
        }
        // Unchanged: an offer a slow sort made stands; none is made here.
        CommitGraphFact::Absent { declined: false } => next.commit_graph,
    };
}

/// `B` builds, `X` declines — only while the offer can be acted on and the
/// graph has the keyboard ([`offer_keys_live`]). Otherwise nothing is
/// claimed, and the key is swallowed as any unbound key is.
pub(in crate::update) fn commit_graph_key(next: &mut AppState, key: KeyPress) -> Option<Command> {
    if key.modifiers.control || key.modifiers.alt || !offer_keys_live(next) {
        return None;
    }
    match key.code {
        KeyCode::Char('B') => {
            next.commit_graph = CommitGraphOffer::Writing(None);
            Some(Command::WriteCommitGraph)
        }
        KeyCode::Char('X') => {
            next.commit_graph = CommitGraphOffer::Declining(None);
            Some(Command::DeclineCommitGraph)
        }
        _ => None,
    }
}

/// The runner announced the write or decline job: remember its id.
fn commit_graph_started(next: &mut AppState, id: JobId) {
    next.commit_graph = match next.commit_graph {
        CommitGraphOffer::Writing(None) => CommitGraphOffer::Writing(Some(id)),
        CommitGraphOffer::Declining(None) => CommitGraphOffer::Declining(Some(id)),
        other => other,
    };
}

/// The write job answered.
fn commit_graph_written(
    next: &mut AppState,
    id: JobId,
    result: Result<mergewright_git::commit_graph::Written, String>,
) {
    if next.commit_graph != CommitGraphOffer::Writing(Some(id)) {
        return;
    }
    next.commit_graph = match result {
        Ok(written) => CommitGraphOffer::Written {
            commits: written.commits,
        },
        Err(why) => {
            next.status = Status::Failed(why);
            CommitGraphOffer::Failed
        }
    };
}

/// The decline job answered.
fn commit_graph_declined(next: &mut AppState, id: JobId, result: Result<(), String>) {
    if next.commit_graph != CommitGraphOffer::Declining(Some(id)) {
        return;
    }
    next.commit_graph = match result {
        Ok(()) => CommitGraphOffer::Declined,
        Err(why) => {
            next.status = Status::Failed(format!("could not remember the choice: {why}"));
            CommitGraphOffer::Offered
        }
    };
}

/// A commit-graph offer job was announced or answered; `answer` routes only
/// those events here.
pub(super) fn commit_graph_event(next: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::Started(id, _) => commit_graph_started(next, id),
        AppEvent::Produced(id, Product::CommitGraph(result)) => {
            commit_graph_written(next, id, result);
        }
        AppEvent::Produced(id, Product::CommitGraphDeclined(result)) => {
            commit_graph_declined(next, id, result);
        }
        // Not an offer event: `answer`'s pattern keeps these from arriving.
        AppEvent::Core(_) | AppEvent::Progress(..) | AppEvent::Produced(..) | AppEvent::Rested => {}
    }
}

/// `id` is the commit-graph write or decline still wanted.
pub(super) fn commit_graph_job(next: &AppState, id: JobId) -> bool {
    matches!(
        next.commit_graph,
        CommitGraphOffer::Writing(Some(job)) | CommitGraphOffer::Declining(Some(job)) if job == id
    )
}

/// The write or decline job ended without answering: it broke, or it was
/// dropped before git started (shutdown). A broken write is a failure the
/// user is told about; one that never began leaves the offer standing.
pub(super) fn commit_graph_job_ended(next: &mut AppState, event: Event) {
    let writing = matches!(next.commit_graph, CommitGraphOffer::Writing(_));
    match event {
        Event::JobFailed(id, failure) => {
            next.status = Status::Failed(format!("{id} failed: {failure}"));
            next.commit_graph = if writing {
                CommitGraphOffer::Failed
            } else {
                CommitGraphOffer::Offered
            };
        }
        _ => next.commit_graph = CommitGraphOffer::Offered,
    }
}
