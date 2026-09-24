//! Handing the file to another program — `e` (`$EDITOR`) and `M` (the
//! mergetool) — and what waits for the takes still landing before that, or
//! before `Esc` closes (m6-conflicts/28 R6, review m6-amend-15 M1).
//!
//! A take in flight or queued was chosen against the document on screen.
//! A hand-over re-reads the file on return (`answers::handed_back`), which
//! withdraws every one of them; `Esc` drops the resolver that holds the
//! queue. Both wait instead, in words: takes land in milliseconds, and
//! losing them silently is worse than one more key.

use super::{ResolverState, write_refusal};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// The words a key that would drop the takes is refused with.
const TAKES_LANDING: &str = "a take is still landing · try again in a moment";

/// `Some` while a take is in flight or queued; the words say so.
pub(super) fn takes_refusal(state: &AppState) -> Option<&'static str> {
    let open = state.resolver.as_ref()?;
    (open.in_flight.is_some() || !open.queued.is_empty()).then_some(TAKES_LANDING)
}

/// `e`/`M`: refused while a take is landing, then wherever a write would be
/// ([`write_refusal`]) — the hand-over's program writes the same file.
///
/// The takes are asked **first**: a take in flight is also the
/// [`ResolverState::job`] that `write_refusal` words as "still writing the
/// last take", which would hide the reason the user can act on (wait) behind
/// one that reads as a refusal of `e` for ever.
pub(super) fn handover_refusal(state: &AppState) -> Option<&'static str> {
    takes_refusal(state).or_else(|| write_refusal(state))
}

/// `M`: the banner's own mergetool handover, refusal and all — `no
/// merge.tool configured · e edits <path>` is the same answer here.
pub(super) fn mergetool_here(state: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(words) = handover_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    if let Some(path) = state.resolver.as_ref().map(|open| open.path.clone()) {
        let launched = crate::update::banner::open_mergetool(state, commands, Some(&path));
        debug_assert!(launched, "a path was passed, so `M` was answered");
    }
}

/// `e`: hand the terminal to `$EDITOR` on this file, at the cursor block's
/// own marker line.
///
/// [`Command::OpenExternalEditor`], not `OpenConflictEditor`: the banner's
/// `e` asks the loop to find the first `<<<<<<< ` because `update` opens no
/// files, but the resolver is *holding* the parsed document and knows
/// exactly which line the user is looking at.
pub(super) fn edit_here(state: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(words) = handover_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    commands.push(Command::OpenExternalEditor {
        path: open.path.clone(),
        line: marker_line(open),
    });
}

/// The cursor block's first line, or `None` — nothing under the cursor to
/// aim at opens the file at its top, as `$EDITOR` would anyway.
fn marker_line(open: &ResolverState) -> Option<u32> {
    let (file, block) = (open.file.as_ref()?, open.cursor_block?);
    let first_line = file.blocks().get(block)?.first_line;
    u32::try_from(first_line).ok()
}
