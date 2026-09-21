//! The bounded slice a Ghost Preview is laid out from, and the badge
//! overlay drawn over it. Both pure: no [`AppState`](crate::update::AppState),
//! no repository, no clock.
//!
//! Hard choice 1 of the M5 design note: a preview does **not** intern
//! synthetic commits into the live table and re-walk. It takes the rows
//! already resident around the cursor, inserts the plan's ghosts as ordinary
//! [`Commit`]s with synthetic ids, and calls [`graft_graph::layout`] once.
//! The result replaces the graph rows while the preview is open.
//!
//! Two consequences the rest of the app has to live with, and which this
//! module makes visible rather than hiding:
//!
//! - **A preview never loads more history.** A parent that is not in the
//!   slice is dropped from its child's copy and counted in [`Slice::clipped`],
//!   so `layout` is never handed a merge whose parent it cannot find. The
//!   lane simply ends at the slice edge.
//! - **When the part that matters is the part missing, the slice refuses.**
//!   A ghost whose real parent is neither resident nor an anchor cannot be
//!   drawn in the right place at all, and drawing it in the wrong one would
//!   be a picture the user could misread. That is
//!   [`SliceError::MissingParent`], and the box says so in words
//!   (ADR-0038 §5, the conductor's binding condition).

use std::collections::{HashMap, HashSet};

use graft_core::commit::Commit;
use graft_core::oid::Oid;
use graft_ops::preview::{HeadMove, PreviewPlan, Target};

use crate::refs::{RefBadges, RefLabel, Upstream};

/// How many commits a preview will read to anchor ghosts whose parents are
/// off the slice. Past it the answer is [`SliceError::MissingParent`]
/// rather than a spawn storm: a preview that costs sixty-four commit reads
/// is already more than a keystroke should buy, and "not loaded" is an
/// honest answer where "wait" is not.
pub const PREVIEW_ANCHOR_CAP: usize = 64;

/// The commits one preview lays out, in drawing order (newest first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    /// What [`graft_graph::layout`] is handed.
    pub commits: Vec<Commit>,
    /// Where each of the plan's ghosts landed in [`Slice::commits`], in the
    /// plan's own ghost order. `ghost_at[i]` indexes `commits`.
    pub ghost_at: Vec<usize>,
    /// How many parent links were dropped because the parent is outside the
    /// slice. A lane that ends at the slice edge is the accepted cost of a
    /// bounded picture (hard choice 1) — but it is a cost, so it is counted
    /// and the box says how many.
    pub clipped: usize,
}

/// Why a slice could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SliceError {
    /// A ghost's real parent is neither among the resident rows nor among
    /// the anchors the job read. There is no correct row to put the ghost
    /// above, and a ghost in the wrong place is worse than no picture.
    MissingParent(Oid),
}

