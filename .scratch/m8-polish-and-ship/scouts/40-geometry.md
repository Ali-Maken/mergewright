# Scout 40: health-geometry-one-source (HEAD 2acf822b)

All paths under `crates/`.

## Mirror 1: `detail_split` vs `split_area`. It differs on purpose and does not fit the ticket's signature.
- Copy: `graft/src/update/geometry.rs:73` (`detail_split(layout, size, banner_rows, hint_rows) -> (Option<Bounds>, Option<Bounds>)`). Source: `graft-ui/src/detail_pane/layout.rs:96` `split_area(layout, Rect)`.
- **Right:** the source takes the gutter off the graph width (`layout.rs:104`, `gutter` at :85). The copy does not (`geometry.rs:84`). This is deliberate. Comment at `layout.rs:81-84`: "the only cell that changes hands is the gutter itself, which routes to the region it came out of rather than to nothing". The test says so too, at `update/tests/tab_strip_geometry.rs:202-207`: "Its width is deliberately one wider here in the `Right` layout". The ticket's stop rule applies.
- **Tab:** the copy also takes `banner_rows`/`hint_rows`. The source takes neither. What the copy really mirrors is `graft/src/view/frame/layout.rs:193` `detail_rects` → `tab_content_area` (:214) → `tab_banner_area` (:228), which is graft-side, not graft-ui. So `split_cells(layout, Cells)` alone cannot replace it.
- Users of the copy: `diff_body_rows` :222, `detail_hit` :277, `graft_row_at` :302, `context_menu_graph_bounds` :350.
- Mirrors the ticket omits, in the same file: `detail_hit_areas` (:158) and `percent_of` (:151) copy `graft_ui::detail_pane::detail_areas` (`layout.rs:159`). `strip_bounds` (:384) mirrors the carve-out in `view::frame::render_graph_screen`.
- Copy==source test: `the_geometry_mirror_matches_the_rects_the_frame_hands_out` (`tab_strip_geometry.rs:221`; widths 20..=240, heights 3/8/24/40/48). It compares against `view::frame::detail_rects`, not against `split_area`. This is the only user of ARCH:267-276's exception that I found.
- Behavioural tests: `a_click_below_one_hundred_columns_still_lands_on_the_row_it_named` (`tab_strip_geometry.rs:93`); graft-ui `pane_boundaries.rs` (named in the comment at :207).

## Mirror 2: `tour::card_rows` vs `layout::tour_rows`
- Copy: `graft/src/update/tour.rs:55`. Source: `graft-ui/src/graph_screen/layout.rs:349` (private, `fn tour_rows(view: &GraphView, left: u16)`).
- The source only does the `left > wanted` test. The copy also rebuilds `left`: `chrome_rows` + harness banner + commit-graph line + tab rows (`tour.rs:59-64`). That carve lives in `screen_rows` (`layout.rs:308`). Making `tour_rows` public removes one line. The `left` arithmetic stays a mirror.
- The source takes `&GraphView`, which update has no way to build. Reaching it needs a new signature, e.g. `(on: bool, left)`. The ticket says "as today".
- Behavioural test: `the_card_takes_three_rows_off_the_graph_while_the_tour_is_on` (`update/tour/tests.rs:276`). I could not confirm a click-row test for the card.

## Mirror 3: `MIN_BOX_ROWS`/`BoxRect` vs `preview_box`
- Copy: `graft/src/update/preview.rs:87` const, `:92` `BoxRect`, and the body `preview_box_rect` in `graft/src/update/preview/rect.rs:28`. Source: `graft-ui/src/preview_box.rs:46` const (private), and `preview_box_area(frame, view)` at :172 (the ticket calls it "dock"). The arithmetic is identical (rect.rs:39-51 vs preview_box.rs:173-187), except that the copy returns `None` at 0×0. The source takes `&PreviewBox`, so `dock_cells(area)` as the ticket signs it is missing the view.
- `choice_at` (rect.rs:63) mirrors the private `frame_box::buttons_row` (`graft-ui/src/frame_box.rs:172`). The ticket does not name it.
- Behavioural tests: `a_click_lands_on_the_button_the_widget_drew` (`update/preview/tests.rs:676`), `the_click_targets_match_what_the_widget_draws` (named at rect.rs:61; I could not confirm its location). The view drawing calls the source directly (`view/frame/boxes.rs:112`).

## Mirror 4: `pending_rows_taken`. The ticket's premise is wrong here.
- Copy: `graft/src/update/banner.rs:319`. Its source is **graft-ui** `graph_screen/layout.rs:357` `pending_rect` (private), called from `screen_rows` :309. It is not a carve in `view::frame`: no `view` file references it (grep). "`view::frame`'s carve calls it" is impossible, because graft-ui cannot call graft. The `left` rebuild (banner.rs:325-331) is a second copy of the same carve as mirror 2.
- Users: `geometry::rows_above_graph` (:37), `AppState::graph_rows` (`update/state.rs:584`).
- I could not confirm any test for the pending block's rows or clicks. The nearest is `pending_view_lists_this_branchs_rows_and_summarises_followers` (`banner/rewrite_stop_tests.rs:526`), which tests content, not geometry.

## Other contradictions
- ARCH:267-270 says production `update` never imports `view`. It does: `crate::view::uncommitted_shown` (`geometry.rs:315`, :432; `mouse.rs:118`, :153) and `crate::view::badge_index_at` (`geometry.rs:468`).
- `Cells`: the precedent `picker_box` (`graft-ui/src/picker.rs:144`) uses a bare tuple. Update's own `Bounds` (`geometry.rs:46`, with `contains`) and `BoxRect` would become conversions from it. Users would be geometry.rs, preview/rect.rs and ticket 38.

## Size
One context covers it, but only if the ticket is first re-specified: the Tab split, the carve function for `left` shared by tour and pending, and `dock_cells` taking a view. **Build with: Opus (medium).**
