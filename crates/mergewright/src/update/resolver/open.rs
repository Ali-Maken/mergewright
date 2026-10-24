//! Opening the resolver, and moving it to another file: `open_resolver`
//! for `⏎` on the banner or a `CONFLICTS` row, `reopen` for the walk, the
//! file list and a write landing — and the one refusal both ask first.

use std::path::PathBuf;

use mergewright_core::job::JobId;

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
            keys: state
                .keymap
                .key_verb(&[mergewright_ui::keymap::ActionId("app.back")], "closes it"),
            kind: mergewright_ui::graph_screen::ProblemKind::Notice,
        };
        return true;
    }
    if let Some(words) = open_refusal(state) {
        state.status = words;
        return true;
    }
    start(state, commands, path, from, next_id);
    // The banner reads as a pause from here on (Q7 a).
    state.picks.resolver_opened();
    // A new resolver reads the set afresh: nothing carries over from a
    // resolver opened on an earlier step.
    files::reread_set(state, commands, next_id);
    true
}

/// **Land on a stash clash** (m6-conflicts/72b): the `S` overlay closes,
/// the changes pane opens with its cursor on `first` under `CONFLICTS`, and
/// the resolver opens on `first` from the pane (`OpenedFrom::Pane`), so
/// `Esc` gives the keyboard to that row. There is no banner to land from.
///
/// `false` when the resolver could not open: one is already open, or a box
/// owns the input (its refusal is on the status line, as for `⏎`).
pub(crate) fn land_on_clash(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    first: PathBuf,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_some() || box_open(state) {
        return false;
    }
    if state.stash.shown {
        crate::update::stash::close(state, commands);
    }
    crate::update::relaunch::land_on_the_work(state, commands, Some(first.clone()), next_id);
    state.detail.focus = mergewright_ui::detail::DetailFocus::Files;
    // The status held may be from before the put-back: clean, so the pane
    // cannot stay on the working tree yet. The read the answer asked for
    // lands it again.
    state.clash_pane_owed = !state.uncommitted_row_shown();
    open_resolver(state, commands, first, OpenedFrom::Pane, next_id);
    state.resolver.is_some()
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
            keys: state
                .keymap
                .key_verb(&[mergewright_ui::keymap::ActionId("app.back")], "stops it"),
            kind: mergewright_ui::graph_screen::ProblemKind::Notice,
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
///
/// `true` when `path` was opened afresh; `false` when refused (or there was
/// no resolver to reopen). Only the file list carries over: anything else
/// the old state held is the old file's.
pub(super) fn reopen(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: PathBuf,
    from: OpenedFrom,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if let Some(words) = open_refusal(state) {
        state.status = words;
        return false;
    }
    let Some(was) = state.resolver.take() else {
        return false;
    };
    start(state, commands, path, from, next_id);
    let Some(open) = state.resolver.as_mut() else {
        return false;
    };
    open.files = was.files;
    open.set_job = was.set_job;
    open.seen = was.seen;
    true
}
