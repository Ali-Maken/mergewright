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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use graft_core::journal::{EntryId, Touches};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

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
    /// The staged state — `None` when the index is unmerged, which has no
    /// tree at all (`git write-tree` refuses one). The working tree is still
    /// captured, so no file content is lost; only the stage-2/3 entries of a
    /// half-finished merge are, and they are already recorded in the entry
    /// this snapshot belongs to as "unknown".
    pub index_tree: Option<Oid>,
    /// Tracked working-tree content plus untracked files.
    pub worktree_tree: Oid,
    /// `{ index/ → index_tree, worktree/ → worktree_tree }`, committed.
    pub commit: Oid,
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
    /// `git` printed something that is not an object id.
    #[error("git printed {what} where an object id belongs: {printed}")]
    NotAnOid {
        /// Which command.
        what: &'static str,
        /// What it printed, first line, lossy.
        printed: String,
    },
    /// The index holds unmerged paths, which have no tree.
    #[error("the index has unmerged paths, which cannot be written as a tree")]
    Unmerged,
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

/// The staged state as a tree, read without touching the real index.
pub fn index_tree(
    repo: &RepoHandle,
    scratch: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let temp = TempIndex::copied_from(repo, scratch)?;
    write_tree(repo, temp.path(), stop)
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
    let temp = TempIndex::copied_from(repo, scratch)?;
    // An unmerged index has no tree, and that must not stop the backup: the
    // working tree is where the user's work is, and `add -A` over the copy
    // resolves the conflict there (verified, git 2.54).
    let index_tree = match write_tree(repo, temp.path(), stop) {
        Ok(tree) => Some(tree),
        Err(SnapshotError::Unmerged) => None,
        Err(why) => return Err(why),
    };
    add_all(repo, temp.path(), stop)?;
    let worktree_tree = write_tree(repo, temp.path(), stop)?;
    drop(temp);

    let commit = commit_trees(repo, index_tree, worktree_tree, id, label, parents, stop)?;
    let name = backup_ref_name(worktree, id);
    update_ref(repo, &name, commit, stop)?;
    Ok(Snapshot {
        index_tree,
        worktree_tree,
        commit,
    })
}

/// Join two trees into one backup commit, without touching any ref.
///
/// `take` uses it for the before-snapshot; the protocol uses it again for the
/// after-snapshot, whose parent is the before-commit.
pub fn commit_trees(
    repo: &RepoHandle,
    index_tree: Option<Oid>,
    worktree_tree: Oid,
    id: &EntryId,
    label: &str,
    parents: &[Oid],
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let joined = mktree(repo, index_tree, worktree_tree, stop)?;
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
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = scratch.join(format!("tmp-index-{}-{unique}", std::process::id()));
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

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        // Nothing can be done about a failed removal here and it must not
        // mask the operation's own result; the next snapshot uses a fresh
        // name, so a survivor is inert scratch, never read again.
        if let Err(why) = std::fs::remove_file(&self.path)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            debug_assert!(false, "temp index {} survived: {why}", self.path.display());
        }
    }
}

/// `GIT_INDEX_FILE=<temp> git write-tree`.
fn write_tree(repo: &RepoHandle, index: &Path, stop: &dyn Stop) -> Result<Oid, SnapshotError> {
    let output = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop)
        .map_err(|why| match &why {
            // `<path>: unmerged (<oid>)` is git's own first line here, and
            // the only way `write-tree` fails that the caller can work with.
            RunError::Exited {
                stderr_first_line, ..
            } if stderr_first_line.contains("unmerged") => SnapshotError::Unmerged,
            _other => stopped(why),
        })?;
    oid_from(&output.stdout, "write-tree")
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
fn mktree(
    repo: &RepoHandle,
    index_tree: Option<Oid>,
    worktree_tree: Oid,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut listing = String::new();
    if let Some(index_tree) = index_tree {
        listing.push_str(&format!("040000 tree {index_tree}\tindex\n"));
    }
    listing.push_str(&format!("040000 tree {worktree_tree}\tworktree\n"));
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
