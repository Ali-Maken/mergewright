# m1-the-graph/22 — The pixel graph rasteriser

Status: ready
Build with: **Fable** (high) — algorithm: anti-aliased curves and nodes from lane geometry
Blocked by: 05
Leaves runnable: yes — pure `graph` code with buffer snapshots; nothing reaches the terminal until 24.

Why: the spike drew chunky straight segments. The pixel tier is only worth having if it looks clearly better than Unicode.

## Read first
- PROMPT.md §1.1 (the pixel tier paragraph) and §7.2 item 3
- DESIGN.md §3.8 (the paragraphs about `RasterFrame`, not the cache)
- ADR-0003 (transparency) and ADR-0012 (spike findings only)
- Code: public API of `crates/graft-graph/src/layout.rs`; grep `RasterFrame` for any existing type; 14's fold description

## Scope
In:
- A pure rasteriser in `graph`: a row range, cell size and lane colours in, RGBA out. No I/O, no cache.
- Anti-aliased lane lines, Bézier curves where branches fork and merge, circular nodes with a distinct ring per commit kind.
- Real alpha: never paint the terminal background.
- The `+N` column and the paint attribute (dimmed / highlighted).

Out:
- The strip cache (23). Any escape-sequence encoding (24–26).

## Acceptance
- [ ] RGBA buffer snapshots at a pinned 8×16 cell for criss-cross, octopus and foxtrot.
- [ ] A pixel outside every line and node has alpha 0 (asserted on the buffer).
- [ ] Commit kinds differ in ring shape, asserted on the buffer independent of colour.
- [ ] Render time for a 32-row strip at a 17×37 cell (the user's measured size) reported with the machine.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
