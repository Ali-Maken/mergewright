//! The branch picker (`C`, m7-power-tools/10; keymap §2.1, §2.3, §2.16):
//! type part of a branch's name, `⏎` switches to it through the one switch
//! rule (`update::switch`), `Tab` shows it on the graph.
//!
//! A text field: while it is up it claims every key but `Ctrl+C`, so `j`,
//! `k`, `q`, `u` and `C` are typed ([`AppState::text_field_open`] names it).
//! The rows are `mergewright_ops::switch_list`'s, read by a job when it opens and
//! filtered per key by `mergewright_ops::switch_list::filter_switch_rows`.
//!
//! A preview opened from it hides it rather than closing it: `Esc` on the
//! preview shows it again with the query and the row kept, as the journal
//! comes back under its own revert preview; `y` closes it for good. The
//! hook is [`preview_closed`] / [`preview_confirmed`], called by
//! `update::preview` where it restores the journal.

use std::sync::Arc;

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_core::ref_name::RefName;
use mergewright_ops::operation::Tip;
use mergewright_ops::switch_list::{SwitchKind, SwitchList, SwitchRow, filter_switch_rows};
use mergewright_ui::graph_screen::HintContext;
use mergewright_ui::text_input::TextField;

use super::detail::Loading;
use super::events::Command;
use super::geometry::Bounds;
use super::state::{AppState, Screen};
use super::switch::SwitchTarget;

/// The key the switch preview names when the picker opened it.
const OPENED_BY: &str = "C";

/// The picker's own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PickerState {
    /// What he has typed.
    pub query: TextField,
    /// The rows as read, in drawn order.
    pub list: Loading<Arc<SwitchList>>,
    /// Index into [`PickerState::rows`].
    pub selected: usize,
    /// A switch preview it opened is up; the picker comes back on its `Esc`.
    pub under_preview: bool,
    /// Why the last `⏎` did nothing, drawn on the box's bottom line until
    /// the next key.
    pub note: Option<String>,
    /// The last click's row and time (ms), for a double-click (blame's
    /// rule, `place::keys`).
    pub last_click: Option<(usize, i64)>,
    /// The branch [`open_on`] asked for, held until the list lands
    /// ([`listed`] puts `selected` on it); cleared by any key that moves
    /// the selection or edits the query first: his move wins (09b).
    pub wanted: Option<RefName>,
}

impl PickerState {
    /// The rows the query keeps, best first; empty while reading or after a
    /// failure (`list` says which).
    #[must_use]
    pub fn rows(&self) -> Vec<&SwitchRow> {
        match &self.list {
            Loading::Ready(list) => filter_switch_rows(list, self.query.text()),
            Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => Vec::new(),
        }
    }

    /// The row under the cursor.
    #[must_use]
    pub fn selected_row(&self) -> Option<&SwitchRow> {
        self.rows().get(self.selected).copied()
    }

    /// Whether `row` is the list's previous branch (`↩ before`).
    #[must_use]
    pub fn is_previous(&self, row: &SwitchRow) -> bool {
        matches!(&self.list, Loading::Ready(list)
            if list.previous.as_ref().is_some_and(|previous| previous.name == row.name))
    }
}

/// The picker is on screen and has the keyboard: up, and not hidden under a
/// preview it opened.
#[must_use]
pub fn shown(state: &AppState) -> bool {
    state
        .picker
        .as_ref()
        .is_some_and(|picker| !picker.under_preview)
}

/// The footer's context while the picker is [`shown`]: on HEAD's own row
/// `⏎` closes rather than switches (m8 usability Q2), so the bar says so;
/// any other row, or none yet (the list loading), is `Picker`.
#[must_use]
pub fn hint_context(state: &AppState) -> Option<HintContext> {
    let picker = state.picker.as_ref().filter(|_| shown(state))?;
    let on_current = picker.selected_row().is_some_and(|row| row.current);
    Some(if on_current {
        HintContext::PickerOnCurrent
    } else {
        HintContext::Picker
    })
}

/// Opens the picker and asks for the list. The caller has checked that `C`
/// is the graph's to answer.
pub fn open_picker(state: &mut AppState) -> Vec<Command> {
    let mut local_jobs = state.local_jobs;
    let id = local_jobs.next_id();
    state.local_jobs = local_jobs;
    state.picker = Some(PickerState {
        list: Loading::Pending(id),
        ..PickerState::default()
    });
    vec![Command::ReadSwitchList(id)]
}

