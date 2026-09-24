# Review: m6-conflicts ticket 14 (resolver three panes)

Ticket `.scratch/m6-conflicts/issues/14-resolver-three-panes.md`: commit `2c45c54`, merged `7bef4b7`. Range `9af0460..7bef4b7`. Read-only. Reviewer: Opus, 2026-09-24.

**Count: 0 Critical · 3 Major · 7 Minor.**

Rulings already made (not re-reported): D1–D9 accepted by the conductor (`docs/log/m6.md`). The ADR-0043 and design-note edits for D1–D6 are **uncommitted** in the working tree while I reviewed. I judged against the committed text at `7bef4b7`.

## Gates

All ran one at a time on `main` at `c56b64e`. Its `crates/` are identical to `7bef4b7` (`git diff --stat 7bef4b7 HEAD` touches only `KNOWN_GAPS.md` and `docs/log/m6.md`). A scout and a researcher were working at the same time on the same machine.

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2811 passed, 1 skipped. **102.5 s** (nextest summary; build already warm). Peak RSS of the timed process tree 272 MB. |
| `cargo test --doc --workspace` | 3 passed, 2.7 s |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 2846 passed, 0 failed, 1 ignored. **322.3 s** wall. Max RSS 1.12 GB. |
| Release perf (`frames::` + `heavy::budgets_hold`, `--test-threads=1`) | **13 passed**, 73.3 s (89.9 s wall with the build) |
| Fuzz, benchmarks | Not run. No fuzz target or bench covers the three-pane layout: `conflict_parse` is graft-git's parser, which this ticket does not touch, and `row_render` benches graph rows. |

