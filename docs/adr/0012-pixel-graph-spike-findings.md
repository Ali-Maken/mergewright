# Pixel graph: transmit-once + crop for Kitty, re-send for iTerm2

> **Corrected 2026-09-12.** The live Ghostty run showed a blank graph. The
> bullets marked ⚠ below were wrong; the Correction section at the end
> sets them right. The decision itself stands.

Findings from the graph-drawing spike (`prototypes/graph-spike/`,
PROGRESS.md step 3): a hard-coded 30-commit, 3-lane graph with merges,
drawn as one RGBA strip, aligned with text rows, scrolled with `j`/`k` on
the Kitty graphics and iTerm2 inline-image rendering tiers. Verified at the
escape-stream level under a pty for both tiers, plus a `--snapshot` PNG
geometry check. Both tiers were then confirmed to actually draw in a real
terminal — Ghostty at the step-3 checkpoint, iTerm2 by the user on
2026-09-12 (see the Correction and the iTerm2 section below).

Decision: **the pixel-graph approach in docs/design/03-graph-layout-algorithm.md §3.8 stands.** Scrolling
is a placement-crop on Kitty and a full re-send on iTerm2, and text/pixel
alignment comes from placing images into a *cell* rectangle, not from
pixel-perfect cell-size knowledge.

## What worked

- **Alignment by cell-rect placement.** The strip is rasterized at
  `rows × cell_h` pixels, one band per terminal row, but the placement
  names a cell rectangle (Kitty `c=…,r=…`; iTerm2 `width=…;height=…` in
  cells with `preserveAspectRatio=0`). The terminal scales the image to
  exactly that many cells, so alignment survives even the assumed-cell-size
  fallback. Cell size affects sharpness, never registration.
- **Kitty: transmit once, then crop.** One `a=t,f=32` upload of the whole
  strip (base64 in 4096-byte chunks, `m=1`/`m=0`), then each frame is only
  `a=p` with a source rect whose `y` moves by exactly `cell_h` per scroll
  step. Captured stream confirms: 168,000 RGBA bytes uploaded once, then
  five placements with `y = 0,20,40,60,40` for `j j j k`. This is
  docs/design/03-graph-layout-algorithm.md §3.8's image-reuse idea working in miniature.
  ⚠ *Overstated — the capture proved the bytes were emitted, not that
  anything rendered. In Ghostty the pattern as written drew nothing. See
  Correction.*
- **iTerm2: re-send per frame is affordable at viewport scale.** The
  protocol has no image ids or crops, so every frame re-rasterizes and
  re-encodes the visible band. Measured on a 23-row viewport: 0.4 ms
  raster + 6.9 ms PNG encode, ~13.5 KB per keystroke. Fine for a viewport;
  never do it for whole histories.
- **Cell-size probe chain.** `CSI 16 t` → reply `CSI 6 ; h ; w t`
  (height first) → fallback to `window_size()` pixel division → fallback
  to an assumed 10×20 flagged `(assumed)` in the status line. Provenance
  in the UI made the fallback path debuggable.
- **Kitty detection by round-trip, not env.** A dummy `a=q` query fenced
  with DA1 (`CSI c`): if `ESC _ G` comes back before the DA1 reply, the
  tier is real. iTerm2 is the one tier detected by env alone
  (`TERM_PROGRAM`), per docs/design/04-terminal-capability-detection.md §4.1.
- Transparent RGBA canvas with overwrite (no blending): merge rings punch
  through to the terminal background, as ADR-0003 requires.
- DEC 2026 synchronized-output brackets around every frame; placement
  delete (`a=d,d=i`) per frame and free-all (`a=d,d=A`) on exit left the
  terminal clean.
  ⚠ *Wrong — "clean" was the bug. The frame's `ESC[2J` freed the image
  data in Ghostty, every later placement silently failed, and the "clean
  terminal" was a graph-less one. See Correction.*

## What didn't

- **Probes eat typeahead.** Keys arriving during the ~300 ms probe window
  are consumed by the reply reader and lost (piped input hung the first
  smoke test). The spike's reader keeps only the bytes it recognizes.
- **Degenerate window sizes break layout silently.** A pty reporting 0×0
  clamped the view to one row and the text budget to one byte. The spike
  neither re-probes on resize nor guards small sizes.
- **crossterm's event system can't own stdin during probes.** Its parser
  swallows APC/DA1 replies, so the spike bypasses it with raw
  `poll()`/`read()` — workable, but two readers on fd 0 is a hack.
- The rasterizer has no anti-aliasing and only straight lane segments;
  visibly chunky at 1× cell scale.
- Frames 1 and 3 of the iTerm2 capture were byte-identical PNGs encoded
  twice — no cache, wasted 7 ms.

## What M1 must do differently

- Fixed-height strips with ids and LRU eviction (docs/design/03-graph-layout-algorithm.md §3.8), never
  one whole-history image; the terminal retains every uploaded pixel.
