# M8 close: the quiet-machine run

Reviewer, 2026-10-11. HEAD **58367ddd**, read-only on source (`git diff HEAD -- crates scripts` is empty; the
`+uncommitted` in `docs/perf/m8-accept.txt`'s header is this run's own docs/perf files). `CARGO_TARGET_DIR` was a
scratch dir (8.2 GB at the end, deleted). Every run was started from a script writing to `docs/perf/`; this file reads
only their summary lines.

**Machine.** At the first dispatch (≈08:50Z) the laptop was on battery with Low Power Mode on and load 5.05, so nothing
ran. On re-dispatch I checked it myself: AC, `lowpowermode 0`, load 1.10. The builds took 152 s, and load 1-min was
4.12 when they finished. It settled under 3 after 40 s, and the runs started then.

| Run | Start (UTC) | load 1/5/15 | power | LPM |
|---|---|---|---|---|
| build | 08:57:03 | 1.27 1.15 1.57 | AC | 0 |
| 1 heavy tier | 09:00:15 | 2.36 2.24 1.98 | AC | 0 |
| 2 release perf | 09:08:52 | 3.67 2.80 2.38 | AC | 0 |
| 3 `m8-accept.sh --perf` | 09:10:42 | 1.50 2.31 2.24 | AC | 0 |
| 4 trace run 1 / run 2 | 09:20:19 / 09:20:29 | 2.89 4.14 3.50 / 2.75 4.07 3.48 | AC | 0 |
| 1b heavy, rest of workspace | 09:21:35 | 2.20 3.62 3.35 | AC | 0 |

The machine was not idle during the trace runs, though 1-min load was under 3. Busy processes (other than claude, over
40 % CPU): `mediaanalysisd` at 72 % in trace run 1 and `fseventsd` at 85 % in trace run 2. Neither one is a dev server.

## Verdicts

| Run | Verdict | File |
|---|---|---|
| 1 Heavy tier | **RED**: 5 failures, all `held_key` in a debug build | `docs/perf/m8-heavy.txt` |
| 2 Release perf (ADR-0021) | **RED**: 8 of 21 failed. Every `frames::` input-to-frame p99 is over 16 ms. First paint passes | `docs/perf/m8-release.txt` |
| 3 `m8-accept.sh --perf` | **rc=1**: steps 1, 3 and 5 pass. Step 2 has 9 passed and 2 failed, both on raw Kitty (the M7/46 family) | `docs/perf/m8-accept.txt` |
| 4 Trace follow-up ×2 | **MISSED**: draw_us p50 and p99 missed in both runs; bytes and the rest were met | `docs/perf/m8-trace-followup.txt` (tail) |

### 1. Heavy tier — `cargo test --features heavy-tests -- --test-threads=2`

- graft `it` had 890 passed, **5 failed** and 6 ignored, in 477 s. The whole command took 517 s wall. The peak RSS of the largest single process was 754 MB (`/usr/bin/time -l`).
- cargo stops at the first failing binary, so the rest of the workspace never ran. I ran it after:
  `cargo test --workspace --exclude graft --features heavy-tests --no-fail-fast` plus graft doctests gave 35 binaries,
  3146 passed and **0 failed**, in 591 s. With both parts together, every heavy binary ran.
- Failures. Each panics at `crates/graft/tests/it/held_key.rs:259` (the verdict assert):
  - `held_key::held_j_stops_on_release_pixel`: `frames_after_release=3` (≤2)
  - `held_key::held_j_stops_on_release_pixel_free`: `frames_after_release=3`, plus `bytes_per_frame_held=2632880` (≤512 KiB)
  - `held_key::held_j_stops_on_release_pixel_iterm2`: `frames_after_release=3`
  - `held_key::held_j_with_the_pane_shown_stops_on_release_pixel_deflate`: `frames_after_release=3`
  - `held_key::held_j_with_the_pane_shown_stops_on_release_unicode_40mb`: `frames_after_release=4`
