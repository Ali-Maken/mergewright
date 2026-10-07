# G-look polish 3 — at Pixel, an open Ghost Preview draws the graph column at Unicode

Status: ready (planner spec-only pass 2026-10-07; split from 17b item 6 against the scout
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
- [ ] 1: Pixel, preview with ghost rows open → `graph_tier` = Unicode; no preview → `tier_in_force`; F3 to ASCII
      during the preview → ASCII. (`view::tests::graph_tier_is_unicode_only_under_a_pixel_preview`)
- [ ] 2: Pixel fixture, preview open → frame bytes hold no image placement for the graph; text layer holds `┊`
      ghost glyphs; close → strips placed again, zero new rasters. (`event_loop::tests::pixel_scroll::
      a_preview_at_pixel_draws_the_unicode_graph_and_places_no_strip`, `…::closing_the_preview_replaces_the_strips`)
- [ ] 2: mouse hit-test and draw agree on the graph column width during the preview (click a ghost-free real row,
      assert the row hit). (`view::tests::preview_hit_test_uses_the_graph_tier`)
- [ ] 3: header `gfx` field reads the same before and during the preview. (`view::tests::gfx_field_holds_during_a_preview`)
- [ ] 4: scroll during a Pixel preview requests no strip (command list asserted). (`update::tests::
      a_pixel_preview_requests_no_strips`)
- [ ] Capture: `docs/shots/m8/polish-ghost-pixel-120x36.txt` (text layer); m8 contact sheet re-assembled. **Colour
      unverified** (rule 5).
- [ ] §1.7 as amended by ADR-0060 matches what was built (read, NOTES says so or names the difference); MAP.md graft
      (`graph_tier`, which callers read it). (doc, checked by reading)

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
