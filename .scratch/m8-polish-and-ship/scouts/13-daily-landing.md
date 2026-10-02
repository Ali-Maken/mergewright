# Scout: 13 daily-landing

## Q1. Where the cursor is set after an operation answer
- Launch records the landing: `update/operate.rs:155` `next.lands = landing_of(&op)`; `landing_of` at `operate.rs:188-212`. `Head` iff `moves_head()` (`graft-ops/src/operation.rs:1341`; `Pull` listed at `:1350`). So pull always owes `Landing::Head`, whatever the before/after.
- Spent: `operate/outcome.rs:281` `owed_landing` (takes `state.lands`).
- Paid: `operate/outcome.rs:169-190` `landed()` → `banner::finished_lands_on` (`update/banner/stop.rs:175`, `finished_lands_on_head` `:165`) or `banner::lands_on_changes`. Head landing deferred via `AppState::land_on_head` (`state.rs:335`), dropped on refs failure (`mod.rs:998`).
- `Done` path: `done_answered` `outcome.rs:327-341` = landed + `amend_into::landed` + `recover::tag_rows` (`recover.rs:88`). `StagingNotKept` `outcome.rs:289` same family.
- `Incomplete`: `outcome.rs:26-40` sets sticky line, `commit_editor::committed`, `refresh_scope` — no landing, no `tag_rows`. Confirms ruling 3.
- Undo/redo: `undo.rs:369-372` sets `undo_landing_owed = in_progress.is_none()`; paid in `mod.rs:830` → `land_after_undo` `undo.rs:270-290`, which returns at `:274` unless an operation is in progress (banner focus only, never moves cursor). Dropped on read failure `mod.rs:842`. Confirms ruling 2.
- Other landers (not ops answers): `relaunch.rs:49,191`, `clash.rs:168,178,246`, `banner/pressed.rs:123`, `resolver::land_on_clash`.
- **Touched-refs reading: none.** Every landing comes from the launched `Operation` (`landing_of`, `tag_rows` matches `Operation::Recover` only). Undo's `Running.op` is `None` (`remote.rs:211`), so undo has no op to derive from; nothing reads the journal entry's refs for landing. `undo/consequence.rs:132 landing()` builds the status line, not the cursor.

## Q2. Running write vs fetch/read
- Fetch is a write: `Operation::Fetch` (`operation.rs:96`), `is_remote` `:1716`. Any write sets `AppState::operating` (`state.rs:94`) and `remote.running: Option<Running>` (`remote.rs:172,206-221`) with `description`, `op`, `stoppable`. Reads (`remote.reading` `remote.rs:166`, refs/tree reads) are separate fields. So "write running" = `operating.is_some()`; fetch distinguishable via `running.op.is_remote()`.
- Quit today: `keys.rs:320` `graph_key` sets `Quit::Requested` unconditionally; `event_loop/running.rs:196` exits. Ctrl+C test `confirm.rs:835-845` asserts quit never confirms.

## Spec assumptions that disagree with code
- `operate.rs:959,980` — those lines are `in the way` wording (`operate.rs:950-985`), not pull landing. Real lines: `operate.rs:155,188-190`.
- `graft-ops operation.rs:1223` — no pull logic there; `moves_head` is `:1341`, Pull `:1350`.
- `LandedOnHead` exists only in `graft/MAP.md` and `tests/it/m5_walk/{steps,run}.rs`; also `tests/it/land_on_result.rs` covers landings (not grepped for that name).
- Journal browser `J` revert path landing: could not confirm (not traced).

## Files the ticket would edit
`crates/graft/src/update/operate.rs`, `update/operate/outcome.rs`, `update/undo.rs` (+ maybe `undo/consequence.rs`), `update/mod.rs` (:830 call site), `update/state.rs` (lands/undo_landing_owed fields), `update/keys.rs` (:320 quit), `update/confirm.rs` (quit box), `update/banner/stop.rs` if `finished_lands_on` generalised, `update/recover.rs` (tag_rows), possibly `graft-ops/src/operation.rs`; tests `crates/graft/tests/it/land_on_result.rs`, `m5_walk/*`; `crates/graft/MAP.md`. Overlap risk with 01: `event_loop/running.rs` only if quit-box handled there. With 03: `state.rs`, `mod.rs` shared — could not confirm 03's list.
