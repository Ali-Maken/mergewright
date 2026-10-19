# branch-pane-cost — the pane answers in under a second on his monorepo

Status: draft (planner spec-only pass 2026-10-19, from 09's Perf NOTES at 59b290d1)
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
- [ ] Step 1: the `breakdown` line from before the change is quoted in NOTES. The reads that scale are named.
- [ ] Equivalence: on every 01r topology in ops' `branch_status` tests, plus a fixture with stale and sibling Others, the route and the full `BranchStatus` equal what a test oracle gives by measuring **every** candidate (the oracle is `#[cfg(test)]` and calls today's measure for each candidate).
- [ ] `refs_where` git tests: each filter alone, `--merged`+`--no-merged` together (system git version in NOTES), `--contains`, names read back exactly.
- [ ] The script exits 0 with `budget ok`. NOTES quotes the `candidates`, `focus`, `window`, `scaling` and `breakdown` lines as read from `docs/perf/branch-pane.txt`.
- [ ] `scaling`: `a == b`.
- [ ] Full nextest green. No existing branch-pane or G12 test changes its expected words.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide. Plus `scripts/branch-pane-cost.sh` (a measurement: run from the script, read the summary lines only).

## Size
~50k.
