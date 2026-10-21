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

- [ ] Step 0, before any code change: `docs/shots/ease-pass/20-pinned-row-before.png` — the top 6 pane rows of the
  pixel graph column with the uncommitted row pinned (`TeamSpec::MONOREPO`, 187×50, after `j`×60, in-process as
  `ghostty_shot.rs` draws, stand-in dark background), ≤200 KB. Drawn at HEAD 3123840e.
- [ ] Red first, then green: `view::strips::tests::a_pinned_uncommitted_row_leaves_strip_keys_alone` — `PixelRows`
  at `top = t` and `top = t + 1`, both pinned, same fold and roles: every strip present in both has an equal cache
  key. Red at HEAD (the strip holding `top` differs), green after.
- [ ] Red first, then green: `pixel_scroll::a_pinned_uncommitted_row_resends_no_strip_while_scrolling` — 46c's
  test shape with a dirty working tree and `HEAD` scrolled off: after the first frame, `j` steps inside the cached
  strips with the fold unchanged transmit zero strips; the pinned row's image is transmitted once and placed by id
  after. (Roles: hold the focus branch fixed in the fixture, or count role re-keys apart; say which.)
- [ ] `view::strips::tests::the_pinned_row_image_matches_the_virtual_row` — the pinned one-row image's pixels equal
  the virtual row's band as HEAD 3123840e rasterises it (golden bytes taken at step 0 and committed with the test).
- [ ] Red first, then green: `view::strips::tests::under_the_pinned_row_the_top_history_row_is_its_real_self` — the
  top history row's band under the pinned row equals that row's band rasterised in its strip with no uncommitted row.
  Red at HEAD confirms the debugger's premise (`pen.rs:261` reads the virtual row). **If it is green at HEAD, stop
  and report**: the premise is wrong and the visible-change paragraph above is void.
- [ ] `view::strips::tests::the_pinned_row_keeps_every_screen_line` — pinned row on pane line 0, history row `top` on
  line 1, …, each equal to `history_row_line`; the mouse hit-test lands on the same rows. Plus 48's
  `scroll-cursor-under-uncommitted-row` test(s) green unmodified (name them).
- [ ] `view::strips::tests::the_uncommitted_row_sits_on_head_in_its_strip` (`strips/tests.rs:114-130`) green
  **unmodified**: attached to `HEAD` on screen, the row stays in its strip (its index does not move with scroll).
- [ ] `event_loop::graphics::tall::tests::the_cursor_band_under_the_pinned_row_keeps_its_lines` (name to fit the
  module) — cursor on the top history row with its second line (`TallRow`): pinned image, cursor strip twice,
  continuation band, strips below one row lower; no two images on one line.
- [ ] `view::strips::tests::a_one_row_pane_draws_no_pinned_image` — 48's rule: a 1-row pane shows the cursor row,
  uncommitted row unpinned; no pinned placement.
- [ ] iTerm2: `event_loop::graphics::iterm2::tests::the_pinned_row_is_its_own_band` — the pinned row drawn as its own
  `ItermImages` band over the same line; strips below lowered as on Kitty.
- [ ] Bytes: `held_key::{held_j_stops_on_release_pixel, held_j_stops_on_release_pixel_free,
  j_then_journal_opens_at_once_pixel}` green, 3 debug runs each from `scripts/ease-20-held-pixel-bytes.sh`, appended
  to `docs/perf/20-held-pixel-bytes.txt` as `# after` with `bytes_per_frame_held` per run. Budget unchanged. Any run
  over 512 KiB → box open, report the numbers; do not raise it (budget is the conductor's). Also record the deflate
  test's `bytes_per_frame_held` and `strip_transmits_held` after (expected lower; reported, not judged).
- [ ] Text tiers untouched: every Unicode/text capture test green with no shot file rewritten (`git diff --stat
  docs/shots` lists only this ticket's PNGs).
- [ ] `docs/shots/ease-pass/20-pinned-row-after.png`, same frame as step 0, ≤200 KB, and a contact-sheet entry
  (`docs/shots/ease-pass/20-pinned-row.md`): before above after, under the line *"Right looks like: the pinned
  uncommitted row identical in both; in the after, the history row directly under it has its lanes and its commit's
  upward line running to the top edge of the row; every other row identical. Colour unverified."* Judged by the
  user, not ticked by the builder's eye.
- [ ] `ghostty_shot.rs` / `pieces_shot.rs` PNGs: not re-shot here (ticket 21 owns them, runs after this). NOTES
  say whether either frame has a pinned row, so 21 knows what changes.
- [ ] Touches: line covering every file in the debugger's list, each with how it was checked.
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
  `cargo test --doc`.

Size: one context. Files: `crates/mergewright/src/view/strips.rs` (+ tests), `crates/mergewright/src/event_loop/graphics.rs`,
`graphics/layers.rs`, `graphics/tall.rs`, the iTerm2 image path; `mergewright-graph` raster only if the one-row image
needs an entry point (name it in NOTES; no key-shape change). For the conductor after merge: gap row
M7/46-kitty-no-deflate takes the new raw numbers (tail/j may still miss; bytes no longer the reason).
