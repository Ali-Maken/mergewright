//! `B`: show or hide the merged common ancestor inside every open block
//! (m6-conflicts/05, design note Proposed change 2b).

use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::conflict::ConflictFile;
use graft_git::status::UnmergedHow;

use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `B`: show or hide the merged common ancestor inside every open block.
///
/// Hiding is free — the bases stay attached, only `base_shown` flips.
/// Showing for the first time loads them: `Command::LoadBase` under its own
/// [`super::ResolverState::base_job`], refused while [`super::ResolverState::job`] is busy
/// (an open read or a take in flight) so the two never race the same
/// document. No card to load a base for (a whole-file shape, or the opening
/// read still in flight) is answered the same way `n` answers "nothing to
/// walk to" — quietly, since `B` on nothing is not a mistake worth a line.
pub(super) fn toggle_base(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.base_shown {
        if let Some(open) = state.resolver.as_mut() {
            open.base_shown = false;
        }
        return;
    }
    if open.job.is_some() || open.base_job.is_some() {
        state.status = Status::sticky("still writing the last take");
        return;
    }
    let Some(file) = open.file.clone() else {
        return;
    };
    if open.how == Some(UnmergedHow::BothAdded) {
        // Added on both sides: git holds no stage 1, so there is no base to
        // load — answered here, in the spec's words, with no job.
        if let Some(open) = state.resolver.as_mut() {
            // A fact of the file, not a failure: not a problem (planner
            // ruling, 2026-09-23).
            open.notice = Some(("no base (added on both sides)".to_owned(), false));
        }
        return;
    }
    let path = open.path.clone();
    let id = next_id();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.base_job = Some(id);
    commands.push(Command::LoadBase(id, path, Box::new((*file).clone())));
}

/// `B`'s `LoadBase` answered.
pub fn base_loaded(
    state: &mut AppState,
    id: JobId,
    result: Result<(Arc<ConflictFile>, usize), (String, bool)>,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.base_job != Some(id) {
        return;
    }
    match result {
        Ok((file, attached)) => {
            let Some(open) = state.resolver.as_mut() else {
                return;
            };
            open.base_job = None;
            open.file = Some(file);
            open.base_shown = true;
            state.status = Status::Done(format!("base shown for {attached} conflicts"));
        }
        Err((words, problem)) => {
            let Some(open) = state.resolver.as_mut() else {
                return;
            };
            open.base_job = None;
            // `base_shown` stays `false`: nothing was attached, so there is
            // nothing for the widget to draw even if it were set. Only a
            // failure (git's `diff3`, the repo not opening) is a problem;
            // no base or a file edited past git is a fact, shown plainly.
            open.notice = Some((words, problem));
        }
    }
}
