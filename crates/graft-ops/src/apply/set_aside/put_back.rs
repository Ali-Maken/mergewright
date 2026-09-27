//! Step 6's apply: `S` back **with the index**, and the fallback when git
//! cannot reinstate it (ADR-0045 §4 step 6, amended 2026-09-26 b).
//!
//! `git stash apply --index <S>` first, then the status, read whatever the
//! exit code — never git's message:
//! - nothing unmerged and git succeeded → [`PutBackRan::Whole`];
//! - something unmerged → [`PutBackRan::Clashed`] (git resets the index
//!   before its merge, so it is the clash a plain apply gives);
//! - git failed, nothing unmerged, and **no tracked file differs from
//!   `HEAD`** → git wrote nothing: `git stash apply <S>` once, then the same
//!   read. Clean is [`PutBackRan::Unstaged`] naming the staging lost;
//! - anything else → [`PutBackRan::Failed`]. A second apply never runs over
//!   a tree the first one changed: the edits would go in twice.
//!
//! Absorb's rebuilt stash ([`from_index_base`]) is one input to the same
//! apply.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError};
use graft_git::status::WorkingTreeStatus;

use crate::facts::FactsError;

use super::Context;

/// `apply.whitespace=nowarn` for the put-back, through `GIT_CONFIG_COUNT`
/// (git ≥ 2.31): `stash apply --index` re-applies the index with a child
/// `git apply --cached`, which reads his `apply.whitespace`. His edits come
/// back as he left them — `fix` would strip a staged line's trailing
/// spaces, `error` would fail the put-back (review 06c M3).
const AS_THEY_WERE: [(&str, &str); 3] = [
    ("GIT_CONFIG_COUNT", "1"),
    ("GIT_CONFIG_KEY_0", "apply.whitespace"),
    ("GIT_CONFIG_VALUE_0", "nowarn"),
];

/// How a put-back ran: read from the status, never from git's exit code or
/// message.
#[derive(Debug)]
pub(crate) enum PutBackRan {
    /// `--index` applied clean: the staged split is as it was.
    Whole,
    /// `--index` refused and wrote nothing; the plain apply ran clean.
    /// `paths`: the staged paths whose staging did not come back.
    Unstaged { paths: Vec<PathBuf> },
    /// Unmerged after either apply (ADR-0045 §2).
    Clashed { paths: Vec<PathBuf> },
    /// `S` stays listed: git refused and either wrote something or refused
    /// the plain apply too — or the staging lost could not be read before
    /// the plain apply, which then never ran.
    Failed { error: RunError },
}

/// Step 6's apply of `stash` (`S`, or absorb's rebuilt copy), with the
/// fallback. `Err`: the status could not be read after an apply, so whether
/// it clashed is unknown.
pub(super) fn run_put_back(
    ctx: &Context<'_>,
    stash: Oid,
    commands: &mut Vec<String>,
) -> Result<PutBackRan, FactsError> {
    let first = apply(ctx, stash, true, commands);
    match read(ctx, first)? {
        Read::Clean => return Ok(PutBackRan::Whole),
        Read::Clashed(paths) => return Ok(PutBackRan::Clashed { paths }),
        Read::Wrote(error) => return Ok(PutBackRan::Failed { error }),
        // git could not reinstate the index on this tree and wrote nothing.
        // Its refusal is handled, not dropped: the plain apply below is the
        // answer to it, and `Unstaged` says what it cost.
        Read::WroteNothing(_refused) => {}
    }
    // The staging a plain apply loses is read first: without it the words
    // could not name it, so when that read fails nothing more runs and `S`
    // stays listed, with the read's failure as the error.
    let paths = match staging_lost(ctx, stash) {
        Ok(paths) => paths,
        Err(error) => return Ok(PutBackRan::Failed { error }),
    };
    let second = apply(ctx, stash, false, commands);
    Ok(match read(ctx, second)? {
        Read::Clean => PutBackRan::Unstaged { paths },
        Read::Clashed(paths) => PutBackRan::Clashed { paths },
        // Never a third apply.
        Read::Wrote(error) | Read::WroteNothing(error) => PutBackRan::Failed { error },
    })
}

/// `git stash apply [--index] <stash>`, its line on the Command Log.
fn apply(
    ctx: &Context<'_>,
    stash: Oid,
    index: bool,
    commands: &mut Vec<String>,
) -> Result<(), RunError> {
    let mut args = vec![GitArg::Flag("stash"), GitArg::Flag("apply")];
    if index {
        args.push(GitArg::Flag("--index"));
    }
    args.push(GitArg::Oid(stash));
    let mut git = GitInvocation::new(ctx.repo).args(args);
    for (key, value) in AS_THEY_WERE {
        git = git.env(key, value);
    }
    commands.push(git.command_line());
    git.run(&()).map(|_output| ())
}

/// What one apply left, from the status.
#[derive(Debug)]
enum Read {
    Clean,
    Clashed(Vec<PathBuf>),
    /// git failed and a tracked file differs from `HEAD`.
    Wrote(RunError),
    /// git failed and every tracked file is `HEAD`'s.
    WroteNothing(RunError),
}

