//! The commit screen owns the mouse (ease-pass/08s M5 + Q2, mock
//! `mergewright-commit-screen.html:146`): a left click on a file row picks
//! it as `↑`/`↓` would, a click in the message box moves focus there, and
//! every other event while the screen is up is swallowed — the graph behind
//! it never moves its cursor, opens a menu or starts a drag.

use mergewright_core::event::{MouseButton, MouseInput, MouseKind};
use mergewright_ui::Cells;
use mergewright_ui::commit_editor::{CommitFocus, commit_layout, file_at};

use super::files::{ask_diff, display_order};
use super::{AppState, Command};
use crate::update::GraphOverlay;

/// Every mouse event while the commit screen is the screen drawn: `true`,
/// taken. `false` when it is not up — or the finish summary or the
/// resolver is drawn over it — so the event goes on down
/// `update::mouse`'s chain.
pub(crate) fn mouse(state: &mut AppState, commands: &mut Vec<Command>, mouse: MouseInput) -> bool {
    if state.graph_overlay() != Some(GraphOverlay::CommitEditor) {
        return false;
    }
    // A left press is the one that acts; not while a write runs (its keys
    // wait too) or a box stands over the screen.
    let acts = mouse.kind == MouseKind::Down(MouseButton::Left)
        && state.operating.is_none()
        && !state.modal_owns_bottom_row();
    if !acts {
        return true;
    }
    // The screen is drawn over the whole frame (`view::frame`'s
    // `draw_commit_editor`), so its layout is the terminal's size's.
    let layout = commit_layout(Cells {
        x: 0,
        y: 0,
        width: state.size.columns,
        height: state.size.rows,
    });
    let on = |cells: Cells| {
        (cells.x..cells.x.saturating_add(cells.width)).contains(&mouse.column)
            && (cells.y..cells.y.saturating_add(cells.height)).contains(&mouse.row)
    };
    let focus = if on(layout.subject) {
        CommitFocus::Subject
    } else if on(layout.body) {
        CommitFocus::Body
    } else if on(layout.files) {
        return pick_file(state, commands, layout.files, mouse.row);
    } else {
        return true;
    };
    if let Some(editor) = state.commit_editor.as_mut() {
        editor.focus = focus;
    }
    true
}

/// A click on list row `row`: the file drawn there takes the cursor and
/// focus, and its diff is asked; a heading or an empty row does nothing.
fn pick_file(state: &mut AppState, commands: &mut Vec<Command>, files: Cells, row: u16) -> bool {
    // The list as drawn — its order and its scroll — names the file.
    let Some(at) =
        crate::view::commit_screen_view(state).and_then(|view| file_at(&view, files, row))
    else {
        return true;
    };
    let Some(editor) = state.commit_editor.as_mut() else {
        return true;
    };
    let Some(index) = display_order(&editor.files).get(at).copied() else {
        return true;
    };
    editor.focus = CommitFocus::Files;
    editor.file_cursor = index;
    let mut jobs = state.local_jobs;
    ask_diff(state, commands, &mut || jobs.next_id());
    state.local_jobs = jobs;
    true
}

#[cfg(test)]
mod tests;
