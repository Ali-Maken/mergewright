//! The harness's key prelude (m2-details-and-diffs/23): keys a gallery stop
//! presses for the user before its first frame — `Enter` to open the pane,
//! `x j j` to mark a compare, `/query⏎ Enter` to open a searched-for row.
//!
//! Fed one key at a time, each only while the state is *settled*: the first
//! history window is on hand, the refs answer has landed, and nothing an
//! earlier key started is still in flight (the radar context, a search, the
//! pane's file list or diff). So `Enter` opens on a real row and not on
//! nothing, `j` after it moves the files cursor and not an empty list, and a
//! second `Enter` after a search waits for the hit. Empty in the shipped
//! binary, so `update` never feeds anything there.

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
    let window = state
        .history
        .as_ref()
        .is_some_and(|history| history.window.is_some());
    let refs = state.refs.is_some();
    let quiet = !matches!(state.radar.context, Loading::Pending(_))
        && state
            .search
            .as_ref()
            .is_none_or(|search| search.job.is_none())
        && !matches!(state.detail.files, Loading::Pending(_))
        && !matches!(state.detail.diff, Loading::Pending(_));
    window && refs && quiet
}
