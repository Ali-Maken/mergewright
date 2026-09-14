# M1 The graph — review round 1

Scope: `62fdaa5..HEAD` (M1 start to `b7a3d62`). Reviewer: Opus (M1 is grandfathered). Read lean: CLAUDE.md, PROMPT.md §1.2/§1.3/§6/§7.1/§8 M1, ADR-0018 to 0027, KNOWN_GAPS.md, ticket 56's review section, PROGRESS.md deviations and open questions, every `crates/*/MAP.md`, then targeted spots in the diff. Visuals not judged (user's eye).

## Verdict

**0 Critical, 4 Major, 8 Minor.** No Critical, so M1 is not blocked. No memory leak at 1M, with or without a commit-graph. Every gate, the heavy tier, the release perf run and the manual 1M frames run are green. The Majors:
- the 400-line rule;
- a frame margin at 50k refs that explains m1/64's flake;
- a still-open wheel/search hang;
- correctness gaps whose "M1 review" expiry is now due.

## Checks run (one at a time, outputs in `target/review-m1/`)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test` (fast tier) | pass, **20.1 s wall**, cargo process max RSS 326 MiB, footprint 66 MiB. Slowest test binaries: 95 tests 6.23 s, 5 tests 3.55 s, 97 tests 1.05 s. Per-test top 5 **not measured** (needs nightly `--report-time`) |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2 --skip scrolling_the_1m_commit_graph_fixture` | pass, 158.2 s wall, max RSS 562 MiB; graft `it` 31 passed / 2 filtered in 76.6 s; 2 `#[ignore]` hand tools |
| Release perf run (`frames:: heavy::budgets_hold`, `--skip` the 1M pair) | 10 passed, 46.7 s wall, RSS 322 MiB. Frames p99/max: Pixel 50k refs 7.1/8.2 ms, **Unicode 50k refs 12.5/14.5 ms**, Pixel 100k 8.6/11.1, Unicode 100k 8.5/9.5 (bar 16, max asserted). Idle-wake up to 18.8 ms (printed, not asserted). First paint 37.5 (50k refs) / 12.2 / 13.9 / 12.2 / 14.3 ms (bar 150). Peak RSS 92.5 / 27.1 / 68.7 / 31.0 / 75.8 MiB (bar 300). `G` 48–503 ms |
| Manual 1M frames `GRAFT_1M=1 … scrolling_the_1m_commit_graph_fixture` | 2 passed, 21.3 s. Pixel p99 9.27 / max 11.7 ms; Unicode p99 9.85 / max 10.4 ms; idle-wake ≤ 13.8 ms; RSS 401 MiB; no rows-on-hand misses |
| Benches (`hot_paths`, `row_render`, warm-up 1 s, measure 3 s) | advance chunk 982 µs; rasterise strip 608 µs (−7.7 %); StripCache hit 1.69 µs (+1.7 %), miss 615 µs (−7.2 %); GraphColumn render 15.1 µs (+1.7 %). No regression > 10 % |
| Fuzzing | no targets yet (M2) |
| `target/` size | **31 GB**: over §7.1's 20 GB. Not cleaned, because it holds the evidence dirs (`target/56c`, `61`, `64`) and the 1M fixtures (Minor 8) |

## Memory leak check (user asked)

Harness: `target/review-m1/leak/` (not committed).
- **Setup:** it drives `graft::update` and the real `JobRunner` exactly as `examples/budgets` does, at 120×40.
- **One cycle:** `G` to the bottom (wait until complete), then `g`, then `Ctrl+R` (wait for the repaint), then `/Trunk commit 123457` Enter (a far hit at row 876,543; wait until the cursor is on screen).
- **Sampling:** `graft_fixtures::footprint()` is read after each cycle.

