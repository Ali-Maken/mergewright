# scroll-cursor-under-uncommitted-row — held `j` keeps the cursor on a row the graph draws

Status: ready (planner 2026-10-02, 47 ruling 3). Build before the M7 stop. Scout first.
Build with: Opus. The approach is decided here and a red test judges it. The work is wide (about 20 callers to
audit) but each one is a mechanical check. It is not Fable: a wrong caller shows up as a failing test, not as a
structural mistake.
Scout first (ADR-0035): one `scout`, ≤ 1 page, on `AppState::graph_rows` and its callers, on how the views count
history rows under the uncommitted row (`view/entries.rs`, `view/strips.rs`), and on the mouse's row math
(`update::geometry`, `update/mouse.rs`). It reports which callers want **history rows** and which want **pane
height**.
Needs: 47 merged (its captures and TESTING row are re-taken here). No signature from another open ticket.
Blocked by: 47

## Need
Who: him, daily, on a dirty tree (his repo usually is). What hurts: holding `j` makes the highlighted row slide off
the bottom of the pane and stay gone. Each `j` still moves, but he can't see where he is. Seen in 47 Q1. Confirmed
by `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`
(`.scratch/m7-power-tools/47-q1-cursor-test.patch`): after 17 `j`, the cursor is at 17, `top` 0, `graph_rows` 18,
and 18 rows are drawn, none of them the cursor's.

## Cause (47 debugger NOTES, lines named there)
There are two numbers for one fact:
- The views draw `graph_rows() − 1` history rows when the uncommitted row is pinned on top
  (`view/entries.rs:109-113`, `view/strips.rs:211-212`).
- Every scroller passes `graph_rows()` as the viewport height (`update/keys.rs:439` for `j`, plus the callers in
  Touches).

This is the m5/25 failure again: the same arithmetic in more than one copy, and the copies disagree.

## Decided
1. **One number.** `AppState::graph_rows()` keeps its documented meaning, "history rows the graph pane shows"
   (`crates/graft/MAP.md:233`). It now subtracts 1 when `uncommitted_row_shown()`, the same way it already subtracts
   the commit-graph line (`:161`), the pending block (`:420`), the banner (`:148`) and the preview box (`:855`). It is
   still never zero (see 4).
2. **The views stop subtracting.** `view::entries` and `view::strips` draw `graph_rows()` history rows, below the
   uncommitted row when it shows. Every screen row lands where it does today. Only the scroll arithmetic changes.
3. **The callers stay as they are**, with two exceptions. A caller that wants the **pane's** height, if the scout
   finds one, reads it from where the layout carves it; it does not add 1 back by hand. A caller that already
   compensates for the uncommitted row (grep `uncommitted_row_shown` / `uncommitted_shown` near each call) **drops
   its compensation**, or it would be off by one the other way.
4. **A one-row pane with the uncommitted row.** If the app's minimum size guarantees a graph pane of at least 2 rows,
   nothing special is needed; cite the line that guarantees it. If the scout finds a reachable 1-row pane, **stop and
   report**: what draws there is a spec decision, not a build one.
5. **Keep the cursor in view when the pane's history rows shrink.** `graph_rows()` now changes when the tree turns
   dirty, because a status job answers. A cursor on the last drawn row would then fall off. After `update`, if
   `graph_rows()` shrank and the cursor was on screen before the event, shift `top` so it stays on screen. The rule
   applies only when the cursor was on screen: a cursor the wheel left off screen stays off screen (m7/37, MAP.md
   `:69`). It does nothing while `on_working_tree` is set, because then the cursor is the uncommitted row itself.

## Signatures
```rust
// crates/graft — signature unchanged; meaning sharpened, doc comment updated
impl AppState {
    /// History rows the graph pane shows at the current size: the pane less the chrome, the banner,
    /// the pending block, the preview box, the commit-graph line and the pinned uncommitted row. Never zero.
    pub fn graph_rows(&self) -> /* return type as today */;
}

// crates/graft/src/update/ — one place, called once at the end of `update`
pub(crate) fn keep_cursor_in_view(before: &AppState, after: &mut AppState);
```

## Scope
In:
- `graph_rows`, and the two views' row counts.
- `keep_cursor_in_view`.
- The caller audit.
- The tests below.
- Re-taking 47's two captures.
- MAP.md lines for `graph_rows`, `PixelRows::uncommitted_on_top` and `strip_marks`'s viewport.

