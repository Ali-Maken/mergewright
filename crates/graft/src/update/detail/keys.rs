//! The pane's own keys: divider resize, focus cycling, the files/diff
//! region's own bindings, and the view-option keys both regions share.

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_ui::detail::{DetailFocus, DiffMode};

use crate::update::events::Command;
use crate::update::state::AppState;

use super::control::{diff_content_len_of, effective_layout};
use super::state::{DetailSpec, DetailState, Loading};

/// `PageUp`/`PageDown` step, in rows, for the files list and the diff body.
const PAGE_STEP: isize = 10;
/// `]`'s and `[`'s ceiling on context lines: past this, more context stops
/// changing what most diffs show.
const MAX_CONTEXT: u8 = 20;

/// `Esc`'s two priority arms, answered before anything else `Esc` does:
/// closing the Explain panel first (`m2-details-and-diffs/17`, the pane
/// stays, on the same commit), then clearing a pinned compare base
/// (`m2-details-and-diffs/11`, closing the pane in the same press when it
/// is shown — ADR-0023's own `Escape if detail.shown` arm, further down
/// [`detail_key`]). `true` when either one answered the key.
fn escape_priority(next: &mut AppState, key: KeyPress) -> bool {
    if key.code != KeyCode::Escape {
        return false;
    }
    if next.detail.explain.is_some() {
        super::control::close_explain(next);
        return true;
    }
    if next.compare_base.take().is_some() {
        if next.detail.shown {
            next.detail.close();
        }
        return true;
    }
    false
}

/// The pane's keys, answered before the graph's. `true` when the key was the pane's.
pub(crate) fn detail_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let plain = key.modifiers == Modifiers::NONE;
    // `<`/`>` (ADR-0028): the focused divider, whichever region has focus,
    // whenever the pane is shown — the pane being shown is what makes the
    // divider exist. A no-op (nothing changes, no command) in the `Tab`
    // layout, hidden, or against a bound: `resize_pane` says so itself.
    if plain && next.detail.shown && matches!(key.code, KeyCode::Char('<' | '>')) {
        let grow = key.code == KeyCode::Char('>');
        let size = (next.size.columns, next.size.rows);
        let layout = effective_layout(next.detail.focus, &next.detail.pane_size, size);
        if next.detail.resize_pane(&layout, grow, size) {
            commands.push(Command::SaveLayout(next.detail.pane_size));
        }
        return true;
    }
    if escape_priority(next, key) {
        return true;
    }
    if changes_pane_write_key(next, commands, key, plain) {
        return true;
    }
    let AppState {
        detail, local_jobs, ..
    } = next;
    let ids = &mut || local_jobs.next_id();
    match key.code {
        KeyCode::Enter if plain && detail.shown && detail.focus == DetailFocus::Diff => {
            // Diff already has focus: Enter has nothing further to open.
            true
        }
        KeyCode::Enter if plain && detail.shown && detail.focus == DetailFocus::Files => {
            detail.focus = DetailFocus::Diff;
            true
        }
        KeyCode::Enter if plain => {
            let Some(spec) = next.selected_spec() else {
                return false;
            };
            let AppState {
                detail, local_jobs, ..
            } = next;
            commands.extend(detail.open(spec, &mut || local_jobs.next_id()));
            true
        }
        KeyCode::Escape if detail.shown => {
            detail.close();
            true
        }
        KeyCode::Tab if detail.shown => {
            detail.cycle_focus(plain);
            true
        }
        KeyCode::BackTab if detail.shown => {
            detail.cycle_focus(false);
            true
        }
        _ if detail.shown && detail.focus == DetailFocus::Files => {
            files_key(detail, commands, ids, key.code, plain)
        }
        _ if detail.shown && detail.focus == DetailFocus::Diff => {
            diff_key(detail, commands, ids, key.code, plain)
        }
        _ => false,
    }
}

