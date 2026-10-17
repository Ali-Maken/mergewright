# fold-integration-tests — the integration tier agrees with fold by default; boxes 5, 7, 8

Status: ready
Blocked by: 06d
Build with: Opus (`builder-opus-xhigh`) — several timeouts are undiagnosed, and each needs a cause with a line (rule 7) before any change.
Needs: from 06d: `Row::is_merge`, `Row::drawn_parents`, `View::folded(head)`. From 06: `space` handler, `Window.fold`, `FoldMark`.

## Read first
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Handoff 2 items 3, 4, 6; **Decisions for continuation 2, D3**; Acceptance boxes 5, 7, 8.
- 06d's NOTES: which tests still fail after the parents and HEAD fixes.
- `docs/design-notes/ease-pass.md` Journeys (the counts the m8 journeys assert).
- `crates/graft/MAP.md`: `history`, `update::keys`, `view::rows`, `event_loop::*`.

## Scope
In: graft integration tests (`crates/graft/tests/**`) that **write no capture**, plus their fixtures. The fold-related fast-tier budget tests. The tour's repository or steps, if the tour is the cause.
- Every test in Handoff 2.3 that is not a `capture_*` test, and not in `lane_cap`:
  - `m8_walk::journeys::{find_key_by_name, run_unrecalled, what_does_key, light_theme}`;
  - `m8_walk::teaches::*`;
  - `look_chrome::every_drawn_row_is_the_row_a_click_selects_at_every_designed_size` — run it both folded and with one merge opened, since click mapping across hidden rows is the risk;
  - `context_menu::the_menus_band_stops_at_the_detail_pane_instead_of_clearing_it`.
  Fix each by D3's route order. **Journey step counts stay as they are.**
- The undiagnosed tests: `m7_walk::pr_contents::pr_contents`, the `tour*` tests, `preview::the_branch_badge_still_lands_on_the_newest_predicted_row`, `radar_not_checked`. NOTES gives each one's cause, file and line first. Fold hid a row → D3. A ghost row folded, or the tour asking for a hidden row → a product fix, per D3. Anything else → stop and report.
- `m8_walk::journeys::fold_merge_count` rewritten: the tail reads `▸ N commits from …`; `space` → `▾` and the side's rows appear. Its step count comes from the design note's journey.
- **Box 5:** a real fixture repo (`main ← wallet-release ← sara/fix-toc`, PR merges with the branch deleted, one named merge) driven through the worker. First paint shows no merged-branch commits, and its tails read as 06 Scope says.
- **Box 7:** app-side Q10 row shape. After `space`, the opened side's rows sit on a lane that is not the merge's column and that joins the merge row.
- **Box 8:** a fast-tier first-paint test and a held-`j` test over a walked repo whose window carries folding merges (`fold` non-empty). Release numbers go to the milestone review.

Out: every `capture_*` test, `lane_cap.rs`, `look_polish` and re-shooting (06f).

## Signatures
None new. Fixture helpers are the builder's to name; NOTES names them.

## Touches
Shared fixtures (`merge_on_screen`, the m8 fixture repo). An added unmerged branch changes what the capture tests draw, so NOTES lists every capture test using that fixture, for 06f. The tour's repository and text. Journey counts in `TESTING.md` (unchanged). `over_400.txt` if test files grow.

## Acceptance
- [ ] Every test above passes, or NOTES names a cause with a line and the conductor's call. Each tick names its test.
- [ ] No m8 journey's asserted step count changed (diff of the assertions, quoted).
- [ ] Box 5 and box 7 in 06 ticked, each naming its test.
- [ ] Box 8 in 06 ticked: the test shows the window really carries fold facts (an assertion on `fold`, so the tick is not vacuous).
- [ ] Workspace nextest: the only failures left are `capture_*` / `lane_cap` / `look_polish`, listed for 06f.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k. The tour diagnosis is the unknown. If it turns into a product change bigger than one step, stop and report.
