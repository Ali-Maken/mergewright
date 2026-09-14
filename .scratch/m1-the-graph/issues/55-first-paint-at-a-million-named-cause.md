# m1-the-graph/55 — First paint at a million commits: name the cause by measurement, then fix it

Status: done (Phase A; Phase B fix 1 and fix 2 — 1M first paint 98–105 ms warm with jobs)
Build with: **Opus** (high) — phase instrumentation inside the walk and the app's open path, then a fix in the rev-walk or the first-window path; correctness is numbers plus the existing order tests, so not Fable.
Needs: `Walk::next_links`, `CommitTable` (commit-handles-and-table); the walk reading the graph with dense state and its odb-read counter (walk-reads-the-commit-graph); rows hydrating on demand (rows-hydrate-on-demand **and rows-pass-through**, split 2026-09-14). Runs after all of them: their numbers are this ticket's baseline.
Blocked by: 49, 50, 52, 52b (rows-pass-through)

Why (user, 2026-09-14, ADR-0026 "User decisions on approval"): "The 1M first-screen miss gets its own ticket with a named cause, not just a number in a report." m1/46 measured 292–383 ms at 1M (100k: 26–85). ADR-0026 *names* the commit-graph path's in-degree pass as the likely cause; that is a hypothesis. **Both paths miss by about the same amount** (cg 292–346, no-cg 297–383), and the no-cg path has no in-degree pass before its date-order first window — so at least part of the cause is common to both paths and unnamed. This ticket names it by measurement, fixes what the design allows, and re-measures.

Budgets (ADR-0026, accepted): 1M is judged against §6.3's kernel-class **500 ms** and reported against **150 ms**. The 100k heavy-tier assertion at 150 ms stays.

## Read first
- ADR-0026 "Context" (first-paint sentence), "The gate at 1M" (first-paint bullet — the provisional-paint option "a later decision, flagged here, not taken"), "User decisions on approval"
- ADR-0008 (all); DESIGN.md §3.2 step 2 (both bullets; "This path is what meets the 150 ms first paint")
- Ticket 46 "Curve" table (first paint and "RSS at first paint" columns), ticket 27 lines 54–64 (what `budget_run` times and excludes), ticket 50 "Acceptance" (the first-paint finding it wrote, the odb counter)
- KNOWN_GAPS rows "First paint at 1M commits is 292–383 ms", "The first `advance` is a full 500-row chunk"
- `crates/graft-git/MAP.md` (`history`, `branches`, `repo`), `crates/graft/MAP.md` (`repos`, `repository`, `history` `stream`/`serve`, `refs`, `jobs`), `crates/graft-graph/MAP.md` (`layout::stream`)
- `crates/graft/examples/budgets.rs`: a scout summarises what it times and which jobs run concurrently during first paint (one page).

## Scope
### Phase A — name the cause (no fix yet)
- `examples/budgets` gains `--phases`: wall time, odb objects read and RSS delta per phase, serially on the loop's own thread, no concurrent jobs: `open` (`SharedRepo::open` + `facts`) · `refs` (`references`, `commit_tips`, `Branches::resolve`, `seeds_for`) · `table` (`CommitTable::open`) · `walk_new` (`Walk::all_refs` returns) · `first_link` (first `Link` out of `next_links`) · `first_chunk` (first `advance` returns) · `first_window` (rows sent). Then the same run with the jobs that normally start at open (`ReadRefs`, `ReadWorkingTree`, system git) enabled, to see contention.
- Run on the 1M small fixture, cg and no-cg, release, 3 runs each; 100k both paths once for the slope. Output to `target/<ticket>/phases.txt`.
- **Name the cause:** the phase (or concurrent job) holding ≥ 40 % of first paint on each path, with a per-commit cost (ns/commit, from 100k → 1M) and the mechanism — what runs once per commit and why (e.g. "the in-degree pass visits every graph commit down to the oldest tag's generation", "`Branches::resolve` rule 4 counts first-parent merges over the whole history", "gix parses every tip's commit for the date queue", "the first `advance` lays out 500 rows"). Confirm with one discriminating change (disable or stub it; the number drops by its share). Rank the hypotheses above; measure, don't guess.
- Write the phase table and the named cause(s) into this ticket before touching the fix. **Seam: stop here with a handoff note if over ~40k tokens.**

