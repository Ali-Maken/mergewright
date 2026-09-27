//! The one-command-at-a-time half: each [`Step`] of an [`InversePlan`], and
//! the reads that scope it.
//!
//! Nothing here decides anything. A step is handed in, the `git` that carries
//! it out is built and run, and its command line is recorded for the Command
//! Log. What to run, in what order, and what to do when one refuses are
//! [`super::run_plan`]'s and [`super::rollback`]'s business.
//!
//! [`InversePlan`]: crate::undo::InversePlan

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use graft_core::journal::{HeadRef, IndexState};
use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError};

use crate::apply::Context;
use crate::snapshot;
use crate::undo::{IndexPaths, PathScope, Step};

use super::RestoreError;

pub(super) fn run_step(
    ctx: &mut Context<'_>,
    step: &Step,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    match step {
        Step::UpdateRef { name, expect, to } => update_ref(ctx, name, *expect, *to, commands),
        Step::SetHead(head) => set_head(ctx, head, commands),
        Step::RestoreIndex { index, paths } => restore_index(ctx, *index, paths, commands),
        Step::RestoreWorktree { tree, paths } => restore_worktree(ctx, *tree, paths, commands),
        Step::StashStore { commit } => stash_store(ctx, *commit, commands),
        Step::StashDrop { commit } => stash_drop(ctx, *commit, commands),
        Step::SetMergeState(state) => {
            crate::merge_state::write_merge_state(ctx.repo, state.as_ref(), ctx.stop, commands)
                .map_err(|why| match why {
                    // A stop stays a stop: the cancel path is chosen on
                    // `RestoreError::STOPPED` alone.
                    crate::merge_state::MergeStateError::Run(RunError::Stopped) => {
                        RestoreError::STOPPED
                    }
                    other => RestoreError::MergeState(other),
                })
        }
        Step::SetBranchConfig { key, to } => set_branch_config(ctx, key, to, commands),
    }
}

