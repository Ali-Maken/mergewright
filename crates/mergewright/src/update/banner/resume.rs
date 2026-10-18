//! Resuming a stopped operation — mergetool, edit, and the line a resume leaves.

use super::{AppState, Command, JobId, Path, PathBuf, Resume, Status, live};
use mergewright_ui::keymap::{ActionId, ContextId, KeyChord, Keymap};

mod answers;

pub use answers::resumed;
// Only the tests reach these through `banner`.
#[cfg(test)]
pub(super) use answers::{kept_open, resumed_done, resumed_words};

/// Queue a `--continue`/`--skip`/`--abort`.
///
/// Unstoppable, like every other history write: `Esc` killing git partway
/// through a `--continue` would leave git's state on disk with the entry
/// still `Pending` and no outcome line, which is the shape `mergewright_ops` does
/// not model ([`crate::update::preview::HISTORY_CANNOT_STOP`]).
pub fn resume(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    what: Resume,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if crate::update::operate::operation_in_flight(state) {
        return false;
    }
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return false;
    };
    let verb = match what {
        Resume::Continue { .. } | Resume::ContinueWith { .. } => "continue",
        Resume::Skip => "skip",
        Resume::Abort => "abort",
    };
    let description = format!("{verb} {}", kind.name());
    let id = next_id();
    let claimed = crate::update::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        crate::update::operate::say_if_refused(state, claimed);
        return true;
    }
    state.resuming = Some((id, what.clone()));
    state.status = Status::Busy(description.clone());
    commands.extend(crate::update::remote::launched(
        state,
        id,
        description,
        None,
        crate::update::remote::Stoppable::No(crate::update::preview::HISTORY_CANNOT_STOP),
        None,
    ));
    commands.push(Command::Resume(
        id,
        what,
        std::sync::Arc::clone(&state.keymap),
    ));
    true
}

/// `M`: hand the terminal to the configured mergetool for `path`.
///
/// Refused in words when there is no `merge.tool`: `git mergetool` with none
/// configured asks "which tool?" on stdin, and stdin at that moment belongs
/// to a terminal Mergewright has just given away, so the question would have no
/// way back to the user.
pub fn open_mergetool(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    let Some(tool) = state.mergetool.as_deref() else {
        let place = crate::update::edge::context_of(state);
        state.status = Status::Sticky {
            reason: "no merge.tool configured".to_owned(),
            keys: mergetool_fallback(place, path, &state.keymap),
        };
        return true;
    };
    state.status = Status::Busy(format!("{tool} on {}", path.display()));
    commands.push(Command::OpenMergetool {
        path: path.to_path_buf(),
    });
    true
}

/// The rows a press of `e` may answer with "open this file in `$EDITOR`".
pub(super) const EDIT_ROWS: [&str; 3] = ["operation.edit", "resolver.edit", "changes.edit"];

/// The rows a press of `M` may answer with "hand this file to the mergetool".
pub(super) const MERGETOOL_ROWS: [&str; 3] = [
    "operation.mergetool",
    "resolver.mergetool",
    "changes.mergetool",
];

/// The row the shipped `key` acts as at `place`, when it is one of `rows`
/// (m8/10j ruling 1). Read off the shipped table, not his: the words name
/// the *action* that answers there, then spell it from his keymap — so a
/// remap moves the label, never the action the clause promises.
pub(super) fn row_at(place: ContextId, key: char, rows: &[&str]) -> Option<ActionId> {
    Keymap::shipped()
        .acting(place, KeyChord::key(key))
        .filter(|id| rows.contains(&id.0))
}

/// The one place → edit-row lookup both stop lines share, so the two can
/// never name different `e` keys in one place (m8/10j ruling 4).
pub(super) fn edit_row(place: ContextId) -> Option<ActionId> {
    row_at(place, 'e', &EDIT_ROWS)
}

/// Two clauses two spaces apart, either dropped; `None` when both are.
pub(super) fn joined(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(a), Some(b)) => Some(format!("{a}  {b}")),
        (one, other) => one.or(other),
    }
}

/// The fallback clause after `no merge.tool configured · `: `e edits {path}
/// a abort`, each key from the row answering it at `place` (m8/10j ruling
/// 1); a clause with no row there or an unbound key dropped; nothing left →
/// `None`. `a` is kept only where it is `operation.abort` (the banner): the
/// resolver is modal and the changes pane refuses `a` while an operation
/// is live.
pub(crate) fn mergetool_fallback(place: ContextId, path: &Path, keys: &Keymap) -> Option<String> {
    let edit =
        edit_row(place).and_then(|row| keys.key_verb(&[row], &format!("edits {}", path.display())));
    let abort =
        row_at(place, 'a', &["operation.abort"]).and_then(|row| keys.key_verb(&[row], "abort"));
    joined(edit, abort)
}

/// `e edit  M mergetool` as said at `place` (m8/10j ruling 4): `e` from
/// the edit row answering it there, `M` only where it is a mergetool row
/// (on the graph `M` is `graph.map`); a clause with no row or an unbound
/// key dropped; nothing left → `None`.
pub(crate) fn edit_or_mergetool(place: ContextId, keys: &Keymap) -> Option<String> {
    let edit = edit_row(place).and_then(|row| keys.key_verb(&[row], "edit"));
    let tool =
        row_at(place, 'M', &MERGETOOL_ROWS).and_then(|row| keys.key_verb(&[row], "mergetool"));
    joined(edit, tool)
}

/// `e`: open `$EDITOR` on `path` at its first conflict marker.
///
/// The marker's line number is **not** looked up here. `update` is pure —
/// it reads no clock and opens no file (ARCHITECTURE.md) — and finding the
/// first `<<<<<<< ` means reading the file. So the command carries the path
/// and the loop, which is about to open the file anyway, does the read on
/// its way past ([`Command::OpenConflictEditor`]).
pub fn edit_conflict(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    state.conflict_editing = Some(path.to_path_buf());
    commands.push(Command::OpenConflictEditor {
        path: path.to_path_buf(),
    });
    true
}

/// The path under the changes-pane cursor, when that file is unmerged.
///
/// The file list is git's diff output and the status entries are git's
/// porcelain output; only the path joins them, which is why this matches on
/// bytes rather than on an index into either.
#[must_use]
pub fn cursor_conflict(state: &AppState) -> Option<PathBuf> {
    use std::ffi::OsStr;

    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = usize::try_from(entry)
        .ok()
        .and_then(|index| list.files.get(index))?;
    state.working_tree.as_deref().and_then(|status| {
        status
            .entries
            .iter()
            .find(|candidate| {
                candidate.unmerged
                    && OsStr::as_encoded_bytes(candidate.path.as_os_str()) == file.path.as_slice()
            })
            .map(|candidate| candidate.path.clone())
    })
}
