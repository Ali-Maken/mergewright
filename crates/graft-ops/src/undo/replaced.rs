//! What an inverse run over drift replaces (m6-conflicts/77).
//!
//! [`crate::undo::drift`] compares tree ids, so it can say *that* the
//! working tree changed since the operation but never *which* paths. The
//! drift box and the landing after `y` owe the user the paths — "undo
//! replaces your edit to fix.txt" — so this reads them, over trees that
//! already exist: nothing is hashed or written.
//!
//! A path counts when the inverse writes it (it is in the plan's scope, the
//! operation's own footprint), the user changed it since (it differs from the
//! entry's recorded after-state), and the write loses something (what is
//! there now is not already what the inverse puts back). Edits to other paths
//! stay in the tree and are not named.
//!
//! Run by `execute::run_plan` only when the drift is not clean, so a clean
//! undo pays nothing — that rests on the gate there and on the early returns
//! in `in_worktree` and `in_index` (`after == now || after == before`).
//! A forced run pays at most **8** spawns (m6-conflicts/80, review m1): two
//! `diff-tree`s for the working tree, and three index comparisons of which
//! each is one `diff-tree` for two states of a shape but **two** `ls-tree -r`
//! for a merged state against an unmerged one (an undo inside a live merge,
//! m6-conflicts/57) — 2 + 3 × 2. Each `ls-tree -r` lists the whole index
//! tree; on a very large repository that cost is not measured. Every spawn
//! is in the entry's Command Log.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use graft_core::journal::IndexState;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::apply::Context;
use crate::facts::FactsError;
use crate::snapshot::{self, SnapshotError};
use crate::undo::{IndexPaths, InversePlan, PathScope, Replaced, RepoStateNow, Step};

/// What running `plan` now would replace, against the repository as the rail
/// read it (`now`). Every read's command line goes to `commands`, for the
/// forced run's Command Log: the working tree's `diff-tree`s and the index's
/// comparisons through [`snapshot::index_paths_changed`] alike
/// (m6-conflicts/80).
pub(super) fn replaced(
    ctx: &Context<'_>,
    plan: &InversePlan,
    now: &RepoStateNow,
    commands: &mut Vec<String>,
) -> Result<Replaced, FactsError> {
    let mut paths = BTreeSet::new();
    let mut whole_tree = false;
    let mut whole_index = false;
    for step in &plan.steps {
        match step {
            Step::RestoreWorktree {
                paths: PathScope::ChangedBetween { from, to },
                ..
            } => paths.extend(in_worktree(ctx, *from, *to, now.worktree_tree, commands)?),
            Step::RestoreIndex {
                paths: IndexPaths::ChangedBetween { from, to },
                ..
            } => paths.extend(in_index(ctx, *from, *to, now.index, commands)?),
            Step::RestoreIndex {
                index,
                paths: IndexPaths::Exactly(only),
            } => {
                // Never planned by `inverse` today (`d` asks for one path,
                // outside any undo). The step carries no after-state, so the
                // "changed since" half cannot be asked: what the write would
                // change is the most that can be said, and it is named.
                if !only.is_empty() {
                    let lost = snapshot::index_paths_changed(
                        ctx.repo, now.index, *index, ctx.stop, commands,
                    )?;
                    paths.extend(only.iter().filter(|path| lost.contains(*path)).cloned());
                }
            }
            Step::RestoreWorktree {
                paths: PathScope::Everything,
                ..
            } => whole_tree = true,
            Step::RestoreIndex {
                paths: IndexPaths::Everything,
                ..
            } => whole_index = true,
            Step::UpdateRef { .. }
            | Step::SetHead(_)
            | Step::StashStore { .. }
            | Step::StashDrop { .. }
            | Step::SetMergeState(_) => {}
        }
    }
    Ok(Replaced {
        paths: paths.into_iter().collect(),
        whole_tree,
        whole_index,
    })
}

/// The working tree's share: of the paths the operation changed (`after` →
/// `before`), those that now hold neither the recorded after-content nor the
/// before-content the restore writes.
///
/// Two raw diffs, each giving the second tree's `(mode, blob)` per path, so
/// the three-way comparison needs no third read: a path in both diffs is
/// compared side by side; a path only in the first is outside the restore's
/// scope and is left alone.
fn in_worktree(
    ctx: &Context<'_>,
    after: Oid,
    before: Oid,
    now: Oid,
    commands: &mut Vec<String>,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if after == now || after == before {
        return Ok(BTreeSet::new());
    }
    let since = raw_diff(ctx, after, now, commands)?;
    if since.is_empty() {
        return Ok(BTreeSet::new());
    }
    let writes = raw_diff(ctx, after, before, commands)?;
    Ok(since
        .into_iter()
        .filter(|(path, holds_now)| writes.get(path).is_some_and(|written| written != holds_now))
        .map(|(path, _)| path)
        .collect())
}

