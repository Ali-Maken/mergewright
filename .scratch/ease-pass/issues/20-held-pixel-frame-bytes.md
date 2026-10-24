# 20 — Held-key pixel frames write 1.8–4.6 MB each

Status: ready (ruling 2026-10-22, below) · Build with: Opus — brownfield, five pixel-path files across `view::strips`, `event_loop::graphics`, kitty and iTerm2, plus a visible change; the band-and-lower mechanism already exists (`TallRow`, ease-13), so a wrong approach is not structural (Fable test 1 fails) · Blocked by: none
Source: 16 NOTES `16-fix-heavy-tier.md:30`; scout `.scratch/ease-pass/scouts/19b-held-bytes-and-pngs.md`.

## Need
Holding `j` at the pixel tier (the user's daily view in Ghostty is Unicode, but pixel is shipped) writes 1.8–4.6 MB per
held frame (debug) against the test's 512 KiB (`held_key.rs:331`, 844c1e5b; not a design number). The test was never
genuinely green (`m8-settle.txt:7-9`: frames_while_held=0). Scout could not find the cause line.

## Scope
Find what a held pixel frame writes (image data re-sent vs placed by id, full redraw, role strokes from ease-pass-13 —
the scout's unconfirmed guess). Name the cause line. If the fix is at cause and needs no decision, fix it. If it is
"budget vs code" (the bytes are what the design asks for), stop and report with numbers both ways.

## Acceptance
- [x] Cause named with file:line and a measurement (bytes per held frame, split by what they are).  <- `held_key::held_j_stops_on_release_pixel` instrumented run, `docs/perf/20-held-pixel-bytes.txt:23-40` (SPLIT); ablations `:44-66`
- [x] Either fixed at cause with the three held_key pixel tests green (name them), or a decision written for the conductor.  <- decision below; `held_key::{held_j_stops_on_release_pixel, held_j_stops_on_release_pixel_free, j_then_journal_opens_at_once_pixel}` measured red at HEAD (`:2-16`), not fixed
- [x] Measurement from a script → `docs/perf/20-held-pixel-bytes.txt`.  <- `scripts/ease-20-held-pixel-bytes.sh` (`:1-15`)
- [x] Touches: line (other screens drawing strips, captures, kitty/iTerm2 paths).  <- NOTES Touches (grep-checked; nothing changed in code)

## NOTES (debugger)
Status: not fixed, decision needed. Scout: 19b (named); no new scout, known files. No compaction.

**Cause (measured).** A held raw frame is strip re-sends at raw RGBA (2.2 MB per 10-lane strip, 45× its deflated
50 KB, `:68-69`). Split, A at 40 MB/s, 10 held frames, 17.7 MB Kitty bytes (`docs/perf/20-held-pixel-bytes.txt:23-40`):
- **uncommitted row moved, 20 %, and what keeps it there**: `crates/mergewright/src/view/strips.rs:290`
  (`put_uncommitted`) inserts the pinned uncommitted row into the strip holding `top`, at `top - first_row`. Each
  scroll step moves the index, so that strip's layout hash changes (`mergewright-graph/src/raster/cache/key.rs:126-128`
  hashes the rows), and `mergewright-term/src/kitty/mod.rs:278` frees it and sends it again, with its full-height
  emphasis pieces, every frame. Frames 6-11 are this alone (444 KB each at 1 lane; 2.2 MB + emphasis at 10 lanes,
  frame 3).
- roles 24 %: focus branch changed (`view/entries/roles.rs:52`; ease-pass-13 by design: the role key is per strip,
  `key.rs:141-144`).
- new strips 17 %, emphasis 22 %, fold 3 %, other 12 % (frame 3 strip 32, unattributed).
- **Ablation** (temporary env switch, removed): no insertion → 240-494 KB/held frame across 3 runs × 3 tests, all
  ≤ 512 KiB (`:48-51, 60-66`); roles off alone → 1.29-1.34 MB (`:52-55`); both → 177-327 KB (`:56-59`).
  Removing the insertion also roughly quadruples frames_while_held (8-11 → 32-55): smaller frames, more of them.

**Why not fixed: it changes pixels.** No re-key-free way keeps the virtual row inside the strip. The fix is the pinned
row as its own one-row image (like ease-13's continuation band, `graphics/tall.rs`), with every strip lowered one
row. Then the top history row's own pixels change: its node's upward stub and lanes are drawn from the row above
(`mergewright-graph/src/raster/pen.rs:78-82, 261`), now the real row and not the lane-less virtual row
(`view/entries.rs:280-283`). That is a visible change (rule 6, build-to-the-mocks).

**Decision for the conductor / user:**
1. Fix: pinned row as its own image when it is detached from `HEAD`; capture for his eye. Expected 0.24-0.49 MB/held
   frame raw, under 512 KiB but tight (worst 494 KB of 524 KB). Deflate (Ghostty) also gets fewer re-sends.
2. Leave it: the three raw tests stay red under known gap M7/46-kitty-no-deflate (the user's call 2026-10-01,
   `docs/log/m7.md:572`). Deflate is 64 KB/held frame and green (`:69`).
Context: 512 KiB (844c1e5b, 10-01) is older than the pinned row (48f3041a, m7-48, 10-02). The steady state with
the insertion (444 KB at 1 lane) is under budget. Only the wide frames break it.

Touches (if 1): `view/strips.rs` pixel_rows/put_uncommitted + `strips/tests.rs:114-130`
(`the_uncommitted_row_sits_on_head_in_its_strip`); `event_loop/graphics.rs:220` (`lowered`), `graphics/layers.rs`
`on_screen`, `graphics/tall.rs` make_room (line offsets); the iTerm2 path (`ItermImages` bands, `iterm2.rs:252`); prefetch
(backdrops only, unchanged); `tests/it/ghostty_shot.rs`, `pieces_shot.rs` (read `uncommitted_on_top`, ticket 21 PNGs).
Text tiers are not touched (pixel only). Grep: `uncommitted_on_top|uncommitted_at()` hits only those five files.
This ticket changed no code: a script and a perf file. No gates run (no code changed); the three tests' run is the check.

## Ruling (planner, 2026-10-22)

**Option 1. A defect at cause, whatever the budget.** The strip bound was ruled on 2026-10-01 (46 ruling,
`docs/log/m7.md:570`; test `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes`, 46c): a strip
re-sends only when its own content changes — fold, and since ease-13 roles (by design, `key.rs:141-144`). 48
(48f3041a, the next day) made the pinned row's *screen position* part of a strip's key (`strips.rs:290` →
`key.rs:126-128`); 46c's test has no uncommitted row, so nothing caught it. A pinned row is an overlay on the
viewport; a strip's key must never depend on where the viewport is. This also costs Ghostty (deflate) a re-send of
that strip and its emphasis every scroll step, not only the raw path. The 512 KiB is the test's number, not a design
number: it is not why we fix, and it does not move here either way.

**Pixels — two parts, ruled apart.**
- *The pinned row itself:* drawn identically. It is lane-less (`view/entries.rs:280-283`, per debugger), and a row's
  pixels are drawn from the row *above* it (`pen.rs:78-82, 261`, per debugger), so alone in a one-row image it
  rasterises the same. Its key no longer depends on `top`: uploaded once, placed by id each step.
- *The top history row under it:* changes, and the change is the correct graph. Today (per debugger's lines) its
  upward stubs and lane tops are drawn as if the row above were the lane-less uncommitted row, so lanes that run on
  up into history look as if they start there — a passing lane drawn like a branch tip (M1 promise; "different
  things must look different"). After: drawn from the real row above, as at any scrolled-off top edge, and as the
  Unicode tier draws its own `│` cells. Keeping today's look would need a `top`-keyed piece — the same re-key under
  another name, keeping a false picture. Rejected.
- Screen lines do not move: the pinned row on the pane's first line, history from the second, cursor where 48 put
  it (`history_row_line` unchanged). Text tiers untouched.

**What the user will see (conductor: put before him by eye before merge, ADR-0063).** Pixel tier only (Ghostty for
him; Zed/VS Code are Unicode, unchanged), only while the uncommitted row is pinned at the top (`HEAD` scrolled
off): the history row directly under the pinned row shows its lanes and its commit's upward line reaching the top
edge of that row, where today they stop at it. Nothing else moves or changes colour. Before/after PNGs below.
Colour unverified (rule 5). If he rejects it, the ticket does not merge and option 2 stands.

**Terminals.** His: Zed/VS Code terminals (Unicode, unaffected); Ghostty (pixel, deflate probed, unverified on his
machine — gap row M7/46-kitty-no-deflate). Raw Kitty (no deflate) is not a terminal he runs; it is still the path
the budget tests measure, and the one this makes usable.

**Scout:** the debugger's NOTES above name every line and the Touches; they serve as the scout (ADR-0035). No new
scout unless a named line has moved; then stop and report.

## Replacement acceptance (ruling)

Order: step 0 first, red-checked; then build; boxes tick only with the named test, green, inline.

- [x] Step 0: `docs/shots/ease-pass/20-pinned-row-before.png` (5.6 KB), drawn at HEAD 3123840e by `pinned_row_shot::pinned_row_png` (`tests/it/pinned_row_shot.rs`, `MWR_UPDATE_SHOTS=1`, committed 6c014e92 before any code change).
- [x] Red first, then green: `view::strips::pinned_tests::a_pinned_uncommitted_row_leaves_strip_keys_alone`. <- red at f408cea2 ("strip 32 re-keyed when the viewport moved one row"), green after 87fc9ae7. (Lives in `pinned_tests.rs`, not `tests.rs`: `tests.rs` is an over_400 row.)
- [x] Red first, then green: `event_loop::tests::pixel_scroll::pinned::a_pinned_uncommitted_row_resends_no_strip_while_scrolling`. <- red on the HEAD 3123840e sources ("scroll step 1 re-sent a strip under a pinned row, left 1 right 0"), green after. Raw and deflate. Roles: `history_of(100)` is one branch, so the role key never changes.
- [x] `view::strips::pinned_tests::the_pinned_row_image_matches_the_virtual_row`. **Deviation, see NOTES 1:** golden = the virtual row as HEAD 3123840e rasterised it with *no history row above it* (`top` 0, `HEAD` on row 1, 1296 bytes, `pinned_row.golden`). With a history row above it, HEAD drew that row's lane into the pinned row's top edge (pixel rows 0-2, x 3-5 at the test cell); that is not reproducible in a one-row image.
- [x] Red first, then green: `view::strips::pinned_tests::under_the_pinned_row_the_top_history_row_is_its_real_self`. <- red at f408cea2 (the premise holds; not green at HEAD), green after 87fc9ae7.
- [x] `event_loop::graphics::pinned_tests::the_pinned_row_keeps_every_screen_line` (pinned image on the column's first line in `PINNED_SLOT`, each history row on `history_row_line`, no two images on one line) + `...::the_cursor_on_the_pinned_row_still_makes_room_for_its_second_line`. Mouse hit-test: `update::mouse::tests::click_on_the_last_row_under_the_uncommitted_row` and `a_click_in_a_one_row_pane_selects_what_is_drawn` green unmodified (`graph_row_at` is `update`-private and unchanged); 48's `view::tests::held_j_keeps_the_cursor_on_screen_under_the_uncommitted_row`, `update::tests::every_cursor_key_keeps_the_cursor_drawn`, `n_and_capital_n_keep_the_cursor_drawn_under_the_uncommitted_row`, `a_landing_under_the_uncommitted_row_keeps_the_cursor_drawn`, `a_dirty_tree_arriving_keeps_a_shown_cursor_shown`, `view::strips::tests::held_j_keeps_the_cursor_lit_under_the_uncommitted_row` green unmodified. Found by this test: `layers::on_screen` showed a lowered strip from row `top - 1`, which would land on the pinned row's line; fixed (`first = top - first_row`), `layers::tests::a_strip_holding_top_under_the_pinned_row_starts_at_top`.
- [x] `view::strips::tests::the_uncommitted_row_sits_on_head_in_its_strip` green **unmodified** (attached: the row stays in its strip).
- [x] `event_loop::graphics::pinned_tests::the_cursor_band_under_the_pinned_row_keeps_its_lines`: pinned image, cursor strip twice (rows `top..top+1` on line 1, the rest from line 3), band on line 2, no two images on one line.
- [x] `view::strips::pinned_tests::a_one_row_pane_draws_no_pinned_image`.
- [x] iTerm2: `event_loop::graphics::pinned_tests::the_pinned_row_is_its_own_band` (the same placements as Kitty; a one-row band in its own slot; `Graphics::frame` on iTerm2 ends `gfx ok` and encodes bands).
- [x] Bytes: `held_key::{held_j_stops_on_release_pixel, held_j_stops_on_release_pixel_free, j_then_journal_opens_at_once_pixel}` green in all 3 runs of `scripts/ease-20-held-pixel-bytes.sh` (`docs/perf/20-held-pixel-bytes.txt`, `# ease-20 ... [after run N]`). `bytes_per_frame_held` (A 40 MB/s / A free / B): run 1 466,032 / 426,548 / 452,426; run 2 466,126 / 473,211 / 495,976; run 3 245,995 / 458,529 / 480,396. Worst 495,976 = 94.6 % of 524,288: tight, passes, budget not moved. Deflate test after: 18,037 B/held frame, 36 strip transmits (before 64,343 / 55). Before: 1.65-1.85 MB.
- [x] Text tiers untouched: `git diff --stat main..HEAD -- docs/shots` lists only `docs/shots/ease-pass/20-pinned-row*`; text/Unicode paths not edited (`view/strips`, `event_loop/graphics` are pixel-only; grep below).
- [x] `docs/shots/ease-pass/20-pinned-row-after.png` (5.9 KB) + `20-pinned-row.md` with the "Right looks like" line. **Judged by the user 2026-10-24: "yes" (merge approved on the before/after PNGs).** The md states a measured difference the ruling did not expect (NOTES 1).
- [x] `ghostty_shot.rs` / `pieces_shot.rs` PNGs not re-shot here. See NOTES 3.
- [x] Touches: NOTES.
- [x] Gates workspace-wide: fmt, clippy, clippy --features heavy-tests clean; nextest 3759/3759 (7 skipped); doc tests 3/3.

Size: one context. Files: `crates/mergewright/src/view/strips.rs` (+ tests), `crates/mergewright/src/event_loop/graphics.rs`,
`graphics/layers.rs`, `graphics/tall.rs`, the iTerm2 image path; `mergewright-graph` raster only if the one-row image
needs an entry point (name it in NOTES; no key-shape change). For the conductor after merge: gap row
M7/46-kitty-no-deflate takes the new raw numbers (tail/j may still miss; bytes no longer the reason).

## NOTES (builder, ease-20)

Scout: none. The debugger's NOTES name every line and the ruling designates them as the scout (ADR-0035); no named line had moved. No compaction.

**1. One thing the ruling got wrong (flag, not a silent deviation).** The ruling says the pinned row is "drawn identically" alone in a one-row image. It is not, at HEAD, whenever a history row sits above `top`: the pen draws a row's upward stub when the row above goes down its lane (`pen.rs:261`), and at HEAD the virtual row's "row above" was the real history row `top - 1`, so the pinned row (a lane-less node on the screen's first line) got a short grey stub into its top edge (measured: 9 pixels, x 7-9 y 0-3 of the capture; at the test cell, pixel rows 0-2 x 3-5). That stub is the same mistake the ruling fixes for the row under it: a line from a row that is off screen. A one-row image cannot draw it, and drawing it needs `above` in the key, which is the re-key this ticket removes. So the pinned row loses it. The golden test pins the virtual row with no row above (what the ruling means by "identical"). The user sees: dashed ring with no tick at its top edge. `docs/shots/ease-pass/20-pinned-row.md` says so.

**2. Behaviour.** Attached to `HEAD` (HEAD on the top visible row): unchanged, the row sits in `HEAD`'s strip (its index does not move while attached). Detached (the case that scrolled): `PixelRows.pinned`, a one-row `StripRows` at `first_row` 0 (aligned for the cache, one row, `Ring::Uncommitted`, so its key cannot equal a history strip's). `graphics::drawn` places it on line 0 in `PINNED_SLOT = usize::MAX` (Kitty finds a strip by `first_row`, so slot 0 would have freed strip 0's image each frame). `pixel_rows` was over clippy's 60 lines: `view/strips/ranges.rs` (new) holds `strip_ranges`, `aligned`, `cursor_row_on_its_way`, `put_uncommitted`, `place_uncommitted`; `strips.rs` 404 -> 370, so its `over_400.txt` row is deleted, no row raised. `pixel_scroll.rs` (604): one doc line rewrapped so the new `mod pinned;` does not grow it. Bug found by the new tests: `layers::on_screen` (above).

**3. Ticket 21.** Both frames have a pinned row (`pieces_shot` printed `pinned: true`; `ghostty_shot` has `uncommitted_on_top`). Both PNG tests were already red before this ticket and stay red: `ghostty_shot` drawn 1720 B (same as 21 recorded, committed 10128), `pieces_shot` drawn 28,111 B now (21 recorded 29,756; committed 34,406). The pieces crop (cursor +-10 strip rows) no longer contains the pinned row, which sits outside the strips: 21 re-shoots with that in mind, and should add the pinned row to its stack. My `ghostty_shot.rs` changes, only these two: (a) `stack()` takes `frame.pinned` first (a `None` row, width, pixels) and sizes the cache `+ 1`; `Stack` and `stack` and its fields are `pub(crate)` so `pinned_row_shot.rs` reuses them; (b) new `under_the_pinned_row()` drops the stack rows between the pinned row and `top` before `composed` crops, so it reads as before (without it `composed` panicked on a slice). `pieces_shot.rs` untouched. `MAP.md`: one line added under `## view::strips`.

**Touches:** `view/strips.rs` (pixel_rows, PixelRows.pinned), `view/strips/{ranges,pinned_tests}.rs`; `strips/tests.rs:114-130` unmodified, green; `event_loop/graphics.rs` (`drawn`), `graphics/layers.rs` (`on_screen`; the four old on_screen tests green unmodified), `graphics/tall.rs` (unchanged, driven by the new tall tests), iTerm2 path (`ItermImages` unchanged; slot is a new `first_row`, band key per `place`), prefetch (skips the pinned strip: one-row, cheap, only strips are prefetched), `tests/it/ghostty_shot.rs` (`stack` takes `pinned`), `tests/it/pieces_shot.rs` (reads `uncommitted_at()` only; see result), `MAP.md`. Text tiers: not edited; grep `uncommitted_on_top|uncommitted_at` hits only the six files above and `docs/shots/m7/fix-scroll-contact.md`.
