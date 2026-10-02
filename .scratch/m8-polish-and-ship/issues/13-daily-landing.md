# G-daily 1 — where he lands after pull, undo, redo, a partial answer, and `q`

Status: done (stopped-rebase `q` boxes pending the user's ruling)
Build with: Opus (conductor, after scout `.scratch/m8-polish-and-ship/scouts/13-daily-landing.md`; the scout corrects two line refs)
Blocked by: none
Needs: none in code. Runs beside the scroll lane on **disjoint files** (ADR-0051): `update/operate*`, `update/undo.rs`,
the quit path — not the cursor-motion/detail-follow files `settle-work-at-rest` holds; the conductor checks both
scouts' file lists before running them together.

## Read first
- `docs/research/ux-principles.md` §3.1 (where he lands)
- `docs/design/02-keymap.md` §2.2 (`q` row, `u`/`U` rows)
- `docs/design-notes/m8-polish-and-ship.md`: Need row "Two daily rulings"; Journeys J18, J20; "Where he lands" last row
- `crates/graft/MAP.md`: `update::operate`, `update::undo` (`land_after_undo`, `undo_landing_owed`), `update::banner`
  (`continue_words`); `crates/graft-ops/MAP.md` `Operation::moves_head`, `journal::undoable`

## Rulings (his, 2026-10-02, and planner) and the rows they close
1. **Pull with nothing new: the cursor stays** (his ruling). Today it moves to HEAD (`crates/graft/src/update/
   operate.rs:959,980`; `graft-ops operation.rs:1223`). After a pull whose before equals after on every field: cursor
   where it was, status `up to date ✓`.
2. **Undo and redo land on the result.** After `u` the cursor sat on the stash row; after `U` off the redone commit;
   `land_after_undo` acts only with an operation in progress (`crates/graft/src/update/undo.rs:270-277`). Ruling: after
   any `u`/`U` that answers `Done`, land as `--continue` does: on HEAD when HEAD moved, else on the row of the ref the
   entry touched first, else stay; tag rows that came back as after a finished operation.
3. **A partial answer lands too.** `Operated::Incomplete` (`NotAsTheyWere`) neither lands nor tags recovered rows nor
   drops `error`, unlike the complete answer (`crates/graft/src/update/operate/outcome.rs:26-40` vs `:336-338`).
   Ruling: land and tag as complete; keep its own warning words on the status row.
4. **`q` while a write runs.** Keymap §2.2 "confirms if an operation is in progress", read as: a write job running
   (it would be cancelled). Box: `<op> is still running · quit stops it` `[ Cancel ] [ Quit ]`, `y` quits. A stopped
   git operation (banner showing) quits without a box: relaunch offers continue/abort (PROMPT §9). Wording of §2.2 is
   the conductor's to amend (Proposed in the note).

## Scope
In: rulings 1–4. Out: `a` reword (G-daily 2). Any landing not listed.

## Signatures
None public. `land_after_undo` may be generalised; MAP.md records the shape.

## Acceptance
- [x] J18 `pull-nothing-new` = 1: `p` on an up-to-date branch, cursor on a non-HEAD row → stays; status `up to date ✓`
      (walk test). — `tests/it/daily_landing.rs::j18_a_pull_with_nothing_new_leaves_the_cursor`
- [x] J20 `undo-see-result` = 1: `u` after a commit → cursor on the new HEAD (parent); after a stash → on HEAD; `U` →
      on the redone commit (tests, one per operation kind that moves HEAD, plus a branch delete: lands on the
      recreated branch's row). — real repo: `daily_landing::j20_u_and_capital_u_land_on_the_result` (commit u/U),
      `daily_landing::u_after_a_stash_lands_on_head`, `land_on_result::undo_of_a_merge_lands_on_head`,
      `land_on_result::capture_landing_after_undo_and_redo` (merge u/U); entry-level, per kind:
      `update::undo::lands_on::tests::{the_branch_head_is_on_moving_lands_on_head, a_different_head_lands_on_head,
      a_branch_come_back_lands_on_its_row_and_is_tagged, a_stash_undone_lands_on_head, a_branch_gone_again_leaves_the_cursor,
      nothing_moved_leaves_the_cursor, a_branch_beats_the_stash}`. Branch delete covered at entry level only (synthetic
      journal entry), not by a real-repo walk.
- [x] Incomplete answer: lands, tags, keeps its warning (test). —
      `operate::tests::landing::an_incomplete_put_back_lands_tags_and_keeps_its_warning`,
      `operate::tests::lands_on_head::an_incomplete_answer_lands_and_keeps_its_warning`
- [x] `q` during a fetch job → box; `y` quits and the job is cancelled; `q` with a stopped rebase → quits, relaunch
      offers continue (tests). — Box: `quit::tests::q_during_a_fetch_opens_the_box`; `y` quits + cancels:
      `quit::tests::y_quits_from_the_box_and_cancels_the_fetch` and `daily_landing::capture_quit_while_a_fetch_runs`
      (real app, bar shows `y quit`, `y` → quit); `[ Quit ]` (`→ ⏎`): `quit::tests::quit_from_the_box_cancels_the_fetch_and_quits`;
      Esc/Cancel keep both: `cancel_or_esc_keeps_the_app_and_the_fetch`; Ctrl+C never boxes:
      `ctrl_c_during_a_fetch_quits_without_a_box`. **Stopped rebase deviates from this box's text**: per conductor it
      follows DESIGN §2.2 literally (boxes) pending the user's ruling —
      `quit::tests::q_with_a_stopped_rebase_boxes_pending_the_ruling`. Job cancel checked as the `CancelJob` command
      emitted, not as a real process stopped by the event loop.
- [x] Captures: `docs/shots/m8-landing-undo-120x36.txt` (`land_on_result::capture_landing_after_undo_and_redo`),
      `m8-quit-while-running-80x24.txt` (`daily_landing::capture_quit_while_a_fetch_runs`). **Colour unverified**.

## NOTES (builder, 2026-10-02)
- Scout used: `.scratch/m8-polish-and-ship/scouts/13-daily-landing.md`. No compaction.
- graft-ui (unblocked by conductor, only this): `ConfirmYes::Quit` (label `Quit`, word `quit`, in `ALL`, now 10);
  graft-ui coverage `COVERAGE_SLOTS` 111 → 112, `confirm_slot` gives `Quit` slot 111 (63 is the changes pane's).
  graft `update/keys/bound.rs`: `("y", "quit")` so the hint-coverage tests know `y quit` is answered.
  Re-shot `m8-quit-while-running-80x24.txt` (bar now leads `y quit`).
- Stopped-rebase `q`: isolated in `update/quit.rs::stopped_box`; to take the planner's proposal, return `None` there
  and flip its test.
- Unstoppable write (`Stoppable::No`): `[ Quit ]` does not quit; `stop_write` puts why on the status row. Not tested
  by its own test.
- Undo landing interpretation (ruling 2, "first ref touched"): first branch/tag in ref-name order that moved and still
  exists; a stash ref moving → HEAD (the acceptance's stash case); a branch deleted again → stays. `came_back` tags
  recreated branch/tag tips `↺ put back`.
- Not changed, stated: journal time machine `reverted_to` (J → revert-to) does not land — separate answer path;
  `banner::resume::resumed_incomplete` (a `--continue`'s Incomplete) still does not land — ruling 3 named
  `Operated::Incomplete` only.
- Re-shot: `docs/shots/m6/fix-undo-drift-way-back-{120x40,80x24}.txt` (cursor now on HEAD after the undo).
  `docs/shots/m6/m6-contact-sheet-4.txt` re-assembled (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`);
  besides the undo frames it picked up three `152 rows so far` → `148 rows so far`, already stale against its
  committed source capture before this ticket.
- Gates (final tree): fmt, clippy -D warnings, doc tests pass workspace-wide. nextest, workspace, clean
  (no `GRAFT_UPDATE_SHOTS`): run 1 5141/5141, run 2 5141/5141 (8 skipped each). Earlier run (before the `y` change)
  5139/5140: `graft-git conflict::tests::first_marker_line_agrees` failed once, passed alone; did not recur in
  the two runs above. Not investigated (graft-git is another builder's); seed of the failing run not kept.
- Known edge: an `amend_into` tag-read failure after an Incomplete replaces the sticky warning with its own
  (`amend_into::tags_resolved`), same as `StagingNotKept` today.

## Review fixes (review `docs/reviews/m8-13-landing.md`, 2026-10-02)
- [x] **Major 1** — `q` with the banner focused reaches `quit::asked` (`update/banner/focus_keys.rs`, arm before the
      catch-all; stale doc replaced). Cause fix. Tests, each starting banner-focused after the way focus lands there,
      each seen **failing** with the arm disabled (4/4 FAIL) and passing with it:
      conflict stop — `update::quit::tests::q_on_the_banner_after_a_conflict_stop_reaches_the_quit_box`;
      relaunch — `update::quit::tests::q_on_the_banner_after_a_relaunch_reaches_the_quit_box`;
      undo puts a merge back — `update::undo::tests::q_on_the_banner_after_an_undo_puts_a_merge_back_reaches_the_quit_box`;
      a write running under the focused banner — `update::quit::tests::q_on_the_banner_while_a_write_runs_names_the_write`.
      Each asserts the box line; the first two also that `y` quits.
- [ ] **Major 2** — **not done: a question, not a build decision.** Neither keymap §2.2 (`q` "confirms if an
      operation is in progress"), the design note's "Where he lands" row (`q` while a write runs → `quit stops it`,
      `y` quits) nor ruling 4 above says what `q` does while a `Stoppable::No` write (merge, rebase, cherry-pick,
      revert, time machine, J put-back) runs. Options: (a) box `<op> runs to the end · quit when it finishes`,
      `[ Cancel ]` only; (b) same words, `[ Quit ]` quits once the write ends (quit deferred); (c) other. Code
      unchanged (`update/quit.rs` `running_box`, `confirmed`).
- [x] **Major 3** — `update/undo/landing.rs` → `update/undo/lands_on.rs` (`lands_on::of`, `lands_on::came_back`);
      `consequence::landing` keeps its name, so each name means one thing. Pure rename, no behaviour change;
      its tests now `update::undo::lands_on::tests::*` (J20 tick above updated), passing. MAP.md row updated.
- Touches: `banner/tests.rs::the_focused_banner_claims_every_key_but_control_c` still passes (`q` is still
  claimed, now with a box); `daily_landing.rs:195` (`q` from the graph) unchanged; MAP.md `banner_key`,
  `update::quit`, `undo::lands_on` rows. **Not changed, for the conductor:** keymap §2.6a's banner table does not list
  `q` (the conductor asked for it; §2.2 needs it to apply); the focused banner's hint bar does not draw `q`
  (graft-ui / hints, not my files) — grep `banner_list`.
- Gates (workspace-wide): fmt --check, clippy --all-targets -D warnings, nextest 5209/5209 (9 skipped), test --doc pass.
- Scout: none — three small, named sites; the review gave file:line for each, and I read them directly.

## Touches
- `Operation::moves_head` callers; `undo_landing_owed`; status line priority; confirm box (`y` per M6 rule); the
  journal browser's own undo path (`J` → revert-to) — same landing? check and state; captures after `u` (re-shoot);
  m5/m6/m7 walks asserting cursor after pull or undo (grep `LandedOnHead`); MAP.md graft.

## Scout questions
- Every place the cursor is set after an operation answer (file:line); which read the journal entry's touched refs?
- Is a running write distinguishable from a fetch/read job in `AppState` today?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens.
