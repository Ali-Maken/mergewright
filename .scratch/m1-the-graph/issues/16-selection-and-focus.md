# m1-the-graph/16 — Selection, ancestry highlight and Focus mode

Status: done
Build with: **Opus** (high)
Blocked by: 14
Leaves runnable: yes — selecting a commit lights up its ancestry; `F` turns it up.

Why: "where did this commit come from" should be answered by looking, not by reading.

## Read first
- docs/design/05-visualization.md §5.2 and docs/design/02-keymap.md §2.3 (selection keys only)
- Code: 08's paint attribute, 09's cursor, 14's protected-set parameter, public API of `crates/graft-graph/src/layout/stream.rs`

## Scope
In:
- Selection replaces 09's cursor: moves with the keys and stays on the same commit id across scrolls, arriving chunks and chunk eviction.
- The selected commit's ancestry is highlighted and unrelated lanes are dimmed, through 08's paint attribute: O(visible rows) per selection change, never a history pass.
- Focus mode (`F`): the same mechanism turned up.
- The ancestry set is passed to 14 as its protected set.

Out:
- Mouse and the tooltip (17).

## Acceptance
- [x] Snapshots of ancestry highlight and Focus mode on criss-cross at truecolor and 16 colours.
- [x] A selection change makes no layout or walk calls (asserted).
- [x] A lane on the selected ancestry is never folded (integration test with 14).
- [x] Selection stays on the same commit id after its chunk is evicted and rebuilt (test).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (2026-09-13, builder-opus, no compaction)

**Done.**
- `graft_graph::ancestry`: `Ancestry::of(rows, selected) -> Option<Ancestry>`, `kin(row)`, `lit(row)`, `columns()`. One pass over the window on hand; kin exact inside it.
- `GraphLine::thread: Option<BTreeSet<usize>>`: thread lanes bold, other lanes dim. Focus (`F`) also dims unrelated rows as a whole (`RowPaint::Dimmed`); header says `focus (F/Esc off)`; `Esc` leaves Focus before it cancels anything.
- Thread columns on screen join `Compaction.pinned`: never folded.
- Selection = `History::cursor` (row number per view, same commit across eviction); `History::reanchor` re-finds the selected commit after a view change (`Space`, `H`, `1`).
- Tests: 6 ancestry (incl. fold integration), thread paint, `F`/`Esc`, no-request selection change, eviction round trip, reanchor. Snapshots `selection__criss-cross-{truecolor,16}-{highlight,focus}`.
- Shots: `docs/shots/16-*` (plain frame + bold/dim map). The user judges them by eye.

**Judgement calls.** "Highlight" = bold thread and dim other lanes, always on while something is selected. "Focus" = that plus faded rows. docs/design/05-visualization.md §5.2 asks for low-saturation colour; this is bold/dim only (KNOWN_GAPS).

**Gaps.** `1` keeps the selection only within the first two screens of the other view (KNOWN_GAPS row updated). Thread limited to the window. Bold/dim only.