/// Builds the slice for `plan` from the rows already on screen.
///
/// `resident` is the window the caller chose — every resident row in
/// `[top − graph_rows, top + 2·graph_rows)` — in drawing order, newest
/// first. `anchors` are the commits the job had to read because a target or
/// a ghost parent was not resident; they are appended after the last
/// resident row **with their own parents cleared**, because whatever they
/// point at is certainly not in the slice either.
///
/// Ghosts go in in the plan's order, each immediately above the topmost
/// slice row among its transitive real parents — so a rebase chain sits
/// right above its `onto` (newest replay first, the order history is drawn
/// in) and a ghost merge sits right above the higher of its two parents.
pub fn build_slice(
    resident: &[Commit],
    anchors: &[Commit],
    plan: &PreviewPlan,
) -> Result<Slice, SliceError> {
    // The real rows, in order: what is resident, then the anchors with no
    // parents of their own.
    let mut real: Vec<Commit> = resident.to_vec();
    // An anchor is the bottom of the picture: its own parents were never
    // read, so they are cleared — and **counted**, because history plainly
    // continues below it and a preview that quietly ended there would be
    // drawing a shorter history than the one it is about to change.
    let mut clipped: usize = anchors.iter().map(|anchor| anchor.parents.len()).sum();
    real.extend(anchors.iter().map(|anchor| Commit {
        parents: Vec::new(),
        ..anchor.clone()
    }));

    let at: HashMap<Oid, usize> = real
        .iter()
        .enumerate()
        .map(|(index, commit)| (commit.id, index))
        .collect();

    // Where each ghost wants to sit, as an index into `real` **before** any
    // ghost is inserted. Computing it against the unmodified list is what
    // makes a chain come out newest-first: several ghosts anchored at the
    // same row are emitted in reverse plan order below, so each newer one
    // pushes the older ones down rather than landing under them.
    let mut anchor_of: Vec<usize> = Vec::with_capacity(plan.ghosts.len());
    for index in 0..plan.ghosts.len() {
        let mut lowest = None;
        for parent in transitive_real_parents(plan, index) {
            match at.get(&parent) {
                Some(&row) => lowest = Some(lowest.map_or(row, |seen: usize| seen.min(row))),
                // The one case that cannot be drawn: a ghost's real parent
                // that the job did not read and the window does not hold.
                None => return Err(SliceError::MissingParent(parent)),
            }
        }
        // A ghost with no real parent anywhere (a plan that would create a
        // root commit) goes to the very top: there is nothing for it to sit
        // above, and the top is where the newest row is.
        anchor_of.push(lowest.unwrap_or(0));
    }

    // Emit: before each real row, every ghost anchored at it, newest first.
    let mut commits: Vec<Commit> = Vec::with_capacity(real.len() + plan.ghosts.len());
    let mut ghost_at: Vec<usize> = vec![0; plan.ghosts.len()];
    let ghost_oids: Vec<Oid> = plan.ghosts.iter().map(|ghost| ghost.oid).collect();
    for (row, commit) in real.iter().enumerate() {
        for index in (0..plan.ghosts.len()).rev() {
            if anchor_of[index] == row {
                ghost_at[index] = commits.len();
                commits.push(ghost_commit(plan, index, &ghost_oids));
            }
        }
        commits.push(commit.clone());
    }

    // Clip: `layout` requires every parent of every commit it is given to be
    // in the batch (`MergeNames::gather`'s reader is `unreachable!()` there).
    // A bounded slice cannot promise that, so the links that leave it are
    // dropped here — visibly, counted — rather than left to panic.
    let present: HashSet<Oid> = commits.iter().map(|commit| commit.id).collect();
    for commit in &mut commits {
        let before = commit.parents.len();
        commit.parents.retain(|parent| present.contains(parent));
        clipped += before - commit.parents.len();
    }

    Ok(Slice {
        commits,
        ghost_at,
        clipped,
    })
}

/// One ghost as a [`Commit`] `layout` can take: the plan's synthetic id, its
/// parents resolved through the plan's own ghost list, and the subject and
/// author the plan worked out. The committer is the author — a ghost has no
/// commit date of its own, and the row draws `—` for `when` anyway.
fn ghost_commit(plan: &PreviewPlan, index: usize, ghost_oids: &[Oid]) -> Commit {
    let ghost = &plan.ghosts[index];
    Commit {
        id: ghost.oid,
        parents: ghost
            .parents
            .iter()
            .filter_map(|parent| match *parent {
                Target::Real(oid) => Some(oid),
                Target::Ghost(at) => ghost_oids.get(at).copied(),
            })
            .collect(),
        author: ghost.author.clone(),
        committer: ghost.author.clone(),
        message: ghost.subject.clone().into_bytes(),
    }
}

/// Every real commit the ghost at `index` reaches through its parents,
/// following [`Target::Ghost`] links inside the plan.
///
/// Cycle-safe: a plan's ghosts are parents-before-children, so a walk can
/// only go backwards through the list, but the `seen` set means a plan that
/// broke that rule would still terminate rather than hang the render thread.
fn transitive_real_parents(plan: &PreviewPlan, index: usize) -> Vec<Oid> {
    let mut seen: HashSet<usize> = HashSet::new();
    let mut queue = vec![index];
    let mut reals = Vec::new();
    while let Some(at) = queue.pop() {
        if !seen.insert(at) {
            continue;
        }
        let Some(ghost) = plan.ghosts.get(at) else {
            continue;
        };
        for parent in &ghost.parents {
            match *parent {
                Target::Real(oid) => reals.push(oid),
                Target::Ghost(next) => queue.push(next),
            }
        }
    }
    reals
}

