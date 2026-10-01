# 46-scroll-strips-wiring — the app draws backdrop + emphasis; a `j` never re-sends a strip; deflate on when probed

Status: ready
Build with: Opus — switches the live pixel path to two layers, wires 45's deflate, and turns the held-key gate green
at Pixel in two encodings; scout 46 found four wrong premises, so judgement is needed, not a port.
Needs: `44-scroll-emphasis-layer` (`Backdrop`, `PlainRow<'a>`, `PlainStrip<'a>`, `Lit<'a>`, `Selection<'a>`,
`Emphasised`, `StripCache::{backdrop, emphasis, composited, contains_backdrop}`, `composite`);
`45-scroll-kitty-layer-deflate` (`Layer`, `EMPHASIS_ID_BIT`, `Place::layer`, `KittyImages::{deflating, is_deflating,
answer}`, `Capabilities::kitty_deflate`); `42-scroll-held-key-gate` (`held_key`, `DrainSink::kitty_transmits`);
`43-scroll-coalesce-input` **merged first** — it owns `event_loop/running.rs` (prefetch call `:115` at 2bee64aa),
`event_loop/mod.rs`, `frames/paced.rs` while it builds.
Blocked by: 42, 43, 44, 45
Scout: `.scratch/m7-power-tools/scouts/46.md` (HEAD 2bee64aa). Line numbers below are from it; re-locate by name.

File set:
- graft: `src/view/strips.rs`, `src/view/strips/tests.rs`, `src/event_loop/graphics.rs` (frame `:124`, prefetch
  `:195`, `KittyImages::new` `:79`, `answer` `:108`), `src/event_loop/running.rs` (the prefetch call only, after 43),
  `tests/it/pixel_scroll.rs` (new) + its `mod` line in `tests/it/main.rs`, `tests/it/held_key.rs` (layer const `:42`
  + the variants in Decided 7), `tests/it/drain_sink.rs` (`KittyTransmit` gains two header fields, Signatures),
  `crates/graft/MAP.md`.
- graft-graph: `src/raster/emphasis.rs` (`Backdrop::lowest` beside `Backdrop` `:28`; `of` `:51` **stays** — deleted
  by the follow-up), `tests/it/emphasis_layer.rs` (`lowest_backdrop_table` + the ruling-2 case only),
  `crates/graft-graph/MAP.md` (`## raster::emphasis` one line).
- `docs/perf/fix-scroll-held-key.txt` (append). `scripts/fix-scroll-held-key.sh` only if its summary selects tests by
  exact name (then add the new names; no env switch, Decided 7).
- **Not here:** deleting the old path (`RasterRow`, `Strip`, `rasterise`, `StripCache::{get, contains}`,
  `Backdrop::of`, the legacy oracle) and porting `tests/it/raster.rs`, `raster_pixels.rs`, `benches/hot_paths.rs` →
  `scroll-old-raster-deletion` (follow-up spec, Needs this one).
Overlap and order: after 43 (shared `running.rs`); `held_key.rs` and `drain_sink.rs` are free once 42 merged; nothing
else concurrent may hold `graphics.rs`, `strips.rs` or `emphasis.rs`.

## Read first
- `docs/adr/0053-selection-paint-is-its-own-layer.md` (whole).
- Scout `.scratch/m7-power-tools/scouts/46.md` (whole, 1 page).
- `42-scroll-held-key-gate.md` §Need, §Journeys, Decided 3–5.
- `44-scroll-emphasis-layer.md` §Decided 2–3, §Signatures, NOTES 1–4, "Planner ruling 2026-10-01 (44 return)".
- `45-scroll-kitty-layer-deflate.md` §Decided 1, 2, 4 and the NOTES Step 0 table.
- `crates/graft/MAP.md`: `## view::strips` (`:67-70`); `max_lanes(state)` (`:864`; scout's `:218` is the
  `AppState::max_lanes` field, not the fn); `Wake::Terminal` (`:84`); `pump` idle prefetch (`:86`); `pump` resize /
  theme flush (`:913`).
