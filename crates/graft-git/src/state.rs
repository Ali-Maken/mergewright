//! Whether the repository is in the middle of a merge, rebase, cherry-pick,
//! revert or bisect — what a commit/amend refuses against until it is over
//! (ADR-0034: the refusal itself is M5's; this is what it reads).
//!
//! A plain filesystem check under [`RepoHandle::git_dir`], no `git` spawn:
//! git itself marks these states by writing a handful of well-known files
//! and directories there, and reads them the same way.

use std::io;
use std::path::Path;

use crate::repo::RepoHandle;

/// An operation partway through, blocking a plain commit until it is
/// resolved, continued or aborted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InProgress {
    /// `MERGE_HEAD` is present: a merge with conflicts to resolve.
    Merge,
    /// `rebase-merge/` or `rebase-apply/` is present.
    Rebase,
    /// `CHERRY_PICK_HEAD` is present.
    CherryPick,
    /// `REVERT_HEAD` is present.
    Revert,
    /// `BISECT_LOG` is present.
    Bisect,
}

impl InProgress {
    /// A short, human name for the state.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            InProgress::Merge => "merge",
            InProgress::Rebase => "rebase",
            InProgress::CherryPick => "cherry-pick",
            InProgress::Revert => "revert",
            InProgress::Bisect => "bisect",
        }
    }

    /// The `git` subcommand that carries the operation forward.
    #[must_use]
    pub fn continue_command(self) -> &'static str {
        match self {
            InProgress::Merge => "git commit",
            InProgress::Rebase => "git rebase --continue",
            InProgress::CherryPick => "git cherry-pick --continue",
            InProgress::Revert => "git revert --continue",
            InProgress::Bisect => "git bisect good/bad",
        }
    }

    /// The `git` subcommand that abandons the operation.
    #[must_use]
    pub fn abort_command(self) -> &'static str {
        match self {
            InProgress::Merge => "git merge --abort",
            InProgress::Rebase => "git rebase --abort",
            InProgress::CherryPick => "git cherry-pick --abort",
            InProgress::Revert => "git revert --abort",
            InProgress::Bisect => "git bisect reset",
        }
    }
}

/// Whether `repo` is in the middle of a merge, rebase, cherry-pick, revert or
/// bisect right now.
///
/// Checked in this order — the same one git itself resolves them in when more
/// than one marker happens to be present (a rebase started while a bisect was
/// already running, say): merge, rebase, cherry-pick, revert, bisect.
///
/// # Errors
/// [`io::Error`] if a marker path could not be checked (permissions, an
/// unreadable git directory) — never for the ordinary case of it simply not
/// existing.
pub fn in_progress(repo: &RepoHandle) -> Result<Option<InProgress>, io::Error> {
    let git_dir = repo.git_dir();
    for (marker, kind) in [
        ("MERGE_HEAD", InProgress::Merge),
        ("rebase-merge", InProgress::Rebase),
        ("rebase-apply", InProgress::Rebase),
        ("CHERRY_PICK_HEAD", InProgress::CherryPick),
        ("REVERT_HEAD", InProgress::Revert),
        ("BISECT_LOG", InProgress::Bisect),
    ] {
        if exists(&git_dir.join(marker))? {
            return Ok(Some(kind));
        }
    }
    Ok(None)
}

/// `Path::try_exists`, but a broken symlink (neither present nor absent, by
/// that call's own definition) counts as present: git leaves `MERGE_HEAD` and
/// the rest as plain files, never symlinks, so seeing one at all is already
/// the answer.
fn exists(path: &Path) -> Result<bool, io::Error> {
    match path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
