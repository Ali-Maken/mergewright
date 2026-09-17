# 80 — 1M tests gate on a build feature, not an environment variable

Status: ready
Blocked by: none
Build with: Sonnet

## Why

Round-2 review Minor m4 (`docs/reviews/m1-the-graph-round-2.md` §m4): the two 1M frame tests in `crates/graft/tests/it/frames.rs` guard on `GRAFT_1M` (`const ONE_MILLION` :108) and **fail** when it is unset, so the documented release perf command is red as written. User, 2026-09-17: "behind a build feature, matching ADR-0018. Tests gate on features, never on the environment."

## Scope

In:
- A new cargo feature `heavy-1m` on the crate owning `tests/it/frames.rs`, implying `heavy-tests`. The two 1M tests are `#[cfg(feature = "heavy-1m")]`; the `GRAFT_1M` constant and every env read of it are removed. No `#[ignore]` (ADR-0018).
- Any other `GRAFT_1M` read anywhere in the repo (`git grep -n GRAFT_1M`): code, scripts, TESTING.md, docs, PROGRESS "how to run" lines → the feature. Tickets under `.scratch/` and review files are history: leave them.
- ADR-0018: an **Amendment (2026-09-17, m1/80)** section: `heavy-1m` is a second feature, implies `heavy-tests`, never in `default`, gates only the 1M-commit tests; the environment never gates a test. Commands: `cargo test --release --features heavy-tests …` (budgets, no 1M) and `… --features heavy-1m …` (adds 1M).
- KNOWN_GAPS / PROGRESS open question m4 → resolved line pointing at this ticket.

Out: any change to budgets, fixtures or test bodies; running the heavy tier or 1M tests (measurement never runs in a helper's context; the next milestone review runs them).

## Acceptance

- [ ] `git grep -n GRAFT_1M -- ':!.scratch' ':!docs/reviews'` is empty.
- [ ] Quiet gates pass: `cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D warnings`, `cargo test --quiet`.
- [ ] Compile-only proof both features build their tests without running them: `cargo test --release --features heavy-1m --no-run --quiet` succeeds (report that line only).
- [ ] `cargo test --features heavy-tests -- --list 2>/dev/null | grep -c <1M test name>` is 0 and with `heavy-1m` is 1 per test (`--list` doesn't run tests; use the debug profile to avoid a second release build if the no-run above already covers release).
- [ ] ADR-0018 amendment written; commands in TESTING.md match it.

## Read first

- `docs/adr/0018-*.md` whole (short).
- `docs/reviews/m1-the-graph-round-2.md` §m4 only (grep `m4.`).
- `crates/graft/Cargo.toml` `[features]`; `crates/graft/tests/it/frames.rs` around each `ONE_MILLION` use (grep -n, then read ±20 lines).
