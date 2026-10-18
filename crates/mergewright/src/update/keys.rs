//! The graph's own keys: movement, view changes and the toggles that are not
//! the search prompt or the commit-graph offer (those are `search` and
//! `commit_graph`).

use mergewright_core::event::{KeyCode, KeyPress, Modifiers};

use super::events::Command;
use super::state::{AppState, Status};

mod acknowledge;
mod actions;
#[cfg(test)]
mod bound;
mod commit;
pub(crate) mod edit_stop;
#[cfg(test)]
mod edit_stop_tests;
mod escape;
#[cfg(test)]
mod live_c_tests;
mod modal;
mod movement;
mod panes;
mod predicates;
mod refresh;
mod remote;
#[cfg(test)]
mod tab_words_tests;
#[cfg(test)]
mod table_check;
#[cfg(test)]
mod tests;
pub(in crate::update) mod view_change;

pub(in crate::update) use acknowledge::acknowledge;
use acknowledge::dismiss_unseen;
pub(in crate::update) use actions::strip_rows_for;
use actions::{copy_selected_sha, cycle_tier, toggle_compare_selected};
#[cfg(test)]
pub(crate) use bound::bound_actions;
use commit::commit_key;
use escape::escape_key;
use modal::{
    banner_key, boxes_key, finish_key, place_claim, resolver_key, rewrite_key, undo_key_anywhere,
};
use movement::move_cursor;
use panes::{editor_key, pane_and_overlay_key};
use predicates::{
    is_commit_key, is_compare_key, is_context_menu_key, is_copy_key, is_focus_toggle,
    is_lane_toggle, is_name_key, is_quit, is_refresh, is_remote_key, is_rewrite_key,
    is_screen_toggle, is_tier_cycle, is_undo_key, motion_for,
};
pub(in crate::update) use refresh::refresh_key;
use remote::remote_key;
use view_change::{apply_view_change, view_change};

/// What every key does to the bottom row before anything answers it — the
/// edge's unbound keys included (`edge::unanswered`). `true` when the key is
/// spent doing it.
///
/// An unseen failure on the row is what a key answers first: `Esc` only
/// dismisses it; any other key dismisses it and acts, without also
/// acknowledging the status under it (m6-conflicts/78, Decided 1).
pub(super) fn before_any_key(next: &mut AppState, key: &KeyPress) -> bool {
    let unseen_shown = next.unseen.is_some() && !next.modal_owns_bottom_row();
    if dismiss_unseen(next, key) {
        return true;
    }
    if !unseen_shown {
        acknowledge(next);
    }
    false
}

/// The half of [`super::core_event`] that answers a key.
///
/// The detail pane answers its keys first (`Enter` opens it, ADR-0023).
///
/// A blame place on screen is a modal pane (keymap §2.1, [`place_claim`]).
pub(super) fn key_event(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    if before_any_key(next, &key) {
        return;
    }
    // The modal claims, in order, each returning `true` once it has taken
    // the key: a badge in the air is dropped by any key before that key
    // means anything (`drag::drag_key`; only `Esc` is spent doing it), then
    // the confirm box and the preview, then the resolver, then the focused
    // banner, then a blame place on screen. `||` is the early return, one
    // claim per line.
    if super::drag::drag_key(next, key)
        // `?`, `⌃P`, `:` open the palette (§2.12) over every place but
        // a live confirm box, before any of them can claim the key: help
        // answers on every screen (§2.2).
        || super::palette::open_key(next, commands, key)
        || boxes_key(next, commands, key)
        // `Esc` while Mergewright waits to quit drops the wait, whatever modal is
        // up — the finish summary, the resolver, the rewrite editor each
        // spend `Esc` on themselves otherwise, and on the summary while its
        // commit runs it was swallowed (review m8-13b Major 2). After the
        // box (its `Esc` closes it, the wait standing) and before every
        // pane: the first `Esc` is the wait's, as on the graph.
        || (key.code == KeyCode::Escape && super::quit::drop_wait(next))
        || rewrite_key(next, commands, key)
        || finish_key(next, commands, key)
        || resolver_key(next, commands, key)
        // A shown picker is a text field (keymap §2.1): it claims `Tab`
        // before the banner's `take_focus` could (review m7-10-to-07c
        // Major 1). Opening it on `C` stays below, with the menu.
        || (super::picker::shown(next) && super::picker::picker_key(next, commands, key))
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
    // Then the branch picker (`C`, m7-power-tools/10): on the graph `C`
    // opens it. Once shown it claims every key but `Ctrl+C` — above, ahead
    // of the banner.
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
        undo_key_anywhere(next, commands, redo);
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

/// The graph's own keys, once the detail pane has passed on the key.
fn graph_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    if is_quit(key) {
        super::quit::asked(next, key);
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
    } else if write_key(next, commands, key) || view_key(next, commands, key) {
    } else if let Some(motion) = motion_for(key) {
        move_cursor(next, commands, motion);
    } else if key.code == KeyCode::Escape {
        escape_key(next, commands);
    }
}

/// The view's keys (`view_change`), `space` first asked of the edit stop:
/// on the stopped row or the Uncommitted row there, `space` stages every
/// change (07d D1); elsewhere it folds. `true` when one of them answered.
fn view_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if edit_stop::space_key(next, commands, key) {
        return true;
    }
    let Some(change) = view_change(key) else {
        return false;
    };
    apply_view_change(next, commands, change);
    true
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
        // the branch, previewed, through its own fn (`preview::take_key`),
        // not a `HistoryKey`.
        let mut local_jobs = next.local_jobs;
        super::preview::take_key(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
        return true;
    }
    if is_rewrite_key(key) && next.graph_overlay().is_none() {
        // `R` (keymap §2.3, m7-power-tools/08): the rewrite editor on the
        // cursor's row, once its read answers. From a pane the keyboard goes
        // to the graph first, as `S`/`J` do (m6/85, m8/09e), so `Esc` out of
        // the editor lands on the graph with the pane still open.
        next.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
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
