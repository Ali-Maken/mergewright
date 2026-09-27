# 11a pick-refusals — 11's graft-ops part: `check_pick` refuses what it can't take; set-aside for a pick

Status: draft
Build with: Opus. Two refusals plus an ADR-0045 extension (a new set-aside kind: undo must hold).
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md` (after 10b and 05: all three hold
graft-ops `operation.rs` / `operation/history.rs`).
Split by: planner, 2026-09-27, from `11-cherry-pick-key.md` (ADR-0051 §1). 11 keeps the `o` route, the `.` slot-2
label and dim, the app tests, walks and captures.
Crates: graft-ops only.
Files: crates/graft-ops/src/{operation.rs, operation/history.rs, operation/history/tests.rs}; crates/graft-ops/src/
preview/put_back.rs (only if the set-aside forecast needs a CherryPick arm); crates/graft-ops/tests/it/
cherry_pick.rs (new, seam); MAP.md (graft-ops). A graft exhaustive match on `Refusal` that breaks, or a graft test that
asserted today's pick-over-edits behaviour → `blocked: needs <graft file>`, stop.
Scout: 11's scout (ADR-0035), brief items 1 and 4. Item 1 answers KNOWN_GAPS :806's question with its line.

## Scope
11 "Decided": the two `Refusal`s with their words; "Unsaved edits the pick touches" (set aside and put back, one entry)
**only if** the conductor confirms "Where I'd differ" 4 is approved — else nothing is set aside and the refusal words
11 names stand. 11 "Signatures", graft-ops block.

## Acceptance (tick 11's graft-ops box in 11's file, each test named)
- [ ] The five graft-ops tests of 11 "Acceptance" (the set-aside one only if approved).
- [ ] All set-aside tests green (`cargo nextest run --workspace -E 'test(/set_aside|put_back/)'`).
- [ ] NOTES: KNOWN_GAPS :806 answered with its line. ADR-0045 amendment rows owed (the conductor). MAP.md (graft-ops).
- [ ] Gates workspace-wide.

## Size
~35k.
