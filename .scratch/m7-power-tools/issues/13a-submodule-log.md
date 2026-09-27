# 13a submodule-log — 13's graft-ops read: the commits a submodule pointer move pulled in or rewound

Status: draft
Build with: Opus. Correctness against git's own submodule states, judged by real repos with real submodules.
Needs: none.
Blocked by: none. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `13-submodule-truth.md` (ADR-0051 §1: an additive graft-ops read that stands
alone). 13b builds the rest of 13.
Crates: graft-ops, graft-fixtures.
Files: crates/graft-ops/src/submodule.rs (new, seam); crates/graft-ops/tests/it/submodule_log.rs (new, seam);
crates/graft-fixtures/src/shapes/submodule.rs (new, seam; only if the scout finds no fixture makes a repo with a
submodule); MAP.md (graft-ops, graft-fixtures). Outside this list → `blocked: needs <file>`.
Scout: 13's scout (ADR-0035), brief item 3 (can `graft-fixtures` make a repo with a submodule: a local file URL,
`protocol.file.allow=always`). Items 1–2 go to 13b's builder with the same scout file.

## Scope
13 "Signatures", graft-ops block: `SubmoduleLog`, `submodule_log`, from `git diff --submodule=log` as 13 "Decided"
(the diff pane) describes. The cap of 50 and `+N more` are drawn by 13b; the read returns every row.
- NOTES records, measured, what `submodule_log` returns for a submodule **not set up** and for **missing commits**, so
  13b maps both to `the submodule's commits aren't here · not set up`.

## Acceptance (tick 13's boxes in 13's file, each test named)
- [ ] `submodule_log_lists_pulled_in_and_rewound`; `submodule_log_honours_stop`.
- [ ] The fixture (if added): one test that it makes a gitlink with a checked-out submodule.
- [ ] MAP.md (graft-ops; graft-fixtures if touched). Gates workspace-wide.

## Size
~30k.
