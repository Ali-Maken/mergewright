//! The backup snapshot: both trees, one commit, one ref (ADR-0009).
//!
//! `git stash create` silently drops untracked files, so PROMPT.md §3 rules
//! it out. What Graft takes instead is two trees:
//!
//! - the **index tree**, the staged state exactly as it stands;
//! - the **worktree tree**, every tracked file's working-tree content plus
//!   every untracked file (design note change 1 — §6.3's "index tree plus
//!   untracked blobs" loses an unstaged edit to a tracked file, which is the
//!   commonest thing a discard destroys).
//!
//! Both come from a **copy** of the real index at
//! `<git-path graft>/tmp-index-*`. The copy carries git's stat cache, so
//! `add -A` re-hashes only the files whose stat data moved: 15 ms against
//! 236 ms for a fresh `read-tree` on a 5,000-file tree (git 2.54). Nothing
//! here ever writes the real `.git/index` — a read that rewrote the index
//! would fight the user's own git (M2 review round 2).
//!
//! The two trees are joined by `mktree` into `{ index/, worktree/ }` and
//! committed under `refs/graft/backup/<worktree>/<ulid>` (ADR-0010's
//! namespacing, its Correction's `<ulid>`), so both stay reachable and a
//! restore reads each by name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use graft_core::journal::{EntryId, IndexState, JournalEntry, Touches};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::apply::Context;

/// How much of the repository a snapshot has to cover before one operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotScope {
    /// Nothing to back up: the operation moves refs only.
    None,
    /// Both trees (ADR-0009: unconditional for anything touching the index
    /// or the working tree).
    Full,
}

/// One snapshot: the trees and the commit that holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The staged state. A half-finished merge has no tree `write-tree` will
    /// give, so it is captured as the **stage tree** instead
    /// ([`IndexState::Unmerged`]) — never as an absence (m6-conflicts/08).
    pub index: IndexState,
    /// Tracked working-tree content plus untracked files.
    pub worktree_tree: Oid,
    /// `{ index/ | unmerged/ → the index state, worktree/ → worktree_tree }`,
    /// committed.
    pub commit: Oid,
}

/// One `(mode, oid, stage)` an index holds for a path.
///
/// A merged path has exactly one, at stage 0. A conflicted one has up to
/// three: 1 the merge base, 2 ours, 3 theirs — and fewer when a side has no
/// version (`UD`, `AU`, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StageEntry {
    /// The file mode, as git prints it (`100644`, `100755`, `120000`).
    pub mode: u32,
    /// The blob.
    pub oid: Oid,
    /// 0, 1, 2 or 3.
    pub stage: u8,
}