- `crates/graft-graph/MAP.md` `## raster::emphasis` (`:132-141`), `## raster::cache` (`:143-161`).
- `crates/graft-term/MAP.md` `## kitty`, `## iterm2`; `crates/graft-core/MAP.md` `Capabilities`.

## Scope
In: `pixel_rows` computes the frame's `Backdrop` from its rows; `event_loop::graphics` places backdrop + emphasis on
Kitty and composites for iTerm2; idle prefetch uses `contains_backdrop`/`backdrop`; Kitty image capacity doubled;
`KittyImages` deflates when the probe said yes; the held-key gate green at Pixel with deflate on and off.
Out: deleting the old raster path (follow-up spec); Unicode/ASCII tiers; coalescing (43); Kitty encoding (45).

## Decided
1. **Backdrop per frame = `Backdrop::lowest` over every row of every strip in the frame** (44 ruling item 2). One
   backdrop per frame. It changes when the thread comes or goes (cursor on/off screen, 37), when Focus dims rows with
   no thread, and when a stash-only row (m7/16) enters or leaves a frame with no thread. Nothing in graft calls
   `Backdrop::of` (scout); it is not introduced.
2. **Data shape: `StripRows` keeps owning its rows; the borrowed views are built per call.** No `PixelStrip`, no
   lifetime on `PixelRows`. `StripRows::with_layers` (Signatures) replaces `with_strip` (`strips.rs:125-144`): it
   builds a `Vec<PlainRow<'_>>` and a `Vec<Lit<'_>>` from `self.rows`, wraps them in `PlainStrip` / `Selection`, and
   hands both to the closure. Why: a struct holding owned rows plus `PlainRow<'_>`s into them is self-referential;
   an index into a frame-wide row vector would change the fold's ownership in `pixel_rows` for nothing — strips are
   already contiguous and owned; a self-referencing crate is a new dependency. The closure is today's pattern, so the
   callers (`graphics.rs:141,157,201,206,214`, `strips/tests.rs`) change shape, not structure. Cost: two `Vec`s of
   ≤ strip-height pointers per strip per frame, against a hash and a cache lookup — not measurable; say so with the
   fast-tier number if you have one.
3. **`PaintedRow` (`:89-92`) keeps its fields** (`row: Row`, `ring`, `emphasis`, `thread: Option<BTreeSet>`, the
   stubs): its selection paint becomes `Lit { emphasis, thread: self.thread.as_ref() }`, its layout `PlainRow`.
   `StripRows.above` stays `Option<Row>` (owned); `PlainStrip.above` is `self.above.as_ref()`.
4. **Waiting rule unchanged:** while the cursor is in the viewport and its row is missing, all strips wait (30b).
5. **Kitty:** per strip, `Place { layer: Layer::Strip, .. }` for the backdrop, then `Place { layer: Layer::Emphasis,
   .. }` at `first_column` when `Some`. **iTerm2:** `composited` only, one band per strip, as today. Capacity:
   `KittyImages::new(2 * STRIPS_KEPT)` (two images per strip); `ItermImages` and `StripCache` keep `STRIPS_KEPT`
   (`graphics.rs:31,80,81`; the cache already has an LRU per layer).
6. **Deflate is wired here.** At the `KittyImages::new` site (`graphics.rs:79`), `.deflating(caps.kitty_deflate)`
   from the `Capabilities` the session holds — the value the tier was settled from, never a literal. 45 left it
   unwired, so until this lands its deflate does nothing in the app. The runtime fallback stays: every image reply
   still reaches `KittyImages::answer` (`graphics.rs:108`).
7. **Gate in both encodings, by parameter, not env** (42 Decided 3: env is process-global and races
   `--test-threads=2`). `held_key` takes `run::capabilities(Tier::Pixel)` and sets the field on the value
   (`caps.kitty_deflate = true;` — still no `Capabilities {…}` literal, 42 Decided 4). The existing `_pixel` tests run
   deflate **off**; new `_pixel_deflate` twins run it **on**; one `_pixel_iterm2` run sets the graphics protocol to
   iTerm2 the same way. 44 ruling item 3's "env switch" in the script is withdrawn.
