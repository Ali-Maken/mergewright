//! Facts beside the graph arriving (system git, refs, working tree counts),
//! the failures of the reads that bring them, and the name prompt's claim on a key.

use graft_core::event::KeyPress;

use super::tree_answers::{stash_entries_read, working_tree_answered};
use super::{AppState, Command, Product, Quit, Status, banner, clash, name_prompt, palette, state};

/// A fact about the repository beside the graph arrived: the system git
/// check, the refs, or the working tree counts. `answer` routes only those.
#[expect(
    clippy::too_many_lines,
    reason = "the not-a-fact arm names every other `Product`, exhaustively and with no \
              catch-all, for the same reason `answer` does"
)]
pub(super) fn took_fact(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
    product: Product,
) {
    match product {
        Product::SystemGit(check) => {
            next.system_git = Some(check);
        }
        Product::Refs(Ok(refs)) => {
            next.refs = Some(std::sync::Arc::new(refs));
            next.refs_read.answered(id, true);
            // A finished `--continue` waits on this read for the new
            // `HEAD` (m6-conflicts/12); any other read passes it by.
            banner::refs_answered(next, commands, id);
            // `⌃P` with a query was waiting on these for TAGS (m8/12).
            palette::refs_landed(next, commands);
            // The stash list is one of a clash's facts (m6-conflicts/72b).
            let mut local_jobs = next.local_jobs;
            clash::refresh_and_land(next, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
            stash_entries_read(next, commands);
        }
        Product::Refs(Err(failure)) => refs_failed(next, id, failure),
        Product::WorkingTree(answer) => working_tree_answered(next, commands, id, answer),
        // Not a fact: `answer`'s pattern keeps these from arriving.
        Product::Repository(_)
        | Product::Resumed(_)
        | Product::Mergetool { .. }
        | Product::ContinuedInGit(_)
        | Product::TopologicalOrder(_)
        | Product::CommitGraph(_)
        | Product::CommitGraphDeclined(_)
        | Product::Files(_)
        | Product::FileDiff(..)
        | Product::Blame(_)
        | Product::Trace(_)
        | Product::WholeFile(_)
        | Product::ContainedIn(..)
        | Product::RadarContext(_)
        | Product::EvilProbe(..)
        | Product::ForcePushProbe(..)
        | Product::DuplicatesProbe(_)
        | Product::StashOnly(_)
        | Product::Map(_)
        | Product::Operated(_)
        | Product::Cloned(_)
        | Product::Undone(_)
        | Product::Journal(_)
        | Product::Outside(_)
        | Product::TourProgress(_)
        | Product::LostFound(_)
        | Product::RecoverFor(_)
        | Product::Preview(_)
        | Product::RevertedTo(_)
        | Product::Stashes(_)
        | Product::SwitchList(_)
        | Product::PaletteBranches(_)
        | Product::PaletteNames(_)
        | Product::TrackedPaths(_)
        | Product::PaletteCommits(_)
        | Product::IndexState(_)
        | Product::Remotes(_)
        | Product::Leaves { .. }
        | Product::FoldedCounted { .. }
        | Product::CommitTemplate(_)
        | Product::HeadMessage(_)
        | Product::CommitFiles(_)
        | Product::RewritePlanned(_)
        | Product::RowTagsResolved(_)
        | Product::RewriteOpened(_)
        | Product::ExternalEditor(_)
        | Product::HandedOver(_)
        | Product::ConflictOpened(_)
        | Product::Taken(_)
        | Product::Untaken(_)
        | Product::BaseLoaded(_)
        | Product::ConflictSet(_)
        | Product::FinishSummary(_)
        | Product::FinishDiff(_) => {}
    }
}

/// The refs read failed. The graph still draws without badges; the reason
/// goes on the status line rather than leaving them silently missing.
pub(super) fn refs_failed(next: &mut AppState, id: graft_core::job::JobId, failure: String) {
    next.status = Status::Failed(failure);
    next.refs_read.answered(id, false);
    // The read a finished `--continue` waited on has no `HEAD` to give:
    // the landing is dropped (the failure is on screen) rather than spent
    // on the stale refs still held.
    if next.head_landing.refs == Some(id) {
        next.land_on_head = false;
        next.head_landing = state::HeadLanding::default();
    }
}

/// A repository failed to open, or to re-open.
pub(super) fn repository_open_failed(next: &mut AppState, failure: String) {
    if next.repository.is_none() {
        // Nothing was ever opened, so there is no screen to show this on and
        // nothing for the user to do with Graft. Leaving, with the reason,
        // beats an empty screen with an error in a corner.
        next.quit = Some(Quit::Fatal(failure));
    } else {
        // A refresh of a repository that is already open failed — the
        // directory went away mid-run, say. The screen still has something to
        // show, so this goes on the status line.
        next.status = Status::Failed(failure);
    }
}

/// The name prompt's turn at a key, right after the credential prompt's and
/// before the search prompt's: a one-field modal that claims every key but
/// `Ctrl+C`, so a branch named `search` cannot open the search prompt
/// halfway through being typed (m3/06 review, Critical 1).
///
/// `local_jobs` is borrowed out because the closure and `next` cannot
/// overlap.
pub(super) fn name_prompt_claimed(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = name_prompt::name_prompt_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}
