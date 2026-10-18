//! The keys a modal box, editor or banner claims before the panes and the
//! graph see them, and the `u`/`U` every screen answers.

use mergewright_core::event::KeyPress;

use crate::update::events::Command;
use crate::update::state::AppState;

/// The two modal boxes drawn over everything, in the order they claim keys.
///
/// The confirm box (`d` discard, m3-working-tree-undo-and-stash/04) wins
/// even over the context menu: a menu key opened it, but it never opens one
/// back over itself, so there is no ordering conflict — only whether it is
/// `Some`.
///
/// The Ghost Preview sits **between** the confirm box and the context menu
/// (m5-history-and-ghost-preview/08). A confirm box may open over a preview
/// and never the reverse, so the box answers first; the menu is one of the
/// things that opens a preview, and closes as it does, so the two are never
/// up together. Modal: while a preview is open every key but `Ctrl+C` is its
/// own — `u` included, because nothing may act over a picture the user is
/// still reading.
pub(super) fn boxes_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let opened =
        crate::update::confirm::confirm_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    if opened {
        return true;
    }
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::preview::preview_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The rewrite editor (`R`, m7-power-tools/08), right after the boxes:
/// modal, every key but `Ctrl+C` is its own while it is up (§2.7), and
/// `Esc` while its read is out cancels the read.
pub(super) fn rewrite_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = crate::update::rewrite_editor::rewrite_key(next, commands, key, &mut || {
        local_jobs.next_id()
    });
    next.local_jobs = local_jobs;
    claimed
}

/// The finish summary, right after the boxes and before the resolver
/// (m6-conflicts/19): modal, every key but `Ctrl+C` is its own. Before the
/// resolver because finish-from-the-resolver opens it over one.
pub(super) fn finish_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::finish::finish_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The conflict resolver, right after the preview and before the banner
/// (m6-conflicts/04). Modal: while it is open every key but `Ctrl+C` is
/// its own.
///
/// **Before the banner on purpose.** `⏎` on the focused banner is what
/// opens it, and the banner is modal too — leaving it first would mean the
/// resolver's own `c`, `s`, `a`, `M` and `e` reached the operation instead
/// of the file, with a document on screen saying otherwise.
pub(super) fn resolver_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::resolver::resolver_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The operation banner, right after the preview and before the menu
/// (m5-history-and-ghost-preview/09). Modal while focused: a merge or
/// rebase stopped on a conflict is a state the user has to resolve or
/// leave, and every key that would start a *second* thing on top of it —
/// `u`, `S`, `m`, `r` — is swallowed rather than obeyed. `Ctrl+C` passes,
/// and `Tab`/`Esc` give the keyboard back, so the mode always has a door.
/// A blame place on screen claims every key but the live globals, before
/// the menu, undo, a write's `Esc` and the panes — so the graph's verbs,
/// `u` and `Tab` are inert in it, and its `Esc` never stops a running
/// write (m7-power-tools/20).
pub(super) fn place_claim(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if !crate::update::place::owns_keys(next) || crate::update::place::live_global(key) {
        return false;
    }
    commands.extend(crate::update::place::place_key(next, key));
    true
}

pub(super) fn banner_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if crate::update::banner::take_focus(next, key) {
        return true;
    }
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::banner::banner_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The global `u`/`U`, on every screen. In J it is `⏎` on the `← u`/`← U`
/// row: both are `journal::undo_from_j`, which reads the launch's one
/// outcome (m7/30o R3) — J closes once the undo is launched; the push box
/// and a refusal leave J where he pressed the key.
pub(super) fn undo_key_anywhere(next: &mut AppState, commands: &mut Vec<Command>, redo: bool) {
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    if next.journal.shown {
        crate::update::journal::undo_from_j(next, commands, redo, &mut next_id);
    } else {
        // The outcome is on the state (`operating`, the box or the words);
        // off J there is nothing to close.
        let _started: crate::update::undo::UndoStart =
            crate::update::undo::start_undo(next, commands, redo, &mut next_id);
    }
    next.local_jobs = local_jobs;
}
