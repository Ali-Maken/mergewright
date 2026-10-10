# G-trace follow-up — remove the per-frame cost that grows with history or refs

Status: **Amendment 1 build B partial 2026-10-11 — built; after-measurement draw_us p50/p99 missed in run 1 of 2 (NOTES B).** Was: Part 2 partial 2026-10-10. Part 1: finding made, ruled below. Was: **deferred by the user 2026-10-08 ("lets finish the product, and then deal with this") — runs after the rest of M8 is built, before the milestone stop.** His trace run exists (2026-10-08, docs/perf/m8-trace-2026-10-08-{ghostty,zed}.txt; was held until it did) (`docs/perf/m8-trace-<date>.txt` from his machine, not the
builder's). No run → this ticket does not start (rule 1: green fixtures did not match his screen, :84).
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: diagnosis from numbers, a fix at the cause.
Blocked by: 02, 03
Needs: `settle-work-at-rest` merged (Part B of that ticket is this one); `pace-fence-and-sync` (`fence_us` filled).

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Need (scroll row), Journeys J1–J3, "Where he lands" first row
- His summary `docs/perf/m8-trace-<date>.txt` and the builder's `docs/perf/m8-trace-monorepo.txt` — summary lines only
- `crates/graft/MAP.md`: `event_loop` (`pump`, `joins_frame`), `view` (rows, strips), `update` (cursor motion)
- `.scratch/m8-polish-and-ship/scouts/frame-loop.md`

## Scope
In:
- Read his summary: which field dominates a frame (`update_us`, `draw_us`, `bytes`, `write_us`, `fence_us`) and how
  the tail after release compares with the fixture's. State the finding with its numbers in NOTES before any change.
- Find, by test, any per-frame or per-turn work whose cost grows with history length or ref count. Known candidate
  (unverified since M2): the overview strip's search-hit set is rebuilt every frame it is shown (`view/strip.rs`).
  Fix each found at the cause: computed once per change, not per frame.
- A guard test per fix: frame work counted (calls or allocations, not time) at 1k and 100k commits / 10 and 50k refs
  is equal within the window.
Out:
- Anything his numbers don't point at. If his run shows the paced loop within budget (tail ≤ 100 ms, p99 frame
  ≤ 16 ms), close with the summary in NOTES and no code. Wheel step change (his scene 1 verdict, separate).

## Signatures
None public. Internal names are the builder's; MAP.md records them.

## Acceptance
- [x] NOTES: his numbers quoted (file:line of the summary), the dominant field named, the cause named with file:line.
      Verified by the measurement `scripts/m8-trace-followup.sh` → `docs/perf/m8-trace-followup.txt:31-32` (upload split by
      layer) and `held_key::held_j_stops_on_release_pixel_deflate_free` (release, heavy: emphasis_transmits_held=382/188 keys).
- [ ] (no fix landed — needs a ruling, NOTES) Each fix: a count-based guard test at two history sizes and two ref counts (test named at tick).
- [ ] (before line only, `docs/perf/m8-trace-followup.txt`; no after) `scripts/m8-trace-summary.sh` re-run on MONOREPO after the fix → `docs/perf/m8-trace-followup.txt`, before/after
      lines side by side. No timing assert in the fast tier.
- [ ] (nothing landed for him to re-judge) TESTING.md "How he runs the trace": one line asking him to run it again after this lands.

## Touches
- `view` per-frame builders (strip, rows, header tail), `update` motion path; the held_key and frames tests; MAP.md
  graft; KNOWN_GAPS rows 414, 1095 (fate in NOTES for the conductor).

## Scout questions
- Per frame, which builders iterate over all refs, all laid-out rows or all search hits rather than the visible
  window (file:line)? Does any run when nothing it reads changed?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens. More than two causes found → fix the larger, file the rest for the conductor.

## NOTES (builder-opus, 2026-10-10)

No compaction. No Rust changed (gates not needed for a scripts/docs-only diff; `m8-trace-summary.sh --self-test` ok).
Scout `.scratch/m8-polish-and-ship/scouts/05-trace-followup.md` used and re-checked at ad7a435f: graphics.rs:145-212
`frame`, :313/:318-322 paint/backdrop/emphasis keys, kitty/mod.rs:211-228 `frame` + :219 `put_placement`,
:278-313 `ensure_uploaded` (free :290, transmit :294), cache.rs:25/:215 backdrop key, strip.rs:24-57, summary
script tail definition — all held. Pace/rest/turn lines not re-checked (not used).

### His numbers
- Zed, Unicode (`docs/perf/m8-trace-2026-10-08-zed.txt`): within budget. draw_us p99 1060 (:8), write_us p99 177
  (:10), tail_ms 96.2 (:13). Only big field is fence_us p50 7054 p99 13743 (:11): Zed's terminal answering the fence,
  i.e. Zed's own parse/paint time; nothing Graft draws.
- Ghostty, Pixel (`docs/perf/m8-trace-2026-10-08-ghostty.txt`): over budget at the tail. draw_us p95 4020 p99 6569
  (:8), bytes p95 76751 p99 127189 (:9), write_us p95 4428 p99 14064 (:10), fence_us p99 5968 (:11). **Dominant
  field: bytes → write_us** (write p99 14 ms alone nearly fills a 16 ms frame; Unicode in Zed writes 177 µs at p99),
  then draw_us (4x Zed's at p95). Per-frame totals (update+draw+write) cannot be computed from summary lines, so the
  "p99 frame ≤ 16 ms" check is **unverified** for his run.
- tail_ms 1748.7 (:13) **cannot be judged**: the script takes the last frame of the whole 50 s trace minus the last
  frame with motions>0 (`scripts/m8-trace-summary.sh:43-46,62-63`), so one late frame (quit, rest, a job result)
  gives that number. The builder run shows 0 frames after the last key at his shape (`docs/perf/m8-trace-followup.txt:30`).
- Fixture (`docs/perf/m8-trace-monorepo.txt`) was Unicode, no fence: not comparable with Ghostty.

### Cause (measured, builder run at his shape; `docs/perf/m8-trace-followup.txt`)
Per `j`, the Pixel tier re-rasterises, deflates and re-transmits the **emphasis image** of every visible strip the
thread's lit set changed in. 50 keys → 148 emphasis uploads = **82 %** of all bytes (4.3 MB of 5.2 MB); strips 7
uploads (15 %). Half the emphasis images are near full graph width × full strip height (1173×1221 px ×41, 1173×1184
×11): the thread lights lanes across the whole graph on a merge-heavy repo, and an emphasis image covers 32 rows.
Same picture in the heavy test at 187x50: emphasis_transmits_held=382 for 188 keys, bytes_per_key 181707.
- Key changes per cursor move: `crates/graft/src/event_loop/graphics.rs:313,320-322` (paint key = backdrop key +
  `lit_hash`, `crates/graft-graph/src/raster/cache/key.rs:146-158`).
- Old slot freed, whole image sent again: `crates/graft-term/src/kitty/mod.rs:285-294`.
- Raster+deflate is inside draw_us (`event_loop/running/frame.rs:101-107`): builder draw_us p50 6345 µs per key.
Cost scales with **viewport strips × thread width**, not with history length or ref count.

### History / ref growth (the ticket's premise): none found — by reading, **not by test**
- `view/entries.rs:40-46` `thread` → `Ancestry::of` over `history.window.rows`, run twice per frame (`view/rows.rs:66`,
  `view/strips.rs:224`); the window is the viewport ±3 heights (`history/scroll/query.rs:228-229`), not the history.
- `view/strips.rs:196-271` `pixel_rows`: viewport strip ranges only. `refs/mod.rs:41-55`: per-row `on`/`head`/`stash`
  are map/field lookups; no `branches()/remotes()/tags()` call in `view/` or `graft-ui` (grep, empty).
- The named candidate `view/strip.rs:38-42` (KNOWN_GAPS row 414): runs only with `O` shown and a search active. It is
  worse than the row says: `Search::hit` walks the batches (`search.rs:298-308`), so building the set is
  O(hits × batches) per frame — it **does** grow with history when a search matches most of it. His numbers don't
  point at it (Zed draw p99 1 ms; no sign he had `O` + a search). Row 414 stays open; suggested fix: keep a hit set in
  `Search`, extended once per batch.
- KNOWN_GAPS row 1095 (M8/01 review minors): not touched by this ticket; fate unchanged.

### Why no fix landed (rule 2: the cause is a design decision)
Emphasis following the cursor as one image per 32-row strip is ADR-0053 (`docs/adr/0053-selection-paint-is-its-own-layer.md`).
Cutting the bytes at the cause means changing that layer's shape, e.g. (planner's call, none tried):
1. emphasis in shorter tiles (8 rows), sent only where the lit set changed — helps only if changes are local; the
   share of rows whose lit set changes per key is **not measured** (next step: count it in `held_key`);
