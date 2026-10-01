# scroll-cursor-under-uncommitted-row — held `j` keeps the cursor on a row the graph draws

Status: ready (planner 2026-10-02, 47 ruling 3; corrected against scout `.scratch/m7-power-tools/scouts/48.md`).
Build before the M7 stop. Scout done (HEAD 75306603).
Build with: Opus. The approach is decided here and a red test judges it. The work is wide (about 20 callers to
audit) but each one is a mechanical check; the two judgement points (mouse row math, 1-row pane) are decided below.
It is not Fable: a wrong caller shows up as a failing test, not as a structural mistake.
Needs: 47 merged (its captures and TESTING row are re-taken here). No signature from another open ticket.
Blocked by: 47 (merged, 7a5b362b)

## Need
Who: him, daily, on a dirty tree (his repo usually is). What hurts: holding `j` makes the highlighted row slide off
the bottom of the pane and stay gone. Each `j` still moves, but he can't see where he is. Seen in 47 Q1. Confirmed
by `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`
(`.scratch/m7-power-tools/47-q1-cursor-test.patch`, committed on main as a file, **not applied** to `crates/`):
after 17 `j`, the cursor is at 17, `top` 0, `graph_rows` 18, and 18 rows are drawn, none of them the cursor's.

## Cause (47 debugger NOTES; scout 48 confirms at HEAD 75306603)
There are two numbers for one fact:
- `AppState::graph_rows` (`update/state.rs:478-499`) has no uncommitted term.
- The views draw `graph_rows() − 1` history rows when the uncommitted row is pinned on top
  (`view/entries.rs:109-113`, `view/strips.rs:211-212`).
- Every scroller passes `graph_rows()` as the viewport height (`update/keys.rs:439` for `j`, plus the callers in
  Touches).

This is the m5/25 failure again: the same arithmetic in more than one copy, and the copies disagree.

## Decided
1. **One predicate, one number.** Three methods on `AppState`, all in `update/state.rs`, nothing else computes them:
   - `graph_pane_rows()`: today's `graph_rows` formula, renamed: the pane less chrome, banner, commit-graph line,
     tabs, preview, pending block. `.max(1)` as today.
   - `uncommitted_row_pinned()`: `uncommitted_row_shown() && graph_pane_rows() >= 2`. "The uncommitted row takes the
     pane's top row." Every place that offsets drawn rows or click rows by the uncommitted row reads **this**, not
     `uncommitted_row_shown()`.
   - `graph_rows()`: keeps its documented meaning, "history rows the graph pane shows" (`crates/graft/MAP.md:233`):
     `graph_pane_rows() − uncommitted_row_pinned() as <its type>`. Never zero, because pinning needs ≥ 2 pane rows.
   `uncommitted_row_shown()` keeps its meaning (the row exists in the list: `k` at the top reaches it,
   `update/selection.rs:40,80`). It is not changed.
2. **The views stop subtracting.** `view::entries` and `view::strips` draw `graph_rows()` history rows, below the
   uncommitted row when `uncommitted_row_pinned()`. Every screen row lands where it does today at ≥ 2 pane rows.
3. **The callers stay as they are**, with these exceptions:
   - **Mouse row math, `update/geometry.rs:302-309` (`graph_row_at`)** wants the pane, not history rows. It changes
     both ways: the bounds check reads `graph_pane_rows()` (not `graph_rows()`), and the subtraction of 1 reads
     `uncommitted_row_pinned()` (not `uncommitted_shown`). Without this the last drawn history row stops being
     clickable. No `+ 1` by hand anywhere.
   - **Every other place that offsets by the uncommitted row** switches its predicate to `uncommitted_row_pinned()`:
     `update/geometry.rs::{rows_above_graph, uncommitted_shown}`, `:422-424` (picker anchor),
     `update/mouse.rs:101,146` (click on the uncommitted row), `view/rows.rs:312` (`reveal_index`),
     `event_loop/graphics.rs:152` (strips lowered by 1). Same value at ≥ 2 pane rows; they differ only at 1.
   - **A caller that already compensates** for the uncommitted row (grep `uncommitted_row_shown` /
     `uncommitted_shown` near each call) **drops its compensation**, or it would be off by one the other way.
     Scout found none among the `graph_rows(` callers. **Unconfirmed: `update/detail/landing.rs:111`** reads
     `uncommitted_row_shown`. The builder reads it, says in Touches whether it compensates the viewport (then drop
     it) or only decides landing on the working tree (then leave it), and covers it in the cursor-key test.
4. **A one-row pane with the uncommitted row** (reachable: no app-wide minimum graph height; `GRAPH_MIN_ROWS = 6`,
   `detail/state.rs:39`, only limits the detail split; `view/tests.rs:181` reaches 2 history rows at 6 terminal rows).
   Decision: **the single row shows the cursor's row.**
   - Not pinned (Decided 1). `graph_rows()` = 1. History scrolls as on a clean tree.
   - Cursor on a commit: the row draws the history row at `top`, so the cursor is drawn.
   - `on_working_tree`: the row draws the uncommitted row in place of the history row (it is the cursor). `top`
     is untouched, so `j` returns to the same history row.
   - A click on that row selects what is drawn there.
   Why: one rule ("the cursor is always drawn") with no new screen, and it degrades a useless size without hiding
   the preview or banner above it. The status counts still show where they show today; the builder checks this
   and names the place in Touches.
