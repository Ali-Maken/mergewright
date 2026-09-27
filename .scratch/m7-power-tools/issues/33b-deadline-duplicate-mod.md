# 33b — `deadline.rs` declared twice: heavy-tests clippy red

Status: done
Milestone: M7 (m7-power-tools)
Source: 12f builder report (2026-09-27). Filed by the conductor as a gate break, not a spec change.
Blocked by: none (33 merged 7846ec18, 12f merged).
Crates: graft (tests only: `tests/it/`).
Build with: Sonnet. Mechanical.
Scout: none; the lines are named.

## What is wrong

`cargo clippy --all-targets --features heavy-tests -- -D warnings` fails with `duplicate_mod`: `tests/it/deadline.rs` is declared by `mod` in both `tests/it/budget_run.rs:37` and `tests/it/app_harness.rs:25`, since 33 (acf45986). The default gates don't build `budget_run.rs`, so they stayed green.

## Acceptance

- [x] Red quoted: `cargo clippy --all-targets --features heavy-tests -- -D warnings` before the fix: `error: file is loaded as a module multiple times: crates/graft/tests/it/deadline.rs` (`clippy::duplicate_mod`), `could not compile graft (test "it")`.
- [x] One declaration of `deadline`: `tests/it/main.rs`; `app_harness` and `budget_run` reach it as `crate::deadline`. Examples that include those files by path declare their own at their root (`budgets.rs`, `m3_walk.rs`, `m4_walk.rs`, `m5_walk.rs`: found by the first attempt's compile error in `m3/m4/m5_walk`). Verified: `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` and `--features heavy-1m`: both clean.
- [x] `cargo nextest run --workspace --features heavy-tests -E 'test(pump_fails) | test(deadline_fits) | ...'`: `app_harness::pump_fails_on_a_stream_that_never_settles` passed. `budget_run` users: `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs` passed run alone (22.8 s, 5/5 of `heavy::budgets_hold*`); it and `many_refs::*_within_budget` failed on timing budgets (e.g. `refs::read took 2.24s, over the 1.5s budget`) when run in parallel in debug with 34 heavy tests: contention, not the deadline. Heavy perf is the review's release run (ADR-0021); not verified here.
- [x] MAP.md: none lists `deadline.rs` (grep of every `MAP.md`).
- [x] Gates workspace-wide: `cargo fmt --check` ok; clippy default, `heavy-tests`, `heavy-1m` clean; `cargo nextest run --workspace` 4177/4177 passed (3 skipped); `cargo test --workspace --doc` 0 failed.

### Added from review m7-18f-32b-06f-33 (conductor, mid-ticket)

- [x] Major 1: as above.
- [x] Major 2: `deadline::DEADLINE` 600 s -> 120 s (fast tier, = harness `PATIENCE`); `budget_run` has its own `TOTAL` = 600 s. Red first: `app_harness::deadline_fits_inside_the_fast_tier` panicked at `app_harness.rs:339` on 600 s; green after. The `budgets` example keeps `deadline` with `#[expect(dead_code)]` (it uses only `Deadline`, not the fast-tier constant).
- [x] Minor 3: `idle_advance.rs` `laid_out_to_the_end`, `refresh_race.rs` `two_refreshes_before_started_leave_one_history_job`, `credential_by_key.rs` `drive` each note events on a `Deadline::of(DEADLINE)`. Verified green by the same filtered run (`test(idle_advance) | test(refresh_race) | test(credential_by_key)`) and the full nextest. No test proves they trip; the trip path is `pump_fails_on_a_stream_that_never_settles`'s.
- [x] Minor 4: `PATIENCE`'s doc comment sits on `const PATIENCE` again (`budget_run.rs`); `use` above it. Checked by reading; no test.

Scout: none (lines named in ticket and review).

## Touches

Every `mod deadline` / `deadline::` use (grep). No product code, captures, keymap, journal.
