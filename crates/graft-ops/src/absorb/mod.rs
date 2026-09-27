//! Absorb (M7 note "Pass 2a" R2; git-absorb, jj): each staged hunk goes to
//! the commit on his branch that last changed its lines, and a hunk that
//! fits none stays staged, with the reason in words.
//!
//! How a hunk is placed, against `base..HEAD`:
//! - its old lines are blamed at `HEAD`, restricted to the range
//!   (`git blame --porcelain -L … <base>..<HEAD>`); every one blamed to one
//!   commit C in the range → C;
//! - a pure insertion looks at the line above and the line below instead:
//!   both must be C's;
//! - lines blamed only to commits below the range → [`StaysWhy::BeforeYourBranch`];
//!   to two or more commits → [`StaysWhy::TwoCommitsChangedIt`]; a new file →
//!   [`StaysWhy::NewFile`]; either side of a staged rename →
//!   [`StaysWhy::Renamed`]; a submodule bump → [`StaysWhy::Submodule`];
//!   anything else (a binary or mode-only change, an
//!   insertion at either end of a file) → [`StaysWhy::NoCommitChangedIt`];
//! - a hunk placed in C must also apply on C's own tree (the commutation
//!   check): `git apply --cached --check` on a temporary index. It fails →
//!   the hunk stays, [`StaysWhy::TwoCommitsChangedIt`] — a later commit
//!   changed the lines next to it.
//!
//! Each hunk is carried as a patch with **one line of `HEAD`'s content on
//! either side** as context, not as the bare `-U0` hunk. Why: git's replay
//! is a three-way merge, and a merge conflicts on a change next to another
//! one. The context line makes the commutation check fail exactly there (a
//! later commit changed the neighbouring line, so C's tree holds a different
//! one), where a bare `-U0` hunk would apply anywhere its own lines match —
//! and git places a context-free insertion by line number alone.
//!
//! Nothing here writes a ref, the index or a file. Placing hunks into
//! commits ([`commit_pieces`]) writes objects only, for `rewrite::plan`.

use std::ops::Range;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::diff::FileStatus;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitInvocation, RunError, Stop};
use graft_git::version::GitVersion;

use crate::facts::FactsError;
use crate::operation::Tip;
use crate::snapshot::{TempIndex, path_of};

mod build;
mod judge;
mod read;

use build::{bare, empty, staged_hunk, with_context};
pub(crate) use build::{commit_pieces, empties};
use judge::{commutes, verdict};
use read::{FileAtHead, blame, commits_in, gitlinks, staged};

/// One staged hunk: its file and its two sides, as git's `-U0` header says
/// them — `start..start + count`. An empty side sits at the line it follows
/// (`@@ -2,0 +3 @@` is `old: 2..2, new: 3..4`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedHunk {
    /// The file, relative to the working tree.
    pub path: PathBuf,
    /// Its lines in `HEAD`'s file.
    pub old: Range<u32>,
    /// Its lines in the staged file.
    pub new: Range<u32>,
}

/// Why a staged hunk stays staged: the words the preview shows after
/// `stays staged ·`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaysWhy {
    /// No commit on his branch changed these lines (a binary or mode-only
    /// change; an insertion at the top or bottom of a file).
    NoCommitChangedIt,
    /// Two or more commits changed these lines, or a later commit changed
    /// the lines next to them.
    TwoCommitsChangedIt,
    /// A new file: no commit has it.
    NewFile,
    /// The lines are older than his branch.
    BeforeYourBranch,
    /// A staged rename (`-M`; copies are never detected, so never this):
    /// both sides stay, so no commit loses the old path while the new one
    /// waits in the index (review m7-06 M2).
    Renamed,
    /// A submodule's commit changed (a gitlink, mode `160000`): absorb
    /// moves lines, and a submodule bump has none (review m7-06 M4).
    Submodule,
    /// On a branch with no base of its own (the default branch, or the only
    /// one, with no upstream), the lines are older than his unpushed
    /// commits: pushed, or more than 10 back, or below a merge (06c
    /// ruling). Where [`StaysWhy::BeforeYourBranch`] would say "older than
    /// your branch", which there has no answer.
    OutsideYourCommits,
}

/// Where each staged hunk goes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsorbMap {
    /// Each hunk that is placed, with the commit that gains it, in the
    /// staged diff's order.
    pub placed: Vec<(StagedHunk, Oid)>,
    /// Each hunk that stays staged, and why. Every one is still staged
    /// after the absorb, the ones only the index holds included: the
    /// set-aside puts the edits back **with the index** onto the rewritten
    /// `HEAD` (`RewriteParams::index_base`, 06b). ADR-0045's "comes back
    /// unstaged" does not hold for absorb.
    pub stays: Vec<(StagedHunk, StaysWhy)>,
}

