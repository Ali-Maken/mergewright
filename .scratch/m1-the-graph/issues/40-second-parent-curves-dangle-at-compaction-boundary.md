# m1-the-graph/40 — Pixel tier: folded edges are fading tails, the rail is a dotted line

Status: done — user check pending (40 capture)
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
- [x] Buffer assertions at 17×37: the tail's alpha at its last pixel column is < 0.1 × its first; the tail's pixels stay left of the next column's centre; every rail pixel has zero chroma (R=G=B within 1) and the on/off pattern repeats at `cell_height/3`.
- [x] No digit glyph code remains (the font module is gone); `Marker` has no count.
- [x] Snapshot `raster-folded-merge` committed; changed `22-*` snapshots regenerated with the reason in the notes.
- [x] `docs/shots/40-raster-folded-on-dark.png` committed; the user runs `cargo run --release -p graft -- ~/.cache/graft/git-git.git` in Ghostty and judges by eye. Report paths only.
- [x] TESTING.md row: "Ghostty, git/git, pixel tier: each merge node has a short tail that fades out to its right; the rightmost graph column is a faint dotted line; no number is drawn in the graph; no curve crosses other lanes to reach the dotted line."
- [x] MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## NOTES (build, 2026-09-13, builder-opus)

No compaction.

- Built: `Pen::tail` (cubic from node right edge `radius + gap`, half a band toward `Toward`, tiny-skia linear gradient along x, lane alpha → 0, one tail per row in the first stub's colour, drawn before the node disc); `Pen::rail` (1 px whole-pixel column, three dashes of `cell_height/6` from each band's top, muted, `OFF_THREAD_ALPHA` under a thread). `Marker { column }`; `raster/label.rs` (3×5 font) deleted; `view::strips` follows; `layout_hash` doc.
- Tests: `folded_merge_snapshot` (new `raster-folded-merge`, 8×16), `a_folded_edge_is_a_tail_that_fades_out_before_the_next_column` and `the_rail_is_a_grey_dotted_line_whose_dashes_never_jump` (17×37, incl. a second strip starting at row 2 matching the first's band). `strips/tests.rs`: `marker.folded > 0` → rail is the last drawn column. 798 pass.
- Snapshots: `22-*` text snapshots and PNGs unchanged (byte-identical; no marker in them). `raster-pixel-digests`: only `wide-marker` lines changed (digits → rail), expected.
- Differs from ticket (say so, user/planner judges): (1) tail spans ≈ `2·cell_width − clearance − stroke` (≈19 px at 17×37), not a full 34 px: starting at the node's right edge, a full lane width would cross the next centre line, which the ticket also forbids. (2) Rail sits on the marker column's lane centre line (where text tiers put `⋮`), not the middle of its two cells.
- Cause or symptom: cause — the edge into a folded lane no longer draws toward the rail; the fold model (39) hands stubs, 40 draws them. Cost to the user: the tail shows direction only, not where the folded branch goes; the count lives in the header.
- Frame time (release, uncounted, `target/40/frames-release.txt`, pixel tier 100k): p50 5.7 ms, p99 9.1 ms, max 12.1 ms, under 16.
- User check: `docs/shots/40-seen-120-pixel.png` (git/git at 120×40, release binary, Kitty images decoded and composited at 17×37 on dark, no text; `docs/shots/40-capture.py`). Script-checked only: 2 strips placed, graph pixels on screen rows 1–38. Not eyeballed. Also `docs/shots/40-raster-folded-on-dark.png`.
