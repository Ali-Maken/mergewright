# pixel-roles-and-tall-row — the Pixel tier draws roles and the cursor row's second line; then the Ghostty lag is measured

Status: done — commit 1 (roles + `colours_off`, bcc58b40), settle 2 (4e357112), tall row + capture (9450a77a), measurement (see NOTES 2). Was: ready (planner spec-only pass 2026-10-18, fitted to scout `.scratch/ease-pass/scouts/13-pixel-roles-and-tall-row.md` at c021e6a6: 5 mismatches ruled, `colours_off` settled by a test first)
Blocked by: 07b-graph-rows-cursor (`SecondLine`, `commit_rows`, `body_line`; the tall row's input). Built and merged: 01, 03, 06, 06b, 06s, 06t, 07 part (f2269602: `Look.route`, `Look.colours_off`, `lane_roles` at `view/entries/roles.rs:16`). 07c is not needed (scout). **Not beside 09**: 09 changes `view/frame/layout.rs:169` `graph_columns`, which sets the strips' width. Either order works; the later one re-takes its captures. The measurement runs with no other cargo-running helper active.
Build with: Opus — raster, cache key, placement and a perf script; the tall row's placement is a judgement the tests then pin (scout's call). Not Fable: correctness is judged by trace and tests, not by eye.
Needs: graph `LaneRole` (`role.rs:14`), `PlainStrip<'a>` (`raster/emphasis.rs:164-165`, `Clone, Copy`, borrowed fields), `StripKey` / `plain_layout_hash` (`raster/cache/key.rs:~14,123`), `stroke_width` (`raster/pen.rs:30`), tiny-skia dash support (`raster/geometry.rs:154-160`); app `look.route` via `frame_route` (`view/entries/roles.rs:27`, `pub(in crate::view)` — `view::strips` is inside `view`), `look.colours_off` (`update/state/look.rs:10`), `lane_roles`; strips `view/strips.rs` (palette :49, `graph_rows` :215, strip cut :279-318), placement `event_loop/graphics/layers.rs:142-175`; 07b `SecondLine`, `commit_rows`, `body_line`.

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours G1, G2, G9, G11, G14; Genuinely risky tickets; Question 4 (answered a: build both, then measure; results go to him before anything is cut).
- `docs/design/03-graph-layout-algorithm.md` §3.8. ADR-0021 (release frame budgets), ADR-0053 amendment 1 (pieces).
- `crates/mergewright-graph/MAP.md`: `raster`, `raster::cache`, `fold`. `crates/mergewright/MAP.md`: `event_loop::graphics`, `view::strips`, `trace`. `crates/mergewright-term/MAP.md`: the Kitty/iTerm2 placement entries.
- `scripts/scroll-lag-big-repo.sh` header, `scripts/scroll-lag-pty.py` usage (Ghostty-profile pty), `scripts/m8-trace-summary.sh` usage, `scripts/41-scroll-team-fixture.sh` usage.
- `07b-graph-rows-cursor.md` Decided (second line, `commit_rows`, cursor never on the last line).
- The scout's file.

## Settle first — committed tests, before the code they decide
1. **`colours_off` reaches the strips** (scout: *could not confirm*; the palette comes from `session.theme.raster_lanes()`, `view/strips.rs:49`, not `Look`). Test `colours_off_mutes_every_strip_lane`: set `look.colours_off`, build the frame's strips, and assert every lane takes the muted hue and `theme_id` differs from colours-on. Expected red today. It must pass after commit 1: when `colours_off`, the palette is muted for every role, `theme_id` re-keys and strokes stay the same. NOTES records whether it was red.
2. **Two placements of one strip image** (M3, below). A placement-command test for **Kitty** and **iTerm2** on a strip holding the cursor row: two placements with disjoint source rows and one transmit. If the iTerm2 path must re-send the image for the second placement, the test records the bytes and NOTES says so. The perf script measures it; no fallback is chosen here.

## Decided here (the scout's mismatches)
- **M1 — `PlainStrip` stays `Copy`:** roles ride as a borrowed slice, `roles: &'a [(BranchIdentity, LaneRole)]`; an identity absent from it is `Other`.
- **M2 — the stroke enum is `LaneStroke`**, so it doesn't clash with `tiny_skia::Stroke` (`raster/geometry.rs:5`).
  - This thick, Under thin, Trunk double, Other dotted grey (tiny-skia dash, `geometry.rs:154-160`); connectors in the joining branch's role.
  - Width still comes from `stroke_width(cell_width)` per strip. The role scales it (This ×1.5, Under ×1, Trunk two ×1 strokes, Other ×1 dashed), so `pen.rs:112` "never varies with emphasis" still holds.
