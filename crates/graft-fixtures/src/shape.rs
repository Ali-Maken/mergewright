//! The named shapes the corpus contains.
//!
//! PROMPT.md §7.2 insists the corpus is *named*: a test asserts against a
//! shape someone can describe, not against whatever a developer's own
//! repository happens to look like. The list below is that spec's list, plus
//! the ref states a graph has to draw, plus the trivial case — a repository
//! with nothing to draw at all, which is the one the graph is easiest to get
//! wrong on.

/// One history or ref shape, and one repository on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Shape {
    /// A small linear repository: no merges, no tags, no branch but the
    /// default one. The trivial case (PROMPT.md §7.2).
    Trivial,
    /// 200 commits in a straight line — still small enough for the fast
    /// tier, long enough that a viewport has to scroll.
    Linear,
    /// Several feature branches off different points, two merged back, one
    /// still open.
    FeatureBranches,
    /// One merge with four parents.
    OctopusMerge,
    /// Two branches that merged each other, so they share two merge bases.
    CrissCrossMerge,
    /// A merge on the default branch whose *first* parent is the feature
    /// branch, so first-parent history leaves the default branch.
    FoxtrotMerge,
    /// The default branch merged *into* a feature branch.
    BackwardMerge,
    /// A merge whose tree holds a change that is in neither parent.
    EvilMerge,
    /// The merge `git pull` makes when a push was rejected: `Merge branch
    /// 'main' of <remote>`.
    AccidentalPullMerge,
    /// A second root commit: a branch with no ancestor in common with the
    /// default branch.
    OrphanBranch,
    /// The same patch committed on two branches, so the two commits share a
    /// patch id and no ancestry.
    CherryPickDuplicates,
    /// A commit dated a day ahead of its children — a fast clock — reached
    /// from two branches, so commit-date order shows it above a child and
    /// only a topological walk puts it below both (ADR-0008 promotion).
    SkewedDates,
    /// One straight lane whose seven commits sit at ages from `CORPUS_NOW`
    /// back to a bit over a year: 5s, 90s, 90min, 36h, 14d, 70d, 400d
    /// (m1-the-graph/48) — every unit the relative-date column has,
    /// `now`/`m`/`h`/`d`/`w`/`mo`/`y`, in one screen. Dated off
    /// [`crate::CORPUS_NOW`] (m1-the-graph/53), the same as every other
    /// shape is dated off the corpus's fixed 2020-01-01 clock: its object
    /// ids are byte-identical run to run, and viewing it at `CORPUS_NOW` —
    /// not the wall clock — is what keeps the seven ages reading right.
    RelativeDates,
    /// Lightweight and annotated tags, including one annotated tag whose
    /// message a graph row has to show.
    Tags,
    /// Branch names, tag names, paths, commit messages and an author name
    /// that are valid UTF-8 and not ASCII.
    UnicodeNames,
    /// Remote-tracking states: one branch level with its upstream, one
    /// ahead, one behind, and a commit no branch reaches — only a tag does.
    RefStates,
    /// `HEAD` naming a commit directly, with no branch to advance.
    DetachedHead,
    /// `HEAD` on a branch that has no commit yet, while other branches do.
    UnbornBranch,
    /// Two stash entries, so `refs/stash` and its reflog both exist.
    Stash,
    /// A path that is not valid UTF-8, living only inside a tree object.
    /// Nothing is ever checked out, so this one builds anywhere — including
    /// on APFS, where a non-UTF-8 *file* name is impossible.
    NonUtf8Path,
    /// A branch name that is not valid UTF-8. A loose ref is a file, so this
    /// one is built only where the filesystem accepts such a name — not on
    /// APFS.
    NonUtf8RefName,
    /// A repository left at a rebase conflict: `.git/rebase-merge/` present,
    /// `HEAD` detached at `main`, and one stop with every unmerged shape git
    /// has — thirteen paths, listed file by file in `MAP.md` (m6-conflicts/01).
    /// In `STOPPED`, not `ALL`: see there.
    StoppedRebase,
    /// The same files and the same stop reached by `git merge feature` on
    /// `main` instead, so `MERGE_HEAD` is present and both `in_progress`
    /// kinds have a corpus.
    StoppedMerge,
}

impl Shape {
    /// Every shape, in a fixed order.
    pub const ALL: &'static [Shape] = &[
        Shape::Trivial,
        Shape::Linear,
        Shape::FeatureBranches,
        Shape::OctopusMerge,
        Shape::CrissCrossMerge,
        Shape::FoxtrotMerge,
        Shape::BackwardMerge,
        Shape::EvilMerge,
        Shape::AccidentalPullMerge,
        Shape::OrphanBranch,
        Shape::CherryPickDuplicates,
        Shape::SkewedDates,
        Shape::RelativeDates,
        Shape::Tags,
        Shape::UnicodeNames,
        Shape::RefStates,
        Shape::DetachedHead,
        Shape::UnbornBranch,
        Shape::Stash,
        Shape::NonUtf8Path,
        Shape::NonUtf8RefName,
    ];

    /// The shapes stopped mid-operation (m6-conflicts/01), built by the
    /// corpus alongside `ALL` but listed apart: every consumer of `ALL` — the
    /// gallery's stops, the graph's snapshots, the mistake radar's expected
    /// findings — owes each shape a decision of its own, and those belong to
    /// the crates that draw them, not to the fixture. Fold these into `ALL`
    /// when those decisions are made.
    pub const STOPPED: &'static [Shape] = &[Shape::StoppedRebase, Shape::StoppedMerge];

    /// The shape's directory name inside the corpus, and the name tests and
    /// reports call it by.
    pub fn name(self) -> &'static str {
        match self {
            Shape::Trivial => "trivial",
            Shape::Linear => "linear",
            Shape::FeatureBranches => "feature-branches",
            Shape::OctopusMerge => "octopus-merge",
            Shape::CrissCrossMerge => "criss-cross-merge",
            Shape::FoxtrotMerge => "foxtrot-merge",
            Shape::BackwardMerge => "backward-merge",
            Shape::EvilMerge => "evil-merge",
            Shape::AccidentalPullMerge => "accidental-pull-merge",
            Shape::OrphanBranch => "orphan-branch",
            Shape::CherryPickDuplicates => "cherry-pick-duplicates",
            Shape::SkewedDates => "skewed-dates",
            Shape::RelativeDates => "relative-dates",
            Shape::Tags => "tags",
            Shape::UnicodeNames => "unicode-names",
            Shape::RefStates => "ref-states",
            Shape::DetachedHead => "detached-head",
            Shape::UnbornBranch => "unborn-branch",
            Shape::Stash => "stash",
            Shape::NonUtf8Path => "non-utf8-path",
            Shape::NonUtf8RefName => "non-utf8-ref-name",
            Shape::StoppedRebase => "stopped-rebase",
            Shape::StoppedMerge => "stopped-merge",
        }
    }

    /// The shape whose [`Shape::name`] is `name`, or `None`.
    pub fn from_name(name: &str) -> Option<Shape> {
        Shape::every().find(|shape| shape.name() == name)
    }

    /// Every shape the corpus builds: [`Shape::ALL`] then [`Shape::STOPPED`].
    pub fn every() -> impl Iterator<Item = Shape> {
        Shape::ALL.iter().chain(Shape::STOPPED).copied()
    }
}

impl std::fmt::Display for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