2. send the thread as its own small images per lane run instead of a full-width rectangle;
3. hold the emphasis while a key is held and send it once at rest (a symptom-side throttle: costs the live thread
   while scrolling — needs his OK).
A cap or throttle without a ruling would be a workaround. Ticket stays open for the ruling; boxes 2–4 wait on it.

### What he should run (to judge by feel, rule 6)
Nothing new yet. After a fix lands: the same Ghostty command as TESTING.md "M8: how he runs the trace", then
`scripts/m8-trace-summary.sh`; and a by-eye hold of `j` in Ghostty full screen on his repo.
Touches: scripts/m8-trace-followup.sh (new), docs/perf/m8-trace-followup.txt (new), this ticket. No Rust, no MAP.md,
no captures, no keymap. `m8-trace-summary.sh --self-test` ok (unchanged).

## Ruling (05 Part 2) — planner, 2026-10-10

**Spec change, internal; no visible change.** Amends ADR-0053 Decision 3 and DESIGN §3.8 (mechanism only — the look
ADR-0024 decided stays). Not a user question (memory: stop bringing him design decisions); conductor records it.
**Scope reshaped:** the ticket's premise (growth with history/refs) did not hold (NOTES); his numbers point at
Pixel emphasis bytes, so that is the fix. Box 2 (history/ref guard) **retired** — cost is viewport × thread width,
the guard below counts that instead. Row 414 **out** (his numbers don't point at it; Out clause) — stays open with the
fix written in it. Zed: no change.

### Decision: emphasis in per-column pieces, held by the terminal, placed not re-sent
Cause: one emphasis image per strip whose key hashes the whole strip's paint (`key.rs:146-158`), so any lit change
anywhere in 32 rows × full width makes a new picture. Re-placing by id (research `kitty-layering-deflate.md` §3) is
already what `UploadKey` does — it can't help while each `j` makes new pixels. Fix the key, not the transport:
- A strip's emphasis is split into **pieces, one per drawn column**. A piece holds exactly the elements whose ADR-0024
  alpha is decided in that column (diagonal edges: the column whose lit bit decides them; rule written in MAP.md),
  drawn alone at `(a − b)/(1 − b)`, cropped to whole cells of its own elements.
- **Piece key** = hash(strip backdrop key, column, alpha of each element the piece owns) — nothing else. A `j` that
  leaves a column's alphas alone leaves its key, so the terminal already holds it: one placement, zero transmit.
- Kitty: pieces are rasterised **only when `KittyImages` does not hold the key**, transmitted, pixels dropped. They
  never take a `RasterPool` buffer (one-size rule would weigh each at a full strip) and are not cached Graft-side.
  Terminal-side, unplaced uploads are bounded by **decoded bytes** (64 MiB; Kitty/Ghostty storage defaults are
  ~320 MB — builder cites the doc line), LRU by last placed; a placed image is never freed.
- iTerm2 unchanged: whole-strip `rasterise_emphasis` + `composite` stay (also the equivalence reference).
- Rejected: 8-row tiles (still full width: any column's change re-sends the tile); unaligned lane-run images (no strip
  alignment, scroll re-keys); hold-while-held throttle (symptom, loses the live thread, needs his OK); thinner thread
  (reverses ADR-0024).

**Gate before wiring (rule 1):** implement `emphasis_pieces` first and count, on the 50-key walk at 222x70 on
MONOREPO, the pixel area of pieces whose key changed per key vs today's emphasis area per key. ≥ 50 % → stop, report
`blocked: pieces don't localise` with the numbers; no wiring.

### Signatures
```rust
// graft-graph::raster
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece { pub column: usize, pub key: u64 }
/// Pieces this strip's emphasis needs; no rasterising. Empty when nothing draws above the backdrop.
pub fn emphasis_pieces(strip: &PlainStrip, selection: &Selection, strip_key: u64) -> Vec<Piece>;
/// One piece's pixels, cropped to whole cells; `None` if it draws nothing.
pub fn rasterise_piece(strip: &PlainStrip, selection: &Selection, piece: Piece) -> Result<Option<Emphasised>, RasterError>;
// graft-term::kitty
impl KittyImages {
    pub fn new(unplaced_bytes: usize) -> Self;      // was `capacity: usize` (count); callers: graft event_loop/graphics, tests
    pub fn holds(&self, key: UploadKey) -> bool;    // a look: terminal holds it, no retransmit
}
```
`Place`: one per piece (`layer: Emphasis`, column from `Piece`/`Emphasised::first_column`); placement ids unique per
place (derivation internal). If `Place.frame` is not already optional, make it `Option<…>`; `None` only after `holds`
said yes this frame — an unservable `None` goes down the missing-image path (retransmit next frame), never a blank.

### Acceptance (each tick names its test)
- [x] Count gate above, numbers in NOTES — `piece_gate::pieces_localise_on_his_walk` (heavy, release): 18 %.
- [ ] **(not as specified: the `b × 255` bound does not hold on overlaps — Part 2 NOTES)**
      `raster_pieces::pieces_composite_equals_single_emphasis`: merge-heavy fixture, ≥ 20 selections incl. Focus: backdrop +
      all pieces == `composite(backdrop, rasterise_emphasis)`; exact where lit elements don't overlap, else ≤ `b × 255`
      per channel (ADR-0053 bound).
- [x] `raster_pieces::piece_key_moves_only_with_its_column`: changing one element's alpha changes exactly its owner's key.
- [x] graft: `event_loop::tests::pixel_scroll::emphasis_pieces::transmits_equal_changed_pieces` (held `j`, merge-heavy: emphasis transmits per frame == pieces whose key
      the terminal didn't hold; linear history: 0 after the first frame, or say with a line why the cursor's own piece
      moves) and `…::emphasis_pieces::back_and_forth_retransmits_nothing` (`j` then `k`: 0 transmits). Linear: 0.
- [x] graft-term: `kitty::tests::pieces::held_unplaced_bytes_stay_under_budget` (1000 keys) and
      `kitty::tests::pieces::holds_false_after_missing_image_reply`.
- [ ] **(MISSED: 74 → 53 cells per key, 72 %; the test asserts it and is red — Part 2 NOTES)**
      Heavy `held_key::held_j_stops_on_release_pixel_deflate_free`: emphasis upload bytes per key ≤ 25 % of its value
      at HEAD before the change (recorded in the test comment). Count, not time.
- [ ] **(run and recorded; emphasis bytes MISSED by 2.6 %, the rest met — Part 2 NOTES)** After-measurement, same script, same shape: `scripts/m8-trace-followup.sh` → "after" block under "before" in
      `docs/perf/m8-trace-followup.txt`. Targets: emphasis bytes ≤ 1,078,750 (¼ of 4,315,003); bytes_per_key ≤ 46,054
      (½); draw_us p50 ≤ 3,172 (½); after_last_motion frames=0; deflate=yes; broken=no. Miss → report numbers, no
      constant tuning (rule 2). Script's layer split must still recognise piece uploads (check: emphasis id bit).
- [x] `pieces_shot::emphasis_pieces_png` (heavy; 34,412 B). Capture `docs/shots/m8-05-emphasis-pieces.png` (≤ 200 KB): MONOREPO at a merge selection, then Focus on.
      Existing pixel PNG captures regenerated; any changed pixel listed in NOTES. Colour **unverified** (rule 5).
- [x] (doc; no test) TESTING.md "M8: how he runs the trace", one line: in Ghostty full screen, release build, his repo — hold `j` 3 s,
      let go (right: stops on the row he let go at, thread lit at every step, never half-drawn), hold `k` back, `F` and
      hold `j` again; then the trace + `scripts/m8-trace-summary.sh`. Expected from the after-run (finding, not a fail
      if missed): write_us p99 ≤ 7,000 (was 14,064), bytes p95 ≤ 30,700 (was 76,751). `tail_ms` is not a judge (NOTES).
- [x] (doc; grep below) ADR-0053: "Amendment 2026-10-10" — Decision 3 reads per-column pieces as above; "one image per row" rejection
      kept, with why pieces differ (content-keyed, re-placed not re-sent). DESIGN §3.8 (`03-graph-layout-algorithm.md`
      :384-423): "a `j` re-sends a small emphasis image" → pieces; emphasis LRU now iTerm2-only (if so, grep).

### Touches
graft-graph `raster` (+ `cache`: `paint_key`/`lit_hash`/`StripCache::emphasis` callers after the change — remove only
if grep shows none); graft-term `kitty` (`new` callers, `frame`, `answer`, `forget_uploads` after resize, `free_all`
must free every piece, `EMPHASIS_ID_BIT`), `iterm` (grep: untouched); graft `event_loop/graphics`, `running/frame.rs`,
`row_clear`, idle prefetch (backdrops only, unchanged); held_key + frames tests; pixel PNG captures; scripts
`m8-trace-followup.sh`; MAP.md ×3; ADR-0053; DESIGN §3.8; KNOWN_GAPS M7/46-kitty-no-deflate (budget wording), 414
(unchanged). Not touched: keymap, journal/undo, Unicode tier.

### Build with, size
**Opus** — brownfield across three crates, but shape and signatures are fixed here and correctness is pinned by the
pixel-equivalence and transmit-count tests, not judged from the diff. Reuse scout `05-trace-followup.md`, re-checked
at the new hash; scout addendum on `Place`/placement ids and `KittyImages` capacity if the conductor wants one.
~70k. Over → split at the seam: (A) graft-graph pieces + count gate + equivalence/key tests; (B) graft-term + graft
wiring + measurement + docs.

## NOTES Part 2 (builder-opus, 2026-10-10)

**Compacted once mid-ticket** (context summary after the graft-term/graft wiring). Nothing left unbuilt; what remains
is the three misses below, for the conductor/planner.

Scout: none new — `.scratch/m8-polish-and-ship/scouts/05-trace-followup.md` reused (shape and signatures fixed by the
ruling); key lines re-read in place (`graphics.rs` `frame`/`layers`, `kitty/mod.rs` `frame`/`ensure_uploaded`/`evict`).

### Built
- graft-graph (commit bd93b388): `Piece`, `emphasis_pieces`, `rasterise_piece` (`raster/emphasis/pieces.rs`); the pen
  draws one column in strip coordinates through a translate (`raster.rs` `Part`, `pen.rs` `shift`), so a piece's
  anti-aliasing is the whole pass's. Cell scan + crop moved to `raster/emphasis/cells.rs` (file-size ratchet).
- graft-term: `Place.frame: Option<&RasterFrame>`; `KittyImages::new(unplaced_bytes)`, `holds`, `unplaced`; emphasis
  uploads found by key, strips by slot; eviction by decoded bytes, least recently placed (`kitty/mod.rs`). A `None`
  frame the terminal does not hold is skipped that frame (`frame`), and `holds` says no next frame, so it is sent then.
  `GraphicsReply` moved to `kitty/reply.rs`; piece tests in `kitty/tests/pieces.rs` (ratchet). iTerm2: a band with no
  pixels is an `InvalidInput` error (`iterm2.rs` `band`), never reached (graft always sends iTerm2 pixels).
- graft: `event_loop/graphics/layers.rs` (moved out of `graphics.rs`): per strip, the backdrop plus one place per
  piece; a held key with known cells is placed with `frame: None`, else rasterised, sent, dropped. `Graphics.pieces`
  (key → cells) is pruned to what `KittyImages` holds after every frame. `UNPLACED_BYTES` = 64 MiB.
  `view::max_lanes` is now `pub` (the gate test reads it).
- Signature note: `emphasis_pieces` returns `Vec` (no error) and is empty on a row-count mismatch;
  `rasterise_piece` returns `RasterError::Selection` there. A piece that draws nothing is never held, so it is
  rasterised again each frame it is wanted (rare: `emphasis_pieces` only lists columns with alpha above the backdrop).

### Numbers (before → after)
- Gate (pre-wire, `piece_gate::pieces_localise_on_his_walk`, 222×70, 50 `j`): pieces' area 17,992,545 px vs today's
  96,933,932 px = **18 %** (< 50 %), so wiring went ahead.
- `docs/perf/m8-trace-followup.txt` (two after runs, identical bytes): emphasis upload bytes **4,315,003 → 1,107,238**
  (target ≤ 1,078,750: **missed by 28,488**); bytes_per_key **92,108 → 25,802** (≤ 46,054 met); draw_us p50
  **6,345 → 2,504 / 2,854** (≤ 3,172 met); after_last_motion frames=0, deflate=yes, broken=no (met); the script's
  layer split reads piece uploads as emphasis (bit 31 still set: `emphasis=125`) (met).
- **Worse:** draw_us p95 15,459 → 26,526 / 28,713, p99 19,965 → 46,723 / 46,951; bytes p95 288,075 → 448,816,
  p99 486,064 → 573,577. Kept trace (scratch run): the heavy frames are 3, 6, 8 (motions=0, load) and 11
  (motions=1: bytes 486,334, draw 46 ms); every other frame is ≤ 5.1 ms. Cause **not measured**. Suspect (unverified):
  a frame where every piece is new rasterises each piece in its own pass with its own fresh pixmap
  (`rasterise_piece`, no pool by the ruling) and deflates each separately. Write p99 5,277 → 6,419 / 6,609.
- Heavy held_key free, A-deflate, release: `emphasis_cells_per_key` (Σ emphasis transmit widths in cells / 188; every
  emphasis image is one strip tall, so it is decoded upload bytes over a constant) **74 → 53** (72 %; target ≤ 25 %:
  **missed**). bytes_per_key 173,089 → 137,783. HEAD value measured at 61b643d5 sources, recorded in the test as
  `EMPHASIS_CELLS_PER_KEY_BEFORE`. Why, measured on the fast fixture (`pixel_scroll::a_capped_graph_…`, `--no-capture`):
  on a graph wider than the lane cap every `j` that moves the fold re-sends both strips **and every piece of them**
  (frames 1–16: 2 backdrops + 18/16/…/4 pieces each), because the piece key holds the strip key (the ruling's key).
  At his 187×50 the cap folds MONOREPO on most of the hold (`strip_transmits_held` 139); at 222×70 (the gate, the
  trace script) it folds less. No constant tuned (rule 2). A key without the strip key would not help: a fold change
  moves columns, so most pieces' pixels change anyway — that is for the planner.

### Deviation: the overlap bound (`raster_pieces::pieces_composite_equals_single_emphasis`)
Exact on every pixel at most one piece paints. Where two pieces paint one pixel, alpha matches within 2 but the colour
does not hold to `b × 255`: the whole pass draws a row's node, stubs and tail above every column's edges
(`raster/pen.rs`), the pieces stack by column. Measured: **53 of 6,030** overlapped pixels past `b × 255` (test prints
`SUMMARY pieces`). The test asserts alpha ±2 and counts colour. Recorded in the ADR-0053 amendment. Colour unverified.

### Captures (rule 5, 6)
- New: `docs/shots/m8-05-emphasis-pieces.png` (34,412 B), MONOREPO 222×70, cursor on row 4 (first merge past the
  start), top Focus off, bottom Focus on. **Question (not a defect, no line proving intent):** the two panels are
  byte-identical (`focus_on=true focus_changes_pixels=false`). With a thread, a Focus-dimmed row still gets
  `thread: Some(..)` (`view/strips.rs:388-397`), and `Lit::alpha_at` ignores `emphasis` whenever `thread` is `Some`
  (`graft-graph/src/raster/emphasis.rs:135-141`), so Focus may never fade a row at the Pixel tier once a thread is on
  screen. Same at 61b643d5 by reading; not changed here.
- Regenerated: `docs/shots/m7/fix-scroll-ghostty.png` — **231 pixels changed**, all in column x=824 (the rail),
  rows 0–400, max channel delta 8. Not from this ticket: the same test on 61b643d5 sources produces the new bytes
  (checked). Fast-tier PNG checks (`raster.rs` `check_png`, `emphasis_layer::thread_pngs`) green: unchanged.

### Touches (how each was checked)
- graft-graph `raster` + `cache`: `StripCache::emphasis`/`paint_key`/`lit_hash` kept — grep: `composited`
  (iTerm2, cache.rs:283), `graphics/layers.rs` (iTerm2 key), tests. graft-graph snapshots green.
- graft-term `kitty`: `new` callers (graphics.rs, `examples/kitty_check/driver.rs`, tests) updated; `frame`, `answer`
  (ENOENT → `holds` false: test), `forget_uploads`/`free_all` iterate every upload incl. pieces (existing
  `forget_covers_emphasis`, `free_all_frees_emphasis` green); `EMPHASIS_ID_BIT` unchanged. `iterm2`: only the
  `Option` unwrap (grep `place.frame`).
- graft `event_loop/graphics`, `running/frame.rs`, `row_clear`, idle prefetch: call `Graphics::frame`/`prefetch`
  unchanged in signature; prefetch draws backdrops only (`graphics.rs` `prefetch`, unchanged). Fast tier green.
- held_key + frames tests: `held_key` gains `emphasis_cells_per_key` (`Encoding` moved to `held_key/encoding.rs`,
  ratchet); `pixel_scroll` (c) now "paint changed ⇒ emphasis placed changed".
- `scripts/m8-trace-followup.sh`: unchanged; its bit-31 split still sees pieces (after block).
- MAP.md ×2 (graft-graph pieces + ownership rule; graft-term `Place.frame`, `new`, `holds`, `unplaced`). graft MAP:
  `map_coverage` green without a `max_lanes` line.
- ADR-0053 amendment; DESIGN §3.8 (two paragraphs); TESTING.md line; `over_400.txt` rows.
- For the conductor: the ruling lists KNOWN_GAPS M7/46-kitty-no-deflate (budget wording); not read here (builders
  do not). `KittyImages` is now budgeted in unplaced decoded bytes, not a count. Row 414 unchanged.
- Not touched: keymap, journal/undo, Unicode tier.

## NOTES Part 2, continuation 1 (builder-opus, 2026-10-10) — **blocked: needs ruling**

No compaction. No Rust changed (a temporary probe was built, measured with and removed; diff kept out of the repo).
No scout: the four misses are in files the previous builder named; I read `pieces.rs`, `layers.rs`, `raster.rs`
`paint_part`, `pen.rs`, `emphasis.rs` `lit_reach`, `kitty/mod.rs` `frame`/`ensure_uploaded` directly.
Worktree started from d1443600 by merge (fec68248; `reset --hard` was refused by the permission system); the
merge adds only `docs/log/m8.md` lines.

### Measured cause (`docs/perf/m8-trace-followup.txt`, "diagnosis" block)
**One cause behind misses 1, 2 and 3: a piece is full strip height and as wide as its column's farthest arc.**
- An incoming/outgoing arc in column `c` widens the piece's reach to the node's lane
  (`graft-graph/src/raster/emphasis.rs:346-348`); the pixmap is every strip row tall (`raster.rs:225-228`) and
  `crop` trims columns only (`raster/emphasis/cells.rs:77-83`). On MONOREPO most merges land on lane 0, so the
  piece of column 33 is 1156×1184 px for one 37-px-tall arc.
- Trace at his shape, first `j` (frame 10): 31 pieces, **13.9M px** rasterised + deflated (piece_us 29 ms,
  images_us 23 ms of draw 73 ms). One whole-strip emphasis image was ≤ ~4.3M px for three strips. Load frame 3:
  27 pieces, 7.5M px, 44 ms of 57 ms. The other heavy frame (8, 33 ms) is the strip load, as before. → miss 3.
- Heavy held_key 187×50: pieces wider than one lane = **91 % of piece pixels** (228M of 252M px, 288 of 866
  transmits). emphasis_cells_per_key sums those widths → miss 1; same bytes in the trace → miss 2.
- The previous builder's cause (piece key holds the strip key, `pieces.rs:85`; strip key moves with the
  cursor-dependent fold, `view/strips.rs:347-356`) is **real but secondary**: 263 of 866 piece transmits (30 %)
  were pixel-identical to a piece already rasterised under another key. Fixing it alone cannot reach 25 %.
- Miss 4 (overlap colour): cause as the previous builder said — the single pass draws all columns' edges, then
  every row's node over them (`raster.rs:253-270`); pieces stack by column (`layers.rs:118-131`, emitted in that
  order by `kitty/mod.rs:222-224`), so a later column's arc lands over an earlier lane's node. A piece holding
  both edges and its node has no stacking order that matches.

