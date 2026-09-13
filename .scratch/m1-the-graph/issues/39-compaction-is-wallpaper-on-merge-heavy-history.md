# m1-the-graph/39 — Compaction draws stubs and a rail; the count moves to the header

Status: draft (until the user approves ADR-0022)
Build with: **Opus** (high) — changes the fold model's output shape that 40 builds on; property tests and snapshots judge it, so not Fable
Needs: none
Blocked by: user approval of ADR-0022
Leaves runnable: yes — text tiers draw the new fold vocabulary; the pixel tier draws no folded edges until 40 (KNOWN_GAPS row, expiry 40)

Finding (user gallery walk, 2026-09-13, git/git, `docs/shots/user-37-git-git-pixel-after-scroll.png`): "Compaction is wallpaper: every row shows +54." and (40) "Second-parent curves end in dangling stubs at the compaction boundary." Both have one cause and one fix: `+N` is a per-window number printed per row, and every folded lane is nodeless in its window, so a curve into the marker says nothing. Ticket 40 is the pixel half; this is the model and the text tiers.

## Read first
- `docs/design-notes/m1-the-graph.md`: "What the screenshot says", "Interfaces and data shapes" (graft-graph::fold, graft-ui lines), decision 1, "New UI" table
- `docs/adr/0022-…` (Decision 1–2, Consequences)
- DESIGN.md §3.5 (what this supersedes; the ranking rule, which stays)
- Ticket 14 "NOTES" (the rule, the tests, the shots to compare against)
- `crates/graft-graph/MAP.md` (fold, row, raster, raster::cache), `crates/graft-ui/MAP.md` (graph_column, graph_screen), `crates/graft/MAP.md` (view, view::strips); scouts for source

## Scope
In:
- `graft_graph::fold`: `Toward`, `FoldedEdge`, `DrawnRow`; `Fold::view` returns `DrawnRow` (borrowed row when `is_empty()`, remapped copy otherwise); folded edges are removed from the drawn row's `outgoing`/`incoming` and listed in `folded`; `row.lanes[marker]` is always `None`; `Fold::folded_count`.
- `graft_graph::raster`: define `Stub` and `RasterRow::stubs` now (interface-first; 40 draws them); `raster::cache::layout_hash` covers `stubs`. `Marker.folded` stays fed from `folded_count()` until 40 removes it.
- Unicode `GraphColumn::through`: stub glyph in the gap cell right of the node (`STUB_BELOW` `▾` for `Toward::Below`, `STUB_ABOVE` `▴` for `Above`), coloured by `Theme::branch(identity)`; several folded edges → one glyph, first edge's colour. Rail `RAIL` `⋮` in `text-muted` on every row of the marker column; no `+N` text anywhere in the column. ASCII table: `v`, `^`, `:`. Dash parity (13) untouched.
- Header: after `Filters::text`, ` · +N lanes` while `folded_count() > 0` and not expanded; nothing otherwise. The screen plans the fold, so the screen appends it.
- `view::strips::pixel_rows` adapts to `DrawnRow` (passes stubs into `RasterRow::stubs`).
- KNOWN_GAPS: "pixel tier draws no folded edges (39 → 40)"; "clicking the rail does not expand; `+` does".
- MAP.md for every public change.
Out: pixel drawing (40); the width-aware cap (`lane-cap-follows-width`); rail click; fold ranking.

## Signatures
```rust
// graft-graph::fold
pub enum Toward { Below, Above }
pub struct FoldedEdge { pub identity: BranchIdentity, pub toward: Toward }
pub struct DrawnRow<'a> { pub row: std::borrow::Cow<'a, Row>, pub folded: Vec<FoldedEdge> }
impl Fold {
    pub fn view<'a>(&self, row: &'a Row) -> DrawnRow<'a>;
    pub fn folded_count(&self) -> usize;
}
// graft-graph::raster
pub struct Stub { pub identity: BranchIdentity, pub toward: Toward }
pub struct RasterRow { /* existing fields */ pub stubs: Vec<Stub> }
// graft-ui::graph_column
pub const STUB_BELOW: char = '▾';
pub const STUB_ABOVE: char = '▴';
pub const RAIL: char = '⋮';
```
Callers to update: `GraphColumn::through` / `width_through`, `view::strips::pixel_rows`, `raster::cache::layout_hash`, `GraphScreen` header.

## Acceptance
- [ ] Property test: for every row of a random window, kept edges plus `folded` equal the original edges (none lost, none doubled); no folded lane's column appears in the drawn row; `row.lanes[marker]` is `None`.
- [ ] `Fold::is_empty()` → `Cow::Borrowed` and `folded` empty (test).
- [ ] `layout_hash` differs when a row gains a stub (test).
- [ ] Widget snapshots (truecolor, 16, NO_COLOR, ASCII) on the many-lanes fixture: a folded merge row reads `◉▾`, every row's last column is `⋮`, no digit in the column; expanded: no rail.
- [ ] Screen snapshot: header ends `+N lanes` while folded; absent when expanded or nothing folds.
- [ ] pty captures, release binary, as 14 did: `docs/shots/39-git-git-80x24.txt`, `39-git-git-120x40.txt`, `39-many-lanes.txt`, `39-many-lanes-expanded.txt`. User judges by eye.
- [ ] TESTING.md row: "git/git at 80×24: header ends `+N lanes` (N ≥ 40); each merge row shows `◉▾` right of its node; the last graph column is `⋮` on every row; no `+` digits in any row; no curve reaches the `⋮` column."
- [ ] KNOWN_GAPS and MAP.md updated. Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting; the seam is "fold model + tests green" / "widgets + header + captures".
