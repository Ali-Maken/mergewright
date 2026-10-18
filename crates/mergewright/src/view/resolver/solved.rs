//! The line under a file whose every clash is picked (ease-pass/11, mock
//! M3): `✓ Every clash in receipt.rs is solved.` and, in a merge, the way
//! on — `n next file, N clashes left`, or `y review the merge`.

use mergewright_git::conflict::ConflictFile;
use mergewright_term::sanitize_line;
use mergewright_ui::conflict_resolver::RegionView;
use mergewright_ui::keymap::ActionId;

use crate::update::AppState;
use crate::update::resolver::review::{in_a_merge, next_open_file, open_clashes};

/// The solved line for `file`, open at `path`: `None` while a clash in it
/// is open, or it has none (a file with no blocks has nothing solved).
pub(super) fn solved_line(
    state: &AppState,
    path: &std::path::Path,
    file: &ConflictFile,
) -> Option<RegionView> {
    if file.blocks().is_empty() || file.open() > 0 {
        return None;
    }
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    Some(RegionView::Solved {
        name: sanitize_line(&name),
        next: sanitize_line(&way_on(state)),
    })
}

/// A merge's way on from a solved file, each key from his keymap (left out
/// unbound): `n` while a clash may be open in another file, with the count
/// the resolver has read; else `y` to the review. Elsewhere none — the
/// file still waits for `⏎`, which the bar names.
fn way_on(state: &AppState) -> String {
    if !in_a_merge(state) {
        return String::new();
    }
    let keys = &state.keymap;
    if next_open_file(state).is_some() {
        let Some(n) = keys.spell_keys(&[ActionId("resolver.next_block")]) else {
            return String::new();
        };
        return match open_clashes(state) {
            // Only unread files are left: no count to give.
            0 => format!("{n} next file"),
            1 => format!("{n} next file, 1 clash left"),
            left => format!("{n} next file, {left} clashes left"),
        };
    }
    keys.spell_keys(&[ActionId("resolver.review")])
        .map(|y| format!("{y} review the merge"))
        // Not a failure: an unbound key has nothing to name (m8/10g).
        .unwrap_or_default()
}
