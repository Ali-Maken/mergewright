# G-trace follow-up — remove the per-frame cost that grows with history or refs

Status: **partial 2026-10-10 — finding made, no product change; cause is designed (ADR-0053), needs a planner ruling (NOTES).** Was: **deferred by the user 2026-10-08 ("lets finish the product, and then deal with this") — runs after the rest of M8 is built, before the milestone stop.** His trace run exists (2026-10-08, docs/perf/m8-trace-2026-10-08-{ghostty,zed}.txt; was held until it did) (`docs/perf/m8-trace-<date>.txt` from his machine, not the
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
- [ ] Count gate above, numbers in NOTES (test named).
- [ ] `raster::pieces_composite_equals_single_emphasis`: merge-heavy fixture, ≥ 20 selections incl. Focus: backdrop +
      all pieces == `composite(backdrop, rasterise_emphasis)`; exact where lit elements don't overlap, else ≤ `b × 255`
      per channel (ADR-0053 bound).
- [ ] `raster::piece_key_moves_only_with_its_column`: changing one element's alpha changes exactly its owner's key.
- [ ] graft: `transmits_equal_changed_pieces` (held `j`, merge-heavy: emphasis transmits per frame == pieces whose key
      the terminal didn't hold; linear history: 0 after the first frame, or say with a line why the cursor's own piece
      moves) and `back_and_forth_retransmits_nothing` (`j` then `k`: 0 transmits).
- [ ] graft-term: `held_unplaced_bytes_stay_under_budget` (1000 keys) and `holds_false_after_missing_image_reply`.
- [ ] Heavy `held_key::held_j_stops_on_release_pixel_deflate_free`: emphasis upload bytes per key ≤ 25 % of its value
      at HEAD before the change (recorded in the test comment). Count, not time.
- [ ] After-measurement, same script, same shape: `scripts/m8-trace-followup.sh` → "after" block under "before" in
      `docs/perf/m8-trace-followup.txt`. Targets: emphasis bytes ≤ 1,078,750 (¼ of 4,315,003); bytes_per_key ≤ 46,054
      (½); draw_us p50 ≤ 3,172 (½); after_last_motion frames=0; deflate=yes; broken=no. Miss → report numbers, no
      constant tuning (rule 2). Script's layer split must still recognise piece uploads (check: emphasis id bit).
- [ ] Capture `docs/shots/m8-05-emphasis-pieces.png` (≤ 200 KB): MONOREPO at a merge selection, then Focus on.
      Existing pixel PNG captures regenerated; any changed pixel listed in NOTES. Colour **unverified** (rule 5).
- [ ] TESTING.md "M8: how he runs the trace", one line: in Ghostty full screen, release build, his repo — hold `j` 3 s,
      let go (right: stops on the row he let go at, thread lit at every step, never half-drawn), hold `k` back, `F` and
      hold `j` again; then the trace + `scripts/m8-trace-summary.sh`. Expected from the after-run (finding, not a fail
      if missed): write_us p99 ≤ 7,000 (was 14,064), bytes p95 ≤ 30,700 (was 76,751). `tail_ms` is not a judge (NOTES).
- [ ] ADR-0053: "Amendment 2026-10-10" — Decision 3 reads per-column pieces as above; "one image per row" rejection
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
