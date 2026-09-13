# m1-the-graph/42 — Lane 0 is a line: one stroke width, emphasis by alpha

Status: done
Build with: **Sonnet** (medium) — confirm a cause, then constants and snapshots; the decision is ADR-0024. Escalate per CLAUDE.md if the cause is elsewhere in the rasteriser.
Needs: none (40 needs this)
Blocked by: 38
Leaves runnable: yes — every graph line in the pixel tier is the same width; the thread is the full-colour one.

Finding (user gallery walk, 2026-09-13, pixel tier, Ghostty, same screenshot): "Lane 0 renders as a thick slab, not a line to follow." Hypothesis (ADR-0024): the ancestry thread draws heavier, the cursor starts on the top row, so lane 0 is always on the thread and always heavy. Confirm before changing anything.

## Read first
- `docs/design-notes/m1-the-graph.md`: decision 3, "Interfaces" raster lines
- `docs/adr/0024-…`
- Ticket 22 "Notes" (`Emphasis`, `thread`, snapshot files, `write_shots` command), ticket 16 "Notes" (thread = bold/heavier, always on while selected)
- `crates/graft-graph/MAP.md` (raster); a scout for the stroke/emphasis code in `raster.rs`

## Scope
In:
- Cause first: rasterise a 5-lane strip at 17×37 twice, thread on lane 0 and no thread; measure lane 0's drawn width (pixels with alpha > 0.5 across x on a plain row) both ways and record the numbers in the notes. If the slab is not the thread's stroke (a double draw, a base width already over 2.5 px, a lane-0 rule), name that cause and fix it too.
- `stroke_width(cell_width) = (cell_width as f32 / 8.0).clamp(1.5, 2.5)`, used by lanes, curves and node rings alike. The thread does not widen.
- `OFF_THREAD_ALPHA = 0.5` for columns off the selection's thread; `FADED_ALPHA = 0.3` for `Emphasis::Dimmed` rows (Focus). `Emphasis::Highlighted` no longer widens; remove the variant if nothing distinguishes it from `Normal` afterwards, and update MAP.md.
- Regenerate raster snapshots that change; say why in the notes.
- Shots: `docs/shots/42-raster-thread-on-dark.png` (lane 0 on the thread among four others, 17×37) and `42-raster-rings-on-dark.png`, via `write_shots`. User judges by eye.
- KNOWN_GAPS "Focus and the ancestry highlight are bold/dim only": pixel half closes; Unicode half stays.
Out: the Unicode tier's bold/dim; stubs and the rail (40); ring radius.

## Signatures
```rust
// graft-graph::raster
pub(crate) fn stroke_width(cell_width: u32) -> f32;
pub const OFF_THREAD_ALPHA: f32 = 0.5;
pub const FADED_ALPHA: f32 = 0.3;
```

## Acceptance
- [x] Notes state the measured widths before and after (px, on-thread and off) and the confirmed cause.
- [x] Test at 17×37: on a plain row, the alpha > 0.5 run of any lane line across x is ≤ 3 px, thread or not; an off-thread lane's peak alpha is within 0.05 of half an on-thread lane's.
- [x] Test at 8×16: `stroke_width` is 1.5; at 24×48: 2.5.
- [x] `docs/shots/42-*.png` committed; user judges by eye. Report paths only.
- [x] TESTING.md row: "Ghostty, git/git, pixel tier, cursor on the top row: lane 0 is the same thickness as every other lane and brighter than them; nothing in the graph is wider than a node ring's stroke."
- [x] Snapshots regenerated with reasons; KNOWN_GAPS and MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.

## Notes

**Cause, confirmed before any change (a scout of `raster.rs` plus my own
pixel measurement, not the ticket's hypothesis taken on faith):**
`Pen::ink()` multiplied stroke width by 1.6x whenever a column's emphasis
was `Emphasis::Highlighted`, and `RasterRow::emphasis_at` (now `alpha_at`)
set every column on the selection's ancestry thread to `Highlighted`. The
cursor starts on row 0, whose thread is the whole of lane 0, so that
"highlight" is the graph's default render, not an occasional emphasis —
exactly ADR-0024's hypothesis. No divergence found in the ticket-30g
scanline-copy path (`Pen::straight`): it shares the same `Ink`, so ruled
out as a second cause.

**Measured, 17×37, a plain row's alpha>0.5 run in x (synthetic 5-lane
strip, `raster_pixels::synthetic`):**
| | before | after |
|---|---|---|
| lane with no thread | 3 px | 3 px |
| lane on the thread | 5 px (1.6x) | 3 px |
| off-thread lane peak alpha (thread present) | 90/255 (~0.35, the old dim) | 128/255 (0.50, `OFF_THREAD_ALPHA`) |
| on-thread lane peak alpha | 255/255 | 255/255 |

Post-fix, on-thread and off-thread lanes are the same width; only alpha
differs. Cost to the user pre-fix: lane 0 was never a line to follow, it
was the graph's permanent default state, because the cursor defaults there.

**Why snapshots changed:** `stroke_width` replaced
`(cell_width * 0.2).max(1.5)` with `(cell_width / 8.0).clamp(1.5, 2.5)`
(e.g. 8 px cell: 1.6 → 1.5 px), and node ring geometry derives from the
same `stroke` value (`NodeSize::for_cell`). Both text-art and pixel-digest
snapshots for `foxtrot-merge`, `criss-cross-merge`, `octopus-merge` and the
pixel-digest table moved by that amount; each diff was inspected before
`INSTA_UPDATE=always` and matched the expected width/ring change, nothing
else.

**`Emphasis::Highlighted` removed:** once emphasis is alpha-only, nothing
distinguished it from `Normal`/`Dimmed`, so it was deleted rather than kept
dead. `graft`'s `Painter::row` no longer sets a cursor-row emphasis at all
(the thread already draws at full alpha regardless of row emphasis) — see
`crates/graft/src/view/strips.rs`.

Shots: `docs/shots/42-raster-thread-on-dark.png` (5-lane synthetic strip,
lane 0 on the thread, 17×37, dark backdrop),
`docs/shots/42-raster-rings-on-dark.png` (node rings at the new stroke
width). User judges by eye.
