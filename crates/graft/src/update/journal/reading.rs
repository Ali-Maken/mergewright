//! What a J row means, read once for both the keys (`⏎`) and the view (the
//! row's words), so the two cannot disagree.

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;

use crate::update::switch::SwitchTarget;

/// The branch a shell switch left, as a switch target: `checkout: moving
/// from main to spike` → `main`; a detached start is its commit.
pub(crate) fn switched_from(subject: &str) -> Option<SwitchTarget> {
    let (from, _) = subject
        .strip_prefix("checkout: moving from ")?
        .split_once(" to ")?;
    let hex = (from.len() == 40 || from.len() == 64) && from.bytes().all(|b| b.is_ascii_hexdigit());
    Some(if hex {
        SwitchTarget::Detached(Oid::from_hex(from).ok()?)
    } else {
        SwitchTarget::Branch(format!("refs/heads/{from}"))
    })
}

/// The word J draws for a step Mergewright doesn't undo, or `None` for one it
/// does. The predicate is `graft_ops::journal::undoable`, the one
/// `plan_revert_to` skips by, so J's `⏎` and its bar cannot disagree with
/// what the time machine would do.
pub(crate) fn not_undone_kind(entry: &JournalEntry) -> Option<&'static str> {
    if graft_ops::journal::undoable(entry) {
        return None;
    }
    Some(match entry.kind {
        OperationKind::Fetch => "fetch",
        OperationKind::Push => "push",
        // `undoable` is false for a pull only when it changed nothing.
        OperationKind::Pull => "pull that changed nothing",
        // `undoable` answers false for no other kind; "step" is honest if
        // graft-ops ever adds one.
        _ => "step",
    })
}