### Ruling change needed (exact)
Every fix changes the piece shape the ruling fixed ("one per drawn column", "cropped to whole cells"):
1. **Split each column's piece by kind:** a *lane* piece (straight runs, node, stubs, tail, rail — never wider than
   the lane's own cells + tail) and one *arc* piece per row an arc in that column is drawn on (one row band tall,
   as wide as the arc). Crop both kinds to whole cells **in both axes**.
2. **Signature:** `Piece { column, row: Option<u16> /* arc row; None = lane */, key }`; `Emphasised` gains
   `first_row: u16` (rows cropped); `Place.rows` intersected with the piece's own rows in `layers::places`.
3. **Stacking:** all arc pieces placed before all lane pieces (fixes node-over-arc, miss 4); arcs crossing a later
   column's straight run then sit under it, which the single pass does only for columns left of it — bound to be
   re-measured by `pieces_composite_equals_single_emphasis`, not assumed.
4. **Count:** `held_key`'s `emphasis_cells_per_key` becomes cell **area** per key (cells × rows / strip rows), since
   pieces are no longer strip-tall.
5. Optional, separate: key pieces on what they draw (theme, cell size, backdrop, the column's own rows' geometry)
   instead of the whole strip key — worth ≤ 30 % of transmits on the held run.
Estimate (**unverified**, from the shapes above): narrow area 23.6M px + one lane part per wide piece (~17M) +
arcs at one row (~8.5M per arc row) ≈ 50M of 252M px → ~12 cell-equivalents per key vs 74 before (≈ 16 %).

