# preview-shows-your-screen: scrolled down, you confirm a picture of the screen above yours — group 08 fix round

Status: done
Blocked by: 16 (same crate — `graft`; never two cargo commands in one worktree)
Build with: **Opus.** F1 is the milestone promise failing silently, and the
reason it survived is a whole test suite pinned at `top == 0`.
Crates: graft (add `graft-ui` only if F3 forces it — say so if it does)
Scout: **not needed.** `docs/reviews/m5-08-app-preview-and-keys.md` (`1afd7fc`)
names every finding with file and line. Read it, not the tree.

## F1 — Critical (escalated by the conductor from the review's Major)

**The preview draws the wrong rows.** The slice is
`[top − graph_rows, top + 2·graph_rows)` — correct as a *fetch* window, three
screens centred on the viewport. The picture is then drawn from the slice's
**first** row instead of from `top`. Scrolled one screen or more, the box
describes your rebase while the rows above it show the screen above yours; the
cursor and the ghost commits are off-screen entirely.

**Why I escalated it.** M5's promise to the user is *"you see the result before
you confirm."* A preview of a different part of the graph than the one the user
is looking at does not narrow that promise — it inverts it. They confirm
against a picture that is not theirs.

**Why nothing caught it: every test and every capture has `top == 0`**, where
the slice start saturates to 0 and the bug is invisible. `view/rows.rs:40`
states the intent exactly right — *"the picture the box describes and the rows
on screen are the same picture"* — and the code does not do it.

**Do this first, before any fix:** write the failing test at `top > 0`. Watch
it fail. Then fix it. A capture at `top == 0` proves nothing here, and the
suite currently has 33 ticks that agree with each other about a state the bug
cannot appear in.

## F2 — Major: `Enter` on a non-next `Undo` row

Plans a revert that **does not land where its title says**. The time machine's
whole value is that the row you pick is the state you get. Fix at the cause,
and say in the report which it was.

## F3 — Major: a label that lies about its own shape

`Fix: revert the merge (-m N)` still wears the trailing key-hint shape that
`be719c6` made mean one thing everywhere. Worse, **the invariant test never
sees verb labels at all**, so the rule it exists to enforce is unenforced on
exactly the labels that break it. Another test that cannot go red. Fix both:
the label, and the test's reach.

## F4 — the review's remaining Majors and 11 Minors

Read `docs/reviews/m5-08-app-preview-and-keys.md` and work them. One tick was
held **by inspection only** — `u` over a preview is not tested through
`key_event`. Close that by testing it, or mark it unverified and say so.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] At `top > 0` the drawn rows are the viewport's, and the cursor and the
      ghosts are on screen — asserted on the **drawn frame** — test:
      `view::tests::a_preview_scrolled_into_history_draws_the_screen_the_user_is_on`
      (drawn frame, unit) and `it::preview::a_preview_scrolled_into_history_draws_the_rows_the_user_was_on`
      (the real app, `j` to a row then `r`).
- [x] The same at a `top` near the end of history, where the window clamps —
      test: `view::tests::a_preview_near_the_end_of_history_still_starts_at_the_users_top_row`.
- [x] `Enter` on a non-next `Undo` row lands where the row's title says —
      test: `update::journal::tests::enter_on_an_undo_row_refuses_rather_than_reverting_somewhere_else`
      (and `…::enter_on_the_next_redo_row_is_still_capital_u`). Fixed by
      **refusing in words**, not by planning something else — see NOTES.
- [x] The key-hint invariant test sees verb labels, and `Fix: revert the merge`
      no longer breaks it — test:
      `update::context_menu::verbs::tests::a_trailing_parenthesis_on_a_verb_label_is_always_a_live_key`
      over `every_label_the_app_can_build()` (six fixture shapes, >15 labels).
- [x] `u` over an open preview, through `key_event` rather than by inspection —
      test: `update::preview::tests::u_over_an_open_preview_does_nothing_through_the_real_key_path`.

## Mutation table (each applied, watched red, restored)