- One input-mux task owns stdin: probe replies routed to the prober,
  everything else preserved as key events. No typeahead loss.
- Re-probe cell size and rebuild strips on SIGWINCH; treat `(assumed)`
  cell size as degraded, not equal.
- Guard tiny/zero terminal sizes explicitly.
- Anti-aliased drawing and curved lane transitions.
- Cache encoded PNG bands by (offset, viewport) on the iTerm2 tier.
- Keep: cell-rect placement, probe chain with provenance, transmit-once +
  crop on Kitty, ~~delete-placements-then-place per frame~~ *(corrected:
  replace one fixed placement id per frame; never `ESC[2J` between
  transmit and placement)*, free-all on exit.

## Correction (2026-09-12)

The step-3 checkpoint failed: in Ghostty 1.3.1, no graph at all — text
only, status bar showing `raster 0.0 encode 0.0`. This section records
what the original findings got wrong and what was actually true.

### What the pty captures did and did not prove

The captures proved exactly one thing: the spike *emitted* well-formed
escape sequences in the intended order. They proved nothing about
rendering — a pty has no graphics implementation, so it accepts any byte
stream, including one a real terminal rejects on every frame. "Verified at
the escape-stream level" was treated as acceptance evidence; it must never
be again. Only a real terminal answering the graphics commands (or human
eyes) verifies rendering.

### The actual failure, root-caused in a real Ghostty

Reproduced by replaying the spike's stream from a harness *inside a
Ghostty window* with `q=0` (all replies logged):

1. Each frame began `ESC[?2026h ESC[2J`, then deleted placements and
   placed a crop, with `q=2` on every graphics command.
2. **Ghostty frees kitty image *data* on ED (`ESC[2J`)** — not just
   placements. Even a transmitted-but-never-placed image is gone after a
   clear. Every placement after the first frame's clear answered
   `ENOENT: image not found`.
3. `q=2` suppressed every one of those replies. The failure was invisible
   by construction — the spike asked the terminal not to report errors,
   then treated silence as success.
4. `raster 0.0 encode 0.0` was a red herring: legitimate on the Kitty
   path, where scroll frames only place a crop (raster cost is paid once
   at transmit; encode is iTerm2-only). The numbers were honest; the
   acceptance evidence was not.

Also checked: the spike uses **direct placement** (`a=p` at the cursor),
not U+10EEEE unicode placeholders. Ghostty supports direct placement fine
(probe and placements ACK `OK` at `q=0`); placeholders were never needed
and are not the issue.

### What actually renders in Ghostty

- Transmit the strip **after every full clear**, not "once per session".
  Steady-state frames must not emit `ESC[2J` at all; the spike now clears
  only when the terminal size changes, and immediately retransmits.
- Replace a **single fixed placement id** (`p=1`) each frame instead of
  delete-then-place; the terminal swaps the crop atomically.
- Run graphics commands at **`q=1`** (silent on success, errors
  reported); the spike now reads APC error replies off stdin and shows
  them in the status line. `q=2` hid this entire bug and is banned from
  the frame path.
- Text rows erase their own tails (`ESC[K` starting right of the text)
  rather than relying on a full clear; erasing cells under a kitty image
  can delete it.

Verified 2026-09-12 in a real Ghostty 1.3.1 window: probe ACK, one
transmit, cell-rect placement at `q=1`, **zero error replies** — where
the pre-fix stream answered ENOENT on every placement. Pixel appearance
and scroll alignment still require human eyes at the checkpoint.

### Amendments to "What M1 must do differently"

- Never assume transmitted image data survives a screen clear. Treat any
  ED/alt-screen transition as invalidating uploads; retransmit after.
  (Whether this is Ghostty-specific or general kitty-protocol behavior is
  unknown — assume the worst.)
- Never run steady-state graphics at `q=2`. Use `q=1` and route APC error
  replies through the input mux to a visible channel.
- Acceptance evidence for rendering tiers is a real terminal's replies or
  a human looking at it — never a pty capture.

## Follow-up (2026-09-12): measuring the fixed spike

The corrected spike renders in Ghostty, so the next question was cost. The
status line read `frame 24.9ms (raster 2.0 encode 0.0 upload 23)`, which
looks like a per-frame re-upload — the very optimisation the ENOENT fix
was suspected of dropping. It was not. Two separate things were wrong, and
neither was the transmit policy.

### How it was measured

A pty cannot time a frame: it drains writes instantly, so the write
back-pressure that dominates a real graphics frame disappears. The spike
grew a **bench mode** instead — `GRAPH_SPIKE_BENCH=<frames>` scrolls
itself back and forth, paced by `GRAPH_SPIKE_BENCH_HZ` (default 30, `0` =
flat out), times each frame *including the final flush*, drains Kitty
`q=1` error replies every frame, and writes a report to
`GRAPH_SPIKE_BENCH_OUT`. Run it inside a real terminal window and the
numbers are real. This is the honest version of "measure it"; a pty run
only proves the bytes are well formed (see the Correction above).