8. **Layer count in tests** by image id: `EMPHASIS_ID_BIT` set = emphasis. `held_key.rs`'s `EMPHASIS_ID_BIT_42`
   (`:42`) is replaced by `graft_term::kitty::EMPHASIS_ID_BIT`.
9. **Equivalence case before the follow-up deletes the oracle:** add to `emphasis_layer::
   composite_matches_the_single_pass` (`emphasis_layer.rs:363`, oracle `legacy` `:158`) the ruling-2 case: no
   thread, one `Dimmed` row, backdrop `lowest` (= `Faded`), `a < b` count 0, same pixel classes. Quote its green run
   in the report; the follow-up quotes it again before deleting.

## Signatures
```rust
// crates/graft-graph/src/raster/emphasis.rs — beside `Backdrop` (:28); `of` (:51) untouched here
impl Backdrop {
    /// The lowest alpha any of these rows draws at (ADR-0053 Decision 2): `Faded` if some row has
    /// `thread: None` and `Emphasis::Dimmed`; else `OffThread` if some row has `thread: Some(_)` (∅ included);
    /// else `Full`. Derived from the paint, never from the features that produce it.
    pub fn lowest<'a>(lits: impl IntoIterator<Item = Lit<'a>>) -> Backdrop;
}

// crates/graft/src/view/strips.rs — no lifetime parameter anywhere, as today
pub struct PixelRows {
    pub strips: Vec<StripRows>,                      // unchanged type
    /* lane_columns, marker, uncommitted_on_top: unchanged */
    pub backdrop: graft_graph::raster::Backdrop,     // Backdrop::lowest over every PaintedRow of every strip
}
impl PaintedRow {
    fn plain(&self) -> graft_graph::raster::PlainRow<'_>;
    fn lit(&self) -> graft_graph::raster::Lit<'_>;
}
impl StripRows {
    /// Replaces `with_strip`; same visibility. Builds the views over `self.rows` and calls `f` once.
    fn with_layers<R>(
        &self,
        backdrop: graft_graph::raster::Backdrop,
        f: impl FnOnce(&graft_graph::raster::PlainStrip<'_>, &graft_graph::raster::Selection<'_>) -> R,
    ) -> R;
}
pub fn pixel_rows(/* unchanged */) -> PixelRows;

// crates/graft/tests/it/drain_sink.rs — header fields, parsed like the id (header bytes only)
pub(crate) struct KittyTransmit { /* existing: offset, id */ pub width_px: Option<u32> /* s= */, pub deflated: bool /* o=z */ }
```
`event_loop::graphics`'s private functions change freely; no new public item there.

## Acceptance (each box names its test; red first)
- [ ] **Fast-tier guard:** `team::TeamSpec::SMALL`, Pixel, 120×40, 20 `j` within the strips on screen → **0**
      backdrop transmits (transmit with `EMPHASIS_ID_BIT` clear), ≥ 1 emphasis transmit; deflate off and on.
      Red at the base. — `pixel_scroll::j_inside_resident_strips_sends_no_strip`.
- [ ] `Backdrop::lowest`: empty → `Full`; all `None`+`Normal` → `Full`; one `Some(∅)` → `OffThread`; one
      `None`+`Dimmed`, with or without threaded rows → `Faded`; `Some(_)`+`Dimmed` only → `OffThread`.
      — `emphasis_layer::lowest_backdrop_table`.
- [ ] Ruling-2 equivalence case added and green (Decided 9). — `emphasis_layer::composite_matches_the_single_pass`.
- [ ] **Stash-only rows keep their dimming** (`strips.rs:369-371`): stash-only row on screen, cursor wheeled off →
      `PixelRows.backdrop == Faded`; same window without it → `Full`.
      — `pixel_scroll::stash_only_rows_stay_dimmed_without_a_thread`; and
      `view::strips::tests::a_stash_only_commit_is_dimmed_in_its_strip` (`strips/tests.rs:265`) green, ported to `lit()`.
