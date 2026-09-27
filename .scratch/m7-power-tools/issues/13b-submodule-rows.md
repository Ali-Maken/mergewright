# 13b submodule-rows — the rest of 13: status parse, the uncommitted row, the diff pane on a submodule path

Status: done
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

## NOTES (builder, 2026-09-27) — build, after the conductor's grant
- **Compacted once** mid-ticket (after code + captures, before docs). Nothing remains; all work below is committed.
- **Literal-only changes (`submodule: false` → `submodule: None`), 24 files:** graft-git `status/tests.rs`; graft-ops
  `apply/set_aside/put_back.rs`, `operation/tests.rs`, `operation/history/tests.rs`; graft `update/banner/tests.rs`,
  `update/clash/tests.rs`, `update/commit_editor/tests.rs`, `update/detail/landing/tests.rs`,
  `update/detail/landing/tests/select.rs`, `update/finish/tests.rs`, `update/keys/live_c_tests.rs`, `update/operate.rs`
  (7), `update/place/tests/{navigation,open,pinned_base}.rs`, `update/relaunch/tests.rs`, `update/resolver/tests.rs`
  (2), `update/resolver/tests/file_list.rs`, `update/selection.rs` (3), `update/stash/tests.rs`, `view/frame/tests.rs`,
  `view/resolver/tests.rs`, `view/tests.rs` (5), and **granted: `update/tests.rs` lines 2754, 2764, 2774, 4968 only**
  (checked by diff: nothing else in that file). None of the others is in 16's or 01b's held files.
- **File outside the `Files:` line:** `crates/graft/src/jobs/detail/assemble.rs` — ticket 34 split it out of
  `jobs/detail.rs` after this ticket was written; `push_tree` lives there. Not held by 16 or 01b.
- **Untouched from `Files:`:** graft `working_tree.rs`, `view/detail.rs` (nothing needed).
- **Measured, git 2.54:** worktree pointer move `.M SC..` (diff-files lists it); untracked-only `.M S..U` (diff-files
  silent → row synthesized); changed-inside-only `.M S.M.` (diff-files lists it, 0/0); staged move `M. S...` (C is
  worktree-vs-index only); an uninitialised submodule gets **no** status entry → `submodules_not_set_up` reads
  `ls-files --stage` for `160000` paths with no `<path>/.git`.
- **Clean check (Touches):** a submodule with only untracked inside **counts as dirty / unstaged 1 today**; kept.
  Test `untracked_inside_a_submodule_counts_as_unstaged` (graft-git).
- **space:** `space_on_a_submodule_stages_the_pointer` (new; index gets `160000 <new>`, row moves to STAGED).
  **`d` on a submodule: untested** (unverified, not broken).
- **Stash/set-aside:** only the literal change; full nextest green incl. stash tests.
- **Keymap, journal:** none (no key bound; grep of the diff for `Key`/`journal`: none).
- **Findings:**
  1. Capture Right line can't hold at 120: Below layout gives the file list 30 cols → `▣ libs/parser  submodule ·…`.
     Needs a 187 capture, or a shorter word ladder (e.g. `moved · inside · untracked`). Conductor's call.
  2. A commit's (not the working tree's) gitlink rows still show letter `M`: the commit file list has no modes. Its
     diff pane does get the submodule view (via `pointer_of`).
  3. Exit 128 → `NotHere` (13a's contract); a typed `FactsError` variant would be cleaner (13a proposed it).
  4. Added/removed submodules are worded `new at <id>` / `removed (was <id>)` — invented, 13 names only a move.
  5. The submodule commit list does not scroll (no hunks → body cursor has 0 rows); capped at 50 + `+N more`.
  6. `submodules_not_set_up` failure fails the job (not softened, rule 3).
- **Fuzz `S` records to seed** (`fuzz/corpus/parse_porcelain_v2/`, gitignored, not committed), NUL-terminated:
  `1 .M SC.. 160000 160000 160000 <oid> <oid> libs/parser`; `1 .M S.M. …`; `1 .M S..U …`; `1 M. SCMU …`;
  `2 R. SC.. 160000 160000 160000 <oid> <oid> R100 libs/new<NUL>libs/old`.
- **Gates:** fmt ok, clippy clean, nextest 4288 passed / 3 skipped, doc tests ok (workspace-wide, own target dir).
- Scout used: `scouts/13.md` items 1–2.
