# 22c lost-work-order — the lost-work fixture reordered for F1 as amended

Status: done
Blocked by: 22b (merged e6d620e8); 23b-review-fixes (holds graft-ops). Must land before 24's walks.
Crates: graft-fixtures (`lost_work`, `src/lost_work/tests.rs`), graft-ops **tests only** (`tests/it/lost_work.rs`).
Build with: Sonnet. The planner fixed the order; this moves two steps and their times, and updates the order asserts.
Scout: none. 22b's NOTES ("Order follows the mock-up") say how the fixture is built.
Split by: conductor, 2026-09-26, from the planner's ruling in `24-journal-screen.md` "Ruling (planner, 2026-09-26)"
(the paragraph "The fixture must change first"). No new decision.

## Scope
Exactly that paragraph:
- Move the Graft discard and its undo (the lost edit to `src/a.rs`) **before the shell's spike steps** (after A–D,
  before `switch -c spike`).
- The fetch stays at 10:44, the newest row.
- Newest first, the list reads: `● fetch` · `$ reset └ ✗ D` · `$ switch spike→main └ ✗ E` · … · `● undo └ ✗ your
  edit` · `● discard`.
- The shell steps after the move must leave the lost edit showing, and must not commit `src/a.rs` into E.

## Acceptance
- [x] The fixture's order and time tests (`src/lost_work/tests.rs`) assert the new order, red first against the old
  order: `graft_discards_and_undoes_before_the_spike_and_fetches_last` (renamed from `graft_is_called_at_its_three_points_in_order`; red, then green), `heads_log_is_the_shells_steps_at_fixed_times`.
- [x] `the_lost_work_fixture_draws_every_row_in_place` asserts the new row order.
- [x] `the_lost_rows_are_the_fixtures_commits_and_his_edit` stays green, unedited (diff: no change to it).
- [x] The lost edit still shows: `the_spike_steps_leave_the_undone_edit_on_disk_and_out_of_e` (disk = `DISCARDED_EDIT` != `LOST_EDIT`; E and S carry only their own files); graft-ops `the_lost_rows_are_the_fixtures_commits_and_his_edit` (backup blob = `LOST_EDIT`, disk = `DISCARDED_EDIT`).
- [x] Gates workspace-wide: fmt, clippy, nextest 3800/3800, test --doc.

## Touches
Every user of `graft_fixtures::lost_work` (grep: graft-ops tests, and any graft test or walk). 24 and 26 walk it.
Name each and how it was checked.

## NOTES (builder, 22c)
- Discard 10:16:00, undo 10:16:40 (between D 10:15 and `switch -c spike` 10:18). Fetch stays 10:44.
- To keep the undone file on disk through the later shell steps: commits `add -- <file>` (not `-A`); reset is `--keep`
  (same reflog text; `src/a.rs` equal in C and D); stash is `push -- src/metrics.rs`. The pathspec stash writes **no HEAD
  reflog record** at 10:42 (the list never drew it); heads-log test updated accordingly.
- New row order: Fetch, reset, ✗ D under 1, switch, ✗ E under 3, commit E, switch, Undo, ✗ edit under 7, Discard, commits.
- Touches: only users of `lost_work` are graft-fixtures' own tests and graft-ops `tests/it/lost_work.rs` (grep); MAP.md updated.
- No scout (ticket: none). No compaction.
