//! The gestures a scenario makes that are more than one key: moving the
//! cursor to a row, picking a verb from a menu, dragging a badge, staging a
//! file through the changes pane.

use graft::update::AppState;
use graft::update::detail::Loading;
use graft_core::event::{KeyCode, MouseButton, MouseKind};

use super::run::Walk;
use crate::repo_truth::check;

impl Walk<'_> {
    /// `g`, then `j` until `here` holds of the selected row.
    pub(super) fn cursor_to(
        &mut self,
        state: &AppState,
        here: impl Fn(&AppState) -> bool,
    ) -> Result<AppState, String> {
        let mut state = self.app.key(state, KeyCode::Char('g'));
        for _ in 0..40 {
            if here(&state) {
                return Ok(state);
            }
            state = self.app.key(&state, KeyCode::Char('j'));
        }
        Err(format!(
            "forty rows down and never found it: {:?}",
            subjects(&state)
        ))
    }

    /// `.` (unless the menu is already open), `j` to the verb whose label
    /// starts with `prefix`, `Enter`.
    pub(super) fn menu_pick(
        &mut self,
        state: AppState,
        prefix: &str,
        open: bool,
    ) -> Result<AppState, String> {
        let mut state = if open {
            self.app.key(&state, KeyCode::Char('.'))
        } else {
            state
        };
        let labels: Vec<String> = state
            .context_menu
            .as_ref()
            .ok_or_else(|| format!("`.` opened no menu; status {:?}", state.status))?
            .menu
            .items
            .iter()
            .map(|item| item.label.to_string())
            .collect();
        for _ in 0..labels.len() {
            let menu = &state
                .context_menu
                .as_ref()
                .ok_or_else(|| "the menu closed while moving in it".to_owned())?
                .menu;
            if menu.items[menu.cursor].label.starts_with(prefix) {
                return Ok(self.app.key(&state, KeyCode::Enter));
            }
            state = self.app.key(&state, KeyCode::Char('j'));
        }
        Err(format!("no menu row starts with {prefix:?}: {labels:?}"))
    }

    /// Pick the first draggable badge up with the mouse, drag it onto the
    /// row carrying `badge`, release: three real mouse events.
    pub(super) fn drag_badge_onto(
        &mut self,
        state: &AppState,
        badge: &str,
    ) -> Result<AppState, String> {
        let (state, column, from_screen_row) = self.grab_a_badge(state)?;
        let from_row = state
            .drag
            .as_ref()
            .map(|drag| drag.from_row)
            .ok_or_else(|| "no badge is in the air".to_owned())?;
        let onto = row_of_badge(&state, badge)
            .ok_or_else(|| format!("no row carries {badge}: {:?}", subjects(&state)))?;
        check(onto != from_row, || {
            format!("{badge} is on the row the badge came from")
        })?;
        // The drawn rows are consecutive, so the target's screen row is the
        // press's, moved by the distance between the two row indices.
        let distance = i32::try_from(onto).map_err(|why| why.to_string())?
            - i32::try_from(from_row).map_err(|why| why.to_string())?;
        let onto_screen_row = u16::try_from(i32::from(from_screen_row) + distance)
            .map_err(|_| format!("{badge}'s row is above the screen"))?;
        let state = self.app.mouse(
            &state,
            MouseKind::Drag(MouseButton::Left),
            column,
            onto_screen_row,
        );
        check(
            state.drag.as_ref().and_then(|drag| drag.over) == Some(onto),
            || format!("the badge is not over {badge}'s row"),
        )?;
        let state = self.app.mouse(
            &state,
            MouseKind::Up(MouseButton::Left),
            column,
            onto_screen_row,
        );
        check(state.context_menu.is_some(), || {
            format!("the release opened no drop menu: {:?}", state.status)
        })?;
        Ok(state)
    }

    fn grab_a_badge(&mut self, state: &AppState) -> Result<(AppState, u16, u16), String> {
        for screen_row in 1..8_u16 {
            for column in 0..self.columns {
                let pressed = self.app.mouse(
                    state,
                    MouseKind::Down(MouseButton::Left),
                    column,
                    screen_row,
                );
                if pressed.drag.is_some() {
                    return Ok((pressed, column, screen_row));
                }
            }
        }
        Err(format!(
            "no draggable badge in the top rows of a {}-column frame",
            self.columns
        ))
    }

    /// The changes pane from the top row, `Space` on `name`, the write,
    /// the list re-read, `Esc` back to the graph.
    pub(super) fn stage(&mut self, state: &AppState, name: &str) -> Result<AppState, String> {
        let state = self.app.key(state, KeyCode::Char('g'));
        let state = if state.on_working_tree {
            state
        } else {
            self.app.key(&state, KeyCode::Char('k'))
        };
        check(state.on_working_tree, || {
            "`k` from the top row did not select the uncommitted-changes row".to_owned()
        })?;
        let state = self.app.key(&state, KeyCode::Enter);
        let state = self.pump(state, "the changes pane's file list", |state| {
            matches!(state.detail.files, Loading::Ready(_))
        })?;
        let state = crate::app_harness::cursor_to(self.app, state, name);
        let state = self.app.key(&state, KeyCode::Char(' '));
        let state = self.settle_write(state, &format!("staging {name}"))?;
        let state = self.pump(state, "the file list after the stage", |state| {
            matches!(state.detail.files, Loading::Ready(_)) && state.journal.reading.is_none()
        })?;
        Ok(self.app.key(&state, KeyCode::Escape))
    }
}

/// The selected row carries the branch or tag badge `name`.
pub(super) fn on_badge(state: &AppState, name: &str) -> bool {
    let Some(refs) = state.refs.as_deref() else {
        return false;
    };
    let Some(row) = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
    else {
        return false;
    };
    refs.on(row.commit.id).iter().any(|label| {
        matches!(label, graft::refs::RefLabel::Branch { name: found, .. } if found == name.as_bytes())
            || matches!(label, graft::refs::RefLabel::Tag(found) if found == name.as_bytes())
    })
}

/// The selected row's subject is `subject`.
pub(super) fn on_subject(state: &AppState, subject: &str) -> bool {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .is_some_and(|row| row.commit.subject() == subject.as_bytes())
}

/// Which drawn row carries the branch badge `name`.
fn row_of_badge(state: &AppState, name: &str) -> Option<usize> {
    let refs = state.refs.as_deref()?;
    let window = state.history.as_ref()?.window.as_ref()?;
    window.rows.iter().position(|row| {
        refs.on(row.commit.id).iter().any(|label| {
            matches!(label, graft::refs::RefLabel::Branch { name: found, .. } if found == name.as_bytes())
        })
    })
}

/// The drawn rows' subjects, for a failure message.
fn subjects(state: &AppState) -> Vec<String> {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| {
            window
                .rows
                .iter()
                .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
                .collect()
        })
        .unwrap_or_default()
}