| Fixture | Cycles | Footprint after warm-up | Peak footprint | Process max RSS | Per cycle | Verdict |
|---|---|---|---|---|---|---|
| 1M small cg (`…-commit-graph-511a25b425758b13`) | 60, then 125 | 125-run: sawtooth 31–58 MiB for cycles 1–51, then **29–30 MiB flat for cycles 52–125** | 62.4 MiB (125-run) | 454 MiB (pack mmap) | `G` 100 ms (first 509 ms), refresh 82 ms, far find 3.16 s | **no leak** |
| 1M small no-cg (`…-no-commit-graph-d5f8e6a5494a9ead`) | 50 | 80–82 MiB for cycles 21–40; bump to 93–107 at 41–48, **back to 80 at 50** | 148 MiB (set at open; never exceeded) | 485 MiB | `G` 379 ms (first 2.9 s), refresh 2.37 s, far find 5.49 s | **no leak** |

- The 60-cycle cg run alone looked like a rise: 38 MiB up to 87 MiB over cycles 40–51. The 125-cycle run shows it is allocator retention, which is released later. The trough keeps returning to ~30 MiB and the peak never grows. So a 50-cycle run can mislead; judge on ≥ 100.
- `#![forbid(unsafe_code)]` is set in core, git, graph, ops, ui and the `graft` lib.
  - `graft-term` and `graft-fixtures` use `deny`, with site-scoped allows (ADR-0013, ADR-0015).
  - `crates/graft/src/main.rs` has no attribute, but the workspace `unsafe_code = "deny"` (Cargo.toml:36) covers it. OK.
- `Arc` cycles: none found. The shared state is the input mux waiters/outlet, the terminal `SessionState` and the jobs queue. There is no `Rc`/`Weak` graph.
- Caches that never shrink:
  - `StripCache` is a bounded LRU.
  - `StreamingLayout::checkpoint_stride` doubles and never shrinks. That is by design (the stride is a function of history length), and it is bounded at 16,000 for 1M.
  - `EmittedOrder` grows with history. That is the product (ADR-0026).
  - No unbounded per-event holder found. The flat curves confirm it.
- Informational, no spec budget exists: a far Find at 1M takes **3.2 s with a cg and 5.5 s without**, and `Ctrl+R` without a cg takes 2.4 s. The user may want a budget in M2.

## Ticket 56's "For the M1 review" table (verbatim)

The reported cell (1M without a commit-graph) and its neighbours, from 56c's re-run after the findings tickets (58b, 59b, 60, 61, 63). Release, 3 runs, `scripts/memory-curve.sh 10000 100000 1000000` on 2026-09-14, `docs/shots/m1-memory-curve.txt`. Peak RSS = process max RSS over open → first paint → `G` → `g` (`budgets --gate`, the gate); footprint = `/usr/bin/time -l` peak memory footprint (anonymous pages; excludes clean file-backed pack pages); first paint run 1 is cold (ADR-0027). `heap B/commit`: no-cg = live heap after `g` from `prototypes/heap-profile-59` (100k→1M); cg = `heavy::slopes_per_commit_with_a_commit_graph` footprint slope (10k→100k), with the 100k→1M footprint slope from the cells beside it.