The five slowest fast-suite tests:
- `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`: 8.83 s
- `graft-ops::it conflict_ops::whole_file_shapes_resolve_like_git`: 8.04 s
- `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean`: 7.07 s
- `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys`: 6.95 s
- `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 6.82 s

`target/` is 18 GB.

Release numbers:
- Frames (input to frame, budget 16 ms):
  - Pixel 50k refs: p99 8.55 ms, max 10.1 ms.
  - Unicode 50k refs: p99 8.70 ms.
  - Diff scroll: p99 8.24 ms.
  - Detail pane open: p99 8.15 ms.
  - Map: p99 8.31 ms, max 10.3 ms.
  - Pixel 100k: p99 8.15 ms.
  - Unicode 100k: p99 8.57 ms.
- First paint:
  - 100k with 50k refs: 62.3 ms.
  - 100k with a commit-graph: 13.6 ms.
  - 100k without one: 15.7 ms.
  - Realistic 100k with a commit-graph: 75.2 ms.
  - Realistic 100k without one: 93.3 ms.
- Peak RSS: 31–111 MB.

## Ticks: spot-check

**I checked 13 of 13 ticks, and 13 held.** For each one I opened the named test and read what it asserts against what the tick says.
- `card_rows_are_unchanged`: held. The goldens `resolver-{120,80}.txt` are not in the diff, and the test renders them through `Arrangement::Card` by name.
- `three_panes_stay_level`: held. It sweeps 7 files × 5 states and checks each region's start row, its content-then-pad per pane, and that no rows are left over.
- `each_pane_is_numbered_without_gaps`: held. Yours and theirs are checked as `1..=n`. The result is checked `+1` per line, with the placeholder standing for its block's ours count, and against `render()` when nothing is open.
- `plain_regions_number_each_pane_by_its_projection`: held.
- `the_chosen_side_lands_in_the_middle`: held. It checks the cells and the drawn text.
- `the_rule_row_is_the_block_row`: held.
- `panes_split_by_the_rule`: held. It covers 147..=157 and 167..=177, and the 48 floor for columns 22..=32 at 180.
- `resolver_layout_is_pinned_three_200`: held. The golden carries a "generated, not a mockup match" header.
- `a_click_in_any_pane_picks_the_block`: held. It clicks in 3 panes and on 2 separators.
- `draw_and_click_agree_across_180`: held. At 179/180/181 the geometry's layout and the drawn frame agree on the arrangement, and a click picks block 1.
- `next_block_scrolls_three_panes_to_its_rule_row`: held.
- `resize_across_180_keeps_the_cursor_block_in_view`: held by reading. **The "mutation-checked" claim was not re-run by me.** By reading, the card at 179 puts block 1's header about 1 row lower than the panes' rule row. So the test's starting `top`, with block 1's rule row on the last row, would push it off screen without `resized`.
- `three_panes_capture`: held, with one assertion that proves nothing (Minor 1).

## Spec, ADR-0043, design note, keymap

- ADR-0043 §2 (amended): the rows level by region, the placeholder, and the chosen side in the middle all match `panes.rs:111-211`. The amended widths are 48–53 at 180 and 55–59 at 200. I recomputed them from `three_panes` (`panes.rs:91-108`) and the 22–32 clamp, and they are right. The 180 capture's panes are 50/52/50.
- ADR-0043 §5: plain rows are numbered per projection (`panes.rs:116-129`). The open placeholder draws no number, and the rule rows draw no number.
- Design note Amendment 2 has Need, Prior art (cites `docs/research/conflict-layout-audit.md`, which exists) and Journeys. This ticket adds no keys and no steps: the journeys count keys, and the three panes change none. Steps to count in a walk: none here (the walk is ticket 21's).
- Keymap §2.6: the ticket binds nothing, and `scroll_key` is untouched (S3 held: `j`/`k` are not in the diff). Mouse per §2.15: picking is by row only, and the wheel scrolls one `top`. Both are consistent.
- **D6 check:** ticket 14 never promised an advance. Its scope (items 1–7) is layout, draw, hit-test and resize. The advance is P2, owned by ticket 15 (keymap `02-keymap.md:260-262`). Only the capture's key sequence depends on it. So D6 contradicts nothing ticket 14 promised. The forward risk is Minor 5.
- Rule 3: no ignored `Result`, no `let _ =`. The silent returns sit on paths the draw must keep total: `panes/draw.rs:109-111` (a stale block), `:125-141` (a stale line), `:162-164`. Each is documented as "the draw stays total" or is unreachable. The `let … else { return }` at `panes.rs:234-236` is unreachable (the only caller passes `WholeFile`), but see Major 2.
- Sanitisation: every drawn string is either a `SanitizedText` or a literal. `absent_on` sanitises its words (`view/resolver.rs:122`).
- No `unwrap`/`expect` outside tests. `drawn_for_tests` is `#[cfg(test)]`.
- Captures vs "what right looks like" (textual only), checked on `fix-three-panes-{180,187,200}x50.txt`:
  - The titles row reads `yours · main │ result │ theirs · feature`.
  - Block 1's row: `2 two from main │ 2 TWO from feature │ 2 TWO from feature`.
  - Block 2 shows `── o take │ ── 2 of 2 · open │ ── t take` and `? open · o t b`.
  - After it, `8 eight │ 8 eight │ 9 eight`, which is 6 + 1 + 1 = 8.
  - `▸ notes.txt` is in the list column.
  - At 180 the list column is 25, then `│`, then 50/52/50 with two `│`, which is 180.
  - **All hold.** The look is the user's call (rule 6), and colour is unverified (rule 5).

## Flake verdict: `two_boxes_over_content`

**It depends on timing, by reading. Whether that is what failed once, I cannot confirm: no failure output was kept.** See Major 3.

---

## Critical

None.

## Major

