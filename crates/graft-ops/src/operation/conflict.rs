//! The two conflict operations: a whole file resolved to one side, and a
//! file put back the way git left it.
//!
//! These are the shapes a block take cannot reach. `UD`/`DU`/`AU`/`UA`/`DD`
//! have no markers at all, and a binary `UU`/`AA` has nothing a text take
//! could apply to, so their resolution *is* a git write — `checkout --ours`,
//! `add`, `rm` — and a git write is an [`Operation`], not an fs write
//! (m6-conflicts design note, hard choice 3).
//!
//! # The table, run against real git
//!
//! Verified on git 2.54.0 (Apple Git-157), 2026-09-22, in a throwaway
//! repository stopped on a merge with one path per `UnmergedHow`. Every row
//! exits 0 and leaves `git ls-files -u -- <path>` empty:
//!
//! | how | side | git | index after | working tree after |
//! |---|---|---|---|---|
//! | `BothModified` (UU) | ours | `checkout --ours` + `add` | stage 2's blob | stage 2's bytes |
//! | `BothModified` (UU) | theirs | `checkout --theirs` + `add` | stage 3's blob | stage 3's bytes |
//! | `BothAdded` (AA) | ours | `checkout --ours` + `add` | stage 2's blob | stage 2's bytes |
//! | `BothAdded` (AA) | theirs | `checkout --theirs` + `add` | stage 3's blob | stage 3's bytes |
//! | `DeletedByThem` (UD) | ours | `add` | stage 2's blob | kept |
//! | `DeletedByThem` (UD) | theirs | `rm --quiet` | gone | removed |
//! | `DeletedByUs` (DU) | theirs | `add` | stage 3's blob | kept — git already left theirs' bytes there |
//! | `DeletedByUs` (DU) | ours | `rm --quiet` | gone | removed |
//! | `AddedByUs` (AU) | ours | `add` | stage 2's blob | kept |
//! | `AddedByUs` (AU) | theirs | `rm --quiet` | gone | removed |
//! | `AddedByThem` (UA) | theirs | `add` | stage 3's blob | kept |
//! | `AddedByThem` (UA) | ours | `rm --quiet` | gone | removed |
//! | `BothDeleted` (DD) | either | `rm --quiet` | gone | already absent |
//!
//! Two of those were open questions the group spec marked "confirm", and
//! both came back the way it guessed: git does leave theirs' bytes in the
//! working tree for a `DeletedByUs` path, so `add` alone stages them; and
//! `rm --quiet` on a `BothDeleted` path exits 0 although there is no file on
//! disk to remove, because the index entry is what it takes away. `rm` needs
//! no `-f` on any of these: an unmerged entry is not "staged content" as far
//! as `git rm`'s safety check is concerned.

use std::path::{Path, PathBuf};

use graft_git::run::GitArg;
use graft_git::status::UnmergedHow;

use super::stage::with_paths;
use super::{Invocation, Refusal, RepoFacts};
use crate::snapshot::StageEntry;

/// Which side of a conflict a whole-file resolution keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The side the branch being merged *into* has: stage 2.
    Ours,
    /// The side being merged *in*: stage 3.
    Theirs,
}

impl Side {
    /// `ours` / `theirs`, for the journal line and the refusal text.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Ours => "ours",
            Self::Theirs => "theirs",
        }
    }

    /// The `git checkout` flag that writes this side into the working tree.
    fn checkout_flag(self) -> GitArg {
        match self {
            Self::Ours => GitArg::Flag("--ours"),
            Self::Theirs => GitArg::Flag("--theirs"),
        }
    }
}

/// One conflicted path resolved whole, to one side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolveWholeParams {
    /// The path, as the status lists it.
    pub path: PathBuf,
    /// The side to keep.
    pub side: Side,
    /// The shape the status read, which decides what git is asked to do.
    pub how: UnmergedHow,
}

/// One conflicted path put back the way git left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartParams {
    /// The path, as the status lists it.
    pub path: PathBuf,
}

