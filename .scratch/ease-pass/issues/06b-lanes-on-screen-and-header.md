# lanes-on-screen-and-header — lanes only for branches with a commit in view; `lanes drawn: N of M · K merged branches folded`

Status: done 2026-10-18 (builder; re-spec 2026-10-18 after scout `.scratch/ease-pass/scouts/06b-lanes-on-screen-and-header.md`; Q14 answered (a), 2026-10-17; expanded-lanes rule confirmed by the conductor 2026-10-18)
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
- [x] Fold test, constructed rows with 8 columns open in the window, 5 holding a node (column 0 among them), one node-less column `selected` in `Keep`: under the cap (`max_lanes` 10) `kept()` = the 5, `folded_count()` 0, `open_columns()` 8; the selected node-less column not kept. — `graft_graph::fold::tests::in_view::under_the_cap_only_columns_holding_a_node_are_drawn`.
- [x] Same rows, cap 3: kept ⊆ node columns, `kept().len() <= 3`, folded = the other node columns, dropped columns in neither. Column 0 open with no node in the window: kept. A merge edge from a dropped column draws as a `▾`/`▴` stub (`Fold::view`). — `fold::tests::in_view::over_the_cap_node_columns_fold_and_node_less_ones_stay_dropped` (cap 3, stubs both ways), `fold::tests::in_view::column_zero_stays_while_open_though_no_node_sits_in_it`.
- [x] Proptest (`fold/tests.rs:345-384`) restated: kept ∪ folded = {open columns holding a node in the window} ∪ {0 if open}; kept ∩ folded = ∅; `kept().len() <= max(max_lanes, required)`; `open_columns()` ≥ kept + folded and equals the open set's size. — `fold::tests::properties::only_columns_holding_a_node_and_lane_zero_are_drawn_or_folded` (proptest moved to `fold/tests/properties.rs`; also asserts `drawn_columns() <= max_lanes`).
- [x] Expanded: render with `Compaction.expanded` on the same rows → the node-less columns are not drawn, `kept().len()` = 5, header `lanes drawn: 5 of 8`. — `graft_ui::graph_screen::tests::lanes_in_view::expanded_lanes_still_draw_only_the_columns_holding_a_node`.
- [x] `lane_phrases` table: (5,8,3) → both, `lanes drawn: 5 of 8`, `3 merged branches folded`; (8,8,0) → `[]`; (8,8,1) → `["1 merged branch folded"]`; (5,8,0) → lanes only. — `graph_screen::tests::lanes_in_view::the_lane_words_say_only_what_there_is_to_say`.
- [x] Fitting table (`fitted_tail` with the two `Drop::Lanes` items after a filters and gfx item): room for all → both; room 1 short of both → `lanes drawn: 5 of 8` and no fold phrase; room 1 short of the lanes phrase alone → neither; in every row no `…` touches a lane phrase and every other group is untouched. — `graph_screen::tests::lanes_in_view::the_lane_phrases_drop_first_whole_and_right_to_left` (rooms 68 / 67 / 40, and every room 20..=68: no `…`, `first parent  gfx ok` whole).
- [x] `LaneTally::of`: window of 4 entries with fold marks closed, closed, open, none → `folded_in_view` 2. — `graph_screen::tests::lanes_in_view::the_tally_counts_the_closed_folds_in_view`.
- [x] `+N lanes` and `N merges folded` gone: NOTES shows `grep -rn 'merges folded' crates/graft-ui/src` and a grep for the `+N lanes` format string empty outside tests, and the 120 test asserts the top line contains neither `lanes ` after a `+` nor `merges folded`. — `mergewright` it `main_header::capture_main_header_at_120_and_90` (asserts no `+`, no `merges folded`); greps in NOTES › Touches.
- [x] Capture `docs/shots/ease-pass/main-header-120.txt`, 120×36: `wallet_release` at a scroll where N < M and K ≥ 2; if none exists, a copy `wallet_release_wide()` with one unmerged branch forked below the window (never change `wallet_release`: 06c/06f/06h capture it). The test asserts the top line's tail **as a literal**; NOTES lists the window's open columns (node yes/no) and its merge rows (fold mark) and derives N, M, K by hand. The top line holds HEAD's branch name exactly once (`matches(branch).count() == 1`) and no ` on `. — `main_header::capture_main_header_at_120_and_90` (literal suffix `47 commits  lanes drawn: 1 of 2 · 2 merged branches folded`, `wallet-release` once, no ` on `); derivation in NOTES › Box 9.
- [x] Capture `main-header-90.txt`, same fixture and scroll at 90×36: test asserts the literal top line; NOTES shows `tail_room` at 90 against each phrase's length, so which phrase dropped is arithmetic, not taste. — `main_header::capture_main_header_at_120_and_90` (literal whole top line at 90); arithmetic in NOTES › Box 10.
- [x] Fast-tier frame tests pass: the test starting `event_loop/tests/settle.rs:315`, `pace.rs:220`, the test starting `pixel_scroll.rs:191`. — `event_loop::tests::settle::a_held_j_follows_the_first_row_and_the_row_it_rests_on_and_nothing_between`, `event_loop::tests::pace::three_hundred_held_keys_never_put_two_frames_in_the_pipe`, `event_loop::tests::pixel_scroll::held_j_on_wide_strips_stays_within_the_byte_budget` (fixture restated to `woven(12, 20)`: see NOTES).
- [x] Captures in the contact sheet after 06f's; look unjudged (rule 6), colour unverified (rule 5). NOTES: §3.5 wording, every re-shot capture with its D3 class. — no test; `docs/shots/ease-pass/ease-pass-contact-sheet.txt` regenerated by `scripts/contact-sheet.sh --captions-order`, 06b's entries after 06f's (captions tsv); NOTES › Proposed §3.5 wording, › Re-shot captures.
- [x] Each tick names its test. — box 12 is a document box and names the file instead.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~40k.

