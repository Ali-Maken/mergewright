# 43 — fixes from review m8-health-1

Status: done
Build with: Sonnet. Both fixes are spelled out in `docs/reviews/m8-health-1.md` M1/M2; nothing here decides a product question.
Source: `docs/reviews/m8-health-1.md` Majors M1, M2 (conductor, as found, no new decision).
Blocked by: 37 Addition merged (same crate; conductor dispatches after).

## Crate set

graft: `update/tests/purity.rs`, `jobs/runner/*.rs` (imports and `super::` paths only).

## Boxes (each names its test or check when ticked)

- [x] M1 — the purity guard reads the whole of every scanned `mod.rs`, not up to the first `#[cfg(test)]` (`purity.rs:76-81`); `update/mod.rs` lines after `:91` (incl. `pub fn update`) are scanned. Test: a planted impure call after an inline `#[cfg(test)]` item in a fixture goes red with the old logic, green with the new; name it. Doc comment at the top of `purity.rs` says what is skipped and why. **Ticked:** `update::tests::purity::a_mid_file_cfg_test_item_does_not_end_the_scan` — seen red on the old `split_once` ("the scan dropped `update`s body: "use a::b;\n""), green on `without_tests_module` (cuts at the last `mod tests` line via `declares_tests`).
- [x] M2 — `runner/mod.rs:10-14` re-imports of `jobs` modules removed; the 47 `super::X::` calls that mean `crate::jobs::X` spelled as such (e.g. `runner/reads.rs:129`). Move-only otherwise. Check: `rg -n 'super::' crates/graft/src/jobs/runner` shows only real parent-module references, pasted count before/after. **Ticked:** before 64 `super::` lines (47 module paths); after 17: mod.rs 12 (real `jobs` items: `MAX_WORKERS`, `pool`, `types`, `askpass`, `SHUTDOWN_GRACE`, `FORCED_WAIT`) + one `use super::JobRunner` in each of the 5 children.
- [x] Gates workspace-wide green; MAP.md lines for the touched files still true (`map_coverage` green). **Ticked:** fmt/clippy clean; nextest 6230/6230 incl. `map_coverage`; doctests ok. MAP purity line gained the new test.

Out of scope: Minors m1–m7 and Q1–Q2 (conductor routes to gaps / the stop).

Touches: list every file and how checked (CLAUDE.md, Running work).

## NOTES

Touches:
- `crates/graft/src/update/tests/purity.rs` — new helper + test; `update_cannot_reach_the_world` green now scanning `update/mod.rs` whole up to `mod tests;`.
- `crates/graft/src/jobs/runner/{mod,lists,operations,queue,reads}.rs` — path spelling only (sed of the 19 names in the deleted `use`); clippy/nextest green; `dispatch.rs` had none.
- `crates/graft/MAP.md` purity line — checked by `map_coverage`.
- No other callers: the deleted `use` was private to `runner` (build proves it).
Scout: none (review citations re-read at HEAD). No compaction.
