# m1-the-graph/40 — Pixel tier: folded edges are fading tails, the rail is a dotted line

Status: ready
Build with: **Opus** (high) — drawing in the rasteriser (gradients, Béziers); the approach is decided, the result is judged by eye, so not Fable
Needs: 39 (`DrawnRow`, `FoldedEdge`, `Toward`, `raster::Stub`, `RasterRow::stubs`); 42 (`stroke_width`, `OFF_THREAD_ALPHA`, `FADED_ALPHA`)
Blocked by: 39, 42
Leaves runnable: yes — the pixel tier draws the same fold vocabulary as the text tiers.

Finding (user gallery walk, 2026-09-13, git/git, same screenshot): "Second-parent curves end in dangling stubs at the compaction boundary — ambiguous on every row." Decision 1 (ADR-0022): a folded edge is a stub on its node, the marker is a rail, no digits. 39 built the model and the text tiers; this is the pixel half.

## Read first
- `docs/design-notes/m1-the-graph.md`: decision 1, "New UI" table (Pixel column), "Interfaces" raster lines
- `docs/adr/0022-…` Decision 1–2 and Consequences (bitmap font, cache key)
- Ticket 22 "Notes" (rasteriser API, snapshot layout, `write_shots`), ticket 39 notes, ticket 42 notes (stroke and alpha constants)
- `crates/graft-graph/MAP.md` (raster, raster::cache), `crates/graft/MAP.md` (view::strips); scouts for `raster.rs`

## Scope
In:
- `Marker { column }`: drop `folded`; remove the 3×5 bitmap font and every digit path. `view::strips::pixel_rows` and `layout_hash` follow.
- Rail: a 1 px dotted vertical centred in the marker column on every row of the strip (dash `cell_height/6` on, `cell_height/6` off, phase continuous across strips so the pattern doesn't jump at strip edges), in `Strip.muted`.
- Stubs: for each `RasterRow::stubs` entry, a cubic tail leaving the node's right edge, curving right-and-down (`Below`) or right-and-up (`Above`), spanning one lane column horizontally (`CELLS_PER_LANE × cell_width`) and half a row vertically; stroke `stroke_width(cell_width)`; alpha runs from the lane's alpha (full on the thread, `OFF_THREAD_ALPHA` off it, `FADED_ALPHA` on a faded row) down to 0 at the far end — a tiny-skia linear-gradient shader along the tail, or at least six segments stepping alpha, whichever bands less at 17×37. The tail never reaches the neighbouring column's centre line. Several stubs on a row → one tail, first stub's colour.
- Pass-through verticals in the marker column: still not drawn (22's rule).
- Raster snapshot `raster-folded-merge` at 8×16: one `Below` stub, one `Above` stub, a rail; alpha/slot text art like the others.
- `docs/shots/40-raster-folded-on-dark.png` at 17×37 via `write_shots`; regenerate `22-*` shots that change.
- KNOWN_GAPS: close the 39 → 40 row.
Out: text tiers (39); stroke width and alpha constants (42); Kitty/iTerm2 transport; the strip cache's policy.

## Signatures
```rust
// graft-graph::raster (changed)
pub struct Marker { pub column: usize }
// consumed from 39 and 42
pub struct Stub { pub identity: BranchIdentity, pub toward: Toward }
pub(crate) fn stroke_width(cell_width: u32) -> f32;
pub const OFF_THREAD_ALPHA: f32; pub const FADED_ALPHA: f32;
```

## User check before close (user, 2026-09-13, ADR-0022)

- [ ] Capture git/git `seen` at **120 columns** after 39 and 40 (both tiers where they apply) under `docs/shots/<NN>-seen-120-*`; the user judges by eye whether it reads as **history or decoration**. 39 and 40 stay open until the user has judged it.
- Report: cause fixed or symptom hidden, and what it costs the user.

## Acceptance
- [ ] Buffer assertions at 17×37: the tail's alpha at its last pixel column is < 0.1 × its first; the tail's pixels stay left of the next column's centre; every rail pixel has zero chroma (R=G=B within 1) and the on/off pattern repeats at `cell_height/3`.
- [ ] No digit glyph code remains (the font module is gone); `Marker` has no count.
- [ ] Snapshot `raster-folded-merge` committed; changed `22-*` snapshots regenerated with the reason in the notes.
- [ ] `docs/shots/40-raster-folded-on-dark.png` committed; the user runs `cargo run --release -p graft -- ~/.cache/graft/git-git.git` in Ghostty and judges by eye. Report paths only.
- [ ] TESTING.md row: "Ghostty, git/git, pixel tier: each merge node has a short tail that fades out to its right; the rightmost graph column is a faint dotted line; no number is drawn in the graph; no curve crosses other lanes to reach the dotted line."
- [ ] MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
