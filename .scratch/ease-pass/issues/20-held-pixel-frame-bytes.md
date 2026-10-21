# 20 — Held-key pixel frames write 1.8–4.6 MB each

Status: decision needed · Build with: debugger (Opus high) · Blocked by: none
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
