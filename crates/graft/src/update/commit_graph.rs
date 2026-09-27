//! The commit-graph offer's own share of [`super::update`] (m1-the-graph/54,
//! ADR-0008): deciding whether to offer, and the `B`/`X` state machine.

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;

use super::events::{AppEvent, Command, CommitGraphOffer, Product};
use super::state::{AppState, Status};

/// The repository's facts are in: decide whether to offer a commit-graph.
/// Only from a state nothing has happened in yet — a refresh never re-offers
/// after a write, a failure or a decline this run.
pub(super) fn decide_commit_graph_offer(next: &mut AppState) {
    use crate::repository::CommitGraphFact;
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
        CommitGraphFact::Absent { declined: false } if next.harness.offer_commit_graph => {
            CommitGraphOffer::Offered
        }
        CommitGraphFact::Absent { declined: false } => CommitGraphOffer::Suppressed,
    };
}

/// `B` builds, `X` declines — only while the offer can be acted on.
pub(in crate::update) fn commit_graph_key(next: &mut AppState, key: KeyPress) -> Option<Command> {
    if key.modifiers.control || key.modifiers.alt || !next.commit_graph_actionable() {
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
    result: Result<graft_git::commit_graph::Written, String>,
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
        AppEvent::Core(_) | AppEvent::Progress(..) | AppEvent::Produced(..) => {}
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
