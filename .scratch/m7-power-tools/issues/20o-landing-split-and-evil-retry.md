# 20o — Split `landing.rs`; Ctrl+R retries a crashed evil probe

Status: ready
Milestone: M7 (m7-power-tools)
Source: review `docs/reviews/m7-33c-to-34.md` M1 and m1 (filed by the conductor as a review fix; scope is the review's).
Blocked by: 14 (holds `update/detail/landing/tests.rs`). Start once 14 merges.
Crates: graft only.
Files: crates/graft/src/update/detail/{landing.rs, landing/disk.rs*, landing/select.rs*, landing/tests.rs, landing/tests/*, ops.rs}; the evil-probe test file (update/tests.rs or update/detail tests — name it); MAP.md (graft).
Build with: Sonnet. A move-only split plus a one-line reset with its test.
Scout: none; the review names the lines.

## Acceptance

- [ ] M1: `landing.rs` split into `landing/disk.rs` (`DiskLine`, `disk_line_in`, `index_line`) and `landing/select.rs` (`select`, `row_is`, `unfold_to`, `say_missing`). Move-only: show the `--color-moved` check; list every non-move line. `wc -l` each file, none over the PROMPT.md 6.2 limit.
- [ ] M1: `disk_line_in` gets a worked example in its doc (a doctest, if it fits the crate's doctest pattern).
- [ ] m1 red first: a test that crashes the evil probe, presses Ctrl+R, and asserts the probe is asked again once (quote the red). Then `DetailState::refresh` (`ops.rs:72`) resets a `Failed` evil so the guard at `:187` lets it through. Also assert that moving off and back still retries, and that no loop happens (5 `j`/`k` → one ask).
- [ ] Existing landing, esc_back and radar tests green, unchanged in body.
- [ ] MAP.md graft updated.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches

`landing::` callers (compiler), `evil` readers (grep). No captures, keymap or journal.