/// Make `key` hold exactly `to` in the local config: `--replace-all` and
/// `--add`, or `--unset-all` for none.
fn set_branch_config(
    ctx: &mut Context<'_>,
    key: &str,
    to: &[String],
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    for args in crate::branch_config::writes(key, to) {
        match run(ctx, args.clone(), None, commands) {
            Ok(_) => {}
            // An unset of a key that is not set leaves it as asked.
            Err(RestoreError::Run(why)) if crate::branch_config::already_so(&args, &why) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(())
}

/// Compare-and-swap a ref back.
///
/// `expect` is where the journal says the ref stands now; `to` is where it
/// stood before. `None` on the `expect` side is "it should not exist", which
/// git spells as an empty old value — the all-zero id would have to be the
/// right hash length, and the empty string does not. `None` on both sides is
/// nothing to do.
fn update_ref(
    ctx: &mut Context<'_>,
    name: &str,
    expect: Option<Oid>,
    to: Option<Oid>,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let mut args = vec![GitArg::Flag("update-ref")];
    match (to, expect) {
        (None, None) => return Ok(()),
        (Some(to), expect) => {
            args.push(GitArg::Ref(name.as_bytes().to_vec()));
            args.push(GitArg::Oid(to));
            match expect {
                Some(expect) => args.push(GitArg::Oid(expect)),
                None => args.push(GitArg::Word(String::new())),
            }
        }
        (None, Some(expect)) => {
            args.push(GitArg::Flag("-d"));
            args.push(GitArg::Ref(name.as_bytes().to_vec()));
            args.push(GitArg::Oid(expect));
        }
    }
    match run(ctx, args, None, commands) {
        Ok(_) => Ok(()),
        // git prints `cannot lock ref` for a ref another process holds too,
        // and "is not where it was" would be the wrong diagnosis for that.
        // These two lines are the compare-and-swap ones (m3/03 review,
        // Minor 7); anything else stays the git error it is.
        Err(RestoreError::Run(RunError::Exited {
            stderr_first_line, ..
        })) if stderr_first_line.contains("but expected")
            || stderr_first_line.contains("reference already exists") =>
        {
            Err(RestoreError::RefMoved {
                name: name.to_owned(),
            })
        }
        Err(other) => Err(other),
    }
}

/// Put `HEAD` back where it pointed.
fn set_head(
    ctx: &mut Context<'_>,
    head: &HeadRef,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let args = match head {
        // An unborn branch is the same command as a born one: the branch ref
        // itself is put back (or left deleted) by its own `UpdateRef`.
        HeadRef::Branch(name) | HeadRef::Unborn(name) => vec![
            GitArg::Flag("symbolic-ref"),
            GitArg::Ref(b"HEAD".to_vec()),
            GitArg::Ref(name.as_bytes().to_vec()),
        ],
        HeadRef::Detached(id) => vec![
            GitArg::Flag("update-ref"),
            GitArg::Flag("--no-deref"),
            GitArg::Ref(b"HEAD".to_vec()),
            GitArg::Oid(*id),
        ],
    };
    run(ctx, args, None, commands).map(|_output| ())
}

/// Put the scoped paths back the way the recorded index state holds them,
/// through **one** `update-index --index-info`.
///
/// Never `read-tree` on the real index: that would replace the whole index,
/// including paths the operation never touched. Never `checkout --merge`
/// either — the stages come out of the recorded tree, so nothing here replays
/// a merge (m6-conflicts/08's constraint).
///
/// Every path gets a **mode-0 removal line before its entries**. That is
/// git's documented rule for placing higher-stage entries, and without it a
/// path that is unmerged now keeps its stale stages when the recorded state
/// has it merged, and the other way round (verified, git 2.54).
fn restore_index(
    ctx: &mut Context<'_>,
    index: IndexState,
    scope: &IndexPaths,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let paths = index_scoped_paths(ctx, index, scope, commands)?;
    put_stages_back(ctx, index, &paths, commands).map_err(as_snapshot_error)
}

/// The write half of [`restore_index`], over paths the caller already has.
///
/// Its error set is exactly [`snapshot::SnapshotError`]'s — one `ls-tree`
/// read and one `update-index`, with the stop kept as a stop — so a caller
/// outside a restore plan ([`crate::restore::restore_stages`], which `d`
/// after `⏎` goes through) gets the failure it can actually map, rather than
/// a [`RestoreError`] whose other dozen variants it would have to pretend to
/// handle.
pub(crate) fn put_stages_back(
    ctx: &mut Context<'_>,
    index: IndexState,
    paths: &[PathBuf],
    commands: &mut Vec<String>,
) -> Result<(), snapshot::SnapshotError> {
    if paths.is_empty() {
        return Ok(());
    }
    let held = snapshot::stage_entries_for(ctx.repo, index, paths, ctx.stop, Some(commands))?;
    let stdin = index_info_records(paths, &held, index.tree().as_bytes().len() * 2);
    let args = vec![
        GitArg::Flag("update-index"),
        GitArg::Flag("-z"),
        GitArg::Flag("--index-info"),
    ];
    let invocation = GitInvocation::new(ctx.repo).args(args).stdin(stdin);
    commands.push(invocation.command_line());
    invocation
        .run(ctx.stop)
        .map(|_output| ())
        .map_err(|why| match why {
            // The stop stays a stop: folding it into `Run` would read as a
            // git failure and take the cancel path away from the caller.
            RunError::Stopped => snapshot::SnapshotError::Stopped,
            other => snapshot::SnapshotError::Run(other),
        })
}

/// The bytes one `update-index -z --index-info` takes: per path, the mode-0
/// removal, then every entry the recorded state holds for it, in stage order.
///
/// Pure, so the order can be asserted without a repository — it is the whole
/// correctness of the step (`unmerged_index_info_lines`). A path the state
/// does not hold gets the removal line alone, which is how a restore takes a
/// staged entry back out.
fn index_info_records(
    paths: &[PathBuf],
    held: &BTreeMap<PathBuf, Vec<snapshot::StageEntry>>,
    zeros: usize,
) -> Vec<u8> {
    let zero = "0".repeat(zeros);
    let mut stdin = Vec::new();
    for path in paths {
        let name = path.as_os_str().as_encoded_bytes();
        stdin.extend_from_slice(format!("0 {zero}\t").as_bytes());
        stdin.extend_from_slice(name);
        stdin.push(0);
        for entry in held.get(path).into_iter().flatten() {
            stdin.extend_from_slice(
                format!("{:06o} {} {}\t", entry.mode, entry.oid, entry.stage).as_bytes(),
            );
            stdin.extend_from_slice(name);
            stdin.push(0);
        }
    }
    stdin
}

/// The paths an index restore may touch, and no others.
///
/// `ChangedBetween` is the operation's own footprint over the two recorded
/// states; `Exactly` is one file being started over; `Everything` is the
/// fallback for an operation that died before its after-state could be
/// recorded — the union of what the backup holds and what the index tracks
/// now, since an unmerged index has no tree to diff against.
fn index_scoped_paths(
    ctx: &mut Context<'_>,
    index: IndexState,
    scope: &IndexPaths,
    commands: &mut Vec<String>,
) -> Result<Vec<PathBuf>, RestoreError> {
    let found = match scope {
        IndexPaths::Exactly(paths) => return Ok(paths.clone()),
        IndexPaths::ChangedBetween { from, to } => {
            snapshot::index_paths_changed(ctx.repo, *from, *to, ctx.stop, commands)
        }
        IndexPaths::Everything => snapshot::index_paths(ctx.repo, index, ctx.stop, commands)
            .and_then(|mut union| {
                union.extend(snapshot::tracked_paths_now(ctx.repo, ctx.stop, commands)?);
                Ok(union)
            }),
    };
    Ok(found.map_err(as_snapshot_error)?.into_iter().collect())
}

/// Put the working-tree content of the scoped paths back.
///
/// Four cases, because git has no one command for them: a tracked path the
/// tree has, an untracked path the tree has (`restore --source` refuses a
/// path git does not track — hence the temporary index), a tracked path the
/// tree does not have (git removes it), and an untracked path the tree does
/// not have. Nothing outside `paths` is touched.
fn restore_worktree(
    ctx: &mut Context<'_>,
    tree: Oid,
    scope: &PathScope,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let paths = scoped_paths(ctx, tree, scope, commands)?;
    if paths.is_empty() {
        return Ok(());
    }
    let present = tree_entries(ctx, tree, &paths, commands)?;
    let tracked = tracked_now(ctx, &paths, commands)?;

    let mut restore_from_tree = Vec::new();
    let mut checkout_from_temp = Vec::new();
    let mut clean = Vec::new();
    for path in &paths {
        match (present.contains_key(path), tracked.contains(path)) {
            // Tracked: git writes the tree's content, or removes the path
            // when the tree does not have it.
            (_, true) => restore_from_tree.push(path.clone()),
            // In the tree but not tracked: `restore --source` would refuse
            // it, so it comes out of a temporary index.
            (true, false) => checkout_from_temp.push(path.clone()),
            // Untracked and absent from the tree: it was never git's.
            (false, false) => clean.push(path.clone()),
        }
    }

    if !restore_from_tree.is_empty() {
        let mut args = vec![
            GitArg::Flag("--literal-pathspecs"),
            GitArg::Flag("restore"),
            GitArg::Word(format!("--source={tree}")),
            GitArg::Flag("--worktree"),
            GitArg::Separator,
        ];
        args.extend(restore_from_tree.into_iter().map(GitArg::Path));
        run(ctx, args, None, commands)?;
    }
    if !checkout_from_temp.is_empty() {
        let temp = ctx
            .scratch
            .join(format!("tmp-restore-{}", std::process::id()));
        let outcome = checkout_untracked(ctx, tree, &temp, &checkout_from_temp, commands);
        // The temporary index is scratch: removing it cannot fail the
        // restore, and a survivor is overwritten by the next one.
        if temp.exists() {
            let _removed = std::fs::remove_file(&temp);
        }
        outcome?;
    }
    if !clean.is_empty() {
        let mut args = vec![
            GitArg::Flag("--literal-pathspecs"),
            GitArg::Flag("clean"),
            GitArg::Flag("-f"),
            GitArg::Separator,
        ];
        args.extend(clean.into_iter().map(GitArg::Path));
        run(ctx, args, None, commands)?;
    }
    Ok(())
}

fn checkout_untracked(
    ctx: &mut Context<'_>,
    tree: Oid,
    temp: &std::path::Path,
    paths: &[Vec<u8>],
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let read = GitInvocation::new(ctx.repo)
        .env("GIT_INDEX_FILE", temp)
        .arg(GitArg::Flag("read-tree"))
        .arg(GitArg::Oid(tree));
    commands.push(read.command_line());
    read.run(ctx.stop).map_err(as_restore_error)?;

    let mut checkout = GitInvocation::new(ctx.repo)
        .env("GIT_INDEX_FILE", temp)
        .arg(GitArg::Flag("--literal-pathspecs"))
        .arg(GitArg::Flag("checkout-index"))
        .arg(GitArg::Flag("-f"))
        .arg(GitArg::Separator);
    for path in paths {
        checkout = checkout.arg(GitArg::Path(path.clone()));
    }
    commands.push(checkout.command_line());
    checkout.run(ctx.stop).map_err(as_restore_error)?;
    Ok(())
}

/// Put a stash entry back in the reflog.
///
/// Idempotent by the executor's doing, not git's: `git stash store` of a
/// commit already at `stash@{1}` adds a **duplicate** on top (verified, git
/// 2.54), so the reflog is scanned first. The message is the commit's own
/// subject, which is byte-identical to the reflog message git wrote when the
/// stash was made.
fn stash_store(
    ctx: &mut Context<'_>,
    commit: Oid,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    if stash_reflog(ctx, commands)?.contains(&commit) {
        return Ok(());
    }
    let subject = GitInvocation::new(ctx.repo)
        .arg(GitArg::Flag("log"))
        .arg(GitArg::Flag("-1"))
        .arg(GitArg::Flag("--format=%s"))
        .arg(GitArg::Oid(commit));
    commands.push(subject.command_line());
    // A commit that cannot be read is the one failure a pin exists to
    // prevent, and it is not a generic git error: say which commit is gone.
    let subject = subject
        .run(ctx.stop)
        .map_err(|why| match why {
            // Only git running and refusing means the commit is gone; git
            // missing from PATH must not read as "the stash is gone"
            // (m3/03 review, Minor 6).
            RunError::Exited { .. } => RestoreError::StashGone { commit },
            other => as_restore_error(other),
        })?
        .stdout;
    let message = String::from_utf8_lossy(&subject).trim_end().to_owned();
    let args = vec![
        GitArg::Flag("stash"),
        GitArg::Flag("store"),
        GitArg::Flag("-m"),
        GitArg::Word(message),
        GitArg::Oid(commit),
    ];
    run(ctx, args, None, commands).map(|_output| ())
}

/// Take a stash entry back out of the reflog.
///
/// A no-op when the reflog does not hold the commit: that is what makes the
/// undo of an *aborted* pop correct, since a failed `git stash pop` keeps its
/// entry (verified, git 2.54).
fn stash_drop(
    ctx: &mut Context<'_>,
    commit: Oid,
    commands: &mut Vec<String>,
) -> Result<(), RestoreError> {
    let reflog = stash_reflog(ctx, commands)?;
    let Some(at) = reflog.iter().position(|held| *held == commit) else {
        return Ok(());
    };
    let args = vec![
        GitArg::Flag("stash"),
        GitArg::Flag("drop"),
        GitArg::Word(format!("stash@{{{at}}}")),
    ];
    run(ctx, args, None, commands).map(|_output| ())
}

/// `refs/stash`'s reflog, newest first: the stash list itself.
fn stash_reflog(
    ctx: &mut Context<'_>,
    commands: &mut Vec<String>,
) -> Result<Vec<Oid>, RestoreError> {
    let exists = GitInvocation::new(ctx.repo)
        .arg(GitArg::Flag("for-each-ref"))
        .arg(GitArg::Flag("--format=%(refname)"))
        .arg(GitArg::Ref(b"refs/stash".to_vec()));
    commands.push(exists.command_line());
    let found = exists.run(ctx.stop).map_err(as_restore_error)?;
    if found.stdout.is_empty() {
        return Ok(Vec::new());
    }
    let args = vec![
        GitArg::Flag("reflog"),
        GitArg::Flag("show"),
        GitArg::Flag("--format=%H"),
        GitArg::Ref(b"refs/stash".to_vec()),
    ];
    let output = run(ctx, args, None, commands)?;
    let text = String::from_utf8_lossy(&output);
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match Oid::from_hex(line) {
            Ok(id) => found.push(id),
            // `reflog show` prints one full id per line and nothing else;
            // anything that is not an id means git changed its output, which
            // must not be read as "the stash is empty".
            Err(_what) => {
                return Err(RestoreError::Run(RunError::Exited {
                    status: 0,
                    stderr_first_line: format!("reflog show printed {line}, not an object id"),
                    command: "git reflog show --format=%H refs/stash".to_owned(),
                }));
            }
        }
    }
    Ok(found)
}

/// The paths a working-tree step may touch, and no others.
///
/// `ChangedBetween` is the operation's own footprint. `Everything` is the
/// fallback for an operation that died before its after-state could be
/// recorded: the "now" side is read fresh here, so the step still touches
/// only paths that actually differ from the backup — wide, but not blind.
/// `drift` has already told the user this is what is about to happen.
fn scoped_paths(
    ctx: &mut Context<'_>,
    tree: Oid,
    scope: &PathScope,
    commands: &mut Vec<String>,
) -> Result<Vec<Vec<u8>>, RestoreError> {
    let (from, to) = match scope {
        PathScope::ChangedBetween { from, to } => (*from, *to),
        PathScope::Everything => {
            let now = snapshot::worktree_tree(ctx.repo, &ctx.scratch, ctx.stop)
                .map_err(RestoreError::Snapshot)?;
            (tree, now)
        }
    };
    if from == to {
        return Ok(Vec::new());
    }
    let args = vec![
        GitArg::Flag("diff-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("--name-only"),
        GitArg::Flag("-z"),
        GitArg::Oid(from),
        GitArg::Oid(to),
    ];
    let output = run(ctx, args, None, commands)?;
    Ok(split_nul(&output))
}

/// `<mode> <oid>` for each of `paths` the tree holds.
fn tree_entries(
    ctx: &mut Context<'_>,
    tree: Oid,
    paths: &[Vec<u8>],
    commands: &mut Vec<String>,
) -> Result<BTreeMap<Vec<u8>, (String, Oid)>, RestoreError> {
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Oid(tree),
        GitArg::Separator,
    ];
    args.extend(paths.iter().cloned().map(GitArg::Path));
    let output = run(ctx, args, None, commands)?;
    let mut found = BTreeMap::new();
    for record in split_nul(&output) {
        // A record this cannot parse must never be read as "the tree does
        // not have this path": `restore_index` writes an absent path to
        // `update-index` as a deletion, so a parse slip would drop a staged
        // entry with nothing said (m3/03 review, M4). `stash_reflog` treats
        // git printing the unexpected the same way.
        let malformed = |what: &str| {
            Err(RestoreError::Run(RunError::Exited {
                status: 0,
                stderr_first_line: format!(
                    "ls-tree printed a record this cannot read ({what}): {}",
                    String::from_utf8_lossy(&record)
                ),
                command: format!("git ls-tree -r -z {tree}"),
            }))
        };
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return malformed("no tab");
        };
        let (head, path) = record.split_at(tab);
        let head = String::from_utf8_lossy(head);
        let mut fields = head.split_whitespace();
        let (Some(mode), Some(_kind), Some(id)) = (fields.next(), fields.next(), fields.next())
        else {
            return malformed("fewer than three fields");
        };
        let Ok(id) = Oid::from_hex(id) else {
            return malformed("not an object id");
        };
        found.insert(path[1..].to_vec(), (mode.to_owned(), id));
    }
    Ok(found)
}