/// Why a snapshot could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// A `git` command failed.
    #[error("{0}")]
    Run(#[from] RunError),
    /// The index could not be copied, or the temporary copy could not be
    /// removed.
    #[error("snapshot scratch: {0}")]
    Io(#[from] std::io::Error),
    /// `git` printed something that is not an object id — or a record whose
    /// shape the parser could not read one out of.
    #[error("git printed {what} where an object id belongs: {printed}")]
    NotAnOid {
        /// Which command.
        what: &'static str,
        /// What it printed, first line, lossy.
        printed: String,
    },
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

/// Hard choice 3: today, anything touching the index or the working tree
/// takes the full snapshot, and nothing else takes one.
///
/// ADR-0009 stands — detection can miss, a snapshot cannot — so this is
/// deliberately not clever. It is one function so that narrowing it later
/// (index-only ops to an index tree alone) is one edit with numbers behind
/// it, measured on the heavy fixture, never a silent drift.
pub fn scope(touches: &Touches) -> SnapshotScope {
    if touches.index || touches.worktree {
        SnapshotScope::Full
    } else {
        SnapshotScope::None
    }
}

/// `refs/graft/backup/<worktree>/<ulid>` — shared across worktrees by git,
/// namespaced per worktree by us (ADR-0010).
pub fn backup_ref_name(worktree: &str, id: &EntryId) -> String {
    format!("refs/graft/backup/{worktree}/{id}")
}

/// The staged state, read without touching the real index.
///
/// A merged index is the plain tree `write-tree` gives. An unmerged one —
/// what a stopped merge or a conflicted stash pop leaves — has no such tree
/// at all, so it is written as the **stage tree**: every index entry moved to
/// `<stage>/<path>` at stage 0 on a second temporary index, which `write-tree`
/// then accepts (m6-conflicts/08 decision 1).
///
/// No blob is written by either path. Every stage's blob is already an object
/// — ours is `HEAD`'s, theirs is `MERGE_HEAD`'s or the stash commit's, the
/// base is the merge base's — and the tree only makes them reachable from the
/// backup ref.
pub fn index_state(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<IndexState, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    index_state_of(repo, temp.path(), scratch, stop)
}

/// [`index_state`] over an index copy the caller already holds.
fn index_state_of(
    repo: &RepoHandle,
    index: &Path,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<IndexState, SnapshotError> {
    match try_write_tree(repo, index, stop)? {
        Some(tree) => Ok(IndexState::Merged(tree)),
        None => Ok(IndexState::Unmerged {
            unmerged: stage_tree(repo, index, scratch, stop)?,
        }),
    }
}

/// Tracked working-tree content plus untracked files, as a tree.
pub fn worktree_tree(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    add_all(repo, temp.path(), stop)?;
    write_tree(repo, temp.path(), stop)
}

/// Both trees, the joined tree, the commit and the backup ref.
///
/// `parents` are the commits the snapshot must keep reachable — the pins of
/// an operation that is about to make a stash commit unreachable — plus, for
/// an after-snapshot, the before-commit.
///
/// One index copy serves both trees: `write-tree` gives the index tree, then
/// `add -A` over the same copy and a second `write-tree` gives the worktree
/// tree. Two copies would pay the stat walk twice for nothing.
pub fn take(
    repo: &RepoHandle,
    scratch: &Path,
    id: &EntryId,
    worktree: &str,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Snapshot, SnapshotError> {
    let (index, worktree_tree) = both_trees(repo, scratch, stop)?;
    let commit = commit_trees(
        repo,
        Some(index),
        Some(worktree_tree),
        id,
        label,
        parents,
        stop,
    )?;
    let name = backup_ref_name(worktree, id);
    update_ref(repo, &name, commit, stop)?;
    Ok(Snapshot {
        index,
        worktree_tree,
        commit,
    })
}

/// Both trees from one index copy: `write-tree` (or the stage tree) for the
/// index side, then `add -A` over the same copy and a second `write-tree` for
/// the working tree. Two copies would pay the stat walk twice for nothing.
///
/// The index side is read **before** `add -A`, which resolves every conflict
/// in the copy: after it, the stages are gone.
fn both_trees(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<(IndexState, Oid), SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    let index = index_state_of(repo, temp.path(), scratch, stop)?;
    add_all(repo, temp.path(), stop)?;
    let worktree_tree = write_tree(repo, temp.path(), stop)?;
    Ok((index, worktree_tree))
}

/// Decision 2's chained commit: the repository as it stands **now**, hung off
/// the entry's backup ref, with no journal line of its own.
///
/// A step taken inside a live operation folds into that operation's one entry
/// (M5 hard choice 2), but git's own `add` drops stages 1–3 for the path and
/// nothing else keeps them. So every folded step and every `resume` takes one
/// of these first, parented on the ref's current commit: the ref stays a
/// single chain — pre-op → before step 1 → … → after — and `d`
/// ([`crate::operation::Operation::RestartConflict`]) reads the stages back
/// out of it instead of re-running the merge.
///
/// `Ok(None)` when the entry names no backup ref: there is nothing to chain
/// onto, which is exactly the case of an operation begun outside Graft.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run. Nothing is
/// written when it fails: the ref moves last.
pub fn chain_backup(ctx: &Context<'_>, entry: &JournalEntry) -> Result<Option<Oid>, SnapshotError> {
    let Some(name) = entry.backup_ref.as_deref() else {
        return Ok(None);
    };
    if ctx.stop.should_stop() {
        return Err(SnapshotError::Stopped);
    }
    let (index, worktree_tree) = both_trees(ctx.repo, ctx.scratch(), ctx.stop)?;
    let parents: Vec<Oid> = read_ref(ctx.repo, name, ctx.stop)?.into_iter().collect();
    let label = format!("step within: {}", entry.description);
    let commit = commit_trees(
        ctx.repo,
        Some(index),
        Some(worktree_tree),
        &entry.id,
        &label,
        &parents,
        ctx.stop,
    )?;
    move_backup_ref(ctx.repo, name, commit, ctx.stop)?;
    Ok(Some(commit))
}

/// The index state a backup commit records, or `None` when the commit is not
/// one of Graft's backups.
///
/// A backup commit's tree is `{ index/ | unmerged/, worktree/ }`
/// ([`commit_trees`]), so the **name** of the index subtree is what says which
/// shape it holds — the commit is readable on its own, without the journal
/// line that named it. A commit with neither subtree is somebody's own
/// history, which is how [`backup_chain`] knows it has walked off the end of
/// the chain.
///
/// # Errors
/// [`SnapshotError`] when `ls-tree` will not run or prints a record with no
/// object id in it.
pub fn recorded_index_state(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<IndexState>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-z"),
            GitArg::Oid(commit),
        ],
        None,
        stop,
    )?;
    for record in split_nul(&output.stdout) {
        // The name decides the shape, so the entry is read as a plain tree
        // record whatever it turns out to be.
        let (name, entry) = ls_tree_entry(&record, false)?;
        match name.as_os_str().as_encoded_bytes() {
            b"index" => return Ok(Some(IndexState::Merged(entry.oid))),
            b"unmerged" => {
                return Ok(Some(IndexState::Unmerged {
                    unmerged: entry.oid,
                }));
            }
            _ => {}
        }
    }
    Ok(None)
}

/// A backup ref's chain of recorded index states, **newest first**.
///
/// The ref's own commit, then its first parent, and so on — the single linear
/// chain [`chain_backup`] keeps: pre-operation → before step 1 → … The walk
/// stops at the first commit that is not a backup commit
/// ([`recorded_index_state`] gives `None`), which is where the pre-operation
/// snapshot's parents hand over to the user's own history. An absent ref is
/// an empty chain, not a failure.
///
/// This is what `d` reads after `⏎`: the stages git dropped when it staged
/// the path are in one of these, and nothing here re-runs the merge.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run.
pub fn backup_chain(
    repo: &RepoHandle,
    name: &str,
    stop: &dyn Stop,
) -> Result<Vec<IndexState>, SnapshotError> {
    let mut chain = Vec::new();
    let mut at = read_ref(repo, name, stop)?;
    while let Some(commit) = at {
        let Some(state) = recorded_index_state(repo, commit, stop)? else {
            break;
        };
        chain.push(state);
        at = first_parent(repo, commit, stop)?;
    }
    Ok(chain)
}

/// A commit's first parent, or `None` when it is a root.
///
/// `rev-list --parents -n 1` prints `<commit> <parent>…`, so one spawn
/// answers it without `rev-parse <c>^1` failing loudly on a root commit.
fn first_parent(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Flag("-n"),
            GitArg::Flag("1"),
            GitArg::Oid(commit),
        ],
        None,
        stop,
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(parent) = text.split_whitespace().nth(1) else {
        return Ok(None);
    };
    Oid::from_hex(parent)
        .map(Some)
        .map_err(|_| SnapshotError::NotAnOid {
            what: "rev-list --parents",
            printed: parent.to_owned(),
        })
}

/// The `(mode, oid, stage)` triples one path has in a recorded index state;
/// empty when the state does not hold the path at all.
///
/// # Errors
/// [`SnapshotError`] when `ls-tree` will not run or prints a record with no
/// object id in it.
pub fn stage_entries(
    repo: &RepoHandle,
    index: IndexState,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Vec<StageEntry>, SnapshotError> {
    let held = stage_entries_for(repo, index, std::slice::from_ref(&path.to_path_buf()), stop)?;
    Ok(held.into_values().next().unwrap_or_default())
}

/// The same for several paths at once: one `ls-tree`, not one per path.
pub(crate) fn stage_entries_for(
    repo: &RepoHandle,
    index: IndexState,
    paths: &[PathBuf],
    stop: &dyn Stop,
) -> Result<BTreeMap<PathBuf, Vec<StageEntry>>, SnapshotError> {
    tree_stage_entries(repo, index, Some(paths), stop)
}

/// Every path a recorded index state holds.
pub(crate) fn index_paths(
    repo: &RepoHandle,
    index: IndexState,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    Ok(tree_stage_entries(repo, index, None, stop)?
        .into_keys()
        .collect())
}

/// Every path the real index tracks right now, unmerged ones included.
pub(crate) fn tracked_paths_now(
    repo: &RepoHandle,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    let output = run_git(
        repo,
        vec![GitArg::Flag("ls-files"), GitArg::Flag("-z")],
        None,
        stop,
    )?;
    Ok(split_nul(&output.stdout).into_iter().map(path_of).collect())
}

/// The paths whose index entries differ between two recorded states.
///
/// Two states of the same shape diff as trees, with the stage prefix stripped
/// and the result de-duplicated (three stage lines are one path). Two of
/// **different** shapes cannot be diffed at all — one holds `2/a.rs` where the
/// other holds `a.rs` — so each is listed and the entry sets compared.
///
/// # Errors
/// [`SnapshotError`] when `git` will not run or prints an unreadable record.
pub fn index_paths_changed(
    repo: &RepoHandle,
    from: IndexState,
    to: IndexState,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    if from == to {
        return Ok(BTreeSet::new());
    }
    if from.is_unmerged() == to.is_unmerged() {
        let output = run_git(
            repo,
            vec![
                GitArg::Flag("diff-tree"),
                GitArg::Flag("-r"),
                GitArg::Flag("--name-only"),
                GitArg::Flag("-z"),
                GitArg::Oid(from.tree()),
                GitArg::Oid(to.tree()),
            ],
            None,
            stop,
        )?;
        let strip = from.is_unmerged();
        return Ok(split_nul(&output.stdout)
            .into_iter()
            .map(|name| if strip { strip_stage(&name) } else { name })
            .map(path_of)
            .collect());
    }
    let before = tree_stage_entries(repo, from, None, stop)?;
    let after = tree_stage_entries(repo, to, None, stop)?;
    let mut changed = BTreeSet::new();
    for (path, entries) in &before {
        if after.get(path) != Some(entries) {
            changed.insert(path.clone());
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            changed.insert(path.clone());
        }
    }
    Ok(changed)
}

/// Join the trees into one backup commit, without touching any ref.
///
/// `take` uses it for the before-snapshot; the protocol uses it again for the
/// after-snapshot, whose parents are the before-commit and every commit the
/// entry's after-fields name.
///
/// Either side may be absent: an after-state read that failed for one should
/// still keep the other reachable. A commit over neither is still worth
/// making — its parents are what keeps the entry's recorded commits alive.
///
/// The index side lands under `index/` when it is merged and `unmerged/` when
/// it is the stage tree, **never both**. A merged commit's layout is therefore
/// byte-identical to every backup ref written since M3, and the shape is
/// readable from the commit alone rather than only from the journal.
pub fn commit_trees(
    repo: &RepoHandle,
    index: Option<IndexState>,
    worktree_tree: Option<Oid>,
    id: &EntryId,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let joined = mktree(repo, index, worktree_tree, stop)?;
    commit_tree(repo, joined, parents, &format!("graft {id} {label}"), stop)
}

/// Point an existing backup ref at another commit (the after-snapshot).
pub fn move_backup_ref(
    repo: &RepoHandle,
    name: &str,
    to: Oid,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    update_ref(repo, name, to, stop)
}

/// A copy of the real index, removed when it goes out of scope on **every**
/// path — a leaked temp index would be re-copied forever and is invisible to
/// the user.
struct TempIndex {
    path: PathBuf,
}

impl TempIndex {
    /// Copy `<git-dir>/index` into `scratch` under a name no other snapshot
    /// in this process or any other can take.
    ///
    /// A missing index is not an error: a repository with no `.git/index`
    /// yet (a fresh `git init`) has an empty index, and `write-tree` over a
    /// missing `GIT_INDEX_FILE` prints the empty tree — verified on git 2.54.
    fn copied_from(repo: &RepoHandle, scratch: &Path) -> Result<TempIndex, SnapshotError> {
        let path = scratch.join(Self::unique_name("tmp-index"));
        let source = repo.git_dir().join("index");
        match std::fs::copy(&source, &path) {
            Ok(_) => {}
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                // No index file: the empty index. `write-tree` and `add -A`
                // both accept a `GIT_INDEX_FILE` that does not exist yet.
                if path.exists() {
                    std::fs::remove_file(&path)?;
                }
            }
            Err(why) => return Err(why.into()),
        }
        Ok(TempIndex { path })
    }

    /// A name no index file exists at yet: `update-index` creates it. The
    /// stage tree is built on one of these rather than on a copy, so nothing
    /// the real index holds can survive into it.
    fn fresh(scratch: &Path) -> Result<TempIndex, SnapshotError> {
        let path = scratch.join(Self::unique_name("tmp-stage-index"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(TempIndex { path })
    }

    fn unique_name(prefix: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}-{}-{unique}", std::process::id())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        // Nothing can be done about a failed removal here and it must not
        // mask the operation's own result; the next snapshot uses a fresh
        // name, so a survivor is inert scratch, never read again.
        // Said out loud rather than asserted: a panic in `Drop` while a test
        // is already unwinding turns the real failure into an abort with no
        // message (m3/03 review, Minor 4).
        if let Err(why) = std::fs::remove_file(&self.path)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("graft: temp index {} survived: {why}", self.path.display());
        }
    }
}

/// `GIT_INDEX_FILE=<temp> git write-tree`, with an unmerged index an ordinary
/// failure. Used after `add -A`, which resolves every conflict in the copy, so
/// an unmerged index cannot reach it.
fn write_tree(repo: &RepoHandle, index: &Path, stop: &dyn Stop) -> Result<Oid, SnapshotError> {
    let output = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "write-tree")
}

