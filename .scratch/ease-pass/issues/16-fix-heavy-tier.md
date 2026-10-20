# fix round: heavy tier compiles, clippy clean, 16 failures triaged

Status: ready
Blocked by: —
Build with: Sonnet (builder-sonnet, model sonnet); fails twice → Opus

From the combined review, **Major 4** (`docs/reviews/ease-pass-07-14.md:160-180`, failure list under Gates → "Heavy tier").

## Scope
1. Add the missing `HistoryUpdate::FoldedHits { .. }` arms (`tests/it/heavy.rs:321,347`).
2. Split the three functions over clippy's line limit (`frames/journal_find.rs:253`, `frames/run.rs:207`, `pieces_shot.rs:70`). Don't raise a limit.
3. Run the heavy tier (fast tier only on a laptop-safe filter where possible; one heavy run, script to `docs/perf/16-heavy.txt`). For each of the 16 failures: fix it **only** if the cause is shown by a line and the fix needs no decision; otherwise record it in NOTES as a question with its line. **Never loosen a budget or an expectation to make it pass.** Specifically decisions, not fixes: what "every commit laid out" means under the default fold (rows ≠ commits), idle `FoldedCounted` wakes vs `frames.rs:393`, Pixel byte verdicts, PNG re-shoots.
4. Do not touch: `history/seeds.rs`, `view/entries/branch_column.rs`, `update/branch_pane.rs`, `mergewright-ops/src/branch_status.rs` (ticket 15 holds them).

## Acceptance (tick each with its test/command)
- [ ] `cargo clippy --all-targets --features heavy-tests -- -D warnings` clean.
- [ ] Heavy tier compiles and runs; per-failure table in NOTES: test · cause line · fixed / question.
- [ ] Standard four gates green.