/// What a whole-file resolution does to the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Keep {
    /// The side's bytes are already in the working tree: stage them.
    Add,
    /// The side does not have the file: take it out of the index and the
    /// working tree.
    Remove,
    /// Both sides have a version and the working tree holds neither (it
    /// holds markers, or binary stage-2 bytes): write the side out first,
    /// then stage it.
    CheckoutThenAdd,
}

/// The table above, as one function.
fn keep(how: UnmergedHow, side: Side) -> Keep {
    match (how, side) {
        // Both sides have a version: git wrote markers (or, for a binary
        // file, ours' bytes), so the side has to be written out first.
        (UnmergedHow::BothModified | UnmergedHow::BothAdded, _) => Keep::CheckoutThenAdd,
        // The side that has the file: its bytes are already on disk.
        (UnmergedHow::DeletedByThem | UnmergedHow::AddedByUs, Side::Ours)
        | (UnmergedHow::DeletedByUs | UnmergedHow::AddedByThem, Side::Theirs) => Keep::Add,
        // The side that has not got it — and `BothDeleted`, where neither
        // side has, so either answer removes it.
        (UnmergedHow::BothDeleted, _)
        | (UnmergedHow::DeletedByThem | UnmergedHow::AddedByUs, Side::Theirs)
        | (UnmergedHow::DeletedByUs | UnmergedHow::AddedByThem, Side::Ours) => Keep::Remove,
    }
}

/// `resolved src/a.rs (ours)`; a `BothDeleted` path has no side to name.
pub(super) fn describe_resolve(params: &ResolveWholeParams) -> String {
    if params.how == UnmergedHow::BothDeleted {
        format!("resolved {} (deleted)", params.path.display())
    } else {
        format!(
            "resolved {} ({})",
            params.path.display(),
            params.side.word()
        )
    }
}

/// `restarted src/a.rs`.
pub(super) fn describe_restart(params: &RestartParams) -> String {
    format!("restarted {}", params.path.display())
}

/// Both operations need the path to be unmerged *now*: the resolver is only
/// ever open on one that is, and running either on a path git has no stages
/// for at all is what `checkout --merge`'s "did not match any file(s) known
/// to git" is (verified, git 2.54).
pub(super) fn check_unmerged(path: &Path, facts: &RepoFacts) -> Result<UnmergedHow, Refusal> {
    match facts
        .status
        .entry(path)
        .and_then(|entry| entry.unmerged_how)
    {
        Some(how) => Ok(how),
        None => Err(Refusal::NotUnmerged {
            path: path.to_path_buf(),
        }),
    }
}

/// `check` for a whole-file resolution: the path is unmerged, and the shape
/// the caller passed is the shape the status reads now.
pub(super) fn check_resolve(params: &ResolveWholeParams, facts: &RepoFacts) -> Result<(), Refusal> {
    let how = check_unmerged(&params.path, facts)?;
    if how == params.how {
        Ok(())
    } else {
        // The status moved on under the box the user answered: the side they
        // chose may not even exist in the shape it is now.
        Err(Refusal::NotUnmerged {
            path: params.path.clone(),
        })
    }
}

pub(super) fn plan_resolve(params: &ResolveWholeParams) -> Vec<Invocation> {
    let paths = std::slice::from_ref(&params.path);
    match keep(params.how, params.side) {
        Keep::Add => vec![Invocation::new(with_paths(
            vec![GitArg::Flag("add")],
            paths,
        ))],
        Keep::Remove => vec![Invocation::new(with_paths(
            vec![GitArg::Flag("rm"), GitArg::Flag("--quiet")],
            paths,
        ))],
        Keep::CheckoutThenAdd => vec![
            Invocation::new(with_paths(
                vec![GitArg::Flag("checkout"), params.side.checkout_flag()],
                paths,
            )),
            Invocation::new(with_paths(vec![GitArg::Flag("add")], paths)),
        ],
    }
}

