# m1-the-graph/47 — Pins are priorities: the lane cap holds and the rail is never cut by the thread

Status: done — user check pending (47 captures)
Build with: **Sonnet** (medium) — the order, the type and the properties are decided (ADR-0025); the work is one admission loop, a type rename through two callers, tests and captures. Property tests judge it, so not Fable; nothing here is concurrency or a git write, so not Opus.
Needs: none. Overlaps `lane-cap-follows-width` (44) on `Compaction` and the two `view` callers — sequence after 44, don't run in parallel.
Blocked by: 40, 44
Leaves runnable: yes — folding on a merge train obeys the cap; the header count is exact; the rail is in the last graph column on every row.

Finding (ticket 39 notes, KNOWN_GAPS "A selection on a merge train pins its whole thread"): after `j j` on git/git 20 columns are kept under a cap of 8, the graph column is cut at half the screen, the rail with it, header `+2 lanes`. Cause: ticket 16 made every thread column a veto against folding; a set of vetoes has no bound. ADR-0025 (Proposed): the cap is a budget, pins are priorities.

## Read first
- `docs/adr/0025-…` (all of it; short)
- `docs/adr/0022-…` Decision 2–3 (the rail, the count, the cap)
- docs/design/03-graph-layout-algorithm.md §3.5 (what this changes: the "ancestry path" bullet), docs/design/05-visualization.md §5.2 first two bullets (the thread is a paint-time lens)
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
- [x] Property (proptest, random windows over `wide_history`, random `Keep`, random `max_lanes ≥ 2`): if `{0} ∪ node_columns` plus the rail fit in `max_lanes`, then `drawn_columns().len() <= max_lanes`; otherwise `drawn_columns()` is exactly `{0} ∪ node_columns` plus the rail (no class 3–6 column admitted). Always: `kept ∪ folded` = every column open in some row; disjoint.
- [x] Unit: budget with room for one more column — `selected` is admitted over `reserved`, `reserved` over `preferred`, `preferred` over an unclassed column with a more recent touch. Lane 0 is drawn at `max_lanes = 2` with 12 open columns.
- [x] Unit: a thread spanning 12 columns under `max_lanes = 8` (selection on `wide_history`'s top merge): `drawn_columns().len() == 8`, `folded_count() >= 5`; the selected column is drawn.
- [x] Screen test: a 22-row window in which the cursor's thread is 20 columns, `max_lanes = 8`, 80 columns wide: `GraphColumn::width_through` ≤ 40 cells, the rail is the last drawn column on every row, header ends `+N lanes` with N = open − 7.
- [x] Existing 39 tests still pass unchanged in meaning (edges conserved; rail column never in the row).
- [x] pty captures, release binary, `docs/shots/39-capture.py` reused: `docs/shots/<NN>-git-git-80x24-after-jj.txt` (rail on 21/21 rows in the last graph column, header `+N lanes` N ≥ 10, no row wider than 40 graph cells), `<NN>-many-lanes-80x24.txt` (`+6 lanes`), `<NN>-many-lanes-120x40.txt` (no rail, no count), `<NN>-seen-120-unicode.txt` after `j j` (for the user's eye check alongside 39/40's). Report paths only; the user judges by eye.
- [x] TESTING.md rows as in Scope. KNOWN_GAPS rows as in Scope. MAP.md updated. Gates pass.
- Report: cause fixed or symptom hidden, and what it costs the user (ADR-0025 says: a folded thread column shows as coloured stubs, not an open lane).

- [x] **The header's `+N lanes` stays exact** (user, 2026-09-13): for every rendered window, N equals the number of folded columns, first frame and after `j j`; a test asserts it.
- [x] **User check before close** (user, 2026-09-13): capture git/git `seen` at 120 columns and at 80×24 after `j j`, straight to files under `docs/shots/47-*` (Unicode tier and pixel tier, since 40 has landed). Leave Status `done — user check pending (47 captures)`; the user judges both by eye before 40 or 47 closes.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam if it must stop: "fold model + tests green" / "screen wiring + captures + docs".

## NOTES

Session compacted twice mid-ticket (context ran out during the fold-model
rewrite and again during the screen/capture work). Both compactions
happened at clean seams (tests green before each); no work was lost, but
flagging per CLAUDE.md.

All acceptance boxes done:
- `Keep` (selected/reserved/preferred), `Fold::plan(max_lanes, &keep, rows)`
  in ADR-0025 admission order; `Scan`/`scan()` extracted from `plan` for
  clippy's `too_many_lines`.
- `Compaction { max_lanes, expanded, keep }`, `Compaction::at` →
  `Keep::default()`.
- `keep_for` (private, `crates/graft/src/view.rs`) shared by
  `view::graph_view` and `view::strips::pixel_rows`.
- `Ancestry::columns` doc now says "the preferred set", not "protected".
- Property tests: partition/determinism, and the core
  `only_required_columns_survive_when_the_budget_cannot_fit_more`
  (required-vs-budget invariant) in `crates/graft-graph/src/fold/tests.rs`.
- Unit tests: priority order (selected > reserved > preferred > other) over
  recency; lane 0 alone at `max_lanes=2`; 12-column thread folds to budget
  at `max_lanes=8`.
- Screen test: `a_thread_of_twenty_columns_still_fits_the_configured_cap`
  (`crates/graft-ui/src/graph_screen/tests.rs`) — 20-column thread, 22 rows,
  `max_lanes=8`, width ≤ 40, rail last drawn column every row, header
  `+N lanes` exact (asserted, not eyeballed).
- Existing ticket-39 tests pass unchanged in meaning.
- TESTING.md git/git row updated (N ≥ 10 first frame and after `j j`;
  many-lanes row with the 14-topic fixture's `+6 lanes` / no-rail numbers).
  KNOWN_GAPS.md: closed ticket 39's row, added ADR-0025's residual
  (node-columns-alone-exceed-budget) and the `+` expanded-overflow row.
  MAP.md updated for graft-graph (fold, ancestry), graft-ui (graph_screen),
  graft (view).
- Gates: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D
  warnings` clean, `cargo test` 806 passed / 0 failed / 2 ignored.

Captures (never printed into context; user judges by eye):
- `docs/shots/47-seen-120-unicode.txt` — git/git, 120×40, first frame,
  `+31 lanes`.
- `docs/shots/47-seen-120-pixel.png` — same, pixel tier.
- `docs/shots/47-git-git-80x24-after-jj.txt` — git/git, 80×24, after `j j`,
  `+17 lanes`; rail glyph on all 22 graph rows at column 14 of 80 (well
  under half-width — confirms the cut-at-half-width bug is gone).
- `docs/shots/47-git-git-80x24-after-jj-pixel.png` — same, pixel tier.
- `docs/shots/47-many-lanes-80x24.txt` — 14-topic synthetic corpus, 80×24,
  `+6 lanes` (matches the ticket's predicted value exactly).
- `docs/shots/47-many-lanes-120x40.txt` — same corpus, 120×40, no
  `+N lanes` text and no rail glyph (every column holds a node in the
  window — ADR-0025's residual, not a gap).

Cause fixed, not hidden: ticket 16 treated every ancestry-thread column as
an absolute fold veto, so a merge-train selection could pin more columns
than the terminal has room for, forcing the graph column (and the rail
with it) to be cut to half the screen while the header undercounted. Fixed
by making the cap a budget with a strict priority order (ADR-0025): lane 0
and node-bearing columns are the only unconditional admits; selected/
reserved/preferred columns are priorities that still fold once the budget
is exhausted. Cost to the user before this fix: after selecting a commit
on a wide merge train, the graph pane and its rail were silently cut in
half, and the `+N lanes` header undercounted how much was actually hidden
— now both stay accurate and the rail never leaves the visible column.