5. **Keep the cursor in view when the pane's history rows shrink.** `graph_rows()` now changes when the tree turns
   dirty, because a status job answers. A cursor on the last drawn row would then fall off. After `update`, if
   `graph_rows()` shrank and the cursor was on screen before the event, shift `top` so it stays on screen. The rule
   applies only when the cursor was on screen: a cursor the wheel left off screen stays off screen (m7/37, MAP.md
   `:69`). It does nothing while `on_working_tree` is set, because then the cursor is the uncommitted row itself.

## Signatures
```rust
// crates/graft/src/update/state.rs — `T` is graph_rows' return type today; all three return the same type.
impl AppState {
    /// Rows of the graph pane: the terminal less chrome, banner, commit-graph line, tabs, preview box and
    /// pending block. Never zero. The mouse's bounds check reads this; scrollers do not.
    pub fn graph_pane_rows(&self) -> T;
    /// The uncommitted row takes the pane's top row: it is shown and the pane has room for it and one history row.
    pub fn uncommitted_row_pinned(&self) -> bool;
    /// History rows the graph pane shows: `graph_pane_rows()` less the pinned uncommitted row. Never zero.
    pub fn graph_rows(&self) -> T;
}

// crates/graft/src/update/ — one place, called once at the end of `update`
pub(crate) fn keep_cursor_in_view(before: &AppState, after: &mut AppState);
```

## Scope
In:
- `graph_pane_rows`, `uncommitted_row_pinned`, `graph_rows`; the two views' row counts.
- `graph_row_at` and the predicate switch in Decided 3.
- The 1-row pane rule (Decided 4).
- `keep_cursor_in_view`.
- The caller audit.
- The tests below.
- Re-taking 47's two captures.
- MAP.md lines for `graph_rows` (`:233`), the two new methods, `PixelRows::uncommitted_on_top`, and
  `view::strip::strip_marks`'s viewport. **`strip_marks` is at `view/strip.rs:69`** (viewport `:72`); fix its MAP
  line, wherever MAP has it.

Out: any key change; the wheel's off-screen cursor; strip cache sizing (that is `scroll-pixel-memory-budget`).

## File set (one builder; no concurrent builder may hold these)
All in `crates/graft`:
- `src/update/`: `state.rs`, `geometry.rs`, `mouse.rs`, `selection.rs` (read; change only if Decided 4 needs it),
  `keys.rs`, `keys/actions.rs`, `keys/escape.rs`, `history.rs`, `mod.rs`, `search.rs`, `map.rs`,
  `detail/landing.rs`, `preview.rs`, `rewrite_editor/after.rs`, `banner/stop.rs`, the new `keep_cursor_in_view`
  home, `tests.rs`.
- `src/view/`: `entries.rs`, `strips.rs`, `strips/tests.rs`, `strip.rs`, `rows.rs`, `preview.rs`, `tests.rs`.
- `src/event_loop/`: `graphics.rs`, `tests.rs`. `src/preview/tests.rs`.
- `tests/it/`: `ghostty_shot.rs`, `held_key.rs`, `frames/readings.rs`, `m7_walk/rewrite_stops.rs`.
- `MAP.md`. Outside the crate: `TESTING.md` (47 section), `docs/shots/m7/`, `docs/perf/`.

## Touches (each checked and named in the report)
- **Every `graph_rows(` caller.** Grep the whole workspace; don't trust this list. Scout's count (all pass it as
  history viewport height, none compensates):
  - `update/keys.rs:408,439`, `update/history.rs:21,63`
  - `update/mod.rs:743,864,1039,1286`, `update/mouse.rs:189,209`, `update/search.rs:147,178,198`
  - `update/map.rs:204`, `update/keys/actions.rs:22`, `update/keys/escape.rs:10`
  - `update/detail/landing.rs:147,177`, `update/preview.rs:373`, `update/rewrite_editor/after.rs:69` (3× height
    resident window), `update/banner/stop.rs:319`
  - view side: `view/strip.rs:72`, `view/preview.rs:85` (`picture_start`)
  - pane height, switched to `graph_pane_rows()`: `update/geometry.rs:302`

  For each caller, the report says one of: history rows, unchanged and now right; pane height, switched to
  `graph_pane_rows()`; or compensated, compensation removed.
- **Every `uncommitted_row_shown` / `uncommitted_shown` reader.** For each, the report says: kept (the row exists)
  or switched to `uncommitted_row_pinned()` (the row takes a screen row). Include `detail/landing.rs:111` with what
  it does (Decided 3).