| Cell | pack on disk | peak RSS (run 1 / 2 / 3) | footprint (run 1 / 2 / 3) | first paint (run 1 / 2 / 3) | `G` | `overflow_len` | checkpoint stride | heap B/commit | verdict |
|---|---|---|---|---|---|---|---|---|---|
| 1M small, **no cg** | 297 MiB | **445.5 / 445.2 / 444.7 MiB** | **155.6 / 155.3 / 154.8 MiB** | 997 / 112 / 104 ms | 5.33–6.08 s | 1,000,000 | 16,000 | live **76** (bar ≤ 100); footprint 138–140 (floor ~140, reported) | **reported** (ADR-0026): live heap pass; footprint < 300 yes; RSS = footprint + 290 MiB ≈ the pack |
| 1M realistic, **no cg** | 358 MiB | **507.5 / 508.5 / 508.8 MiB** | **156.7 / 157.6 / 157.9 MiB** | 1,032 / 108 / 111 ms | 5.88–6.30 s | 1,000,000 | 16,000 | footprint 139–140 (live profiled on the small fixture only) | **reported**: RSS = footprint + 351 MiB ≈ the pack |
| 1M small, cg | 297 MiB | 174.3 / 173.5 / 174.3 MiB | 37.8 / 37.0 / 37.8 MiB | 710 / 91 / 90 ms | 527–564 ms | 0 | 16,000 | 29–31 (guard, bar ≤ 50); ~33 from the cells | **gate pass** (< 300 MB) |
| 1M realistic, cg | 358 MiB | 183.7 / 182.6 / 183.2 MiB | 46.7 / 45.5 / 46.2 MiB | 790 / 88 / 91 ms | 566–637 ms | 0 | 16,000 | ~38–40 from the cells | **gate pass** |
| 1M small, no cg → **after `B`** (the offer, same repository; `docs/shots/m1-memory-curve-offer.txt`) | 297 MiB | 173.4 / 172.8 / 173.9 MiB | 37.0 / 36.3 / 37.5 MiB | 124 / 89 / 84 ms | 535–552 ms | 0 | 16,000 | as cg | **gate pass**; write 5.3 s wall (`git commit-graph write --reachable`) |
| 100k: small / cg / realistic / realistic cg | 29–35 MiB | 67.6–68.1 / 26.6–26.8 / 75.6–76.1 / 30.3–30.5 MiB | 35.5–36.0 / 9.1–9.3 / 37.4–37.9 / 12.6–12.8 MiB | 96, 12, 12 / 16, 13, 11 / 109, 12, 14 / 80, 10, 10 ms | 469–507 / 52–79 / 516–544 / 50–60 ms | 100k / 0 / 100k / 0 | 2,000 | (guard span) | pass |
| 10k: small / cg / realistic / realistic cg | 2–3 MiB | 15.7–17.0 / 12.0–12.1 / 24.5–24.6 / 15.9–16.0 MiB | 9.3–10.5 / 6.3–6.4 / 17.4–17.5 / 10.0–10.1 MiB | ≤ 23 ms all | 37–61 / 13–16 / 40–42 / 14–18 ms | 10k / 0 / 10k / 0 | 2,000 | (guard span) | pass |