/// The status after an apply, judged by [`judge`]. Never stopped: the apply
/// has run, and the second half of a write already done is not cut short.
fn read(ctx: &Context<'_>, applied: Result<(), RunError>) -> Result<Read, FactsError> {
    let status = graft_git::status::read(ctx.repo, &())?;
    Ok(judge(applied, &status))
}

/// The status decides, not the exit code: `stash apply` exits 1 on a clash
/// and on a refusal alike (ADR-0045 §7).
///
/// "Wrote nothing" is every tracked file equal to `HEAD`: no entry staged,
/// unstaged or unmerged. Untracked files do not count (`S` holds none).
/// The put-back starts on the tree the operation left, which is `HEAD`'s
/// after the reset, so a tree still equal to `HEAD` holds none of the
/// edits, and a second apply cannot put any in twice.
fn judge(applied: Result<(), RunError>, status: &WorkingTreeStatus) -> Read {
    let unmerged: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect();
    if !unmerged.is_empty() {
        return Read::Clashed(unmerged);
    }
    let Err(error) = applied else {
        return Read::Clean;
    };
    let tracked_changed = status
        .entries
        .iter()
        .any(|entry| entry.is_staged() || entry.is_unstaged());
    if tracked_changed {
        Read::Wrote(error)
    } else {
        Read::WroteNothing(error)
    }
}

/// The staged paths a plain apply of `stash` loses: `diff-tree -r` from its
/// base (`^1`) to its index (`^2`), less the paths added — a plain apply
/// brings a staged new file back staged (m6-conflicts/83). No rename
/// detection, so a staged rename names its old path (its deletion comes
/// back unstaged), and there are no copies.
fn staging_lost(ctx: &Context<'_>, stash: Oid) -> Result<Vec<PathBuf>, RunError> {
    let output = GitInvocation::new(ctx.repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("--name-status"),
            GitArg::Flag("-z"),
            GitArg::Word(format!("{stash}^1")),
            GitArg::Word(format!("{stash}^2")),
        ])
        .run(&())?;
    lost_from_name_status(&output.stdout)
}

/// `--name-status -z` output (`<X>\0<path>\0` pairs) → every path whose
/// status is not `A` (added) or `C` (copied).
fn lost_from_name_status(printed: &[u8]) -> Result<Vec<PathBuf>, RunError> {
    let mut fields = printed.split(|byte| *byte == 0).filter(|f| !f.is_empty());
    let mut lost = Vec::new();
    while let Some(status) = fields.next() {
        let path = fields.next().ok_or_else(|| {
            RunError::Unparsable(format!(
                "diff-tree printed a status with no path: {:?}",
                String::from_utf8_lossy(status)
            ))
        })?;
        if !matches!(status.first(), Some(b'A' | b'C')) {
            lost.push(PathBuf::from(String::from_utf8_lossy(path).into_owned()));
        }
    }
    Ok(lost)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};

    use super::*;

    fn refused() -> RunError {
        RunError::Unparsable("git refused".to_owned())
    }

    fn entry(path: &str) -> StatusEntry {
        StatusEntry {
            path: PathBuf::from(path),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: false,
        }
    }

    fn status(entries: Vec<StatusEntry>) -> WorkingTreeStatus {
        WorkingTreeStatus { entries }
    }

    /// T8: git failed and a tracked file changed → `Wrote`, which
    /// `run_put_back` never follows with a second apply.
    #[test]
    fn a_failed_apply_that_changed_a_tracked_file_wrote() {
        let changed = status(vec![StatusEntry {
            unstaged: Some(Change::Modified),
            ..entry("f.txt")
        }]);
        assert!(matches!(judge(Err(refused()), &changed), Read::Wrote(_)));
        let staged = status(vec![StatusEntry {
            staged: Some(Change::Added),
            ..entry("n.txt")
        }]);
        assert!(matches!(judge(Err(refused()), &staged), Read::Wrote(_)));
    }

    #[test]
    fn a_failed_apply_with_only_untracked_files_wrote_nothing() {
        let untracked = status(vec![StatusEntry {
            untracked: true,
            ..entry("junk")
        }]);
        assert!(matches!(
            judge(Err(refused()), &untracked),
            Read::WroteNothing(_)
        ));
        assert!(matches!(
            judge(Err(refused()), &status(vec![])),
            Read::WroteNothing(_)
        ));
    }

    #[test]
    fn unmerged_is_a_clash_whatever_the_exit_code() {
        let clash = status(vec![StatusEntry {
            unmerged: true,
            ..entry("f.txt")
        }]);
        assert!(matches!(
            judge(Ok(()), &clash),
            Read::Clashed(paths) if paths == [PathBuf::from("f.txt")]
        ));
        assert!(matches!(judge(Err(refused()), &clash), Read::Clashed(_)));
    }

    #[test]
    fn staging_lost_leaves_out_added_and_copied() {
        let printed = b"M\0a.txt\0A\0new.txt\0D\0gone.txt\0T\0link\0C75\0copy\0";
        assert_eq!(
            lost_from_name_status(printed).unwrap(),
            ["a.txt", "gone.txt", "link"].map(PathBuf::from)
        );
        assert!(lost_from_name_status(b"M\0").is_err());
    }
}