/// Where each staged hunk goes, in `base..HEAD` (`head` is `HEAD`'s branch
/// and tip). Reads only.
///
/// # Errors
/// [`FactsError`] when a read fails; [`FactsError::Stopped`] when `stop`
/// asks.
pub fn absorb_map(
    repo: &RepoHandle,
    _git: &GitVersion,
    head: &Tip,
    base: Oid,
    stop: &dyn Stop,
) -> Result<AbsorbMap, FactsError> {
    Ok(mapped(repo, head.oid, base, StaysWhy::BeforeYourBranch, stop)?.into_map())
}

/// A staged hunk and the one-hunk patch that puts it in (with a line of
/// `HEAD`'s content either side, see the module doc).
pub(crate) struct Piece {
    pub(crate) hunk: StagedHunk,
    patch: Vec<u8>,
}

/// [`AbsorbMap`] with each hunk's patch kept, for `rewrite::plan`.
pub(crate) struct Mapped {
    pub(crate) placed: Vec<(Piece, Oid)>,
    pub(crate) stays: Vec<(Piece, StaysWhy)>,
}

impl Mapped {
    /// The map without the patches.
    pub(crate) fn into_map(self) -> AbsorbMap {
        AbsorbMap {
            placed: self
                .placed
                .into_iter()
                .map(|(piece, target)| (piece.hunk, target))
                .collect(),
            stays: self
                .stays
                .into_iter()
                .map(|(piece, why)| (piece.hunk, why))
                .collect(),
        }
    }
}

/// [`absorb_map`], with the patches. `below` is the reason a hunk whose
/// lines are all older than `base..head` stays with.
pub(crate) fn mapped(
    repo: &RepoHandle,
    head: Oid,
    base: Oid,
    below: StaysWhy,
    stop: &dyn Stop,
) -> Result<Mapped, FactsError> {
    if stop.should_stop() {
        return Err(FactsError::Stopped);
    }
    let range = commits_in(repo, base, head, stop)?;
    let mut placed = Vec::new();
    let mut stays = Vec::new();
    let submodules = gitlinks(repo, head, stop)?;
    for diff in staged(repo, head, stop)? {
        let path = path_of(diff.entry.path.clone());
        // No copy arm: `diff-index -M` reports renames only (`-C` would be
        // needed for copies, and plumbing ignores `diff.renames`).
        if let FileStatus::Renamed { .. } = diff.entry.status {
            // Both sides stay: placing the old side's deletion would take
            // the file out of history while the new one waits in the index.
            if let Some(old) = &diff.entry.old_path {
                stays.push((empty(&path_of(old.clone())), StaysWhy::Renamed));
            }
            for hunk in &diff.hunks {
                stays.push((bare(&path, hunk), StaysWhy::Renamed));
            }
            if diff.hunks.is_empty() {
                stays.push((empty(&path), StaysWhy::Renamed));
            }
            continue;
        }
        if submodules.contains(&diff.entry.path) {
            stays.push((empty(&path), StaysWhy::Submodule));
            continue;
        }
        if diff.entry.status == FileStatus::Added {
            for hunk in &diff.hunks {
                stays.push((bare(&path, hunk), StaysWhy::NewFile));
            }
            if diff.hunks.is_empty() {
                stays.push((empty(&path), StaysWhy::NewFile));
            }
            continue;
        }
        if diff.binary.is_some() || diff.hunks.is_empty() {
            stays.push((empty(&path), StaysWhy::NoCommitChangedIt));
            continue;
        }
        let file = FileAtHead::read(repo, head, &diff.entry.path, stop)?;
        let blamed = blame(repo, base, head, &diff, file.lines.len(), stop)?;
        for hunk in &diff.hunks {
            let piece = Piece {
                hunk: staged_hunk(&path, hunk),
                patch: with_context(&diff, hunk, &file)?,
            };
            match verdict(hunk, file.lines.len(), &blamed, &range, below)? {
                Ok(target) if commutes(repo, target, &piece.patch, stop)? => {
                    placed.push((piece, target));
                }
                Ok(_) => stays.push((piece, StaysWhy::TwoCommitsChangedIt)),
                Err(why) => stays.push((piece, why)),
            }
        }
    }
    Ok(Mapped { placed, stays })
}

fn with_index(repo: &RepoHandle, index: &TempIndex) -> GitInvocation {
    GitInvocation::new(repo).env("GIT_INDEX_FILE", index.path())
}

fn parse_oid(hex: &str) -> Result<Oid, FactsError> {
    Oid::from_hex(hex).map_err(|_| unparsable(format!("{hex:?} is not a commit id")))
}

fn unparsable(what: String) -> FactsError {
    FactsError::History(RunError::Unparsable(what))
}