/// The badges the preview draws, over the ones the repository has.
///
/// Borrows the real badges rather than copying them: only the commits a ref
/// actually moves off or onto are recorded here, and everything else is
/// answered straight from the repository's own map.
#[derive(Clone, Debug)]
pub struct BadgeOverlay<'a> {
    /// What the refs read last said.
    base: &'a RefBadges,
    /// Refs that leave a commit, by commit: short name **and kind**. A
    /// branch and a tag can share a short name on one commit, and moving
    /// the branch must not take the tag's badge with it (review 08 Minor
    /// 11).
    removed: HashMap<Oid, Vec<(LabelKind, Vec<u8>)>>,
    /// Labels that arrive at a commit, by commit.
    added: HashMap<Oid, Vec<RefLabel>>,
    /// Where the `HEAD` node mark goes once the plan has run.
    head: Option<Oid>,
}

impl BadgeOverlay<'_> {
    /// The labels `oid`'s row draws once the plan has run: the repository's
    /// own, less the refs that move off this commit, plus the refs that move
    /// onto it.
    #[must_use]
    pub fn on(&self, oid: Oid) -> Vec<RefLabel> {
        let gone = self.removed.get(&oid);
        let mut labels: Vec<RefLabel> = self
            .base
            .on(oid)
            .iter()
            .filter(|label| match gone {
                Some(gone) => !gone.iter().any(|(kind, name)| is_ref(label, *kind, name)),
                None => true,
            })
            .cloned()
            .collect();
        if let Some(arriving) = self.added.get(&oid) {
            labels.extend(arriving.iter().cloned());
        }
        labels
    }

    /// The commit the `HEAD` node mark sits on once the plan has run.
    #[must_use]
    pub fn head(&self) -> Option<Oid> {
        self.head
    }
}

/// Where every ref lands once `plan` has run.
///
/// Every move comes from [`PreviewPlan::ref_moves`] and [`PreviewPlan::head`]
/// — nothing is inferred from the operation's kind. A branch or tag delete is
/// a `RefMove` with `to: None` and needs no special case: the badge leaves
/// its commit and arrives nowhere.
#[must_use]
pub fn badges_for_preview<'a>(badges: &'a RefBadges, plan: &PreviewPlan) -> BadgeOverlay<'a> {
    let mut removed: HashMap<Oid, Vec<(LabelKind, Vec<u8>)>> = HashMap::new();
    let mut added: HashMap<Oid, Vec<RefLabel>> = HashMap::new();
    let ghost_oids: Vec<Oid> = plan.ghosts.iter().map(|ghost| ghost.oid).collect();

    for moved in &plan.ref_moves {
        let short = short_of(&moved.name);
        let kind = LabelKind::of_full(&moved.name);
        // The label as it is drawn today, so a branch that moves keeps its
        // upstream chip and its `current` mark instead of being redrawn as a
        // plain untracked branch it is not.
        let existing = moved
            .from
            .and_then(|from| {
                badges
                    .on(from)
                    .iter()
                    .find(|label| is_ref(label, kind, &short))
                    .cloned()
            })
            .unwrap_or_else(|| label_for(&moved.name, short.clone()));
        if let Some(from) = moved.from {
            removed.entry(from).or_default().push((kind, short.clone()));
        }
        if let Some(to) = moved.to
            && let Some(oid) = resolve(to, &ghost_oids)
        {
            added.entry(oid).or_default().push(existing);
        }
    }

    BadgeOverlay {
        base: badges,
        removed,
        added,
        head: head_after(badges, plan, &ghost_oids),
    }
}

/// Where the `HEAD` mark ends up: the commit the named branch lands on for
/// [`HeadMove::ToBranch`], the target itself for a detach, and — when the
/// plan moves no `HEAD` — wherever `HEAD`'s own commit went, which is how a
/// branch moving under a `HEAD` still on it carries the mark along.
fn head_after(badges: &RefBadges, plan: &PreviewPlan, ghost_oids: &[Oid]) -> Option<Oid> {
    match &plan.head {
        Some(HeadMove::ToBranch(name)) => plan
            .ref_moves
            .iter()
            .find(|moved| moved.name == *name)
            .and_then(|moved| moved.to)
            .and_then(|to| resolve(to, ghost_oids))
            // The branch `HEAD` moves to is not one this plan moves: it is
            // already somewhere, and that is where the mark goes.
            .or_else(|| {
                let short = short_of(name);
                badges
                    .branches()
                    .find(|(_, branch, _, _)| *branch == short.as_slice())
                    .map(|(oid, _, _, _)| oid)
            }),
        Some(HeadMove::Detached(target)) => resolve(*target, ghost_oids),
        None => {
            let here = badges.head()?;
            // `HEAD` stays where it is unless the ref it is on moves out
            // from under it.
            plan.ref_moves
                .iter()
                .find(|moved| moved.from == Some(here) && is_head_branch(badges, here, &moved.name))
                .map_or(Some(here), |moved| {
                    moved.to.and_then(|to| resolve(to, ghost_oids))
                })
        }
    }
}

