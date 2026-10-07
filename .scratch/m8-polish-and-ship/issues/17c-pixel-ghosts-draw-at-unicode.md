# G-look polish 3 — at Pixel, an open Ghost Preview draws the graph column at Unicode

Status: done (6/7; contact sheet half of the capture box open — none exists. Builder 2026-10-07.) Was: ready
(planner spec-only pass 2026-10-07; split from 17b item 6 against the scout
`.scratch/m8-polish-and-ship/scouts/17b-fold-count-and-pixel-ghosts.md`; decision **approved** by the user
2026-10-07, field notes :92 item 1, ADR-0060)
Build with: **Opus** — a tier rule on the draw path that `event_loop::graphics` must obey; ~15 callers classified by
reading, each class held by a test. Decided in ADR-0060; nothing structural left open. Not Fable.
Blocked by: **17b** (shares `view/rows.rs` and MAP.md graft + graft-ui). **09g** (holds `update/mod.rs`; this may
touch `strip_rows_for` at update/mod.rs:774 — see Decision 4). Not blocked by 12 (no palette file; `overlays.rs`
callers stay as they are — Decision 3).
Needs: `view::tier_in_force` (view/frame.rs:84); `draws_pixels` (frame.rs:93–100); placements
(`event_loop/graphics.rs:153`); preview state (`view/preview.rs`).
**Crate set:** graft only (`view/frame.rs`, `view/rows.rs`, `view/layout.rs` if Decision 3 moves it, the mouse
hit-test file, `event_loop/graphics.rs`, possibly `update/mod.rs:774`, MAP.md). No graft-ui, graft-git,
graft-graph change.

## Read first
- `.scratch/m8-polish-and-ship/scouts/17b-fold-count-and-pixel-ghosts.md` (the **Tier** paragraph)
- `docs/adr/0060-at-pixel-an-open-preview-draws-the-graph-at-unicode.md` (Decision)
- `docs/design/01-view-mockups.md` §1.7 (:286–306, the ADR-0060 paragraph included); `docs/design/03-graph-layout-
  algorithm.md` §3.8 (:371–423, strips; alpha in the key)
- `crates/graft/MAP.md` `view::strips` (:70–73), `AppState::tier` (:253), Ghost Preview `update::preview` (:545–573)

## The contradiction this fixes
ADR-0060 and 17b named `AppState::graph_tier()` over `tier()` (update/state.rs:502). The scout found the graph does
not draw by `tier()`: it draws by `view::tier_in_force(session, state)` (frame.rs:84: `renderer_override`, else
`session.rendering.tier`). `tier()` reads `settled_tier` and is read by `strip_rows_for` (update/mod.rs:774), F3
(keys/actions.rs:13, :21), bars.rs:365 and trace.rs:502. Two tier sources; the graph follows `tier_in_force`. The
rule therefore sits **beside `tier_in_force`**, not on `AppState`. ADR-0060's substance is unchanged (whole graph
column at Unicode during a preview; `gfx` and F3 unchanged); its decision 1 names the wrong function — **NOTES asks
the conductor for a wording correction to ADR-0060 decision 1** (`view::graph_tier`, not `AppState::graph_tier`).
Unifying `settled_tier` and `session.rendering.tier` is **out**: whether their disagreement is intended is unverified
(rule 7 — a question, not a defect); NOTES records the question with both file:lines.

## Decisions (planner)
1. **One function, the graph column's tier:** `view::graph_tier(session, state)` = Unicode when a preview with ghost
   rows is open and `tier_in_force` is Pixel; else `tier_in_force`. F3 during the preview falls out:
   `min(tier_in_force, Unicode)`.
2. **Every graph-column draw, width, placement and hit-test path reads `graph_tier`:** `draws_pixels`
   (frame.rs:93–100), the graph-row path in `view/rows.rs`, `event_loop/graphics.rs:153` (no placement while the
   preview holds the column at Unicode), the mouse hit-test (which tier it reads is **unconfirmed** — builder's first
   step, file:line in NOTES), and any of the ~15 `tier_in_force` callers that sizes or draws the graph column.
3. **Chrome stays on `tier_in_force`.** Callers drawing boxes, bars, conflicts and overlays (boxes.rs:51, :453;
   bars.rs:74; conflicts.rs:53, :162; overlays.rs:27, :132, :196, :228, :268, :324) keep reading it — the tier in
   force did not change. frame.rs:179 and layout.rs:251: builder reads each and classifies it (graph column → move;
   chrome → stay). NOTES lists all ~15 with file:line and moved/stayed + why. `gfx` (bars.rs:365) and F3 keep
   `tier()`.
4. **No strip rastered for the preview, none evicted by it** (ADR-0060 3). `strip_rows_for` (update/mod.rs:774)
   reads `tier()`, which stays Pixel during the preview — if it requests strips for rows scrolled into view during the
   preview, it must request none while a preview with ghost rows is open (one condition there). Builder confirms by
   test, not by reading alone.
