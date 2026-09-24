//! Opening the resolver, and moving it to another file: `open_resolver`
//! for `⏎` on the banner or a `CONFLICTS` row, `reopen` for the walk, the
//! file list and a write landing — and the one refusal both ask first.

use std::path::PathBuf;

use graft_core::job::JobId;

use super::{OpenedFrom, ResolverState, files};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// **Open the resolver on `path`**, or refuse in words.
///
/// `true` when the key was spent — which is either way: a refusal is an
/// answer, and the key that earned it must not also mean whatever it means
/// further down the chain.
pub fn open_resolver(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_some() {
        state.status = Status::Sticky {
            reason: "the resolver is already open".to_owned(),
            keys: Some("esc closes it".to_owned()),
        };
        return true;
    }
    if let Some(words) = open_refusal(state) {
        state.status = words;
        return true;
    }
    start(state, commands, path, from, next_id);
    // A new resolver reads the set afresh: nothing carries over from a
    // resolver opened on an earlier step.
    files::reread_set(state, commands, next_id);
    true
}

/// The resolver on `path`, its opening read queued.
fn start(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    let id = next_id();
    state.resolver = Some(ResolverState::opening(path.clone(), from, id));
    commands.push(Command::OpenConflict(id, path));
}

/// Why no resolver may open right now, as the status line says it.
fn open_refusal(state: &AppState) -> Option<Status> {
    // A box that is waiting for an answer keeps it. Opening a full-screen
    // overlay over a `d discard` confirmation would leave the user deciding
    // about a file with the question that started it off screen.
    if box_open(state) {
        return Some(Status::sticky("finish the box first"));
    }
    // A write in flight owns the working tree; a take landing in the middle
    // of one would be writing the very file git is staging.
    if state.operating.is_some() {
        return Some(Status::Sticky {
            reason: "a write is running".to_owned(),
            keys: Some("esc stops it".to_owned()),
        });
    }
    None
}

/// A box waiting for an answer — a confirm or the commit editor — is up,
/// and so owns the input.
pub(in crate::update) fn box_open(state: &AppState) -> bool {
    state.confirm.is_some() || state.commit_editor.is_some()
}

/// Close this file and open `path`, keeping where the walk started and the
/// file list — its set, the read in flight for it, and what each file was.
///
/// Refused in `open_resolver`'s words when a box or a write stands in the
/// way — and the resolver then stays **as it was**, on the file it had
/// open. The refusal is decided before anything is taken: taking first
/// closed the resolver on a refusal and left the box that caused it
/// standing over the graph (review m6-amend-13, Major 1).
pub(super) fn reopen(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) {
    if let Some(words) = open_refusal(state) {
        state.status = words;
        return;
    }
    let Some(was) = state.resolver.take() else {
        return;
    };
    start(state, commands, path, from, next_id);
    if let Some(open) = state.resolver.as_mut() {
        open.files = was.files;
        open.set_job = was.set_job;
        open.seen = was.seen;
    }
}