- **Tests asserting `graph_rows` values** (may move by 1 with a dirty tree): `update/tests.rs:504, 807, 2671-2673,
  5860-5879`; `view/tests.rs:181, 3746`; `preview/tests.rs:664`; `event_loop/tests.rs:1058`; `view/strip.rs:158`;
  `strips/tests.rs:235`; `tests/it/held_key.rs:551`, `frames/readings.rs:306`, `m7_walk/rewrite_stops.rs:183`.
  `ghostty_shot.rs:196-202` already adds a row for `uncommitted_on_top`: switch it to `graph_pane_rows()`, so its
  crop doesn't shrink by 1. A changed expected value says why in the test.
- **Preview:** `view::preview::picture_start` / `PreviewState::view_start` and `update/preview.rs:373`. Check a
  preview opened with a dirty tree.
- **Pixel:** `StripCache::strips_for(viewport)` gets a viewport one row shorter with a dirty tree.
  - The idle prefetch in `event_loop/graphics.rs` may read the same viewport.
  - The history job's `ShowRows` window may read it too.
  - Fast tier `pixel_scroll::*` must stay green. 47's transmit counts are re-checked at the M7 review (47 script).
- **Captures:** grep `docs/shots/` and the capture tests for any frame with a dirty tree and the cursor in the
  bottom rows. Re-take each one that changes and list it. 47's two are re-taken in every case.
- **Docs:** TESTING.md 47 row "cursor on the row shown at release" (this ticket ticks it);
  `docs/shots/m7/fix-scroll-contact.md` captions; MAP.md (Scope). No DESIGN.md change.
- **Not touched:** the keymap (no keys change); journal and undo (no operation); `graft-ui` (`CHROME_ROWS` read
  only); `graft-graph` (no API change).

## Acceptance (each tick names its test)
- [ ] Test-first, red commit before any fix. Apply `.scratch/m7-power-tools/47-q1-cursor-test.patch`
      (`git apply`). If it doesn't apply, re-type the same test by hand from the patch file and 47's debugger
      NOTES (80×20, 60 commits, dirty tree, 40 × `j`, cursor row among the drawn rows each press). Commit it red,
      then green: `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`.
- [ ] **Every graph-pane key that moves the cursor**, table-driven, on a clean tree and on a dirty one, at 20 and 50
      rows: the cursor's row is among the drawn rows after the key.
      - Keys: `j`/`k`, `↓`/`↑`, `g`/`G`, `Home`/`End`, `PgUp`/`PgDn`, search `n`/`N` (`docs/design/02-keymap.md`
        §2.2, `:44`, `:185`). The builder adds any other graph-pane cursor key that doc names.
      - Each key is pressed repeatedly, past the pane's edge.
      - Also covered: landing after an operation (`detail/landing.rs`), with a dirty tree.
      - New: `update::tests::every_cursor_key_keeps_the_cursor_drawn`.
- [ ] Pixel twin: `pixel_rows` after held `j` with the uncommitted row pinned. The cursor's row is in a strip, and
      `cursor_on_screen` is true. New: `view::strips::tests::held_j_keeps_the_cursor_lit_under_the_uncommitted_row`.
- [ ] Status arrival: the cursor is on the last drawn row of a clean tree, then a dirty status arrives, and the cursor
      is still drawn. Then wheel the cursor off screen, let another status arrive, and the cursor stays off screen.
      New: `update::tests::a_dirty_tree_arriving_keeps_a_shown_cursor_shown`.
- [ ] Mouse, dirty tree, 80×20: clicking the last drawn history row selects that row; clicking the uncommitted row
      selects the working tree; a click one row below the pane is not a graph-row hit (`graph_row_at` → `None`).
      New: `update::mouse::tests::click_on_the_last_row_under_the_uncommitted_row`.
- [ ] 1-row pane, dirty tree (size or preview chosen so `graph_pane_rows() == 1`): `uncommitted_row_pinned()` is
      false, `graph_rows() == 1`; held `j` keeps the cursor's history row drawn; `k` at the top onto the working
      tree draws the uncommitted row in that one row; `j` back draws the same history row. New:
      `view::tests::a_one_row_pane_draws_the_cursor_row`.
- [ ] Fast tier `pixel_scroll::*` and `event_loop::tests::queued_keys_draw_once` green, by name.
- [ ] 47's captures re-taken. Run `ghostty_shot::fix_scroll_ghostty_png` (heavy) and the pty `text` walk with
      `GRID_OUT` from a script into `docs/perf/scroll-cursor-under-uncommitted-row.txt`, reading summary lines only.
      `docs/shots/m7/fix-scroll-ghostty.png` and `fix-scroll-release-unicode.txt` must each show the cursor row
      inside the crop (cursor ± 10) and print `cursor_on_screen=true`. Captions say "colour unverified". Rules 5 and
      6: nobody ticks "looks right".
- [ ] TESTING.md 47 row "cursor on the row shown at release" names the tests above.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (with and without `graft/heavy-tests`),
`cargo nextest run`, `cargo test --doc`, all workspace-wide.

## Size
Medium-large (~60k). One crate (`graft`). If the context runs past ~80k, hand off after the red test and the
`state.rs` / views / geometry change; the caller audit and captures go to the next context.
