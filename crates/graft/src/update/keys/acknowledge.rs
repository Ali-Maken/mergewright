//! What a key does to the status line before it acts.

use graft_core::event::{KeyCode, KeyPress};

use super::super::events::CommitGraphOffer;
use super::super::state::{AppState, Status};

/// Clears what only acknowledges something the user just did: the copy
/// notice, a finished write's `<description> done` line and the
/// commit-graph written line. Each lasts until the next key **or click**,
/// whatever that key or click does (m6-conflicts/42) — one rule, called
/// from [`key_event`] and from `mouse::mouse_event` on a button press.
///
/// Not while the row cannot show it (m6-conflicts/78, Decided 1): under a
/// modal, which owns the bottom row, a `Done` that arrived there stays until
/// the modal closes and the row has drawn it; and not while an unseen
/// failure stands in for the status, which the key that dismisses it does
/// not also acknowledge.
pub(in crate::update) fn acknowledge(next: &mut AppState) {
    if next.modal_owns_bottom_row() || next.unseen.is_some() {
        return;
    }
    if matches!(
        next.status,
        Status::Copied { .. } | Status::Done { .. } | Status::Waiting(_)
    ) {
        next.status = Status::Ready;
    }
    if matches!(next.commit_graph, CommitGraphOffer::Written { .. }) {
        next.commit_graph = CommitGraphOffer::NotNeeded;
    }
    // `config: N problems · ? shows them` (ADR-0055): said once, at startup.
    next.config_notice = false;
}

/// A failure that arrived under a box and is drawn now the box has closed
/// (`AppState::unseen`): any key dismisses it. `true` when the key is spent
/// doing so — `Esc`, which must not also cancel the write the box's act
/// started; that is a second `Esc` (m6-conflicts/78, Decided 1). Left alone
/// while a modal owns the row: it is not on screen to dismiss.
pub(super) fn dismiss_unseen(next: &mut AppState, key: &KeyPress) -> bool {
    if next.unseen.is_none() || next.modal_owns_bottom_row() {
        return false;
    }
    next.unseen = None;
    key.code == KeyCode::Escape
}