/// Is `name` the branch `HEAD` is currently on, as the badges have it?
fn is_head_branch(badges: &RefBadges, at: Oid, name: &str) -> bool {
    let short = short_of(name);
    badges.on(at).iter().any(
        |label| matches!(label, RefLabel::Branch { name, current: true, .. } if *name == short),
    )
}

/// A [`Target`] as a commit id, resolving a ghost index through the plan's
/// own ghost list. `None` only for an index the plan does not have, which a
/// well-formed plan never produces.
fn resolve(target: Target, ghost_oids: &[Oid]) -> Option<Oid> {
    match target {
        Target::Real(oid) => Some(oid),
        Target::Ghost(at) => ghost_oids.get(at).copied(),
    }
}

/// The badge a ref name would draw as if the repository is not already
/// drawing one for it: the kind is read off the full name's prefix, and a
/// branch arriving fresh is neither current nor tracking anything this
/// preview knows about.
fn label_for(full: &str, short: Vec<u8>) -> RefLabel {
    match LabelKind::of_full(full) {
        LabelKind::Tag => RefLabel::Tag(short),
        LabelKind::Remote => RefLabel::Remote(short),
        LabelKind::Branch => RefLabel::Branch {
            name: short,
            current: false,
            upstream: Upstream::None,
        },
    }
}

/// Which kind of ref a label or a full ref name is. A short name alone does
/// not identify a badge: `v1` can be a tag and a branch at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelKind {
    /// `refs/heads/…`
    Branch,
    /// `refs/remotes/…`
    Remote,
    /// `refs/tags/…`
    Tag,
}

impl LabelKind {
    /// A `RefMove`'s own name. Anything unprefixed is a branch — which is
    /// what `origin/main` in a push's `RefMove` is not, but `short_of`
    /// leaves that one unchanged and `RefLabel::Remote` is matched by the
    /// `refs/remotes/` arm above it.
    fn of_full(full: &str) -> LabelKind {
        if full.starts_with("refs/tags/") {
            LabelKind::Tag
        } else if full.starts_with("refs/remotes/") {
            LabelKind::Remote
        } else {
            LabelKind::Branch
        }
    }
}

/// Is `label` this exact ref — the same kind and the same short name?
fn is_ref(label: &RefLabel, kind: LabelKind, short: &[u8]) -> bool {
    let matches_kind = match label {
        RefLabel::Branch { .. } => kind == LabelKind::Branch,
        RefLabel::Remote(_) => kind == LabelKind::Remote,
        RefLabel::Tag(_) => kind == LabelKind::Tag,
        RefLabel::DetachedHead | RefLabel::Stash(_) => false,
    };
    matches_kind && short_name(label) == Some(short)
}

/// A label's short name, or `None` for the shapes that have none
/// (`HEAD` detached, a stash entry).
fn short_name(label: &RefLabel) -> Option<&[u8]> {
    match label {
        RefLabel::Branch { name, .. } | RefLabel::Remote(name) | RefLabel::Tag(name) => Some(name),
        RefLabel::DetachedHead | RefLabel::Stash(_) => None,
    }
}

/// `refs/heads/main` → `main`; `origin/main` (what a push's `RefMove` calls
/// a remote branch) is already short and comes back unchanged.
#[must_use]
pub fn short_of(full: &str) -> Vec<u8> {
    for prefix in ["refs/heads/", "refs/remotes/", "refs/tags/"] {
        if let Some(rest) = full.strip_prefix(prefix) {
            return rest.as_bytes().to_vec();
        }
    }
    full.as_bytes().to_vec()
}

#[cfg(test)]
mod tests;