/// The same, with `Ok(None)` exactly when git refused because the index holds
/// unmerged paths — the one failure the caller has another answer for.
fn try_write_tree(
    repo: &RepoHandle,
    index: &Path,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let ran = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop);
    match ran {
        Ok(output) => oid_from(&output.stdout, "write-tree").map(Some),
        // `<path>: unmerged (<oid>)` is git's own first line here, and the
        // only way `write-tree` fails that the caller can work with.
        Err(RunError::Exited {
            stderr_first_line, ..
        }) if stderr_first_line.contains("unmerged") => Ok(None),
        Err(why) => Err(stopped(why)),
    }
}

/// The stage tree of an unmerged index copy (decision 1).
///
/// `ls-files --stage` on the copy, every record rewritten to `<stage>/<path>`
/// at stage 0 on a **second**, empty temporary index, then `write-tree`. The
/// real index is never read or written by either step.
fn stage_tree(
    repo: &RepoHandle,
    index: &Path,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let listing = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("ls-files"))
        .arg(GitArg::Flag("--stage"))
        .arg(GitArg::Flag("-z"))
        .run(stop)
        .map_err(stopped)?;
    let stdin = prefixed_stage_records(&listing.stdout)?;
    let staged = TempIndex::fresh(scratch)?;
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", staged.path())
        .arg(GitArg::Flag("update-index"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--index-info"))
        .stdin(stdin)
        .run(stop)
        .map_err(stopped)?;
    write_tree(repo, staged.path(), stop)
}

/// `<mode> <oid> <stage>\t<path>\0` → `<mode> <oid> 0\t<stage>/<path>\0`.
///
/// Pure, so the bytes fed to `update-index --index-info` are testable without
/// a repository. A path is never parsed as text: it is moved whole, after the
/// one tab that separates it from the fixed-shape head.
fn prefixed_stage_records(listing: &[u8]) -> Result<Vec<u8>, SnapshotError> {
    let mut out = Vec::with_capacity(listing.len() + listing.len() / 8);
    for record in split_nul(listing) {
        let read = split_stage_record(&record, "ls-files --stage")?;
        out.extend_from_slice(read.mode);
        out.push(b' ');
        out.extend_from_slice(read.oid);
        out.extend_from_slice(b" 0\t");
        out.extend_from_slice(read.stage);
        out.push(b'/');
        out.extend_from_slice(read.path);
        out.push(0);
    }
    Ok(out)
}

/// One `ls-files --stage` record, in the bytes git printed: nothing is parsed
/// here, because the only use rewrites the path and hands it straight back.
struct StageRecord<'a> {
    mode: &'a [u8],
    oid: &'a [u8],
    stage: &'a [u8],
    path: &'a [u8],
}