Out: any key change; the wheel's off-screen cursor; strip cache sizing (that is `scroll-pixel-memory-budget`).

## Touches (each checked and named in the report)
- **Every `graph_rows(` caller.** Grep the whole workspace; don't trust this list. Debugger's count:
  - `update/keys.rs:408,439`, `update/history.rs:21,63`
  - `update/mod.rs:743,864,1039,1286`, `update/mouse.rs:189,209`, `update/search.rs:147,178,198`
  - `update/map.rs:204`, `update/keys/actions.rs:22`, `update/keys/escape.rs:10`
  - `update/detail/landing.rs:147,177`, `update/preview.rs:373`, `update/rewrite_editor/after.rs:69`,
    `update/banner/stop.rs:319`

  For each caller, the report says one of: history rows, unchanged and now right; pane height, switched to the
  layout's carve; or compensated, compensation removed.
- **Views:** `view::entries` (`:109-113`) and `view::strips` (`:211-212`, `PixelRows::uncommitted_on_top`). Also
  `view::strip_marks`: its viewport `top..top+graph_rows` (MAP.md `:856`) was one row too tall with a dirty tree,
  and is now right.
- **Preview:** `view::preview::picture_start` / `PreviewState::view_start` and `update/preview.rs:373`. Check a
  preview opened with a dirty tree.
- **Mouse:** `update::geometry::{rows_above_graph, uncommitted_shown}`, the bound checks in `update/mouse.rs`, and
  `picker_box_in` (MAP.md `:657`). Clicking the last drawn history row with a dirty tree must select that row. The
  overview-strip click (`select_row`) must still land.
- **Pixel:** `StripCache::strips_for(viewport)` gets a viewport one row shorter with a dirty tree.
  - The idle prefetch in `event_loop/graphics.rs` may read the same viewport.
  - The history job's `ShowRows` window may read it too.
  - The fast tier `pixel_scroll::*` must stay green. The transmit counts in 47's run are re-checked at the M7
    review (47 script).
- **Shared state, read only:** `uncommitted_row_shown` / `uncommitted_row_attached` are not changed. Grep their
  callers anyway, to find the compensations in Decided 3.
- **Captures:** grep `docs/shots/` and the capture tests for any frame with a dirty tree and the cursor in the
  bottom rows. Re-take each one that changes and list it. 47's two are re-taken in every case (Acceptance).
- **Docs:**
  - TESTING.md 47 section, row "cursor on the row shown at release": this ticket ticks it.
  - `docs/shots/m7/fix-scroll-contact.md`: captions updated.
  - No DESIGN.md change. The behaviour returns to what the design already says.
- **Not touched:** the keymap (no keys change); journal and undo (no operation); `graft-ui` (`CHROME_ROWS` read
  only); `graft-graph` (no API change).

## Acceptance (each tick names its test)
- [ ] Test-first. Apply `.scratch/m7-power-tools/47-q1-cursor-test.patch` and commit it red. Then green:
      `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`.
- [ ] **Every graph-pane key that moves the cursor**, table-driven, on a clean tree and on a dirty one, at 20 and 50
      rows: the cursor's row is among the drawn rows after the key.
      - Keys: `j`/`k`, `↓`/`↑`, `g`/`G`, `Home`/`End`, `PgUp`/`PgDn`, search `n`/`N` (`docs/design/02-keymap.md`
        §2.2, `:44`, `:185`). The builder adds any other graph-pane cursor key that doc names.
      - Each key is pressed repeatedly, past the pane's edge.
      - Also covered: landing after an operation (`detail/landing.rs`).
      - New: `update::tests::every_cursor_key_keeps_the_cursor_drawn`. This is the guard that stops a future
        scroller drifting.
- [ ] Pixel twin: `pixel_rows` after held `j` with the uncommitted row pinned. The cursor's row is in a strip, and
      `cursor_on_screen` is true. New: `view::strips::tests::held_j_keeps_the_cursor_lit_under_the_uncommitted_row`.
- [ ] Status arrival: the cursor is on the last drawn row of a clean tree, then a dirty status arrives, and the cursor
      is still drawn. Then wheel the cursor off screen, let another status arrive, and the cursor stays off screen.
      New: `update::tests::a_dirty_tree_arriving_keeps_a_shown_cursor_shown`.
- [ ] Mouse: with a dirty tree, clicking the last drawn history row selects that row. New:
      `update::mouse::tests::click_on_the_last_row_under_the_uncommitted_row`.
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
Medium (~45k). One crate (`graft`).
