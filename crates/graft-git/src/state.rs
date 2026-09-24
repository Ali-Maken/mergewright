//! Whether the repository is in the middle of a merge, rebase, cherry-pick,
//! revert or bisect — what a commit/amend refuses against until it is over
//! (ADR-0034: the refusal itself is M5's; this is what it reads).
//!
//! A plain filesystem check under [`RepoHandle::git_dir`], no `git` spawn:
//! git itself marks these states by writing a handful of well-known files
//! and directories there, and reads them the same way.

use std::io;
use std::path::Path;

use graft_core::oid::Oid;

use crate::history_reads::{CommitSummary, rev_list};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

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

/// The step a rebase is on, git's own tip and the branch it is rebasing,
/// read straight from the files git itself writes there (ADR-0034: M5's
/// operations read this to build the operation banner and its refusals).
///
/// `step`/`head_name`/`onto` are only ever set for [`InProgress::Rebase`]: a
/// merge, cherry-pick, revert or bisect has no analogous "step n of m" or
/// "onto" concept in a single well-known file, so those fields stay `None`.
/// `theirs` is set for every operation but bisect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InProgressDetail {
    /// Which operation is in progress.
    pub kind: InProgress,
    /// `(current, total)`: `rebase-merge/msgnum`+`rebase-merge/end`, or
    /// `rebase-apply/next`+`rebase-apply/last`, whichever backend is in
    /// use. `None` if either file is missing or does not parse as a number
    /// — never an error for the whole call.
    pub step: Option<(u32, u32)>,
    /// `rebase-merge/head-name` or `rebase-apply/head-name`: the full ref
    /// the rebase started on (`refs/heads/x`), as git wrote it. `None` on
    /// the same terms as `step`.
    pub head_name: Option<String>,
    /// `rebase-merge/onto` or `rebase-apply/onto`: the commit being rebased
    /// onto. `None` on the same terms as `step`.
    pub onto: Option<Oid>,
    /// The incoming commit — the resolver's theirs side: `MERGE_HEAD`'s
    /// first line (merge, pull), `CHERRY_PICK_HEAD`, `REVERT_HEAD`, or
    /// `REBASE_HEAD` (either rebase backend), with its subject. `None` for
    /// bisect, or when git has not written that file (a rebase stopped on
    /// an `edit`, say).
    ///
    /// A `Result` of its own, not a failure of the whole read: this is a
    /// label, and an unreadable head file must not hide the operation — the
    /// banner that offers `--abort` is drawn from it. The error is carried
    /// here for the label to show, never dropped.
    pub theirs: Result<Option<CommitSummary>, IncomingError>,
}

/// Why the incoming commit — [`InProgressDetail::theirs`] — could not be
/// named: its head file does not hold a commit id, or git could not list the
/// commit it names (or was stopped while it did).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("could not read {file}: {reason}")]
pub struct IncomingError {
    /// The head file read: `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`
    /// or `REBASE_HEAD`.
    pub file: &'static str,
    /// What went wrong, as text: kept `Clone`/`Eq` so the detail stays so.
    pub reason: String,
}

/// Why [`in_progress_detail`] could not even check whether an operation is
/// in progress. Reading the rebase state *files* — `step`, `head_name`,
/// `onto` — never produces this: a missing or unparsable one is just `None`
/// on the field, per [`InProgressDetail`]'s own doc.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// [`in_progress`] itself failed: a marker path could not be checked.
    #[error("could not check the repository's in-progress state: {0}")]
    Io(#[from] io::Error),
}

/// [`in_progress`], plus — for a rebase — the step it is on, the branch it
/// started from and what it is rebasing onto.
///
/// # Errors
/// [`StateError`] on the same terms as [`in_progress`]: a marker path could
/// not be checked. Never for a rebase detail file that is missing or does
/// not parse; those fields are simply `None`. Nor for an incoming commit git
/// cannot read: that is [`InProgressDetail::theirs`]'s own `Err`.
///
/// Reads theirs with no way to stop git; a caller holding a [`Stop`] uses
/// [`in_progress_detail_with_stop`].
pub fn in_progress_detail(repo: &RepoHandle) -> Result<Option<InProgressDetail>, StateError> {
    in_progress_detail_with_stop(repo, &())
}

