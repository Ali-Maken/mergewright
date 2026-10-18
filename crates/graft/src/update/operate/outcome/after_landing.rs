//! What every landing runs, whichever of the three answers it came in —
//! `Done`, `Incomplete`, `StagingNotKept` (08s m7): one list, so a hook
//! added for one path cannot be missing from another.

use graft_ops::apply::Applied;

use crate::update::events::Command;
use crate::update::operate::{JobId, Landing, Operation, refresh_scope};
use crate::update::state::{AppState, Status};

/// A write that ran, whatever its line says: the commit screen closed when
/// it was the screen's own, the scope re-read, the cursor where the launch
/// said it lands, a plain commit made [`crate::update::commit_editor::LastCommit`],
/// a put-in's commits tagged, recovered rows marked.
///
/// The words a plain commit earns (`Committed N files to <branch>.`) come
/// back rather than going on the status row: a `Done` shows them, the two
/// warning paths drop them for their own line.
pub(crate) fn after_landing(
    state: &mut AppState,
    launched: Option<&Operation>,
    applied: &Applied,
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> (Vec<Command>, Option<Status>) {
    let mut commands = landed(state, launched, applied, lands, next_id);
    let words = crate::update::commit_editor::commit_landed(state, launched, &applied.entry);
    crate::update::amend_into::landed(state, launched, &mut commands, next_id);
    crate::update::recover::tag_rows(state, launched);
    (commands, words)
}

/// The screen, the scope and the cursor ([`after_landing`]'s first half).
fn landed(
    state: &mut AppState,
    launched: Option<&Operation>,
    applied: &Applied,
    lands: Option<Landing>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // The screen is open from its launch until its write lands or fails
    // (`commit_editor::keys::editor_key` holds it while `operating`), so a
    // commit or amend landing while it is up is its own: it closes, the
    // words now a commit (m3-working-tree-undo-and-stash/05). Any other
    // write leaves an open screen, and every kept word, as they are (08s
    // M1). A failure never gets here: the message is what he has to fix.
    let own = matches!(launched, Some(Operation::Commit(_) | Operation::Amend(_)));
    if own && state.commit_editor.is_some() {
        crate::update::commit_editor::committed(state);
    }
    // Whatever landed, the commit `c` made is no longer the last thing
    // that happened (C13); a commit sets it again in `commit_landed`.
    state.last_commit = None;
    // Nor is the merge the review made (ease-pass/12 M18).
    state.last_merge = None;
    let mut commands = refresh_scope(state, applied.refresh, next_id);
    // A write that moved `HEAD` lands the cursor on its new row once
    // this refresh's refs and walk are in, exactly as a `--continue`
    // does (UX §3.1, m7-power-tools/03); a put-back on what it restored
    // (m7-power-tools/24).
    match lands {
        Some(Landing::Changes(path)) => {
            crate::update::banner::lands_on_changes(state, &mut commands, path, next_id);
        }
        Some(on) => crate::update::banner::finished_lands_on(state, &commands, on),
        None => {}
    }
    commands
}