## NOTES (builder, 2026-10-18)

**Scout used:** `.scratch/ease-pass/scouts/06b-lanes-on-screen-and-header.md`, all of it. **Compacted twice** mid-ticket. What remained after the second compaction (full gates, re-shooting `main-folded-50`, the contact sheet, these notes) is done in this commit. Nothing is left for a split.

### Question, filed with its lines (rule 7): connector columns become stubs
The rule as decided (`fold.rs:316`: `open[c] && (c == 0 || noded[c])`) drops a column holding no node even when **both ends of its edge are in view**. That is the "merge main into feature" connector, and the criss-cross and merge-of-ancestor shapes too. The line between two visible commits becomes two stubs, and no key brings it back: `+` / expanded uses the same rule. Evidence, before → after:
- `docs/shots/m7/landed-after-merge-120x40.txt`: `◎┈╮ [◆ feature] Merge branch 'main' into feature` … `●─│─╯ [main] main moves again` → `◎▾` … `●▴│`.
- `docs/shots/m6/fix-preview-verb-120x40.txt`: `◉─╮ Commit 4` … `●─╯ Commit 2` → `◉▾` … `●▴`. The header reads `lanes drawn: 1 of 2` on a 5-commit repo.
- `docs/shots/m2-overview-strip.txt`, `m2-duplicate-commit.txt`, `m5-hints-marked-*`, `m7/rewrite-edit-stop-80x24.txt`; the criss-cross insta snapshots (`selection__criss-cross-truecolor-highlight.snap`: `◉▾ beta: merge alpha`).

This is the box 2 rule as written ("A merge edge from a dropped column draws as a `▾`/`▴` stub"), so it is built that way. But he merges main into feature daily in a team (memory: works in a team on dependent branches), and M1's promise is "the shape of history is obvious at a glance". Possible change, **not built**: count a column as drawable when it carries an edge whose two ends are both window rows. That leaves pass-throughs with one end outside the window dropped, which is what mock :90 is about. **Conductor/user to decide.**

### Deviations and decisions of mine
- **Node columns now fold over the cap.** Boxes 2/3 require it (`kept().len() <= max(max_lanes, required)`). It supersedes §3.5 class 2 and ADR-0025's "a column holding a node may still exceed the budget". The budget now holds, rail included (`properties.rs` asserts `drawn_columns() <= max_lanes`).
- **A folded node draws on the rail** (`graph_column/mod.rs` `fold_overlay`), with its stub in the gap beside it. Before this, a node whose column folded had its `●` overwritten by `⋮`. Test: `graph_column::tests::fold::a_node_whose_column_folded_draws_on_the_rail_in_its_place`.
- **The lane phrases give way before the branch name** (`header.rs` `branch_floor`, `layout.rs` `lanes_room = room − branch_floor`). Without it, `main-folded-50` lost ` · wallet-release` to `2 merged branches folded`, because the path ladder yields before `Drop::Lanes` is even tried. Now the lane words drop first whenever keeping them would cost the branch name. Test: `graph_screen::tests::lanes_in_view::the_lane_phrases_give_way_before_the_branch_name`. The ticket did not decide this; it is my call.
- **Lane 0 drops when not open.** It was always kept before. Visible in `m8-landing-undo-120x36.txt`: the empty first column is gone and the graph shifts 2 cells left.