- [ ] Focus `F` on, `j` ×5, `F` off: a backdrop transmit only on a frame whose `PixelRows.backdrop` differs from the
      previous one; test prints the backdrop per frame; 0 backdrop transmits on the `j` frames.
      — `pixel_scroll::focus_changes_the_backdrop_only_when_lowest_changes`.
- [ ] Wheel moves the cursor off screen (no dimmed row on screen) → `Full`, no emphasis placed; back → `OffThread`.
      — `pixel_scroll::cursor_off_screen_draws_full`.
- [ ] iTerm2: one band per strip, composited; `j` re-encodes only bands whose selection changed.
      — `pixel_scroll::iterm2_bands_are_composites`.
- [ ] **Deflate follows the probe, in the app:** `kitty_deflate` true → every transmit `deflated`; false → none;
      an error reply to a deflated transmit → later transmits raw (45 Decided 4, through `pump`, not `KittyImages`
      alone). — `pixel_scroll::deflate_follows_the_probe`, `pixel_scroll::a_deflate_error_falls_back_in_the_app`.
- [ ] **Gate green at Pixel, release, both encodings:** `held_key::{held_j_stops_on_release_pixel,
      j_then_journal_opens_at_once_pixel, held_j_stops_on_release_pixel_deflate,
      j_then_journal_opens_at_once_pixel_deflate}` via `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` under `# rev strips`; the `_unicode` pair still green. Any line not `ok` →
      quote it; never loosen a budget. Red only in deflate-off or `_pixel_iterm2` → report the line and stop (the
      follow-up is cropping the emphasis to its row span — a new ticket, 44 ruling item 3).
- [ ] Cost lines recorded, not asserted beyond the gate: deflate on, deflate off, iTerm2 — each `bytes_per_key`,
      `tail_ms`, emphasis width p99 in cells (from `KittyTransmit::width_px` ÷ cell width; iTerm2 from the band).
      — `held_key::held_j_stops_on_release_pixel_iterm2` + the two Kitty runs; script summary lines quoted.
- [ ] Existing `frames::` Pixel scroll tests unchanged and green in the fast tier.
- [ ] MAP.md: graft `view::strips` (`with_layers`, `PixelRows::backdrop`; drop "`RasterRow::stubs`" from `:70`),
      `pump` (`:913`) says both layers; graft-graph `Backdrop` line (`:133`) gains `lowest`.

## Touches (report each on the **Touches:** line)
- Callers of `pixel_rows` / `with_strip`: `graphics.rs:141,157,201,206,214`, `strips/tests.rs` (scout grep; re-run).
- Hit-test (`max_lanes`, MAP `:864`; `screen_area`) reads columns, not pixels — confirm unchanged.
- Resize (MAP `:913`: "flushes nothing and retransmits nothing") for both layers: name the test or add one.
- Theme / cell-size flush: every LRU flushed, the `KittyImages` emphasis images freed too.
- Re-detection (`Wake::Terminal`, MAP `:84`): if `pump` keeps the same `KittyImages` across a re-detect, a probe
  answer that flips `kitty_deflate` does not reach it. Say what the code does, with the line; if it's kept, note it
  as a question in NOTES. No setter on `KittyImages` (graft-term not in this file set).
- Exit: every upload freed, both layers (MAP `:913`, 24c).
- Uncommitted row on `HEAD`: ring in `PlainRow`, selection paint in `Lit`. Fold stubs (39): in `PlainRow`.
- Pixel captures in `docs/shots/` (`tests/it/captures.rs`, `gallery.rs`, `frames/{sink,run,readings,journal_find}.rs`):
  scout could not confirm which assert pixel bytes; run them and name any that changed.
- KNOWN_GAPS "Focus and the ancestry highlight are bold/dim only": unchanged.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`. Release runs via the script
only.

## Size
~65k. Split point if it compacts: `lowest` + `with_layers` + switch + fast-tier tests first; deflate wiring + gate
variants + cost lines second.
