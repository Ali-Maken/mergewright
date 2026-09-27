# 20o — Split `landing.rs`; Ctrl+R retries a crashed evil probe

Status: done
Milestone: M7 (m7-power-tools)
Source: review `docs/reviews/m7-33c-to-34.md` M1 and m1 (filed by the conductor as a review fix; scope is the review's).
Blocked by: 14 (holds `update/detail/landing/tests.rs`). Start once 14 merges.
Crates: graft only.
Files: crates/graft/src/update/detail/{landing.rs, landing/disk.rs*, landing/select.rs*, landing/tests.rs, landing/tests/*, ops.rs}; the evil-probe test file (update/tests.rs or update/detail tests — name it); MAP.md (graft).
Build with: Sonnet. A move-only split plus a one-line reset with its test.
Scout: none; the review names the lines.

## Acceptance

- [x] M1: `landing.rs` split into `landing/disk.rs` (`DiskLine`, `disk_line_in`, `index_line`) and `landing/select.rs` (`select`, `row_is`, `unfold_to`, `say_missing`). Move-only: show the `--color-moved` check; list every non-move line. `wc -l` each file, none over the PROMPT.md 6.2 limit.
- [x] M1: `disk_line_in` gets a worked example in its doc (a doctest, if it fits the crate's doctest pattern).
- [x] m1 red first: a test that crashes the evil probe, presses Ctrl+R, and asserts the probe is asked again once (quote the red). Then `DetailState::refresh` (`ops.rs:72`) resets a `Failed` evil so the guard at `:187` lets it through. Also assert that moving off and back still retries, and that no loop happens (5 `j`/`k` → one ask).
- [x] Existing landing, esc_back and radar tests green, unchanged in body.
- [x] MAP.md graft updated.
- [x] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches

`landing::` callers (compiler), `evil` readers (grep). No captures, keymap or journal.

## NOTES (builder, 2026-09-27)

Scout: none — the review named the lines.

- Split tick: `git diff --color-moved=plain` shows only these non-move lines: landing.rs imports pruned; `mod disk; mod select;`; `use disk::index_line; use select::select;`; `#[cfg(test)] use disk::{DiskLine, disk_line_in};` (for `landing/tests/disk.rs`); new files' `//!` headers and `use` lines; `pub(super)` on `select`, `row_is`, `index_line`. `wc -l`: landing.rs 374, disk.rs 113, select.rs 126, tests.rs 390. Verifying tests: `landing::tests::*`, `landing::tests::disk::*`, unchanged, green.
- Doc tick: `disk_line_in` example is a `text` block, not a doctest — item is `pub(crate)`, `graft` has no doctests; same cases in `landing/tests/disk.rs` (`disk_line_in_below_the_hunks_shifts_by_them` etc.).
- m1 tick: `update::tests::ctrl_r_retries_a_crashed_evil_probe_once`. Red before fix: `assertion left == right failed ... left: 0 right: 1` (Ctrl+R's commands carried no `EvilProbe`). Fix (cause): `DetailState::refresh` resets a `Failed` evil to `Idle`. Same test: 5 further events → 0 probes. Move off/back: `a_failed_evil_probe_is_not_asked_again_for_the_same_commit` (existing, unchanged, green).
- Side effect: every write's refresh (operate.rs, relaunch.rs callers of `refresh`) also retries a crashed probe once. Intended — a write is a new ask.
- Gates workspace-wide: fmt, clippy, nextest 4337/4337, test --doc pass.
