//! The words an [`ApplyError`](super::ApplyError) and an
//! [`AfterState`] print.

use std::path::PathBuf;

use graft_git::run::RunError;

use crate::journal::JournalError;
use crate::keys::{KeyNames, OpKey, clause, then};
use crate::restore::RestoreError;

use super::{AfterState, reason_for};

/// What closing the askpass server reported, in his words: a refused peer
/// of another uid (read from the typed payload, never by `ErrorKind`) says
/// so and that nothing was sent; anything else is a cleanup failure. One
/// wording for every remote op and the clone notice (20b R1).
#[must_use]
pub fn askpass_words(error: &std::io::Error) -> String {
    match graft_git::run::askpass::refused(error) {
        Some(refused) => {
            let mut words = format!(
                "another user's program asked for your login and was turned away · nothing was \
                 sent: {refused}"
            );
            if let Some(cleanup) = refused.cleanup() {
                words.push_str(&format!(
                    " · and the credential prompt's socket could not be cleaned up: {cleanup}"
                ));
            }
            words
        }
        None => format!("the credential prompt's socket could not be cleaned up: {error}"),
    }
}
/// A failed raw step's words (Ruling 2026-10-01 §1): the path named as
/// [`AfterState::StagingNotKept`] names its paths, the error's own words
/// never re-worded, and no `u` clause.
pub(in crate::apply) fn not_as_they_were_words(error: &RestoreError, files: usize) -> String {
    match error {
        RestoreError::WriteRaw { path, source, .. } if files <= 1 => format!(
            "your edits are back · {} not written back as it was: {source} · its exact bytes are in \
             the backup",
            path.display()
        ),
        RestoreError::WriteRaw { path, source, .. } => format!(
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

/// A lost edit's put-back whose raw step failed: the words above, then the
/// route (20b R2) `: fix that, then <u> and put it back again` (`them` when
/// more than one path, or the check, failed). `u` unbound → the words
/// above exactly (W3).
pub(in crate::apply) fn lost_edit_words(
    error: &RestoreError,
    files: usize,
    keys: &dyn KeyNames,
) -> String {
    let words = not_as_they_were_words(error, files);
    let Some(undo) = keys.label(OpKey::Undo) else {
        return words;
    };
    let one = matches!(error, RestoreError::WriteRaw { .. }) && files <= 1;
    let it = if one { "it" } else { "them" };
    format!("{words}: fix that, then {undo} and put {it} back again")
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
/// a failed raw step's words when there were any (`kept`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{Letters, NoKeys};

    fn denied(path: &str) -> RestoreError {
        RestoreError::WriteRaw {
            path: PathBuf::from(path),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
            tried: 1,
        }
    }

    /// 20b R2: the route clause, singular and plural; with `app.undo`
    /// unbound, today's words byte for byte (W3).
    #[test]
    fn the_route_clause_drops_when_undo_is_unbound() {
        let one = denied("src/a.rs");
        assert_eq!(
            lost_edit_words(&one, 1, &Letters),
            format!(
                "{}: fix that, then u and put it back again",
                not_as_they_were_words(&one, 1)
            )
        );
        assert_eq!(
            lost_edit_words(&denied("src/b.rs"), 2, &Letters),
            "your edits are back · not all written back as they were: src/b.rs: denied · their \
             exact bytes are in the backup: fix that, then u and put them back again"
        );
        assert_eq!(
            lost_edit_words(&one, 1, &NoKeys),
            "your edits are back · src/a.rs not written back as it was: denied · its exact bytes \
             are in the backup",
            "unbound: no clause"
        );
    }
}