### Boxes
None ticked or unticked by this continuation; the three missed ticks and the overlap deviation stand as Part 2
left them. Gates not run (no Rust diff). Touches: `docs/perf/m8-trace-followup.txt` (diagnosis block), this ticket.

## Ruling (05 Part 2, amendment 1) — planner, 2026-10-10

Read: continuation 1 NOTES and diagnosis block (on the branch at ddcf2d20: this ticket :293-339,
`docs/perf/m8-trace-followup.txt:85-101`); Part 2 NOTES (worktree-agent-a26cdf27370943c36: this ticket :210-291).
This copy of the ticket does not yet hold those NOTES; when the branch merges they go **above** this section.

**Proposal accepted, modified.** The cause is measured, not suspected: 91 % of piece pixels sit in strip-tall pieces
wider than one lane; the first `j` rasterises 13.9M px (draw 73 ms). Cropping each piece to what it draws, in both
axes, is the fix at that cause. Three modifications:
1. **The lane piece splits again, into run and node.** The builder's "arcs before lanes" would put every lit arc under
   every lit straight run it crosses — on a merge-heavy graph an arc into lane 0 crosses most lanes. Splitting nodes
   out lets the stack follow the whole pass (edges, then nodes), which is what miss 4 needs.
2. **Key on content, not the strip key** (the builder's optional item 5, now required). Arc pieces need a new key
   anyway; a content key is what turns a fold or a shift of rows into a re-place instead of a re-send (30 % of
   transmits in the held run). Risk: a missed input places a stale picture, which no count test sees — pinned by
   `equal_keys_equal_pixels` below.
3. **Stacking order is explicit in the place command**, never the order commands happen to be written.

Still internal; the look ADR-0024 decided is unchanged; not a user question.

### Decision
- **Pieces per strip, three kinds** (ownership otherwise as MAP.md has it now):
  - **Run** of lane column `c`: the straight edges drawn in `c`.
  - **Arc** of (`c`, row `r`): every arc the pass draws in column `c` on row `r`, incoming and outgoing together.
  - **Node** of lane column `c`: node, lane stubs, tail of every row whose lane is `c`.
  - Rail: goes with Run of the marker column if the pass draws it with the edges, else with Node. MAP.md records
    which, with the `raster.rs` line.
- **Crop:** every piece to the whole cells its own pixels touch, columns **and rows**. `Emphasised` gains `first_row`.
  `rasterise_emphasis` sets it to 0 (iTerm2 path unchanged).
- **Key** = hash of exactly what the piece's pixels are drawn from: kind; cell size; backdrop alpha; per owned element
  its geometry in cells relative to the piece's **anchor** (`(c, 0)` for Run/Node, `(c, r)` for Arc), its resolved
  RGBA (not a palette index) and its alpha. No strip key, no absolute row or column. One picture anywhere on screen is
  one upload, placed as often as it appears. The crop offset from the anchor is then a function of the key: Graft
  stores (offset, size) per held key at first raster and places held pieces from anchor + offset, no raster.
- **Stack** (miss 4): Run and Arc pieces in the whole pass's edge order (`raster.rs:253-270`; the builder writes that
  order's sort key in MAP.md), then Node pieces. `emphasis_pieces` returns pieces in that order; `Place.stack`
  carries it. The builder cites the Kitty graphics-protocol doc line saying how overlapping images at one z stack and
  maps `stack` so the screen order never depends on emit order. Ghostty follows the same doc: **unverified** on his
  screen (rule 5).
- **Miss 4 target:** 0 pixels past `b × 255`. Non-zero → the count and the pass line no piece order can match, in NOTES
  and the ADR. Not allowed: reordering the whole pass (changes the backdrop's and iTerm2's look), raising the bound.
- **Unchanged:** Kitty only (iTerm2 whole strip + `composite`); pieces take no `RasterPool` buffer and no Graft-side
  cache; 64 MiB unplaced budget, least recently placed freed first.
- **Out:** the Focus-not-fading question (Part 2 NOTES, Captures) — the conductor files it as a question; row 414.

### Signatures (replace Part 2's for these items; `KittyImages::new`/`holds` stand)
```rust
// graft-graph::raster
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceKind { Run, Node, Arc { row: u16 } } // `row`: strip-relative
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece { pub column: usize, pub kind: PieceKind, pub key: u64 }
/// Bottom first, in stacking order. Empty when nothing draws above the backdrop or rows mismatch.
pub fn emphasis_pieces(strip: &PlainStrip, selection: &Selection) -> Vec<Piece>; // `strip_key` removed
/// One piece, cropped in both axes; `first_column`/`first_row` strip-relative. `None` if it draws nothing.
pub fn rasterise_piece(strip: &PlainStrip, selection: &Selection, piece: Piece)
    -> Result<Option<Emphasised>, RasterError>;
pub struct Emphasised { /* frame, first_column as today */ pub first_row: u16 }
// graft-term
pub struct Place { /* fields as today */ pub stack: u16 } // rank in its layer, higher on top; strips and iTerm2: 0
```
Callers of `emphasis_pieces`: graft `event_loop/graphics/layers.rs`, `piece_gate`, raster tests (grep). Of `Place`
literals: graft `layers.rs`, graft-term tests, `examples/kitty_check` (grep).

### Acceptance (replaces Part 2's open boxes; each tick names its test)
**Build A — graft-graph, plus graft's `piece_gate` test file:**
- [x] `raster_pieces::pieces_composite_equals_single_emphasis`: backdrop + pieces in `emphasis_pieces` order ==
      `composite(backdrop, rasterise_emphasis)`; alpha ±2 on every pixel; colour ≤ `b × 255` on every pixel (printed
      count 0, else the finding above).
- [x] `raster_pieces::piece_key_moves_only_with_its_pixels` (replaces `…_with_its_column`): one element's alpha
      changed → exactly its owner's key changes.
- [x] `raster_pieces::equal_keys_equal_pixels`: merge-heavy fixture, ≥ 20 selections incl. Focus, at two lane caps
      (fold on and off): any two pieces with one key rasterise byte-equal with the same anchor-relative crop. Prints
      pieces / distinct keys.
- [x] `raster_pieces::pieces_crop_both_axes`: every piece's frame has a drawn pixel in its first and last cell column
      and its first and last cell row.
- [x] **Gate** (`piece_gate::pieces_localise_on_his_walk`, `piece_gate::no_frame_rasterises_more`, heavy, release: 2.13 % / 2.11 %, 0 frames over), MONOREPO, 50 `j`, at 222×70 **and** 187×50: `piece_gate::pieces_localise_on_his_walk` — area of pieces
      whose key was not seen earlier in the walk ≤ 25 % of the whole-strip emphasis area over the walk;
      `piece_gate::no_frame_rasterises_more` — in every frame, px of unseen-key pieces ≤ px of that frame's whole-strip
      emphasis images. Numbers in NOTES. Either misses → stop, `blocked: pieces don't localise` with the numbers; no
      build B; the conductor asks the user.

**Build B — graft-term, graft, docs:**
- [x] (`kitty::tests::pieces::one_upload_two_places`, `kitty::tests::pieces::stack_reaches_the_command`; the four
      existing tests green in the workspace run) graft-term `kitty`: `one_upload_two_places` (one key placed twice in a frame: one transmit, two placements) and
      `stack_reaches_the_command` (`stack` maps to the field the cited doc line names). Existing
      `held_unplaced_bytes_stay_under_budget`, `holds_false_after_missing_image_reply`, `forget_covers_emphasis`,
      `free_all_frees_emphasis` green.
- [x] (`event_loop::tests::pixel_scroll::emphasis_pieces::transmits_equal_changed_pieces`, `…::back_and_forth_retransmits_nothing`,
      workspace nextest after the stack change) graft: `transmits_equal_changed_pieces` and `back_and_forth_retransmits_nothing` green under content keys.
- [x] (`held_key::held_j_stops_on_release_pixel_deflate_free`, release, heavy: `emphasis_area_per_key=3.2`, verdict ok)
      Heavy `held_key::held_j_stops_on_release_pixel_deflate_free`: `emphasis_cells_per_key` becomes emphasis **area**
      per key, Σ(cells × rows) / `STRIP_ROWS` / keys (a strip-tall image counts as before, so
      `EMPHASIS_CELLS_PER_KEY_BEFORE` = 74 stands); **≤ 18.5** (25 %). Count, not time.
- [ ] **(run and recorded; bytes and the rest met both runs; draw_us p50 and p99 MISSED in run 1, met in run 2 —
      NOTES B)** After-measurement, `scripts/m8-trace-followup.sh`, same shape, **two runs, both on AC** (power line in each
      block), "after, amendment 1" blocks under the existing ones. Targets, **each run**:
      emphasis upload bytes ≤ 1,078,750; bytes_per_key ≤ 46,054; bytes p95 ≤ 288,075, p99 ≤ 486,064 (no regression;
      deterministic); draw_us p50 ≤ 3,172, **p95 ≤ 15,459, p99 ≤ 19,965** (no regression vs before);
      after_last_motion frames=0; deflate=yes; broken=no; the emphasis id bit still splits the layers. Miss → the
      numbers, the trace frame line of each over-bound frame and whether it loads strips; no constant tuning (rule 2);
      the conductor goes to the user.
- [x] (`pieces_shot::emphasis_pieces_png`, `ghostty_shot::fix_scroll_ghostty_png`; all 126 heavy `shot|png` tests green)
      `docs/shots/m8-05-emphasis-pieces.png` and the pixel PNG captures regenerated; changed pixels listed in NOTES.
      Colour **unverified** (rule 5).
- [x] (doc; no test) TESTING.md line from Part 2 stands; its expected numbers unchanged (write_us p99 ≤ 7,000, bytes p95 ≤ 30,700).
- [x] (doc; no test — `map_coverage` green for MAP.md) ADR-0053 text as below; DESIGN §3.8 pieces paragraph → three kinds, both-axis crop, content key, explicit stack;
      MAP.md graft-graph (`PieceKind`, ownership incl. rail, key inputs, edge order) and graft-term (`Place.stack`,
      the doc line).

### ADR-0053 amendment: text change
- Heading → "Amendment 2026-10-10 (m8-polish-and-ship 05 Part 2, amended after continuation 1): emphasis in pieces".
- "Decision 3 now reads" paragraph, first two sentences, becomes: "per strip, the emphasis is split into **pieces** of
  three kinds: a **run** per lane column (the straight edges drawn in it), an **arc** per column and row (the arcs drawn
  in that column on that row), and a **node** per lane column (the nodes, lane stubs and tails of rows on that lane);
  the rail goes with the marker column's [run|node]. Each holds exactly its elements, drawn alone at
  `(a − b) / (1 − b)`, cropped to the whole cells its pixels touch in both axes. **Piece key** = hash of what its
  pixels are drawn from — kind, cell size, backdrop alpha, and each element's geometry relative to the piece's anchor,
  its colour and its alpha — so one picture anywhere on screen is one upload. Pieces stack as the whole pass draws:
  edges (runs and arcs) in its edge order, then nodes; each place command carries that order." Rest of the paragraph
  (Kitty `holds`, 64 MiB, iTerm2) stays.
- "Why" gains: "The first cut — one piece per column, strip-tall, as wide as its farthest arc — cut bytes per key by
  72 % but not pixels: an arc into lane 0 made its column's piece the graph's width and 32 rows tall, so 91 % of piece
  pixels were in such pieces and the first `j` rasterised 13.9M px (draw 73 ms, `docs/perf/m8-trace-followup.txt`,
  diagnosis). An arc is one row tall, so its piece is. Keying pieces on the strip re-sent 30 % of them unchanged after
  a fold; a content key re-places them."
- "One emphasis image per row stays rejected", second sentence becomes: "a row image was re-made whenever its row's
  thread changed, which a move onto another lane does to most rows; an arc piece is one row tall too, but holds one
  column's arcs and is keyed by its pixels, so a move re-sends only arcs whose own alpha changed, and the same arc
  anywhere is placed, not sent."
- "Overlaps" paragraph becomes: "Pieces stack in the whole pass's order (edges, then nodes), so overlapping lit pixels
  match it within `b × 255`: `raster_pieces::pieces_composite_equals_single_emphasis` counts N past it." N measured
  by build A; if non-zero, the pass line that no piece order can match. Colour **unverified** sentence stays.
- "Consequences changed" gains: "A frame places one strip plus its runs, nodes and arc pieces per strip; one upload
  may be placed more than once a frame."

### Touches (beyond Part 2's list)
`Emphasised` users (`rasterise_emphasis`, `composite`, iTerm2 path in `layers.rs`, cache): `first_row` = 0, grep.
`Graphics.pieces` (key → cells) becomes key → anchor-relative (offset, size). `Place` literal sites (grep). `piece_gate`
at a second shape. `held_key` metric rename. Captures. Not touched: keymap, journal/undo, Unicode tier, prefetch.

### Build with, size
**Opus, split A then B, sequential** (B calls A's signatures). A ≈ 35k (graft-graph rework, four raster tests, gate at
two shapes); B ≈ 45k (kitty `stack` + two-place test, `layers.rs` anchor/offset/stack, `held_key` area, two AC runs,
ADR/DESIGN/MAP, captures). One context for both would repeat Part 2's compaction. Not Fable: shape, key inputs and
order are fixed here; the equality, crop and count tests pin correctness; the one thing no test judges (stack on his
real terminal) rests on a cited doc line and his by-eye capture. Scout: none new — Part 2's scout plus continuation 1's
reading cover `pieces.rs`, `layers.rs`, `raster.rs`, `kitty/mod.rs`; each build re-reads its lines in place. A
stopping at its gate ends the pair: this is the last continuation before the user is asked.

## NOTES (05 Part 2, amendment 1 — build A), 2026-10-11

**Compacted once mid-build** (context ran out after the gate passed, before file splits / MAP / ticket). Remaining at
that point, all done since: test-file split under 400, graft-graph MAP.md, workspace gates, this note. Then stopped on a
network error before the gates; resumed in place, worktree intact. Scout: none new, as the ruling says; lines re-read.

**Numbers** (all printed by the tests named in the ticks):
- Gate, release, heavy, MONOREPO 50 `j`: 222×70 pieces 2,062,491 px vs today 96,933,932 px = **2.13 %**, 135 pieces
  sent vs 144 strips changed; 187×50 1,551,743 vs 73,382,914 = **2.11 %**, 112 sent vs 96 changed.
  `no_frame_rasterises_more`: 0 frames over at both; first frame 91,834 vs 1,512,745 (222×70), 71,706 vs 1,285,047
  (187×50); most pieces in one frame 1,327,190 / 693,158 px.
- Composite: 144 selections, all lit, 12,060 overlapped pixels, **N = 0 past `b × 255`** (miss 4 closed; ADR's
  "Overlaps" paragraph can say 0, no pass line to cite). Stack sort: (edges 0 / nodes 1, column, Run before Arc, row).
- Equal keys: 288 selections (both caps: `branchy` with marker, and fold off / `marker = None`), 5,112 pieces, 614
  distinct keys, 0 mismatched.
- Rail: drawn in the row pass (`raster/pen.rs:265`, `Pen::row` → `rail`), so it is the **marker column's Node**.
  Recorded in graft-graph MAP.md.
- Workspace gates: fmt, clippy (`--all-targets`, also `--features graft/heavy-tests`), nextest 6321/6321, doctests ok.

**Implementation choices the reviewer should check** (none changes the ruling's shape):
- Equal keys needed equal pixels at any position, which f32 `p + T` rounding broke (112/5,112 arc pieces off by one
  AA step). Pieces are now drawn in a local frame (arc: its column cell, band top; node: its lane cell, band top) and
  placed by a whole-pixel translate; stroke outlines are snapped to a 1/1024 px grid first (`geometry.rs`
  `snapped`, `stroke_placed`, `fill_placed`) so the translate is exact. The whole pass draws the same way, so whole ==
  pieces holds too.
- That snapping moved anti-aliasing in the whole pass by at most one AA step. Captures regenerated
  (`TRIB_UPDATE_SHOTS=1`); changed px / max channel delta: 22-criss-cross 15/16, -on-dark 15/11; 22-foxtrot 8/16,
  -on-dark 8/9; 22-octopus 11/16, -on-dark 11/12; 22-rings-on-dark 12/9; 40-folded-on-dark 12/9; 42-rings-on-dark
  12/9; 42-thread-on-dark 24/12; m7/fix-scroll-thread-composite 95 of 442,816 / 12. Snapshots
  `raster-criss-cross-merge`, `raster-pixel-digests` re-accepted. AA level only, no reorder. Colour **unverified**
  (rule 5); the user judges by eye (rule 6).
- Bug fixed on the way (cause): `node.rs` `fill_square` (Stash ring) shifted by `tx` only, never `ty`, so a y-cropped
  frame lost it. It now adds both.
- Taken from B early, for interim correctness only: `layers.rs` `Crop` (key → anchor-relative columns/rows and size)
  and `Graphics.pieces: HashMap<UploadKey, Crop>`; `places` clips piece rows to the strip. Without it A's pieces would
  have been placed wrong in the app between A and B.
- Files split for the 400-line ratchet: `pen/straight.rs`, `emphasis/pieces/extent.rs`,
  `tests/it/raster_pieces/support.rs`.

**Handoff for build B** (A's signatures are as the ruling fixed them; nothing else for B to change in graft-graph
except MAP wording if the ADR text wants it):
- graft-term `Place.stack` + kitty `one_upload_two_places`, `stack_reaches_the_command`. **Known interim defect until
  then:** kitty places with `p=1` per image, so one content key placed twice in a frame shows once. Equal keys are
  common (614 distinct of 5,112), so this is visible in the app until B lands. Not a gate case; flagging so nobody
  reads A's state as shippable alone.
- `layers.rs`: `Crop` / anchor already done; B adds `stack` from `emphasis_pieces` order (index in the Vec) into each
  `Place`, and placement ids per (key, place) so duplicates coexist.
- graft `transmits_equal_changed_pieces`, `back_and_forth_retransmits_nothing`: still green under content keys (in the
  6321 run) — B re-checks once stacking changes placement.
- `held_key` area metric ≤ 18.5; two AC after-runs; `docs/shots/m8-05-emphasis-pieces.png` (not regenerated by A).
- ADR-0053 text with N = 0 and rail → node; DESIGN §3.8; graft-term MAP.md. graft-graph MAP.md is done.

## NOTES (05 Part 2, amendment 1 — build B), 2026-10-11

No compaction. Scout: none (the ruling says none new; A's handoff named the lines; I read `kitty/mod.rs`,
`kitty/encode.rs`, `layers.rs`, `held_key.rs` in place). Started by merging A (2fe6d052); no ticket conflict.

**Built**
- graft-term: `Place.stack: u16` (`kitty/place.rs`, split out of `mod.rs` for the 400-line ratchet, re-exported).
  Emphasis places send `z = 1 + stack`; strips send no `z`, as before. Doc line: "If two images with the same z-index
  overlap then the image with the lower id is considered to have the lower z-index" (sw.kovidgoyal.net/kitty/
  graphics-protocol, "Controlling displayed image layout"), so ids (an old upload keeps its old id) never decide order.
- **A's interim defect fixed (cause):** `KittyImages::frame` numbers each upload's placements this frame `p=1..=n`
  and removes `p > n` left from last frame; `hide` removes all `n`. One content key placed twice shows twice.
  `Upload.placed: bool` → `placements: u32`. Pinned by `one_upload_two_places`.
- graft `layers.rs`: each piece's `stack` = its index in `emphasis_pieces` (A's order: edges by (column, Run before
  Arc, row), then nodes); strips 0.