- **Slope guard** (`heavy::slopes_per_commit_with_a_commit_graph`, 10k cg → 100k cg, release, 3 runs alone, `target/61/guard-{1,2,3}.txt`): heap 29 / 31 / 29 B/commit (bar ≤ 50); peak RSS 167 / 169 / 167 (bar ≤ 200, floor ~166 named: cg pages ~56 + pack index ~68 + heap ~35). Before 58: 237. Pass.
- **Far jumps at 1M** (after a cold `G`, `once:` lines of the curve file): rows 500,000 and 900,000 → 13.0–16.3 ms, 4,500 rows replayed, on all four 1M fixtures and the post-`B` repo; stride 16,000. Before m1/60: 500,500 rows, 1.39–1.46 s. Pass.
- **First paint at 1M:** warm (runs 2–3) 88–112 ms on every path — under 150, yes. Cold (run 1) 710–790 ms cg, 997–1,032 ms no-cg — over 500; judged warm (ADR-0027); the cold cause (page cache) is the user's `sudo purge` check at the checkpoint.
- **Heavy tier** once, alone, release: 691 passed / 0 failed / 2 ignored (`target/56c/heavy-full.log`). **Release perf run** (ADR-0021, `target/56c/perf-run.log`): 10 passed; `frames::` p99 5.3–10.0 ms against 16; `budgets_hold_*` first paint 12.4–54.9 ms against 150 warm, peak RSS 27.0–96.3 MiB against 300. **No 1M `frames::` test exists** — the 1M scroll frame numbers ADR-0026 deferred to this run are missing (KNOWN_GAPS "56d" section; user's call).
- **The 2× check:** `overflow_len` = commits without a cg, 0 with one, every cell and run.

## KNOWN_GAPS "m1-the-graph/56d" (verbatim)

> Every budget in 56's checklist passes on 56c's numbers (`docs/shots/m1-memory-curve.txt`, `target/56c/`). Three things are reported, not proven, and each names whose call it is.
>
> - **Cold first paint at 1M is ~0.7–1.0 s (ADR-0027).** Run 1 of every 1M cell in 56c's re-run: no-cg 997–1,032 ms, cg 710–790 ms; runs 2–3 are 88–112 ms on every path, under 150. The budget is judged warm (ADR-0027, user decision). Evidence the cost is the page cache and not Graft: the offer copy, written seconds earlier, painted in 124 ms on its run 1; 55's just-built fixture painted in 134 ms. Not proven: only `sudo purge` empties the cache on macOS. **Expiry: the user runs one `budgets --gate` first paint straight after `sudo purge` on the 1M fixture at the M1 checkpoint (TESTING.md manual checklist), or accepts the warm convention as final.** User.
> - **~~The 1M `frames::` scroll numbers ADR-0026 deferred to the perf run were never measured.~~ Closed by m1-the-graph/64.** Two tests next to the 100k ones (`scrolling_the_1m_commit_graph_fixture_keeps_input_to_frame_under_16_ms_at_the_{unicode,pixel}_tier`), manual (`GRAFT_1M=1`, TESTING.md), on the 1M small commit-graph fixture: release, 3 runs each, alone — Unicode p99 6.37–6.75 ms, Pixel p99 6.89–8.01 ms, both well under 16 ms, no key drawn before its rows arrived on any run (`target/64/`). PROMPT.md §1.2's 60 fps at 1M+ is now measured at frame granularity, not just the worker-side jump numbers 56c had.
> - **`examples/budgets`'s counters hold 4× the app path (harness only).** `order_bytes`, `cold_jump` and `far_jump` lay history out again through `direct_layout`: 100k cg 124 MiB process max RSS vs 33 MiB for the gated path, 1M cg 997 vs 240 (56 pass 1). `--gate` keeps them out of every gate and out of `scripts/memory-curve.sh`'s RSS/footprint cells; the counters run is a separate process. Related: `ru_maxrss` is a lifetime maximum, so any guard that measured two fixtures in one process read a false green in the wrong order — fixed by m1-the-graph/57 (guards measure each fixture in a fresh child). **Expiry: not planned** — the harness is not the app, and the gated path is what every number in this file reports.

### Agreement check

- **Table vs KNOWN_GAPS "No-commit-graph footprint slope at 1M (59b)" and 56d:** they agree. 1M no-cg footprint 154.8–157.9 MiB, RSS 444.7–508.8, live heap 76, floor ~140. Cold first paint 997–1,032 / 710–790, warm 88–112.
- **ADR-0026's status line** ("footprint 155–158, RSS 445–509"): rounded, consistent.
- **Stale:** the table's heavy-tier bullet still says "No 1M `frames::` test exists". KNOWN_GAPS 56d closes it via m1/64 (Minor 7).
- **Expected difference:** "1M cg 997 vs 240" in 56d is pass 1's number, not the table's 174. The text says "56 pass 1", so this is not a contradiction.
- **This review's own numbers:**
  - 1M cg process max RSS 454–498 MiB in the leak harness, against the table's 174 MiB. Expected: the harness adds 60–125 search/refresh cycles, and the pack's file-backed pages count toward RSS but not toward footprint.
  - Footprint 29–62 MiB agrees with the table's 37–38.

## Findings

### Critical

None.

### Major

**M1. Files far over the 400-line rule (PROMPT.md 6.2).**
- **Where (non-test lines):**
  - `crates/graft/src/update.rs:1–1542`
  - `crates/graft-ui/src/graph_screen/mod.rs` (~1115; tests are external)
  - `crates/graft-git/src/repo.rs:1–803`
  - `crates/graft/src/jobs/mod.rs` (~800)
  - `crates/graft-graph/src/layout/stream.rs:1–775`
  - `crates/graft/src/event_loop/mod.rs:1–761`
  - `crates/graft/src/history/mod.rs` (~700)
  - `crates/graft/src/view.rs:1–704`
  - `crates/graft-graph/src/raster.rs:1–654`
  - several more between 400 and 600 (terminal.rs, graph_column/mod.rs, layout.rs, synthetic.rs)
- **Failure scenario:** M2 adds Mistake Radar events and the change goes into `update.rs`. A 1,542-line reducer has to be read whole to change safely, so the next builder compacts mid-ticket (CLAUDE.md "Size" rule), or misses an arm that clears `seeking` (see M3).
- **Fix (cause):** split along existing seams, one file per concern, with MAP.md updated. For example:
  - `update.rs` into `update/{keys,history,search,refs,jobs}.rs`;
  - `event_loop/mod.rs` into its startup/headless/loop parts;
  - `graph_screen/mod.rs` by pane.

  A clippy allow or a raised threshold would be a symptom fix.

**M2. Every event clones the whole `AppState`, including 50k-ref `RefBadges` by value, which makes m1/64's Unicode flake a real margin problem.**
- **Where:** `crates/graft/src/update.rs:505` (`let mut next = state.clone();`); the field at `update.rs:173` (`pub refs: Option<crate::refs::RefBadges>`); the type at `crates/graft/src/refs/mod.rs:32` (`HashMap<Oid, Vec<RefLabel>>`).
- **Evidence:**
  - The release perf run at 50k refs, Unicode tier: p99 12.5 ms, **max 14.5 ms**, against an asserted max of 16 ms (`crates/graft/tests/it/frames.rs:180–250`), so 1.5 ms of margin.
  - The same tier with fixture refs: p99 8.5, max 9.5.
  - Pixel with 50k refs: p99 7.1.
  - The cost scales with the ref count, not the tier's drawing alone.
  - m1/64's `fifty_thousand_refs_…_unicode` failure fits this. Blaming CPU power state does not explain why only the 50k-ref case sits near the bar.
- **Failure scenario:** a user with a monorepo of ~50k tags holds `j` on a laptop in low-power mode. Frames cross 16 ms, and the heavy tier goes red intermittently.
- **Fix (cause):** profile first (unprofiled here; KNOWN_GAPS 30h already suspects this clone). Then hold `Arc<RefBadges>`, and more generally `Arc` the large immutable sub-states, so the per-event clone is O(1).
- **Symptom fixes to reject:** a rerun, asserting p99 instead of max, or raising the bar.

**M3. A wheel scroll during a far search jump strands the search, still open at its expiry.**
- **Where:** `crates/graft/src/history/scroll.rs:288–297`. `History::scroll` calls `self.wanted(height, false)` without taking `jump` or clearing `seeking`. Compare `select_at` (`:278–279`) and `seek` (`:311–312`), which do both.
- **Gap:** KNOWN_GAPS.md:197, expiry "M1 review"; also a PROGRESS open question.
- **Failure scenario:** `/fix` Enter on a hit 800k rows down, then a wheel flick while it seeks. The newer request's answer doesn't match `jump`, `seek_landed` never fires, and the header says "seeking" forever. `Esc` is the only way out.
- **Fix (cause):** withdraw the jump on a wheel ask, as `select_at` does, or clear `seeking` when an answer for a newer request lands. Add a test in `history` that scrolls between `seek` and the answer.

**M4. Correctness gaps whose expiry is "M1 review" are still open. The expiry is now, and M1's promise ("the shape of history is obvious at a glance") is what they break.**
- **Tied dates** (KNOWN_GAPS.md:190): with no commit-graph and all commits sharing a timestamp, a parent is drawn above its child. It is wrong on screen for any `git filter-repo`- or script-generated history.
- **Dangling edge** (KNOWN_GAPS.md:189): the layout can name an `incoming` column that is not in the row's `lanes`, drawing a `─` into nothing.
- **Piped stdout** (`crates/graft/src/event_loop/mod.rs:434–439`, KNOWN_GAPS.md:181): `headless_settled` counts `history: None` as settled, so `graft repo | cat` can print a frame before the repository opens.
- **Other M1-review rows** (key order under a pty :262, the gallery gaps :316–321, ref-states HEAD :177, 50k-refs debug first paint :178, git/git headroom :152–153): not bugs, but also past due.
- **Fix:** for the three correctness rows, a fix ticket each (cause). Each needs a regression test: equal-timestamp topo order, the `incoming ∈ lanes` property, headless waits on `history.is_some()`. For the rest, an expiry the user re-dates at the checkpoint. Re-dating the three bugs silently would be a symptom fix.

### Minor

**m1. `EmittedOrder::reserve` is `reserve_exact` per batch (51d).**
- **Where:** `crates/graft-graph/src/layout/order.rs:115–118`, called per batch at `layout.rs:261`.
- **Risk:** exact-size growth defeats doubling, so each batch may copy the whole order. That is quadratic in theory.
- **Measured:** linear. cg `G` goes from 52–79 ms at 100k to 527–564 ms at 1M (10×); no-cg goes from 469–507 ms to 5.33–6.08 s. So the allocator grows in place at these sizes on macOS.
- **Failure scenario:** another allocator (Linux glibc, or musl) that moves big blocks turns `G` at 1M into many 8 MB copies.
- **Fix (cause):** `reserve` (amortised) instead of `reserve_exact`, confirmed by the `G` numbers.

**m2. A panic outside tests.** `crates/graft-graph/src/layout/order.rs:137`: `unwrap_or_else(|_| panic!("more than u32::MAX rows in one order"))`. It is unreachable while `CommitHandle` is u32, but it dodges the no-unwrap rule by other means. Fix: return an error through `StreamError`, or document the invariant at the type (cause: the handle width).

**m3. KNOWN_GAPS rows are stale or past due.**
- **Stale:**
  - :234 "`AppState` keeps every row it is sent" is no longer true. `History` holds `window: Option<Window>` with `rows: Arc<[Row]>` (`history/mod.rs:150, 228`), and `chunks` is gone.
  - :226 "`rows()` clones every row": `StreamingLayout::rows` (`layout/stream.rs:406`) returns a viewport `Vec<Row>`, which is bounded, so the row's premise has changed.
- **Past due:** rows expiring on M1 tickets 08, 10, 11, 27, 30 (:193, :207, :176, :210, `graph.integration_branches`) are still open with expiries already gone.
- **Failure scenario:** the checkpoint report reads "gaps: N". It is wrong both ways.
- **Fix:** close the stale rows with the evidence and re-date the rest (housekeeping).

**m4. The `GRAFT_1M` tests fail when the variable is unset, which contradicts ADR-0018's "one command runs the whole tier".**
- **Where:** `crates/graft/tests/it/frames.rs:108–130`.
- **Current state:** TESTING.md:54 and :66 add `--skip scrolling_the_1m_commit_graph_fixture`. `.claude/agents/reviewer.md`'s perf command and PROMPT.md §7.1 don't, so they are red as written.
- **Failure scenario:** the next reviewer or CI runs the documented command and gets 2 failures that aren't regressions.
- **Fix (cause):** a separate `heavy-1m` cargo feature (the ADR-0018 mechanism, no env var), with an ADR-0018 amendment. Or update the three documents in one change. Adding `--skip` everywhere is the symptom path already taken.

**m5. `#[ignore]`d tests exist although TESTING.md:57 says "none is `#[ignore]`d".**
- **Where:** `crates/graft-graph/tests/it/raster.rs:659` (writes `docs/shots` PNGs) and `strip_cache.rs:368` (a timing tool).
- **What they are:** hand tools, not heavy tests. They show as "2 ignored" in every run.
- **Fix:** move them to `examples/`, or reword TESTING.md and ADR-0018 to allow hand tools.

**m6. The 1M frame numbers drifted up.**
- **This run:** Unicode p99 9.85 / max 10.4 ms, Pixel p99 9.27 / max 11.7.
- **m1/64 (quoted in ADR-0026's status line and KNOWN_GAPS 56d):** 6.37–6.75 and 6.89–8.01.
- **Judgement:** still under 16. Either machine state or real, from a single run.
- **Fix:** quote ranges from ≥ 3 runs including a review run. Don't treat one set as the number.

**m7. The ticket-56 table's heavy-tier bullet is stale.** It still reads "No 1M `frames::` test exists", which m1/64 closed. Fix: add a "closed by m1/64" note in the ticket (docs only).

**m8. `target/` is 31 GB, over §7.1's 20 GB.** It was not cleaned because `target/56c`, `61` and `64` hold the evidence the table cites, along with the 1M fixtures. Fix: move cited logs under `docs/shots/` or accept them as lost, then run `cargo clean` after the checkpoint (the user's call).

## Carried items

- **Builder deviations** (PROGRESS session log):
  - m1/18 `WalkSource::in_order`, m1/49b table from `walk.table()`, m1/50 overflow-only frontier and paged arrays, m1/51 `Included` private sequence (4 B/commit), m1/52 read-count checks (reads = hydrated + merge_reads), m1/54 `Option<JobId>`: all logged, reflected in MAP.md, and none contradicts ADR-0026.
  - The m1/52 test-check change weakens nothing: it counts merge reads that were previously hidden.
- **51d `reserve_exact`:** Minor m1. Linear as measured, not proven for other allocators.
- **Commit reads not batched:** not profiled. The 1M frames max is 11.7 ms with no rows-on-hand misses, and far jumps are 13–16 ms, so there is no user-visible cost at 1M on this machine. No finding.
- **`KnownCommits::covers_the_graph`** (off-spec, 58b): documented in graft-git MAP.md and logged. It is a small public addition that serves the cg/no-cg decision. It needs the user's nod at the checkpoint; no finding.
- **ratatui `unstable-rendered-line-info` (m1/34):** no longer present in any `Cargo.toml` (grep empty). Resolved.
- **`GRAFT_1M` vs ADR-0018:** Minor m4.
- **The m1/64 flake:** real, not power state (Major M2).

## Open questions for the M1 checkpoint (PROGRESS.md), not answered here

- **Leak check:** measured above, no leak.
- **Wheel during far search:** Major M3 confirms it is still open.
- **ratatui unstable feature:** removed.
- **Pixel blank frame on PgDn (30b), DESIGN §3.8 resize flush, new keys, uncommitted row only at top, node glyphs vs `ui.icons`, png 0.18:** no finding here bears on them.

## Also checked, no finding

- **Crate rules:** ui has no normal dependency on git/ops. graft-ui and graft-graph dev-depend on graft-git and graft-fixtures, which ADR-0006 allows. graft-term's png 0.18 is an external dependency.
- **No `unwrap`/`expect` outside tests:** the clippy deny gate is green, and the M1 diff adds no `allow`.
- **Terminal sanitising:** enforced by type. `SanitizedText` comes from `graft_term::sanitize::*`, used across view.rs and ui graph_screen/badges.
- **Swallowed errors:**
  - `tty.rs:144/154 let _ = child.kill()` has comments.
  - `.ok()` on in-memory buffer writes in kitty.rs, iterm2.rs and row_clear.rs cannot fail.
  - `scroll.rs:369 unwrap_or_default` and `layout.rs:338` are clamped or semantic.
- **UI thread:** git runs on the `JobRunner` workers. The frames tests' rows-on-hand assertion holds at 1M.

## Not yet checked

- Per-test 5 slowest in the fast tier (needs nightly `--report-time`).
- Line-by-line diff of graft-graph `layout/{stream,fold}` and graft-git walk/included: only MAP.md plus targeted spots were read.
- DESIGN.md §3 section-by-section compliance, and ARCHITECTURE.md beyond the crate map.
- The git/git memory rows (KNOWN_GAPS :152–153): no git/git clone run.
- Profiling of M2's cause (the RefBadges clone versus per-row badge work).
- Leak behaviour on the realistic 1M fixtures, and on Linux.