### Proposed §3.5 wording (for the conductor; doc not edited)
Replace class 2 and the last sentence of the priority paragraph:
> **Only columns with a commit in view are drawn** (ease-pass 06b). Before ranking, a column open in the window with no node in the window is dropped, at any cap and whatever its priority. Its edges draw as stubs (`▾` / `▴`). It never sits behind the rail and never counts as folded. Lane 0 is the one exception: it stays while open, so trunk scrolling out does not shift every lane left. The remaining node-holding columns are admitted by priority until the budget is spent, rail included: lane 0, the selected commit's column, reserved integration lanes, the selection's ancestry thread, then everything else. Within a class the least recently touched folds first. A node whose column folded draws on the rail cell in place of `⋮`, with its stub beside it. Expanded (`+`) plans with no cap: nothing folds, and node-less columns are still not drawn.

Replace the "+N lanes" paragraph's header sentence:
> The header counts at the top line's right end, after every other tail item. `lanes drawn: N of M` shows while N < M (N = real columns drawn, rail not counted; M = columns open in the window, dropped pass-throughs included). `K merged branches folded` shows while K > 0 (K = closed fold marks on rows in view). The two are joined ` · `. They are the first tail items to give way, right phrase first, each whole and never cut with `…`, and they give way before the branch `HEAD` is on.

### Box 9: the window of `main-header-120.txt` (fixture `wallet_release_wide`, `tests/it/main_header.rs`)
The rows in view are `release: note 30` … `note 01`, then the named merge `Merge branch 'sara/fix-toc'` (closed fold) and `Merged PR 16021` (closed fold, cursor).
- Open columns (2):
  - wallet-release's: node yes.
  - feature/ledger's pass-through: node no. It branches from main at `Add the readme`, below the window, and its tip `ledger: sketch the export` is above the window, so its lane crosses every row in view.
- Main's column 0 is not open in the window: its lane starts at `Add the readme`, below.

So N = 1, M = 2, K = 2. The test asserts the top line ends `47 commits  lanes drawn: 1 of 2 · 2 merged branches folded`, as a literal suffix.

### Box 10: arithmetic at 90
`tail_room` = 90 − 1 margin − 6 (` ◆ api`) − 2 gap = **81**. The branch floor ` · wallet-release` = 17, so `lanes_room` = 64. The tail is `47 commits` 10 + 2 + `lanes drawn: 1 of 2` 19 + ` · ` 3 + `2 merged branches folded` 24 = **58 ≤ 64**, so no lane phrase drops. The path gives way instead (`~/code/api` → `…api`). The test asserts the whole top line as a literal.

### Re-shot captures, by class
06b's two legitimate classes, after 06 D3:
- **(A) header words only:** `lanes drawn …` / `… folded` added to the top line, nothing else.
- **(B) node-less column dropped:** the graph narrows by the dropped column(s), subjects move left, and the dropped edges become `▾`/`▴`, plus (A).

Every diff below was read; none moves the cursor or changes a badge, row, menu or banner.

| Class | Captures |
|---|---|
| A | `ease-pass/main-folded-120`, `ease-pass/main-opened-120`, `m6/fix-finish-shell-merge-120x40`, `m7/radar-not-checked-evil-120x40`, `m7/take-refused-merge-120x40`, `m8-help-80x24`, `m8-help-graph-120x36`, `m8-keymap-config-problem-120x36`, `m8-keymap-remapped-120x36`, `m8-bar-after-search-120x36`, `m8/search-hint-120x36`, `m8-look-chrome-{80x24,120x36,187x50}`, `m8-look-cursor-row-{80x24,187x50,dark-120x36,light-120x36}`, `m8-look-help-graph-187x50`, `m8-palette-{empty,everything,find}-120x36`, `m8/doorway-marked-{120x36,140x40}`, `m8-tour-finished-120x36` |
| B | `m2-duplicate-commit`, `m2-evil-merge`, `m2-force-push`, `m2-overview-strip`, `m5-hints-marked-{80x24,120x40,160x48}`, `m6/fix-preview-verb-120x40`, `m7/landed-after-merge-120x40`, `m7/rewrite-edit-stop-80x24`, `m8-landing-undo-120x36` (lane 0 not open, so dropped), `m8/settle-following` |
| Fixture restated | `m5-lane-cap-160x24`: topics now hold a commit in view; `lanes drawn: 7 of 15` with the pane open, `11 of 15` closed; a folded topic tip draws on the rail |
| Unchanged | `ease-pass/main-folded-50`: re-shot mid-ticket, and back to its committed text after `branch_floor` (`git diff` empty) |

