//! Landing on an operation git is part-way through
//! (m3-working-tree-undo-and-stash/06, m6-conflicts/06, m6-conflicts/12):
//! the banner takes the keyboard, the stop is recorded as a state, and the
//! changes pane opens where the work is.
//!
//! **Keyed on git, not the journal** (m6-conflicts/12). The launch used to
//! need a journaled live entry, so a merge begun in the shell — which writes
//! none — opened on the graph with the banner unfocused and the resolver
//! several keys away (`.scratch/m6-conflicts/scout-stuck-after-merge.md`,
//! path B). Now the first working-tree read that finds a focusable
//! operation lands, whatever the journal holds; the journal only decides
//! whose operation it is ([`LiveOrigin`]).

use graft_core::job::JobId;

use super::events::Command;
use super::state::{AppState, Stop};

/// Whose operation the banner is standing over — the one fact the stop's
/// words need that git's own state does not carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveOrigin {
    /// An operation this run started, which stopped.
    ThisRun,
    /// A journaled operation a previous run started and never finished.
    PreviousRun,
    /// Git is part-way through something no journal entry names: begun in
    /// the shell, or by another tool.
    OutsideGraft,
}

/// Land on the operation git is part-way through: record the stop, hand the
/// banner the keyboard, and open the changes pane on the work.
///
/// Called from the **first** working-tree read (`update::took_fact`) and
/// the **first** journal read (`update::journal`), and from the first
/// working-tree read after a modal closed while the landing was owed. A
/// conflict that happens later in the session is `update::operate`'s to
/// announce.
///
/// While a modal owns the keys it records the stop only and returns
/// `false`: grabbing the keyboard out from under a commit message or a
/// confirm box would be worse than landing a read later
/// ([`AppState::live_landing_owed`]). `true` when it landed.
///
/// The launch leaves `Status` alone: at launch nothing of the operation's is
/// on it, and anything that is (a failed refs read) is a real problem the
/// stop must not wipe.
pub(super) fn land_on_live_operation(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    origin: LiveOrigin,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(kind) = state.in_progress.as_ref().map(|detail| detail.kind) else {
        return false;
    };
    // A bisect has no banner to focus, so there is nothing to hand the
    // keyboard to and no `c`/`a` to offer.
    if !super::banner::banner_focusable(kind) {
        return false;
    }
    let paths = super::banner::unmerged_paths(state);
    // An earlier stop of the same kind keeps its own words (a `ThisRun`
    // stop that a re-read reaches here after must not turn into "from a
    // previous run").
    if state.stopped.as_ref().is_none_or(|stop| stop.kind != kind) {
        state.stopped = Some(Stop {
            origin,
            again: false,
            kind,
            paths_at_stop: paths.len(),
        });
    }
    if state.modal_owns_keys() {
        state.live_landing_owed = true;
        return false;
    }
    state.live_landing_owed = false;
    state.banner_focus = true;
    land_on_the_work(state, commands, paths.into_iter().next(), next_id);
    true
}

/// The first journal read landed: it is the fact that turns
/// [`LiveOrigin::OutsideGraft`] into [`LiveOrigin::PreviousRun`] when the
/// working-tree read got there first, and it lands the operation itself when
/// the working-tree read has come in but nothing has landed yet.
pub(super) fn journal_read_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let journaled = !state.live_entries.is_empty();
    match state.stopped.as_mut() {
        Some(stop) if stop.origin == LiveOrigin::OutsideGraft && journaled => {
            stop.origin = LiveOrigin::PreviousRun;
        }
        None if state.working_tree.is_some() => {
            let origin = if journaled {
                LiveOrigin::PreviousRun
            } else {
                LiveOrigin::OutsideGraft
            };
            land_on_live_operation(state, commands, origin, next_id);
        }
        // Already landed with the right origin, or the working-tree read is
        // still out and lands when it comes.
        Some(_) | None => {}
    }
}

/// A working-tree read came in with an operation live. Lands it when this
/// is the first read, or when a modal made the launch owe its landing and
/// has since closed.
pub(super) fn working_tree_read_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    first_read: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    if first_read {
        let origin = if state.live_entries.is_empty() {
            LiveOrigin::OutsideGraft
        } else {
            LiveOrigin::PreviousRun
        };
        land_on_live_operation(state, commands, origin, next_id);
    } else if state.live_landing_owed && !state.modal_owns_keys() {
        let origin = state
            .stopped
            .as_ref()
            .map_or(LiveOrigin::OutsideGraft, |stop| stop.origin);
        land_on_live_operation(state, commands, origin, next_id);
    }
}

/// The changes pane, open, its cursor put where [`land_on_live_operation`]
/// says: the first conflicted file under CONFLICTS — or, with nothing left
/// conflicted, STAGED, which is what `c` would commit (design note
/// Decision 2a). One `⏎` from the banner opens the resolver; the landing
/// itself opens nothing and writes nothing.
///
/// The cursor goes through [`KeptCursor`] — the same record a refresh uses
/// to put the user back — so it lands when the list arrives, not before:
/// `first` is a path, the list is a job.
///
/// [`KeptCursor`]: super::detail::KeptCursor
pub(in crate::update) fn land_on_the_work(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    first: Option<std::path::PathBuf>,
    next_id: &mut impl FnMut() -> JobId,
) {
    use graft_ui::detail::WorkingTreeSection;
    // The pane follows the selection at the tail of every update, so the
    // selection has to be the working-tree row or the pane would be pulled
    // straight back onto a commit.
    state.on_working_tree = true;
    let (path, section) = match first {
        Some(path) => (
            path.into_os_string().into_encoded_bytes(),
            WorkingTreeSection::Unmerged,
        ),
        // No path: "the same place down that section" is its first file.
        None => (Vec::new(), WorkingTreeSection::Staged),
    };
    let spec = super::detail::DetailSpec::WorkingTree;
    if state.detail.shown && state.detail.spec == Some(spec) {
        // Already open on it (a user who was quicker than the second
        // launch read): a refresh is what carries a kept cursor.
        commands.extend(state.detail.refresh(next_id));
    } else {
        commands.extend(state.detail.open(spec, next_id));
    }
    state.detail.kept_cursor = Some(super::detail::KeptCursor {
        path,
        section: Some(section),
        position: 0,
        body: 0,
    });
}

#[cfg(test)]
mod tests;