/// `C` on the graph, and every key while the picker is shown. `true` when
/// the key was the picker's.
pub fn picker_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if shown(state) {
        // `Ctrl+C` is how the app is left; no field claims it.
        if key.code == KeyCode::Char('c') && key.modifiers.control {
            return false;
        }
        shown_key(state, commands, key);
        return true;
    }
    if opens(state, key) {
        // From a pane the keyboard goes to the graph first, as `S`/`J` do
        // (m6/85, m8/09e): `Esc` out of the picker lands on the graph.
        state.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
        match open_on::opens_on(state) {
            Some(branch) => commands.extend(open_on(state, &branch)),
            None => commands.extend(open_picker(state)),
        }
        return true;
    }
    false
}

/// `C` opens it on the graph screen with nothing standing over the graph
/// (§2.3), from the graph or either detail pane (m8/09e). The commit
/// editor's `C` is the editor's: the editor is an overlay, so this never
/// sees it.
fn opens(state: &AppState, key: KeyPress) -> bool {
    key.code == KeyCode::Char('C')
        && !key.modifiers.control
        && !key.modifiers.alt
        && state.screen == Screen::Graph
        && state.picker.is_none()
        && state.graph_overlay().is_none()
        && !state.modal_owns_keys()
}

fn shown_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    let Some(picker) = state.picker.as_mut() else {
        return;
    };
    picker.note = None;
    if matches!(
        key.code,
        KeyCode::Up | KeyCode::Down | KeyCode::Backspace | KeyCode::Char(_)
    ) {
        picker.wanted = None;
    }
    match key.code {
        KeyCode::Escape => close(state, commands),
        KeyCode::Up => picker.selected = picker.selected.saturating_sub(1),
        KeyCode::Down => {
            let last = picker.rows().len().saturating_sub(1);
            picker.selected = (picker.selected + 1).min(last);
        }
        KeyCode::Backspace => {
            picker.query.backspace();
            picker.selected = 0;
        }
        KeyCode::Char(character) if !key.modifiers.control && !key.modifiers.alt => {
            picker.query.insert(character);
            picker.selected = 0;
        }
        KeyCode::Enter => enter(state, commands),
        KeyCode::Tab => show_on_graph(state, commands),
        // Every other key is swallowed: a text field lets nothing through
        // to the graph underneath (§2.1).
        _ => {}
    }
}

/// Closes the picker, cancelling a read still in flight.
fn close(state: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(picker) = state.picker.take()
        && let Loading::Pending(id) = picker.list
    {
        commands.push(Command::CancelJob(id));
    }
}

/// `⏎`: the switch rule on the row under the cursor.
fn enter(state: &mut AppState, commands: &mut Vec<Command>) {
    let Some(row) = state
        .picker
        .as_ref()
        .and_then(PickerState::selected_row)
        .cloned()
    else {
        return;
    };
    // On the branch he is on there is nothing to switch to: `⏎` closes the
    // picker as `Tab` does, cursor on HEAD's row, nothing written or said
    // (m8 usability Q2, keymap §2.16). Before the refusal check, so a dirty
    // tree or a running write does not stop him leaving.
    if row.current {
        show_on_graph(state, commands);
        return;
    }
    let short = String::from_utf8_lossy(row.name.shorthand()).into_owned();
    // In the box, not on the status line under it: the picker stays up
    // (ticket 10, "⏎ while a write runs"), as the name prompt does.
    if let Some(words) = super::preview::preview_refusal(state) {
        say(state, words);
        return;
    }
    let Ok(full) = String::from_utf8(row.name.as_bytes().to_vec()) else {
        say(
            state,
            format!("{short} is not a UTF-8 name · switch in git"),
        );
        return;
    };
    let target = match &row.kind {
        SwitchKind::Local => SwitchTarget::Branch(full),
        SwitchKind::RemoteOnly { local, .. } => SwitchTarget::Track {
            remote: Tip {
                name: full,
                oid: row.tip,
            },
            local: local.clone(),
        },
    };
    let mut local_jobs = state.local_jobs;
    let went = super::switch::switch_into(state, commands, target, OPENED_BY, &mut || {
        local_jobs.next_id()
    });
    state.local_jobs = local_jobs;
    if !went {
        // Refused, and `switch_into` worded why on the status line (the refs
        // not read yet, a launch refused); the picker stays for another go.
        return;
    }
    if state.preview.is_some() {
        if let Some(picker) = state.picker.as_mut() {
            picker.under_preview = true;
        }
    } else {
        // Launched at once: the landing (`lands_on_head`) takes it from here.
        state.picker = None;
    }
}

