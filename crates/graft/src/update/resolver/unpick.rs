//! `u` in the resolver (ease-pass 05): the last pick put back, exactly as
//! it was before — the block open again, or the take a re-take replaced —
//! with the cursor on it, in whichever file it was made.
//!
//! The pick stack is [`AppState::picks`](crate::update::AppState::picks)
//! (`picks.rs`); this is the key, and the answer. An untake is a file write
//! on the take's queue, gated as a take is: one thing writes this file at a
//! time, and the write is against the fingerprint the pick left, so an edit
//! made outside Mergewright since is never written over.

use std::path::Path;
use std::sync::Arc;

use graft_core::event::KeyPress;
use graft_core::job::JobId;
use graft_git::conflict::ConflictFile;
use graft_ui::keymap::{ActionId, Keymap};

use super::files::remember;
use super::picks::Pick;
use super::{done_refusal, reopen, scrolled_to};
use crate::jobs::conflict::UntakeFailure;
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// What `u` says with no pick to undo, outside a stash clash.
pub(crate) const NOTHING_TO_UNDO: &str = "nothing to undo here";

/// What a key that would leave the file says while an untake is out.
pub(super) const UNDO_LANDING: &str = "still undoing the last pick \u{b7} try again in a moment";

/// What the status line says when a fresh read of `path` found other bytes
/// than its newest pick wrote, so every pick in it was dropped (review
/// ease-pass-05 M2, ticket ease-pass/05r): `<file> changed since your picks
/// · u can't undo them now`, then `· next u: <file>` naming `next`, the file
/// the next `u` reaches. Keys spelled from his keymap; with `u` unbound only
/// the first clause stands. Said, never kept: an untake over changed bytes
/// would write over his edit.
#[must_use]
pub fn dropped_words(path: &Path, next: Option<&Path>, keys: &Keymap) -> String {
    let undo = [ActionId("app.undo")];
    let head = format!("{} changed since your picks", path.display());
    let clauses = [
        keys.key_verb(&undo, "can't undo them now"),
        next.and_then(|next| {
            keys.spell_keys(&undo)
                .map(|key| format!("next {key}: {}", next.display()))
        }),
    ];
    std::iter::once(head)
        .chain(clauses.into_iter().flatten())
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// `u`: put the last pick back.
///
/// Refused with nothing popped: on a done file (its own words), and while
/// anything else writes or reads this file — a take landing first, since
/// the pick `u` should undo is not on the stack until it has. With no pick,
/// a stash clash keeps the clause its landing used (`u` from outside puts
/// the whole put-back back); anywhere else, [`NOTHING_TO_UNDO`].
pub(super) fn undo_pick(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if let Some(words) = done_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    if let Some(words) = refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    if state.picks.is_empty() {
        let words =
            crate::update::clash::undo_clause(state).unwrap_or_else(|| NOTHING_TO_UNDO.to_owned());
        if let Some(open) = state.resolver.as_mut() {
            open.notice = Some((words, false));
        }
        return;
    }
    let id = next_id();
    // `refusal` answered an untake already out, so this pops.
    let Some(pick) = state.picks.start_undo(id) else {
        return;
    };
    if let Some(open) = state.resolver.as_mut() {
        open.job = Some(id);
        open.reading = false;
    }
    commands.push(Command::UntakeSide(id, pick));
}

/// The review's `u` (ease-pass/05b): the last pick put back, as the
/// resolver's `u` does — the review has closed already. `open_after`: no
/// resolver was open under the review (it was asked for from the banner),
/// so the untake's landing opens one on the pick, from the banner.
pub(crate) fn undo_from_review(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    open_after: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    undo_pick(state, commands, next_id);
    if open_after
        && state.resolver.is_none()
        && let Some(id) = state.picks.undoing()
    {
        state.picks.open_on_landing(id);
    }
}

/// Why `u` must wait, or `None`. A take landing is asked first: its words
/// say to wait, where the busy file's would read as a refusal.
fn refusal(state: &AppState) -> Option<&'static str> {
    super::handover::takes_refusal(state).or_else(|| super::take::take_refusal(state))
}

/// `app.undo` (`u`, `⌃Z`) puts the last pick back; `app.redo` (`U`) says
/// where undo is. `false` for any other key, which `act` goes on with.
pub(super) fn undo_keys(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    match asks_undo(state, key) {
        Some(Asks::Undo) => undo_pick(state, commands, next_id),
        Some(Asks::Redo) => undo_answered(state),
        None => return false,
    }
    true
}

/// Which of `app.undo` and `app.redo` a key asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asks {
    /// `u`, `⌃Z`: the last pick back (ease-pass 05).
    Undo,
    /// `U`: says where undo is ([`undo_answered`]); a pick has no redo.
    Redo,
}

/// `key` is one of `app.undo`'s or `app.redo`'s chords as shipped: `u`,
/// `⌃Z`, `U`. The edge hands a handler shipped chords only
/// (`update/edge.rs`), so this is the action itself, whichever chord he
/// pressed and whatever he moved it to. `⌃Z` was swallowed until m8/09g C.
/// With `suspend_on_ctrl_z`, `⌃Z` is `app.suspend`'s, not undo's, and
/// stays swallowed here: a suspend from the resolver would come back as
/// the `HandedOver` that `update/mod.rs` reads as `e`'s edit returning.
/// Moved from `resolver.rs` (ease-pass 05), which only told the two apart.
fn asks_undo(state: &AppState, key: KeyPress) -> Option<Asks> {
    use graft_ui::keymap::{ActionId, KeyChord, Keymap};
    let chord = KeyChord::from_press(&key)?;
    let of = |id| Keymap::shipped().chords(&ActionId(id)).contains(&chord);
    let suspends = state.suspend_on_ctrl_z && chord == KeyChord::ctrl('z');
    if of("app.undo") && !suspends {
        Some(Asks::Undo)
    } else if of("app.redo") {
        Some(Asks::Redo)
    } else {
        None
    }
}