- `held_key`: `emphasis_area` = Σ(cells × rows) of emphasis transmits; bound `area × 4 ≤ 74 × 32 × 188` (18.5 per key,
  compared exactly). Drain parser reads `v=` (`height_px`). Moved for the ratchet: `held_key/transmits.rs`,
  `drain_sink/tests.rs`; `an_upload_is_matched_by_its_key` from `kitty/tests.rs` to `kitty/tests/pieces.rs`. Rows lowered.
- `scripts/m8-trace-followup.sh`: optional `TRACE_KEEP=<path>` keeps the raw trace (measurement unchanged).

**Numbers** (before → after)
- held_key A-deflate free, release: emphasis area per key **74 → 3.2** (≤ 18.5 met); bytes_per_key 173,089 → 120,198;
  emphasis transmits 656, strip 157.
- Trace, `docs/perf/m8-trace-followup.txt` "after, amendment 1" (two runs, both AC, eee492a7), run 1 / run 2:
  emphasis bytes 4,315,003 → 176,656 / 176,656 met; bytes_per_key 92,108 → 13,096 / 13,133 met; bytes p95 288,075 →
  125,744 met; p99 486,064 → 439,220 met; draw p95 15,459 → 11,869 / 4,319 met; **draw p50 6,345 → 4,781 / 986
  (≤ 3,172: MISSED run 1)**; **draw p99 19,965 → 35,747 / 12,740 (≤ 19,965: MISSED run 1)**; frames after last
  motion 0, deflate yes, broken no, id bit splits the layers (emphasis=144): met.
