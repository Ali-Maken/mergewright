# octopus-merge-state: an octopus merge in progress makes every journaled operation fail to begin

Status: ready
Blocked by: 20 (Part A `bf218b44` + Part B, merged together). Same crate as nothing else then building.
Was: filed by the conductor 2026-09-24 from 20 Part A's hand-back. A regression Part A introduced; it was not there at `09db5433`. Found by reading; no test yet.
Build with: **Opus** (medium) — git writes, undo.
Needs: 20 merged.
Crates: graft-ops (graft-core only if the journal's `MergeState` shape must change).

## The defect (lines from the Part A builder)
- `crates/graft-ops/src/merge_state.rs:196`: a `MERGE_HEAD` with more than one line → `Octopus` error.
- `merge_state.rs:86`: `read_merge_state` propagates it with `?`.
- `apply/record.rs:66-67`: `begin` maps it to `PrepareError` with `?`, so **every** entry's `begin` fails while an octopus merge is in progress (e.g. `git merge --no-commit a b`). Reachable from the shell; rare.

## Scope
In:
1. Failing test first (real git: two branches, `git merge --no-commit --no-ff a b` → `MERGE_HEAD` has 2 lines). Run any journaled op (e.g. a stage or a branch create) → today `PrepareError`. Red on unchanged code.
2. Fix at the cause: reading the merge state never blocks an unrelated operation. The recorded state must hold an octopus `MERGE_HEAD` **byte-exact**, so undo puts it back as it was. Test: after `u`, `MERGE_HEAD` is byte-equal, both lines.
3. `FinishMerge` (`c`) on an octopus merge: finish it or refuse it with a named refusal (never `PrepareError`). Say which in NOTES and why. Test that.
4. MAP.md (graft-ops §merge_state, §apply).

Out: an octopus fixture in `graft-fixtures` beyond what the tests need.

## Acceptance (each tick names its test, at tick time)
- [ ] Failing test first, red on unchanged code — test:
- [ ] An unrelated op begins and records during an octopus merge — test:
- [ ] Undo restores the 2-line `MERGE_HEAD` byte-exact — test:
- [ ] `c` on an octopus merge: finishes or names its refusal — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
