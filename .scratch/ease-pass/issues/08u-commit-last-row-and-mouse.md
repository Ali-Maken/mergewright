# 08u — commit fixes, part B2: the landing's last row, mouse, narrow bars

Status: ready (split by the conductor from 08s, 2026-10-18; no new decision)
Blocked by: 08t (same files: `view/frame`, captures), 06b-lanes-on-screen-and-header (holds graft-ui `graph_screen/layout.rs` and `mod.rs`, which M4 may need and M6 lowers)
Build with: Opus (`builder-opus-xhigh`), as 08s.

## Scope
08s's Part B, second half, in the handoff order:
1. **M4** — graft-ui `hints/status_row.rs` (`RowBar::is_landing`, `StatusRow`, `status_row`); graft `view/frame` gives the graph area's last row to the landing bar and the done line.
2. **M5 + Q2** — new `update/commit_editor/mouse.rs`; graft-ui `commit_layout`, `file_at`. `update/mouse.rs` stays ≤400.
3. **m4, m12**, then **M6**'s last two rows: split `graph_screen/layout.rs` (653) and `graph_screen/mod.rs` (578) below 400 — split, don't raise (user, 2026-10-18).
4. The remaining captures in 08s's captures box.

Everything else is 08s's spec; read it first, plus its Step 1 answers.

## Acceptance
Tick in **08s's file**, each naming its test: **M4**, **M5 + Q2**, **m4**, **m12**, **M6**, and the rest of the captures box.
