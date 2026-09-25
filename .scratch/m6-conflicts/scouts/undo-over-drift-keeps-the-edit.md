# Scout: 77 undo-over-drift-keeps-the-edit (base 5285a7b6)

## Key pin: `U` after `u y` — RAN, it works
Ran in a detached worktree (removed): 69's `drift_box_after_a_checkout`, then `y`, then `KeyCode::Char('U')`.
Observed: status `Done { "redo: checkout fix" }`; `HEAD` = `refs/heads/fix`; `fix.txt` == `"edited after the checkout\n"`
byte-exact; porcelain `1 .M ... fix.txt` (unstaged edit, as before `u`). Staged-edit case not run.
Traced why: Redo = `inverse(undo_entry)` `graft-ops/src/undo.rs:226`; `generic_steps` :278 builds `RestoreWorktree{tree:
worktree_before, paths: scope(worktree_after, tree)}` :316-321 — `worktree_before` is the Undo entry's own snapshot
taken by `begin` at `y` (`restore.rs:222-242`); `ChangedBetween` includes `fix.txt` (absent after vs B before).

## Q1 ops API
`undo`/`redo` -> `Result<Applied, UndoError>` (`undo/execute.rs:60,65`); `revert_to` -> `Vec<Applied>` :87. `Applied`
`apply.rs:69`. `drift` is computed **always**, force or not (`execute.rs:162`), then discarded when forced (:163-166).
## Q2 Drift shape
`Drift` `undo.rs:559-580`: `worktree: bool`, `index: Option<(IndexState,IndexState)>`, `unknown: Vec<&str>`. **No path
list anywhere in Drift.** `drift` :649-684 compares tree oids only. Plan paths are `PathScope::ChangedBetween{from,to}`
/`Everything` (`undo.rs:99-107`) — oids, not paths; resolving needs a tree diff read.
## Q4 time machine
Drift checked **per step at run time** (`revert_to` loop :99-122 -> `run_plan`); `force` passed to every step; drift on
step k>0 returns `PartWay{done,..}`, so steps can have run first.
## Q5 box
Lines built `graft/src/update/undo.rs` `revert_drift_box` ~:490 and `drift_box` ~:519; the undoable line at :500, :527.
graft-ui wraps (`confirm.rs:37`) but caps at `MAX_MESSAGE_LINES = 4` (`confirm.rs:31`), truncating with `…`.
Whether the new longer line + drift line fits 4 at 80×24: **could not confirm** (not rendered).
## Q6 Nothing
`Undone::Nothing(String)` is a **tuple** variant (`jobs/undo.rs:43`), built at :220 from the error only. Whether the
job still holds entries there: could not confirm; `revert_to` job reads `ctx.journal.entries()` :383.
## Q7 preview
`PreviewPlan` has `worktree: WorktreeEffect` (`graft-ops/src/preview.rs:182`); its content for revert not traced.
## Q8 assertions / captures
`itself undoable`: code `update/undo.rs:500,527`; `graft-core/src/journal.rs:28` (doc comment only). Captures:
`docs/shots/m3-09-undo-drift-box-{160x48,80x24}.txt`, `m3-undo-drift.txt`, `m3-contact-sheet.txt`,
`m6/fix-box-owns-the-bar-{80x24,120x40}.txt`, `m6/m6-contact-sheet-4.txt`. Pin asserting `nothing to undo`:
`graft/tests/it/undo_over_drift.rs:86-90`.
Keys: `u`/`U`/`y` only; no new key; keymap not edited here.

## Mismatches with spec
- Spec path `tests/it/undo_over_drift.rs` is actually `crates/graft/tests/it/undo_over_drift.rs`.
- Spec signature `Undone::Nothing { .., redo }` — code is `Nothing(String)` (`jobs/undo.rs:43`); a shape change.
- Spec calls `Replaced` "computed from reads `drift` already holds": `drift` holds no paths (Q2); a new tree-diff read is needed.
- 80×24 4-line cap may truncate the new line (Q5, unconfirmed).
- m3/m6 contact sheets also contain the words (not in the spec's capture list).

## Model
builder-opus-xhigh: new graft-ops read in the undo path, per-step time-machine `presses`, M3 promise.