/// `<mode> <oid> <stage>\t<path>` split into its four parts, unparsed.
fn split_stage_record<'a>(
    record: &'a [u8],
    what: &'static str,
) -> Result<StageRecord<'a>, SnapshotError> {
    let unreadable = || SnapshotError::NotAnOid {
        what,
        printed: String::from_utf8_lossy(record).into_owned(),
    };
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or_else(unreadable)?;
    let (head, rest) = record.split_at(tab);
    let mut fields = head.split(|byte| *byte == b' ');
    let mode = fields.next().ok_or_else(unreadable)?;
    let oid = fields.next().ok_or_else(unreadable)?;
    let stage = fields.next().ok_or_else(unreadable)?;
    if fields.next().is_some() || mode.is_empty() || oid.is_empty() || stage.len() != 1 {
        return Err(unreadable());
    }
    Ok(StageRecord {
        mode,
        oid,
        stage,
        path: &rest[1..],
    })
}

/// Every path a recorded index state holds, with its entries, newest read
/// straight out of the tree. `only` narrows it to one path.
fn tree_stage_entries(
    repo: &RepoHandle,
    index: IndexState,
    only: Option<&[PathBuf]>,
    stop: &dyn Stop,
) -> Result<BTreeMap<PathBuf, Vec<StageEntry>>, SnapshotError> {
    if only.is_some_and(<[PathBuf]>::is_empty) {
        return Ok(BTreeMap::new());
    }
    let mut args = vec![
        GitArg::Flag("--literal-pathspecs"),
        GitArg::Flag("ls-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Oid(index.tree()),
    ];
    if let Some(paths) = only {
        args.push(GitArg::Separator);
        for path in paths {
            if index.is_unmerged() {
                for stage in 0..4u8 {
                    args.push(GitArg::Path(stage_prefixed(stage, path)));
                }
            } else {
                args.push(GitArg::Path(path.as_os_str().as_encoded_bytes().to_vec()));
            }
        }
    }
    let output = run_git(repo, args, None, stop)?;
    let mut held: BTreeMap<PathBuf, Vec<StageEntry>> = BTreeMap::new();
    for record in split_nul(&output.stdout) {
        let entry = ls_tree_entry(&record, index.is_unmerged())?;
        held.entry(entry.0).or_default().push(entry.1);
    }
    for entries in held.values_mut() {
        // By stage, explicitly. `StageEntry`'s derived order is mode, then
        // oid, then stage — and three stages of one path share a mode, so a
        // plain sort orders them by blob id, which is to say at random.
        entries.sort_unstable_by_key(|entry| entry.stage);
    }
    Ok(held)
}

/// `<mode> <type> <oid>\t<name>` from `ls-tree -r -z`, as a path and one
/// entry. The stage comes from the name's first component for a stage tree,
/// and is 0 for a plain one.
fn ls_tree_entry(record: &[u8], unmerged: bool) -> Result<(PathBuf, StageEntry), SnapshotError> {
    let unreadable = || SnapshotError::NotAnOid {
        what: "ls-tree",
        printed: String::from_utf8_lossy(record).into_owned(),
    };
    let tab = record
        .iter()
        .position(|byte| *byte == b'\t')
        .ok_or_else(unreadable)?;
    let (head, rest) = record.split_at(tab);
    let mut fields = head.split(|byte| *byte == b' ');
    let mode = fields.next().ok_or_else(unreadable)?;
    let _kind = fields.next().ok_or_else(unreadable)?;
    let oid = fields.next().ok_or_else(unreadable)?;
    let name = &rest[1..];
    let mode = std::str::from_utf8(mode)
        .ok()
        .and_then(|text| u32::from_str_radix(text, 8).ok())
        .ok_or_else(unreadable)?;
    let oid = std::str::from_utf8(oid)
        .ok()
        .and_then(|text| Oid::from_hex(text).ok())
        .ok_or_else(unreadable)?;
    let (stage, path) = if unmerged {
        let digit = name.first().copied().ok_or_else(unreadable)?;
        if !digit.is_ascii_digit() || name.get(1) != Some(&b'/') {
            return Err(unreadable());
        }
        (digit - b'0', strip_stage(name))
    } else {
        (0, name.to_vec())
    };
    if stage > 3 {
        return Err(unreadable());
    }
    Ok((path_of(path), StageEntry { mode, oid, stage }))
}

/// `<stage>/<path>` as the bytes a pathspec takes.
fn stage_prefixed(stage: u8, path: &Path) -> Vec<u8> {
    let mut name = vec![b'0' + stage, b'/'];
    name.extend_from_slice(path.as_os_str().as_encoded_bytes());
    name
}

/// `2/src/a.rs` → `src/a.rs`. Anything without a prefix is left alone: a
/// caller that asked for a stage tree and got a plain name has a bigger
/// problem than a mangled path, and mangling it would hide it.
fn strip_stage(name: &[u8]) -> Vec<u8> {
    match name.get(1) {
        Some(b'/') if name[0].is_ascii_digit() => name[2..].to_vec(),
        _ => name.to_vec(),
    }
}

fn path_of(name: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(name))
}