5. **Clearing placements on open:** reuse whatever clears Kitty/iTerm2 placements when F3 leaves Pixel (`graphics.rs`
   or `row_clear.rs` — unconfirmed; builder finds it, file:line in NOTES). If F3 leaving Pixel leaves placements on
   screen today, that is a pre-existing defect: **stop**, report it with its line; do not patch around it here.
6. On close, strips re-place from the cache; zero new rasters while the budget kept them.

## Scope
In: Decisions 1–6; the KNOWN_GAPS row ask `M8/pixel-ghosts-text` (NOTES, for the conductor).
Out: rasterised ghosts (post-v1, ADR-0060 4); unifying the two tier sources; the rewrite editor's right side
(report only — does it draw strips at Pixel? file:line in NOTES).

## Signatures
```rust
// graft::view (pub(crate)) — beside tier_in_force (view/frame.rs:84); same parameter and return types as it
pub(crate) fn graph_tier(session: &Session, state: &AppState) -> Tier;
```
(`Session`, `Tier` = exactly what `tier_in_force` takes and returns; no new type.)

## Acceptance (each tick names its test, at tick time)
- [x] 1: Pixel, preview with ghost rows open → `graph_tier` = Unicode; no preview → `tier_in_force`; F3 to ASCII
      during the preview → ASCII. (`view::tests::graph_tier_is_unicode_only_under_a_pixel_preview` — file
      `view/tests/pixel_preview.rs`; seen failing on `left: Pixel right: Unicode` against a stub, then green)
