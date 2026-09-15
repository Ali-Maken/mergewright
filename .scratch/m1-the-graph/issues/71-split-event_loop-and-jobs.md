# m1-the-graph/71 — Review M1: split event_loop and jobs under the 400-line rule

Status: done

## Result
- event_loop/mod.rs: 1787 (766 non-test) → mod.rs 210, startup.rs 228, headless.rs 116, running.rs 275 (all non-test, all ≤400); tests moved verbatim to tests.rs (1028 lines).
- jobs/mod.rs: 803 → mod.rs 94, types.rs 108, pool.rs 198, kinds.rs 129, runner.rs 359 (all ≤400); jobs/tests.rs (already external, 581 lines) untouched.
- All gates green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features heavy-tests -- -D warnings`, `cargo test` (full workspace, 0 failures).
- Public API unchanged: `event_loop::{run, pump}`, `jobs::{JobRunner, Outcome, Work, Cancel, JobEvents, Delivery, Dispatch, Shutdown, MAX_WORKERS, SHUTDOWN_GRACE}` all still resolve at the same paths via `pub use`.
- MAP.md updated (event_loop, jobs sections) to note the internal split; ARCHITECTURE.md not touched — no crate-boundary change.
- Files still over 400 non-test lines in crates/graft/src (out of scope for this ticket): view.rs (704), history/mod.rs (713), history/scroll.rs (495), config/mod.rs (431).
- Pre-existing observation, not fixed (out of scope, pure-move ticket): `event_loop/startup.rs`'s `finish()` and `report_stalled_jobs()` swallow a stderr-write failure with `.unwrap_or(())`, moved verbatim from the original file.
Build with: **Sonnet** (medium) — mechanical moves along existing seams; no behaviour change.
Blocked by: 65, 66, 67, 68, 69, 70

From the M1 review round 1, "M1" (PROMPT.md 6.2: files over 400 non-test lines). Split into 70–76, one crate area each, after the fix tickets so moves don't conflict with fixes.

## Read first
- docs/reviews/m1-the-graph.md "M1" only; PROMPT.md §6.1–6.2; ARCHITECTURE.md crate map section.
- The `codebase-design` skill (module changes).
- The crate MAP.md files for: crates/graft/src/event_loop/mod.rs (761), crates/graft/src/jobs/mod.rs (~800).

## Scope
In: split crates/graft/src/event_loop/mod.rs (761), crates/graft/src/jobs/mod.rs (~800), event_loop into startup/headless/loop; jobs by job kind. Each resulting file ≤400 non-test lines, one concern per file. Pure moves plus the visibility changes they need; public API unchanged unless a MAP.md line must move. Update MAP.md, and ARCHITECTURE.md if structure changes. Before finishing, list every file in these crates over 400 non-test lines (`wc`-based, tests excluded) and fix the ones this ticket names; list the rest in the report.
Out: behaviour changes, renames of public items, anything outside the named files' crates.

## Acceptance
- [ ] Named files split; each new file ≤400 non-test lines.
- [ ] No test changed except import paths; all gates green; no snapshot changes.
- [ ] MAP.md (and ARCHITECTURE.md if needed) updated.
- [ ] Report: before/after line counts; any file still over 400 in these crates.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
