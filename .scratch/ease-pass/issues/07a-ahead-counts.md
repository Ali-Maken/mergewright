# ahead-counts — how far every local branch is ahead of its upstream, read once per refs read

Status: partial — perf over budget, stopped per ticket (planner spec-only pass 2026-10-18, after the user's answer: `↑N to push` on every local branch with unpushed commits)
Blocked by: none for path A (below). Path B appends to `jobs/kinds.rs`, `update/state.rs`, `update/events.rs` → after 06s merges, and not beside 11b/12b if either appends there (11b:10, 12b:14).
Build with: Sonnet (builder-sonnet) — set by conductor after scout `.scratch/ease-pass/scouts/07a-ahead-counts.md`: counts already in `RefLabel::Branch.upstream` (`refs/mod.rs:295-322`), path A, an accessor plus tests, no new job. **User, 2026-10-18:** a local branch with **no upstream** draws the label `not pushed yet` (no number, no base guess); 07 draws it beside `↑N to push`. Acceptance: one fixture branch without upstream → accessor answers NotPushed; one with upstream at 0 → nothing.
Needs: none. Feeds: 07 (`RefBadges::ahead_of_upstream`).

## Why
User, 2026-10-18: `↑N to push` shows on **every** local branch with unpushed commits, not HEAD's only. 07's scout confirmed only HEAD's count (`counts.ahead: usize`, `mergewright-git/src/history/divergence.rs:29`). A per-branch source is **not confirmed**. The MAPs point at three candidates: app `refs` (MAP:187 "how far each branch is from its upstream", `Upstream` :191); ops `BranchFacts.upstream: UpstreamFacts::Apart { ahead, behind }` (ops MAP:180–181); ui `Tracking` (ui MAP:191). Nobody has checked these against the code.

## Read first
- `crates/mergewright/MAP.md` `refs` (:186–191), `jobs::kinds`, `event_loop::rest` (:64). `crates/mergewright-git/MAP.md` `AheadBehind`, `ahead_behind` (:161–163), `upstream_of` (:210). `crates/mergewright-ops/MAP.md` `UpstreamFacts`, `BranchFacts` (:180–181). `crates/mergewright-ui/MAP.md` `Tracking` (:191).
- CLAUDE.md Gates: "Measurement never runs in a helper's context".
- The scout's file.

## Scout questions (≤1 page)
- Does `refs::Upstream` (or `RefBadges`) carry an ahead count for **every** local branch with an upstream? Which read fills it (fn, file:line), and when does it run (startup, after each operation, on refresh, per frame)?
- Do `UpstreamFacts` / `Tracking` hold the same count for every branch, or only for HEAD's? Who reads them?
- How many `ahead_behind` calls one refs read makes today, and on which thread.

## Decided here
- **Settle first, one committed test:** `every_tracked_local_branch_has_its_ahead_count` — fixture with 3 local branches: one 2 ahead of its upstream, one in sync, one with no upstream. Assert what `RefBadges` holds after one refs read. The outcome picks the path. NOTES records it.
  - **Path A — the count is already there for every branch:** add only the accessor below. No new read.
  - **Path B — it isn't:** the counts become **their own job**, asked when a refs read lands, so the graph's refs never wait on it. One `ahead_behind` per local branch that has an upstream, none for the others. Results are keyed by the refs read they answer, and a result for an older read is dropped. Job kind, event and `AppState` field go at the end of their files.
- **Never per frame, never per cursor move.** The count changes only when refs change.
- **Before it lands:** nothing is drawn. No `↑?` and no `reading…`. In-sync (0) and no-upstream draw nothing either. The header's `N commits to push` keeps its own source. A test asserts that once both have landed, HEAD's label equals the header's count.
- **Cost bound:**
  - **Fast tier, by count, not by time:** `the_ahead_read_asks_once_per_tracked_branch` runs on a 500-branch fixture (200 with upstreams). It asserts exactly 200 `ahead_behind` calls per refs read, and 0 calls from 40 cursor moves and 40 redraws.
  - **Time, in a script:** `scripts/ease-ahead-read.sh` runs in release on 2,000 local branches (500 tracked, each ≤50 commits apart) and writes `docs/perf/07a-ahead-counts.txt`. The bound is **≤100 ms per refs read, off the UI thread**. The builder reads only the summary line. The milestone review re-runs the script with the release perf run.

## Scope
In: app `refs` (accessor; path B: the read's call site). Path B adds `jobs/` (a new job file and its `kinds.rs` row), `update/events.rs` and `update/state.rs` (append), and `event_loop` (asked when refs land). Also `scripts/ease-ahead-read.sh` and `docs/perf/07a-ahead-counts.txt`.
Out: drawing the label (07); push itself; the header's count; remote-tracking-only branches (they have nothing to push).

## Signatures
```rust
// mergewright app, refs
impl RefBadges {
    /// Commits `branch` (a local branch) has that its upstream lacks.
    /// `None`: no upstream, or the count has not landed yet. `Some(0)`: in sync.
    pub(crate) fn ahead_of_upstream(&self, branch: &RefLabel) -> Option<usize>;
}
```
Path B adds a job kind `AheadCounts`, an event carrying `Vec<(RefLabel, usize)>` plus the read it answers, and the keyed store. NOTES gives the exact names.

## Touches
Refs read timing; startup cost (path B: a job after refs, no wait added before the first frame); `jobs::kinds` and the job-kinds coverage test; operations that re-read refs (push, pull, commit, branch ops — the count refreshes with them); `classify_refs` `UnpushedOnDefault` (ops; reads its own count, unchanged); the header's `N commits to push`; 07's labels; MAP `refs`.

## Acceptance
Each tick names its test inline.
- [x] The settle test is committed first (`refs::ahead_tests::every_tracked_local_branch_has_its_ahead_count`). The path taken and why are in NOTES.
- [x] `ahead_of_upstream` (`ahead_of_upstream_counts_the_ahead_branch`, `_is_zero_in_sync`, `_without_an_upstream_is_not_pushed`, `_is_none_off_a_local_branch`; path A, so no "before landing" case): 2 for the ahead branch, `Some(0)` in sync, `None` with no upstream, `None` before landing (path B) (unit tests).
- [~] `the_ahead_read_asks_once_per_tracked_branch` (200 walks cold, 0 on a warm re-read — ticked part). **0 on moves/redraws not tested**: shown by grep only, `refs::read` has one caller, `jobs/runner/reads.rs:99` (the ReadRefs job): 200 calls per refs read, 0 on moves or redraws.
- [x] N/A, path A. Path B: a result for an older refs read is dropped (state test).
- [ ] `docs/perf/07a-ahead-counts.txt` summary line ≤100 ms (release, 2,000 branches). The figure is in NOTES. If it exceeds 100 ms: **stop** and report the figure. No cap or sampling without the user's OK.
- [x] (`heads_count_equals_the_headers`) HEAD's count from this source equals the header's after both land (state test).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Path A ~25k; path B ~55k.

## NOTES (builder, 2026-10-18)
- Scout used (`.scratch/ease-pass/scouts/07a-ahead-counts.md`). Path A: settle test shows one refs read already stores every tracked branch's count (`refs/mod.rs` `badges`). No new job, no new read.
- **Deviation from the signature, on the user's decision:** `RefLabel::ahead_of_upstream(&self) -> Option<AheadOfUpstream>` (`refs/ahead.rs`), `AheadOfUpstream::{NotPushed, By(n)}`. Not on `RefBadges`: the label already holds its upstream (scout point 5), and `Option<usize>` cannot say `not pushed yet` apart from "not a branch". `By(0)` = in sync; 07 draws nothing for it.
- **Perf: over budget, stopped.** `scripts/ease-ahead-read.sh` summary: cold refs read median 121.4 ms (max 179.1), warm 3.2 ms; 2000 local, 500 tracked; load 15–21 (another builder compiling beside). Cold includes opening the repo and reading 2,500 refs, not only the 500 walks; split not measured. No cap or sampling added. Needs: re-run on an idle machine, then the user/conductor decides.
- Pre-existing: `--features heavy-tests` breaks `tests/it/heavy.rs:321,347` (`HistoryUpdate::FoldedHits` not covered) — not this ticket.
