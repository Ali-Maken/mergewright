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
use graft_core::ref_name::RefName;
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
/// An unmerged index — what a `stash pop` that hit a conflict leaves — has no
/// tree at all, and that is recorded as `index_tree: None` rather than failing
/// the read: `HEAD`, the named refs and the working tree are all still
/// readable, and `drift` has to be able to report them (m3/03 review, M5).
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
    let index_tree = match snapshot::index_tree(repo, scratch, stop) {
        Ok(tree) => Some(tree),
        Err(SnapshotError::Unmerged) => None,
        Err(why) => return Err(why.into()),
    };
    let worktree_tree = snapshot::worktree_tree(repo, scratch, stop)?;
    Ok(RepoStateNow {
        head,
        refs: named,
        index_tree,
        worktree_tree,
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
