# lane-connectors — a column carrying an edge between two commits in view is drawn

Status: ready (spec-only pass 2026-10-18, after 06b's NOTES question "connector columns become stubs")
Blocked by: 06b-lanes-on-screen-and-header (done; must be merged to main first). Re-shoots the captures listed in box 9: not beside any ticket re-shooting one of them (conductor checks 06c, 06h, 06r against the list).
Build with: Opus (`builder-opus-xhigh`) — one rule in `Fold::plan`'s scan and rank, pinned by fold tables and a proptest; the judgment step is sorting the re-shot captures into "connector restored" vs regression. No new public signature, no structural choice left open: not Fable. Scout skipped: 06b's NOTES name the line (`fold.rs:316`) and every capture it touched; Step 1 replaces it.
Needs: none new. Existing (as of 06b): `Fold::plan(max_lanes, &Keep, rows) -> Fold`, its `scan` (`graft-graph/src/fold.rs` ~:288-330; the drop rule `open[c] && (c == 0 || noded[c])` at :316), `Fold::{kept, folded, folded_count, open_columns, view, marker, drawn_columns}`, `FoldedEdge`/`Toward`, `Row.{lane, incoming, outgoing, lanes}`; graft-ui `fold_overlay` (`graph_column/mod.rs`, the rail node), `LaneTally::of`, `lane_phrases`.

## Read first
- This ticket, all.
- `.scratch/ease-pass/issues/06b-lanes-on-screen-and-header.md`: Decided (all); NOTES › "Question … connector columns become stubs", › "Deviations and decisions of mine", › "Re-shot captures, by class", › "Pixel strips", › "Fixtures restated".
- `docs/design-notes/ease-pass.md`: the paragraph **Lanes in view settled** (top of the note).
- Mock `prototypes/ease-mocks/mergewright-main-screen.html` :90 (the rule's words). Nothing else in the mock.
- `docs/design/03-graph-layout-algorithm.md` §3.5 (all; this ticket replaces most of it). ADR-0025 Decision and Residual.
- `crates/graft-graph/MAP.md` `fold`, `row`. `crates/graft-ui/MAP.md` `graph_column` (`fold_overlay`, `width_through`), `graph_screen` (`LaneTally`).

## Step 1 (replaces the scout; ≤10 lines in NOTES before any change)
Read `Fold::plan`/`scan` and say: how an edge's two ends are found from `Row.{incoming, outgoing, lanes}` (which row the edge leaves a node into column c, which row it joins a node from c); where node columns are ranked; where the rail is reserved; whether 06b built ADR-0025's `╷`/`╵` end marks (grep `╷` in `graft-ui/src/graph_column`).

## Decided
1. **Three kinds of open column**, per window W (the rows the plan sees):
   - **node column** — some row in W has its node in c (as 06b's `noded`).
   - **connector** — no node in c on W, and c carries at least one edge whose **two ends are both nodes on rows in W**: it leaves a node on row r1 ∈ W into c, runs down c, and joins a node on row r2 ∈ W. (Both nodes sit in other columns, by definition.) Shapes: merge main into feature, criss-cross, merge of an ancestor.
   - **pass-through** — every other open column (each edge it carries has an end outside W).
2. **Drop:** pass-throughs dropped at any cap, whatever their `Keep` priority (unchanged from 06b). Lane 0 kept while open, whatever its kind; not open → dropped (unchanged from 06b).
3. **Budget** (`max_lanes` drawn columns, rail included), admitted in order: lane 0; node columns by `Keep` class (selected, reserved, preferred, rest; least-recently-touched folds first) — unchanged from 06b; **then connectors, by the same classes and tie-break**. A connector is admissible only while, for at least one edge that made it a connector, both end nodes sit in kept columns (not on the rail). A connector not admitted — out of budget or ends on the rail — is **dropped** like a pass-through: not in `kept()`, not in `folded()`, never behind the rail, not in `folded_count()`, its edges drawn as stubs (`▾` at the upper end, `▴` at the lower). The rail appears only when a node column folds (unchanged).
4. **A drawn connector draws everything its column carries on W's rows**, as any kept column does: a second segment in it with an end outside W draws as a lane line, not a stub.
5. **Header:** no code change. N (`kept().len()`) now counts drawn connectors; M (`open_columns()`) unchanged. On the 5-commit "merge main into feature" repo N = M = 2 → no lane words.
6. **Expanded (`+`):** plans with `usize::MAX` (as 06b); nothing folds, so every connector is drawn; pass-throughs still are not.
7. **Pixel strips** plan through the same function: connectors in a strip's window draw; strip keys change for those windows (13 re-measures). No strip code change expected; if one is needed, stop and report.
8. **Rejected** (design note): per-branch identity as the test (merge-resolved identities; keeps lines leading off screen); "one end in view" (keeps lanes for merged branches whose commits are all below — the opposite of mock :90).

## Scope
In: graft-graph `fold` (`scan`/rank; `fold/tests/{in_view,properties}.rs`; MAP.md `fold` line); graft-ui only if `fold_overlay` must skip a dropped connector (report it) and re-accepted insta snapshots; graft `tests/it/*` captures listed below; `docs/design/03-graph-layout-algorithm.md` §3.5; ADR-0025 amendment; `docs/capabilities/m1-the-graph.md` M1-08 if it states the drop rule; the contact sheet.
Out: header words and fitting (06b, settled); graph mode (06h); seeks (06c); the fold itself (06); colours; any re-layout (the fold stays a lens, §3.5 last paragraph).

## Signatures
No new public items. `Fold::plan(max_lanes: usize, keep: &Keep, rows: &[Row]) -> Fold` (as today), `open_columns`, `kept`, `folded`, `folded_count`, `view`, `marker`, `drawn_columns` keep their signatures. Private helpers: builder's naming. MAP.md `Fold::plan` line becomes: "`plan(max_lanes, &Keep, rows) -> Fold`; drops pass-throughs, then admits lane 0, node columns, connectors by priority up to the budget."

## §3.5 wording to land (literal; the builder edits the doc)
Replace §3.5's ease-pass paragraph (the one starting **Ease pass (ADR-0063 …**), the block from **The cap is a budget** through "breakpoints, ADR-0022).", and the paragraph **The marker column is a rail** with:

> **Ease pass (ADR-0063; user approved 2026-10-17; tickets 06b, 06u)** — a branch gets a lane only while it has a commit in view (main mock :90), and the header's `+N lanes` becomes `lanes drawn: N of M`, shown only while something is not drawn.
>
> **Which columns can be drawn.** Before ranking, `Fold::plan` sorts the columns open in the window into three kinds:
> - a **node column** holds a commit of the window;
> - a **connector** holds none, but carries an edge whose two ends are both commits in the window (merging main into a feature branch, a criss-cross, a merge of an ancestor);
> - a **pass-through** is any other open column: every edge it carries leads out of view.
>
> Pass-throughs are dropped at any cap, whatever their priority: their edges draw as stubs, they never sit behind the rail and never count as folded. Lane 0 is kept while open, whatever its kind, so trunk scrolling out of view does not shift every lane left. A column open in no row of the window, lane 0 included, is dropped.
>
> **The cap is a budget, admitted by priority, not a set of vetoes** (ADR-0025). `Fold::plan` draws at most `max_lanes` columns, rail included, admitting in this order until the budget is spent:
> 1. lane 0 (always fits: the cap is ≥ 2);
> 2. node columns — the selected commit's own column, reserved integration lanes, the selection's ancestry thread's columns, then the rest;
> 3. connectors, in the same order.
>
> Within a class the least recently touched goes first. A node column past the budget **folds**: its node draws on the rail on its row in place of `⋮`, its edges as stubs beside it (ADR-0025's residual, built in 06b). A connector past the budget, or one whose end commit sits on the rail, is **dropped** like a pass-through, never folded: the rail marks hidden commits only. `graph.max_lanes` is clamped to what the screen can hold at its width (unset: 8/10/12 across the §1 breakpoints, ADR-0022). Expanded (`+`) plans with no budget: nothing folds, and pass-throughs are still not drawn.
>
> **The marker column is a rail (`⋮`), not a counter.** It appears only while a node column folds. The header counts at the top line's right end, after every other tail item: `lanes drawn: N of M` while N < M (N = real columns drawn, rail not counted; M = columns open in the window, pass-throughs included), then `K merged branches folded` while K > 0 (K = closed fold marks on rows in view), joined ` · `. They are the first tail items to give way, right phrase first, each whole and never cut with `…`, and they give way before the name of the branch `HEAD` is on. `+` expands and collapses every fold at once; a click on the rail to expand one row is not built (ADR-0022).

In the paragraph **A folded column is a stub**, the first sentence becomes "**An edge into a folded or dropped column is a stub, not a curve to a marker** (ADR-0022):". The last paragraph ("This is a rendering-time decision …") stays.

ADR-0025, appended section (fill the bracket from Step 1):
> ## Amended (ease-pass 06b, 06u; 2026-10-18)
> The residual is built: a node column past the budget folds and its node draws on the rail, so the budget holds for every class (`drawn_columns() <= max_lanes`, property-tested). [The `╷`/`╵` end marks are built / are not built.] Before ranking, a column with no commit in view is dropped unless it connects two commits in view; such connectors rank after every node column and, out of budget, drop rather than fold. `+N lanes` is replaced by `lanes drawn: N of M`. Current rule: DESIGN §3.5.

## Touches
Each checked, and how, in the report's **Touches:** line:
- `fold.rs` scan/rank; every fold test (`fold/tests/{in_view,properties}.rs`) — 06b's tests must still pass unchanged except where a fixture holds a connector (list each).
- graft-ui `graph_column` `fold_overlay`: a dropped connector must not draw on the rail (only folded node columns do) — test or grep.
- `GraphColumn::width_through`: wider where a connector returns; subjects move right in those windows.
- `LaneTally`/`lane_phrases`: no code change, numbers change; 06h builds on them — shape unchanged.
- Pixel strips `view::strips` (Decided 7); `event_loop` fast-tier frame tests (box 11).
- `update/{geometry,mouse,drag}.rs`: re-run 06b's grep (`fold|expanded|drawn(|real(`); a click on a connector cell lands on the row.
- Fixtures 06b restated (`view/tests/lane_cap.rs`, `view/strips/tests.rs` `woven`, `tests/it/lane_cap.rs` `((7,15),(11,15))`, `ancestry/tests.rs`): re-run; any changed number derived by hand in NOTES.
- Tests pinning capture sizes/rows (`tour_walk.rs` `file_size`).
- Docs: §3.5, ADR-0025, MAP.md `fold`, capabilities M1-08. Keys: none. Journal/undo: none.
- For the conductor (builder does not edit): KNOWN_GAPS row for ADR-0025's residual closes.

## Acceptance
- [ ] Merge-main-into-feature, constructed rows (5 rows; the merge's second-parent edge leaves its node on row 0 into column 1 and joins a node in column 0 on row 3; column 1 holds no node), window = all rows, cap 10: `kept()` = {0, 1}, `folded_count()` 0, `open_columns()` 2, no `FoldedEdge` in `view` of any row. — `fold::tests::in_view::a_column_joining_two_commits_in_view_is_drawn`.
- [ ] Same rows, window without the merge row: column 1 dropped, `open_columns()` still counts it, the lower end's row has one `FoldedEdge` `Toward::Above`; window without the lower end's row: dropped, the merge row has one `Toward::Below`. — `fold::tests::in_view::a_connector_with_an_end_out_of_view_is_a_pass_through`.
- [ ] Over the cap: node columns fill the budget, one connector besides → connector in neither `kept()` nor `folded()`, `folded_count()` counts node columns only, `marker()` is `Some` only when a node column folded; both its ends stubs. — `fold::tests::in_view::a_connector_past_the_budget_drops_and_never_folds`.
- [ ] A connector whose lower end's node column folded onto the rail is not kept; its upper end draws a stub. — `fold::tests::in_view::a_connector_to_a_commit_on_the_rail_drops`.
- [ ] A drawn connector also carrying a segment with an end out of view: that segment's rows draw it as a lane (in `view(row).row.lanes`, no `FoldedEdge`). — `fold::tests::in_view::a_drawn_connector_draws_all_it_carries`.
- [ ] Proptest restated: kept ∪ folded ⊆ node ∪ {0 if open} ∪ connectors; folded ⊆ node columns; every kept connector has a qualifying edge with both end columns kept; with `max_lanes = usize::MAX`, kept = node ∪ {0 if open} ∪ connectors and folded = ∅; `drawn_columns() <= max_lanes`; `open_columns()` = the open set's size. — `fold::tests::properties::<name>` (builder names it; the 06b proptest renamed or extended, said which).
- [ ] App: on the fixture behind `docs/shots/m6/fix-preview-verb-120x40.txt` (5 commits, main merged into feature), the merge row's graph cells contain `╮`, the lower end's row `╯`, neither row contains `▾` or `▴`, and the top line contains no `lanes drawn`. — the test that shoots that capture, extended with these asserts (named in the tick).
- [ ] Insta snapshots re-accepted: `ascii_tier` criss-cross ×2, `lanes_without_hue` criss-cross ×3, `selection` criss-cross ×4 — each draws the connector as lines, not `▾`/`▴`/`v`/`^`; `lanes_without_hue` many-lanes ×3 re-run (changed only if a connector is in view; say which). NOTES lists each with its before/after graph rows. — `cargo insta` test names per file.
- [ ] Re-shot captures, each classed in NOTES: **(C) connector restored** — the connector draws as lines again; header loses `lanes drawn` when N reaches M; where 06b dropped no other column in that window, the graph cells equal the pre-06b text (`git show <06b-merge>^:<path>`, graph cells diffed); or **(U) unchanged** (`git diff` empty). Anything else → stop, report it as a regression, do not re-shoot it. List: `m2-duplicate-commit`, `m2-evil-merge`, `m2-force-push`, `m2-overview-strip`, `m5-hints-marked-{80x24,120x40,160x48}`, `m6/fix-preview-verb-120x40`, `m7/landed-after-merge-120x40`, `m7/rewrite-edit-stop-80x24`, `m8-landing-undo-120x36`, `m8/settle-following`. — the shooting test per capture, named in the NOTES table.
- [ ] Re-run, expected (U): `ease-pass/main-header-{120,90}` (feature/ledger is a pass-through: tip above the window), `ease-pass/main-folded-*`, `ease-pass/main-opened-120`, `m5-lane-cap-160x24`. Any change: classed as above with N, M derived by hand. — `main_header::capture_main_header_at_120_and_90`, `fold_tail_narrow::capture_main_folded_narrow`, `lane_cap::*`.
- [ ] Fast-tier frame tests pass: `event_loop::tests::settle::a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between`, `event_loop::tests::pace::three_hundred_held_keys_never_put_two_frames_in_the_pipe`, `event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget`.
- [ ] Docs: §3.5 replaced with the literal wording above; ADR-0025 amendment appended (bracket filled from Step 1); MAP.md `fold` line; capabilities M1-08 if it states the rule. NOTES gives `git diff --stat` for the docs. — document box: names the files.
- [ ] Contact sheet `docs/shots/ease-pass/ease-pass-contact-sheet.txt` regenerated (`scripts/contact-sheet.sh --captions-order`), 06u's entries after 06b's, each caption naming its class and what right looks like ("`◉─╮` on the merge and `●─╯` on main's commit, no `▾`/`▴`, no `lanes drawn`"). Under 200 KB per part. Look unjudged (rule 6), colour unverified (rule 5). — document box.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~35k (one rule, five table tests, a proptest edit; ~15 captures and 9 snapshots re-shot and classed).