/// Which of `paths` the index tracks right now.
fn tracked_now(
    ctx: &mut Context<'_>,
    paths: &[Vec<u8>],
    commands: &mut Vec<String>,
) -> Result<BTreeSet<Vec<u8>>, RestoreError> {
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-files"),
        GitArg::Flag("-z"),
        GitArg::Separator,
    ];
    args.extend(paths.iter().cloned().map(GitArg::Path));
    let output = run(ctx, args, None, commands)?;
    Ok(split_nul(&output).into_iter().collect())
}

fn split_nul(bytes: &[u8]) -> Vec<Vec<u8>> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// Run one `git`, recording its command line for the Command Log.
fn run(
    ctx: &mut Context<'_>,
    args: Vec<GitArg>,
    stdin: Option<Vec<u8>>,
    commands: &mut Vec<String>,
) -> Result<Vec<u8>, RestoreError> {
    let mut invocation = GitInvocation::new(ctx.repo).args(args);
    if let Some(bytes) = stdin {
        invocation = invocation.stdin(bytes);
    }
    commands.push(invocation.command_line());
    invocation
        .run(ctx.stop)
        .map(|output| output.stdout)
        .map_err(as_restore_error)
}

/// A snapshot failure as a restore failure — the **stop kept as a stop**.
///
/// `RestoreError::Snapshot(Stopped)` would read as a git failure, and the
/// cancel path (m5/06: a stop rolls back what already ran) is chosen on
/// `RestoreError::STOPPED` alone, so folding the two would leave a cancel
/// half-applied.
fn as_snapshot_error(why: snapshot::SnapshotError) -> RestoreError {
    match why {
        snapshot::SnapshotError::Stopped => RestoreError::STOPPED,
        other => RestoreError::Snapshot(other),
    }
}

