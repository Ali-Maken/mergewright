# m1-the-graph/16 — Selection, ancestry highlight and Focus mode

Status: ready
Build with: **Opus** (high)
Blocked by: 14
Leaves runnable: yes — selecting a commit lights up its ancestry; `F` turns it up.

Why: "where did this commit come from" should be answered by looking, not by reading.

## Read first
- DESIGN.md §5.2 and §2.3 (selection keys only)
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
- [ ] Snapshots of ancestry highlight and Focus mode on criss-cross at truecolor and 16 colours.
- [ ] A selection change makes no layout or walk calls (asserted).
- [ ] A lane on the selected ancestry is never folded (integration test with 14).
- [ ] Selection stays on the same commit id after its chunk is evicted and rebuilt (test).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
