# Scout 26: promise-walk helpers (paths under crates/graft/tests/it/)

## A. Rewrite: squash, follower follows, `u`, edit stop
- Fixture: `mock_repo()` m7_walk/rewrite_editor.rs:42. It is **inline, not in graft-fixtures**: main m1 ← feature-a (OAuth, fixup!, tokens, wip: tests, wip) ← feature-b (b1). HEAD feature-a.
- Helpers (all private `fn`): `place_on` :106, `capital_r` :125, `settled` :141, `replanned` :160, `y` :175 (settle_write), `undo` :181, `message_shown` :187, `every_ref` :85, `subjects` :76, `read` :72; `open`/`frame`/`within` imported.
- Sketch, `squash_last_three` :203:
  `place_on(HEAD)` → `capital_r` → `replanned(press('s'))` ×2 → `shown = message_shown(frame)` → `y` → `subjects("main..feature-a")` → `git log -1 --format=%B feature-a` == shown → `read("feature-b^") == read("feature-a")` → `undo` → `every_ref == before` :245.
- **Contradiction:** there is no tree assertion and no "message leads with the kept row" assertion. The message is only compared to what the editor drew (:227).
- Edit stop: `stopped_on_a1()` rewrite_stop.rs:125. It uses a different repo (`line_repo`, a1 a2 ← b1 ← c1). Git runs it through `graft_rewrite_stops` :33 and the app is `relaunched` :85. It is **not driven by `R` in the editor**.
- `space a c`: `edit_stop_amend_continue` m7_walk/rewrite_stops.rs:302. Steps: `focus_gained` + pump working_tree :318 → `' '` + settle_write :328 → pump `status_read.is_none()` → `'a'` pump operating → settle_write :340 → pump `status_read` again → `'c'` pump `operating.is_none() && in_progress.is_none()` :349.
- The end is asserted by `!rebase_merge`. No assertion of "on the graph" or "rewrite stack gone". Could not confirm the field for that.

## B. Trace (Shape::Session)
- Fixture: `session()` m7_walk/blame.rs:36. `FILE`/`LINE` are consts, imported by history.rs:15.
- "Diff pane on line 22": `on_line_22` :79 → `open_on_line_22` :86. It opens main (`⏎`), `file_row` :150, `⏎`, `]`, then `j` until `diff_cursor().at == New/LINE`.
- Readers:
  - `blame`/`blame_ready` :171/:161
  - `cursor_line` :185 (`.final_line`, `.text`), `cursor_author` :194
  - trailer from frame: `drawn` :66 filtered by `TRAILER` + `CURSOR_ROW` :390-395
  - `graph_cursor` :229, `landed` :216, `oid` :209, `Route` :240
- `^`: `blame_line_before` :411: `B`, blame_ready, `^`, blame_ready → `trail.places.len()==2`, final_line 20, author Tom, title `before <sha7>`.
- `L`: `function_history` history.rs:53. Its route is `B L`, not `B ^ Esc L`. Readers: `traced` :29, `history(state)` :20, `place.selected_row().id == lina` :76, `place.range_at()` :84.
- `⏎` from history + one `Esc`: history_place.rs:111-135 (`history(&state).selected` kept).
- **No existing test** covers `^` then `Esc` (pop), a `+` mark on line 19 in the file pane, or `Esc Esc Esc` back to the diff pane on the same line. Could not confirm the expected state for any of them.

## C. Recover (LostWork)
- Fixture: `lost_work()` m7_walk/journal.rs:94 (graft-fixtures/src/lost_work.rs:146). Consts `LOST_ON_MAIN`/`LOST_ON_SPIKE`/`EDITED`/`LOST_EDIT` :59/`DISCARDED_EDIT` :57.
- Helpers:
  - `opened` :109 (fixed clock)
  - `open_j` :119, `enter_to_the_box` :129, `confirm` :152
  - `selected_row` :173, `rev` :180, `drawn`/`text` :99/:104
- Journeys: `journey_13/14/15` :265/:302/:372. `n` at :322/:377.
- Journey 13 needs git's clock pinned: it runs in a child process (`the_three_lost_work_journeys` :200, reason :186-198).
- **Contradiction:** the existing chain is 13 → 14 → 15 with **no `u` between**. Journey 14's precondition is "D is on main again, D's ✗ row gone, nothing preselected" (:303). After `u`, main is off D again. Could not confirm what J preselects then, or what `n` lands on.
- Bytes after `u`: journey_15 :431-438 (`std::fs::read == DISCARDED_EDIT`), no autocrlf.
- The autocrlf byte-exact tests are **graft-ops only**, in graft-ops/tests/it/undo_bytes.rs:
  - `lf_edit_under_autocrlf` :31
  - `u_after_a_discard_brings_back_an_lf_file_under_autocrlf` :42
  - `…_after_a_stash_…` :57
  - `u_brings_back_an_lf_executable_under_autocrlf` :97
- No app-level autocrlf test exists (grep `autocrlf` in graft/tests: only repo_truth.rs:100, which sets it `false`).

## D. Ignored-file guard at 80 cols
- **No test in crates/graft/tests/it** exercises the ignored-file refusal. Grep `is in the way` finds none. Only `are in the way` (unsaved edits) at m6_stash_put_back/stash_preview.rs:216 and m3_walk.rs:1468.
- The guard's tests are graft-ops only: graft-ops/tests/it/in_the_way/{one_shot,stash,pull,replay}.rs (ticket 28c:20-26). Words: `c.txt (ignored) is in the way · move it aside first`.
- Terminal size: `app.resize(&state, 80, 24)` stash_preview.rs:184, then `drawn(&state, 80, 24)` :193.
- Status row: `from_bottom(frame, STATUS)` journal_screen.rs:26/:62 reads J's status row. Otherwise it is `state.status` matched as `Status::Sticky`/`Done` (rewrite_stop.rs:219; journal.rs:137).
- No fixture with a long branch name or a deep path found. Could not confirm one exists.
