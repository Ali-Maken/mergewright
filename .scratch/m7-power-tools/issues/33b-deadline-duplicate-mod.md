# 33b — `deadline.rs` declared twice: heavy-tests clippy red

Status: ready
Milestone: M7 (m7-power-tools)
Source: 12f builder report (2026-09-27). Filed by the conductor as a gate break, not a spec change.
Blocked by: none (33 merged 7846ec18, 12f merged).
Crates: graft (tests only: `tests/it/`).
Build with: Sonnet. Mechanical.
Scout: none; the lines are named.

## What is wrong

`cargo clippy --all-targets --features heavy-tests -- -D warnings` fails with `duplicate_mod`: `tests/it/deadline.rs` is declared by `mod` in both `tests/it/budget_run.rs:37` and `tests/it/app_harness.rs:25`, since 33 (acf45986). The default gates don't build `budget_run.rs`, so they stayed green.

## Acceptance

- [ ] Red quoted: the clippy command above, before the fix.
- [ ] One declaration of `deadline` (where the rest of `tests/it` shares helpers); both users reach it. Clippy with `--features heavy-tests` green; also with `heavy-1m` as 12d ran it.
- [ ] `app_harness::pump_fails_on_a_stream_that_never_settles` and `budget_run` still green (name the runs).
- [ ] MAP.md if it lists `deadline.rs`.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings` (default and heavy-tests), nextest, `test --doc`.

## Touches

Every `mod deadline` / `deadline::` use (grep). No product code, captures, keymap, journal.
