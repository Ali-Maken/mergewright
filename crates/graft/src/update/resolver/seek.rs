//! Back to the first **open** clash (ease-pass/05c, Q7 a): the paused
//! banner's `⏎` opens the resolver on git's first conflicted path, and
//! that file may already be solved — picked before the pause, or on disk
//! before a relaunch. Once its read and the set's have landed, a file
//! with no clash open hands on to the next one a clash may be open in.

use std::path::PathBuf;

use graft_core::job::JobId;

use super::files::{RowState, open_state, remember};
use super::{OpenedFrom, open_resolver, reopen};
use crate::update::events::Command;
use crate::update::state::AppState;

/// The banner's `⏎`: the resolver on `first`, opened from the banner, set
/// to hand on to the first open clash ([`seek_first_open`]).
pub(crate) fn open_on_first_open(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    first: PathBuf,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The returned bool says only that the key was spent; a refusal is
    // worded on the status line by `open_resolver` itself.
    open_resolver(state, commands, first, OpenedFrom::Banner, next_id);
    if let Some(open) = state.resolver.as_mut() {
        open.seek_open = true;
    }
}

/// A key or a click in the resolver ends a pending seek: the seek is the
/// landing, and once he has acted — a pick before the reads landed, say —
/// the file on screen is his, not a stop on the way to another.
pub(crate) fn he_acted(state: &mut AppState) {
    if let Some(open) = state.resolver.as_mut() {
        open.seek_open = false;
    }
}

/// Spend a pending seek once everything it waits on has landed: the open
/// file's read and the set's. A file with a clash open — or one that could
/// not be read — keeps the cursor where its read put it (the first open
/// clash). A solved or done one hands on to the next file a clash may be
/// open in; that file's own read decides again. Each hop leaves the file
/// it left in [`super::ResolverState::seen`], so the walk ends.
///
/// State-only, and called at the tail of every answer (`update::answer`),
/// so it does not matter which read lands last.
pub(crate) fn seek_first_open(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if !open.seek_open || open.job.is_some() || open.set_job.is_some() {
        return;
    }
    let nothing_open = matches!(open_state(open), Some(RowState::AllTaken | RowState::Done));
    let from = open.opened_from;
    if let Some(open) = state.resolver.as_mut() {
        open.seek_open = false;
    }
    if !nothing_open {
        return;
    }
    let Some(next) = super::review::next_open_file(state) else {
        return;
    };
    if let Some(open) = state.resolver.as_mut() {
        remember(open);
    }
    if reopen(state, commands, next, from, next_id)
        && let Some(open) = state.resolver.as_mut()
    {
        open.seek_open = true;
    }
}