fn say(state: &mut AppState, words: String) {
    if let Some(picker) = state.picker.as_mut() {
        picker.note = Some(words);
    }
}

/// `Tab`: close, and put the graph cursor on the row's tip, writing nothing.
fn show_on_graph(state: &mut AppState, commands: &mut Vec<Command>) {
    let tip = state
        .picker
        .as_ref()
        .and_then(PickerState::selected_row)
        .map(|row| row.tip);
    let Some(tip) = tip else {
        return;
    };
    close(state, commands);
    super::map::seek_to(state, commands, tip);
}

/// Where the box goes in the graph `pane` (the header row included): over
/// the rows the graph itself draws, so neither the header, a banner above
/// nor the footer's notice and hint rows below are covered, then `mergewright_ui::picker::picker_box`. The one placement both
/// the drawing (`view::frame::boxes`) and the mouse
/// ([`super::geometry::picker_bounds`]) read, so a click lands on the row
/// the eye sees.
#[must_use]
pub fn picker_box_in(state: &AppState, pane: mergewright_ui::Cells) -> mergewright_ui::Cells {
    let above = super::geometry::rows_above_graph(state);
    // The footer's rows (the chrome less the header and the gap under
    // it: the bar, and the rule from 30 rows) and the commit-graph offer's
    // line above them, when it shows.
    let below = (mergewright_ui::graph_screen::chrome_rows(state.size.rows)
        - super::geometry::header_rows(state))
        + u16::from(state.commit_graph_line().is_some());
    mergewright_ui::picker::picker_box(mergewright_ui::Cells {
        y: pane.y + above,
        height: pane.height.saturating_sub(above + below),
        ..pane
    })
}

/// [`picker_box_in`] in `geometry`'s own bounds, for the mouse
/// ([`super::geometry::picker_bounds`]).
pub(super) fn picker_bounds_in(state: &AppState, pane: Bounds) -> Bounds {
    let (x, y, width, height) = (pane.x, pane.y, pane.width, pane.height);
    let at = picker_box_in(
        state,
        mergewright_ui::Cells {
            x,
            y,
            width,
            height,
        },
    );
    let (x, y, width, height) = (at.x, at.y, at.width, at.height);
    Bounds {
        x,
        y,
        width,
        height,
    }
}

/// Two clicks on one row this close together are a double-click (blame's).
pub(super) const DOUBLE_CLICK_MS: i64 = 500;

/// The list landed, or failed. A stale id (the picker closed, or opened
/// again since) is dropped.
pub(super) fn listed(state: &mut AppState, id: JobId, result: Result<Arc<SwitchList>, String>) {
    let Some(picker) = state.picker.as_mut() else {
        return;
    };
    if picker.list != Loading::Pending(id) {
        return;
    }
    picker.list = match result {
        Ok(list) => Loading::Ready(list),
        Err(why) => Loading::Failed(why),
    };
    let wanted = picker.wanted.take();
    picker.selected = wanted
        .and_then(|wanted| picker.rows().iter().position(|row| row.name == wanted))
        .unwrap_or(0);
}

/// The preview it opened closed without writing (`Esc`, or refused): the
/// picker is shown again as it was.
pub(super) fn preview_closed(state: &mut AppState) {
    if let Some(picker) = state.picker.as_mut() {
        picker.under_preview = false;
    }
}

/// The preview it opened was confirmed: the switch is on its way and the
/// picker's job is done.
pub(super) fn preview_confirmed(state: &mut AppState) {
    if state
        .picker
        .as_ref()
        .is_some_and(|picker| picker.under_preview)
    {
        state.picker = None;
    }
}

mod mouse;
mod open_on;
#[cfg(test)]
mod tests;

pub(super) use mouse::picker_mouse;
pub(crate) use open_on::open_on;