### 1. Three files are over PROMPT.md 6.2's ~400 lines, and one grew without a gap row
- `crates/graft/src/view/resolver.rs` went from **671 to 727** lines, +56: `absent_on` and `drawn_for_tests`. That is about 406 of code before its `#[cfg(test)]` (:407). It is **not** in the ticket's gap rows.
- `crates/graft-ui/src/conflict_resolver/tests/three_panes.rs`: **467**. This is a new file.
- `crates/graft-ui/src/conflict_resolver/draw.rs`: 402. It went over by this ticket's `pub(super)` edits.
- Why: rule 6.2 is hard, and new files should start under the rule.
- Fix:
  - Move `drawn_for_tests` beside the `update` tests' helpers, or into a `view/resolver/for_tests.rs`.
  - Split `three_panes.rs`: its corpus and `doc_of`/`swept` fixture (:13–124) go to a `fixtures` child.
  - Add a gap row for `view/resolver.rs`.

### 2. Clever code where plain code would do (6.2)
- `panes/draw.rs:88`: `draw_cell` takes `(block, pane, width): (Option<usize>, Pane, usize)` as one tuple parameter. This packs three arguments into one, which reads like a dodge of `too_many_arguments`. Fix: a small `CellAt { block, pane, number_width }` struct, or pass `Pane` and look the width up inside.
- `panes.rs:264-289`: `pane_number_widths` indexes panes by the magic `0/1/2` (`see(0, …)`, `see(2, …)`), while `draw.rs` has a `Pane` enum for exactly this. Fix: return a struct `{ yours, result, theirs }`, or index by `Pane`.
- `panes.rs:225-236`: `whole_file_rows(view: &RegionView, …)` takes the whole enum and then re-destructures it behind a `let … else { return }` that can never fire. Fix: destructure at the call site (`panes.rs:132`) and pass the fields. Then the impossible branch does not exist.

### 3. `two_boxes_over_content` depends on timing (pre-existing, M5 code, not this ticket's diff)
- The frame's footer and graph height depend on `AppState::system_git`:
  - `commit_graph_line()` returns `None` while `system_git` is `None` (`crates/graft/src/update/state.rs:378`, the `?`).
  - `graph_rows()` spends a row on that line (`state.rs:355`).
  - The captured footer reads `no commit-graph file: history sorts on every open · B build one …` (`docs/shots/m5-ui-confirm-over-graph-160x48.txt`).
- `system_git` is set only when the `CheckSystemGit` job answers (`update/mod.rs:96` dispatches it, `:595-596` stores it).
- The capture's wait, `ready()` (`crates/graft/tests/it/two_boxes_over_content.rs:102-121`), waits for the working tree, refs, remotes, the remote read, a complete walk and no promotion. It does **not** wait for `system_git`. `App::key` and `App::resize` do not pump (`tests/it/app_harness.rs:135-141`, `:189-195`), so nothing drains a late answer before the frame is taken.
- If `git --version` answers after the walk and the remote facts, the frame has no commit-graph line and one more graph row. Then `check_shot` (`tests/it/captures.rs:171`) fails on the bytes. On a loaded machine with 2 workers (`app_harness.rs:108`) that ordering is possible.
- **Not confirmed as the cause of the one failure:** the builder kept no output. I did not try to reproduce it under load.
- Why Major: a flaky gate teaches everyone to re-run, and then green stops meaning green (rule 1).
- Fix (cause): add `&& state.system_git.is_some()` to `ready`'s predicate. Update the KNOWN_GAPS row M6/14 from "not investigated" to this line.

## Minor

### 1. The capture test's "= 180" assertion cannot fail
- `tests/it/resolver/three_panes.rs:93-102` builds `panes` from `bars` and `180`, then asserts `bars[0] + 1 + sum + 2 == 180`. Algebraically this is always 180.
- Only `bars.len() == 3` (:92) and the `≥ 48` check (:103) test anything.
- Also, `? open · o t b` is asserted anywhere in the frame (:77), not in block 2's result pane.
- Fix:
  - Measure the panes from the frame's width independently, for example by checking `bars[2] < 180 - 48`.
  - Assert the placeholder on the row under `── 2 of 2 · open`, between `bars[1]` and `bars[2]`.

