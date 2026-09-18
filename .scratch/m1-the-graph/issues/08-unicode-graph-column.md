# m1-the-graph/08 — The Unicode graph column

Status: done
Build with: **Opus** (high) — visual design drawn from `Row` data
Blocked by: 05
Leaves runnable: yes — a widget with snapshots; the app still opens on the status screen until 09.

Why: the user works in the Zed and VS Code terminals, which have no image protocol, so this is the graph he sees most days (ADR-0016). It must be designed, not degraded — never called a fallback. This ticket gets the shapes right at truecolor and 256; lower depths are 13.

## Read first
- PROMPT.md §1.1 (the Unicode tier paragraph) and §1.2 (colour rules only)
- PROMPT.md §5
- docs/design/01-view-mockups.md §1.3 (the narrow mockup's graph column) and docs/design/03-graph-layout-algorithm.md §3.1
- ADR-0016
- KNOWN_GAPS.md M1 row "A `Row` carries a lane index and no identity"; M0 row "Sanitising does not cover look-alikes, width, or the invisibles…"
- Code: public items of `crates/graft-graph/src/layout.rs` (`Row`, `Lane`), `crates/graft-ui/src/theme/` (lane palette)

## Scope
In:
- A `graft-ui` widget drawing the graph column from `Row`s: rounded box drawing (`╭ ╮ ╰ ╯ │ ─ ├ ┤`), fork and merge elbows that read as curves, pass-through lanes.
- Node glyphs by kind (commit, HEAD, merge, root, stash, uncommitted). Special commits differ in shape, never in colour alone.
- Lane colour from branch identity, stable across scrolls. Truecolor and 256 only.
- Settle the Row-identity gap: `Row` grows an identity field, or the renderer captures `active_lanes()`. Decide, write the reason in the ticket, close the gap row.
- A per-row paint attribute (normal / dimmed / highlighted) the widget honours; nothing sets it until 16.
- Clamp over-wide and combining-heavy cells so the column width is exact (closes the width half of the M0 sanitiser row).

Out:
- Text columns (10), badges (11), 16 colours and `NO_COLOR` (13), `+N` (14), ASCII (19), putting it on screen (09).

## Acceptance
- [x] `TestBackend` + `insta` snapshots for every corpus shape, graph column only, dark and light, truecolor.
- [x] You read criss-cross, foxtrot and the octopus snapshot row by row and they read as pictures; say what you changed after looking.
- [x] A lane keeps its colour across two viewports that overlap by half (test).
- [x] Colour never follows column index: test on a shape where a column is reused by a different branch.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Outcome

**Row identity: `Row` grew `lanes: Vec<Option<BranchIdentity>>`** — the identity drawn in each column on that row, captured in `LayoutCursor::assign_row`. Why not capture `active_lanes()` in the renderer: a renderer only ever holds rows (cached chunks rebuilt from checkpoints, a viewport starting mid-history), never the cursor state at the moment each row was made, so a capture would have to live beside every row anyway; and a root-that-is-also-a-tip closes its lane inside `extend`, where no outside caller can see it. One identity per column per row is unambiguous because a freed column is never reused on the freeing row. A reserved lane is left out until its tip lands, so no line runs up into empty space above `main`'s tip. Merge lanes always open a fresh column, so up/down for every cell derive from `lanes`, `incoming` and `outgoing` alone — no second field needed.

- Widget: `graft_ui::graph_column::{GraphColumn, GraphLine, NodeKind, RowPaint}`. Two cells per lane; glyph from connected sides, rounded corners; a lane crossed by a horizontal edge keeps `│` (hop, not join). Glyphs: commit `●`, merge `◉`, root `◇`, HEAD `◆`, stash `■`, uncommitted `◌`.
- Colour: `Theme::branch(identity)` = palette slot `BranchIdentity::slot(12)`.
- Width clamp: `graft_ui::fit::fit`.
- Snapshots: `crates/graft-ui/tests/snapshots/`, one per shape; dark and light asserted identical (lane palette is theme-independent), so one snapshot per shape instead of two identical ones.
- Capture: `docs/shots/08-unicode-graph-column.txt` (ANSI; `cat` it in a terminal). Example: `cargo run -p graft-ui --example graph_column [shape]`.

Read row by row (criss-cross, foxtrot, octopus, feature-branches, stash, ref-states): every edge lands where the layout says. What I changed after looking: the palette slot. `u64 % 12` of the raw FNV-1a hash, then a 16-bit fold, both left visibly bad pictures (the fold put every octopus lane and `main` in one hue; the top bits alone gave six one-letter names the same hue). `BranchIdentity::slot` now runs the hash through murmur3's 64-bit finaliser and takes the top 16 bits; a graph test pins that 26 one-letter names reach at least 10 hues, and 1,200 names spread uniformly. Still seen and logged, not hidden: adjacent lanes can share a hue by chance (feature-branches: `main`, `feature/two`, `feature/three`), foxtrot's merge lane *is* `main`, and HEAD hides merge-ness — KNOWN_GAPS.md. The user judges the look by eye.
