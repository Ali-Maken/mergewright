//! The two ways out of the resolver from the keyboard: `c` finishes or
//! continues the operation from here (§2.6, P8), `Esc` closes to where `⏎`
//! came from. Neither aborts anything.

use graft_core::job::JobId;

use super::OpenedFrom;
use super::handover::{self, takes_refusal};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `c`: finish (a merge's summary) or continue, from here (§2.6, P8).
/// Refused first while a take or write on this file is landing, in the
/// resolver's own words and the hand-over's order — the summary reads what
/// is staged, so a write still landing would not be in it. Then the
/// banner's own `c`, with its words on the notice row.
///
/// Not gated on a done file ([`super::done::refuses`]): after the last
/// file, a done file is exactly where the user is.
pub(super) fn finish_pressed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if let Some(words) = handover::handover_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    crate::update::banner::continue_pressed(
        state,
        commands,
        crate::update::finish::ReturnTo::Resolver,
        next_id,
    );
}

/// `Esc`: close — once the takes have landed. Closing drops the resolver
/// that holds the queue, so a take still queued would never be sent (review
/// m6-amend-15 M1).
pub(super) fn escape(state: &mut AppState) {
    if let Some(words) = takes_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    close(state);
}

/// Close the resolver and give the keyboard back to where `⏎` came from.
/// **Never** aborts the operation underneath (§2.6).
fn close(state: &mut AppState) {
    let Some(open) = state.resolver.take() else {
        return;
    };
    if open.opened_from == OpenedFrom::Banner {
        // Only if there is still a banner to focus: `banner_focus` without
        // one is the state `AppState::banner_focus` rules out.
        state.banner_focus = crate::update::banner::banner_view(state).is_some();
    }
    // The line the resolver put up was about the keys it has just taken
    // away.
    if matches!(state.status, Status::Sticky { .. }) {
        state.status = Status::Ready;
    }
}
