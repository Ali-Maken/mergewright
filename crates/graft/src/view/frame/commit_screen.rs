//! The commit screen over the whole frame — split out of `boxes`
//! (ease-pass/08s M6).

// One file of `frame`, split for size: it shares the parent's imports and
// siblings rather than restating them.
#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The commit screen, over the whole frame (ease-pass/08, the commit mock):
/// it shows the diff of the file under its cursor itself, so the detail
/// pane is not drawn beside it. Always `None`: it draws no pixels,
/// whatever the terminal can do.
pub(super) fn draw_commit_editor(
    session: &Session,
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
) -> Option<Rect> {
    let view = crate::view::commit_screen_view(state)?;
    // A text field's keys are fixed (ADR-0055): its bar spells the shipped
    // table, never a user's rebinding of the same letters.
    let screen = graft_ui::commit_editor::CommitScreen {
        view: &view,
        theme: &session.theme,
        keys: graft_ui::keymap::Keymap::shipped(),
    };
    frame.render_widget(Clear, area);
    frame.render_widget(&screen, area);
    None
}
