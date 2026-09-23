//! The harness's key prelude (m2-details-and-diffs/23): keys a gallery stop
//! presses for the user before its first frame — `Enter` to open the pane,
//! `x j j` to mark a compare, `/query⏎ Enter` to open a searched-for row.
//!
//! Fed one key at a time, each only while the state is *settled*: the first
//! history window, the refs answer and the working-tree status are on hand,
//! and nothing an earlier key started is still in flight (the radar context,
//! a search, the pane's file list or diff, a write and its undo, a journal
//! or stash read, the commit editor's prefill, a Ghost Preview's plan, the
//! resolver's opening read or base load). So
//! `Enter` opens on a real row and not on nothing, `j` after it moves the
//! files cursor and not an empty list, `k` finds the uncommitted row once
//! the status says the tree is dirty, `u` after `Space` undoes the stage
//! that has landed rather than racing it (m3-working-tree-undo-and-stash/07),
//! and `→ ⏎` after `m` confirm a drawn box and not an empty one. Empty in
//! the shipped binary, so `update` never feeds anything there.

use graft_core::event::KeyPress;

use super::detail::Loading;
use super::state::AppState;

/// The next prelude key to feed, taken off the front; `None` while the
/// prelude is empty or the state has something in flight to wait for.
pub(super) fn take_next(next: &mut AppState) -> Option<KeyPress> {
    if next.harness.prelude.is_empty() || !settled(next) {
        return None;
    }
    Some(next.harness.prelude.remove(0))
}

/// Nothing a prelude key would need is still on its way.
fn settled(state: &AppState) -> bool {
    // A credential prompt is a text field waiting for its next key: that is
    // "ready", though the write that asked is still in flight — waiting for
    // `operating` to clear would deadlock on the answer only a key can give.
    if state.credential.is_some() {
        return true;
    }
    let window = state
        .history
        .as_ref()
        .is_some_and(|history| history.window.is_some());
    let refs = state.refs.is_some();
    let status = state.working_tree.is_some();
    let quiet = !matches!(state.radar.context, Loading::Pending(_))
        && state
            .search
            .as_ref()
            .is_none_or(|search| search.job.is_none())
        && !matches!(state.detail.files, Loading::Pending(_))
        && !matches!(state.detail.diff, Loading::Pending(_));
    // M3's writes and reads: a key pressed while one of these is in flight
    // would act on the state before it, which is exactly the race a capture
    // must not show.
    let writes_landed = state.operating.is_none()
        && state.journal.reading.is_none()
        && !matches!(state.stash.entries, Loading::Pending(_))
        && state.head_message.is_none()
        && state.commit_template.is_none();
    // M5's Ghost Preview: the box a key opened is planned by a job. `→ ⏎`
    // fed before the plan lands would act on a box with no plan — close it
    // — and a capture would show a graph with no box
    // (m5-history-and-ghost-preview/11).
    let planned = state
        .preview
        .as_ref()
        .is_none_or(|preview| !matches!(preview.plan, Loading::Pending(_)));
    // M6's resolver: its opening read and a `B` base load are jobs a key
    // started. `o`/`t`/`b` fed before the read lands is refused — a capture
    // would show the refusal, not the take (m6-conflicts/07 F6). Its
    // `staging` and `restarting` writes are `operating`, covered above.
    let resolver_read = state
        .resolver
        .as_ref()
        .is_none_or(|open| open.job.is_none() && open.base_job.is_none());
    window && refs && status && quiet && writes_landed && planned && resolver_read
}
