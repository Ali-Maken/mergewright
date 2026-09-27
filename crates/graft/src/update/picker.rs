//! The branch picker (`C`, m7-power-tools/10; keymap §2.1, §2.3, §2.16):
//! type part of a branch's name, `⏎` switches to it through the one switch
//! rule (`update::switch`), `Tab` shows it on the graph.
//!
//! A text field: while it is up it claims every key but `Ctrl+C`, so `j`,
//! `k`, `q`, `u` and `C` are typed ([`AppState::text_field_open`] names it).
//! The rows are `graft_ops::switch_list`'s, read by a job when it opens and
//! filtered per key by `graft_ops::switch_list::filter_switch_rows`.
//!
//! A preview opened from it hides it rather than closing it: `Esc` on the
//! preview shows it again with the query and the row kept, as the journal
//! comes back under its own revert preview; `y` closes it for good. The
//! hook is [`preview_closed`] / [`preview_confirmed`], called by
//! `update::preview` where it restores the journal.

use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::operation::Tip;
use graft_ops::switch_list::{SwitchKind, SwitchList, SwitchRow, filter_switch_rows};
use graft_ui::text_input::TextField;

use super::detail::Loading;
use super::events::Command;
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
        commands.extend(open_picker(state));
        return true;
    }
    false
}

/// `C` opens it on the graph screen with the graph focused and nothing
/// standing over the graph (§2.3). The commit editor's `C` is the editor's:
/// the editor is an overlay, so this never sees it.
fn opens(state: &AppState, key: KeyPress) -> bool {
    key.code == KeyCode::Char('C')
        && !key.modifiers.control
        && !key.modifiers.alt
        && state.screen == Screen::Graph
        && state.graph_has_focus()
        && state.picker.is_none()
        && state.graph_overlay().is_none()
        && !state.modal_owns_keys()
}

fn shown_key(state: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) {
    let Some(picker) = state.picker.as_mut() else {
        return;
    };
    picker.note = None;
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
    let short = String::from_utf8_lossy(row.name.shorthand()).into_owned();
    if row.current {
        say(state, format!("you're on {short}"));
        return;
    }
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
    picker.selected = 0;
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

#[cfg(test)]
mod tests;
