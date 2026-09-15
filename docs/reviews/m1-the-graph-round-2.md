# M1 The graph — review round 2 (last round)

Scope: `1b828ca..HEAD` (`ebeb0d1`), fix tickets 65–77. Reviewer: Opus. Read: CLAUDE.md, round 1, ticket Status/scope lines, `git diff --stat`, MAP.md diffs, the fix commits 65–69 and 77 in full, and a line-set comparison of every split commit (70–76b). Outputs: `target/m1-review-2/`. Visuals not judged (user's eye).

## Verdict

**0 Critical, 1 Major, 5 Minor.** M1 is not blocked. Every round-1 Major is closed at the cause. The splits are pure moves.
- **New Major:** ticket 70's split blinded the `update` purity guard. It now scans 1 of 7 files.
- **Gates:** all green.
- **Heavy tier:** green.
- **Release perf, 3 runs:** green on all 10 budget tests. The 2 `GRAFT_1M` tests fail by design when the variable is unset (Minor m4).
- **1M frames:** green.

## Round-1 Majors

| # | Status | Evidence |
|---|---|---|
| M1: files over 400 non-test lines | **Closed** (one leftover, Minor m1) | A `wc` sweep of every tracked `.rs` file outside `tests/` and `tests.rs`, counting up to `#[cfg(test)]`: only `crates/graft/examples/kitty_check.rs` (469) is over 400. Next largest: `view/strips.rs` (385). |
| M2: `AppState` cloned per event (69) | **Closed, cause** | `AppState::refs: Option<Arc<RefBadges>>`. The test `the_next_state_shares_the_ref_badges_instead_of_copying_them` uses `Arc::ptr_eq`. Clone+drop with 50k refs: p50 0 µs, max ≤ 10 µs (was ~1 ms). **Unicode 50k max 5.35–5.50 ms** over 3 runs (round 1: 14.5). |
| M3: wheel strands seek (65) | **Closed, cause** | `History::scroll` now takes `jump` and clears `seeking`, as `select_at` does, and forces a fresh ask. The regression test scrolls between `seek` and the answer. |
| M4a: tied dates (67) | **Closed, cause** | `Dated::next` takes the whole equal-date class, reading down through same-dated parents, and emits it in Kahn order. Checked against a real all-one-date repo, before and after promotion, and against an in-memory graph. Limits: Minor m2. |
| M4b: dangling `incoming` (68) | **Closed, cause** | In layout, a reserved lane still in `unreached` closes with no edge. The `incoming ⊆ lanes` invariant is property-tested in layout, in the view-mode proptest, in fold and on the corpus. No snapshot changed. |
| M4c: piped stdout (66) | **Closed, cause** | `headless_settled` now uses `is_some_and`. `piped_stdout.rs` asserts the tip row is on screen, and a not-a-repo case exits 1. Checked by hand: an empty repo (`git init`) prints "no commits yet" and exits 0, and a 1-commit repo prints the commit and exits 0. No hang. |
| M4, other "M1 review" rows | **Not closed** | Still past due in KNOWN_GAPS (Minor m5). Round 1 left them for the user to re-date. |

## Split tickets 70–76b: pure moves?

**Yes, all 14 commits.**

Method:
1. For each commit, take the old and new versions of every touched `.rs` file.
2. Trim whitespace, strip `pub`/`pub(crate)`/`pub(super)`, and drop `use`, `mod`, comment and bracket-only lines.
3. Sort the lines and `comm` old against new.

What remains (`target/m1-review-2/pure-move.txt`):
- module-qualified call paths (`history::give_up_promotion`, `encode::free`, `super::supported`);
- rustfmt reflows;
- new `impl X {` headers and re-export lists.

For ticket 70, the call counts match old vs new: `set_phase` 5/5, `give_up_promotion` 7/7, `search_key` 4/4, `cancel_search` 2/2, `commit_graph_key` 2/2, `strip_rows_for` 3/3.

One behaviour change hides in a test, not in the code: Major M1 below.

- **74b `super::Commit`:** the old `history/mod.rs:34` was a **private** `use graft_core::commit::{Commit, …}`. Only the in-module test `commits_in_batches` reached it, as `super::Commit`, and it now imports it directly. No `pub use` was removed, and `graft_git::history`'s public re-exports (`read::{WalkError, first_parent_merges, read_commit}`, `walk::{CHUNK, Chunk, LinkChunk, Order, Walk}`) are unchanged in MAP.md. **No public path lost.**
- **MAP.md / ARCHITECTURE.md:** graft MAP.md gains "Internal split" notes (config, view, event_loop, jobs). ARCHITECTURE.md's `graph` paragraph names `assign_row`. A grep for pre-split file names (`update.rs`, `repo.rs`, `stream.rs`, `kitty.rs`, `*/mod.rs` …) across ARCHITECTURE.md, TESTING.md and every MAP.md is empty. Accurate.
- **`kitty_check.rs` (469 lines) and the 400-line rule:** **yes, PROMPT.md 6.2 covers it.** "Anyone who knows basic Rust should be able to follow *any* file", and "files under about 400 lines" has no carve-out for `examples/`. It was added in M1 (24d). Severity Minor (m1), not Major: it is a hand tool, not app code, 17 % over an "about" bound, and 6.2's own Major list names oversized *functions*, which clippy's `too_many_lines = 60` already gates.

## Builder flags: verdicts

- **67, the tie class is read whole before first paint:** **Minor (m2).**
  - *Large same-second rebase:* `git rebase` stamps each commit with wall-clock time, so a 10k-commit rebase makes classes of hundreds to low thousands. Each class costs its commits' reads (µs each), a few ms. Acceptable.
  - *Every commit on one date* (fast-import with a fixed date, some mirrors): first paint becomes the whole walk, ≈ `G` at 1M no-cg, 5.3 s. Before the fix the result was a wrong picture, so this is a trade, not a regression in correctness.
  - *Backwards clock:* the provisional order can misorder rows until promotion. Same class of limit ADR-0008 already accepts for the provisional order. No action.
- **69, pixel 50k max rose:** **no finding.**
  - This review's 3 runs give 10.10 / 7.99 / 7.75 ms, against 9.64 / 7.33 / 7.50 for pixel with fixture refs in the same runs.
  - The 50k pixel max now tracks the fixture pixel max. The frames differ in idle-prefetch interleaving, not ref count.
  - p99 6.3–6.8 ms; the bar is 16.
- **77, six remaining swallow sites:** **none breaks CLAUDE.md.** Each has a comment saying why ignoring is safe, which the rule allows. Detail:
  - `crash.rs:122` (`try_with` fails only while thread locals are destroyed, inside a panic hook): OK.
  - `crash.rs:156` (last stderr write in the panic hook): OK.
  - `main.rs:82` (stderr gone; the exit code still carries the failure): OK.
  - `event_loop/redetect.rs:85`: OK, verified. `Worker::serve` returns only after `remeasure`/`retheme` sent `Wake::TerminalFailed`, or when the loop dropped the receiver.
  - `jobs/runner.rs:227` (join at shutdown): OK. A worker panic outside a job already went through `crash::install`'s hook.
  - `jobs/runner.rs:91`: **the comment justifies the wrong thing** (Minor m3).
  - Also `terminal.rs:326` (77's own list): OK, commented.

## Checks run

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test` | pass. **20.96 s wall**, max RSS 325 MiB, footprint 66 MiB. Slowest binaries: 97 tests 6.23 s, 5 tests 3.65 s, 99 tests 1.19 s, 13 tests 0.90 s, 29 tests 0.92 s. Per-test top 5 **not measured**: no nightly toolchain for `--report-time`. |
| Heavy tier: `cargo test --features heavy-tests -- --test-threads=2 --skip scrolling_the_1m_commit_graph_fixture` | pass. 177.1 s wall, max RSS 579 MiB. graft `it`: 32 passed / 2 filtered, 85.8 s. 2 `#[ignore]` hand tools. |
| Release perf ×3 (`frames:: heavy::budgets_hold`) | 10/10 budget tests pass each run. **2 failed each run: the `GRAFT_1M` guards** (Minor m4). Wall 49.9 / 42.2 / 42.2 s; max RSS 371 / 282 / 282 MiB. |
| `GRAFT_1M=1` 1M frames, once | 2 passed, 49.0 s, max RSS 599 MiB. **Pixel p99 8.71 / max 11.58 ms; Unicode p99 8.49 / max 9.07 ms.** Idle-wake ≤ 12.2 ms. Round 1: 9.27/11.7 and 9.85/10.4. |
| Fuzzing / benches | no fuzz targets (M2). Benches not re-run: 65–77 touch no bench path except 67's date walk, whose `G` the builder measured. |
| `target/` | **33 GB** (round 1: 31), over §7.1's 20 GB (Minor m6) |

Release `frames::`, p99 / max in ms (bar 16, max asserted):

| Test | run 1 | run 2 | run 3 |
|---|---|---|---|
| Unicode, 50k refs | 5.29 / 5.50 | 5.04 / 5.35 | 5.20 / 5.46 |
| Unicode, fixture | 5.17 / 5.21 | 5.21 / 6.88 | 4.97 / 5.20 |
| Pixel, 50k refs | 6.80 / 10.10 | 6.28 / 7.99 | 6.37 / 7.75 |
| Pixel, fixture | 6.36 / 9.64 | 5.95 / 7.33 | 6.26 / 7.50 |

`budgets_hold`, first paint in ms (bar 150) / peak RSS in MiB (bar 300), runs 1/2/3:

| Fixture | First paint | Peak RSS |
|---|---|---|
| 100k + 50k refs | 36.9 / 35.2 / 35.3 | 91.4 / 93.4 / 92.2 |
| 100k cg | 12.0 / 11.9 / 12.0 | 26.8 / 27.0 / 26.8 |
| 100k no-cg | 14.0 / 13.6 / 13.5 | 71.5 / 68.0 / 67.8 |
| realistic cg | 12.4 / 12.0 / 11.9 | 30.7 / 30.7 / 30.8 |
| realistic no-cg | 14.0 / 14.0 / 13.9 | 75.0 / 74.7 / 75.1 |

## Findings

### Critical

None.

### Major

**M1. The `update` purity guard now scans one file of seven. It is a test weakened by a "pure move" (ticket 70).**
- **Where:** `crates/graft/src/update/tests.rs:1296–1311`, together with `crates/graft/src/update/mod.rs:336`.
- **What happens:** `update_cannot_reach_the_world` joins `mod.rs, events.rs, state.rs, keys.rs, search.rs, history.rs, commit_graph.rs` **in that order**, then calls `split_once("#[cfg(test)]")`. That marker sits at `mod.rs:336`, the end of the first file. Everything after it (the other six files) is thrown away as "tests".
- **Today:** a grep of the six files for the forbidden list (`std::io|fs|time|thread|process|net|env`, `SystemTime`, `Instant`, `print!` …) finds 0 hits, so nothing is violated yet.
- **Why it matters:** this test is the enforcement of PROMPT.md 6.1's pure core for the reducer. M2 adds Mistake Radar events straight into `keys.rs`/`events.rs`. An `Instant::now()` there passes green, and the reducer stops being replayable.
- **Also brittle:** the file list is hand-kept, so a new `update/radar.rs` is never scanned.
- **Fix (cause):** strip the test section per file, not after joining: `mod.rs` up to its `#[cfg(test)]`, every other file whole. Build the list so a new file can't be missed. Either assert that the `include_str!` list matches `update/*.rs` (minus `tests.rs`) via a `std::fs::read_dir` in the test, or scan the directory at test time. Add a failing-case check: a forbidden token placed in the last file must be caught.

### Minor

**m1. `crates/graft/examples/kitty_check.rs` is 469 lines,** over PROMPT.md 6.2's "about 400". It is a hand tool from 24d. Fix: split its encoder driver from its terminal/reply handling, as 76b split `kitty.rs`. Or record in TESTING.md that `examples/` hand tools are exempt, with the user's OK.

**m2. The tie class is unbounded before first paint (67).**
- **Where:** `crates/graft-git/src/history/simple.rs`, `Dated::next`. The class `while` loop reads down every same-dated parent before emitting anything.
- **Failure scenario:** a 1M-commit mirror imported with one fixed date and no commit-graph. First paint becomes the whole read (~5 s, `G`-sized) instead of ~100 ms.
- **Also:** 67's acceptance asked for 1M no-cg first paint within 10 % of 56c. Its after numbers are 104–125 ms against 56c's warm 104–112. The 125 is 11.6 % over, from one set of runs.
- **Fix (cause):** a user decision, not a builder one. Either accept it (git's `--date-order` also limits first), or cap the class at a screen's worth. Past the cap, emit the class in id order marked provisional, and let promotion correct it. Re-measure first paint 3× if kept.

**m3. `jobs/runner.rs:91`: `handle.join().unwrap_or(())`'s comment says why the join cannot *hang*, not why ignoring its `Err` is safe.** The real reason is that a panicked worker was already reported by `crash::install`'s hook. Fix: say that in the comment, or use an explicit `match` as 77 did at its own sites.

**m4. The documented release perf command is red as written (round-1 m4, not in 65–77's scope).**
- **Where:** `crates/graft/tests/it/frames.rs:108–111`. Unset `GRAFT_1M` panics, so `frames:: heavy::budgets_hold` reports `2 failed` every run.
- **Who is affected:** CLAUDE.md's and `.claude/agents/reviewer.md`'s perf command. A reviewer has to know to ignore the 2 failures, which is how a real frames failure gets missed.
- **Fix (cause):** a `heavy-1m` cargo feature (ADR-0018 amendment), or `--skip scrolling_the_1m_commit_graph_fixture` in every documented command in one change.

**m5. KNOWN_GAPS rows still dated "M1 review".** Examples:
- 50k-refs debug first paint;
- history walk holds a worker;
- `Esc` during sort not seen in pty;
- promotion answers nothing while advancing;
- folded merge looks plain;
- long path truncates notice;
- Unicode highlight bold/dim only;
- idle prefetch.

Also, closed rows (68, 67) still carry "M1 review" in their expiry column. Fix: the user re-dates or closes them at the checkpoint; set closed rows' expiry to "Closed" (as 66 did).

**m6. `target/` is 33 GB, over §7.1's 20 GB.** It holds round-1 evidence dirs and the 1M fixtures. Fix: `cargo clean` after the checkpoint, keeping only what docs cite (user's call).

## Carried to KNOWN_GAPS

- m2: tie-class bound. The user decides: accept, or cap + provisional.
- m4: `GRAFT_1M` vs ADR-0018 (open since round 1).
- m5: the re-dating of past-due "M1 review" rows.
- 69's "fixture frames got faster too" (Unicode p99 ~8.8 → ~5.0 ms): this review's runs confirm the lower numbers (4.97–5.21). The cause is not identified. Informational.
- Round-1 open items not in 65–77: `reserve_exact` (m1), the `order.rs:137` panic (m2), `#[ignore]` hand tools vs TESTING.md (m5), 1M frame ranges (m6). None changed this round.

## Not checked

- Per-test 5 slowest (no nightly toolchain).
- Benchmarks (not re-run; see Checks).
- Line-by-line read of the split files beyond the normalized line-set diff. A moved line that changed only visibility, or a reordered statement inside a function, would not show in that diff. The gates and the unchanged snapshots cover behaviour.
- The leak check (round 1) was not repeated after 69's `Arc` change.
- Linux / other allocators; git/git memory rows; `sudo purge` cold first paint (user).
- DESIGN.md §3 compliance beyond what 65–68 touched.
