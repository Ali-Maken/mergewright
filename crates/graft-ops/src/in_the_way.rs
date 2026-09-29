//! The one read of "his files in a write's way" (ADR-0049): paths on disk
//! that git doesn't keep — untracked or ignored — where an operation would
//! write one of its own.
//!
//! Decided from the disk, never from `git status` alone: status never lists
//! an **ignored** file, yet `switch`, `merge`, `reset --hard` and the rest
//! write over one with rc 0, and the backup (`add -A`) can't hold it, so
//! `u` could not bring it back (review m7-22, C1; scout 28's probe). The
//! answer is to refuse: [`crate::preview::plan`] and `apply` both ask
//! [`blocking`] before anything is written, so a refusal leaves no journal
//! line.
//!
//! Three parts: this file (the read and its words), [`written`] and
//! [`replay`] (the paths each kind writes, ADR-0049 §4's table) and
//! [`disk`] (which of those his bytes stand on).
//!
//! Spawns, whatever the size of the change: one for the paths the
//! operation writes (a few more for a stash: its parents and each of its
//! trees; for a replay, a `rev-list` of the replayed commits and one
//! `diff-tree --stdin` over all of them), one `ls-files` for the index.
//! Never one per path or per commit.

mod disk;
mod reads;
mod replay;
mod written;

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_git::repo::RepoHandle;
use graft_git::run::Stop;

use crate::facts::FactsError;
use crate::operation::{
    CheckoutTarget, Operation, RecoverParams, RecoverTarget, Refusal, RepoFacts, ResetMode,
    ResetTarget, Source, short, short_ref,
};
use crate::snapshot::tracked_paths_now;

/// Paths on disk that git doesn't keep and that the operation would write
/// over. Each list is sorted and unique.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blocking {
    /// Not listed by `git status`: a `.gitignore` or `info/exclude` rule
    /// hides it. Neither `S` nor `c` takes one; he moves it aside.
    pub ignored: Vec<PathBuf>,
    /// Listed by `git status` as untracked: `S` (`stash push -u`) takes it.
    pub untracked: Vec<PathBuf>,
}

impl Blocking {
    /// Nothing in the way.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ignored.is_empty() && self.untracked.is_empty()
    }
}

/// The one read. Empty for a kind ADR-0049 §2 doesn't cover, and in a
/// repository with no working tree.
///
/// # Errors
/// [`FactsError`]: a `git` read failed, a path could not be looked for on
/// disk ([`FactsError::OnDisk`]), or the caller stopped
/// ([`FactsError::Stopped`]).
pub fn blocking(
    repo: &RepoHandle,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Blocking, FactsError> {
    if repo.workdir().is_none() {
        // A bare repository has no files to write over.
        return Ok(Blocking::default());
    }
    let Some(written) = written::written_paths(repo, op, facts, stop)? else {
        return Ok(Blocking::default());
    };
    if written.is_empty() {
        return Ok(Blocking::default());
    }
    let tracked = tracked_paths_now(repo, stop, &mut Vec::new()).map_err(reads::snapshot_error)?;
    // A tracked path is the set-aside's or git's own check's (ADR-0045 §4),
    // never his file in the way.
    let candidates: BTreeSet<PathBuf> = written
        .into_iter()
        .filter(|path| !tracked.contains(path))
        .collect();
    let found = disk::on_disk(repo, &candidates, &tracked)?;
    Ok(disk::sort_out(found, facts))
}

/// `Some(InTheWay { .. })` when not empty, `None` when empty. Stash kinds
/// never call this; they build [`Refusal::StashInTheWay`].
#[must_use]
pub fn refusal(op: &Operation, blocking: Blocking) -> Option<Refusal> {
    if blocking.is_empty() {
        return None;
    }
    Some(Refusal::InTheWay {
        // Every kind `written_paths` answers for has its words; a kind
        // without them never gets here with anything in the way.
        doing: doing(op).unwrap_or_else(|| op.describe()),
        ignored: blocking.ignored,
        untracked: blocking.untracked,
    })
}

/// The refusal `preview::plan` and `apply` both give: a stash put-back's in
/// the stash preview's own words (m6-conflicts/86), every other kind's
/// through [`refusal`].
pub(crate) fn refused(op: &Operation, blocking: Blocking) -> Option<Refusal> {
    match op {
        Operation::StashApply { .. } | Operation::StashPop { .. } => {
            (!blocking.is_empty()).then(|| Refusal::StashInTheWay {
                unsaved: Vec::new(),
                staged: Vec::new(),
                untracked: blocking.untracked,
                ignored: blocking.ignored,
            })
        }
        // A file put-back writes over his untracked files at its paths:
        // its entry's before-snapshot (`add -A`) holds them, so `u` brings
        // them back and the preview names them as replaced (m7/30o R1).
        // Only an ignored one, which the snapshot skips, is refused.
        Operation::Recover(RecoverParams {
            target: RecoverTarget::Files { .. },
            ..
        }) => refusal(
            op,
            Blocking {
                ignored: blocking.ignored,
                untracked: Vec::new(),
            },
        ),
        _ => refusal(op, blocking),
    }
}

/// `<doing>` in `<path> is in the way of <doing>`. `None` for a kind that
/// isn't covered.
#[must_use]
pub fn doing(op: &Operation) -> Option<String> {
    let words = match op {
        Operation::Checkout(params) => match &params.to {
            CheckoutTarget::Branch(name) => format!("switching to {}", short_ref(name)),
            CheckoutTarget::Detached(oid) => format!("switching to {}", short(*oid)),
            CheckoutTarget::Track { local, .. } => format!("switching to {local}"),
        },
        Operation::BranchCreate(params) if params.checkout => {
            format!("switching to {}", params.name)
        }
        Operation::BranchFromStash(params) => format!("switching to {}", params.name),
        Operation::Merge(params) => format!("merging {}", source_words(&params.source)),
        Operation::Pull(_) => format!("pulling {}", op.remote_names()?.remote_side),
        Operation::Reset(params) => match &params.target {
            ResetTarget::Head {
                branch,
                mode: ResetMode::Hard,
            } => format!(
                "resetting {} to {}",
                short_ref(&branch.name),
                short(params.to)
            ),
            _ => return None,
        },
        Operation::Recover(params) => match &params.target {
            RecoverTarget::MoveRef { name, to, .. } => {
                format!("putting {name} back to {}", short(*to))
            }
            RecoverTarget::Files { .. } => "putting back your edit".to_owned(),
            RecoverTarget::Recreate { .. } => return None,
        },
        Operation::Rebase(params) => format!("rebasing onto {}", source_words(&params.onto)),
        Operation::Rewrite(params) => format!("rewriting {}", short_ref(&params.head.name)),
        Operation::CherryPick(params) => match params.commits.as_slice() {
            [one] => format!("picking {}", short(*one)),
            several => format!("picking {} commits", several.len()),
        },
        Operation::Revert(params) => format!("reverting {}", short(params.commit)),
        _ => return None,
    };
    Some(words)
}

fn source_words(source: &Source) -> String {
    match source {
        Source::Branch(name) => short_ref(name).to_owned(),
        Source::Commit(oid) => short(*oid),
    }
}
