# 46-scroll-strips-wiring — the app draws backdrop + emphasis; a `j` never re-sends a strip; deflate on when probed

Status: partial — all wired and fast-tier green; release gate red (see NOTES). **Planner ruling 2026-10-01 at the end:
the gate box moves to `scroll-strips-fold-churn`; deflate off → known gap + `scroll-kitty-without-deflate`.**
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
- [x] **Fast-tier guard:** `team::TeamSpec::SMALL`, Pixel, 120×40, 20 `j` within the strips on screen → **0**
      backdrop transmits (transmit with `EMPHASIS_ID_BIT` clear), ≥ 1 emphasis transmit; deflate off and on.
      Red at the base. — `pixel_scroll::j_inside_resident_strips_sends_no_strip` (at
      `event_loop::tests::pixel_scroll`; fixture `fan(6, 8)`, not SMALL — NOTES 3). Red at base **not run**.
- [x] `Backdrop::lowest`: empty → `Full`; all `None`+`Normal` → `Full`; one `Some(∅)` → `OffThread`; one
      `None`+`Dimmed`, with or without threaded rows → `Faded`; `Some(_)`+`Dimmed` only → `OffThread`.
      — `emphasis_layer::lowest_backdrop_table`.
- [x] Ruling-2 equivalence case added and green (Decided 9). — `emphasis_layer::composite_matches_the_single_pass`.
- [x] **Stash-only rows keep their dimming** (`strips.rs:369-371`): stash-only row on screen, cursor wheeled off →
      `PixelRows.backdrop == Faded`; same window without it → `Full`.
      — `pixel_scroll::stash_only_rows_stay_dimmed_without_a_thread`; and
      `view::strips::tests::a_stash_only_commit_is_dimmed_in_its_strip` (`strips/tests.rs:265`) green, ported to `lit()`
      (there a thread is in play, so its backdrop is `OffThread`, asserted).
- [x] Focus `F` on, `j` ×5, `F` off: a backdrop transmit only on a frame whose `PixelRows.backdrop` differs from the
      previous one; test prints the backdrop per frame; 0 backdrop transmits on the `j` frames.
      — `pixel_scroll::focus_changes_the_backdrop_only_when_lowest_changes`.
