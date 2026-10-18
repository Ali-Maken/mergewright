//! What a merge preview spawns, one `git` per entry, measured by
//! `the_merge_preview_forecasts_the_put_back` (split out of the parent at
//! its size ceiling).

/// A true merge's preview over a clean tree, one `git` per entry (the merge
/// arm of `sets_aside` spawns nothing; it reads the facts). Ease-pass 10
/// added the first `merge-base` (in-the-way: unrelated histories have none)
/// and the merge look: `merge_forecast` (`merge-base`, `rev-list`,
/// `merge-tree`, `diff-tree`) in place of `forecast_one`'s lone
/// `merge-tree` — still one `merge-tree` — and the incoming `rev-list`.
pub(super) const CLEAN_TREE_SPAWNS: &[&str] = &[
    "merge-base",
    "diff-tree",
    "ls-files",
    "merge-base",
    "var",
    "merge-base",
    "diff-tree",
    "merge-base",
    "rev-list",
    "merge-tree",
    "diff-tree",
    "rev-list",
];

/// The put-back forecast's own, after the merge's (ADR-0045 §8).
pub(super) const PUT_BACK_SPAWNS: &[&str] = &[
    "stash create",
    "merge-base",
    "merge-tree",
    "commit-tree",
    "merge-tree",
];

/// A fast-forward's preview over a clean tree, measured as
/// [`CLEAN_TREE_SPAWNS`] was, with the same ease-pass 10 additions (a
/// fast-forward's look runs the full `merge_forecast` too).
pub(super) const FF_CLEAN_TREE_SPAWNS: &[&str] = &[
    "merge-base",
    "diff-tree",
    "ls-files",
    "merge-base",
    "diff-tree",
    "merge-base",
    "rev-list",
    "merge-tree",
    "diff-tree",
    "rev-list",
];

/// A fast-forward lands on the source itself: no merged tree to make.
pub(super) const FF_PUT_BACK_SPAWNS: &[&str] = &["stash create", "merge-base", "merge-tree"];