- **Question, not a defect.** These are release-shaped tests (TESTING.md's held-keys command uses `--release`), and
  here they ran in a debug build. `docs/reviews/m8-09-keymap.md:18` saw the same `frames_after_release=3` misses under
  load 37 and put them down to the load. This run had load 2.4, so **load is not the explanation**. One of two things is true:
  - the debug heavy tier is holding release budgets, in which case the gate is wrong;
  - or the debug build really does miss them.

  `iterm2` passes the same budget in release (run 3). The two pane-shown tests **were not run in release at all**:
  `m8-accept.sh`'s filter `/held_j_stops_on_release_/` does not match `held_j_with_the_pane_shown_stops_on_release_*`
  (`scripts/m8-accept.sh`, the FILTER line in step 2). So their release verdict is **unverified**.

### 2. Release perf — `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture`

13 passed and **8 failed**, in 110 s. Every failure is the p99 gate (`frames/verdict.rs:75`, `frames/journal_find.rs:163`).
The table compares with M7's milestone run, `docs/perf/m7-review-release.txt` (HEAD 4d7645ce, which had load 3.97):

| Test (`frames::…`) | M7 p50 / p99 µs | **M8 p50 / p99 µs** | over 16 ms |
|---|---|---|---|
| `fifty_thousand_refs_…_at_the_pixel_tier` | 1,325 / 4,097 | **13,913 / 29,801** | 178 / 420 |
| `fifty_thousand_refs_…_at_the_unicode_tier` | 996 / 2,238 | **8,212 / 22,384** | 82 / 420 |
| `fifty_thousand_refs_…_scrolling_the_diff` | 1,393 / 1,622 | **7,836 / 22,406** | 72 / 422 |
| `fifty_thousand_refs_…_with_the_detail_pane_open` | 1,406 / 1,630 | **8,248 / 22,359** | 68 / 421 |
| `fifty_thousand_refs_…_with_the_map_view_open` | 987 / 1,072 | **7,804 / 21,727** | 59 / 421 |
| `scrolling_the_100k_fixture_…_at_the_pixel_tier` | 1,349 / 3,244 | **14,145 / 28,223** | 175 / 420 |
| `scrolling_the_100k_fixture_…_at_the_unicode_tier` | 1,622 / 1,994 | **7,505 / 20,148** | 70 / 420 |
| `journal_find::j_with_a_broad_find_holds_the_frame_budget` | 7,297–7,516 / 9,864–10,442 (`30p-journal-find.txt`, M7 30p) | **15,371 / 30,903** | 61 / 127 |

- These all passed at M7. p50 is now 5–10x what it was, on every tier and every pane. The per-key lines show the
  **own frame** at 13–24 ms (for example "key 15: 24.49 ms … own frame 24.26 ms", map view), so the time is in the frame,
  not in a queue. That makes noise an unlikely explanation (the M3 gap at KNOWN_GAPS.md:211 is a max swinging around
  16 ms, not a 7x p50).
- **Passed, unchanged from M7:** all five `heavy::budgets_hold_*` first-paint budgets. First paint is 15.8–40.6 ms
  against M7's 14.4–44.1 ms. Both `budgets_hold_at_pixel_on_monorepo_*` passed (peak RSS 199 / 231 MB).
  `frames::after_idle::g_after_idle` took 9.4 ms and passed.
- **Cause: not diagnosed** (this run is read-only). It is a **question for the debugger**: which M8 commit moved the
  frame cost. The first candidate to measure is anything in M8 that runs per frame (pace/settle 02–03, the look 06–08).
  That is a lead, not a finding.

### 3. `scripts/m8-accept.sh --perf`, ticket 28 box :66

- Step 1, the fast tier: 6,349 tests passed (4 slow) and 15 were skipped, in 457.5 s. The slowest-test names are not
  in the output, because nextest prints only failures.
- Step 2, scroll in release: 11 tests run, 9 passed, **2 failed**:
  - `held_key::held_j_stops_on_release_pixel` (raw Kitty at 40 MB/s): `tail_ms=1253` and `bytes_per_frame_held=22843143`.
  - `held_key::held_j_stops_on_release_pixel_free` (raw Kitty with a free drain): `bytes_per_frame_held=5606552`.

  Both are raw Kitty, the M7/46-kitty-no-deflate family (KNOWN_GAPS.md:1087, **broken**, user-accepted). But
  TESTING.md:3176-3179 says right looks like "exactly three items `FAILED` and nothing else — A `tail_ms<=100`, …;
  every other raw item `ok`". Two things in this run fall outside that:
  - the `bytes_per_frame_held` item also fails;
  - `…_pixel_free` is not in that list at all.

  **Question:** is the TESTING.md line stale (written before the free-rate variant and the bytes check), or has the gap
  widened?
- Step 3, the contact sheet: 98 captures, 0 without a caption, in 3 parts (176 / 178 / 48 KB; all under the 200 KB cap).
- Step 4, install: quoted. It shows `brew style: FAILED` (the known gap M8/28-release-docs-stale).
- Step 5, journeys: 31 in the fence, none missing a walk.

**Box :66 ("J1/J2 tails, J3, RSS lines quoted with their budgets, all within") is met on the numbers but not tickable as written:**

| Measure | Value | Budget | Status |
|---|---|---|---|
| J1 tails: deflate 40 MB/s, deflate free | 15 ms, 4 ms | ≤100 ms | within |
| J1 tail: iTerm2 | 20 ms | ≤100 ms | within |
| J1 tails: Unicode 500 KB/s, 40 MB/s, free | 40, 3, 4 ms | ≤100 ms | within |
| J3 `g_after_idle` | 13.4 ms | test passed | within |
| RSS after held `j` at Pixel | 209 / 221 MiB | `PEAK_RSS_LIMIT` 300 MiB, `heavy.rs:30` | within |
| J2 | no test | — | known gap M8/28-j2-wheel-tail |

- J3's line is **not quoted in `docs/perf/m8-accept.txt`**: the grep at `scripts/m8-accept.sh:68`
  (`'tail|after release|RSS|rss|budget'`) does not match `frames: g_after_idle … latency_us=`. I read J3 from the raw
  `.scratch/m8-polish-and-ship/accept/perf.txt`.
- The summary also carries the two raw-Kitty fails above.

**Suggested:** add `latency_us` to that grep, and decide whether the raw pair is excluded from the box or listed as
known-gap lines. Verdict: **open, one edit from tickable.** Gap M8/28-perf-quiet-run can move to "run, J3 unquoted".

### 4. Trace follow-up ×2, ticket 05 box :435

Ticket 05 amendment 1's shape: `scripts/m8-trace-followup.sh`, MONOREPO, 222x70, Kitty with deflate, both runs on AC.

| Target | Run 1 / run 2 | Status |
|---|---|---|
| emphasis bytes ≤1,078,750 | 176,656 / 176,656 | met |
| bytes_per_key ≤46,054 | 13,096 / 13,096 | met |
| bytes p95 ≤288,075 | 125,744 / 125,744 | met |
| bytes p99 ≤486,064 | 439,220 / 439,220 | met |
| draw_us p50 ≤3,172 | **3,802 / 3,874** | **missed** |
| draw_us p95 ≤15,459 | 7,890 / 9,500 | met |
| draw_us p99 ≤19,965 | **26,359 / 26,572** | **missed** |
| after_last_motion | 0 | met |
| deflate=yes, broken=no, bit-31 split | yes | met |

I ran a diagnostic run 3 to capture the over-bound frame lines the ruling asks for. It was not counted, and its trace
is not committed. Its numbers matched: p50 3,833, p99 26,579.
- **p99 is a single frame**, before the first key (`motions=0`), carrying the strip load:
  `frame … motions=0 … draw_us=26579 bytes=439220 write_us=4591`.
  It is the same kind of frame (load, `motions=0`, 439,220 B) as amendment 1 run 1's p99 (`m8-trace-followup.txt:156`).
- **p50:** 40 of 56 frames are over 3,172 µs. That is a distribution, not one frame.
- **Verdict: box :435 stays open.** On a near-quiet machine both runs miss p50 and p99. So this no longer looks like the
  load-7.3 noise that gap M8/05-draw-tail-quiet-run assumed. Amendment 1's run 2 (eee492a7) drew p50 986 / p99 12,740
  on identical bytes. Today's binary at 58367ddd draws 3.8–3.9 ms at p50. Whether code since eee492a7 or the machine
  moved it is **not measured**. Run 2 of the release perf above points the same way. Not tuned (rule 2). For the
  conductor and the user.

## Not run

- Fuzzing and benchmarks: none are part of this close-run brief.
- The release `held_key` pane-shown pair (see run 1).