| # | Mutation | Test that went red |
|---|---|---|
| A | `view_start = 0` (the bug itself) | both F1 tests |
| B | highlight by `preview.cursor` | `a_preview_highlights_the_commit_the_cursor_is_on_not_the_choice_index` |
| C | remove the F2 refusal | `enter_on_an_undo_row_refuses_rather_than_reverting_somewhere_else` |
| D | restore the `(-m N)` label | `a_trailing_parenthesis_on_a_verb_label_is_always_a_live_key` |
| E | let `u` past the preview in `boxes_key` | `u_over_an_open_preview_does_nothing_through_the_real_key_path` |
| F | ULID on the busy line | `the_time_machines_busy_line_names_the_entry_not_its_id` |
| G | unconditional undo line | `a_plan_with_nothing_in_it_offers_close_only` |
| H | hint context keyed on rows | `the_footer_is_the_previews_from_the_moment_the_box_opens` |
| I | `window_start` never slides | `a_preview_keeps_the_cursors_commit_on_screen_when_the_box_takes_rows` |
| J | `is_ref` ignores the kind | `moving_a_branch_leaves_a_tag_of_the_same_name_where_it_is` |
| K | drop `forget_stale_divergence` | `the_remembered_upstream_dies_with_the_line_that_explained_it` |
| L | drop Major 4's refusal arm | `a_commit_that_is_there_but_will_not_read_refuses_the_whole_picture` |

## Capture

- `docs/shots/m5-ui-preview-scrolled-80x24.txt` (4049 B) and
  `…-160x48.txt` (8267 B) — the preview at `top > 0`, the state that was
  never pictured. Written by `it::preview::capture_scrolled_preview_for_docs_shots`.
- `docs/shots/m5-ui-preview-time-machine-{80x24,160x48}.txt` regenerated; the
  header no longer asks the user to look for a `git …` line that is not there
  (Minor 6).
- **Colour unverified** (rule 5). **The visual result is not ticked** (rule 6).

## NOTES

**F1, watched failing first.** The test at `top > 0` was written and run
before any production change. It drew ids 92→76 (slice rows 28–44) while the
viewport's top row was id 64 and the ghost was nowhere on the frame — the bug
exactly as the review describes it. Only then was `view_start` added.
**Cause fix.** The picture is now drawn from the user's `top`, mapped into the
slice **by commit id** (ghosts are woven in, so an index is not a position).

**A second real defect, found by the app-level test.** The box docks over the
bottom of the graph, so the picture is shorter than the screen those rows were
scrolled on; at `top=14, cursor=30` the cursor's own commit was still off the
bottom. Fixed at the cause in `view::preview::window_start` — slide down by
exactly as much as it takes, never above `view_start`. Mutation I.

**F2 is a refusal, and that is the cause fix available here.** The review's
option (b) — resolve the `Undo` target to its root operation's position —
yields the *same* wrong plan, because `plan_revert_to` can only invert
operations in force and cannot redo. For `[A, B, undo(B), C]` the only plan
reachable is `inverse(C)`, which leaves `B` undone while the row's title says
otherwise. So `Enter` on an `Undo`/`Redo` row now says what it cannot do
instead of doing something else. Landing there properly needs a redo step in
`graft_ops::undo::plan_revert_to` — `graft-ops`, outside this fence.

**F3: a test was holding the bug in place.** Two assertions in
`verbs/tests.rs` asserted `Fix: revert the merge (-m 2)` — the wrong label.
They were changed to `, -m 2`, because the suite was agreeing with the code
rather than with the `be719c6` ruling. `crates/graft/MAP.md`'s claim that the
rule is whole is true again. **`graft-ui` was not needed**: it cannot depend
on `graft`, so the invariant's reach over verb labels lives in `graft`'s own
`verbs/tests.rs`.

**Deliberately not done.**
- **Minor 8** (cache the preview row count on `PreviewState`): unmeasured by
  the review's own words, and caching would need interior mutability on a
  `Clone + Debug + PartialEq + Eq` struct. CLAUDE.md forbids measurement in a
  helper's context, so this is a change with no evidence behind it. Left open.
- **Minor 5** (`plan_revert_to` prints `(1 steps)`): `graft-ops`, outside the
  crate set. Not touched.

**Compaction.** This ticket compacted once, during F4. Nothing of the ticket
remains: F1–F3 and every Major and Minor above are either done or listed as
deliberately not done, and the gates were re-run after the last change.

**Gates**, workspace-wide: baseline on `main` checked at the start — 2171
passed / 34 skipped. After: `cargo fmt --all --check` clean,
`cargo clippy --all-targets -- -D warnings` clean, `cargo nextest run
--workspace` **2187 passed / 35 skipped**, `cargo test --doc` clean.