- Run 1's p99 frame: `motions=0 draw_us=35747 bytes=439220`, before the first key — it carries strips (439 KB > all
  emphasis bytes of the run, 177 KB). Same binary, identical bytes frame for frame: run 1's draw times are 3–5× run 2's,
  so the spread is not from the work done. Its cause is **not measured**; load average was 7.3 (node 157 %, dasd
  100 %). Two extra after runs (p50 4,497 / 4,364, p99 37,296 / 26,258) and the BEFORE binary rebuilt from ad7a435f and
  interleaved (p50 6,365 / 8,124, p99 39,535 / 38,112 — over its own recorded 19,965) are in the perf file. Under the
  same conditions after is below before on every draw percentile; the absolute targets, set on a quieter run, hold in
  1 of 4 after runs. No constant tuned (rule 2). **For the conductor:** re-run both binaries on a quiet machine, or
  the user decides.

**Captures** (colour unverified, rule 5; the user judges, rule 6)
- `docs/shots/m8-05-emphasis-pieces.png` regenerated (34,406 B): 76 of 1,418,480 px changed, max channel delta 13,
  x 0–344, y 21–1183 — A's AA snapping and stack order (A had not regenerated it). Panels still byte-identical Focus
  on/off (the Part 2 question stands; out of scope).
