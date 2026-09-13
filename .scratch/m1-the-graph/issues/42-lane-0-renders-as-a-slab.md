# m1-the-graph/42 — Lane 0 is a line: one stroke width, emphasis by alpha

Status: ready
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
- [ ] Notes state the measured widths before and after (px, on-thread and off) and the confirmed cause.
- [ ] Test at 17×37: on a plain row, the alpha > 0.5 run of any lane line across x is ≤ 3 px, thread or not; an off-thread lane's peak alpha is within 0.05 of half an on-thread lane's.
- [ ] Test at 8×16: `stroke_width` is 1.5; at 24×48: 2.5.
- [ ] `docs/shots/42-*.png` committed; user judges by eye. Report paths only.
- [ ] TESTING.md row: "Ghostty, git/git, pixel tier, cursor on the top row: lane 0 is the same thickness as every other lane and brighter than them; nothing in the graph is wider than a node ring's stroke."
- [ ] Snapshots regenerated with reasons; KNOWN_GAPS and MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens.
