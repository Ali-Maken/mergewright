# lane-connectors — a column carrying an edge between two commits in view is drawn

Status: done 2026-10-18 (builder; spec-only pass 2026-10-18, after 06b's NOTES question "connector columns become stubs")
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
- [x] Merge-main-into-feature, constructed rows (5 rows; the merge's second-parent edge leaves its node on row 0 into column 1 and joins a node in column 0 on row 3; column 1 holds no node), window = all rows, cap 10: `kept()` = {0, 1}, `folded_count()` 0, `open_columns()` 2, no `FoldedEdge` in `view` of any row. — `fold::tests::in_view::a_column_joining_two_commits_in_view_is_drawn`. Red before the change (`kept()` was `[0]`).
- [x] Same rows, window without the merge row: column 1 dropped, `open_columns()` still counts it, the lower end's row has one `FoldedEdge` `Toward::Above`; window without the lower end's row: dropped, the merge row has one `Toward::Below`. — `fold::tests::in_view::a_connector_with_an_end_out_of_view_is_a_pass_through`.
- [x] Over the cap: node columns fill the budget, one connector besides → connector in neither `kept()` nor `folded()`, `folded_count()` counts node columns only, `marker()` is `Some` only when a node column folded; both its ends stubs. — `fold::tests::in_view::a_connector_past_the_budget_drops_and_never_folds`. Cap 3: the three node columns fill it, no rail; cap 2: two node columns fold, rail at 1, connector not counted; cap 4: the connector draws.
- [x] A connector whose lower end's node column folded onto the rail is not kept; its upper end draws a stub. — `fold::tests::in_view::a_connector_to_a_commit_on_the_rail_drops`. **Limit:** whenever a node column folds the budget is already full (kept + rail = `max_lanes`, Step 1), so this test cannot tell "end on the rail" from "no room": removing the ends check in `plan` keeps it green. The check is the spec's rule, kept for when the budget leaves room; no test isolates it.
- [x] A drawn connector also carrying a segment with an end out of view: that segment's rows draw it as a lane (in `view(row).row.lanes`, no `FoldedEdge`). — `fold::tests::in_view::a_drawn_connector_draws_all_it_carries`. Red under 06b's rule.
- [x] Proptest restated: kept ∪ folded ⊆ node ∪ {0 if open} ∪ connectors; folded ⊆ node columns; every kept connector has a qualifying edge with both end columns kept; with `max_lanes = usize::MAX`, kept = node ∪ {0 if open} ∪ connectors and folded = ∅; `drawn_columns() <= max_lanes`; `open_columns()` = the open set's size. — `fold::tests::properties::<name>` (builder names it; the 06b proptest renamed or extended, said which). — `fold::tests::properties::node_columns_draw_or_fold_connectors_draw_or_drop_and_pass_throughs_never_draw`: 06b's `only_columns_holding_a_node_and_lane_zero_are_drawn_or_folded` **renamed and extended** (keeps lane 0 ⊆ kept, node columns ⊆ kept ∪ folded). Its connector oracle walks each column down the window; the fold finds them by parent id, so the two are independent. A temporary probe (`prop_assert!` no kept connector, reverted) failed on case `{8: [(5, 2)]}`: generated cases do hold drawn connectors. Fast tier only; heavy tier not run.
- [x] App: on the fixture behind `docs/shots/m6/fix-preview-verb-120x40.txt` (5 commits, main merged into feature), the merge row's graph cells contain `╮`, the lower end's row `╯`, neither row contains `▾` or `▴`, and the top line contains no `lanes drawn`. — the test that shoots that capture, extended with these asserts (named in the tick). — `preview::capture::capture_fix_preview_verb_for_docs_shots`, moved from `preview.rs` into `preview/capture.rs` so `preview.rs` drops to 1102 lines instead of raising its `over_400.txt` row (1140); caption path updated. Fails on 06b's frame (`◎▾`, no `╮`). The fixture is 8 commits plus the preview's ghost merge, not 5 (NOTES).
- [x] Insta snapshots re-accepted: `ascii_tier` criss-cross ×2, `lanes_without_hue` criss-cross ×3, `selection` criss-cross ×4 — each draws the connector as lines, not `▾`/`▴`/`v`/`^`; `lanes_without_hue` many-lanes ×3 re-run (changed only if a connector is in view; say which). NOTES lists each with its before/after graph rows. — `cargo insta` test names per file. — `ascii_tier::crowded_shapes_draw_only_the_ascii_glyph_table` (criss-cross no-color, truecolor), `lanes_without_hue::crowded_shapes_at_every_depth_below_truecolor` (criss-cross 16, 256, no-color), `selection::ancestry_highlight_and_focus_on_criss_cross` (16/truecolor × focus/highlight). 9 re-accepted (`INSTA_UPDATE=always`; no cargo-insta here), each equal to its pre-06b text; many-lanes ×3 re-run, unchanged (no connector in view). Rows in NOTES.
- [x] Re-shot captures, each classed in NOTES: **(C) connector restored** — the connector draws as lines again; header loses `lanes drawn` when N reaches M; where 06b dropped no other column in that window, the graph cells equal the pre-06b text (`git show <06b-merge>^:<path>`, graph cells diffed); or **(U) unchanged** (`git diff` empty). Anything else → stop, report it as a regression, do not re-shoot it. List: `m2-duplicate-commit`, `m2-evil-merge`, `m2-force-push`, `m2-overview-strip`, `m5-hints-marked-{80x24,120x40,160x48}`, `m6/fix-preview-verb-120x40`, `m7/landed-after-merge-120x40`, `m7/rewrite-edit-stop-80x24`, `m8-landing-undo-120x36`, `m8/settle-following`. — the shooting test per capture, named in the NOTES table. — NOTES › Re-shot captures: 11 (C), every one equal to its pre-06b text; `m7/rewrite-edit-stop-80x24` (U). No regression.
- [x] Re-run, expected (U): `ease-pass/main-header-{120,90}` (feature/ledger is a pass-through: tip above the window), `ease-pass/main-folded-*`, `ease-pass/main-opened-120`, `m5-lane-cap-160x24`. Any change: classed as above with N, M derived by hand. — `main_header::capture_main_header_at_120_and_90`, `fold_tail_narrow::capture_main_folded_narrow`, `lane_cap::*`. Also `fold_by_default::capture_main_folded_and_opened_at_120`. All green with no drift under `MWR_UPDATE_SHOTS=1`; none of the files in the changed set: all (U).
- [x] Fast-tier frame tests pass: `event_loop::tests::settle::a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between`, `event_loop::tests::pace::three_hundred_held_keys_never_put_two_frames_in_the_pipe`, `event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget`. Green in the full workspace run (6833/6833).
- [x] Docs: §3.5 replaced with the literal wording above; ADR-0025 amendment appended (bracket filled from Step 1); MAP.md `fold` line; capabilities M1-08 if it states the rule. NOTES gives `git diff --stat` for the docs. — document box: names the files. — `docs/design/03-graph-layout-algorithm.md` §3.5, `docs/adr/0025-the-lane-cap-is-a-budget-and-pins-are-priorities.md` (bracket: "are not built"), `crates/graft-graph/MAP.md` (`Fold::plan` as specified; `Keep` too), `docs/capabilities/m1-the-graph.md` M1-08 (it stated the drop rule; proof names updated). Stat in NOTES.
- [x] Contact sheet `docs/shots/ease-pass/ease-pass-contact-sheet.txt` regenerated (`scripts/contact-sheet.sh --captions-order`), 06u's entries after 06b's, each caption naming its class and what right looks like ("`◉─╮` on the merge and `●─╯` on main's commit, no `▾`/`▴`, no `lanes drawn`"). Under 200 KB per part. Look unjudged (rule 6), colour unverified (rule 5). — document box. — `docs/shots/ease-pass/ease-pass-contact-sheet.txt`, 40 captures, 160 242 B, one part; captions in `ease-pass-contact-sheet-captions.tsv`, 06u's 11 entries moved after 06b's last (`m8-tour-finished`).
- [x] Each tick names its test. Boxes 12 and 13 are document boxes and name files.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~35k (one rule, five table tests, a proptest edit; ~15 captures and 9 snapshots re-shot and classed).

## NOTES (builder, 2026-10-18)

### Step 1 (before any change; no scout, as the ticket says)
- **Edge ends** (`Row`, `row.rs:28-56`): an edge leaves its node on row r1 into column c when `r1.outgoing` holds `(c, parent)` with `c != r1.lane`; it runs down c (`lanes[c]` is `Some` on each row it passes); it joins a node on row r2 when `r2.incoming` holds c (the lane closes there, `r2.lane != c`), or it ends in a node sitting in c (`r2.lane == c`, then c is a node column). `lanes` only says which columns are open; `outgoing`/`incoming` give the ends.
- **Node columns ranked:** `scan`, `fold.rs:319-345`: `fold_key = (last_touch, Reverse(c))`, classes other → preferred → reserved → selected, giving `candidates` in fold order (lowest priority first). `drawable` (`:315-317`) is the 06b drop rule.
- **Rail reserved:** `plan`, `fold.rs:92-100`: once `drawable.len() > max_lanes`, columns fold while `drawn + 1 > max_lanes` (the `+ 1` is the rail). So whenever anything folds, kept + rail = `max_lanes` exactly (cap ≥ 2): no room is left for a connector.
- **`╷`/`╵` end marks:** not built. `grep -rn '╷\|╵' crates/` is empty (graft-ui `graph_column` included).

**Scout:** none, as the ticket says; Step 1 above replaced it. **Compacted:** no.

### What was built
- `fold/scan.rs` (new, private): the scan moved out of `fold.rs` (370 → 316 lines; `scan.rs` 159) and gained the connector kind. A connector is found by parent id: for each window row's outgoing edge into a column other than its node's, the parent's row in the window has that column in `incoming`. Its ends are recorded as the two node columns `(upper, lower)`. Columns are ranked by one `rank` fn for node columns and connectors alike (same classes, same tie-break).
- `Fold::plan`: node columns fold as in 06b; then connectors, highest priority first, take the room left after the kept columns and the rail, each only while one of its edges joins two kept columns. A connector never enters `folded`.
- No public signature changed. graft-ui: no code change (`fold_overlay` puts a node on the rail only when `fold.drawn(line.row.lane)` is `None`, `graph_column/mod.rs:297`; a connector holds no node, so it never lands there).

### Insta snapshots: before (06b) → after (06u = pre-06b text)
All 9 are the criss-cross corpus; columns 3 and 4 are the connectors.
- `lanes_without_hue` 16/256/no-color, `selection` ×4 (Unicode):
  - before: `│ ◉▾│ beta: merge alpha` / `│ │ ◉▾ alpha: merge beta` / `│ ●▴│ beta: work` / `│ │ ●▴ alpha: work`, header `lanes drawn: 3 of 5`.
  - after: `│ ◉─│─╮` / `│ │ ◉─│─╮` / `│ ●─│─│─╯` / `│ │ ●─╯`, no lane words. Selection focus variants draw `╌`/`╎` on the faded segments as before 06b.
- `ascii_tier` no-color/truecolor: before `| *v|`, `| | *v`, `| *^|`, `| | *^`; after `| *-|-\`, `| | *-|-\`, `| *-|-|-/`, `| | *-/`.
- `lanes_without_hue` many-lanes ×3: unchanged (no connector in their window).
- Style rows (`modifiers: graph | subject` block) widen with the graph: colour data, unverified (rule 5).

### Re-shot captures, by class
Compared with `git show fcc5b7bd^1:<path>` (06b's merge's first parent), `#` caption lines excluded. Every (C) file is **byte-equal to its pre-06b text**, so in none of these windows had 06b dropped anything but connectors.

| Capture | Class | Shooting test |
|---|---|---|
| `m2-duplicate-commit` | C | `view::tests::capture_m2_duplicate_commit` |
| `m2-evil-merge` | C | `view::tests::capture_m2_evil_merge` |
| `m2-force-push` | C | `view::tests::capture_m2_force_push` |
| `m2-overview-strip` | C (both tiers) | `view::tests::capture_m2_overview_strip` |
| `m5-hints-marked-{80x24,120x40,160x48}` | C | `graph_screen::tests::capture_hint_bar_for_docs_shots` (graft-ui) |
| `m6/fix-preview-verb-120x40` | C (+ caption: test path, 06u line) | `preview::capture::capture_fix_preview_verb_for_docs_shots` |
| `m7/landed-after-merge-120x40` | C | `land_on_result::capture_landed_after_merge` |
| `m7/rewrite-edit-stop-80x24` | U (`git diff` empty) | `m7_walk::rewrite_stops::capture_edit_stop_sizes` |
| `m8-landing-undo-120x36` | C (frame 2) | `land_on_result::capture_landing_after_undo_and_redo` |
| `m8/settle-following` | C | `event_loop::tests::settle::capture_the_pane_following_a_burst` |

N = M in every (C) window, so no header keeps `lanes drawn`; `grep 'lanes drawn\|▾\|▴'` over the 11 files hits only `fix-preview-verb`'s own caption line.

**Two of 06b's NOTES were misattributed** (found while classing; nothing to fix in code):
- 06b › Deviations says lane 0 dropping when not open is "visible in `m8-landing-undo-120x36.txt`". It is not: 06b's diff of that file only turns the connector `◆─╮ … ●─│─╯` into `◆▾ … ●▴│`, and main's `●` stays put. The lane-0 shift is in `m7/rewrite-edit-stop-80x24` (`    ●` → `  ●`). The landing-undo caption is corrected in the captions file, and the rewrite-edit-stop caption now says what 06b did there.
- 06b › Question cites `fix-preview-verb` as `◉─╮ Commit 4 … ●─╯ Commit 2`, `lanes drawn: 1 of 2` on a 5-commit repo. Those rows are `m5-hints-marked-*`. `fix-preview-verb` is 8 commits plus the preview's ghost merge: `◎┈╮ … ●─│─╯`, `lanes drawn: 2 of 3` under 06b. This ticket's box 7 repeats the "5 commits"; its asserts hold on the real fixture.

**For 08t:** `m7/landed-after-merge-120x40` (`merge main into feature ✓`) and `m8-landing-undo-120x36` (`undo: … ✓`, `redo: … ✓`) carry a `✓` done line, and I re-shot both. Neither has `✓ ·`.

### Docs (`git diff --stat`)
```
 crates/graft-graph/MAP.md                          |  4 +--
 ...lane-cap-is-a-budget-and-pins-are-priorities.md |  3 ++
 docs/capabilities/m1-the-graph.md                  | 15 +++++---
 docs/design/03-graph-layout-algorithm.md           | 41 ++++++++--------------
```
§3.5 carries the literal wording. The "A folded column is a stub" paragraph stays between the budget block and the rail paragraph, with its new first sentence; the last paragraph is unchanged.

### Touches
- **Fold:**
  - `fold.rs` scan/rank are in `fold/scan.rs` now.
  - 06b's table tests in `fold/tests.rs` and `in_view.rs` pass unchanged. No 06b fixture holds a connector: `in_view::window()` uses parent ids outside the window; `wide_history` windows have each topic's ends in a node column or out of view.
  - Only the proptest was restated (box 6).
- **`fold_overlay`:** no change. See What was built.
- **`GraphColumn::width_through`:** wider by one lane (2 cells) per connector that is back. Subjects move right in the (C) captures, exactly back to pre-06b.
- **`LaneTally`/`lane_phrases`:** no code change. Numbers change: N = M in the (C) windows.
- **Pixel strips:**
  - No code change.
  - `view/strips/tests.rs` and `event_loop::tests::pixel_scroll::*` are green.
  - Strip keys change for any window with a connector. Not re-measured on a real terminal.
- **`update/{geometry,mouse,drag}.rs`:** `git grep -nE 'fold|expanded|drawn\(|real\('` hits only a test name (`mouse.rs:375`). A click lands on the row, whatever the column.
- **Fixtures 06b restated, all green with no number changed:**
  - `view/tests/lane_cap.rs`;
  - `view/strips/tests.rs` `woven`;
  - `tests/it/lane_cap.rs` `((7,15),(11,15))`;
  - `ancestry/tests.rs`.
- **`tour_walk.rs` `file_size`:** green, unchanged.
- **`over_400.txt`:** `crates/graft/tests/it/preview.rs` lowered 1140 → 1102 (capture test moved to `preview/capture.rs`, 74 lines). No row raised.
- **Other `Fold::plan` callers** (`git grep -n 'Fold::plan'`): `graph_screen/compaction.rs:41`, `view/strips.rs:362`, tests. No change at any of them.
- **Keys:** none. **Journal/undo:** none.
- **For the conductor:** the KNOWN_GAPS row for ADR-0025's residual closes (I did not edit it).

### Gates (worktree, own `CARGO_TARGET_DIR`)
- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo nextest run --workspace --no-fail-fast`: 6833/6833 passed, 16 skipped.
- `cargo test --workspace --doc`: ok.

### Broken vs unverified
- **Broken:** nothing known.
- **Unverified:**
  - Colour of the restored connector lines and of the snapshot style rows (rule 5).
  - The look: unjudged (rule 6).
  - The pixel tier on a real terminal.
  - The proptest at heavy-tier case counts.
  - The "end on the rail" branch of the connector admission: no window reaches it today (box 4).