/// Write keys on the changes pane's file list (m3-working-tree-undo-and-
/// stash/04), tried before the ordinary files keys in [`files_key`]: a write
/// that cannot apply (no write in flight already, a file row under the
/// cursor) falls through to `Space`'s other meaning, folding a directory.
/// `d` and `Ctrl+A` have no other meaning here, so they are safe to answer
/// unconditionally once the pane is on the changes pane's files.
fn changes_pane_write_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    plain: bool,
) -> bool {
    if !(next.detail.shown
        && next.detail.focus == DetailFocus::Files
        && next.detail.spec == Some(DetailSpec::WorkingTree))
    {
        return false;
    }
    let mut local_jobs = next.local_jobs;
    let mut ids = || local_jobs.next_id();
    let handled = if plain && matches!(key.code, KeyCode::Char(' ' | 's')) {
        super::super::operate::stage_selected(next, commands, &mut ids)
    } else if plain && key.code == KeyCode::Char('d') {
        super::super::operate::request_discard(next)
    } else if key.code == KeyCode::Char('a') && key.modifiers == Modifiers::CONTROL {
        super::super::operate::stage_all(next, commands, &mut ids)
    } else {
        false
    };
    next.local_jobs = local_jobs;
    handled
}

/// Keys active with the files list focused: cursor movement, paging, home/end
/// and `Space` to fold or unfold a directory row.
fn files_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            commands.extend(detail.move_files_cursor(1, ids));
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            commands.extend(detail.move_files_cursor(-1, ids));
            true
        }
        KeyCode::PageDown => {
            commands.extend(detail.move_files_cursor(PAGE_STEP, ids));
            true
        }
        KeyCode::PageUp => {
            commands.extend(detail.move_files_cursor(-PAGE_STEP, ids));
            true
        }
        KeyCode::Char('g') if plain => {
            commands.extend(detail.move_files_cursor(isize::MIN, ids));
            true
        }
        KeyCode::Char('G') if plain => {
            commands.extend(detail.move_files_cursor(isize::MAX, ids));
            true
        }
        KeyCode::Char(' ') if plain => {
            if let Loading::Ready(files) = &mut detail.files {
                let cursor = files.cursor;
                std::sync::Arc::make_mut(files).toggle_dir(cursor);
            }
            true
        }
        _ => diff_option_key(detail, commands, ids, code, plain),
    }
}

/// Keys active with the diff body focused: scrolling, plus the same
/// view-option keys `files_key` falls back to (both panes share them).
fn diff_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    let delta = match code {
        KeyCode::Char('j') | KeyCode::Down => Some(1),
        KeyCode::Char('k') | KeyCode::Up => Some(-1),
        KeyCode::PageDown => Some(PAGE_STEP),
        KeyCode::PageUp => Some(-PAGE_STEP),
        KeyCode::Char('g') if plain => Some(isize::MIN),
        KeyCode::Char('G') if plain => Some(isize::MAX),
        _ => None,
    };
    if let Some(delta) = delta {
        let len = diff_content_len_of(detail);
        detail.scroll_body(delta, len);
        return true;
    }
    diff_option_key(detail, commands, ids, code, plain)
}

/// View-option keys shared by both the files and diff panes (anywhere the
/// pane is shown and not focused on the graph): mode, whitespace, context and
/// the full-file load.
fn diff_option_key(
    detail: &mut DetailState,
    commands: &mut Vec<Command>,
    ids: &mut impl FnMut() -> JobId,
    code: KeyCode,
    plain: bool,
) -> bool {
    if !plain {
        return false;
    }
    match code {
        KeyCode::Char('v') => {
            detail.mode = match detail.mode {
                DiffMode::Unified => DiffMode::SideBySide,
                DiffMode::SideBySide => DiffMode::Unified,
            };
            true
        }
        KeyCode::Char('w') => {
            let mut options = detail.options;
            options.ignore_whitespace = !options.ignore_whitespace;
            commands.extend(detail.set_options(options, ids));
            true
        }
        // `word_tint` has no place in `graft_git::diff::DiffOptions` (a
        // git-query option, not a display one) — `W` flips it directly and
        // re-renders from the diff already in hand, no re-fetch.
        KeyCode::Char('W') => {
            detail.word_tint = !detail.word_tint;
            true
        }
        KeyCode::Char('[') => {
            let mut options = detail.options;
            options.context = options.context.saturating_sub(1);
            commands.extend(detail.set_options(options, ids));
            true
        }
        KeyCode::Char(']') => {
            let mut options = detail.options;
            options.context = options.context.saturating_add(1).min(MAX_CONTEXT);
            commands.extend(detail.set_options(options, ids));
            true
        }
        KeyCode::Char('L') => {
            commands.extend(detail.load_all(ids));
            true
        }
        _ => false,
    }
}
