# history-reads-compare-and-stash — three reads in `graft_git::history_reads`: where two commits split, how many commits lie between, and which commits only a stash reaches

Status: draft
Blocked by: none
Crates: **graft-git only** (`history_reads`). Disjoint from graft: runs beside the serial graft chain.
Source: respec of 14 and 16 after scouts `.scratch/m7-power-tools/scouts/14.md` §4 and `16.md` §4, "Contradictions":
`merge_base` and `commits_between` live in **graft-git** (`graft-git/src/history_reads.rs:47`, `:98`), not graft-ops;
`Range` (`:84`) has `capped` but **no total**, so the force box's `+N more` cannot come from it.
Build with: Sonnet. Three plain git reads beside existing ones, each with its own tests; no state, no UI.
Scout: done (14 §4, 16 §Contradictions). No second scout.
Needs: none.
Serves: 14 (`compare_span`), 16 (`only_in_stash`), 16b (`count_between`).

## Decided
- **`compare_span(base, tip)`**: `merge_base(base, tip)`; `Some(split)` → `ahead = count_between(split, tip)`;
  `None` → `NoSharedHistory`. `tip` an ancestor of `base` → `split == tip`, `ahead == 0` (the caller words it).
  `base == tip` is answered the same way (`split == tip`, `ahead == 0`); the caller checks equality first.
- **`count_between(exclude, include)`**: `git rev-list --count <include> ^<exclude>`. Nothing capped: `--count` does
  not list.
- **`only_in_stash(stashes)`**: the commits reachable from a stash entry's **first parent** that no branch, tag,
  remote or `HEAD` reaches — the base a stash was made on after that base was rebased away. Never the index or
  untracked commits (2nd/3rd parents). `stashes` is newest first; the answer pairs each commit with the **lowest**
  index (newest entry) that reaches it. Held set = `unreached`'s minus the stash entries: `--branches --tags
  --remotes HEAD`; `refs/graft/**` does not hold (same as `unreached`, MAP.md `history_reads`).
  - Method is the builder's, with a bound: one `rev-list` per entry (`<entry>^1 --not --branches --tags --remotes
    HEAD`, `--max-count` = what `cap` has left) is acceptable; if the per-entry cost on the perf fixture exceeds
    `unreached`'s 190 ms p95 for one entry, fold them into one `--stdin` call as `unreached` does and say so.
  - An entry with no first parent readable (pruned) is passed over, counted in `skipped`.

## Read first
- `crates/graft-git/MAP.md` `history_reads` (all of it: `merge_base`, `commits_between`, `unreached` and its
  `--stdin` note for git < 2.42).
- Scout 14 §4, scout 16 §4 and "Contradictions".
- Source after the MAP: `graft-git/src/history_reads.rs:47` (`merge_base`), `:84` (`Range`), `:98`
  (`commits_between`), and `unreached`'s method.

## Signatures
The first parameter is **the type `merge_base` takes today** (written `Repo` below; copy it, don't invent one). The
error type is `RunError`, as `unreached`.
```rust
pub enum CompareSpan {
    /// `split` is the merge base; `ahead` = commits in `tip` not in `split`.
    Since { split: Oid, ahead: usize },
    /// `git merge-base` exit 1: no common ancestor.
    NoSharedHistory,
}
pub fn compare_span(repo: &Repo, base: Oid, tip: Oid, stop: &dyn Stop) -> Result<CompareSpan, RunError>;

pub fn count_between(repo: &Repo, exclude: Oid, include: Oid, stop: &dyn Stop) -> Result<usize, RunError>;

pub struct StashOnly {
    /// (commit, n): n is the newest stash entry (`stash@{n}`) whose first parent reaches it.
    pub commits: Vec<(Oid, usize)>,
    pub capped: bool,
    pub skipped: usize,
}
pub fn only_in_stash(repo: &Repo, stashes: &[Oid], cap: usize, stop: &dyn Stop) -> Result<StashOnly, RunError>;
```

## Scope
- In: the three fns, their tests, MAP.md, one perf script for `only_in_stash`.
- Out: every caller (14, 16, 16b). `Range`, `commits_between`, `merge_base` unchanged.

## Acceptance (each box names its test at tick time)
- [ ] `compare_span_counts_commits_since_the_split` — main + feature-a 3 commits off it → `Since { split, ahead: 3 }`.
- [ ] `an_ancestor_has_nothing_ahead` — tip is an ancestor of base → `split == tip`, `ahead == 0`.
- [ ] `unrelated_histories_have_no_split` — two roots → `NoSharedHistory`, not an error.
- [ ] `compare_span_honours_stop`, `count_between_honours_stop`, `only_in_stash_honours_stop` — a raised `Stop` →
      `Err(Stopped)` (whatever `RunError`'s stopped variant is named), never a partial answer.
- [ ] `count_between_counts_what_leaves` — `count_between(new, old)` on a rewritten branch = the commits only `old`
      has.
- [ ] `only_in_stash_finds_the_rebased_away_base` — stash on `main@A`, then `main` rebased so `A` is off every
      branch → `A` paired with `0`.
- [ ] `only_in_stash_never_names_index_or_untracked_commits` — `stash -u` → neither extra parent appears.
- [ ] `a_commit_two_stashes_reach_gets_the_newest` — two entries reaching `A` → `(A, 0)`.
- [ ] `a_stash_on_a_live_branch_has_nothing_only_in_it` — the stash's base still on `main` → empty.
- [ ] `only_in_stash_is_capped` — `cap` reached → `capped == true`, `commits.len() == cap`.
- [ ] Perf: `scripts/m7-stash-only-cost.sh` → `docs/perf/stash-only-cost.txt` on the fixture `lost-commit-cost`
      built (100k commits / 50k refs, no commit-graph): 1 entry and 10 entries, p95 line. The builder reads the
      summary lines only.
- [ ] `crates/graft-git/MAP.md` `history_reads`: the three fns, one line each.

## Touches (check each, report how)
- **`merge_base`, `commits_between`, `replayed_commits`, `unreached`:** unchanged. Check: their tests green,
  `git diff` shows no edit to their bodies.
- **Fuzz targets:** `--count` output is one integer; parse errors go through `RunError` (never `unwrap_or(0)`, rule 3).
  No new fuzz target; say so in NOTES.
- **Other crates:** none compile against the new items yet. Check: workspace gates.

## Rules / Gates
As rewrite-engine. Workspace-wide gates.

## Size
Small-medium (~30k).