### Phase B — fix what the design allows, re-measure
- Anything the first window does not need moves after it or into a job that already exists (refs/branch work, counting, the working-tree read), or is bounded (the first `advance` sized to the viewport plus `buffer_rows`, then the usual chunk). Same emitted order, same rows: every order and snapshot test unchanged.
- Re-measure 1M cg and no-cg (3 runs each) and 100k both paths; write before/after in this ticket, TESTING.md's curve section (one row: first paint per cell), and close or rewrite the two KNOWN_GAPS rows with the numbers.
- **If the named cause on the cg path is the strict-topo in-degree pass itself** (inherent to emitting strict order before the oldest tip's generation) and 1M cg is still over 150 ms after everything else is fixed: **do not build the fallback.** Write the number, the cost per commit, and the recommendation below into the ticket and stop. It changes DESIGN.md §3.2's sentence "This path is what meets the 150 ms first paint" and needs the user's OK at the checkpoint.

### Pre-decided design for the fallback (built only on the user's OK, as its own ticket)
Date-order provisional paint plus promotion on the commit-graph path too — ADR-0008's no-cg machinery, already built (`WalkFacts::is_provisional`, `OrderNotice::Sorting`, `PromoteHistory`, `Views::promote`), gated by a deterministic count, never by elapsed time:
```rust
// graft-git::history
pub const DIRECT_TOPO_LIMIT: u32;   // graph commits the in-degree pass may visit before the first emission; above it the walk starts in date order, provisional, and promotion runs. Value from the measured ns/commit so the pass fits ~100 ms; the measurement in its doc comment.
impl Walk {
    pub fn pass_depth(&self) -> Option<u32>;   // commits with generation ≥ the lowest tip's generation: one scan of the graph's generation column; None without a graph
    pub fn all_refs_with_limit(repo: &RepoHandle, order: Order, table: Arc<CommitTable>, limit: u32) -> Result<Self, WalkError>;   // `all_refs` = this with DIRECT_TOPO_LIMIT
}
```
`full_topological` on the cg path reads the graph (ticket 50), so promotion is the 1M `G` number, not a full object walk. Test: the 5k cg synthetic pair with `limit = 100` goes provisional and the promoted order equals the direct topo order; `limit = u32::MAX` never goes provisional. Cost to the user: on a kernel-sized repo with a graph, one reshuffle a few seconds after open (the notice says so), instead of a 300 ms blank.

Out: memory (memory-curve-acceptance); the walk's emitted order; building the fallback without the user's OK; the 100k assertion's limit.

## Signatures
No new public items in Phase A beyond `examples/budgets --phases`. Phase B's are whatever the named cause needs; if a public item changes, its MAP.md line changes with it and the reviewer checks the order tests. The fallback's signatures are above, for the follow-up ticket.

## Acceptance
- [x] Phase table in this ticket: 7 phases × {1M cg, 1M no-cg} × 3 runs, plus 100k both paths, plus the concurrent-jobs run; `target/<ticket>/phases.txt` referenced.
- [x] Each path's cause named with mechanism, ns/commit and the discriminating measurement that confirmed it. "The in-degree pass" is accepted only with its measured share; "something linear" is not a name.
- [x] After the fix: 1M cg and no-cg first paint < 500 ms on every run (the judged budget); each number written against 150 ms with yes/no and, for a miss, the residual's named cause.
- [x] 100k both paths still ≤ 150 ms in `heavy::budgets_hold_*`; every `graft-git` order test and every corpus snapshot unchanged.
- [x] (n/a — residual is not the in-degree pass, ~1 ms) If the residual is the in-degree pass: the recommendation section filled in with numbers; no fallback code in the diff.
- [x] TESTING.md curve section and KNOWN_GAPS rows updated with numbers; MAP.md for any crate whose public items moved.
- [x] Report: cause vs symptom stated per fix; what was measured and what was not (a real kernel clone is not required; say so).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens: 1M runs are ~10 s each. Seam between Phase A and Phase B; stop there with a handoff note rather than compacting.

## Phase A — measured (2026-09-14, release, `budgets --phases`; raw: `target/55/phases.txt`)

Serial, one thread, no jobs, warm cache. ms; RSS = rise in process peak.

| Phase | 1M cg (3 runs) | 1M no-cg (3 runs) | 100k cg | 100k no-cg |
|---|---|---|---|---|
| `open` (`SharedRepo::open` + `facts`) | 0.3–2.3 | 0.3–3.0 | 3.5 | 0.9 |
| `refs.references` | 68–120 | 66–68 (525 cold) | 53 (cold) | 53 (cold) |
| `refs.commit_tips` | 83–85, **+145 MiB** | 83–91, +145 MiB | 8, +15 MiB | 60 |
| `refs.resolve` (`Branches::resolve`) | 65–66 | 65–67 | 5.8 | 5.9 |
| `refs.seeds_for` | 0.25 | 0.25 | 0.03 | 0.03 |
| `table` (`CommitTable::open`) | 0.3 | 0.02–0.04 | 0.2 | 0.02 |
| `walk_new` (`Walk::all_refs`) | 72–75, +53 MiB | 82–90 | 11.8 | 7.3 |
| `first_link` (fresh walk) | 0.05–0.08 | 0.00 | 0.01 | 0.00 |
| `first_chunk` (500 rows) | 2.2–2.3 | 1.7–3.2 | 3.2 | 2.1 |
| `first_window` (40 rows) | 0.01 | 0.01 | 0.01 | 0.01 |
| **total** | **291–347** | **299–300** (779 cold) | 85 | 130 |

Refs: 6,498 at 1M, 650 at 100k (3,357 `refs/heads`, 3,141 `refs/tags` at 1M), **all loose files, no `packed-refs`**.

With jobs (`budgets`, real `JobRunner` + `update`): 1M cg first paint **255 ms**, 1M no-cg **270 ms**. Contention adds nothing measurable: the history job alone does `resolve` + `references` + `all_refs` ≈ 65 + 68 + 75 = 208 ms serial; `ReadRefs` enumerates them a fourth time on another worker.

Discriminators (same file):
- `Walk::from_tips` with the tips already read: **1.0–1.35 ms** vs `walk_new` 72–79 ms. `all_refs` is ~98 % its own `commit_tips()`; the in-degree pass itself (6,497 graph reads) is ~1 ms.
- `references()` called again right after: 72 ms. Nothing caches; each enumeration pays in full.
- Same 1M cg repo, refs packed (`cp -c` clone + `git pack-refs --all`, `target/55/packed-1M-cg`): `references` **1.0 ms**, `resolve` 1.1, `commit_tips` 13–17, `walk_new` 5–6 (54 cold); serial total **24 ms**; **with jobs first paint 23 ms** (was 255). Same commits, same graph, same walk.

### Named cause (both paths — the same one)

**Loose-ref enumeration, repeated three times on the history job before the first window.** gix's `references().all()` opens and reads one file per loose ref: **~11 µs/ref** (72 ms / 6,498; 1.0 ms packed = 0.15 µs/ref). The first-paint path enumerates every ref in `Branches::resolve` (`RepoStore::branches` → `references()`), again in `history::stream` (`references()` for seeds), and again in `Walk::all_refs` (`commit_tips()`, which also `find_header`s each tip in the pack, ~2 µs/tip). ≈ 3 × 72 ms ≈ 210–230 ms of the 255–300 ms: **~85 % on cg, ~80 % on no-cg**; the rest is the first chunk (2 ms), the job hops and `commit_tips`' header lookups.

It is **per ref, not per commit**: the fixture script adds one ref every ~154 commits, which is why it looked linear in history (1M → 290 ns/commit, 100k → ~800 ns/commit cold; per ref it is flat, ~33–45 µs/ref across the three enumerations). The in-degree/sort hypothesis (ADR-0026) is refuted again: the whole pass is ~1 ms.

Secondary, memory (not this ticket's budget, named for memory-curve-acceptance): `commit_tips` raises peak RSS by **145 MiB at 1M**, `walk_new` by 53 MiB — `find_header` on 6,497 scattered tips faults in pack/index pages (mapped, counted in RSS). That is the unexplained "RSS at first paint 162–219 MiB"; it is present packed too (145.6 MiB), so it is the header lookups, not the ref files.

Realism: `git clone` writes `packed-refs`, so a fresh kernel clone takes the 23 ms path; thousands of **loose** refs are what a repo has after many fetches between `gc`s. Not measured on a real kernel clone.

### Hypotheses ranked
1. Repeated loose-ref enumeration — **confirmed** (packed: 255 → 23 ms).
2. `commit_tips` header lookup per tip — minor for time (~13 ms packed), major for RSS (+145 MiB).
3. `Branches::resolve` rule 5 (first-parent merge count) — **not run**: fixture has `main`; chosen by `InitDefaultBranch`; its 65 ms is only its `references()` call.
4. Strict-topo in-degree pass — **refuted**: ~1 ms.
5. First `advance` of 500 rows — 2 ms; not a cause. Sizing it to the viewport is not worth doing for time.

## Handoff (Phase A → Phase B)

Done: `budgets --phases` (`crates/graft/examples/budgets/phases.rs`, wired in `budgets.rs`); table and cause above; no source outside the example touched.

Next exact step, Phase B (fix the cause: read refs once on the first-paint path; emitted order untouched):
- One ref enumeration serves `Branches::resolve`, `seeds_for` and the walk's tips. Shape to decide (a public item changes, MAP.md with it): e.g. `RepoStore` reads `peeled`/`references` once and caches it next to `branches` (it already caches `Branches` and the table); `Branches::resolve_from(handle, &refs, override)`; `history::stream` calls `Walk::from_tips(handle, order, &tips, table)` with tips derived from that one reading instead of `Walk::all_refs`. Tips need peeling (tags → commits): either `commit_tips` returns refs + tips from one pass, or derive tips from one `peeled_references()` pass plus `HEAD` (check `commit_tips`' dedup/sort and detached-`HEAD` semantics — tips must be byte-identical or the order tests move).
- Expected: 1M ≈ 72 (one enumeration) + ~15 (headers) + ~5 ≈ 90–100 ms with jobs, under 150 on both paths. `ReadRefs` still enumerates once more on its own worker (concurrent, not on the history job's path); leave it.
- Re-measure with `target/release/examples/budgets --phases <fixture>` and plain `budgets <fixture>` (fixtures: `target/fixtures/synthetic-1000000-commit-graph-511a25b425758b13`, `…-no-commit-graph-8ac5be635bb9d262`, `synthetic-100000-commit-graph-0fb9210ab6ca6ac4`, `…-no-commit-graph-024d506de281597a`).
- Gotchas: first run after a build is cold (`references` 120–525 ms) — discard or report separately; `budgets <repo> | head -1` panics on broken pipe after printing (harmless, redirect instead); the ~145 MiB RSS at `commit_tips` is header lookups, not refs — a fix that keeps `find_header` per tip keeps it.
- Not for this ticket: the RSS rise; the provisional-paint fallback (not needed — the in-degree pass is ~1 ms).

## Phase B, fix 1 — refs read once on the history job (2026-09-14; raw: `target/55/phaseB.txt`)

**Fix (cause):** `RepoHandle::ref_snapshot` reads the ref store once and returns `references()` and `commit_tips()` exactly (same iteration, same `tip_of`, same `HEAD`/sort/dedup; `commit_tips` now delegates to it; test pins both halves loose and packed). `RepoStore::refs` caches it; `RepoStore::branches` uses `Branches::resolve_from(handle, &refs, override)`; `history::stream` takes seeds and `Walk::from_tips(…, &refs.tips, …)` from it (the second view too). Emitted order untouched: tips are byte-identical by construction; all order and snapshot tests pass unchanged.
**Refresh race closed with it:** `refresh` is now a lock-free generation bump, called when `OpenRepository` is *dispatched* (loop thread), not inside the job. Before, a history job starting ahead of the open job on another worker could get pre-`Ctrl+R` branches; with refs cached it would also have got stale tips. Cost to the user: none measured; `Ctrl+R` never serves refs from before the keypress.

Serial `--phases`, release, warm (first run per path discarded):

| Phase | 1M cg (3) | 1M no-cg (3) | 100k cg | 100k no-cg |
|---|---|---|---|---|
| `refs.ref_snapshot` | 91–94, +147 MiB | 84–89, +147 MiB | 102 (cold) | 109 (cold) |
| `refs.resolve_from` | 0.08 | 0.08 | 0.01 | 0.02 |
| `walk_new` (`from_tips`) | 3.0–3.3 | 13.4–13.6 | 8.1 | 1.9 |
| **total** | **97–101** (was 291–347) | **100–105** (was 299–300) | 114 (cold) | 114 (cold) |

With jobs (`budgets`, first paint): 1M cg **191–195 ms** (was 255), 1M no-cg **193–203 ms** (was 270), 100k cg 20.5, 100k no-cg 21.1. Against 500 (judged): pass every run. Against 150: **miss**, residual named below.

**Residual named (discriminator, same file):** `ReadRefs` (`refs::read` → `peeled_references`) enumerates all 6,498 loose refs and `find_header`s 6,497 tips on another worker *concurrently* with the history job's read. Disabled (temporary local edit, reverted, not committed): 1M cg **98 / 98 ms**, no-cg **104 / 103 ms** (cold first runs 146 / 452 discarded). So the ~90 ms residual is contention with the fourth enumeration, not the history path.

## Handoff (Phase B fix 1 → fix 2)

Done and committed: fix 1 above, MAP.md (graft-git: `RefSnapshot`, `ref_snapshot`, `resolve_from`; graft: `RepoStore::refs`, `refresh`, `StoreError::Refs`), `phases.rs` mirrors the new path. Gates: fmt, clippy, clippy heavy-tests, `cargo test` 841/0.

Next exact step (fix 2, same cause — "read refs once and reuse"):
- Add `peeled: Vec<(RefName, Oid)>` to `RefSnapshot`, filled in the same loop from `tip_of` (sorted by name, refs with no commit left out — exactly `peeled_references`); make `peeled_references` delegate; extend `snapshot_is_both_halves` in `graft-git/src/repo/tests.rs`.
- `refs::read` (`crates/graft/src/refs/mod.rs:159`) takes `store.refs()?.peeled` instead of `handle.peeled_references()`. It then waits on the store's `refs` mutex while the history job reads (or vice versa): one read total.
- Re-measure `budgets` 1M cg/no-cg ×3, 100k both; expect ~100 ms with jobs. Check `tests/it/many_refs.rs` (50k refs, `refs::read` timing, heavy) still holds.
- Then: TESTING.md curve section (one first-paint row per cell), KNOWN_GAPS rows (156 and the "1M first paint is not the walk" note, and 177 "50k refs slow first paint in debug" — its "shared ref read" decision is now made), heavy tier once at the end.
- Gotchas: `dispatch` in `jobs/mod.rs` is at clippy's 60-line limit — add arms as methods. First run after a build is cold; discard it. The +147 MiB peak at `ref_snapshot` is `tip_of`'s `find_header` on scattered tips (pack pages), not the ref files — left for memory-curve-acceptance.

## Phase B, fix 2 — `ReadRefs` shares the reading (2026-09-14; raw: `target/55/phaseB2.txt`, heavy: `target/55/heavy-fix2.txt`)

**Fix (cause):** `RefSnapshot` gains `peeled: Vec<(RefName, Oid)>`, filled in the same loop from the same `tip_of` (sorted by name, refs reaching no commit left out); `peeled_references` delegates to `ref_snapshot`; `snapshot_is_both_halves` pins the third half loose and packed. `refs::read` takes `store.refs()?.peeled`, so `ReadRefs` and the history job share one read: whichever takes the store's `refs` mutex first reads, the other waits and reuses it.
**Ctrl+R race stays closed:** `ReadRefs` is dispatched only right after `OpenRepository` (`opening_repository`, `graph_key` refresh), and `open_repository` bumps the generation on the dispatching thread before either job exists; a `ReadRefs` can never be served a pre-keypress reading. Cost to the user: a badge read after an external ref change without `Ctrl+R` shows the refs as of the last open/refresh, which is what the graph already shows (same snapshot; before, badges could disagree with the graph).

With jobs (`budgets`, release, first paint; first run per path cold, listed apart):

| Cell | warm runs | cold first run | vs 150 | vs 500 |
|---|---|---|---|---|
| 1M cg | **99.1 / 98.1 / 98.7 ms** (was 191–195) | 1012 ms | yes | yes (warm) |
| 1M no-cg | **105.4 / 104.6 / 104.3 ms** (was 193–203) | 999 ms | yes | yes (warm) |
| 100k cg | 10.3 ms | 107 ms | yes | yes |
| 100k no-cg | 11.4 ms | 101 ms | yes | yes |

Serial `--phases` (warm): 1M cg total 98.2 (`ref_snapshot` 91.9, +149 MiB; `walk_new` 3.1; `first_chunk` 2.3); 1M no-cg 102.7 (`ref_snapshot` 86.6; `walk_new` 13.6); 100k cg 9.9; 100k no-cg 12.1. With jobs now equals serial: contention is gone.

**What remains of first paint at 1M:** ~90 % is the one ref read (6,498 loose files, ~11 µs/ref, plus `find_header` per tip). Inherent to loose refs; packed it is 23 ms. Not a gap against 150.
**Cold runs:** the first process after a build (or after the heavy fixtures were out of page cache) paid ~1.0 s on both 1M paths — ref files and pack pages read from disk. Warm-cache convention as in Phase A; cold-open cost is not budgeted by this ticket and not investigated further.
**Not measured:** a real kernel clone (a fresh clone has `packed-refs`, the 23 ms path); debug first paint with 50k refs (KNOWN_GAPS row updated).
Gates: fmt, clippy, clippy heavy-tests, `cargo test` 841/0; heavy tier (`cargo test --features heavy-tests -- --test-threads=2`) all pass, incl. `heavy::budgets_hold_*` and `many_refs::*`.
