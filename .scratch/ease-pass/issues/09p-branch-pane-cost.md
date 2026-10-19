# branch-pane-cost — the pane answers in under a second on his monorepo

Status: partial: built as specified; window budget missed, stopped per spec (2026-10-19)
Blocked by: none (09 merged).
Beside 09b: **yes**. 09b is app-only (`update::picker`, `C`, `view::entries`). This ticket never touches the app. Beside 09c: **yes**, since 09c is the app's test tree only. File list below.
Order: first, together with 09c (two helpers), then 09b. 09b's NOTES quotes this ticket's new `window` line for its "read every tip in view" question.
Build with: Opus. Pruning which candidates get measured is reasoning over R1/R2 and over git's `--merged`/`--no-merged`/`--contains` meaning. A slip silently changes a route. The equivalence test below pins the result to today's, so correctness is judged by a test, not by eye. That is why this is not Fable.
Needs: ops `branch_status`, `route`, `choose_base`, `Candidate`, `CandidateKind`, `Apart`, `ROUTE_DEPTH` (ops MAP `## branch_status`); git `compare_span`, `contained_in` (git MAP `## contained`, `## history_reads`); `run`'s typed `GitArg`.

## The number
`docs/perf/branch-pane.txt` (team MONOREPO, release; the file's header says `HEAD ffb014bd +uncommitted`):
- `candidates=1231 local=331 remote-only=900`
- `focus runs=10 p50_ms=14394.6 p95_ms=14722.7 spawns=2461`
- `window rows=40 tips=21 total_ms=1470735.1 spawns=254441`

2461 is about 2 × 1231 spawns per focus, so the reads appear to scale with the candidates. **That is an inference from the totals. Nobody has seen which reads they are**, so step 1 checks it first.

## Read first
- `.scratch/ease-pass/issues/09-branch-pane.md`: rulings R and M3/M4, NOTES continuation 2 "Perf".
- `.scratch/ease-pass/issues/01r-roles-and-route-fixes.md` "The rules, corrected" (R1–R3) — the rules must not change.
- `crates/mergewright-ops/MAP.md` `## branch_status`; `crates/mergewright-git/MAP.md` `## contained`, `## history_reads`, `## run`.
- `scripts/branch-pane-cost.sh` (whole; short).

## Step 1 — settle first: which reads scale
- Extend the script's `git` shim to log its subcommand (`$1`), not only `spawn`.
- Add one line to the output: `branch-pane-cost: breakdown <subcommand>=<n> …`, for one focus.
- Run it on main before any change. Quote that line in NOTES.
- **If** the reads that scale with candidates are not the base measure (R1) and the holder search (R2), stop and report the line. The design below assumes those two. Don't build around something else.

## Decided — measure only what can win; never narrow what he sees
Every candidate stays a candidate (09 R). The rules are unchanged. Only candidates that the rules prove can never be chosen go unmeasured.

**R1, at each route step with walker `w`:**
1. Measure the trunk `T` as today.
   - `T` is *able to win* when it is allowed, not already walked, shares history with `w` (R1 skip), and `branch_only > 0`.
2. `exclude = T.tip` if `T` is able to win, else none.
3. Integration contenders: integration candidates (if allowed) whose tip is **not** reachable from `exclude` (`--no-merged exclude`). With no exclude, every integration candidate.
4. Other contenders (if allowed): Other candidates whose tip **is** reachable from `w` (`--merged w`) and not from `exclude`.
5. Measure the contenders as today. `choose_base` runs over `T` plus the contenders. `choose_base` is unchanged.

Why it is exact:
- An Other needs `base_only == 0`, which holds exactly when its tip is reachable from `w`.
- Any non-trunk `c` reachable from `T` has `reach(c) ⊆ reach(T)`, so `branch_only(c) ≥ branch_only(T)`. On a tie the kind order picks Trunk. So such a `c` can never beat a `T` that is able to win. That is why the exclusion applies only when `T` is able to win.

**R2 holders:** one `for-each-ref --contains <branch tip>` lists every holder. Intersect it with the candidates, then run the rule as today in its order. R2 case 2 still runs its per-holder `entry_into`, over the holders only.

**git:** these filters are 2-3 `for-each-ref` spawns per route step, whatever the candidate count.
- I have **not checked** that git 2.38 (the minimum, PROMPT.md:51) takes `--merged` and `--no-merged` together. The `refs_where` test pins it on the system git and notes the version.
- If that test shows 2.38 can't combine them, ask `--merged w` alone, filter out `exclude`'s reach with a second spawn (`--merged exclude`), and subtract. Still a fixed number of spawns.
- One code path either way.

**Rejected:**
- `%(ahead-behind:)` batching needs git 2.41, above the minimum, so it would mean two paths. Held in reserve: use it only if open integration branches dominate after this fix (step 1's breakdown, run again, shows it).
- An in-process gix reachability walk duplicates git's commit-reach and is structural.
- Caching by repo state is already there (09 M3: cleared on refs). It does not help the first focus, which is the 14 s.
- Bounding candidates narrows what he sees. That is his call (Question below).

## Signatures
mergewright-git, module `contained` (beside `contained_in`, which stays as it is):
```rust
pub struct RefFilter<'a> {
    pub patterns: &'a [&'a str],        // "refs/heads", "refs/remotes"
    pub merged_into: Option<Oid>,       // --merged <oid>: tip reachable from it
    pub not_merged_into: Option<Oid>,   // --no-merged <oid>
    pub contains: Option<Oid>,          // --contains <oid>
}
pub struct RefTip { pub name: RefName /* full */, pub tip: Oid }
/// One `for-each-ref --format=%(objectname) %(refname)` spawn, uncapped. Symrefs are listed as git lists them;
/// callers filter to their own set.
pub fn refs_where(repo: /* as contained_in */, filter: &RefFilter<'_>, stop: /* as contained_in */)
    -> Result<Vec<RefTip>, RunError>;
```
mergewright-ops: **no public signature changes.** `branch_status`, `route` and `choose_base` keep their signatures, and the app is not touched. Private contender selection, name free:
```rust
fn contenders<'c>(repo: …, walker: Oid, candidates: &'c [Candidate], at_most: CandidateKind,
                  trunk_able: Option<Oid>, stop: …) -> Result<Vec<&'c Candidate>, RunError>;
```

## Scope
In:
- git `refs_where` and its tests:
  - `--merged` alone, `--no-merged` alone, and both together;
  - `--contains`;
  - a ref name with `/` and with non-ASCII, read back exactly.
- ops: contender selection in the route/R1 step and the R2 holder search; ops MAP `## branch_status` gets one line saying what gets measured.
- `examples/branch_pane_cost.rs`:
  - the candidates line gains `trunk=<n> integration=<n> other=<n>`;
  - the focus run adds the `breakdown` line;
  - a new **scaling** line, on a small repo the example builds in a temp dir: sara on a branch, plus N stale Others (tips on main) and N sibling Others (off main, not under sara), at N=10 and N=500 → `branch-pane-cost: scaling others=10 spawns=<a> others=500 spawns=<b>`.
- `scripts/branch-pane-cost.sh`: the shim logs the subcommand, and a budget line `branch-pane-cost: budget ok` or `branch-pane-cost: budget OVER <which>`. The script exits 1 on OVER.

Out:
- the app (`jobs::branch_status`, cache, `StatusKey`);
- the rules R1–R3;
- 09b's tip labels;
- any cap on candidates.

## Budget — asserted by `scripts/branch-pane-cost.sh`, release, team MONOREPO
- `focus`: p50_ms ≤ 1000, p95_ms ≤ 2000, spawns ≤ 60.
- `window`: total_ms ≤ 1000 × tips, spawns ≤ 60 × tips.
- `scaling`: `a == b`. The spawns do not grow with candidates that cannot win.

Why 1 s: the teammate journey is 0 keys, just resting on his row (`ease-pass.md:106`). The answer has to land while he is still looking, not after he has moved on. 1 s is Nielsen's limit for flow (Usability Engineering, 1993, ch. 5). It is not in `docs/research/ux-principles.md` yet. NOTES proposes adding it there, and the builder does not edit that file.

If the build as specified misses the budget, **stop**. Report the breakdown and the numbers. Don't tune, cap or special-case.

## Question for the user (only if the budget is missed after this fix)
Teammates' branches you never checked out (900 on the team repo) are what make the pane slow. Options:
- (a) keep them all and wait longer for `reading…` to clear;
- (b) count only those with a commit in the last N days;
- (c) count only those whose tip is in the history loaded on screen.

## Files (for disjointness)
- `crates/mergewright-git/src/contained.rs` (or `contained/` and its tests), `crates/mergewright-git/src/lib.rs` (re-export only, if `contained` re-exports), `crates/mergewright-git/MAP.md`
- `crates/mergewright-ops/src/branch_status/*` and `branch_status/tests/*`, `crates/mergewright-ops/MAP.md`
- `crates/mergewright-ops/examples/branch_pane_cost.rs`
- `scripts/branch-pane-cost.sh`, `docs/perf/branch-pane.txt`

No app files, no captures.

## Touches
- `branch_status` / `route` callers: the app's `jobs::branch_status` and any other (grep). Signatures unchanged, results equal by the test below.
- 14's `g12_…` assertions and 09's `reads.rs`: results must be identical. Show it with the full nextest.
- `contained_in`: untouched (grep its callers).
- 09b's widening question: the new `window` line.
- Journal and undo: none (reads only).

## Acceptance
Each tick names its test inline.
- [x] Step 1 (scratch spawn run with the new shim; quoted in NOTES): the `breakdown` line from before the change is quoted in NOTES. The reads that scale are named.
- [x] Equivalence (`equivalence::stale_and_sibling_others_answer_as_the_oracle`; `equivalence::agrees` called from the `status()`/`route_of()` helpers and every direct `route` test in `tests/route.rs`): on every 01r topology in ops' `branch_status` tests, plus a fixture with stale and sibling Others, the route and the full `BranchStatus` equal what a test oracle gives by measuring **every** candidate (the oracle is `#[cfg(test)]` and calls today's measure for each candidate).
- [x] `refs_where` git tests (`contained::tests::refs_where_merged_alone`, `refs_where_not_merged_alone`, `refs_where_merged_and_not_merged_together`, `refs_where_contains`, `refs_where_reads_names_and_tips_back_exactly`): each filter alone, `--merged`+`--no-merged` together (system git version in NOTES), `--contains`, names read back exactly.
- [ ] The script exits 0 with `budget ok`. **Not met: window OVER (spawn run), see NOTES.** NOTES quotes the `candidates`, `focus`, `window`, `scaling` and `breakdown` lines as read from `docs/perf/branch-pane.txt`.
- [x] `scaling`: `a == b` (example `scaling` line, spawn run: 6 and 6).
- [ ] Full nextest green. **Not met: 2 `graph_mode_keep` captures fail, question in NOTES.** No existing branch-pane or G12 test changes its expected words (none changed: the edits in `tests/route.rs` and `tests/status.rs` only add `agrees` calls).

## NOTES (builder, 2026-10-19)

Scout: none. Every file was in the spec's list, and I read each one in full myself.

### Step 1: before the change (spawn run, newest local feature as focus)
`branch-pane-cost: breakdown merge-base=1 rev-list:--count=2459 rev-list:--reverse=1`
What scales is `rev-list --count`, which is `count_between` in the old `measure`. That one function feeds both R1 (the base) and R2 (the holders), so the premise holds.

### After the change. Spawn run only; the timed run is the conductor's, and docs/perf/branch-pane.txt is not rewritten
- `candidates=1231 local=331 remote-only=900 trunk=1 integration=0 other=1230`
- `focus runs=10 p50_ms=78.3 p95_ms=88.0 spawns=8`. Times are from the shimmed run on a loaded machine: indicative only.
- `breakdown for-each-ref=3 merge-base=1 rev-list:--count=3 rev-list:--reverse=1`
- `window rows=40 tips=21 total_ms=2522088.3 spawns=182681`. **OVER**: the budget is 60 × 21 = 1260; it was 254441 before.
- `scaling others=10 spawns=6 others=500 spawns=6`

The focus is in budget, the window is not. I stopped as the spec says: no tuning.

### Why the window is still slow: suspected from the code, not observed per tip
- `able_trunk` (measure.rs:197) needs `branch_only > 0`. A tip that main holds (merged, or on main's line) has nothing to exclude. So `ordinary_contenders` (measure.rs:237) measures every Other under it: about 1000 of them, 2 counts each.
- For an on-line walker, R1 is never consulted: branch_status.rs:206 and status.rs:171 step straight to the line. Those counts are waste.
- For a merged walker, R1 runs, so its Others are real contenders.

Two exact fixes, both outside this spec:
1. Decide R2 case 1 first. Measure R1 contenders only when the walker is not on a line.
2. Among Others under the walker, only tips that no other contender's tip reaches can win: one `merge-base --independent`. If c is strictly under c', then branch_only(c) > branch_only(c'), because c''s tip is in the walker and not in c.

### git
- The system git is 2.54.0 (Apple Git-157). `--merged` and `--no-merged` combine as "from M, not from N" (`refs_where_merged_and_not_merged_together`). Not tested on 2.38 itself.

### Deviations
- "T able to win" leaves out the spec's shared-history check. It is still exact: if T is unrelated to w, so is every c under T, and R1's skip drops each c anyway. One `compare_span` spawn saved.
- A candidate whose ref is missing, or not at the candidate's tip, is always measured, at one extra `for-each-ref` per step. Reason: the filters speak about refs, not about the caller's tips.

### Full nextest (7091 tests): 7088 passed, 3 failed
- `map_coverage::map_lines_are_short…`: my ops MAP line was over 240 characters. Fixed; it passes.
- `graph_mode_keep::picking_a_graph_mode_in_help_switches_keeps_and_reopens_in_it` and `typed_graph_lists_the_group_after_here_at_120_and_90`: the capture has `◆─╮` committed where `◆┄╮` is drawn (graph-mode-switched-120.txt line 5; help-graph-mode-120.txt line 8). **Question, not a defect claim.** A temporary log inside the app run (reverted) recorded `wallet-release same=true route=[sara/fix-toc, main]` against the every-candidate oracle, so the answers are unchanged there. I could not run these tests on the baseline sources: the permission classifier refused it. App files are outside my set.

### Touches
- `route` and `branch_status` callers (the app's `jobs::branch_status`): signatures unchanged.
- `contained_in`: untouched.
- 14's G12 and 09's reads: green, apart from the 2 captures above.
- Journal and undo: none (reads only).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide. Plus `scripts/branch-pane-cost.sh` (a measurement: run from the script, read the summary lines only).

## Size
~50k.