Real Ghostty 1.3.1, 122×40 cells, cell 17×37 px, 300 frames at 30 Hz:

```
transmits     1
graphics err  0 []
frame ms      first 41.4 | min 0.1 med 0.4 p95 0.5 max 41.4
over 16ms     1 of 300
retransmits   [(1, 122, 40)]   (frame, cols, rows)
steady ms     min 0.1 med 0.4 max 0.5   (frames 2..)
```

**One** transmit in 300 frames, and every other frame is a placement crop
at **0.4 ms median, 0.5 ms worst** — against a 16 ms budget. `graphics
err 0` says none of those 299 placements silently ENOENTed, which is the
check that matters: a crop into a freed image is exactly the bug the
Correction above fixed, and it fails silently. Transmit-once + crop
survived that fix intact.

An earlier run of the same bench caught a window still settling and read
`transmits 2, retransmits [(1, 122, 38), (242, 122, 40)]` — the second
transmit being a real 38 → 40 row resize, costing 32 ms. That is the
policy working, not leaking: the only thing that retransmits is a size
change.

### Defect 1: the status line lied, the renderer didn't

`upload` was carried forward from the last transmit, so every crop-only
frame displayed a cost it had not paid. `frame 24.9ms … upload 23` was a
resize frame's real cost (a retransmit *is* ~25–40 ms) shown under a
scroll frame's label. Fixed by making every number belong to the frame
being drawn — except `frame`, which is necessarily the previous frame's
total and is documented as such — and by adding a `tx N` counter to the
status line. A silent per-frame retransmit would now be visible as `tx`
climbing with the scroll. **Numbers that outlive their frame lie**; a
measurement you cannot attribute to one frame is not a measurement.

Retransmit-on-resize stays over budget (~32–41 ms) and is left alone here:
it is one frame per resize, and M1's fixed-height strips (docs/design/03-graph-layout-algorithm.md §3.8)
change the shape of that cost anyway.

### Defect 2: `MAX_VIEW = 24`

`rows 1-24/30` in a window with room for 39 was a leftover constant
capping the viewport, not a strip-height bug. Removed; the view is now
`rows − 1` (the status line owns the last row), clamped to the dataset.
The dataset went from 30 to 60 rows so a scroll range still exists in a
tall window — the tempting alternative, capping visible rows to keep the
scroll interesting, is exactly the bug.

### iTerm2 tier: verified by eye, 2026-09-12

**Verified.** The user ran the spike in a real iTerm2 on 2026-09-12 and the
graph drew correctly. That is eyes-on-pixels evidence — the standard this
ADR's Correction sets, and the only one that counts for a rendering tier.
Both tiers are now confirmed to draw: Kitty by Ghostty's own replies plus
the checkpoint, iTerm2 by the user looking at it.

This was written up as unverified while no iTerm2 existed on the
development machine. What was claimed then still holds and is what was
being looked at: the `ESC[2J` moved to the shared resize-only path (it had
been clearing every frame on that tier too), and PNG encoding switched to
`Compression::Fast` because encode time is frame budget there. Under a pty
at 122×40 it emits correct OSC 1337 args
(`inline=1;size=…;width=7;height=39;preserveAspectRatio=0`), ~60 KB per
frame, ~8.3 ms median CPU — raster ~1 ms, encode ~7 ms, write cost
excluded. `tx` counts every iTerm2 frame, because that tier really does
re-send the image every time.

Still unmeasured on real iTerm2: frame cost with write back-pressure
included. The ~8.3 ms above is pty CPU time, and a per-frame re-send of
~60 KB is the one tier where write cost could dominate. Bench it there in
M1 before trusting the number.

### Resize without retransmitting (m1-the-graph/25, 2026-09-13)

What frees the uploads is `ESC[2J`, not the resize. Checked in a real
Ghostty 1.3.1 (`crates/graft/examples/kitty_check.rs`, report
`docs/shots/25-kitty-resize-check-report.txt`): every row erased with
`ESC[2K`, then placements only — 0 errors; a full `ESC[2J`, then placements
only — one `ENOENT` per strip. ratatui clears with `ESC[2J` on a size
change, so the app's backend (`event_loop::row_clear::RowClear`) turns that
clear into per-row erases. A resize now re-places uploaded strips and
rasterises only strips whose cache key changed; the loop no longer flushes
the strip cache on resize (lane columns are in the layout hash). Measured in
memory, release build: resize frame 0.30 ms (Kitty), 0.25 ms (iTerm2) —
terminal write cost excluded.

iTerm2 bands are cached by (strip pixels, crop, cell rectangle): a frame
whose bands did not change re-sends the stored bytes and encodes nothing.

