# Pixel graph: transmit-once + crop for Kitty, re-send for iTerm2

Findings from the graph-drawing spike (`prototypes/graph-spike/`,
PROGRESS.md step 3): a hard-coded 30-commit, 3-lane graph with merges,
drawn as one RGBA strip, aligned with text rows, scrolled with `j`/`k` on
the Kitty graphics and iTerm2 inline-image rendering tiers. Verified at the
escape-stream level under a pty for both tiers, plus a `--snapshot` PNG
geometry check; a live run in Ghostty/iTerm2 happens at the step-3
checkpoint.

Decision: **the pixel-graph approach in DESIGN.md §3.8 stands.** Scrolling
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
  DESIGN.md §3.8's image-reuse idea working in miniature.
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
  (`TERM_PROGRAM`), per DESIGN.md §4.1.
- Transparent RGBA canvas with overwrite (no blending): merge rings punch
  through to the terminal background, as ADR-0003 requires.
- DEC 2026 synchronized-output brackets around every frame; placement
  delete (`a=d,d=i`) per frame and free-all (`a=d,d=A`) on exit left the
  terminal clean.

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

- Fixed-height strips with ids and LRU eviction (DESIGN.md §3.8), never
  one whole-history image; the terminal retains every uploaded pixel.
- One input-mux task owns stdin: probe replies routed to the prober,
  everything else preserved as key events. No typeahead loss.
- Re-probe cell size and rebuild strips on SIGWINCH; treat `(assumed)`
  cell size as degraded, not equal.
- Guard tiny/zero terminal sizes explicitly.
- Anti-aliased drawing and curved lane transitions.
- Cache encoded PNG bands by (offset, viewport) on the iTerm2 tier.
- Keep: cell-rect placement, probe chain with provenance, transmit-once +
  crop on Kitty, delete-placements-then-place per frame, free-all on exit.
