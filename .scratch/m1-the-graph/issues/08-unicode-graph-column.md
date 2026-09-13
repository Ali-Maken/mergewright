# m1-the-graph/08 — The Unicode graph column

Status: ready
Build with: **Opus** (high) — visual design drawn from `Row` data
Blocked by: 05
Leaves runnable: yes — a widget with snapshots; the app still opens on the status screen until 09.

Why: the user works in the Zed and VS Code terminals, which have no image protocol, so this is the graph he sees most days (ADR-0016). It must be designed, not degraded — never called a fallback. This ticket gets the shapes right at truecolor and 256; lower depths are 13.

## Read first
- PROMPT.md §1.1 (the Unicode tier paragraph) and §1.2 (colour rules only)
- PROMPT.md §5
- DESIGN.md §1.3 (the narrow mockup's graph column) and §3.1
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
- [ ] `TestBackend` + `insta` snapshots for every corpus shape, graph column only, dark and light, truecolor.
- [ ] You read criss-cross, foxtrot and the octopus snapshot row by row and they read as pictures; say what you changed after looking.
- [ ] A lane keeps its colour across two viewports that overlap by half (test).
- [ ] Colour never follows column index: test on a shape where a column is reused by a different branch.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
