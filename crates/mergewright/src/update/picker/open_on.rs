//! `C` on a teammate's row opens the picker on that branch (ease-pass 09b,
//! Q1); the cursor is put on it when the list lands (`super::listed`).

use mergewright_core::ref_name::RefName;
use mergewright_ops::branch_status::Candidate;

use super::open_picker;
use crate::update::events::Command;
use crate::update::state::AppState;

/// Opens the picker on `branch` (ease-pass 09b, Q1): `C` on a teammate's
/// row. The list is still being read, so the name is held in
/// [`PickerState::wanted`] and [`listed`] places the cursor.
pub(crate) fn open_on(state: &mut AppState, branch: &Candidate) -> Vec<Command> {
    let commands = open_picker(state);
    if let Some(picker) = state.picker.as_mut() {
        // Safe to drop a failure: candidate names come from refs git
        // listed, which always parse; without one the picker opens as `C`
        // does today, on the first row.
        picker.wanted = RefName::new(branch.name.as_bytes()).ok();
    }
    commands
}

/// The branch `C` opens on: the cursor row's branch (the lanes' focus)
/// when it is not `HEAD`'s. `None` on HEAD's own row and on a row with no
/// live branch: the picker opens as before.
pub(super) fn opens_on(state: &AppState) -> Option<Candidate> {
    let focused = crate::update::branch_pane::focused(state)?;
    let refs = state.refs.as_deref()?;
    let own = refs.branches().any(|(_, short, current, _)| {
        current && focused.branch.name.as_bytes().strip_prefix(b"refs/heads/") == Some(short)
    });
    (!own).then_some(focused.branch)
}
