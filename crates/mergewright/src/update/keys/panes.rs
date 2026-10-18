//! The detail pane and the overlays: the panes that claim a key before
//! the graph sees it.

use mergewright_core::event::KeyPress;

use mergewright_ui::detail::DetailFocus;

use crate::update::events::Command;
use crate::update::state::AppState;

/// The detail pane, each overlay, then the overview strip: the panes that
/// claim a key before the graph sees it. `true` when one of them took it.
///
/// An open overlay stands where the graph does, so it takes the keys the
/// graph would — **before** the detail pane, which otherwise claims
/// `Enter`, `s`, `d` and `Space` for itself and leaves `⏎ apply` doing
/// nothing but opening the pane. While the pane has focus the pane keeps
/// them, and `Tab` is never claimed by an overlay, so focus can always get
/// back there (m3-working-tree-undo-and-stash/06).
pub(super) fn pane_and_overlay_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let overlay_first = next.detail.focus == DetailFocus::Graph;
    if overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)) {
        return true;
    }
    crate::update::detail::detail_key(next, commands, key)
        || crate::update::map::map_key(next, commands, key)
        || (!overlay_first && (journal_key(next, commands, key) || stash_key(next, commands, key)))
        || crate::update::strip::strip_key(next, key)
}

/// The commit editor's keys, with `local_jobs` borrowed out the same way.
pub(super) fn editor_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::commit_editor::editor_key(next, &key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
        .map(|editor_commands| commands.extend(editor_commands))
        .is_some()
}

/// The journal browser's keys, the same way.
fn journal_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::journal::journal_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}

/// The stash overlay's keys, the same way.
fn stash_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    let mut local_jobs = next.local_jobs;
    let claimed =
        crate::update::stash::stash_key(next, commands, key, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    claimed
}