- [x] Wheel moves the cursor off screen (no dimmed row on screen) → `Full`, no emphasis placed; back → `OffThread`.
      — `pixel_scroll::cursor_off_screen_draws_full` (cursor's row out of the window; NOTES 5).
- [x] iTerm2: one band per strip, composited; `j` re-encodes only bands whose selection changed.
      — `pixel_scroll::iterm2_bands_are_composites`.
- [x] **Deflate follows the probe, in the app:** `kitty_deflate` true → every transmit `deflated`; false → none;
      an error reply to a deflated transmit → later transmits raw (45 Decided 4, through `pump`, not `KittyImages`
      alone). — `pixel_scroll::deflate_follows_the_probe`, `pixel_scroll::a_deflate_error_falls_back_in_the_app`.
- [ ] **Gate green at Pixel, release, both encodings:** `held_key::{held_j_stops_on_release_pixel,
      j_then_journal_opens_at_once_pixel, held_j_stops_on_release_pixel_deflate,
      j_then_journal_opens_at_once_pixel_deflate}` via `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` under `# rev strips`; the `_unicode` pair still green. Any line not `ok` →
      quote it; never loosen a budget. Red only in deflate-off or `_pixel_iterm2` → report the line and stop (the
      follow-up is cropping the emphasis to its row span — a new ticket, 44 ruling item 3).
      **Moved** (planner ruling 2026-10-01, below): deflate on + iTerm2 + Unicode → `scroll-strips-fold-churn`;
      deflate off → `scroll-kitty-without-deflate` (known gap). Not ticked here.
- [x] Cost lines recorded, not asserted beyond the gate: deflate on, deflate off, iTerm2 — each `bytes_per_key`,
      `tail_ms`, emphasis width p99 in cells (from `KittyTransmit::width_px` ÷ cell width; iTerm2 from the band).
      — `held_key::held_j_stops_on_release_pixel_iterm2` + the two Kitty runs; script summary lines quoted.
      iTerm2's line prints `emphasis_cells_p99=none`: its band is the whole composited strip, no separate width.
- [ ] Existing `frames::` Pixel scroll tests unchanged and green in the fast tier. — **not run**: `frames` is
      `heavy-tests` only (`tests/it/main.rs`); fast tier (`cargo nextest run`, 5113 passed) does not include it.
- [x] MAP.md: graft `view::strips` (`with_layers`, `PixelRows::backdrop`; drop "`RasterRow::stubs`" from `:70`),
      `pump` (`:913`) says both layers; graft-graph `Backdrop` line (`:133`) gains `lowest`. — no test; files diffed.

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

## NOTES (builder, 2026-10-01) — partial

No compaction. Scout used (`.scratch/m7-power-tools/scouts/46.md`).

**Gate, release, `scripts/fix-scroll-held-key.sh`, `docs/perf/fix-scroll-held-key.txt` `# rev strips`:**
- Before (rev coalesce): Pixel A tail 1568 ms; B tail 718, j 665, 22.8 MB/frame.
- Deflate on: A tail 27 ms, B tail 33, j 0; 158 KB/frame held. Times green. **`strip_transmits_held` 106 / 109 > 8: FAILED.**
- Deflate off: A tail 2775 ms, B tail 1483, j 1436. FAILED.
- iTerm2 (A): tail 17 ms, 133 KB/frame, all verdicts ok.
- Unicode A/B still green (73 / 77 ms).
- Cost lines: bytes_per_key raw 390 605, deflate 128 059, iTerm2 127 299; emphasis p99 71 cells (Kitty both).

1. **Cause of the deflate strip count — the fold follows the cursor.** `crates/graft/src/view/strips.rs:348`
   `plan()` calls `keep_for(&drawn, cursor, &reserved, thread.ancestry)`, and `crates/graft/src/view/rows.rs:933-937`
   puts the cursor's lane (`selected`) and the thread's columns (`preferred`) in the fold's `Keep`. MONOREPO has more
   lanes than the cap, so which lanes fold changes as `j` moves: a different `PlainRow` layout, a different
   `plain_layout_hash`, a backdrop miss, every strip re-sent. Observed (one diagnostic release run of
   `held_j_stops_on_release_pixel_deflate`, instrumentation removed): 110 backdrop misses, backdrop `OffThread` on all
   of them, `lane_columns` ranging 25–36 with a marker. Same disease as ADR-0053 (a per-`j` value in the key), one level
   up: in the fold, not the paint. Not fixed here — the fold keeping the selection's lanes is shared with the text tiers
   on purpose (`strips.rs` `plan` doc: "a lane never folds differently between tiers"). Needs a ruling: e.g. plan the
   pixel fold without `selected`/`preferred`, or hold a fold per strip set. The fast-tier guard passes only because
   `fan(6, 8)` stays under the cap; a guard over a capped fixture would be red today.
2. **Deflate off cannot meet 100 ms at 40 MB/s** with this drain model: a Kitty layer over the graph column is
   72 cells × 17 px × 1184 px × 4 B ≈ 5.8 MB raw, 7.7 MB base64, per strip; the first frame alone (≥ 2 strips × 2
   layers) is ~1 s of drain, and each `j` re-sends ~2 emphasis images. Spec says stop here: the follow-up is cropping
   the emphasis to its row span (44 ruling item 3) — and the fold churn above multiplies it. Ghostty answers the
   `o=z` probe, so his path is the deflate row; whether his Ghostty build does is **unverified** by me.
3. **Test placement deviates:** the `pixel_scroll::*` tests are unit tests at
   `crates/graft/src/event_loop/tests/pixel_scroll.rs`, not `tests/it/pixel_scroll.rs`: they need `Graphics`,
   `draw`, `pump_with` with scripted replies (the deflate fallback) and `PixelRows`' internals, none public; and the
   team fixture needs the heavy-only `frames::run` harness. Fixture is in-memory `fan(6, 8)` (now `pub(crate)` in
   `view/strips/tests.rs`). No `tests/it/main.rs` line added.
4. **Signature deviations:** `StripRows::with_layers(&self, frame: &PixelRows, paint: &StripPaint, draw)` — the
   spec's `(backdrop, f)` cannot build a `PlainStrip` (needs lane columns, marker, cell size, palette); backdrop comes
   from `frame.backdrop`. `Graphics::new(replies, &Capabilities)` — `Graphics` had no capabilities to read
   `kitty_deflate` from; callers `startup.rs:163`, `running.rs` `pump`/`pump_keeping`, `event_loop/tests.rs`.
   `Emphasised::frame` is now `Arc<RasterFrame>` (Kitty decides re-transmit by `Arc::ptr_eq`; a fresh `Arc` per frame
   would re-send every emphasis). `Backdrop` derives `Default` (`Full`).
5. **Question:** "cursor wheeled off screen → `Full`" holds only when the cursor's row has left the window. With the
   row still on hand, `view/entries.rs:40-46` `thread()` builds the thread from `history.selected()` whether or not it
   is on screen, so the backdrop stays `OffThread` and the emphasis is still drawn. Intended?
6. Re-detection: `running.rs:163` `Wake::Terminal` only applies `CellSize`/`Theme` (`session.rs:182` `apply`); the
   capabilities, `kitty_deflate` included, never change after settle, and the same `Graphics` lives the whole
   session. A flip cannot reach `KittyImages`; none can happen today.
7. Theme / cell-size flush: `StripCache::flush` drops every layer. Kitty uploads (both layers) are not freed at the
   flush: each slot's old image is freed when its new pixels go (`ensure_uploaded`), unplaced ones by LRU past
   capacity (32). Unchanged behaviour, now for both layers.
8. Equivalence (Decided 9) green: `composite_matches_the_single_pass` — "192 cases drawn … 211 crossing pixels of
   3724500 painted (5 per 100 000), 0 elements below the backdrop"; each "one dimmed row" case clear-differ 0,
   edge-over 0.
9. Not verified: red-at-base for the fast-tier guard; heavy `frames::` tests; colour (rule 5); exit-free of an
   emphasis id specifically (`at_the_pixel_tier_strips_are_sent_once_and_freed_on_exit` checks every transmitted id,
   which matches both layers' header, but does not assert an emphasis was among them).

## Planner ruling 2026-10-01 (46 return)

Answers NOTES 1–2. 46 closes on its ticked boxes when the conductor merges 92c12351; the gate box moves (above).
NOTES 4's signature deviations are accepted: `(backdrop, f)` could not build a `PlainStrip`, and
`Graphics::new(replies, &Capabilities)` is where the probe's value enters, as Decided 6 required. NOTES 5 is not ruled
here: it stays a question, routed separately.

### (a) `strip_transmits_held ≤ ceil(rows_scrolled/32) + 2`: wrong premise on a capped graph. Restate it; leave the fold as it is.
- **NOTES 1 is a claim with its lines, not yet a premise.** It cites `strips.rs:348` → `rows.rs:933-937`, but it was
  seen in one instrumented run and has no test. Rule 7: nothing is built on it until `scroll-strips-fold-churn` Step 0
  confirms it with a test. **If Step 0 does not confirm it, the bound stands as written** and the 106 goes to
  `debugger`.
- **If it is confirmed, the fold following the cursor is designed behaviour.** ADR-0025 Decision 2 (user-accepted
  2026-09-13) ranks the selected commit's column (class 3) and its thread columns (class 5) above other lanes. So on a
  graph with more lanes than the cap, which lanes fold changes as `j` moves the thread. 42's bound assumed a strip's
  layout never depends on the selection. ADR-0053 promised less: only that the selection's **paint** never does. On a
  capped graph the bound guarded an invariant the design never gave.
  - Probably class 5 (`preferred`) alone: while the cursor is on screen its column holds a node, so class 2 already
    keeps it. Unverified; Step 0 prints which class changed.
- **Restated invariant:** a backdrop re-sends only on a frame where the fold plan or the strip window changed, never
  because only the paint changed (ADR-0053).
  - Asserted in the fast tier on a capped fixture, which runs every build; the heavy gate runs only at reviews.
  - In `held_key`, fold-aware where the harness can read the fold. Spec: `scroll-strips-fold-churn`.
- **The user-facing budgets do not move:** `tail_ms` and `j_ms` ≤ 100, `bytes_per_frame_held` ≤ 512 KiB, and the
  frame counts. With deflate on, the builder measured them green with the churn (27 / 33 / 0 ms, 158 KB/frame). The
  remainder re-measures them.
- **Rejected:**
  1. Plan the pixel fold without `selected`/`preferred`. The tiers would then fold one graph two ways (`plan` doc;
     m5/21's lesson, `docs/log/m5.md:269`).
  2. Drop class 5 from the fold in both tiers. **Visible change.** While he scrolls a graph wider than the cap, his
     selection's ancestry lanes would stay folded behind `+N` (drawn as stubs, ADR-0025 item 4) instead of opening
     out, and lanes would stop reshuffling under the cursor. It reverses part of an ADR he accepted, so it needs his
     OK, and nothing he has reported asks for it. Reopen only if he reports lanes jumping while he scrolls.
  3. Hold one fold per strip set. The same visible change, applied unevenly: the thread opens out only when the set
     changes.
  4. Debounce the fold while keys arrive. That is a timer standing in for a cause (rule 2).

### (b) Deflate off: over budget by construction. Known gap, its own ticket, no Unicode fallback.
- **Who it hits:**
  - Kitty-protocol terminals that don't take `o=z`.
  - Any session after a deflate error reply (45 Decided 4).
  - **Not his terminal:** Ghostty parses and inflates `o=z` (45 NOTES Step 0, Ghostty `d542774`
    `graphics_command.zig:620-621`, `graphics_image.zig:629-668`). Whether his Ghostty build answers the probe yes is
    **unverified**; his first Ghostty run after the merge settles it (field note the same day).
  - Zed, his daily terminal, runs the Unicode tier and is unaffected. iTerm2 (PNG) is green at 17 ms.
- **Why no wiring fix meets it:**
  - One raw layer is about 5.8 MB, or 7.7 MB base64. That is about 190 ms at the 40 MB/s model (NOTES 2's
    arithmetic, unverified).
  - A `j` that moves the thread sends at least one emphasis, so deflate off misses 100 ms even with no fold churn.
  - Meeting it needs a different emphasis shape. That is design work, not wiring.
- **Unicode fallback rejected:**
  - The tier follows what the terminal can draw, not a guess about speed.
  - A fallback hides the cost instead of fixing it.
  - It would take pixel graphs from those users on small repos, where raw is fine.
- **Ruling:**
  - The conductor adds a KNOWN_GAPS row marked **broken**: A tail 2775 ms; B tail/j 1483/1436 ms; 40 MB/s model;
    MONOREPO 187×50.
  - Ticket `scroll-kitty-without-deflate` stays parked: research first, design after.
  - The deflate-off `_pixel` `held_key` tests keep asserting and stay red until that ticket lands, **unless the user
    OKs printing their verdict instead of asserting it.** That counts as disabling a check, so rule 2 needs his OK.
  - Until he answers, 47's "gate green" box cannot tick for deflate off.

### Touches the conductor carries
- 47:
  - `:37-39` names the deflate-off `_pixel` tests as must-be-green.
  - `:59` quotes the old bound.
  - `:30-32` runs the pty walk with `PROFILE=kitty`. Whether that profile answers the `o=z` probe decides which path it
    measures.
  - 47 needs a spec-only amendment after the user answers.
- TESTING.md "Held keys" paragraph (42) quotes the old bound. `scroll-strips-fold-churn` rewrites it.
- KNOWN_GAPS: the row in (b).