- **M3 — the cursor row sits inside a strip.** Strips are `strip_rows` history rows (`view/strips.rs:279-318`), so offsetting whole strips is not enough.
  - The cursor's strip is placed **twice from the one raster**: rows up to the cursor at `at`, rows after it at `at + 1`, each cropped by `rows: Range` (`layers.rs:142-175`).
  - Strips wholly below shift one cell row; strips above are unchanged.
  - The continuation band (lanes straight through, one cell row) is its own one-row piece.
  - `j` within one strip changes only placements, never the raster. Re-rasterising with a gap is **rejected**: it costs a raster per `j`.
- **M4 — heights from 07b:** `graph_rows` (`strips.rs:215`) counts with `commit_rows`, and screen line → row goes through `body_line`.
- **M5 — the route is `look.route` through `frame_route`** (view-internal, no visibility change). Roles enter the strip key: `plain_layout_hash` includes `roles`. A focus change re-rasterises; `j` within one branch does not.
- Commit in **two commits**: (1) roles + `colours_off`, (2) tall row. The script measures each.

## Scope
In: mergewright-graph `raster`, `raster::cache`; app `view::strips`, `event_loop::graphics` (placement); `scripts/ease-pixel-lag.sh`.
- **Measurement (Q4 a)** — `scripts/ease-pixel-lag.sh`, run by the builder **with no other cargo-running helper active**, writes `docs/perf/ease-pixel.txt`. The builder reads only its `SUMMARY` lines.
  - Release binaries at base (before commit 1), roles, and roles + tall row, each in its own `CARGO_TARGET_DIR` outside `target/`.
  - Each run: team fixture (`41-scroll-team-fixture.sh`), Pixel tier, probe answered like Ghostty (kitty), 120×40 and 200×50, `j` held at 30 Hz for 3 s, at open and settled (`scroll-lag-pty.py`); plus a focus-changing walk (cursor alternating branches every 5 rows).
  - `SUMMARY` per binary × size × walk: `input_age_us p50/p95/p99`, `draw_us p95`, `bytes p95`, `tail_ms`, then deltas vs base.
  - Last lines: the exact command for him to run in his own Ghostty (`MWR_PERF_TRACE=<file> mwr <copy of his work repo>`, hold `j` 3 s, quit) and the summariser line → `docs/perf/ease-pixel-ghostty.txt` (`scripts/m8-trace-summary.sh`).
- **No cut, no fallback, no switch** on the result. The numbers go to the conductor, who takes them to him with Q4's options (b)/(c) still open. ADR-0021 budgets are judged in the milestone review's release run.
Out: Unicode tier (07/07b/07c), roles' meaning (01), the right pane (09), which columns are drawn (06b).