/// `check` for a restart: **a shape `checkout --merge` can actually merge**,
/// wherever the stages for it live.
///
/// "Unmerged" is not the honest gate, and saying it was cost a raw git error
/// where a refusal in words belongs (m6 g03 review, M1). Measured on git
/// 2.54.0 (Apple Git-157) over a merge holding one path per shape:
///
/// | shape | `git checkout --merge -- <p>` |
/// |---|---|
/// | `UU` both modified | exit 0, three stages back |
/// | `AA` both added | exit 0, two stages back (there is no base) |
/// | `UD` `DU` `AU` `UA` `DD` | **exit 1**, `does not have all necessary versions` |
///
/// All seven shapes were run, from the rename/rename fixture
/// `conflict_ops::stopped_merge` builds. Exactly the two that work are the
/// two [`UnmergedHow::has_markers`] names — a shape with one stage has
/// nothing to merge — so that is the gate.
///
/// # Why this no longer refuses a merged path (m6-conflicts/08, scope 5(c))
///
/// `d` has to work **after `⏎`**, which is the one case git alone cannot do:
/// `⏎` is `git add`, so the path sits at stage 0 and the status has no
/// [`UnmergedHow`] for it at all. There is nothing here to read — the stages
/// are in a recorded stage tree, and only a spawn reaches it, while `check`
/// is pure over [`RepoFacts`] by contract.
///
/// So the gate splits along what each half can see, and stays **one rule**:
///
/// - the status calls the path unmerged → decide here, exactly as before:
///   [`UnmergedHow::has_markers`] or [`Refusal::NotMarkerShape`];
/// - it does not → admit it here, and let [`crate::apply::stage_source`]
///   find the stages and apply the *same* test to them
///   ([`shape_of_stages`]): none kept anywhere is
///   [`Refusal::NoStagesKept`], a one-sided set is the same
///   [`Refusal::NotMarkerShape`] it would have been in the index.
///
/// A merged path is therefore never silently admitted: it is admitted here
/// and answered there, before any `git` that writes runs.
///
/// The way out of a one-sided shape is the whole-file resolution the resolver
/// already offers, which is what the refusal's words say.
pub(super) fn check_restart(path: &Path, facts: &RepoFacts) -> Result<(), Refusal> {
    let Some(how) = facts
        .status
        .entry(path)
        .and_then(|entry| entry.unmerged_how)
    else {
        // Stage 0, or not in the status at all: `apply` decides, because only
        // a read can say whether the sides were kept.
        return Ok(());
    };
    if how.has_markers() {
        Ok(())
    } else {
        Err(Refusal::NotMarkerShape {
            path: path.to_path_buf(),
            how,
        })
    }
}

/// The [`UnmergedHow`] a set of index stages spells, or `None` for no stages
/// at all.
///
/// The inverse of git's own `XY` pair, and the only way to apply the marker
/// gate to stages that come out of a recorded tree rather than out of the
/// status. Stage 1 is the merge base, 2 ours, 3 theirs, so: both sides
/// present is a marker shape (with a base `UU`, without one `AA`), one side
/// plus a base is a delete by the other, one side alone is an add by that
/// side, and a base alone is `DD`.
pub(crate) fn shape_of_stages(stages: &[StageEntry]) -> Option<UnmergedHow> {
    let has = |stage: u8| stages.iter().any(|entry| entry.stage == stage);
    Some(match (has(1), has(2), has(3)) {
        (true, true, true) => UnmergedHow::BothModified,
        (false, true, true) => UnmergedHow::BothAdded,
        (true, true, false) => UnmergedHow::DeletedByThem,
        (true, false, true) => UnmergedHow::DeletedByUs,
        (false, true, false) => UnmergedHow::AddedByUs,
        (false, false, true) => UnmergedHow::AddedByThem,
        (true, false, false) => UnmergedHow::BothDeleted,
        (false, false, false) => return None,
    })
}

/// `git checkout --merge -- <path>`: git re-runs its own merge of stages 1,
/// 2 and 3 into the working tree and puts the path back in the index as
/// unmerged. Verified, git 2.54: on a `UU` path with takes already written
/// to disk the markers come back and `ls-files -u` lists three stages again.
pub(super) fn plan_restart(params: &RestartParams) -> Vec<Invocation> {
    vec![Invocation::new(with_paths(
        vec![GitArg::Flag("checkout"), GitArg::Flag("--merge")],
        std::slice::from_ref(&params.path),
    ))]
}
