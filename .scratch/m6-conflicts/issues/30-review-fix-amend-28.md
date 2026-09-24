# review-fix-amend-28: `jobs/conflict.rs` over size with a third fixture copy; the app still decides the whole-file start-over itself

Status: ready
Blocked by: 29-review-fix-amend-18.md (building; same crate, ADR-0036). Runs before 16.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-28.md` Major 1 and one Minor (`update/resolver/restart.rs:87`). The Minor is promoted because it breaks Amendment 3's hard-to-reverse choice 1 ("ops' rules", no app mirror).
Build with: **Opus** (low).
Needs: none.
Crates: graft (graft-ops only if a shared test fixture belongs in graft-fixtures — then graft-fixtures too; say so in NOTES).

## Read first
- `docs/reviews/m6-amend-28.md`: Major 1, and the Minor on `update/resolver/restart.rs:87`.
- `docs/design-notes/m6-conflicts.md` Amendment 3, "Hard-to-reverse choices" 1.
- `crates/graft/MAP.md` (`jobs::conflict`, `update::resolver::restart`); `ARCHITECTURE.md` and `codebase-design` for the move.

## Scope
In:
1. **Major 1.** `crates/graft/src/jobs/conflict.rs` grew 429 → 602 lines, mostly a third copy of the merge fixture (`:455–528`; the others are at `tests/it/resolver/restart.rs:222` and `graft-ops/tests/it/restart_gate.rs:25`). Remove the copy: one shared fixture that the crate rules allow (graft-fixtures if it fits there). Tests move to their own file if still needed. Target ~400 lines, or no-seam with the count. Pure moves; tests pass unchanged apart from paths.
2. **Mirror.** `update/resolver/restart.rs:87` decides the whole-file case in the app and refuses binary `UU`/`AA`, where ops (`graft-ops/src/operation/conflict.rs:248`) allows it. Failing test first (a binary `UU` unmerged file: the app refuses, ops would accept). Fix: the app asks ops' rule (the gate or `check_restart`) instead of its own predicate. If a binary unmerged file cannot reach this path from the UI, say so with the line, and remove the app predicate anyway.

Out: the review's other Minors (→ KNOWN_GAPS).

## Acceptance (each tick names its test, at tick time)
- [ ] M1: fixture copy gone, `jobs/conflict.rs` line count in NOTES — tests: (names)
- [ ] Mirror: failing test first, red on unchanged code — test:
- [ ] Mirror: the app's answer equals ops' answer for binary `UU`/`AA` — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
