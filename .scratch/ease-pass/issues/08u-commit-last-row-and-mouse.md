# 08u — commit fixes, part B2: the landing's last row, mouse, narrow bars

Status: done (2026-10-18, builder; look unjudged, colour unverified). Was: ready (split by the conductor from 08s, 2026-10-18; no new decision. Amended by the planner 2026-10-18 after 08t's NOTES: rulings below, one acceptance box added)
Blocked by: 08t (same files: `view/frame`, captures), 06b-lanes-on-screen-and-header (holds graft-ui `graph_screen/layout.rs` and `mod.rs`, which M4 may need and M6 lowers)
Build with: Opus (`builder-opus-xhigh`), as 08s.

## Scope
08s's Part B, second half, in the handoff order:
1. **M4** — graft-ui `hints/status_row.rs` (`RowBar::is_landing`, `StatusRow`, `status_row`); graft `view/frame` gives the graph area's last row to the landing bar and the done line.
2. **M5 + Q2** — new `update/commit_editor/mouse.rs`; graft-ui `commit_layout`, `file_at`. `update/mouse.rs` stays ≤400.
3. **m4, m12**, then **M6**'s last two rows: split `graph_screen/layout.rs` (653) and `graph_screen/mod.rs` (578) below 400 — split, don't raise (user, 2026-10-18).
4. The remaining captures in 08s's captures box.

Everything else is 08s's spec; read it first, plus its Step 1 answers.

## Rulings on 08t's NOTES (planner, 2026-10-18) — what is and isn't 08u's
Decision lines: `docs/design-notes/ease-pass.md`, paragraph "**Commit secrets and done-line follow-ups settled**".
- **a (key files leak) and d (stale comment):** not 08u's files → `08v-mask-key-files.md`. 08u does not edit `mask.rs` or `mask_tests.rs`.
- **Hunk headings masked (08t deviation):** accepted as built. Nothing to build.
- **b (resolver `done ✓ · ▸` lines):** stay as they are. Nothing to build.
- **c (m3 pty waiter) and the stale pty records / contact sheets:** go to **14 (acceptance)**, which re-walks after every product ticket has landed. 08u re-shoots only the in-process captures its own boxes name (08s's captures box); it does **not** run pty walks or regenerate the m3/m4/m6/m7/m8 contact sheets — its M4 would move those landings again.
- **Out (08u):** `tests/it/m3_walk.rs`, `tests/it/m5_walk/*`, pty records under `docs/shots/m3`, `m4`, `m6/m6-*`, `m6/fix-band-*`, contact sheets.

## Acceptance
Tick in **08s's file**, each naming its test: **M4**, **M5 + Q2**, **m4**, **m12**, **M6**, and the rest of the captures box.
Plus, ticked here:
- [x] **Secrets stay masked through 08u's re-shoots:** after the last capture, `grep -rn 's3cr3t\|SECRET_VALUE' docs/shots` → empty, and `commit-secrets-120.txt` still reads `+ TOKEN=••••••••` (re-shot or not — say which). — `commit_screen::shots::capture_the_commit_screen` (its no-fixture-value assert) passes; grep in NOTES. — **ticked:** `commit_screen::shots::capture_the_commit_screen` passes; after the last re-shoot `grep -rn 's3cr3t\|SECRET_VALUE' docs/shots | wc -l` → 0; `commit-secrets-120.txt` **re-shot** (its header gained the repo words; that is its only changed line) and still reads `+ TOKEN=••••••••` (`grep -c` → 1).

## NOTES (builder, 2026-10-18)
**Compacted twice** during this ticket; everything in scope is done (nothing remains to split). No new scout: 08s Step 1 stands in (as told). The boxes are ticked in `08s-commit-fixes.md`; the detail is under its "NOTES (builder, part B2 = ticket 08u)".