**Contact sheet:** `docs/shots/ease-pass/ease-pass-contact-sheet.txt` has 40 captures, 160 338 B, in one part. 06b's entries come after 06f's last entry (`m5-lane-cap`): the two new captures, the B class, then the A-class captures 06f had not listed. The captions of 06f's A-class entries gain a 06b clause.

### Gates (worktree, `CARGO_TARGET_DIR` own)
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --doc`: all ok.
- `cargo nextest run --workspace --no-fail-fast`: 6775/6777 passed, 2 failed. Both failures were data I had not yet updated, and both pass on re-run, with no source change after the full run:
  - `file_size` (`tour_walk.rs` row 946 → 944);
  - `fold_tail_narrow::capture_main_folded_narrow` (`main-folded-50` re-shot).

Insta snapshots re-accepted, all (B):
- `ascii_tier` criss-cross ×2;
- `lanes_without_hue` criss-cross ×3 and many-lanes ×3 (`lanes drawn: 7 of 11`, nodes on the rail `●▾`);
- `selection` criss-cross ×4.

`ascii_tier`'s allowed glyphs gain `v ^`, the existing `to_ascii` stubs, which criss-cross now shows.

### Fixtures restated (the rule changed what they measure)
- `view/tests/lane_cap.rs` `history_with_lanes`: every column holds a node in view. A pass-through no longer counts against the cap.
- `view/strips/tests.rs` `woven`: interleaved chains, so each column holds a node in each strip. `capped` is deleted; `fan` is kept.
- `event_loop/tests/pixel_scroll.rs`: `held_j_on_wide_strips…` and `capped_at_pixel_tier` now on `woven`.
- `tests/it/lane_cap.rs`: scrolled to `main: step 30`; asserts `((7,15),(11,15))`.
- `tests/it/tour_walk.rs` `ready_to_shoot`: ignores the lane words in the header.
- `graft-graph` `ancestry/tests.rs`: the window now holds the topic commits.

### Pixel strips
Strips plan through the same `Fold::plan`. So wherever a node-less column is in view, both capped **and expanded** strip output change. The ticket's "only under the cap" no longer holds: expanded was already `usize::MAX`, but `usize::MAX` now drops node-less columns too. Strip keys change for those windows. This was re-measured only through the fast-tier tests named in box 11, not on a real terminal.

### Touches
- **Fold tests:** split into `fold/tests/{in_view,properties}.rs` and restated.
- **Captures:** the 39 listed above.
- **`lane_cap::*`:** restated.
- **`GraphColumn::width_through`:** narrower, checked by `expanded_every_node_column_draws_in_its_real_column_from_the_same_rows` (26).
- **`Fold::drawn`/`real` callers:** graph_column `render`/`fold_overlay` and graph_screen layout/footer.
  - `expanded` grep: only `graph_screen/compaction.rs:36` reads it for the graph.
  - `update/{geometry,mouse,drag}.rs`: a grep for `fold|expanded|drawn(|real(` is empty. A click lands on the row whatever the column, as before.
- **`graph_screen/footer.rs`:** 2-line signature change (`applied: Option<&Fold>` → `fold: &Fold`). It borders ticket 10's footer area; no behaviour change.
- **Header phrases:**
  - `grep -rn 'merges folded' crates/graft-ui/src crates/graft/src` gives one hit, a test comment (`update/tests/fixtures.rs:176`).
  - A grep for `} lanes`/` lanes"` format strings in non-test src is empty: no `+N lanes` left.
- **Docs:** `docs/capabilities/m1-the-graph.md` M1-08 and both MAP.md files updated.
- **06h:** builds on `lane_phrases(tally: LaneTally) -> Vec<String>` (crate-private) and `LaneTally::of(&Fold, &[GraphEntry])` as signed.
- **Keys:** none. **Journal/undo:** none.

### Broken vs unverified
- **Broken:** nothing known. The connector question above is open, not a defect.
- **Unverified:**
  - Colour: stub colour, the rail-node's colour, and the header phrases' style. Rule 5.
  - The look of the rail-node and the stubs: unjudged (rule 6).
  - Pixel tier on a real terminal: unverified.
