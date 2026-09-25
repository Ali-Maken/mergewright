# 80 — review fix for 77, 79 (Major M1, M2; Minor m1, m2, m5, m6, m7, m8, m9, m10, m11)

Status: done
Blocked by: 70 (graft)
Crates: graft-ops, graft
Source: `docs/reviews/m6-final-fixes-77-79.md`. Every finding names its lines; re-check each against main after 70.
Build with: builder-opus (undo path; mostly moves and tests)
Scout: none. The review names every line.

## Decided (conductor, 2026-09-25)
- m13 (after the first of `U (2 times)` nothing says a second press is owed): **yes, it should say so**, but it needs
  `replaced` carried on the journal entry, a format change. Not here: KNOWN_GAPS row for M7.
- m3, m4, m12: the conductor's (gap rows).

## Acceptance
- [x] **M1:** `undo_replaced::replaced_names_exactly_what_the_run_took` split into one `#[test]` per scenario over a
      shared `check(scenario)` (or one base repo copied). No test in it over 10 s alone; report before/after times. — `undo_replaced::{an_unstaged_edit_inside_the_checkouts_paths, …, a_redo_over_an_edit}` (11 tests over `check(&Scenario)`); each 1.38–1.53 s alone.
- [x] **M2:** `update/undo.rs` :578-760 (the box's words and fit rule) moved to `update/undo/consequence.rs`, a pure move;
      suite count unchanged. Report non-test line counts of `update/undo.rs`, `consequence.rs`, `jobs/undo.rs` in NOTES. — the build and the moved unit tests (`update::undo::consequence::tests::*`); count 3339 before the ticket's new tests, 3347 after with 8 added, none lost.
- [x] **m1:** D5 and `replaced.rs:16-17` say the true worst case (8 spawns). Measure one drift undo on the 100k fixture
      if the heavy fixture is reachable from the fast tier's helpers, else say "not measured" and why. — doc only (77 D5, `replaced.rs` module doc); not measured, see NOTES.
- [x] **m2:** the index comparisons log their commands (pass `commands` through `index_paths_changed`), so the Command
      Log's "every command it ran" holds. Test: the clean-cost test sees the index half. — `undo_replaced::a_clean_undo_spawns_no_diff_tree` and `undo_replaced::a_forced_undo_logs_both_halves_of_its_read` (red before the logging); `restart_whole_file::start_over_brings_back_a_modify_delete_choice` adjusted.
- [x] **m5:** the time-machine landing fits 80 columns (the box's left-cut applied to the status path, or drop
      `your edit to <path>` before the key). Add an 80×24 frame of it to `fix-undo-drift-way-back`. Test names it. — `undo_over_drift::the_time_machine_names_the_presses` (red first), `update::undo::consequence::tests::the_time_machine_landing_fits_80_columns_and_keeps_its_key`, frame 4 of `undo_over_drift::capture_fix_undo_drift_way_back`.
- [x] **m6:** `fix-undo-drift-way-back` frame 1 settles (`settled_frame`, `radar_answered`) after the checkout, before
      `u`; retaken; no `sorting…`. — `undo_over_drift::capture_fix_undo_drift_way_back` asserts no `sorting` in frame 1 at both sizes; `fix-undo-drift-way-back-{120x40,80x24}.txt` retaken.
- [x] **m7:** the redo landing's sentence moves out of the `keys` slot into `what`. Test. — `undo_over_drift::redo_over_drift_names_the_backup_and_no_key` (red first), `update::undo::consequence::tests::a_redo_landing_says_the_backup_in_what_with_no_keys`.
- [x] **m8:** a real-repo test drives a whole-tree case through `y` then `U`; the wording for an index-only restore is
      decided in the test's doc comment (say "the whole index" if that is what it is). Report which. — `undo_replaced_whole::an_index_only_whole_restore_names_the_whole_index_and_u_redo_puts_it_back`; "the whole index" (new `Replaced.whole_index`); `consequence::tests::a_whole_restore_is_named_by_what_it_restores`.
- [x] **m9:** the unreachable arm in `run_revert_to` removed (or commented why it stays). — removed, comment says why; the build (exhaustive match) and `jobs::` 59/59.
- [x] **m10:** `jobs/operate.rs:174-178`'s refs-for-HEAD test: same hole 79 fixed? Answer with a test, fix it if so. — yes, the hole exists: `checkout_hook_fails::a_checkout_whose_post_checkout_hook_fails_has_moved_head`; fixed, `jobs::operate::tests::a_failed_checkout_that_moved_head_rereads_everything` (red first).
- [x] **m11:** 79's tick 3 reworded to name the passing test and the retake. — doc only; 79's tick 3 names `capture_fix_undo_drift_way_back` and this retake.
- [x] **From 70 (conductor):** retake `docs/shots/m6/m6-18-launch-shell-begun-*` (stale: shows `c continue` on a merge
      since 70's `Banner { merge }`). Then make the design text match what is drawn now, reading the capture, not
      guessing: `docs/design-notes/m6-conflicts.md:258,373,375` and `TESTING.md:2415`. Name the test that writes it. — written by `.scratch/m6-conflicts/pty-walk-m6.py <size> b-launch-to-first-conflict` (`GRAFT_UPDATE_SHOTS=1`), scenario `m6-18-launch-shell-begun` in `crates/graft/tests/it/m5_walk/conflict_journeys.rs`; all 5 sizes retaken, a plain re-run at 80x24 and 187x50 matched. Only change: `c continue` → `c finish`.
- [x] Contact sheets rebuilt for changed captures, each part ≤ 200 000 B. No machine paths. — `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`: 4 parts, 179 772 / 177 281 / 163 449 / 102 490 B; grep of the diff for home paths: none. m3-09 unchanged (its `check_shot` passes in nextest).

## Touches (check each, report how)
- Callers of moved items: the build. MAP.md graft and graft-ops.
- Captures: `fix-undo-drift-way-back-*`, `m3-09-*`, m3/m6 sheets and captions.
- TESTING.md rows for walk 13 and :1344.
- KNOWN_GAPS and `docs/design/` are the conductor's.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## NOTES

- **Context compacted once mid-ticket** (after all code; before the gates, ticks and NOTES). Nothing remains.
- Scout: none (ticket says the review names every line).
- M1 times (load ~10): each split test 1.38–1.53 s; all 12 in `undo_replaced` 4.49 s parallel, 13.8 s at `-j 1`. Before (review): 15.55 s in the suite, 12.54 s alone, one test.
- M2 non-test lines (before `#[cfg(test)]`): `update/undo.rs` 854 → 653; `update/undo/consequence.rs` 287 (413 with tests); `jobs/undo.rs` 592 → 590.
- m1 not measured: the heavy fixture is 100k commits, not 100k files, and the read's cost scales with index size (`ls-tree -r`); it sits behind `heavy-tests`, and CLAUDE.md keeps measurement out of a builder's context. Worst case now documented: 8 spawns (2 `diff-tree` worktree + 3 index comparisons of one `diff-tree` or two `ls-tree`), all logged.
- m5: fitted at update time to 79 cells (`consequence::landing`): path left-cut, then `it`, then `what` cut with `…`; the key always shows. The graph footer (graft-ui) still cuts from the end; not touched (outside crate set).
- m10 fix: `refresh_after_failure`'s `Git` arm answers `WorkingTree` only when no refs moved and `head_after == head_before`; an unread HEAD counts as moved.
- Question, not a defect (no line checked): `docs/design-notes/m6-conflicts.md:374` ("after Esc … c commit") may be stale; out of scope.
- Colour unverified (rule 5).
