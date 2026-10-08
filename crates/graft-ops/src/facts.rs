//! The two repository reads the protocol is built on.
//!
//! [`read_facts`] is what an operation's `check`, `plan` and `touches` are
//! answered from: read **once**, before anything is written, so all four
//! properties see one consistent picture (design note, four properties).
//!
//! [`read_state_now`] is §6.7's other half: the repository as it stands at
//! undo time, to compare against what the journal says the operation left
//! behind. It reads the same two trees the snapshot does, because a plain
//! working-tree edit moves no ref and comparing refs alone was never enough
//! (ADR-0009).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::remote::RemoteError;
use graft_git::repo::{HeadError, HeadState, RefTarget, RefsError, RepoHandle};
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::stash::StashError;
use graft_git::state::InProgress;
use graft_git::status::{StatusError, WorkingTreeStatus};
use graft_git::version::GitVersion;

use crate::operation::{FactsScope, Incoming, RepoFacts};
use crate::snapshot::{self, SnapshotError};
use crate::undo::RepoStateNow;

/// Why the repository could not be read.
#[derive(Debug, thiserror::Error)]
pub enum FactsError {
    /// `git status` failed.
    #[error("{0}")]
    Status(#[from] StatusError),
    /// `HEAD` could not be read.
    #[error("{0}")]
    Head(#[from] HeadError),
    /// The ref store could not be read.
    #[error("{0}")]
    Refs(#[from] RefsError),
    /// The stash list could not be read.
    #[error("{0}")]
    Stash(#[from] StashError),
    /// The in-progress check could not read the git dir.
    #[error("in-progress state: {0}")]
    State(#[from] std::io::Error),
    /// A tree could not be written.
    #[error("{0}")]
    Snapshot(#[from] SnapshotError),
    /// The remotes or the head branch's upstream could not be read.
    #[error("remotes: {0}")]
    Remote(#[from] RemoteError),
    /// A ref name that is not UTF-8. The journal records ref names as text,
    /// so one that is not text cannot be recorded — and a restore that
    /// silently mangled a name would aim at the wrong ref.
    #[error("the ref name {0} is not UTF-8; Graft cannot journal it")]
    NonUtf8Ref(String),
    /// An unmerged path's file could not be read for conflict markers.
    #[error("reading {path} for conflict markers: {source}")]
    Markers {
        /// The path, as the status lists it.
        path: PathBuf,
        /// What the read said.
        source: std::io::Error,
    },
    /// An ancestry read (`merge-base --is-ancestor`) failed, so the re-push
    /// guard could not decide whether the commit is on the branch. Refusing
    /// to guess is the point of the guard (ADR-0038 §4).
    #[error("checking the commit is on the branch: {0}")]
    Ancestry(RunError),
    /// git's merge state (`MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE`) could not
    /// be read (m6-conflicts/20).
    #[error("the merge in progress: {0}")]
    MergeState(#[from] crate::merge_state::MergeStateError),
    /// A history read a rewrite plans from — the branch's commits, the
    /// branches built on it, the worktrees they are checked out in — failed
    /// (ADR-0047).
    #[error("reading the branch's history: {0}")]
    History(RunError),
    /// The default branch could not be worked out, so the rewrite's range
    /// has no base to stop at.
    #[error("the default branch: {0}")]
    Branches(#[from] graft_git::branches::BranchError),
    /// A rewrite's saved plan (`graft/rewrite/<entry>/`) could not be read,
    /// written or removed.
    #[error("the rewrite's saved plan: {0}")]
    SavedPlan(std::io::Error),
    /// git's reflog for `HEAD` or a local branch could not be read (the
    /// journal's list, m7-power-tools/22).
    #[error("reading git's reflog: {0}")]
    Reflog(#[from] graft_git::history::ReflogError),
    /// A path could not be looked for on disk: the put-back's check for his
    /// files in the way (review m7-22, C1).
    #[error("reading {path} on disk: {source}")]
    OnDisk {
        /// The path, relative to the working tree.
        path: PathBuf,
        /// What the read said.
        source: std::io::Error,
    },
    /// What a pull's fetch brought could not be read: the ancestry check or
    /// the diff between `HEAD` and the tracking tip (m7-power-tools/03b).
    #[error("reading what the pull brings: {0}")]
    Incoming(RunError),
    /// The ref store could not be listed around a fetch, so what it moved
    /// could not be told (ADR-0062): before it, the fetch does not run;
    /// after it, the entry records nothing moved as unknown.
    #[error("listing the refs a fetch moves: {0}")]
    FetchedRefs(RunError),
    /// A touched branch's `branch.<name>.*` config could not be read, so
    /// the journal could not record what undo must put back (review
    /// m7-16b..21b, Major 1).
    #[error("the branch's config: {0}")]
    BranchConfig(RunError),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

impl FactsError {
    /// A history read's failure: stopped is [`FactsError::Stopped`], never
    /// an error of its own.
    pub(crate) fn of_history(error: RunError) -> Self {
        match error {
            RunError::Stopped => Self::Stopped,
            other => Self::History(other),
        }
    }
}

impl FactsError {
    /// An incoming read's failure: stopped is [`FactsError::Stopped`].
    pub(crate) fn of_incoming(error: RunError) -> Self {
        match error {
            RunError::Stopped => Self::Stopped,
            other => Self::Incoming(other),
        }
    }

    /// One line for the status line.
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

/// Everything `check`, `plan` and `touches` need, read in one pass: the
/// remotes and `HEAD`'s upstream come from config, no spawn.
///
/// `scope` is [`crate::operation::Operation::facts_scope`]: only fetch,
/// pull and push read the remotes and upstream. For every other operation
/// they are left empty and `None`, so a remote section git's config holds
/// but Graft cannot read (a non-UTF-8 name or URL) never blocks a stage, a
/// commit or a stash (m4/04 review, M2). The named refs are read by name,
/// never by a pass over the store; `HEAD` resolves through the branch it is
/// on. Marker lines are looked for only in unmerged paths, so a clean
/// status opens no file (m5/07). The edit-stop fact is read only mid-rebase
/// (m7-power-tools/07a).
pub fn read_facts(
    repo: &RepoHandle,
    git: GitVersion,
    stop: &dyn Stop,
    scope: &FactsScope,
) -> Result<RepoFacts, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let bare = repo.is_bare();
    let head = head_ref(repo)?;
    let in_progress = graft_git::state::in_progress(repo)?;
    // `status::read` already reports a bare repository as empty.
    let status = graft_git::status::read(repo, stop)?;
    let stashes = graft_git::stash::list(repo, stop)?;
    let (remotes, upstream) = if scope.remotes {
        let remotes = graft_git::remote::list(repo)?;
        let upstream = match &head {
            HeadRef::Branch(branch) => graft_git::remote::upstream_of(repo, branch)?,
            HeadRef::Detached(_) | HeadRef::Unborn(_) => None,
        };
        (remotes, upstream)
    } else {
        (Vec::new(), None)
    };
    let mut refs = BTreeMap::new();
    for name in &scope.refs {
        let target = if name == "HEAD" {
            match &head {
                HeadRef::Branch(branch) => resolve(repo, branch)?,
                HeadRef::Detached(id) => Some(*id),
                HeadRef::Unborn(_) => None,
            }
        } else {
            resolve(repo, name)?
        };
        refs.insert(name.clone(), target);
    }
    let markers = markers_in(repo, &status)?;
    // Only a rebase can stop on an `edit` row; outside one, no read at all.
    let edit_stop = match in_progress {
        Some(InProgress::Rebase) => crate::rewrite::edit_stop(repo, stop)?,
        _ => None,
    };
    Ok(RepoFacts {
        bare,
        head,
        in_progress,
        status,
        stashes,
        git,
        remotes,
        upstream,
        refs,
        markers,
        incoming: None,
        edit_stop,
    })
}

/// What a pull's fetch brought (m7-power-tools/03b): whether `head`
/// fast-forwards to `tip` (`merge-base --is-ancestor`), and if it does, the
/// paths between them (`diff --name-only -z --no-renames`). Two spawns at
/// most, the stop asked by each; a diverged branch costs one.
///
/// # Errors
/// [`FactsError::Incoming`] when either read fails; [`FactsError::Stopped`]
/// when the caller stops.
pub(crate) fn read_incoming(
    repo: &RepoHandle,
    head: Oid,
    tip: Oid,
    stop: &dyn Stop,
) -> Result<Incoming, FactsError> {
    let fast_forward =
        crate::preview::is_ancestor(repo, head, tip, stop).map_err(FactsError::of_incoming)?;
    if !fast_forward {
        return Ok(Incoming {
            tip,
            fast_forward,
            paths: Vec::new(),
        });
    }
    let listed = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            GitArg::Flag("--no-renames"),
            GitArg::Oid(head),
            GitArg::Oid(tip),
        ])
        .run(stop)
        .map_err(FactsError::of_incoming)?;
    let paths = listed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(path_of)
        .collect();
    Ok(Incoming {
        tip,
        fast_forward,
        paths,
    })
}

#[cfg(unix)]
fn path_of(name: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(name.to_vec()))
}

#[cfg(not(unix))]
fn path_of(name: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(name).into_owned())
}

/// The unmerged paths whose file still holds a `<<<<<<< ` line. A path the
/// status lists as unmerged but that is gone from the disk (a delete/modify
/// conflict resolved by deleting) has no markers.
fn markers_in(repo: &RepoHandle, status: &WorkingTreeStatus) -> Result<Vec<PathBuf>, FactsError> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };
    let mut marked = Vec::new();
    for entry in status.entries.iter().filter(|entry| entry.unmerged) {
        match graft_git::history_reads::first_marker_line(&workdir.join(&entry.path)) {
            Ok(Some(_)) => marked.push(entry.path.clone()),
            Ok(None) => {}
            // Resolved by deleting the file: nothing to scan.
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(FactsError::Markers {
                    path: entry.path.clone(),
                    source,
                });
            }
        }
    }
    Ok(marked)
}

/// `HEAD` as the journal records it.
pub fn head_ref(repo: &RepoHandle) -> Result<HeadRef, FactsError> {
    Ok(match repo.head()? {
        HeadState::Branch(name) => HeadRef::Branch(ref_text(name.as_bytes())?),
        HeadState::Detached(id) => HeadRef::Detached(id),
        HeadState::Unborn(name) => HeadRef::Unborn(ref_text(name.as_bytes())?),
    })
}

/// The current value of every named ref: `None` for one that is absent.
///
/// Each name is looked up on its own rather than by reading the whole ref
/// store: `begin`, `finish` and [`read_state_now`] each ask about the one or
/// two refs an operation touches, and a full pass per call is three passes
/// over 50k refs per operation (m3/03 review, M6).
pub fn read_refs(
    repo: &RepoHandle,
    names: &[String],
) -> Result<BTreeMap<String, Option<Oid>>, FactsError> {
    let mut found = BTreeMap::new();
    for name in names {
        found.insert(name.clone(), resolve(repo, name)?);
    }
    Ok(found)
}

/// The repository as it stands now: §6.7's three comparisons, read fresh.
///
/// An unmerged index — what a `stash pop` that hit a conflict leaves — has a
/// state of its own ([`graft_core::journal::IndexState::Unmerged`]), never an
/// absence and never an error: `drift` has to be able to compare it against
/// what an entry recorded, and "no tree" used to make every such comparison
/// read as a change (m6-conflicts/08).
pub fn read_state_now(
    repo: &RepoHandle,
    scratch: &Path,
    refs: &[String],
    stop: &dyn Stop,
) -> Result<RepoStateNow, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let head = head_ref(repo)?;
    let named = read_refs(repo, refs)?;
    let index = snapshot::index_state(repo, scratch, stop)?;
    let worktree_tree = snapshot::worktree_tree(repo, scratch, stop)?;
    let merge = crate::merge_state::read_merge_state(repo)?;
    Ok(RepoStateNow {
        head,
        refs: named,
        index,
        worktree_tree,
        merge,
    })
}

/// Follow a symbolic ref to the object it names, at most a few hops.
fn resolve(repo: &RepoHandle, name: &str) -> Result<Option<Oid>, FactsError> {
    let mut at = RefName::new(name.as_bytes().to_vec())
        .map_err(|_why| FactsError::NonUtf8Ref(name.to_owned()))?;
    for _ in 0..5 {
        match repo.reference(&at)? {
            None => return Ok(None),
            Some(RefTarget::Object(id)) => return Ok(Some(id)),
            Some(RefTarget::Symbolic(next)) => {
                // The hop is checked for text here, not silently dropped: a
                // name the journal cannot record is one a restore would aim
                // wrongly with.
                ref_text(next.as_bytes())?;
                at = next;
            }
        }
    }
    Ok(None)
}

fn ref_text(name: &[u8]) -> Result<String, FactsError> {
    std::str::from_utf8(name)
        .map(str::to_owned)
        .map_err(|_| FactsError::NonUtf8Ref(String::from_utf8_lossy(name).into_owned()))
}
