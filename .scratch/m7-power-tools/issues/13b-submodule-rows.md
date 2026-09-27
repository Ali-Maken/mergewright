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

## NOTES (builder, 2026-09-27) — blocked before any code
- **Readers of `StatusEntry.submodule`:** none read it (grep `\.submodule\b`: 0 reads). The type change
  `bool → Option<SubmoduleStatus>` (13 "Decided", binding) breaks every struct literal instead: 40 `submodule: false`
  sites in 25 files (grep `submodule: (false|true)` over `crates`, `fuzz`). graft-git `status.rs`, `status/tests.rs`;
  graft-ops `apply/set_aside/put_back.rs`, `operation/tests.rs`, `operation/history/tests.rs`; graft
  `update/operate.rs` (7), `update/selection.rs` (3), `update/tests.rs` (4: lines 2754, 2764, 2774, 4968), and 17 test
  files under `update/**` and `view/**`. Each becomes `submodule: None`.
- **Blocked: needs `crates/graft/src/update/tests.rs`** — held by 16 (parallel-plan wave 2). No design keeps those
  4 literals compiling: any new field or type on `StatusEntry` breaks them (literals, no `..Default`), and a side
  channel off `StatusEntry` contradicts 13 "Decided". 01b's files: no overlap (checked its `Files:` line).
- **Next step:** after 16 merges (or on the conductor's grant of those 4 lines, `false → None` only), start at
  graft-git `status.rs` `is_submodule` (:356) → `submodule_status(field) -> Result<Option<SubmoduleStatus>, _>`
  (`N...` → None, `S<c><m><u>` → flags), red test `parse_reads_each_submodule_flag` first.
- Scout used: `scouts/13.md` items 1–2. No compaction.
