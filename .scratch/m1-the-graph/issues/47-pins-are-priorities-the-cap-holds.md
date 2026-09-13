# m1-the-graph/47 — Pins are priorities: the lane cap holds and the rail is never cut by the thread

Status: blocked — waits on the user approving ADR-0025
Build with: **Sonnet** (medium) — the order, the type and the properties are decided (ADR-0025); the work is one admission loop, a type rename through two callers, tests and captures. Property tests judge it, so not Fable; nothing here is concurrency or a git write, so not Opus.
Needs: none. Overlaps `lane-cap-follows-width` (44) on `Compaction` and the two `view` callers — sequence after 44, don't run in parallel.
Blocked by: 44, ADR-0025 approval
Leaves runnable: yes — folding on a merge train obeys the cap; the header count is exact; the rail is in the last graph column on every row.

Finding (ticket 39 notes, KNOWN_GAPS "A selection on a merge train pins its whole thread"): after `j j` on git/git 20 columns are kept under a cap of 8, the graph column is cut at half the screen, the rail with it, header `+2 lanes`. Cause: ticket 16 made every thread column a veto against folding; a set of vetoes has no bound. ADR-0025 (Proposed): the cap is a budget, pins are priorities.

## Read first
- `docs/adr/0025-…` (all of it; short)
- `docs/adr/0022-…` Decision 2–3 (the rail, the count, the cap)
- DESIGN.md §3.5 (what this changes: the "ancestry path" bullet), §5.2 first two bullets (the thread is a paint-time lens)
- Ticket 14 "NOTES" line 42 (the current rank: untouched-first, earliest last touch, ties rightmost; dropped columns)
- Ticket 16 notes lines 39–41 (`Ancestry`, how thread columns reach `Compaction.pinned`)
- Ticket 39 "NOTES" (the fold's output shape; the captures and script `docs/shots/39-capture.py`)
- `crates/graft-graph/MAP.md` (ancestry, fold), `crates/graft-ui/MAP.md` (graph_screen: `Compaction`, `GraphView::compaction`), `crates/graft/MAP.md` (view::graph_view, view::strips); scouts for source

## Scope
In:
- `graft_graph::fold::Keep` and `Fold::plan(max_lanes, &Keep, rows)`: admit columns in ADR-0025's order until `drawn + rail == max_lanes`; class 1 (lane 0) and class 2 (a column holding a node in the window) are admitted even past the budget; classes 3–6 never are. Within a class, ticket 14's rank decides who folds first. Dropped columns (open in no row) unchanged. `folded_count` unchanged in meaning.
- `graft_ui::graph_screen::Compaction { max_lanes, expanded, keep: Keep }`; `Compaction::at` keeps nothing but lane 0 (`Keep::default()`).
- `view::graph_view` and `view::strips::pixel_rows` build `Keep`: `selected` = the cursor row's `Row.lane` when the cursor row is in the window (else `None`); `reserved` = what today goes into `pinned` as reserved integration lanes; `preferred` = `Ancestry::columns()`. Both callers build the same `Keep` (one helper, not two copies).
- `Ancestry::columns` doc: the preferred set, no longer "protected".
- TESTING.md git/git row (ADR-0025 Consequences): first frame at 80×24 `N ≥ 10` with the reason in the row; after `j j` still `N ≥ 10` and the rail is the last graph column on every row. many-lanes row: at 80×24 the first frame reads `+6 lanes` (9 node columns and the rail drawn: lane 0 and topics 14–7 hold nodes in the window; topics 6–1 fold); at 120×40 nothing folds and no rail (every column holds a node in the 38-row window — correct, not a gap).
- KNOWN_GAPS: close the row 39 added ("A selection on a merge train pins its whole thread…"); add ADR-0025's residual ("node columns alone can exceed what fits: trigger, and the rail-node design that closes it") and the `+` expanded overflow row if there isn't one.
- MAP.md for every public change (graft-graph fold + ancestry; graft-ui graph_screen; graft view).
Out: the rail-node drawing (ADR-0025 residual; not until observed); `max_lanes_for` and the fits clamp (44); the fold vocabulary (39/40); how rank inside a class is computed (unchanged); the `+` expanded state.

## Signatures
```rust
// graft-graph::fold (new; replaces the flat pinned set)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keep {
    pub selected: Option<usize>,      // the selected commit's own real column; None when its row is not in the window
    pub reserved: BTreeSet<usize>,    // reserved integration lanes, real columns
    pub preferred: BTreeSet<usize>,   // Ancestry::columns(): the selection's thread, real columns
}
impl Fold {
    pub fn plan(max_lanes: usize, keep: &Keep, rows: &[Row]) -> Fold;   // ADR-0025 order; rail counts toward max_lanes
}
// graft-ui::graph_screen (changed)
pub struct Compaction { pub max_lanes: usize, pub expanded: bool, pub keep: graft_graph::fold::Keep }
impl Compaction { pub fn at(max_lanes: usize) -> Self; }                 // collapsed, Keep::default()
// graft::view (new, private is fine; one builder for both callers)
fn keep_for(rows: &[Row], cursor: Option<usize>, reserved: &BTreeSet<usize>, thread: Option<&Ancestry>) -> Keep;
```
Callers to update: `view::graph_view`, `view::strips::pixel_rows`, every test that builds `Compaction` or calls `Fold::plan`.

## Acceptance
- [ ] Property (proptest, random windows over `wide_history`, random `Keep`, random `max_lanes ≥ 2`): if `{0} ∪ node_columns` plus the rail fit in `max_lanes`, then `drawn_columns().len() <= max_lanes`; otherwise `drawn_columns()` is exactly `{0} ∪ node_columns` plus the rail (no class 3–6 column admitted). Always: `kept ∪ folded` = every column open in some row; disjoint.
- [ ] Unit: budget with room for one more column — `selected` is admitted over `reserved`, `reserved` over `preferred`, `preferred` over an unclassed column with a more recent touch. Lane 0 is drawn at `max_lanes = 2` with 12 open columns.
- [ ] Unit: a thread spanning 12 columns under `max_lanes = 8` (selection on `wide_history`'s top merge): `drawn_columns().len() == 8`, `folded_count() >= 5`; the selected column is drawn.
- [ ] Screen test: a 22-row window in which the cursor's thread is 20 columns, `max_lanes = 8`, 80 columns wide: `GraphColumn::width_through` ≤ 40 cells, the rail is the last drawn column on every row, header ends `+N lanes` with N = open − 7.
- [ ] Existing 39 tests still pass unchanged in meaning (edges conserved; rail column never in the row).
- [ ] pty captures, release binary, `docs/shots/39-capture.py` reused: `docs/shots/<NN>-git-git-80x24-after-jj.txt` (rail on 21/21 rows in the last graph column, header `+N lanes` N ≥ 10, no row wider than 40 graph cells), `<NN>-many-lanes-80x24.txt` (`+6 lanes`), `<NN>-many-lanes-120x40.txt` (no rail, no count), `<NN>-seen-120-unicode.txt` after `j j` (for the user's eye check alongside 39/40's). Report paths only; the user judges by eye.
- [ ] TESTING.md rows as in Scope. KNOWN_GAPS rows as in Scope. MAP.md updated. Gates pass.
- Report: cause fixed or symptom hidden, and what it costs the user (ADR-0025 says: a folded thread column shows as coloured stubs, not an open lane).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam if it must stop: "fold model + tests green" / "screen wiring + captures + docs".