fn as_restore_error(why: RunError) -> RestoreError {
    // A stop included: `Run(Stopped)` is the one stop shape (`KNOWN_GAPS` :495).
    RestoreError::Run(why)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex(&format!("{byte:02x}").repeat(20)).expect("a hex oid")
    }

    fn entry(mode: u32, id: u8, stage: u8) -> snapshot::StageEntry {
        snapshot::StageEntry {
            mode,
            oid: oid(id),
            stage,
        }
    }

    /// The records a conflicted path restores through: the mode-0 removal
    /// first, then stages 1, 2, 3, all NUL-terminated, the path repeated on
    /// every line.
    ///
    /// The removal line is git's own documented rule for `--index-info`: an
    /// entry at stage 0 blocks a higher stage from being added, so the path
    /// has to go before its stages come back. Without it the resolved entry
    /// survives and the path still reads as merged.
    #[test]
    fn unmerged_index_info_lines() {
        let path = PathBuf::from("both.txt");
        let mut held = BTreeMap::new();
        held.insert(
            path.clone(),
            vec![
                entry(0o100_644, 1, 1),
                entry(0o100_644, 2, 2),
                entry(0o100_755, 3, 3),
            ],
        );

        let records = index_info_records(std::slice::from_ref(&path), &held, 40);
        let lines: Vec<String> = records
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
            .map(|record| String::from_utf8_lossy(record).into_owned())
            .collect();

        assert_eq!(
            lines,
            vec![
                format!("0 {}\tboth.txt", "0".repeat(40)),
                format!("100644 {} 1\tboth.txt", oid(1)),
                format!("100644 {} 2\tboth.txt", oid(2)),
                format!("100755 {} 3\tboth.txt", oid(3)),
            ]
        );
        assert_eq!(
            records.last(),
            Some(&0),
            "every record is NUL-terminated, the last one too"
        );
    }

    /// A path the recorded state does not hold gets the removal line and
    /// nothing else: that is how a restore drops an index entry the
    /// operation added.
    #[test]
    fn a_path_with_no_stages_is_only_removed() {
        let path = PathBuf::from("gone.rs");
        let records = index_info_records(std::slice::from_ref(&path), &BTreeMap::new(), 40);
        assert_eq!(
            String::from_utf8_lossy(&records),
            format!("0 {}\tgone.rs\0", "0".repeat(40))
        );
    }

    /// A merged path is one stage-0 line after its removal — the shape every
    /// pre-M6 restore wrote, unchanged.
    #[test]
    fn a_merged_path_is_one_stage_zero_line() {
        let path = PathBuf::from("a.rs");
        let mut held = BTreeMap::new();
        held.insert(path.clone(), vec![entry(0o100_644, 7, 0)]);
        let records = index_info_records(std::slice::from_ref(&path), &held, 40);
        assert_eq!(
            String::from_utf8_lossy(&records),
            format!("0 {}\ta.rs\0100644 {} 0\ta.rs\0", "0".repeat(40), oid(7))
        );
    }
}
