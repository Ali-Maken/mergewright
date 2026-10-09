# 43 — fixes from review m8-health-1

Status: ready
Build with: Sonnet. Both fixes are spelled out in `docs/reviews/m8-health-1.md` M1/M2; nothing here decides a product question.
Source: `docs/reviews/m8-health-1.md` Majors M1, M2 (conductor, as found, no new decision).
Blocked by: 37 Addition merged (same crate; conductor dispatches after).

## Crate set

graft: `update/tests/purity.rs`, `jobs/runner/*.rs` (imports and `super::` paths only).

## Boxes (each names its test or check when ticked)

- [ ] M1 — the purity guard reads the whole of every scanned `mod.rs`, not up to the first `#[cfg(test)]` (`purity.rs:76-81`); `update/mod.rs` lines after `:91` (incl. `pub fn update`) are scanned. Test: a planted impure call after an inline `#[cfg(test)]` item in a fixture goes red with the old logic, green with the new; name it. Doc comment at the top of `purity.rs` says what is skipped and why.
- [ ] M2 — `runner/mod.rs:10-14` re-imports of `jobs` modules removed; the 47 `super::X::` calls that mean `crate::jobs::X` spelled as such (e.g. `runner/reads.rs:129`). Move-only otherwise. Check: `rg -n 'super::' crates/graft/src/jobs/runner` shows only real parent-module references, pasted count before/after.
- [ ] Gates workspace-wide green; MAP.md lines for the touched files still true (`map_coverage` green).

Out of scope: Minors m1–m7 and Q1–Q2 (conductor routes to gaps / the stop).

Touches: list every file and how checked (CLAUDE.md, Running work).
