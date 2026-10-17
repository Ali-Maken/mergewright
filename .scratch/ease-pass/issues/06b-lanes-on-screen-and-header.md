# lanes-on-screen-and-header — lanes only for branches with a commit in view; `lanes drawn: N of M · K merged branches folded`

Status: ready (Q14 answered (a), 2026-10-17)
Blocked by: 06-graph-fold-and-lanes
Build with: Opus (`builder-opus-xhigh`) — `Fold::plan` is the layout engine's lens; checkable by fold tests over a fixture window.
Needs: from 06: `FoldWalk::folds`, `OpenedMerges`, the fold mark per row (a merge row in the window is "folded" when `folds == Some(true)` and it is not opened). Existing: `Fold::plan(max_lanes, &Keep, rows) -> Fold` (`graft-graph/src/fold.rs:73-110`), `Fold::kept`, `Fold::folded_count`, `HeaderView`/`header_line`/`tail_room`, `Filters::text`.

## Read first
- `docs/design-notes/ease-pass.md`: Prior art row "Lane count"; Mock behaviours G10, G11; Questions 11 (a), 14 (a, :254).
- `docs/design/03-graph-layout-algorithm.md` §3.5. ADR-0025 (the lane cap is a budget; pins are priorities).
- `crates/graft-graph/MAP.md` `fold` (all). `crates/graft-ui/MAP.md` `graph_screen` (`Filters`, `HeaderView`, `header_line`, `tail_room`), `graph_column` (`RAIL`, `width_through`). `crates/graft/MAP.md` `view::rows`.
- 06's scout file, "Lanes".

## Decided here
- **Which columns drop:** today `plan` keeps every column when `columns <= max_lanes` (node-less ones included), and over the cap keeps `open` columns (`scan`, `fold.rs:287-305`). Now a column with **no node in the window** is never a candidate, at or over the cap, whatever its `Keep` priority (selected/reserved/preferred only rank node-holding columns). Edges into a dropped column draw as today's stubs (`FoldedEdge`, `▾`/`▴`). The rail `⋮` and `+` stay for node-holding overflow.
- **M and N:** M = `open_columns()` — columns open in the window (any lane present or touched), node-less pass-throughs included. N = `kept().len()` — real columns drawn (the rail is not a lane). Shown only while N < M.
- **K = folded merges in view:** merge rows in the drawn window with a closed fold mark. Not a whole-history count (that needs the whole walk). Replaces today's `N merges folded` (retired by 06).
- **Q14 (a), user 2026-10-17:** no `on <branch>` — today's top line already names it once; only the tail is added.

## Scope
In (graft-graph `fold`; graft-ui `graph_screen` header words, `layout.rs` `left_out`; graft `view::rows` (`Filters` fill)):
- `Fold::plan` drops node-less columns as Decided; `open_columns()` added.
- **Header (G10, Q11 a, Q14 a):** today's top-line frame; at its right end `lanes drawn: N of M` (while N < M) and `K merged branches folded` (`1 merged branch folded`; while K > 0), joined ` · `, replacing `+N lanes` (`graft-ui/src/graph_screen/layout.rs:350-359`, N from `render.rs:85`). Narrow widths: whole phrases dropped from the end by `tail_room`, never cut mid-word.
Out: the fold itself (06); seeks (06c); roles and rows (07).

## Signatures
graft-graph `fold`: `Fold::plan` keeps its signature.
```rust
impl Fold {
    /// Columns open in the window, node-less pass-throughs included: the M of `lanes drawn: N of M`.
    pub fn open_columns(&self) -> usize;
}
```
graft-ui `graph_screen`:
```rust
pub struct Filters { /* existing, `folded` retired by 06 */ pub lanes: Option<(usize, usize)>, pub folded_in_view: usize }
/// The top line's right end: `lanes drawn: N of M · K merged branches folded`, each part only while it has something to say.
pub fn header_tail(filters: &Filters) -> String;
```

## Touches
Every fold test and capture (node-less columns gone at and under the cap); `GraphColumn::width_through`; `Fold::drawn`/`real` mapping callers (hit-tests: `update::geometry`, mouse, drag — a click on a dropped column's cell lands on the row as today); the rail and `+` (§2.3); pixel strips (`view::strips` reads the fold — 13 re-measures; note any strip-key change); header captures at 120/100/90; `docs/design/03-graph-layout-algorithm.md` §3.5 (edit approved 2026-10-17 — text in NOTES for the conductor).

## Acceptance
- [ ] Fold test over the fixture window: no column without a node in the window is drawn, under and over the cap; a merge edge from a dropped column draws as a stub.
- [ ] `open_columns` counts a node-less pass-through; `kept` doesn't.
- [ ] Header at 120 cols on the fixture: ends `lanes drawn: 5 of 8 · 3 merged branches folded` (numbers as the fixture gives; NOTES says how they were checked against the window); neither part shown when nothing is dropped / folded; no `+N lanes`, no `merges folded`; HEAD's branch named once on the top line (Q14 a); `header_tail` table test incl. singular.
- [ ] Width 90: the tail drops whole phrases from the end.
- [ ] Fast-tier frame tests pass: `event_loop/tests/settle.rs:312`, `pace.rs:220`, `pixel_scroll.rs:192`.
- [ ] Capture `docs/shots/ease-pass/main-header-120.txt`, `main-header-90.txt`. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~35k.
