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

### Window cause: confirmed by observation (conductor's ruling 1)
A temporary per-tip line in the example (not committed) logged, for each of the 21 window tips: its spawns, a breakdown, whether main holds it (`merge-base --is-ancestor`), the refs under it, and its answer. Each count below includes 3 probe spawns.
- 10 tips main does not hold (`feature/03743`, `03763`, `03771`, `03775`, and their `origin/`): `spawns=11` to `13`, `merged=NotYet`, route `[main]`.
- `main`: `spawns=4`.
- 10 tips main holds (`feature/03744`, `03752`, `03754`, `03759`, `03760`, `03766`, and their `origin/`): `spawns=12808` to `17986`, of which `rev-list:--count` is 12772 to 17948, plus `for-each-ref=26`. All read `merged=Into(main)`, and each route walks 8 ordinary bases deep (`feature/03640`, `feature/03625`, …). Example: `feature/03744 main_holds=true refs_under=950 spawns=14268 … rev-list:--count=14232`.
So the cause is as suspected. The tips main holds have no able trunk (measure.rs `able_trunk`), so each step measures every Other under the walker (`ordinary_contenders`), about 900 to 1100 refs at 2 counts each. That happens on every step of an 8-deep walk.

### Fix chosen: 2, the `merge-base --independent` tops
- Fix 1 (skip R1 for on-line walkers) is exact, but the heavy tips are not on a line: they are `Into(main)`, and their walked bases are merged features too. It would not touch these counts.
- Fix 2 is exact, with the proof in the measure.rs module doc. If contender c's tip is strictly under contender c''s, then c' is eligible: ordinary, under w, not at w's tip, so `branch_only > 0`. It is never passed over by R1, since it shares history. And `branch_only(c) > branch_only(c')`, because c''s tip is in w and not in c. So c is never chosen. One `merge-base` spawn per step. Nothing the user sees is narrowed: candidates are unchanged, and only counts that cannot win are skipped.
- Tests: `equivalence::features_merged_one_after_another_answer_as_the_oracle` (the team shape: main holds a chain of features; f1 and stale/f1 go unmeasured from f3) and `only_what_can_win_is_measured` (`stale/w1` is now under `stale/s1`, so it goes unmeasured). git: `contained::tests::independent_tips_drop_what_another_reaches`.

### After fix 2 (spawn run; times are shimmed and the machine was loaded)
- `focus runs=10 p50_ms=86.1 p95_ms=91.6 spawns=8`
- `window rows=40 tips=21 total_ms=34526.1 spawns=3337`. **Still OVER**: the budget is 1260 spawns (and 21000 ms). Down from 182681.
- `scaling others=10 spawns=6 others=500 spawns=6`
- Arithmetic, not observed per tip: the 11 light tips cost about 90, so each of the 10 tips main holds still costs about 320 over its 8-step walk, roughly 40 per step. Stopped here, per the spec. The next step is the user question (a/b/c above), or a design decision on the walk.

### git
- The system git is 2.54.0 (Apple Git-157). `--merged` and `--no-merged` combine as "from M, not from N" (`refs_where_merged_and_not_merged_together`). Not tested on 2.38 itself.

### Deviations
- "T able to win" leaves out the spec's shared-history check. It is still exact: if T is unrelated to w, so is every c under T, and R1's skip drops each c anyway. One `compare_span` spawn saved.
- A candidate whose ref is missing, or not at the candidate's tip, is always measured, at one extra `for-each-ref` per step. Reason: the filters speak about refs, not about the caller's tips.

### Full nextest (7091 tests): 7088 passed, 3 failed
- `map_coverage::map_lines_are_short…`: my ops MAP line was over 240 characters. Fixed; it passes.
- `graph_mode_keep::picking_a_graph_mode_in_help_switches_keeps_and_reopens_in_it` and `typed_graph_lists_the_group_after_here_at_120_and_90`: the capture has `◆─╮` committed where `◆┄╮` is drawn (graph-mode-switched-120.txt line 5; help-graph-mode-120.txt line 8). A temporary log inside the app run (reverted) recorded `wallet-release same=true route=[sara/fix-toc, main]` against the every-candidate oracle, so the answers are unchanged there.
  - **Cause, observed: a race in the test, which my branch's timing exposes.** `┄` is `join_glyph(LaneRole::Other)` (mergewright-ui `graph_column/roles.rs`): the lane drawn before the route has landed. The committed capture shows it after the read (`─`, Under).
  - The tests capture on the first screen that shows `every branch shown` + `kept for next launch` (graph_mode_keep.rs, the `switched` wait_for), or `the folded graph` (`folded_and_rested`). Neither waits for the branch read. As a probe I added `&& s.has("up to date with sara/fix-toc")` to the `switched` wait (temporary, reverted), and the test **passed**.
  - Why my branch is later, inferred and not measured: on a tiny repo the new measure adds 2 to 4 `for-each-ref` (and now a `merge-base`) per step, where the old one needed only a few counts. So the read lands after the capture point.
  - The full nextest after fix 2 (7093 tests) gave 7092 passed, 1 failed. Only `typed_graph_lists_the_group…` failed; `picking_a_graph_mode…` passed that run. That is consistent with a race and not with a fixed result.
  - Fix (cause): those waits should also wait for the pane read to land. That edit is in `crates/mergewright/tests/it/graph_mode_keep.rs`, which is outside my set, so I am **blocked on that file**. The committed captures stay as they are; no redraw is needed.

### Touches
- `route` and `branch_status` callers (the app's `jobs::branch_status`): signatures unchanged.
- `contained_in`: untouched.
- 14's G12 and 09's reads: green, apart from the 2 captures above.
- Journal and undo: none (reads only).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide. Plus `scripts/branch-pane-cost.sh` (a measurement: run from the script, read the summary lines only).

## Size
~50k.