/// The index's share, the same rule over [`IndexState`]s: two states of
/// different shapes (merged against a stage tree) cannot be diffed as
/// trees, and [`snapshot::index_paths_changed`] is the one comparison that
/// copes with both.
fn in_index(
    ctx: &Context<'_>,
    after: IndexState,
    before: IndexState,
    now: IndexState,
    commands: &mut Vec<String>,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    if after == now || after == before {
        return Ok(BTreeSet::new());
    }
    let since = snapshot::index_paths_changed(ctx.repo, after, now, ctx.stop, commands)?;
    if since.is_empty() {
        return Ok(BTreeSet::new());
    }
    let writes = snapshot::index_paths_changed(ctx.repo, after, before, ctx.stop, commands)?;
    let written_since: BTreeSet<PathBuf> = since.intersection(&writes).cloned().collect();
    if written_since.is_empty() {
        return Ok(written_since);
    }
    let lost = snapshot::index_paths_changed(ctx.repo, now, before, ctx.stop, commands)?;
    Ok(written_since.intersection(&lost).cloned().collect())
}

/// One side of a raw diff record: the mode and blob a path holds in the
/// second tree, `None` where it is absent there.
pub(crate) type Holds = Option<(String, Oid)>;

/// `git diff-tree -r -z --no-renames <from> <to>`: every path that differs,
/// with what it holds in `to`. Logged to `commands`.
fn raw_diff(
    ctx: &Context<'_>,
    from: Oid,
    to: Oid,
    commands: &mut Vec<String>,
) -> Result<BTreeMap<PathBuf, Holds>, FactsError> {
    tree_diff(ctx.repo, from, to, ctx.stop, commands)
}

/// [`raw_diff`] outside an undo: the put-back's preview and `lost_edits`
/// (m7/23) read the same records. `from` and `to` are trees or commits.
pub(crate) fn tree_diff(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<BTreeMap<PathBuf, Holds>, FactsError> {
    let invocation = GitInvocation::new(repo).args(vec![
        GitArg::Flag("diff-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag("--no-renames"),
        GitArg::Oid(from),
        GitArg::Oid(to),
    ]);
    commands.push(invocation.command_line());
    let output = invocation.run(stop).map_err(|why| match why {
        RunError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(SnapshotError::Run(other)),
    })?;
    parse_raw(&output.stdout).map_err(FactsError::Snapshot)
}

/// `-z` raw output is NUL-separated `:<mode> <mode> <oid> <oid> <status>`
/// headers, each followed by its one path (no renames, so never two).
fn parse_raw(stdout: &[u8]) -> Result<BTreeMap<PathBuf, Holds>, SnapshotError> {
    let unreadable = |record: &[u8]| SnapshotError::NotAnOid {
        what: "diff-tree",
        printed: String::from_utf8_lossy(record).into_owned(),
    };
    let mut found = BTreeMap::new();
    let mut fields = stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(header) = fields.next() {
        let path = fields.next().ok_or_else(|| unreadable(header))?;
        let text = std::str::from_utf8(header).map_err(|_| unreadable(header))?;
        let mut parts = text.trim_start_matches(':').split(' ');
        let (Some(_old_mode), Some(new_mode), Some(_old), Some(new)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(unreadable(header));
        };
        let new = Oid::from_hex(new).map_err(|_| unreadable(header))?;
        let holds =
            (new_mode.bytes().any(|digit| digit != b'0')).then(|| (new_mode.to_owned(), new));
        found.insert(path_of(path), holds);
    }
    Ok(found)
}

fn path_of(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex(&format!("{byte:02x}").repeat(20)).expect("forty hex digits")
    }

    #[test]
    fn a_raw_record_gives_the_second_trees_side() {
        let mut out = Vec::new();
        out.extend_from_slice(format!(":100644 100644 {} {} M", oid(1), oid(2)).as_bytes());
        out.push(0);
        out.extend_from_slice(b"src/a b.rs");
        out.push(0);
        out.extend_from_slice(
            format!(
                ":100644 000000 {} {} D",
                oid(3),
                Oid::from_hex(&"0".repeat(40)).expect("zeros")
            )
            .as_bytes(),
        );
        out.push(0);
        out.extend_from_slice(b"gone.txt");
        out.push(0);
        let parsed = parse_raw(&out).expect("well-formed");
        assert_eq!(
            parsed.get(&PathBuf::from("src/a b.rs")),
            Some(&Some(("100644".to_owned(), oid(2))))
        );
        assert_eq!(parsed.get(&PathBuf::from("gone.txt")), Some(&None));
    }

    #[test]
    fn a_header_with_no_path_is_refused_not_dropped() {
        let header = format!(":100644 100644 {} {} M", oid(1), oid(2));
        assert!(parse_raw(header.as_bytes()).is_err());
    }
}