- [x] 2: Pixel fixture, preview open → frame bytes hold no image placement for the graph; text layer holds ghost
      glyphs (`○` — see NOTES: this fixture's ghost has no straight run, so no `┊`); close → strips placed again,
      zero new rasters. (`event_loop::tests::pixel_scroll::a_preview_at_pixel_draws_the_unicode_graph_and_places_no_strip`,
      `…::closing_the_preview_replaces_the_strips` — written after the code; seen failing by mutation: the first on
      "no strip placed" with `draws_pixels` forced true, the second on "the same strips, by id, from the cache"
      with `graphics.rs:156` switched to `draws_pixels` (free instead of hide); both restored)
- [x] 2: mouse hit-test and draw agree on the graph column width during the preview (click a ghost-free real row,
      assert the row hit). (`view::tests::preview_hit_test_uses_the_graph_tier` — seen failing on "no pixel
      rectangle" `left: Some(Rect{..})` before the code)
- [x] 3: header `gfx` field reads the same before and during the preview. (`view::tests::gfx_field_holds_during_a_preview`
      — passed against the stub (nothing had changed yet); seen failing by mutation once `draws_pixels` moved,
      with `screen.graphics` left on `pixels` (frame.rs:224), then green on `pixel_tier_on_graph`)
- [x] 4: scroll during a Pixel preview requests no strip (command list asserted). (`update::tests::pixel_preview::
      a_pixel_preview_requests_no_strips` — PageDown, `j`, wheel: no `ShowRows`, `draws_pixels` false after each;
      seen failing by mutation on "raster paths are gated off" with `draws_pixels` forced true. See NOTES Decision 4.)
- [ ] Capture: `docs/shots/m8/polish-ghost-pixel-120x36.txt` (text layer) — **written**
      (`view::tests::pixel_preview::capture_a_pixel_preview_for_docs_shots`, 10 005 B, two frames: closed then open).
      **Not ticked: m8 contact sheet** — none exists (same as 17b). **Colour unverified** (rule 5). User judges by eye.
- [x] §1.7 as amended by ADR-0060 matches what was built (read `docs/design/01-view-mockups.md` :286–306; one
      difference in NOTES); MAP.md graft (`graph_tier`, which callers read it; `draws_pixels` line). (doc, checked by
      reading)

## Touches
- Each of the ~15 `tier_in_force` callers (scout list; re-grep) — moved or stayed, listed in NOTES.
- `tier()` callers (update/mod.rs:774, keys/actions.rs:13, :21, bars.rs:365, trace.rs:502) — unchanged except
  Decision 4; grep shown.
- `event_loop::graphics` placement/clear; strip cache budget (no new entries, no evictions for the preview); F3.
- 17b's fold mark on `view/rows.rs` (must still draw at Unicode during the preview — existing test stays green).
- Journal/undo: none. Keymap: none.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~30k: function + classification of ~15 callers ~10k; graphics/placement + strip request ~10k; five tests + capture
~7k; MAP/NOTES ~3k.

## NOTES (builder, 2026-10-07)
Scout used (`scouts/17b-fold-count-and-pixel-ghosts.md`, Tier paragraph); every cited line re-grepped after 17b/12c
merged — shifted only `strip_rows_for`'s caller (now update/mod.rs:797). No compaction.

**Built.** `view::frame::graph_tier` (frame.rs:98, `pub(crate)`, not re-exported: nothing outside `view` reads it).
The "preview drawn" condition is one function, `view::rows::preview_drawn` (open **and** plan landed, rows non-empty),
read by both `rows_for_frame` (which rows) and `graph_tier` (which tier) so they cannot disagree. `draws_pixels`
(frame.rs:127) = `pixel_tier_on_graph` (the old condition, tier in force; private) && `graph_tier == Pixel`.

**Callers, classified** (re-grep, `src/` non-test):
- Moved to `graph_tier`: `draws_pixels` frame.rs:127 (so frame.rs:146 `pixels` and the frame's pixel rectangle,
  and event_loop/running.rs:168 prefetch, follow it); frame.rs:210 `screen.ascii` (the column's glyph set).
- Stayed on `tier_in_force`, chrome / tier in force: frame.rs:211 footer `F3` notice (`renderer: … (F3 cycles)` —
  same text either way, since the override is only ever below the session tier); frame.rs:224 header `gfx` via
  `pixel_tier_on_graph` (ADR-0060 2); event_loop/graphics.rs:156 hide-vs-free (**must** stay: at Pixel a frame
  with no rectangle *hides* and keeps uploads, which is what lets close re-place from the cache; switching it frees
  them — the mutation `closing_the_preview_replaces_the_strips` catches); bars.rs:74 box bars; boxes.rs:51
  palette; boxes.rs:453 credential box; conflicts.rs:53 resolver; conflicts.rs:162 finish; layout.rs:251 tab-layout
  banner and hints; overlays.rs:27 journal, :132 history place, :196 blame place, :228 rewrite editor, :268 map,
  :324 detail beside an overlay — every overlay replaces the graph (`graph_overlay` Some), so no graph column is
  under them.
- `tier()` callers unchanged: update/mod.rs:797, keys/actions.rs:13, :21 (F3), bars.rs:375 (banner title), trace.rs:502.

**Hit-test** reads no tier: view/hit.rs:51–52 builds `graph_view(state, now)` with no `Session`; graft-ui
`graph_screen/layout.rs:386` `row_at` sizes the column by `graph_width`, which ignores `pixel`; the only `pixel`
read is render.rs:158 (blank the cells). So draw and hit-test agreed by construction; the test pins it.

**Decision 4.** No code. `strip_rows_for` (update/mod.rs:797, keys/actions.rs:26) only pads *row reads*
(`ShowRows`) to strip boundaries at walk start / F3; it rasters nothing. Strips are rastered only in
`event_loop::graphics::frame` (needs the frame's pixel rectangle) and prefetch (running.rs:168), both gated by
`draws_pixels`, now false under the picture. Asserted by `a_pixel_preview_requests_no_strips`.
**Question (rule 7, not a defect claim):** the wheel moves the live window *behind* a modal preview —
update/mouse.rs:85–86 → `wheel` :175 → `scroll`, no preview check in :60–:83 (the picker and context menu return
first; the preview does not). Seen in the test: `top` moved 52→55 on one notch. The picture is a snapshot so
nothing changes on screen until close, when the graph is 3 rows off where he left it. Intended?

**Decision 5.** What clears placements when F3 leaves Pixel: event_loop/graphics.rs:155–161 (`frame` with no
rectangle: `images.free_all` off Pixel, `images.hide` at Pixel; `bands.hide` both). Existing tests
`event_loop::tests::f3_off_pixels_frees_every_upload_and_names_the_tier` (tests.rs:557) cover F3; I did not re-run
them separately beyond the workspace run. The preview reuses the same path, at Pixel → hide. No pre-existing defect.

**Ghost glyph.** The ticket's `┊` box: the fixture (`view::tests::one_ghost_over`) has one ghost hanging off its
anchor with no straight run, so it draws `○` (graft-ui `graph_column/style.rs:15`), not `┊`; `┊` only replaces `│`
on a ghost edge through a row (style.rs:29). The event-loop test asserts `○` and `●` in the text stream.

**§1.7 vs built.** Matches, one difference: §1.7 says "while a preview is open"; built is "while its picture is
drawn" — between opening and the plan landing (`plan: Pending`, rows empty) the live graph is drawn and stays in
pixels (no ghost rows yet). Doc wording, if the conductor wants it exact.

**Rewrite editor** (scope "report only"): draws no strips — overlays.rs:221–257 `draw_rewrite` returns `None`
(no pixel rectangle) and draws text at `tier_in_force`.

**For the conductor.**
- ADR-0060 decision 1 already reads `view::graph_tier(session, state)` (corrected 2026-10-07) — nothing to do.
- KNOWN_GAPS row `M8/pixel-ghosts-text` (pixel-rasterised ghosts past v1, ADR-0060 4) — please file.
- Two tier sources (`AppState::tier()` reads `settled_tier`, update/state.rs:502; `tier_in_force` reads
  `renderer_override` else `session.rendering.tier`, view/frame.rs:84): whether their disagreement is intended is a
  **question**, unverified; not touched.
- Contact sheet: none exists for m8 (as 17b found) — the acceptance group's job.
- Capture `docs/shots/m8/polish-ghost-pixel-120x36.txt`: the fixture's subjects carry `␛[31m` (the sanitiser
  fixture `history_of`); harmless, but it is in the picture.
- Not verified: a real Kitty/iTerm2 terminal. Bytes asserted in memory only.
