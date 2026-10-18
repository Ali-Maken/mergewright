//! **The one gate for writing this file**, and the refusal words every key
//! that writes or leaves it asks first (moved whole out of `resolver.rs`,
//! ticket ease-pass/05r).

use super::{ResolverState, unpick};
use crate::update::state::AppState;

/// The words a writing key is refused with while another write on this
/// file is in flight, or `None` when it may go ahead.
///
/// Two channels write a conflicted file: a block take is an fs write on
/// the pool ([`ResolverState::job`]), and `⏎`, a whole-file `o`/`t` and `d`
/// are git writes (`state.operating`). The pool runs jobs on several
/// workers, so a take and a `git add` queued together race — the index can
/// end up holding the old take while the screen and disk show the new one
/// (m6-04-08 review, Major 2). Every writing key asks here, so the two
/// channels are serial on this file.
pub(in crate::update::resolver) fn write_refusal(state: &AppState) -> Option<&'static str> {
    if let Some(words) = busy_refusal(state) {
        return Some(words);
    }
    state
        .operating
        .is_some()
        .then_some("a write is already running")
}

/// [`ResolverState::busy_words`], with `u`'s untake asked **first**: the
/// resolver holds the untake's id as its `job` while it runs, which
/// `busy_words` would word as `still writing the last take` — no take is
/// (review ease-pass-05 minor 1). The untake is an fs write on the pool
/// too; asking the picks also catches a resolver opened since (ease-pass
/// 05). `j`/`k`, `B` and every writing key ask here.
pub(in crate::update::resolver) fn busy_refusal(state: &AppState) -> Option<&'static str> {
    if state.picks.undoing().is_some() {
        return Some(unpick::UNDO_LANDING);
    }
    state.resolver.as_ref().and_then(ResolverState::busy_words)
}
