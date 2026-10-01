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
//! After either clean apply, the **raw step** (m7-power-tools/30s,
//! ADR-0052): `stash apply` writes his edits in git's encoding (CRLF for an
//! LF file under `core.autocrlf=true`; a one-way clean filter's output), so
//! his own bytes, kept in the before-snapshot's `raw/` tree, are written
//! back over every path whose content is still his
//! ([`same_content_paths`]). Never on a clash or a failure. Its failure is
//! carried on the clean variant ([`NotAsTheyWere`]), never a reason to keep
//! `S`: the edits are on disk either way.
//!
//! Absorb's rebuilt stash
//! ([`from_index_base`](super::rebuild::from_index_base)) is one input to
//! the same apply.

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::status::WorkingTreeStatus;

use crate::facts::FactsError;
use crate::restore::{RestoreError, same_content_paths, write_raw};

use super::Context;

/// Where his own bytes are for a put-back (ADR-0052): the entry's
/// `worktree_raw_before` and `worktree_before`, the tree they clean into.
/// `None` when either field is: no raw step, no spawn.
pub(super) struct HisBytes {
    /// The before-snapshot's `raw/` tree.
    pub(super) raw: Oid,
    /// The before-snapshot's working tree, as git cleans it.
    pub(super) tree: Oid,
}

/// The raw step failed after a clean apply: his edits are back in git's
/// encoding, not as he left them. [`crate::apply::AfterState::NotAsTheyWere`]'s
/// fields.
#[derive(Debug)]
pub(crate) struct NotAsTheyWere {
    /// Which check or write failed.
    pub(crate) error: RestoreError,
    /// How many paths it was to write; 0 when the check failed.
    pub(crate) files: usize,
}

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
    /// `--index` applied clean: the staged split is as it was. `raw`: the
    /// raw step's failure, if it failed.
    Whole { raw: Option<NotAsTheyWere> },
    /// `--index` refused and wrote nothing; the plain apply ran clean.
    /// `paths`: the staged paths whose staging did not come back; `raw` as
    /// [`PutBackRan::Whole`]'s.
    Unstaged {
        paths: Vec<PathBuf>,
        raw: Option<NotAsTheyWere>,
    },
    /// Unmerged after either apply (ADR-0045 §2).
    Clashed { paths: Vec<PathBuf> },
    /// `S` stays listed: git refused and either wrote something or refused
    /// the plain apply too — or the staging lost could not be read before
    /// the plain apply, which then never ran.
    Failed { error: RunError },
}

/// Step 6's apply of `stash` (`S`, or absorb's rebuilt copy), with the
/// fallback, then the raw step from `his` after a clean one. `Err`: the
/// status could not be read after an apply, so whether it clashed is
/// unknown.
pub(super) fn run_put_back(
    ctx: &mut Context<'_>,
    stash: Oid,
    his: Option<HisBytes>,
    commands: &mut Vec<String>,
) -> Result<PutBackRan, FactsError> {
    let first = apply(ctx, stash, true, commands);
    match read(ctx, first)? {
        Read::Clean => {
            let raw = as_they_were(ctx, his, commands);
            return Ok(PutBackRan::Whole { raw });
        }
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
        Read::Clean => PutBackRan::Unstaged {
            paths,
            raw: as_they_were(ctx, his, commands),
        },
        Read::Clashed(paths) => PutBackRan::Clashed { paths },
        // Never a third apply.
        Read::Wrote(error) | Read::WroteNothing(error) => PutBackRan::Failed { error },
    })
}

/// The raw step: his own bytes (`his.raw`) over each path the apply wrote
/// back with his content in git's encoding. Never stopped, like the apply
/// before it — the second half of a write already done is not cut short —
/// so `ctx`'s stop is swapped for one that never fires and handed back on
/// the way out. Its reads and writes go on the Command Log as `u`'s
/// [`write_raw`] logs them.
fn as_they_were(
    ctx: &mut Context<'_>,
    his: Option<HisBytes>,
    commands: &mut Vec<String>,
) -> Option<NotAsTheyWere> {
    let his = his?;
    let never: &dyn Stop = &();
    let asked = std::mem::replace(&mut ctx.stop, never);
    let failed = match same_content_paths(ctx, his.raw, his.tree, commands) {
        Ok(paths) if paths.is_empty() => None,
        Ok(paths) => write_raw(ctx, his.raw, &paths, commands)
            .err()
            .map(|error| NotAsTheyWere {
                error,
                files: paths.len(),
            }),
        Err(error) => Some(NotAsTheyWere { error, files: 0 }),
    };
    ctx.stop = asked;
    failed
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
    if tracked_changed(status) {
        Read::Wrote(error)
    } else {
        Read::WroteNothing(error)
    }
}