### 2. "One width source" holds only while the frame's area equals `state.size`
- `draw_resolver` lays out at `split.doc.width` from `resolver_split(state.size)` (`view/frame/conflicts.rs:46-48`). But it renders into `document`, which comes from the frame's area (`:37-40`, `:59`). The widget re-splits that with `resolver_areas(area, placement)` (`graft-ui/src/conflict_resolver.rs:289`).
- They can diverge: the builder's own `one_name_per_side` fixture (`view/resolver.rs:560`) left `state.size` at 0×0 and failed.
- Nothing asserts that they agree.
- **Question (no line proves it either way):** can `terminal.draw` (`event_loop/running.rs:353`, which autoresizes from the real terminal) run between a physical resize and the `Resize` event being handled? If yes, the result is one frame laid out at the old width. The graph's `graph_rows` has the same pattern, so this is not new app-wide.
- Fix: `debug_assert_eq!(document.width, state.size.columns)` in `draw_resolver`, or render into a rect built from the split.

### 3. `resized` runs on every resize, not only across 180
- `update/mod.rs:744` calls `resolver::resized` unconditionally. `geometry.rs:82-90` re-scrolls to the cursor block.
- So a user who wheel-scrolled away from the cursor block to read gets snapped back on any resize, even one column inside the card.
- It also leaves `top` unclamped when `cursor_block` is `None` (`geometry.rs:83-85`). That is harmless today, because whole-file docs are short.
- **Filed as a question:** is the snap wanted outside the 180 crossing? The ticket's item 7 says "Resize across 180".
- Fix, if not wanted: only re-scroll when `placement(old).arrangement() != placement(new).arrangement()`, and otherwise just clamp to `max_top`.

### 4. Missing tests for drawn words the ticket added
- The three-pane ASCII tier has no test. `resolver_ascii_tier_is_ascii` (`conflict_resolver/tests.rs:340-353`) lays out `Arrangement::Card` only. The panes' `✓`→`*`, `·`→`-`, `│`→`|` and `▏`→`|` branches (`panes/draw.rs:37`, `:58`, `:146`, `:179`) are unexercised.
- Other drawn words with no test:
  - D3's `base` head over muted base lines (`panes/draw.rs:148-149`).
  - The binary `? open · o t` (`:171`).
  - D2's `(not on …)` (`view/resolver.rs:117-119`).
  - `absent_on`'s `(how, side)` mapping (`view/resolver.rs:108-123`). Only `DeletedByThem` is drawn, in `a_whole_file_shape_shows_each_side`.
- Fix: add `Arrangement::ThreePane` to the ascii test's loop. Add one drawn test for `base_shown` and one table test for `absent_on` over the five `UnmergedHow` shapes.

### 5. Ticket 15 will break `three_panes_capture`
- The capture presses `t` then `n` and asserts `cursor_block == Some(1)` (`tests/it/resolver/three_panes.rs:58-62`).
- Once P2 lands, `t` advances to block 2 itself. Then `n` on the file's last block crosses to the next file (`docs/design/02-keymap.md:267`), and the assert fails.
- Fix: add "drop the `n` in `three_panes_capture`" to ticket 15's scope now, so its builder doesn't read the failure as a regression.

### 6. Ticket 13's "what right looks like" at 200 is stale (textual)
- `13-resolver-file-list.md:125` reads "then the card (three panes are ticket 14's)".
- `docs/shots/m6/fix-file-list-200x50.txt`, regenerated in this diff, now shows three panes.
- Fix: reword it to "then the three panes (ticket 14)" before the contact sheet (ticket 21) quotes it.

### 7. `pane_number_widths` is recomputed for every drawn row
- `panes/draw.rs:60` calls it inside `draw_pane_row`, once per `Aligned` row. It walks every region each time, so the cost is O(visible rows × regions) per frame.
- It is unmeasured, and `TooLarge` caps the file, so this is unlikely to matter.
- Fix: compute it once per `render`, alongside the layout.
