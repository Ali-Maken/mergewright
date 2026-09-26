# 22c lost-work-order — the lost-work fixture reordered for F1 as amended

Status: ready
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
- [ ] The fixture's order and time tests (`src/lost_work/tests.rs`) assert the new order, red first against the old
  order. Name them.
- [ ] `the_lost_work_fixture_draws_every_row_in_place` asserts the new row order.
- [ ] `the_lost_rows_are_the_fixtures_commits_and_his_edit` stays green, unedited.
- [ ] The lost edit still shows: name the test that proves the file on disk differs from the backup's copy.
- [ ] Gates workspace-wide.

## Touches
Every user of `graft_fixtures::lost_work` (grep: graft-ops tests, and any graft test or walk). 24 and 26 walk it.
Name each and how it was checked.
