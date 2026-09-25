# 79 — an undo that moves HEAD refreshes the graph's HEAD

Status: ready
Blocked by: 77 (merged)
Crates: graft-ops only (`restore.rs`, its tests, `tests/it`). Runs beside 78 (graft only).
Source: ticket 77 NOTES "Pre-existing bug found": `graft-ops/src/restore.rs:275-276` picks `Refresh::WorkingTree` when
`pending.entry.touches.refs.is_empty()`; a checkout's undo moves only HEAD, so the graph keeps the old HEAD badge.
Seen: `docs/shots/m6/fix-undo-drift-way-back-*.txt` frame 2 shows `[◆ fix]` after landing on `main` (marked KNOWN WRONG).
Build with: builder-opus (undo path; one scope rule, checkable by a table test)
Scout: none. 77's builder named the line; re-check it on main.

## Acceptance
- [ ] The refresh scope after an undo/redo/time-machine step covers HEAD whenever the step moves HEAD (checkout, detach,
      any entry whose before/after HEAD differ), not only when it touches refs. Test in graft-ops, red first: undo of a
      checkout returns a scope that re-reads refs/HEAD. Name it.
- [ ] Every other `Refresh` choice in `restore.rs` checked for the same hole (list each and the verdict in NOTES).
- [ ] Graft-side capture: do not retake here (graft is 78's). Say in NOTES which capture frame should change, so the
      next graft builder (70) retakes it.

## Touches
- Callers of the scope (`graft` `refresh_scope`): read, not edited.
- Journal format: unchanged.
- `docs/shots/m6/fix-undo-drift-way-back-*`: retaken by the next graft builder (70) after 78 and 79 land.

## Rules
Worktree only. No machine paths. Gates workspace-wide after the last edit. Don't edit KNOWN_GAPS or `docs/design/`.

## NOTES
