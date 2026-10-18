# Scout 06t — split view/ files (at 2352d87f, after 06h merge)

Paths under `crates/mergewright/src/view/`. No cargo run.

## Sizes now
| file | lines | inline tests | non-test |
|---|---|---|---|
| rows.rs | 844 | none (`mod tests;` :844 → rows/tests.rs 165) | 843 |
| entries.rs | 428 | none | 428 |
| radar.rs | 732 | `mod tests` 251-732 | 250 |
| detail.rs | 407 | `mod tests` 326-407 | 325 |

## rows.rs
- `graph_view` :52-141; header/filters `filters` :142, `mark_stash_only` :151, `modal_blanked` :173, `preview_drawn` :188, `rows_for_frame` :203; location `location` :226, `practised` :250, `home_relative` :260; `opening_view` :271-326 (PlainRow pick :308); `walk_view` :327; footer lines `search_line`/`search_tail`/`compare_footer_line`/`order_notice` :351-404; `reveal_index` :405, `divider_shown` :426, `max_lanes` :440, `compaction` :447 (window); hints `box_context` :491, `stopped_line` :515, `footer_context` :533, `live_c` :559, `tab_word` :578, `pane_hint_context` :598, `overlay_context` :635, `place_context` :691, `hint_context` :722-802 (PlainRow :765), `cursor_file_unmerged` :803, `keep_for` :830.
- `rows/` holds: clash_bars.rs 125 (pub(crate)), context.rs 236, fold.rs 82, tests.rs 165, `fold/` dir.
- Imports `super::entries::{Thread, thread, uncommitted_shown, visible_rows}` (:22); `rows/context.rs` imports back from rows — a move must keep `super::` paths valid (child modules reach parent private fns).

## entries.rs
- `Thread` :22-75; `drag_paint` :76; `visible_rows` :87-180; uncommitted: `uncommitted_shown` :181, `uncommitted_entry` :190, `uncommitted_row` :219, `uncommitted_subject` :267; cells/badges: `row_entry` :291-371, `stopped_here` :372, `badge_view` :383-428.
- 06r's node helper is already `view/node.rs` (+ `node/tests.rs`), not here.

## radar.rs
- facts: `repo_facts` :35, `remote_names` :56, `findings_for` :78; marker: `marker_for` :143; lines: `radar_lines_for` :158, `title_for` :231. Outside callers: update/context_menu, update/detail/control, update/radar, update/tests/evil_probe_retry, view/detail, view/entries, view/strip.

## detail.rs
- `detail_view` :31; headers :96-283 (commit/compare/working tree); `body_view` :284. No radar-lines section here (radar lines live in radar.rs).

## Contradictions with the ticket
1. **`tests/it/stop_ladder_readers.rs:57` pins `"mergewright/src/view/rows.rs"` as the reader of `stop_wordings`** (via `stopped_line` :515). Moving `stopped_line` out of rows.rs requires editing that test's expected list — the ticket allows only `use` edits.
2. detail.rs has no "radar lines section"; the seam named doesn't exist. Non-test is 325: moving tests (:326-407) alone gets <400 and the ticket's rule permits it.
3. radar.rs non-test is 250; its three jobs total 250 lines — the tests (482 lines) are the bulk; tests-only move gets <400 and is permitted.
4. entries.rs 428 with no tests; node helper not here (already view/node.rs).
5. Review cite `entries.rs:300-308,341-346` falls inside `row_entry` :291-371 (could not confirm the review's meaning without reading it).
- Comment-only path cites: view/tests.rs:4942, update/context_menu.rs:48 (entries.rs), ghostty_shot.rs:179 (rows.rs).

## Build with
Sonnet: plain moves; radar/detail are tests-out; rows needs a call on contradiction 1.
