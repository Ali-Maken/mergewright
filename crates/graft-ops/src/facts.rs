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
use std::path::Path;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::repo::{HeadError, HeadState, RefTarget, RefsError, RepoHandle};
use graft_git::run::Stop;
use graft_git::stash::StashError;
use graft_git::status::StatusError;
use graft_git::version::GitVersion;

use crate::operation::RepoFacts;
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
    /// A ref name that is not UTF-8. The journal records ref names as text,
    /// so one that is not text cannot be recorded — and a restore that
    /// silently mangled a name would aim at the wrong ref.
    #[error("the ref name {0} is not UTF-8; Graft cannot journal it")]
    NonUtf8Ref(String),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

impl FactsError {
    /// One line for the status line.
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

/// Everything `check`, `plan` and `touches` need, read in one pass.
pub fn read_facts(
    repo: &RepoHandle,
    git: GitVersion,
    stop: &dyn Stop,
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
    Ok(RepoFacts {
        bare,
        head,
        in_progress,
        status,
        stashes,
        git,
    })
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
pub fn read_refs(
    repo: &RepoHandle,
    names: &[String],
) -> Result<BTreeMap<String, Option<Oid>>, FactsError> {
    if names.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut store: BTreeMap<String, RefTarget> = BTreeMap::new();
    for entry in repo.references()? {
        store.insert(ref_text(entry.name.as_bytes())?, entry.target);
    }
    let mut found = BTreeMap::new();
    for name in names {
        found.insert(name.clone(), resolve(&store, name));
    }
    Ok(found)
}

/// The repository as it stands now: §6.7's three comparisons, read fresh.
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
    let index_tree = snapshot::index_tree(repo, scratch, stop)?;
    let worktree_tree = snapshot::worktree_tree(repo, scratch, stop)?;
    Ok(RepoStateNow {
        head,
        refs: named,
        index_tree,
        worktree_tree,
    })
}

/// Follow a symbolic ref to the object it names, at most a few hops.
fn resolve<'a>(store: &'a BTreeMap<String, RefTarget>, name: &'a str) -> Option<Oid> {
    let mut at = name;
    for _ in 0..5 {
        match store.get(at)? {
            RefTarget::Object(id) => return Some(*id),
            RefTarget::Symbolic(next) => {
                at = std::str::from_utf8(next.as_bytes()).ok()?;
            }
        }
    }
    None
}

fn ref_text(name: &[u8]) -> Result<String, FactsError> {
    std::str::from_utf8(name)
        .map(str::to_owned)
        .map_err(|_| FactsError::NonUtf8Ref(String::from_utf8_lossy(name).into_owned()))
}