/// `U` inside the resolver: not redo — the resolver is modal (§2.1), and a
/// pick put back has no redo — but never silence either (review
/// m6-usability-2 m-i). The notice row says where undo is, as information:
/// with an operation live, what `u` would say anywhere (§2.6a); on a stash
/// or set-aside clash, the clause its landing notice used; else that `u`
/// works from the graph. No command, and the status line is left alone.
fn undo_answered(state: &mut AppState) {
    let words = if let Some(live) = crate::update::banner::live(state) {
        crate::update::undo::live_refusal_words(live.kind)
    } else if let Some(clause) = crate::update::clash::undo_clause(state) {
        clause
    } else {
        // A sequence (W5): `esc` from the resolver's place, `u` from the
        // graph's, where it lands him; an unbound one drops its clause.
        [
            state
                .keymap
                .key_verb(&[graft_ui::keymap::ActionId("app.back")], "first"),
            state.keymap.key_verb(
                &[graft_ui::keymap::ActionId("app.undo")],
                "undoes from the graph",
            ),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
    };
    if let Some(open) = state.resolver.as_mut() {
        // A line that is only key clauses, every key unbound, draws nothing.
        open.notice = (!words.is_empty()).then_some((words, false));
    }
}

/// An untake answered.
pub fn untaken(
    state: &mut AppState,
    id: JobId,
    result: Result<Arc<ConflictFile>, UntakeFailure>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(pick) = state.picks.finish_undo(id) else {
        return;
    };
    // The review's `u` from the banner: the resolver opens on the pick,
    // whatever the answer, so he is where the pick is (ease-pass/05b).
    if state.picks.opens_on_landing(id) && state.resolver.is_none() {
        let path = pick.path.clone();
        crate::update::resolver::open_resolver(
            state,
            commands,
            path,
            super::OpenedFrom::Banner,
            next_id,
        );
        if let (Ok(file), Some(_)) = (&result, state.resolver.as_ref()) {
            state
                .picks
                .land_after_read(&pick.path, Arc::clone(file), pick.first_block());
        }
    }
    let holds = release(state, id);
    let path = pick.path.clone();
    match result {
        Ok(file) => landed(state, &pick, file, holds, commands, next_id),
        Err(UntakeFailure::Stale(why)) => {
            // Nothing written. Every pick in the file was chosen against a
            // document that is gone; dropping them is what lets the next
            // `u` reach the picks made before them, elsewhere.
            state.picks.seal(&path);
            state.status =
                Status::sticky(format!("{} {why} \u{b7} nothing undone", path.display()));
            // What is on screen is not what is on disk: read it again, as a
            // take's `Changed` does.
            let on_screen = state
                .resolver
                .as_ref()
                .is_some_and(|open| open.path == path && open.file.is_some());
            if holds && on_screen {
                reread(state, commands, next_id);
            }
        }
        Err(UntakeFailure::Failed(why)) => {
            // Not written, and not stale: the pick still applies, on top,
            // and the next `u` tries it again. Sticky, as the stale words
            // are: a `Failed` line goes with the next job's answer, before
            // he has read why `u` did nothing (review ease-pass-05 M3).
            state.picks.put_back(pick);
            state.status = Status::sticky(format!(
                "could not undo the pick in {}: {why}",
                path.display()
            ));
        }
    }
}

/// An untake that will never answer — failed as a job, or cancelled.
/// Whether it wrote is unknown, so its pick goes back on top: safe either
/// way, because the next `u` writes only over the bytes the pick left, and
/// says so when they are not.
pub fn untake_ended(state: &mut AppState, id: JobId) {
    if let Some(pick) = state.picks.finish_undo(id) {
        state.picks.put_back(pick);
        release(state, id);
    }
}

/// The resolver stops waiting on `id`; `true` when it was.
fn release(state: &mut AppState, id: JobId) -> bool {
    let Some(open) = state.resolver.as_mut().filter(|open| open.job == Some(id)) else {
        return false;
    };
    open.job = None;
    true
}

/// The untake wrote: the line says so, and the cursor goes to the pick's
/// first block — in the file on screen, or in its own file, opened.
fn landed(
    state: &mut AppState,
    pick: &Pick,
    file: Arc<ConflictFile>,
    holds: bool,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let path = &pick.path;
    let block = pick.first_block();
    state.status = Status::done(format!(
        "undid the last pick in {} \u{b7} {} of {} open",
        path.display(),
        file.open(),
        file.blocks().len()
    ));
    // Written outside any operation, as a take is: the changes pane under
    // the resolver would keep the pre-untake diff (m6-conflicts/85 B).
    commands.extend(state.detail.file_written(path, next_id));
    // Closed meanwhile, or busy with something else: the file is written
    // and the line says so; the next read shows it.
    let Some(open) = state.resolver.as_mut().filter(|_| holds) else {
        return;
    };
    if open.path != *path {
        let from = open.opened_from;
        remember(open);
        if reopen(state, commands, path.clone(), from, next_id) {
            state.picks.land_after_read(path, file, block);
        }
        return;
    }
    open.file = Some(file);
    open.notice = None;
    remember(open);
    if let Some(block) = block {
        let top = scrolled_to(state, block);
        if let Some(open) = state.resolver.as_mut() {
            open.cursor_block = Some(block);
            open.top = top;
        }
    }
}

/// Read the open file again, the resolver waiting on it.
fn reread(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    let id = next_id();
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.job = Some(id);
    open.reading = true;
    commands.push(Command::OpenConflict(id, open.path.clone()));
}