/// [`in_progress_detail`], with `until` able to end the one git run it makes
/// (listing the incoming commit). A stopped run is carried on
/// [`InProgressDetail::theirs`] like any other failure to name it.
///
/// # Errors
/// As [`in_progress_detail`].
pub fn in_progress_detail_with_stop(
    repo: &RepoHandle,
    until: &dyn Stop,
) -> Result<Option<InProgressDetail>, StateError> {
    let Some(kind) = in_progress(repo)? else {
        return Ok(None);
    };
    let (step, head_name, onto) = if kind == InProgress::Rebase {
        let git_dir = repo.git_dir();
        let merge_dir = git_dir.join("rebase-merge");
        let (dir, current_file, total_file) = if exists(&merge_dir)? {
            (merge_dir, "msgnum", "end")
        } else {
            (git_dir.join("rebase-apply"), "next", "last")
        };
        (
            read_step(&dir, current_file, total_file),
            read_field(&dir.join("head-name")),
            read_field(&dir.join("onto")).and_then(|hex| Oid::from_hex(&hex).ok()),
        )
    } else {
        (None, None, None)
    };
    let theirs = incoming(repo, kind, until);
    Ok(Some(InProgressDetail {
        kind,
        step,
        head_name,
        onto,
        theirs,
    }))
}

/// The commit `kind` is bringing in, read from the head file git writes for
/// it. A missing file is `None`; a file naming a commit git cannot read is
/// an [`IncomingError`], for the caller to keep beside the rest of the
/// detail.
fn incoming(
    repo: &RepoHandle,
    kind: InProgress,
    stop: &dyn Stop,
) -> Result<Option<CommitSummary>, IncomingError> {
    let file = match kind {
        InProgress::Merge => "MERGE_HEAD",
        InProgress::CherryPick => "CHERRY_PICK_HEAD",
        InProgress::Revert => "REVERT_HEAD",
        InProgress::Rebase => "REBASE_HEAD",
        InProgress::Bisect => return Ok(None),
    };
    let failed = |reason: String| IncomingError { file, reason };
    // `MERGE_HEAD` lists one id per merged head; the first is the one merged.
    let Some(hex) = read_field(&repo.git_dir().join(file))
        .and_then(|text| text.lines().next().map(str::to_owned))
    else {
        return Ok(None);
    };
    let id = Oid::from_hex(hex.trim())
        .map_err(|_| failed(format!("it does not hold a commit id: {hex}")))?;
    let range = rev_list(repo, 1, vec![GitArg::Oid(id)], stop)
        .map_err(|error| failed(error.to_string()))?;
    let summary = range
        .commits
        .into_iter()
        .last()
        .ok_or_else(|| failed(format!("it names {id}, which git did not list")))?;
    Ok(Some(summary))
}

/// Reads a one-line git state file, trimmed. `None` if it is missing, empty
/// or unreadable — every case this crate treats as "not there" rather than
/// as a failure of the whole call.
fn read_field(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// `(current, total)` from a pair of git state files (`msgnum`/`end`, or
/// `next`/`last`). `None` if either is missing or is not a plain number.
fn read_step(dir: &Path, current_file: &str, total_file: &str) -> Option<(u32, u32)> {
    let current = read_field(&dir.join(current_file))?.parse().ok()?;
    let total = read_field(&dir.join(total_file))?.parse().ok()?;
    Some((current, total))
}

/// The configured merge tool (`git config --get merge.tool`), if any.
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped. An unset
/// `merge.tool` is git's own exit 1, read as `Ok(None)`, not an error.
pub fn mergetool(repo: &RepoHandle) -> Result<Option<String>, RunError> {
    match GitInvocation::new(repo)
        .args([
            GitArg::Flag("config"),
            GitArg::Flag("--get"),
            GitArg::Word("merge.tool".to_owned()),
        ])
        .run(&())
    {
        Ok(output) => Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )),
        Err(RunError::Exited { status: 1, .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests;
