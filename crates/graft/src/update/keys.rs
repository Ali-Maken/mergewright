//! The graph's own keys: movement, view changes and the toggles that are not
//! the search prompt or the commit-graph offer (those are `search` and
//! `commit_graph`).

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::oid::Oid;

use crate::history::Motion;
use graft_ui::detail::DetailFocus;

use super::events::Command;
use super::state::{AppState, Quit, Status};

mod acknowledge;
mod actions;
mod bound;
mod commit;
mod escape;
#[cfg(test)]
mod live_c_tests;
mod predicates;
mod refresh;
#[cfg(test)]
mod tab_words_tests;
#[cfg(test)]
mod tests;
pub(in crate::update) mod view_change;

pub(in crate::update) use acknowledge::acknowledge;
use acknowledge::dismiss_unseen;
pub(in crate::update) use actions::strip_rows_for;
use actions::{copy_selected_sha, cycle_tier, toggle_compare_selected};
pub use bound::bound_actions;
use commit::commit_key;
use escape::escape_key;
use predicates::{
    RemoteKey, is_commit_key, is_compare_key, is_context_menu_key, is_copy_key, is_focus_toggle,
    is_lane_toggle, is_name_key, is_quit, is_refresh, is_remote_key, is_rewrite_key,
    is_screen_toggle, is_tier_cycle, is_undo_key, motion_for,
};
pub(in crate::update) use refresh::refresh_key;
use view_change::{ViewChange, view_change};

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
fn boxes_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let opened = super::confirm::confirm_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    if opened {
        return true;
    }
    let mut local_jobs = next.local_jobs;
    let claimed = super::preview::preview_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The rewrite editor (`R`, m7-power-tools/08), right after the boxes:
/// modal, every key but `Ctrl+C` is its own while it is up (§2.7), and
/// `Esc` while its read is out cancels the read.
fn rewrite_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        super::rewrite_editor::rewrite_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The finish summary, right after the boxes and before the resolver
/// (m6-conflicts/19): modal, every key but `Ctrl+C` is its own. Before the
/// resolver because finish-from-the-resolver opens it over one.
fn finish_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::finish::finish_key(next, commands, key, &mut || local_jobs.next_id());
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
fn resolver_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::resolver::resolver_key(next, commands, key, &mut || local_jobs.next_id());
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
fn place_claim(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if !super::place::owns_keys(next) || super::place::live_global(key) {
        return false;
    }
    commands.extend(super::place::place_key(next, key));
    true
}

fn banner_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if super::banner::take_focus(next, key) {
        return true;
    }
    let mut local_jobs = next.local_jobs;
    let claimed = super::banner::banner_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The half of [`super::core_event`] that answers a key.
