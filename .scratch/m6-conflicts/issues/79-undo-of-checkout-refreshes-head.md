# 79 — an undo that moves HEAD refreshes the graph's HEAD

Status: done
Blocked by: 77 (merged)
Crates: graft-ops only (`restore.rs`, its tests, `tests/it`). Runs beside 78 (graft only).
Source: ticket 77 NOTES "Pre-existing bug found": `graft-ops/src/restore.rs:275-276` picks `Refresh::WorkingTree` when
`pending.entry.touches.refs.is_empty()`; a checkout's undo moves only HEAD, so the graph keeps the old HEAD badge.
Seen: `docs/shots/m6/fix-undo-drift-way-back-*.txt` frame 2 shows `[◆ fix]` after landing on `main` (marked KNOWN WRONG).
Build with: builder-opus (undo path; one scope rule, checkable by a table test)
Scout: none. 77's builder named the line; re-check it on main.

## Acceptance
- [x] The refresh scope after an undo/redo/time-machine step covers HEAD whenever the step moves HEAD (checkout, detach,
      any entry whose before/after HEAD differ), not only when it touches refs. Test in graft-ops, red first: undo of a
      checkout returns a scope that re-reads refs/HEAD. Name it.
      **Tests:** `graft-ops::it history_ops::an_undo_or_redo_that_moves_head_rereads_the_graph` (red first: failed
      `undo moved HEAD` left `WorkingTree`; branch and detach, undo and redo), the table
      `restore::tests::refresh_covers_head_whenever_the_restore_moves_it`, and the negative
      `history_ops::an_undo_that_leaves_head_alone_rereads_the_working_tree_only`.
- [x] Every other `Refresh` choice in `restore.rs` checked for the same hole (list each and the verdict in NOTES). **Verified by** `grep -rn Refresh:: crates/graft-ops/src`; verdicts in NOTES.
- [x] Graft-side capture: do not retake here (graft is 78's). Say in NOTES which capture frame should change, so the
      next graft builder (70) retakes it. **Verified by** `graft::it undo_over_drift::capture_fix_undo_drift_way_back`,
      which now fails; frames in NOTES.

## Touches
- Callers of the scope (`graft` `refresh_scope`): read, not edited.
- Journal format: unchanged.
- `docs/shots/m6/fix-undo-drift-way-back-*`: retaken by the next graft builder (70) after 78 and 79 land.

## Rules
Worktree only. No machine paths. Gates workspace-wide after the last edit. Don't edit KNOWN_GAPS or `docs/design/`.

## NOTES
- Line re-checked on this base (289ccf81): `restore.rs:275-276` read `if pending.entry.touches.refs.is_empty() { Refresh::WorkingTree }`. Confirmed.
- Fix (cause): `restore::refresh_after` — `Everything` when a ref moved, the plan has a `Step::SetHead`, the entry's
  `head_after` differs from `head_before`, or `head_after` was unread (not known unmoved). Covers undo, redo and the
  time machine: all run through `execute_over_drift`. Journal format unchanged.
- Refresh audit, graft-ops:
  - `restore.rs` `execute_over_drift` — the hole; fixed. `AfterState` carries the same `applied.refresh`.
  - `restore.rs` cancel variants (`Cancelled`, `CancelledPartway`) carry no `Refresh`; graft picks `Everything` for them
    (`graft/src/update/undo.rs:459`, `:497`, read only). No hole.
  - `apply.rs:338` `succeeded` — `WorkingTree` only when no refs, not remote, not history; checkout/detach are history
    (`Operation::is_history`). No hole.
  - `apply/resume.rs:375` — always `Everything`. No hole.
  - `apply/fold.rs:161` — `WorkingTree` for a step folded into a live operation (mark resolved, take, restart:
    `add`/`rm`/`checkout -- <path>`), none of which moves `HEAD`. No hole.
- Capture owed to 70: `docs/shots/m6/fix-undo-drift-way-back-120x40.txt` now fails its test. Regenerated locally (not
  committed, file restored): frame 2 (`y`, the landing) shows `[◆ main]` and plain `[fix]` — the KNOWN WRONG line
  fixed; frame 3 (`U`) keeps `[◆ fix]`. Both frames also gain `date order, sorting…` in the header and lose the Radar's
  `ⓘ` on `fix` — the graph reload is still in flight when the frame is taken. Whether the capture should wait for the
  reload to settle is a graft question for 70, not judged here.
- Gates: fmt, clippy `-D warnings`, doc tests pass workspace-wide; nextest 3302/3303, the one fail being that capture.
- No scout: 77's builder named the line; re-checked above.