/// Whether a tracked file differs from `HEAD`: an entry staged, unstaged or
/// unmerged. Untracked files do not count (`S` holds none). The put-back's
/// judge of "wrote nothing" ([`judge`] has returned on unmerged before it
/// asks), and recovery's of whether a killed put-back had landed (30c).
pub(crate) fn tracked_changed(status: &WorkingTreeStatus) -> bool {
    status
        .entries
        .iter()
        .any(|entry| entry.is_staged() || entry.is_unstaged() || entry.unmerged)
}

/// Staged paths a plain `stash apply` loses: every staged change in
/// `S^1..S^2`, except an added path whose staged blob equals its
/// working-tree blob (git re-adds that one from the tree, unchanged). An
/// added path edited on (`AM`) is lost: git re-adds it from the tree (review
/// m7-30-06d Major 1). One deleted since (`AD`) is not: `git stash create`
/// keeps its staged blob in `S`'s tree, so it comes back staged as it was
/// (and on disk again; 30c's A2). No rename detection, so a staged rename
/// names its old path (its deletion comes back unstaged), and there are no
/// copies.
///
/// The second read (`S^2..S`, what moved on after staging) runs only when a
/// path was added, so the usual fallback spawns nothing more. It takes no
/// pathspec: the added paths are matched in memory, so there is no argv
/// limit and no pathspec magic to escape.
fn staging_lost(ctx: &Context<'_>, stash: Oid) -> Result<Vec<PathBuf>, RunError> {
    let staged = name_status(&diff_tree(ctx, stash, "^1", "^2", "--name-status")?)?;
    let moved_on = if staged.iter().any(|(status, _)| *status == b'A') {
        name_only(&diff_tree(ctx, stash, "^2", "", "--name-only")?)
    } else {
        BTreeSet::new()
    };
    Ok(lost(&staged, &moved_on))
}

/// `git diff-tree -r --no-renames <how> -z <stash><from> <stash><to>`'s
/// stdout. A read, so not on the Command Log.
fn diff_tree(
    ctx: &Context<'_>,
    stash: Oid,
    from: &str,
    to: &str,
    how: &'static str,
) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(ctx.repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag(how),
            GitArg::Flag("-z"),
            GitArg::Word(format!("{stash}{from}")),
            GitArg::Word(format!("{stash}{to}")),
        ])
        .run(&())?;
    Ok(output.stdout)
}

/// The rule, pure. `staged`: `diff-tree --name-status S^1 S^2` as
/// [`name_status`] holds it; `moved_on`: the paths of `diff-tree
/// --name-only S^2 S`. Every status but `A` and `C` is named; an `A` only
/// when it moved on. In path order with no duplicates, the order git prints.
fn lost(staged: &[(u8, PathBuf)], moved_on: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    staged
        .iter()
        .filter(|(status, path)| match status {
            b'A' => moved_on.contains(path),
            b'C' => false,
            _ => true,
        })
        .map(|(_, path)| path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// `--name-status -z` output (`<X>\0<path>\0` pairs) → `(X, path)`, `X` the
/// status letter (a score after it, as in `C75`, is dropped).
fn name_status(printed: &[u8]) -> Result<Vec<(u8, PathBuf)>, RunError> {
    let mut fields = printed.split(|byte| *byte == 0).filter(|f| !f.is_empty());
    let mut staged = Vec::new();
    while let Some(status) = fields.next() {
        let (Some(letter), Some(path)) = (status.first(), fields.next()) else {
            return Err(RunError::Unparsable(format!(
                "diff-tree printed a status with no path: {:?}",
                String::from_utf8_lossy(status)
            )));
        };
        staged.push((
            *letter,
            PathBuf::from(String::from_utf8_lossy(path).into_owned()),
        ));
    }
    Ok(staged)
}

/// `--name-only -z` output → its paths.
fn name_only(printed: &[u8]) -> BTreeSet<PathBuf> {
    printed
        .split(|byte| *byte == 0)
        .filter(|f| !f.is_empty())
        .map(|path| PathBuf::from(String::from_utf8_lossy(path).into_owned()))
        .collect()
}

#[cfg(test)]
mod tests;