/// NUL-terminated records, the trailing empty one dropped.
fn split_nul(bytes: &[u8]) -> Vec<Vec<u8>> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// One `git` with no index redirection, for the reads that go against an
/// object id rather than an index.
fn run_git(
    repo: &RepoHandle,
    args: Vec<GitArg>,
    stdin: Option<Vec<u8>>,
    stop: &dyn Stop,
) -> Result<graft_git::run::GitOutput, SnapshotError> {
    let mut invocation = GitInvocation::new(repo).args(args);
    if let Some(bytes) = stdin {
        invocation = invocation.stdin(bytes);
    }
    invocation.run(stop).map_err(stopped)
}

/// A ref's commit, or `None` when it is not there. `for-each-ref` exits 0
/// either way, so an absent ref is an empty answer rather than a failure to
/// tell apart from a broken one.
fn read_ref(repo: &RepoHandle, name: &str, stop: &dyn Stop) -> Result<Option<Oid>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--format=%(objectname)"),
            GitArg::Ref(name.as_bytes().to_vec()),
        ],
        None,
        stop,
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        return Ok(None);
    }
    Oid::from_hex(first)
        .map(Some)
        .map_err(|_| SnapshotError::NotAnOid {
            what: "for-each-ref",
            printed: first.to_owned(),
        })
}