///
/// The detail pane answers its keys first (`Enter` opens it, ADR-0023).
///
/// A blame place on screen is a modal pane (keymap §2.1, [`place_claim`]).
pub(super) fn key_event(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    // An unseen failure on the row is what this key answers first: `Esc`
    // only dismisses it; any other key dismisses it and acts, without also
    // acknowledging the status under it (m6-conflicts/78, Decided 1).
    let unseen_shown = next.unseen.is_some() && !next.modal_owns_bottom_row();
    if dismiss_unseen(next, &key) {
        return;
    }
    if !unseen_shown {
        acknowledge(next);
    }
    // The modal claims, in order, each returning `true` once it has taken
    // the key: a badge in the air is dropped by any key before that key
    // means anything (`drag::drag_key`; only `Esc` is spent doing it), then
    // the confirm box and the preview, then the resolver, then the focused
    // banner, then a blame place on screen. `||` is the early return, one
    // claim per line.
    if super::drag::drag_key(next, key)
        || boxes_key(next, commands, key)
        || rewrite_key(next, commands, key)
        || finish_key(next, commands, key)
        || resolver_key(next, commands, key)
        || banner_key(next, commands, key)
        || place_claim(next, commands, key)
    {
        return;
    }
    // A menu open (`.`/right-click, m2-details-and-diffs/12) gets the key
    // first, but it is **not** modal (`docs/design/02-keymap.md` §2.1, "an
    // overlay that needs only a cursor is not a modal pane"): it claims
    // `j`/`k`, the arrows, `Enter`, `Esc` and `.`, and every other key
    // closes it and carries on down this function to mean what it always
    // means. That is what makes a label reading `Merge main into feature
    // (m)` true (m5-history-and-ghost-preview/13).
    //
    // Then the branch picker (`C`, m7-power-tools/10), a text field: while
    // up it claims every key but `Ctrl+C`; on the graph `C` opens it.
    if super::context_menu::context_menu_key(next, commands, key)
        || super::picker::picker_key(next, commands, key)
    {
        return;
    }
    // The commit editor is modal too, and claims every key but `Ctrl+C`
    // (m3-working-tree-undo-and-stash/05).
    if editor_key(next, commands, key) {
        return;
    }
    // `Ctrl+Z` before anything else claims it. Off by default, because
    // ADR-0002 gives the key to undo; on, it means what it means everywhere
    // else, and the loop hands the terminal back to the shell. With it off
    // the key is left alone here — `undo-and-stash-ui` binds it to undo.
    if next.suspend_on_ctrl_z
        && key.code == KeyCode::Char('z')
        && key.modifiers == Modifiers::CONTROL
    {
        commands.push(Command::Suspend);
        return;
    }
    // `u`, `Ctrl+Z` and `U` (ADR-0002, DESIGN.md §2.2): global, before any
    // pane's own keys, because undo means the same thing wherever the
    // cursor is. **Never while a text field is open** — `text_field_open`
    // names all of them, because relying on "the editor is modal and claimed
    // it above" is exactly what let the stash prompt turn a typed `u` into a
    // journaled undo (m3/06 review, Critical 1).
    if !next.text_field_open()
        && let Some(redo) = is_undo_key(key)
    {
        undo_key(next, commands, redo);
        return;
    }
    // `Esc` while a write runs stops it — any write, not only a remote one
    // (design note, choice 4). Before the panes and overlays: their `Esc`
    // closes a view, and a hung fetch must never need two keys to stop.
    // The confirm box, the menu and the editor have already had the key;
    // a text field keeps its own `Esc` (`cancel_operating` asks).
    if key.code == KeyCode::Escape && super::remote::cancel_operating(next, commands) {
        return;
    }
    // A failure on the status line is dismissed by `Esc` before `Esc` means
    // anything else (PROMPT.md §3, "errors stay until dismissed"). Without
    // this a write failure had no way off the screen at all
    // (m3/04 review gap, m3-working-tree-undo-and-stash/05). An open text
    // field gets its `Esc` first and applies this rule itself, so that a
    // field can never be shut by a key its own handler never saw.
    if !next.text_field_open()
        && key.code == KeyCode::Escape
        && matches!(next.status, Status::Failed(_) | Status::Sticky { .. })
    {
        next.status = Status::Ready;
        // The diverged line is gone, and with it the `m`/`r` it offered:
        // the tip it remembered is only meaningful beside the words that
        // explained it (m5-history-and-ghost-preview/08, piece 9).
        next.remote.diverged_upstream = None;
        return;
    }
    if pane_and_overlay_key(next, commands, key) {
        return;
    }
    if let Some(command) = super::commit_graph::commit_graph_key(next, key) {
        commands.push(command);
        return;
    }
    graph_key(next, commands, key);
}

/// The detail pane, each overlay, then the overview strip: the panes that
/// claim a key before the graph sees it. `true` when one of them took it.
///
/// An open overlay stands where the graph does, so it takes the keys the
/// graph would — **before** the detail pane, which otherwise claims
/// `Enter`, `s`, `d` and `Space` for itself and leaves `⏎ apply` doing
/// nothing but opening the pane. While the pane has focus the pane keeps
/// them, and `Tab` is never claimed by an overlay, so focus can always get
/// back there (m3-working-tree-undo-and-stash/06).
fn pane_and_overlay_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let overlay_first = next.detail.focus == DetailFocus::Graph;
    if overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)) {
        return true;
    }
    super::detail::detail_key(next, commands, key)
        || super::map::map_key(next, commands, key)
        || (!overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)))
        || super::strip::strip_key(next, key)
}

