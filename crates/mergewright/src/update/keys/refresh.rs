//! `Ctrl+R`: re-read the repository from disk.

use mergewright_core::job::JobId;
use mergewright_git::history::Order;

use super::super::events::Command;
use super::super::state::AppState;

/// `r`: refresh the repository from disk, cancelling whatever walk was
/// already in flight — it would otherwise sit waiting for row requests that
/// now go to its replacement.
///
/// Also `super::super::operate`'s `Refresh::Everything`: a write that moved refs
/// leaves exactly what `Ctrl+R` rebuilds stale, and two hand-written lists of
/// what "everything" means drift (m3/04 review M8).
/// `next_id` rather than `next.local_jobs`: a caller that borrowed the
/// counter out into a closure writes its copy back when it returns, so a
/// callee minting from the state's own counter has its ids rewound and the
/// next mint reissues an id still in flight — a job answering under another
/// job's name (m3-working-tree-undo-and-stash/06). One counter per pass.
pub(in crate::update) fn refresh_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The journal is part of "re-read the repository from disk": another
    // Mergewright on the same worktree appends to it, and an entry this run has
    // not seen is one `u` would not offer to take back
    // (m3-working-tree-undo-and-stash/06).
    commands.extend(next.journal.read(next_id));
    // And so is the stash list, while the overlay is up: a stash push or
    // pop in another window moves `refs/stash` under it
    // (m3-working-tree-undo-and-stash/06).
    super::super::stash::refresh(next, commands, next_id);
    commands.extend(next.detail.refresh(next_id));
    // The walk this refresh replaces is cancelled by name — and the name is
    // one `update` minted when it asked for the walk, not one it learned
    // from the pool's `Started` event. Learning it late left a window in
    // which a second `Ctrl+R` had nothing to cancel, so the walk it meant to
    // replace stayed parked on a worker; two of those fill the pool and the
    // refs and working-tree reads never get a thread
    // (m5-history-and-ghost-preview/05).
    if let Some(id) = next.awaiting.take() {
        commands.push(Command::CancelJob(id));
    }
    if let Some(id) = next.promoting.take() {
        commands.push(Command::CancelJob(id));
    }
    let walk = next_id();
    next.awaiting = Some(walk);
    commands.push(Command::OpenRepository);
    next.facts_pending = true;
    commands.push(Command::ReadHistory(walk, Order::Topological));
    let refs = next_id();
    next.refs_read.asked(refs);
    commands.push(Command::ReadRefs(refs));
    super::super::remote::request(next, commands, next_id);
    let tree = next_id();
    commands.push(super::super::status_read::ask(next, tree));
    // The default branch's tip may have moved: a stale chain would
    // misclassify a foxtrot merge against a tip that is no longer real.
    // `radar::request_context` asks for a fresh one once the refreshed refs
    // land.
    super::super::radar::reset_for_refresh(next);
}
