# lanes-on-screen-and-header — lanes only for branches with a commit in view; `lanes drawn: N of M · K merged branches folded`

Status: ready (re-spec 2026-10-18 after scout `.scratch/ease-pass/scouts/06b-lanes-on-screen-and-header.md`; Q14 answered (a), 2026-10-17; expanded-lanes rule confirmed by the conductor 2026-10-18)
Blocked by: 06-graph-fold-and-lanes, 06r-fold-fixes (shares `graft-ui/src/graph_screen/render.rs`: 06r's fold-tail fallback and gap)
Build with: Opus (`builder-opus-xhigh`) — `Fold::plan` is the layout engine's lens and the header slot is shared with 06h; every rule is a table or fold test over constructed rows, so no Fable test is met. Scout done.
Needs: from 06: `GraphEntry.fold: Option<FoldMark>` (`FoldMark.open == false` = folded, not opened). Existing: `Fold::plan(max_lanes, &Keep, rows) -> Fold` (`graft-graph/src/fold.rs:73-110`, `scan` :288-330), `Fold::kept`, `Compaction::plan` (`graft-ui/src/graph_screen/mod.rs:366-369`), the applied fold (`layout.rs:290`), `render.rs:85` → `header(width, folded_lanes)` (`layout.rs:131`) → `tail_items` (:196) → `left_out` (:349-362), `fitted_tail` / `TailItem` / `Drop` (`graph_screen/header_tail.rs:19-57`), `tail_room` (`header.rs:64`).

## Read first
- The scout file above (all; one page).
- `docs/design-notes/ease-pass.md`: Prior art row "Lane count"; Mock behaviours G10, G11 (:104); Questions 11, 14 (:281, :286).
- Mock `prototypes/ease-mocks/mergewright-main-screen.html` :70 (top line words), :90 (why).
- `docs/design/03-graph-layout-algorithm.md` §3.5. ADR-0025 (the lane cap is a budget; pins are priorities).
- `crates/graft-graph/MAP.md` `fold` (all). `crates/graft-ui/MAP.md` `graph_screen` (`Filters`, `FoldMark`, `GraphEntry`, `HeaderView`/`header_line`/`tail_room`), `graph_column` (`RAIL`, `width_through`).
- `.scratch/ease-pass/issues/06h-graph-mode.md` Decided "Header" and Signatures `lane_phrases` (it fills the same slot after this ticket).

## Decided here
- **Which columns drop** (`Fold::plan`): a column open in the window with **no node in the window** is dropped — at, under and over the cap, whatever its `Keep` priority (selected/reserved/preferred rank node-holding columns only). Exception: **column 0 stays while open** (it is `required` today, :304-308; dropping it would shift every lane left as trunk scrolls out). Columns not open in the window drop as today (under the cap too — today's :75-81 keep-all branch goes). Dropped ≠ folded: a dropped column never sits behind the rail and never counts in `folded_count`. Edges into a dropped column draw as today's stubs (`FoldedEdge`, `▾`/`▴`). Rail `⋮` and `+` stay for node-holding overflow only.
- **Expanded lanes follow the same rule** (confirmed 2026-10-18: mock :90 "a branch gets a lane only while it has a commit in view"; Unicode matches pixel). Today the applied fold is `None` while expanded (`layout.rs:290`); pixel strips already plan with `usize::MAX` when expanded (`view/strips.rs:357-362`). Unicode now does the same — `Compaction::plan` with `usize::MAX` while expanded — so both tiers drop the same node-less columns and the header always has a `Fold` to count from.
- **Data flow: render counts, not `view::rows`.** N, M and K are facts of the frame (the plan and the rows in view exist only at render, `mod.rs:368`, `layout.rs:290`). Render builds a `LaneTally` from the applied `Fold` and the window's `GraphEntry`s and passes it to `header` in place of today's `folded_lanes: usize` (`render.rs:85`). Pure function of its inputs, no state, no I/O (PROMPT 6.1). `Filters` and `graft::view::rows` are **not changed** by this ticket.
- **N, M, K.** N = `kept().len()` (real columns drawn; the rail is not a lane). M = `open_columns()` = columns open in the window (a lane present or touched on any window row), node-less pass-throughs included. K = window rows whose `GraphEntry.fold` is `Some(FoldMark { open: false, .. })` — in view, not whole-history.
- **Words** (G10, Q11 a, Q14 a): at the top line's right end, after every existing tail item, `lanes drawn: N of M` while N < M, then `K merged branches folded` (`1 merged branch folded`) while K > 0, joined ` · `. Replaces `+N lanes` (`left_out`, `layout.rs:349-359`). No `on <branch>` (Q14 a). Numbers are the window's, not the mock's (mock :70 says `5 of 8` / `38`).
- **Fitting** (scout 2): the two phrases are separate `TailItem`s in a new group `Drop::Lanes`, the **first group to drop**, right phrase first (`K merged …`, then `lanes drawn …`), each **dropped whole, never cut with `…`**. `fitted_tail`'s `…` cut stays for the other groups (unchanged captures; out of scope).
- **Name** (scout 3): the fn is `lane_phrases`, not `header_tail` (the private module `graph_screen::header_tail` keeps its name; `lane_phrases` may live in it).
- **Shared slot with 06h.** 06b owns slot 1 (`lanes drawn`) and the folded wording of slot 2. 06h, after this ticket, adds the graph mode as a parameter (`lane_phrases(tally, every_branch: bool)`, mode from `Filters.every_branch`) and puts `every branch shown` in slot 2 — same item, same drop order. 06b adds no mode field and no placeholder.

## Scope
In: graft-graph `fold` (`plan`/`scan`, `open_columns`, tests); graft-ui `graph_screen` (`LaneTally`, `lane_phrases`, `Drop::Lanes` in `header_tail.rs`, `header`'s parameter, `left_out` loses `+N lanes`, `render.rs:85`, `layout.rs:290` expanded plan, tests); graft tests only for the two captures (`tests/it/fold_by_default.rs` or beside it) and any fold capture this re-shoots.
Out: `Filters` / `graft::view::rows`; the fold itself (06); seeks (06c); graph mode (06h); roles and rows (07); `fitted_tail`'s `…` rule for other groups; `on <branch>`.

## Signatures
graft-graph `fold` — `Fold::plan` keeps its signature:
```rust
impl Fold {
    /// Columns open in the window (a lane present or touched on any row), node-less pass-throughs
    /// included: the M of `lanes drawn: N of M`. `kept().len() + folded_count() <= open_columns()`.
    pub fn open_columns(&self) -> usize;
}
```
graft-ui `graph_screen` (`LaneTally` re-exported in MAP; `lane_phrases` crate-private):
```rust
/// What the top line says about lanes and folds, counted over the rows in view this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaneTally {
    /// N: real columns drawn (`Fold::kept().len()`; the rail is not a lane).
    pub drawn: usize,
    /// M: columns open in the window (`Fold::open_columns()`).
    pub open: usize,
    /// K: window rows whose fold mark is closed.
    pub folded_in_view: usize,
}
impl LaneTally {
    /// `window` = the `GraphEntry`s drawn this frame, in order.
    pub fn of(fold: &Fold, window: &[GraphEntry]) -> Self;
}
/// The top line's lane words, left to right; each only while it has something to say.
/// `[]`, `["lanes drawn: 5 of 8"]`, `["3 merged branches folded"]`, or both.
pub(crate) fn lane_phrases(tally: LaneTally) -> Vec<String>;
```
`header` (`layout.rs:131`) takes `LaneTally` where it took `folded_lanes: usize`. Private `Drop` gains `Lanes` (dropped first, whole items only).

## Touches
Each checked, and how, in the report's **Touches:** line:
- Every fold test (`fold/tests.rs:86-384`) and every capture whose window has a node-less column (they narrow; list each with its D3 class); `lane_cap::*` and `m5-lane-cap-160x24` (06f re-shot it expecting this).
- `GraphColumn::width_through` (narrower columns move subjects left).
- `Fold::drawn`/`real` callers and every place that assumes no fold while expanded: grep `expanded` in graft-ui `graph_screen` and graft `update::geometry`, mouse, drag. A click on a dropped column's cell lands on the row, as today; a click while expanded hits the column drawn.
- Pixel strips (`view::strips`): its plan's output changes only under the cap (it was already `usize::MAX` when expanded); say whether a strip key changes (13 re-measures).
- Header captures at 120/100/90 that showed `+N lanes` or a node-less column; `docs/design/03-graph-layout-algorithm.md` §3.5 (edit approved 2026-10-17) — text in NOTES for the conductor.
- 06h already names `lane_phrases` / `LaneTally` (updated 2026-10-18); 06c names neither.
- Keys: none bound. Journal/undo: none.

## Acceptance
- [ ] Fold test, constructed rows with 8 columns open in the window, 5 holding a node (column 0 among them), one node-less column `selected` in `Keep`: under the cap (`max_lanes` 10) `kept()` = the 5, `folded_count()` 0, `open_columns()` 8; the selected node-less column not kept. — names its test.
- [ ] Same rows, cap 3: kept ⊆ node columns, `kept().len() <= 3`, folded = the other node columns, dropped columns in neither. Column 0 open with no node in the window: kept. A merge edge from a dropped column draws as a `▾`/`▴` stub (`Fold::view`). — names its test.
- [ ] Proptest (`fold/tests.rs:345-384`) restated: kept ∪ folded = {open columns holding a node in the window} ∪ {0 if open}; kept ∩ folded = ∅; `kept().len() <= max(max_lanes, required)`; `open_columns()` ≥ kept + folded and equals the open set's size. — names its test.
- [ ] Expanded: render with `Compaction.expanded` on the same rows → the node-less columns are not drawn, `kept().len()` = 5, header `lanes drawn: 5 of 8`. — names its test.
- [ ] `lane_phrases` table: (5,8,3) → both, `lanes drawn: 5 of 8`, `3 merged branches folded`; (8,8,0) → `[]`; (8,8,1) → `["1 merged branch folded"]`; (5,8,0) → lanes only. — names its test.
- [ ] Fitting table (`fitted_tail` with the two `Drop::Lanes` items after a filters and gfx item): room for all → both; room 1 short of both → `lanes drawn: 5 of 8` and no fold phrase; room 1 short of the lanes phrase alone → neither; in every row no `…` touches a lane phrase and every other group is untouched. — names its test.
- [ ] `LaneTally::of`: window of 4 entries with fold marks closed, closed, open, none → `folded_in_view` 2. — names its test.
- [ ] `+N lanes` and `N merges folded` gone: NOTES shows `grep -rn 'merges folded' crates/graft-ui/src` and a grep for the `+N lanes` format string empty outside tests, and the 120 test asserts the top line contains neither `lanes ` after a `+` nor `merges folded`. — names its test.
- [ ] Capture `docs/shots/ease-pass/main-header-120.txt`, 120×36: `wallet_release` at a scroll where N < M and K ≥ 2; if none exists, a copy `wallet_release_wide()` with one unmerged branch forked below the window (never change `wallet_release`: 06c/06f/06h capture it). The test asserts the top line's tail **as a literal**; NOTES lists the window's open columns (node yes/no) and its merge rows (fold mark) and derives N, M, K by hand. The top line holds HEAD's branch name exactly once (`matches(branch).count() == 1`) and no ` on `. — names its test.
- [ ] Capture `main-header-90.txt`, same fixture and scroll at 90×36: test asserts the literal top line; NOTES shows `tail_room` at 90 against each phrase's length, so which phrase dropped is arithmetic, not taste. — names its test.
- [ ] Fast-tier frame tests pass: the test starting `event_loop/tests/settle.rs:315`, `pace.rs:220`, the test starting `pixel_scroll.rs:191`. — names them.
- [ ] Captures in the contact sheet after 06f's; look unjudged (rule 6), colour unverified (rule 5). NOTES: §3.5 wording, every re-shot capture with its D3 class.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~40k.
