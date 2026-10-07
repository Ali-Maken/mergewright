//! The words an [`ApplyError`](super::ApplyError) and an
//! [`AfterState`] print.

use std::path::PathBuf;

use graft_git::run::RunError;

use crate::journal::JournalError;
use crate::keys::{KeyNames, OpKey, clause, then};
use crate::restore::RestoreError;

use super::{AfterState, reason_for};
/// A failed raw step's words (Ruling 2026-10-01 §1): the path named as
/// [`AfterState::StagingNotKept`] names its paths, the error's own words
/// never re-worded, and no `u` clause.
pub(in crate::apply) fn not_as_they_were_words(error: &RestoreError, files: usize) -> String {
    match error {
        RestoreError::WriteRaw { path, source } if files <= 1 => format!(
            "your edits are back · {} not written back as it was: {source} · its exact bytes are in \
             the backup",
            path.display()
        ),
        RestoreError::WriteRaw { path, source } => format!(
            "your edits are back · not all written back as they were: {}: {source} · their exact \
             bytes are in the backup",
            path.display()
        ),
        other => format!(
            "your edits are back · not written back as they were: {other} · their exact bytes are \
             in the backup"
        ),
    }
}

/// `your edits are back · staging not kept: <first>[ +N more] · u undoes it,
/// staging included`.
pub(in crate::apply) fn staging_not_kept_words(paths: &[PathBuf], keys: &dyn KeyNames) -> String {
    then(
        format!("your edits are back{}", staging_clause(paths)),
        [clause(keys, OpKey::Undo, "undoes it, staging included")],
    )
}

/// ` · staging not kept: <first>[ +N more]`, or nothing when the index came
/// back. The one formatter for the clause: [`AfterState::StagingNotKept`],
/// a kept set-aside's reason and words, and git's failure line when the
/// fallback ran (review m7-30-06d Minor 1).
pub(in crate::apply) fn staging_clause(unstaged: &[PathBuf]) -> String {
    if unstaged.is_empty() {
        String::new()
    } else {
        format!(" · staging not kept: {}", staging_named(unstaged))
    }
}

/// `<first>[ +N more]`: the paths whose staging did not come back.
pub(super) fn staging_named(paths: &[PathBuf]) -> String {
    let first = paths
        .first()
        .map_or_else(String::new, |path| path.display().to_string());
    match paths.len() {
        0 | 1 => first,
        n => format!("{first} +{} more", n - 1),
    }
}

/// The journal failing, with git's own failure when there was one, and
/// where a set-aside's edits are when they are not back in the tree.
pub(super) fn journal_failed(
    error: &JournalError,
    after: Option<&RunError>,
    kept: Option<&str>,
) -> String {
    let mut line = match after {
        Some(ran) => format!("{error} (git had already failed: {})", reason_for(ran)),
        None => error.to_string(),
    };
    if let Some(kept) = kept {
        line.push_str(" · ");
        line.push_str(kept);
    }
    line
}

/// Every after-state failure on one line.
pub(in crate::apply) fn joined(unread: &[AfterState], keys: &dyn KeyNames) -> String {
    unread
        .iter()
        .map(|state| state.explain(keys))
        .collect::<Vec<_>>()
        .join("; ")
}