## Signatures
mergewright-graph `raster`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneStroke { Thick, Thin, Double, Dotted }
impl LaneStroke { pub fn of(role: LaneRole) -> LaneStroke; }
#[derive(Clone, Copy)]
pub struct PlainStrip<'a> { /* existing fields */ pub roles: &'a [(BranchIdentity, LaneRole)] }   // absent → Other
/// The cursor row's second line inside a strip: lanes straight through, one cell row tall.
pub fn rasterise_continuation(strip: &PlainStrip<'_>, row: usize) -> Option<RasterFrame>;
```
app `view::strips`:
```rust
pub(crate) struct TallRow { pub(crate) row: usize }   // history row with the second line
// the cursor's strip yields two placements (rows ≤ row at `at`, rows > row at `at + 1`); strips below shift one row
```
`plain_layout_hash` includes `roles`; `rasterise_backdrop`/`emphasis_pieces` keep their signatures.

## Touches
Strip cache keys and hit rates (`CacheStats` in NOTES before/after); `RasterPool` buffer sizes; Kitty and iTerm2 placement (two placements per cursor strip); `event_loop::graphics` frees on resize; frame trace fields; Pixel captures (text captures show placement only); memory budget (`scroll-pixel-memory.sh`, review); 06b's per-window column drop (whether it raised the miss rate at base, NOTES); 09's pane width (strip width).

## Acceptance
Each tick names its test inline.
- [x] Settle tests 1–2 committed first; outcomes in NOTES. Settle 1: `view::strips::roles_tests::colours_off_mutes_every_strip_lane` (red first, NOTES 1). Settle 2: `mergewright-term tests/it/two_placements.rs::{kitty_places_one_upload_twice_with_disjoint_crops_and_one_transmit, iterm2_sends_two_bands_and_a_j_inside_the_strip_re_encodes_both}`, committed alone in 4e357112 before the tall row (NOTES 2).
- [x] `LaneStroke::of` table test (`raster::stroke::tests::each_role_has_its_own_stroke`). The strip key differs when one lane's role changes and is equal across `j` within one branch (cache test, hit counter) — `view::strips::roles_tests::a_strip_rekeys_on_a_role_change_and_hits_across_j_within_a_branch`.
- [x] Raster test (`tests/it/raster_roles.rs::other_is_dotted_and_trunk_is_two_strokes_and_this_is_wider_than_under`, `plain_strip_is_still_copy`): Other lane pixels dotted (gaps every N px), Trunk two parallel strokes, This wider than Under (measured in the frame buffer, not by eye). `PlainStrip` still `Copy` (compile).
- [x] `colours_off`: every lane pixel in the muted hue, stroke shapes unchanged (`colours_off_mutes_every_strip_lane`).
- [x] Tall row: strips above the cursor not re-rasterised on `j` (counter); the cursor's strip placed twice with disjoint rows, strips below one row lower (placement command test, both protocols); the continuation frame has a straight segment in every lane column passing the row. — `event_loop::tests::pixel_scroll::tall::{a_j_that_keeps_the_plan_rasterises_no_strip_above_or_below, kitty_places_the_cursor_strip_twice_the_band_between_and_the_next_strip_lower, iterm2_writes_two_bands_of_the_cursor_strip_the_band_and_the_lower_strip}`; graph `tests/it/raster_continuation.rs::every_lane_leaving_the_row_runs_straight_through_its_second_line`.
- [x] `scripts/ease-pixel-lag.sh` exits 0 (run once, alone, 2026-10-19 07:20Z; machine ac=1 lowpower=0); `docs/perf/ease-pixel.txt` has `SUMMARY` lines for all three binaries, both sizes, both walks, and the Ghostty command lines. The report quotes only `SUMMARY` lines. — verified by reading `docs/perf/ease-pixel.txt`: 18 per-run + 12 delta SUMMARY lines, Ghostty lines last (a script, no test).
- [x] Capture: `docs/shots/ease-pass/main-pixel-120.txt` (placement text capture; the pixels are his to judge in Ghostty) — `event_loop::tests::pixel_scroll::tall::capture::capture_the_pixel_main_screen_with_the_cursor_row_s_second_line`.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide; plus `scripts/ease-pixel-lag.sh` run once, alone.

## Size
~70k. Split point: roles + `colours_off` (commit 1) here; tall row + measurement as a follow-on.

## NOTES (builder, 2026-10-19) — partial: commit 1 only

No compaction. Stopped at the ticket's own split point (commit 1), quota tight. Scout: yes — fresh one at 60e07871 (`scratchpad/scout-13b.md`, not committed); the c021e6a6 anchors were re-mapped through 07b's NOTES.

**Settle test 1 — red, as the scout suspected.** `view::strips::roles_tests::colours_off_mutes_every_strip_lane` was run red before the fix: `assertion left != right failed: colours off re-keys the strip` (graphics built `StripPaint::of(session)` and never saw `look.colours_off`). Green after commit 1: `StripPaint::for_frame` (`view/strips/paint.rs`) paints from `session.colours_off()` while `#` is on, so the palette is the one muted hue, `theme_id` re-keys, and the alpha channel is byte-equal (shapes stay).
**Settle test 2 — not written** (it decides the tall row; left for the follow-on with it).

**Built (commit 1).**
- graph `raster/stroke.rs`: `LaneStroke {Thick, Thin, Double, Dotted}`, `LaneStroke::of`, `role_in` (absent → `Other`), `lane_look`, `styled` (dotted = tiny-skia dash 1.5w on/off; double = one wide stroke with its middle masked out, so arcs stay parallel). Straight runs: Double = two scanline-copied runs at ±offset; Dotted is stroked whole.
- Width: `stroke_width(cell_width)` × role (This 1.5, others 1). **Node rings always draw at the base width, plain** (`pen.rs` `ring_ink`): a ring's shape is its ring style alone.
- Piece keys hash (colour, stroke) where they hashed colour (`emphasis/pieces.rs`), so equal keys stay equal pixels.
- `plain_layout_hash` hashes the role of every branch the strip draws (rows, stubs, row above) — not the whole frame's roles, so a focus change re-keys only strips whose lanes' roles changed.
- app: `PixelRows.roles` from `frame_route` (the Unicode tier's route), one pair per branch drawn; `StripPaint` palette is now `Theme::raster_roles()` (new, ui: `role-*` truecolor hues, or the muted hue at depth None).

**Decided here (say if wrong):**
- **`roles: Option<&[(BranchIdentity, LaneRole)]>`, not a bare slice.** `None` = no roles in play: identity colours, one plain stroke, the old look — keeps the graph crate's corpus snapshots and pixel tests byte-identical. `Some(&[])` = every lane `Other`, as the spec says. The app always passes `Some`.
- **Colour follows role too, not only stroke** (G1 "colour = role"; the Unicode tier colours by `role_token`). With roles the palette is indexed by role (This, Under, Trunk, Other); a one-colour palette mutes all. `Theme::raster_lanes` is now unused by the app (kept; ui MAP lists both). Question: drop it?
- Double's gap is `max(0.75w, 1 px)`, so a trunk lane is about 2.75w wide overall, wider than This (1.5w). Judge by eye in Ghostty.

**Knock-on found by the full run (Touches):**
- `event_loop::tests::pixel_scroll`: ADR-0053's "a j re-sends no strip" no longer holds when the j moves the focus onto another branch: roles re-key the strips. This is the Q4 lag risk, now real; **the follow-on's measurement must quantify it.**
  - `j_inside_resident_strips_sends_no_strip` moved to `pixel_scroll/focus.rs`, now asserts no strip on every j whose plan (fold, window, roles) is unchanged, and that ≥10 of 20 j keep the focus.
  - `a_capped_graph_resends_strips_only_when_the_fold_changes`: `Planned` gained `roles`, so (a) reads "fold, window or roles changed".
  - `emphasis_pieces::{sent_and_new}`: counts emphasis transmits only; no longer asserts that no strip went.
- Ratchet (lowered, none raised): `raster/emphasis.rs` split (`emphasis/backdrop.rs`), row removed; `view/strips.rs` split (`strips/paint.rs`) 422→415; `pixel_scroll.rs` 650→628; graph tests `raster.rs` 1003→992 and `strip_cache.rs` 713→704 (struct-update over their helpers, `bare`); `emphasis_layer.rs` held at 588 (merged a `use`).
- MAP.md: graph (`LaneStroke`, `PlainStrip.roles`, key), ui (`raster_roles`), app (`view::strips`).
- Text captures: none changed (strips are pixels; no capture test failed). Insta raster snapshots unchanged (`roles: None`).
- `CacheStats` before/after in a real walk: **not measured** (the follow-on's script).
- Colour and pixels: **unverified** (rule 5); no PNG looked at.

**Remaining (for the conductor to split):**
1. Settle test 2 (Kitty/iTerm2 two placements of one strip). Scout: Kitty counts p=1..n per upload (`mergewright-term/src/kitty/mod.rs:187`, module doc :9 says one p — stale); iTerm2 has no crops and caches bands by `rows` (`iterm2.rs:138-145`), so a j inside a strip re-encodes both PNG crops; twin bands count against `STRIPS_KEPT=16` (`graphics.rs:32`).
2. Tall row (commit 2). Today in Pixel: `view/frame.rs:152-158` clears `second`, ui draws no blank line, but `graph_rows` takes one row off in every tier (`update/state/queries.rs:28-38`) and app `cursor_body_line` (`update/geometry/second_line.rs:25-39`) is tier-blind — a Pixel click may map one line off (scout, **unconfirmed by test**). The follow-on must settle that first.
3. `scripts/ease-pixel-lag.sh` + `docs/perf/ease-pixel.txt` (base = 60e07871, roles = this commit, roles+tall).
4. Capture `docs/shots/ease-pass/main-pixel-120.txt`.

Acceptance: 3/7 ticked (boxes 2, 3, 4); box 1 half (settle 1 done, settle 2 not).

## NOTES 2 (builder, 2026-10-19) — follow-on: settle 2, tall row, capture, measurement

**Compacted once, mid-ticket** (after the tall row's code and tests, before the capture's gates). Nothing was left half-done across it; everything after was re-checked by the full gates. Scout: none this session — the earlier scout file (`scratchpad/scout-13b.md`) and NOTES 1's anchors were enough, files read directly.

**Settle test 2 — passed at once, both protocols** (committed alone, 4e357112, before any tall-row code). Kitty: one transmit, placements `p=1` (`y=0`, rows 0..=cursor) and `p=2` (source `y` past the cursor, one row lower) of the same id; a `j` inside the strip sends placements only. iTerm2: no ids, no crops — two composited bands; a `j` inside the strip **re-encodes both crops** (encoded 2 → 4; bytes 720 B for the first frame, 870 B after one `j`, in the test's small strip). In the tall row: `iterm2_held_j_writes_four_bands_and_re_encodes_the_cursor_strip_s_crops` measures 48 encodes over 20 `j` on the branchy fixture (≥ 2 per `j`). No fallback chosen.

**Found and fixed at base (cause):** the Pixel tier cleared the cursor row's second line (`view/frame.rs`, old `set_pixel`) while `graph_rows`/`cursor_body_line` still reserved it, so every row below the cursor was drawn one line above where a click mapped it, and the pane's last line was blank. The scout's *unconfirmed* off-by-one is now confirmed by `view::tests::pixel_preview::at_the_pixel_tier_every_row_sits_where_the_hit_test_says` — red at row 52 (first row below the cursor) under the old clearing, green now. The second line now draws at every tier; the strips make room.

**Built (commit 9450a77a).**
- graph `raster/pen/continuation.rs`: `continuation_key`, `rasterise_continuation` — one cell row; every lane leaving the row straight through (arcs leaving it included), the thread lit as in the row, the rail drawn when lanes fold.
- app `view/strips/tall.rs` `TallRow`; `event_loop/graphics/tall.rs` `make_room` (splits the cursor's strip at the cursor, shifts strips below +1, `on_screen` over height − 1) and `Band` (raster skipped when Kitty holds the key).

**Deviations from the Signatures (say if wrong):**
- `rasterise_continuation(strip, sel: &Selection, row) -> Result<Option<RasterFrame>, RasterError>` — takes the selection so the thread stays lit through the band, returns `Result` so a selection/strip length mismatch is an error, not a silent `None`.
- `TallRow { line: u16 }` (the graph line of the cursor row, from `cursor_body_line`) rather than `row: usize`: the placement needs the line, and the line already accounts for the uncommitted row and scroll.
- The band uploads as `Layer::Emphasis` keyed by its content key (one upload per distinct band, reused while held), not as a strip slot — strips stay one upload per slot.

**Touches (each checked by the full nextest run, 7048/7048):**
- Snapshot `view/snapshots/mergewright__view__tests__renderer-Pixel.snap`: now shows the second line.
- Capture `docs/shots/m8/polish-ghost-pixel-120x36.txt` (per-scenario capture, not a sheet; 07b re-shot it too): the closed frame gains the second line and loses the blank last line. The m8 contact sheet was **not** edited (past milestone).
- `pixel_scroll::emphasis_pieces` counts the band's key among the emphasis uploads; `iterm2_bands_are_composites` moved to `pixel_scroll/tall.rs` and rewritten for four bands a frame.
- Ratchet lowered, none raised: `view/strips.rs` 415→404 (`stubs_of` → `strips/paint.rs`), `pixel_scroll.rs` 628→604.
- MAP.md: graph (`continuation_key`, `rasterise_continuation`), app (`view::strips::TallRow`, placement around it).
- Hit-test/scroll: unchanged code; `at_the_pixel_tier_every_row_sits_where_the_hit_test_says` pins it.
- Unicode/ASCII tiers: unchanged (their captures and snapshots did not move).
- Colour and pixels: **unverified** (rule 5). Capture is placement text only.
- `CacheStats` before/after: **not measured** per strip cache; the script measures bytes and transmits instead.

**Measurement (`docs/perf/ease-pixel.txt`, SUMMARY only; no cut/fallback/switch).** p95 input_age_us, base → roles → tall, team fixture: 120x40 open 23521→35978→39263, settled 19289→37472→28262; 200x50 open 17248→36201→42150, settled 22094→33625→41695. bytes p95 ~106–122 KB → ~134–147 KB (+25–30 KB). draw_us p95 +1–8 ms. tail_ms flat (174–225 ms in all three; the backlog was already there at base). Focus walk (2-branch fixture, branch every 5 rows): bytes p95 2.3 KB → 37 KB, transmits 17 → 54–58, input_age p95 +0.8–4.3 ms. Single runs, noisy (p99 moves both ways). Ghostty numbers still need his run (command in the file).
- The focus walk uses a generated two-branch history (script builds it), not the team fixture: the team fixture gives no way to make the cursor change branch on a fixed period.
- `crates/mergewright-fixtures/examples/team_path.rs` (new, prints the MONOREPO cache path): the first route, building the fixture through a heavy `held_key::` test, **does not compile**: `crates/mergewright/tests/it/heavy.rs:347` matches `HistoryUpdate` without the `FoldedHits` variant (`history/state.rs:244`). Pre-existing heavy-tier break, not this ticket's; not fixed. Conductor: KNOWN_GAPS row (broken).

**Remaining:** none in scope. His Ghostty run (`docs/perf/ease-pixel-ghostty.txt`) is his to do.
