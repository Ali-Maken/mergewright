//! `O`: the overview strip's only key (`overview-strip`, m2-details-and-diffs/20).
//!
//! Nothing else lives here — the strip has no state beyond the one flag on
//! [`super::state::AppState`]; the marks it draws are built fresh from
//! `view::strip::strip_marks` each frame, the same way `view::map::map_view`
//! rebuilds the Map view.

use graft_core::event::{KeyCode, KeyPress};

use graft_ui::detail::DetailFocus;

use super::state::AppState;

/// `O` on the graph screen (no modifier, focus on the graph, no map shown)
/// flips [`AppState::strip_shown`]. `true` once handled, so `key_event`
/// stops dispatching.
pub(super) fn strip_key(next: &mut AppState, key: KeyPress) -> bool {
    let plain = !key.modifiers.control && !key.modifiers.alt;
    if !plain || next.map.shown || next.detail.focus != DetailFocus::Graph {
        return false;
    }
    if key.code == KeyCode::Char('O') {
        next.strip_shown = !next.strip_shown;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use graft_core::event::Modifiers;

    use super::*;

    fn key(code: KeyCode) -> KeyPress {
        KeyPress {
            code,
            modifiers: Modifiers::default(),
        }
    }

    #[test]
    fn o_toggles_strip_shown_on_and_off() {
        let mut state = AppState::default();
        assert!(!state.strip_shown);
        assert!(strip_key(&mut state, key(KeyCode::Char('O'))));
        assert!(state.strip_shown);
        assert!(strip_key(&mut state, key(KeyCode::Char('O'))));
        assert!(!state.strip_shown);
    }

    #[test]
    fn o_is_ignored_while_the_map_is_shown() {
        let mut state = AppState::default();
        state.map.shown = true;
        assert!(!strip_key(&mut state, key(KeyCode::Char('O'))));
        assert!(!state.strip_shown);
    }

    #[test]
    fn o_is_ignored_off_the_graph_focus() {
        let mut state = AppState::default();
        state.detail.focus = DetailFocus::Files;
        assert!(!strip_key(&mut state, key(KeyCode::Char('O'))));
    }

    #[test]
    fn other_keys_are_not_handled() {
        let mut state = AppState::default();
        assert!(!strip_key(&mut state, key(KeyCode::Char('o'))));
        assert!(!state.strip_shown);
    }
}