/// `GIT_INDEX_FILE=<temp> git --literal-pathspecs add -A --`: the working
/// tree into the copy. The stat cache the copy carries means only files whose
/// stat data changed are re-hashed.
fn add_all(repo: &RepoHandle, index: &Path, stop: &dyn Stop) -> Result<(), SnapshotError> {
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("--literal-pathspecs"))
        .arg(GitArg::Flag("add"))
        .arg(GitArg::Flag("-A"))
        .run(stop)
        .map_err(stopped)?;
    Ok(())
}

/// `git mktree` over the two-entry listing.
///
/// `mktree` wants its entries sorted, and `index` < `unmerged` < `worktree`
/// either way.
fn mktree(
    repo: &RepoHandle,
    index: Option<IndexState>,
    worktree_tree: Option<Oid>,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut listing = String::new();
    match index {
        Some(IndexState::Merged(tree)) => {
            listing.push_str(&format!("040000 tree {tree}\tindex\n"));
        }
        Some(IndexState::Unmerged { unmerged }) => {
            listing.push_str(&format!("040000 tree {unmerged}\tunmerged\n"));
        }
        None => {}
    }
    if let Some(worktree_tree) = worktree_tree {
        listing.push_str(&format!("040000 tree {worktree_tree}\tworktree\n"));
    }
    let output = GitInvocation::new(repo)
        .arg(GitArg::Flag("mktree"))
        .stdin(listing.into_bytes())
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "mktree")
}

