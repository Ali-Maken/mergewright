# 13b submodule-rows — the rest of 13: status parse, the uncommitted row, the diff pane on a submodule path

Status: draft
Build with: Opus (unchanged from 13).
Needs: `graft_ops::submodule_log`, `SubmoduleLog` (13a); 13a's NOTES (what the read returns when not set up / missing).
Blocked by: **13a**. Wave: see `.scratch/m7-power-tools/parallel-plan.md`.
Split by: planner, 2026-09-27, from `13-submodule-truth.md`. No new decision; 13 "Decided" binds.
Crates: graft-git, graft-ui, graft (graft-ops only where it reads `StatusEntry.submodule`).
Files: crates/graft-git/src/status* (the porcelain v2 parse; builder names the file) and its tests; the
`parse_porcelain_v2` fuzz corpus directory (new `S` records); every reader of `StatusEntry.submodule` (the scout's
item 1 lists them; each is in this list by name in NOTES before editing — a reader in a file another running ticket
holds → `blocked: needs <file>`); crates/graft-ui/src/{detail.rs, detail_pane/files.rs, detail_pane/body.rs,
detail_pane/tests.rs}; crates/graft/src/{jobs/detail.rs, jobs/detail/working_tree.rs, working_tree.rs,
view/detail.rs}; crates/graft/tests/it/submodule_truth.rs (new, seam); docs/shots/m7/submodule-{row,diff}-120x40.txt;
MAP.md (graft-git, graft-ops, graft-ui, graft); TESTING.md.
Scout: 13's scout, brief items 1 and 2.

## Scope
Every box of 13 "Acceptance" not ticked by 13a; 13 "Touches" in full. The diff pane calls `submodule_log` through
the existing file-diff read (`jobs/detail.rs`); a new job kind is outside this ticket's files → `blocked`.

## Acceptance
13's boxes, ticked in 13's file, each naming its test. Gates workspace-wide.

## Size
~45k.