- `docs/shots/m7/fix-scroll-ghostty.png` regenerated: 32 of 345,950 px, max delta 13. Built through
  `StripCache::composited`; graft-graph has no change since A's tip (diff --stat against 2fe6d052), so the drift is
  A's raster change, not B's — by reading; A's tip not re-run.

**Touches** (how checked)
- `Place` literal sites (grep `layer: Layer::|layer: crate::kitty|layer: graft_term`): kitty tests, iterm2 tests,
  `examples/kitty_check`, `layers.rs` — `stack` set in each; iTerm2 never reads `stack` (grep).
- Kitty byte stream: strip placements unchanged (`deflate_off_is_unchanged`, pinned bytes, green); emphasis rank 0 is
  `z=1` as before (`emphasis_sits_above_its_strip` green).
- `forget_uploads`/`free_all`/`answer`: paths unchanged; `forget_covers_emphasis`, `free_all_frees_emphasis` green.
- Not touched: keymap, journal/undo, Unicode tier, prefetch, graft-graph.
- MAP.md graft-term (`Place.stack`, `frame` placement ids); ADR-0053 amendment rewritten to the ruling's text with
  N = 0 and rail → node; DESIGN §3.8; TESTING.md line gains a by-eye stack check (dot over lit line), numbers
  unchanged; `over_400.txt` rows lowered.