/// `u`/`U`, with `local_jobs` borrowed out for the closure the way every
/// other call site here does it.
fn undo_key(next: &mut AppState, commands: &mut Vec<Command>, redo: bool) {
    let mut local_jobs = next.local_jobs;
    commands.extend(super::undo::undo_key(next, redo, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// The commit editor's keys, with `local_jobs` borrowed out the same way.
fn editor_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::commit_editor::editor_key(next, &key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
        .map(|editor_commands| commands.extend(editor_commands))
        .is_some()
}

/// The journal browser's keys, the same way.
fn journal_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::journal::journal_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The stash overlay's keys, the same way.
fn stash_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed = super::stash::stash_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The graph's own keys, once the detail pane has passed on the key.
fn graph_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    if is_quit(key) {
        next.quit = Some(Quit::Requested);
    } else if is_refresh(key) {
        let mut local_jobs = next.local_jobs;
        refresh_key(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
    } else if is_screen_toggle(key) {
        next.screen = next.screen.toggled();
    } else if is_lane_toggle(key) {
        next.lanes_expanded = !next.lanes_expanded;
    } else if is_focus_toggle(key) {
        next.focus = !next.focus;
    } else if is_tier_cycle(key) {
        cycle_tier(next, commands);
    } else if is_copy_key(key) {
        copy_selected_sha(next, commands);
    } else if is_compare_key(key) {
        toggle_compare_selected(next, commands);
    } else if is_context_menu_key(key) {
        super::context_menu::open_selected(next);
    } else if let Some(amend) = is_commit_key(key) {
        commit_key(next, commands, amend);
    } else if write_key(next, commands, key) {
    } else if let Some(change) = view_change(key) {
        apply_view_change(next, commands, change);
    } else if let Some(motion) = motion_for(key) {
        move_cursor(next, commands, motion);
    } else if key.code == KeyCode::Escape {
        escape_key(next, commands);
    }
}

/// The keys that change the repository: `m`/`r`, which end in a Ghost
/// Preview rather than straight in a write (m5-history-and-ghost-preview/08),
/// `b`/`t`, which end in the name prompt, and the three remote keys. `true`
/// when one of them answered.
///
/// `local_jobs` is borrowed out because the closure and `next` cannot
/// overlap.
fn write_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if let Some(tag) = is_name_key(key) {
        super::name_prompt::open_from_graph(next, tag);
        return true;
    }
    if let Some(which) = super::preview::is_history_key(key) {
        let mut local_jobs = next.local_jobs;
        super::preview::history_key(next, commands, which, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        return true;
    }
    if super::preview::is_take_key(key) {
        // `o` (keymap §2.3, m7-power-tools/11): take the cursor commit onto
        // the branch, previewed — its own fn rather than a `HistoryKey`, so
        // place mode (which reuses `HistoryKey`) leaves `o` unbound.
        let mut local_jobs = next.local_jobs;
        super::preview::take_key(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        return true;
    }
    if is_rewrite_key(key) && next.graph_has_focus() && next.graph_overlay().is_none() {
        // `R` (keymap §2.3, m7-power-tools/08): the rewrite editor on the
        // cursor's row, once its read answers.
        let mut local_jobs = next.local_jobs;
        super::rewrite_editor::open_rewrite(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        return true;
    }
    if let Some(remote) = is_remote_key(key) {
        remote_key(next, commands, remote);
        return true;
    }
    false
}

/// A view key (order, filter, stash visibility): asks the history job for
/// the rows the changed view shows.
fn apply_view_change(next: &mut AppState, commands: &mut Vec<Command>, change: ViewChange) {
    let height = next.graph_rows();
    let stashes: Vec<Oid> = next.refs.as_deref().map_or_else(Vec::new, |refs| {
        refs.stashes().iter().map(|&(commit, _)| commit).collect()
    });
    if let Some(history) = next.history.as_mut()
        && let Some(view) = change.apply(history, &stashes)
        && let Some(request) = history.change_view(view, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// One remote key, with `local_jobs` borrowed out for the closure.
fn remote_key(next: &mut AppState, commands: &mut Vec<Command>, remote: RemoteKey) {
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    let _claimed: bool = match remote {
        RemoteKey::Fetch => super::remote::fetch_key(next, commands, &mut next_id),
        RemoteKey::Pull => super::remote::pull_key(next, commands, &mut next_id),
        RemoteKey::Push => super::remote::push_key(next),
    };
    next.local_jobs = local_jobs;
}

/// A graph motion key: the working-tree row's flag (`update::selection`)
/// sees it first, then whatever motion is left goes to `History`.
fn move_cursor(next: &mut AppState, commands: &mut Vec<Command>, motion: Motion) {
    let Some(motion) = super::selection::move_from_graph(next, motion) else {
        return;
    };
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.move_cursor(motion, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}
