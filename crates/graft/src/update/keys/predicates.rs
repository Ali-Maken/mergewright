//! Which key is which: the pure `KeyPress` tests the graph's dispatch asks,
//! split out of `keys` (review m6-amend-16 M3).

use graft_core::event::{KeyCode, KeyPress};

use crate::history::Motion;

/// The three remote keys (`docs/design/02-keymap.md` §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RemoteKey {
    /// `f`.
    Fetch,
    /// `p`.
    Pull,
    /// `P`.
    Push,
}

/// `f` fetch, `p` pull, `P` push — plain keys only.
pub(super) fn is_remote_key(key: KeyPress) -> Option<RemoteKey> {
    if key.modifiers.alt || key.modifiers.control {
        return None;
    }
    match key.code {
        KeyCode::Char('f') => Some(RemoteKey::Fetch),
        KeyCode::Char('p') => Some(RemoteKey::Pull),
        KeyCode::Char('P') => Some(RemoteKey::Push),
        _ => None,
    }
}

/// The movement a key asks for, if it is one (DESIGN.md §2.2): vim keys and
/// arrows, a screen at a time, and the ends.
pub(super) fn motion_for(key: KeyPress) -> Option<Motion> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Motion::Down(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Motion::Up(1)),
        KeyCode::PageDown => Some(Motion::PageDown),
        KeyCode::PageUp => Some(Motion::PageUp),
        KeyCode::Char('g') | KeyCode::Home => Some(Motion::Top),
        KeyCode::Char('G') | KeyCode::End => Some(Motion::Bottom),
        _ => None,
    }
}

/// The two keys that mean "leave" (DESIGN.md §2.2).
///
/// `Ctrl+C` is here as well as `q` because a terminal application that ignores
/// it feels broken, and because raw mode means the terminal no longer turns it
/// into a signal on Graft's behalf.
pub(super) fn is_quit(key: KeyPress) -> bool {
    let plain_q = key.code == KeyCode::Char('q') && !key.modifiers.control && !key.modifiers.alt;
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    plain_q || control_c
}

/// `F2`: swap between the graph and the status screen. Not one of DESIGN.md
/// §2.2–§2.3's keys, which are all taken; the status screen is a diagnostic.
pub(super) fn is_screen_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(2) && !key.modifiers.control && !key.modifiers.alt
}

/// `+` expands the lanes folded behind `+N`, and folds them again
/// (DESIGN.md §3.5).
/// `F`: Focus mode on the selected commit (DESIGN.md §2.3, §5.2).
pub(super) fn is_focus_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('F') && !key.modifiers.control && !key.modifiers.alt
}

pub(super) fn is_lane_toggle(key: KeyPress) -> bool {
    key.code == KeyCode::Char('+') && !key.modifiers.control && !key.modifiers.alt
}

/// `F3`: cycle the renderer tier — pixels, Unicode, ASCII, then back to the
/// terminal's own tier — and say so (DESIGN.md §4.2 — config wins over
/// detection, and a key press wins the same way over both; tickets
/// m1-the-graph/19, 24d, 33). Not one of DESIGN.md's keymap, like `F2`.
pub(super) fn is_tier_cycle(key: KeyPress) -> bool {
    key.code == KeyCode::Function(3) && !key.modifiers.control && !key.modifiers.alt
}

/// `y`: copy the selected commit's full SHA (vim's yank; ADR-0023).
pub(super) fn is_copy_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('y') && !key.modifiers.control && !key.modifiers.alt
}

/// `x`: mark the selected row as a compare base, or complete/clear a compare
/// against it (`compare-two-commits`, m2-details-and-diffs/11).
pub(super) fn is_compare_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('x') && !key.modifiers.control && !key.modifiers.alt
}

/// `b` names a new branch at the cursor's commit, `t` a new tag
/// (`docs/design/02-keymap.md` §2.3). `Some(true)` for the tag.
///
/// Both open the name prompt and nothing else: a branch is created only once
/// there is a name to create it under, so neither key writes by itself.
pub(super) fn is_name_key(key: KeyPress) -> Option<bool> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('b') => Some(false),
        KeyCode::Char('t') => Some(true),
        _ => None,
    }
}

/// `R` opens the rewrite editor on the cursor's row (keymap §2.3,
/// m7-power-tools/08).
pub(super) fn is_rewrite_key(key: KeyPress) -> bool {
    !key.modifiers.control && !key.modifiers.alt && key.code == KeyCode::Char('R')
}

/// `c` opens the commit editor, `a` opens it amending `HEAD` (DESIGN.md
/// §2.3; m3-working-tree-undo-and-stash/05). `Some(amend)` says which.
pub(super) fn is_commit_key(key: KeyPress) -> Option<bool> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('c') => Some(false),
        KeyCode::Char('a') => Some(true),
        _ => None,
    }
}

/// `.`: open the context menu for the selected row (`context-actions-menu`,
/// m2-details-and-diffs/12).
pub(super) fn is_context_menu_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('.') && !key.modifiers.control && !key.modifiers.alt
}

/// `u`/`Ctrl+Z` undo, `U` redo (DESIGN.md §2.2, ADR-0002). `Some(redo)` says
/// which.
///
/// `Ctrl+Shift+Z` is not bound: many terminals collapse it to `Ctrl+Z`'s own
/// bytes, so Graft cannot tell the two apart (§2.2's own note). `U` is the
/// binding that works everywhere.
pub(super) fn is_undo_key(key: KeyPress) -> Option<bool> {
    if key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char('u') if !key.modifiers.control => Some(false),
        KeyCode::Char('U') if !key.modifiers.control => Some(true),
        KeyCode::Char('z') if key.modifiers.control => Some(false),
        _ => None,
    }
}

/// Manual refresh (DESIGN.md §2.1): re-read the repository now, rather than
/// waiting for whatever would have noticed on its own.
pub(super) fn is_refresh(key: KeyPress) -> bool {
    key.code == KeyCode::Char('r') && key.modifiers.control
}