/// `git commit-tree <tree> [-p <parent>]…` with a fixed identity.
///
/// The identity is set explicitly rather than left to the user's config: a
/// backup commit is Graft's, not theirs, and a repository configured with
/// `user.useConfigOnly` would otherwise refuse to make one.
fn commit_tree(
    repo: &RepoHandle,
    tree: Oid,
    parents: &[Oid],
    message: &str,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut invocation = GitInvocation::new(repo)
        .env("GIT_AUTHOR_NAME", "graft")
        .env("GIT_AUTHOR_EMAIL", "graft@backup")
        .env("GIT_COMMITTER_NAME", "graft")
        .env("GIT_COMMITTER_EMAIL", "graft@backup")
        .arg(GitArg::Flag("commit-tree"))
        .arg(GitArg::Oid(tree));
    for parent in parents {
        invocation = invocation.arg(GitArg::Flag("-p")).arg(GitArg::Oid(*parent));
    }
    let output = invocation
        .arg(GitArg::Flag("-F"))
        .arg(GitArg::Word("-".to_owned()))
        .stdin(message.as_bytes().to_vec())
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "commit-tree")
}

/// `git update-ref <name> <commit>`.
fn update_ref(
    repo: &RepoHandle,
    name: &str,
    to: Oid,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    GitInvocation::new(repo)
        .arg(GitArg::Flag("update-ref"))
        .arg(GitArg::Ref(name.as_bytes().to_vec()))
        .arg(GitArg::Oid(to))
        .run(stop)
        .map_err(stopped)?;
    Ok(())
}

/// `RunError::Stopped` is the caller's own cancellation, not a git failure;
/// every other error stays as it is.
fn stopped(why: RunError) -> SnapshotError {
    match why {
        RunError::Stopped => SnapshotError::Stopped,
        other => SnapshotError::Run(other),
    }
}

fn oid_from(stdout: &[u8], what: &'static str) -> Result<Oid, SnapshotError> {
    let text = String::from_utf8_lossy(stdout);
    let first = text.lines().next().unwrap_or("").trim();
    Oid::from_hex(first).map_err(|_| SnapshotError::NotAnOid {
        what,
        printed: first.to_owned(),
    })
}

#[cfg(test)]
mod tests;
