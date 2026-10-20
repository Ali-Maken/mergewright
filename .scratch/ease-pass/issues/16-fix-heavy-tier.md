# fix round: heavy tier compiles, clippy clean, 16 failures triaged

Status: done (14 heavy failures left as questions)
Blocked by: —
Build with: Sonnet (builder-sonnet, model sonnet); fails twice → Opus

From the combined review, **Major 4** (`docs/reviews/ease-pass-07-14.md:160-180`, failure list under Gates → "Heavy tier").

## Scope
1. Add the missing `HistoryUpdate::FoldedHits { .. }` arms (`tests/it/heavy.rs:321,347`).
2. Split the three functions over clippy's line limit (`frames/journal_find.rs:253`, `frames/run.rs:207`, `pieces_shot.rs:70`). Don't raise a limit.
3. Run the heavy tier (fast tier only on a laptop-safe filter where possible; one heavy run, script to `docs/perf/16-heavy.txt`). For each of the 16 failures: fix it **only** if the cause is shown by a line and the fix needs no decision; otherwise record it in NOTES as a question with its line. **Never loosen a budget or an expectation to make it pass.** Specifically decisions, not fixes: what "every commit laid out" means under the default fold (rows ≠ commits), idle `FoldedCounted` wakes vs `frames.rs:393`, Pixel byte verdicts, PNG re-shoots.
4. Do not touch: `history/seeds.rs`, `view/entries/branch_column.rs`, `update/branch_pane.rs`, `mergewright-ops/src/branch_status.rs` (ticket 15 holds them).

## Acceptance (tick each with its test/command)
- [x] `cargo clippy --all-targets --features heavy-tests -- -D warnings` clean (ran, exit 0).
- [x] Heavy tier compiles and runs; per-failure table in NOTES (docs/perf/16-heavy.txt: 14 failed after run, table below; resolver fixed and re-run green).
- [x] Standard four gates green (fmt, clippy, nextest 7199/7199, doc 3/3).

## NOTES (builder)
Status: partial-done; Scout: none (known files, 3 mechanical splits). No compaction.
Heavy run: `scripts/ease-16-heavy.sh` -> `docs/perf/16-heavy.txt` (1721 s, --test-threads=2, load from other builders). 14 failures remain after the compile fix (the review's 16; the two `fifty_thousand_refs` frames tests passed this run, the resolver one fixed). Note: an earlier run with a long `TMPDIR` showed ~70 extra askpass failures ("path must be shorter than SUN_LEN"): environment, not code; run the tier with the default TMPDIR.

| test | cause line | result |
|---|---|---|
| resolver::frame_budget::inline_at_187_and_200_columns… | frame_budget.rs:140 asserted `◀`; test takes blocks theirs, inline view marks them `▶ took theirs` (conflict_resolver.rs:8-11) | FIXED (re-pointed to the `▶`+`took theirs` mark; budget untouched: p99 3.2/3.8 ms, 0 late) |
| heavy::budgets_hold_* x5 | heavy.rs:585 `rows 79195 != 100000` under default fold | QUESTION: what does "every commit laid out" mean when the fold draws fewer rows than commits? (rows vs commit count; the budgets after line 585 are not reached) |
| frames::after_idle::g_after_idle | after_idle.rs:58 waits for 100000 rows, gets 79195, 60 s | QUESTION: same as above |
| frames::scrolling_the_100k_fixture…_{pixel,unicode} x2 | frames.rs:393 five idle wakes, all `Produced::FoldedCounted` | QUESTION: should idle fold counts wake the loop, or is the test's "no wake at rest" rule to change? |
| held_key::{held_j_stops_on_release_pixel, _pixel_free, j_then_journal_opens_at_once_pixel} x3 | held_key.rs:259 `bytes_per_frame_held<=512KiB` FAILED: 4.6 MB / 1.8 MB / 1.8 MB per held frame (debug) | QUESTION: Pixel byte verdict; budget stays, cause not established (13's role strokes likeliest) |
| ghostty_shot::fix_scroll_ghostty_png | ghostty_shot.rs:77 committed 10128 B, drawn 1720 B | QUESTION: PNG re-shoot (user judges by eye) |
| pieces_shot::emphasis_pieces_png | pieces_shot.rs:62 committed 34406 B, drawn 29756 B | QUESTION: PNG re-shoot |

Also: `over_400.txt` rows raised by the exact growth of the three touched files (journal_find 466->472, run 413->424, heavy 638->640); helpers add signature lines.
