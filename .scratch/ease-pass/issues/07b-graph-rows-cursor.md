# graph-rows-cursor — the cursor row's second line, scroll and hit-test around it, the bar per row kind

Status: **partial** (2026-10-19): split (i) built (Rewritable fix per conductor ruling, settle tests 2–3, `live_branch`, `row_bar`, the bar switch); `BranchRow` removal and all of split (ii) remain. See NOTES "Remaining". Was: blocked: Rewritable vs mock bar (settle test 2). Was: ready (split from 07 at its old (a)/(b) point, planner spec-only pass 2026-10-18, after scout `.scratch/ease-pass/scouts/07-graph-rows.md` at e9b08d12; mismatches 2, 4, 5 and the two *could not confirm* items settled below)
Blocked by: 07-graph-rows. **Anchors are the scout's, pre-split: map each through 06s's and 06t's NOTES tables and 07's NOTES before starting.**
Build with: Opus — scroll and every screen-line → row hit-test change meaning together, three facts are settled by tests before the code they decide, and the bar choice spans view, update and refs (scout's call).
Needs: from 07: `AppState.route`, `GraphEntry` (+ `badges` undrawn if kept), `RowLabel`, `lane_roles`, `ref_spans`; from 03b/03r: `HintContext::GraphRow { row: RowBar, verbs: RowVerbs }`, `RowBar::{Own, Elsewhere { branch: SanitizedText }, FoldedMerge { inside: u32 }, OpenedMerge, JustCommitted, JustMerged { inside: u32 }}` (`hints/context/row_bar.rs:12`), `bar_name` (:47); from 06: `OpenedMerges`, `InsideMerge { merge, label }` (`view/rows/fold.rs:27`), `FoldMark { open, count, from }`; app `refs::RefBadges`, `refs::RefLabel`; `ContainedIn` (`update/events.rs:318`) from `read_contained` (`jobs/detail.rs:340`); `cursor_row_context` (`view/rows/context.rs:107`) with 08's `landed_bar` arm (08:115).

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours G4, G9, G13 (:116–127); Questions 1, 2, 11.
- `docs/design/01-view-mockups.md` §1.1–1.3 (the cursor row). `docs/design/02-keymap.md` §2.3.
- `ARCHITECTURE.md` :273–280 (shared layout arithmetic lives in `mergewright-ui` in plain numbers, both sides call it).
- `crates/mergewright-ui/MAP.md`: `graph_screen` (`row_carve`, `::hints` :198–214). `crates/mergewright/MAP.md`: `view::rows`, `history::scroll`, `update::geometry`, `update::mouse`, `update::drag`, `update::in_view`, `refs` (:186–191), `jobs::detail` (:129), `event_loop::rest` (:64).
- `03b-ease-vocabulary-rest.md` Signatures; `03r-vocabulary-fixes.md` Decided (the opened merge row); `docs/reviews/ease-pass-03.md` Minor 10.
- The scout's file, items "Row context choice site", "RowBar", "InsideMerge", "Contains read", "Scroll and hit tests".

## Settle first — one committed test each, before the code it decides
1. **The contains read at rest** (scout: cost not measured). Test `the_contains_read_is_asked_when_the_cursor_rests` (state test through `event_loop::rest`, `rest.rs:3`): cursor rests on a commit row, panes as on first launch; assert whether a `read_contained` job is asked. **Asked** → `also in` draws from `ContainedIn` for the cursor's Oid. **Not asked** (e.g. only with the detail pane open) → `also in` draws only when `ContainedIn` for that Oid is already held. **Either way this ticket adds no read and no trigger** — its cost is unmeasured and measuring is a script's job, not a builder's (CLAUDE.md Gates). Not-asked case: NOTES + a KNOWN_GAPS row marked **unverified** (`also in` only after the detail read). Keep the test, renamed for what it found.
2. **`Rewritable`** (`context.rs:141`, scout M4). Before the switch, pin today's whole 120-col bar for `Rewritable(PlainRow)` and `Rewritable(BranchRow { .. })`. After: `GraphRow` sits inside `Rewritable` where the old arms did (wrapper kept). If the composed bar differs from 03r's pinned `RowBar` bars (`each_row_bar_draws_the_mock_bar_at_120_columns`) in anything other than what `Rewritable` added before → **stop**, `blocked: Rewritable vs mock bar`, both bars in NOTES.
3. **The other `PlainRow` pickers** (`view/rows.rs:308`, `:765`, `update/commit_editor/done_row.rs:58`; scout M4). A test pinning each one's pick today; NOTES says in one line what row each stands for. A picker standing for a commit row calls `row_bar` (below); one with no commit row keeps `PlainRow`.

## Decided here
- **Choice site** is `cursor_row_context`, `view/rows/context.rs:107` (scout M2). 08's `landed_bar` arm and the `JustCommitted`/`JustMerged` arms (:128) stay ahead of it; `Marked`, `Searching`, `WorkingTreeRow`, `StashRow` as today.
- **One rule, one pure fn** (`row_bar`): fold open → `OpenedMerge`; fold closed with a `Known` count → `FoldedMerge { inside }` (`Pending`/`Failed` → the branch's bar; `space` still acts, NOTES lists it for the usability walk); else the row's branch: HEAD's → `Own`, another live branch → `Elsewhere { branch }` (full name; `bar_name` shortens); **no branch → `None`, and the caller picks `PlainRow`**. So `PlainRow` stays as the no-branch bar; `BranchRow` is removed with its list arms and sweep arms if nothing else picks it (scout M2: its only input was `cursor.badges`), else NOTES names the picker. 07's undrawn `GraphEntry.badges` goes if nothing else reads it.
- **A row's branch:** an ordinary row → its `made_on`, named as 07's branch column names it. **A row inside an opened merge** (scout M5, *could not confirm* a resolver) → **a new lookup, no existing one assumed:** `RefBadges::live_branch(label)` — the local branch of exactly that name, else a remote-tracking `<remote>/<label>`; neither → no branch (a deleted PR branch never draws `C switch to <b>`). No label → no branch. Answered from a name index `badges` builds once per refs read — never a scan per frame. Unit tests first: local hit, remote-only hit, deleted → `None`, HEAD's branch → `Own`.
- Inner rows: `space` acts (06's handler, closes the merge, cursor on its row) but is **not drawn** (mock).
- **Transient (review 03 Minor 10):** `C switch to <b>` runs ahead of 09's `open_on`: until 09 lands, `C` opens the picker, not on that branch. NOTES says so; 09's acceptance closes it; a KNOWN_GAPS row only if the milestone ends before 09.
- **Second line (G9):** the cursor row — every row, opened ones included — grows one line: `<who> · <N hours> ago · made on <branch>[ · <labels>][ · merged N from <b>][ · also in A, B] · <sha7>`, words as the mock. `made on` absent when `made_on` is `None` (never `made on ?`). `merged N from <b>`: a merge row with `FoldCount::Known(n)`, `<b>` = its `FoldMark.from`; else absent. `also in`: per settle-test 1; absent before it lands, never `reading…`. Uncommitted row: `not committed yet · N files · on <branch>`. Lanes leaving the cursor row downward continue straight through it in their roles; no nodes, no connectors. Ghost Preview's slice draws no second line (unchanged look).
- **Layout arithmetic in ui** (ARCHITECTURE.md:277): `commit_rows` and `body_line` live beside `row_carve` in `mergewright-ui` `graph_screen`; `history::scroll` and every hit-test call them (moved from the old spec's `history::scroll`: both sides need it).
- **Scroll:** `History::visible(height)` (`history/scroll/query.rs:172`) shows `commit_rows(height, cursor_in_view)` rows. The cursor never sits on the body's last line (its second line would be cut): held `j/k` scrolls one row earlier.
- **Hit-tests:** every screen-line → row mapping goes through `body_line`; the second line maps to the cursor row. `update/geometry.rs:311` `graph_row_at` (callers `mouse.rs:98,134`, `drag.rs:71,90`) changes. `History::row_at` callers (`context_menu.rs:56,178`, `drag.rs:180`, `update/fold_count.rs:63`): NOTES says for each whether it is given a screen line or a row index; only screen-line ones change, each with a click test.

## Scope
In — mergewright-ui: `graph_screen` (second line drawing, `commit_rows`, `body_line`, `GraphEntry.second`, `badges` field removal if free), `graph_screen/hints/**` (only the `BranchRow` removal and its list/catalogue arms). mergewright app: `view::rows` (`context.rs`, the two pickers, second-line fill), `view::entries` (second line words), `history::scroll`, `update::geometry`, `update::mouse`, `update::drag`, `update::context_menu`, `update::fold_count` (only a screen-line `row_at` call), `update/commit_editor/done_row.rs` (the one `row_bar` call, if settle-test 3 says so), `refs` (`live_branch` and its index), sweep arms (`sweep.rs`, `dead_keys.rs`, `sweep_screens.rs`) for `BranchRow`.
Out: hint list words (03b, 03r); 06's `space` handler; the picker opening on a branch (09); pixel tier second line (13 — this is its model); making the contains read run unconditionally (needs a perf script; listed for the conductor).

## Signatures
mergewright-ui `graph_screen`:
```rust
pub struct SecondLine { pub words: SanitizedText, pub sha7: SanitizedText }
// GraphEntry gains: pub second: Option<SecondLine>
/// Which drawn row a body line belongs to; the line under the cursor row is its second line.
pub enum BodyLine { Row(u16), SecondOf(u16) }                     // index into the rows drawn
pub fn body_line(line: u16, cursor: Option<u16>) -> BodyLine;     // cursor: its row index if in view
/// Rows the body shows commits on: one fewer while the cursor is in view (never below 1).
pub fn commit_rows(body_rows: u16, cursor_in_view: bool) -> u16;
```
mergewright app:
```rust
// view::rows::context
pub(crate) struct RowBranch { pub(crate) name: SanitizedText, pub(crate) own: bool }
pub(crate) enum MergeFold { NotAMerge, Closed { inside: Option<u32> }, Open }   // inside: Some only when Known
/// The bar a commit row takes; `None` → no live branch → `PlainRow`.
pub(crate) fn row_bar(branch: Option<&RowBranch>, fold: MergeFold) -> Option<RowBar>;
// refs
impl RefBadges {
    /// The live branch a merged branch's label names: local of exactly that name, else `<remote>/<label>`.
    pub(crate) fn live_branch(&self, label: &[u8]) -> Option<RefLabel>;
}
```

## Touches
`history::scroll` and `in_view`; overview strip viewport; Map `⏎` landing and search landing (the cursor-not-on-last-line rule); every hit-test above; drag (§2.15, 07's `ref_spans` lines shift below the cursor); context menu; fold count's `row_at`; Ghost Preview (no second line — check its slice height maths); `cursor_row_context` tests and 08's `landed_bar` arm; 12 (`cursor_row_context` must exist — it still does); hint snapshots (no words change — list any `.snap` edited); sweeps; every graph capture (re-shot with the second line); `docs/design/01-view-mockups.md` §1.1–1.3 (approved edit, text in NOTES); 09 (`C`'s interim word); 13 (second-line model); KNOWN_GAPS (settle-test 1, only if not asked).

## Acceptance
Each tick names its test inline.
- [ ] Settle tests 1–3 committed before the code they decide (commit order in NOTES), each outcome in NOTES.
- [x] `live_branch`: local hit, remote-only hit, deleted → `None` (unit tests); built from an index, not a per-call scan (NOTES names where `badges` builds it). — `refs::live_tests::{a_local_branch_of_exactly_that_name_is_found, a_remote_only_branch_is_found_under_its_remote, a_deleted_branch_is_none, heads_own_branch_is_found_as_current, origin_head_is_not_a_branch}`.
- [x] `row_bar` table test: every `MergeFold` × {own, elsewhere, none}. — `view::rows::row_bar::tests::row_bar_table`.
- [ ] Bar per row kind (state tests through the choice site, words from 03b/03r): own row `⏎ open · P push · c commit`; a `sara/fix-toc` row `C switch to fix-toc` and `x compare`; a folded merge with `Known(3)` `space show the 3 commits inside`, with `Pending` its branch's bar; the opened merge row `space hide the commits inside · ⏎ open · x compare`; an inner row of a merge from a live `sara/fix-toc` → `⏎ open · C switch to fix-toc · x compare`, no `space` hint; an inner row of a merge whose branch was deleted → no `C switch to` (`PlainRow`); `space` on that inner row still closes the merge, cursor on the merge row.
- [ ] `BranchRow` removed with its arms, or NOTES names its picker; `PlainRow`'s pickers named in NOTES.
- [ ] Second line: only the cursor row has one; starts with the author name, contains `made on ` on a named row, ends with a 7-hex hash; the line below continues every lane glyph straight; `j` moves it; uncommitted row reads `not committed yet`; a `Known` merge row reads `merged 3 from fix-toc`; no `reading…` anywhere.
- [ ] `body_line` / `commit_rows` unit tests (cursor absent, at 0, at the last row).
- [ ] Scroll: 40 held `j` from the top never puts the cursor on the body's last line or its second line off screen; click on the second line selects the cursor row; click below it selects the row drawn there (each changed hit-test caller has one).
- [ ] Captures next to the main mock: `docs/shots/ease-pass/main-cursor-own-120.txt`, `main-cursor-teammate-120.txt`, `main-opened-merge-120.txt` (cursor on the opened merge row), and 07's `main-120.txt`, `main-100.txt`, `main-90.txt`, `main-colours-off-120.txt` re-shot with the second line; colour unverified (rule 5).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~70k. Split point if it overruns: (i) settle tests 2–3, `live_branch`, `row_bar` and the bar switch; (ii) settle test 1, second line, scroll and hit-tests, captures.

## NOTES (builder, 2026-10-19)

Stopped at settle test 2, as the ticket orders. No compaction. Scout: the ticket's (`scouts/07-graph-rows.md`); anchors re-found by grep at 7305006d (choice site still `view/rows/context.rs:107`; `PlainRow` pickers now `view/rows.rs:183`, `view/rows/hint_context.rs:307`, `update/commit_editor/done_row.rs:58`).

**Settle test 2 — blocked: Rewritable vs mock bar.** Tests in `crates/mergewright-ui/src/graph_screen/hints/tests/bars.rs`:
- `todays_rewritable_bars_at_120_columns` pins today. `Rewritable` adds only `R rewrite` at the list's end; 120 columns drops it, so each bar equals the bare row's:
  - `Rewritable(PlainRow)`: ` ⏎ open · b branch · r rebase · t tag · c commit · y sha · m merge · u undo · S stash · J journal · / search · ? keys`
  - `Rewritable(BranchRow{current})`: ` ⏎ open · c commit · P push · p pull · f fetch · b branch · t tag · u undo · S stash · J journal · / search · ? keys`
- `a_rewritable_graph_row_draws_the_wide_tail_not_the_mock_bar` pins the composed bar as found (verified by running it):
  - mock (03r, `each_row_bar_draws_the_mock_bar_at_120_columns`), Own: ` ⏎ open · P push · c commit · u undo · . actions · ? keys`
  - composed `Rewritable(GraphRow{Own})`: ` ⏎ open · P push · c commit · R rewrite · q quit · F2 stat · F3 rend · u undo · x compare · + lane · / search · ? keys`
  - mock, Elsewhere `sara/fix-toc`: ` m merge fix-toc into yours · ⏎ open · C switch to fix-toc · x compare · u undo · . actions · ? keys`
  - composed: ` m merge fix-toc into yours · ⏎ open · C switch · x compare · R rewrite · q quit · F2 stat · F3 rend · u undo · ? keys`
- Differs beyond `R rewrite`: `. actions` is gone, `q quit · F2 stat · F3 rend` (and on Own `x compare · + lane · / search`) come in, `C switch to fix-toc` falls to `C switch`.
- **Cause, with its line:** `crates/mergewright-ui/src/graph_screen/hints/filters.rs:347`. `drawable` picks `ROW_BAR_TAIL` only when `matches!(context, HintContext::GraphRow { .. })`. A `Rewritable(GraphRow)` fails that match and gets the screen-wide `TAIL`.
- **Why it matters:** `rewrite_offered` (`view/rows/context.rs`) wraps HEAD's own branch rows whenever HEAD is not the default branch. That covers his daily case, feature-branch rows, which are exactly the `Own` bar.
- **The same composition already ships:** `landed_bar`'s `GraphRow{JustCommitted}` is wrapped by `Rewritable` today (`context.rs:128` → `:141`). Not verified by a test here. It is a question for the fixer.
- **Possible fix, not mine to make** (hints are out of scope): look through `Rewritable` (and `Marked`?) in `filters.rs:347`. Then decide whether `R rewrite` should draw on the mock bar at all. Owner of the hint words: 03b/03r.

### Unblocked: conductor ruling, 2026-10-19
The ruling: the mock bar wins, and the cause is fixed inside 07b. This scope addition was the conductor's call.
- **Fix (cause):** `crates/mergewright-ui/src/graph_screen/hints/filters.rs` `drawable` now asks `HintContext::is_graph_row_bar()` (new, `hints/context/row_bar.rs`). It is true for `GraphRow`, bare or under `Rewritable`, so both take `ROW_BAR_TAIL`.
- **`R rewrite` under `Rewritable(GraphRow)`:** `hints/lists.rs` `rewritable_list` adds no `R rewrite` there, so the bar is exactly the mock's. `R` still answers and `?` lists it. **Question for the conductor:** the mock has no `R`. It was not drawn at 120 before either, because the width dropped it.
- **Test:** `hints::tests::bars::a_rewritable_graph_row_draws_the_mock_bar` replaces the found-pin test. Every `RowBar`, bare, wrapped, and marked-and-wrapped, draws the bare mock bar; Own and `sara/fix-toc` are checked literally (`. actions` is back, as is `C switch to fix-toc`).
- **The landing bar was hit too:** `JustCommitted` under `Rewritable` was already shipping the wide tail; the re-shot captures show `c commit again` back.
- **Other wrapper misses grepped:**
  - In `filters.rs`: `:333` already lists `Rewritable`, `:360` is divider-only, and `status_row.rs` `landing_bar` already looks through wrappers.
  - `catalogue/forms.rs` `has_row_verbs` missed `GraphRow`'s verbs. Fixed.
  - App `view/rows/context.rs`: `search_wrapped`, `live_wrapped` and `back_to_place` did not list `GraphRow`. They were harmless while it was a landing only, and would be wrong once it is every row. Added.

### Settle test 3 (committed with this change, before nothing it decides changed)
None of the three pickers stands for a commit row, so all keep `PlainRow`:
- `view/rows.rs` `opening_view`: no history yet. Test: `view::rows::tests::the_opening_view_bar_is_a_plain_row`.
- `view/rows/hint_context.rs` (`cursor` is `None`): no cursor row drawn. Test: `view::rows::tests::no_drawn_cursor_row_picks_a_plain_row`.
- `update/commit_editor/done_row.rs` `bar_row`: stands for "any bar that is not a landing"; only landing-ness matters. Test: `done_row::tests::off_a_landing_the_done_row_asks_with_a_plain_row`.

### What split (i) built
- **`live_branch`** is in `refs/live.rs`.
  - The index is `LiveNames`, built in `refs::badges` (local branches) and `push_remotes_tags_and_stash` (every remote-tracking ref except `*/HEAD`, badged or not). It is boxed in `RefBadges` to keep `AppEvent` small (clippy `large_enum_variant`).
  - Lookup order: local exact, then **remote shorthand exact** (`origin/x`), then `<remote>/<label>`, first remote listed wins. The middle step is beyond the spec: a lane named by a remote shorthand must resolve.
- **`row_bar`** is in `view/rows/row_bar.rs` (`RowBranch`, `MergeFold::of(FoldMark)`, `cursor_row_branch`).
  - An inner row's branch is `InsideMerge.label` → `live_branch`.
  - Any other row's branch is `made_on` → `view::entries::branch_name` (new; the branch column's `names_of` for one lane) → `live_branch`.
  - The result is `Own` iff the label is a `Branch { current: true }`.
- **Choice site** is `view/rows/context.rs` `cursor_row_context`: `landed` first, then the new rule.
  - **Decision (not in the spec):** while an operation is live (`live_c`), the base is `PlainRow`. The ease-pass bars have no `c`, so `c continue` vanished at stops, and they offer `C switch to`/`m merge`, which a stopped operation refuses.
  - That decision was caught by `m7_walk::rewrite_stops::capture_edit_stop_sizes` and `m5_walk journey_back_after_esc`. Question for the conductor or usability walk: should a stop get its own ease-pass bar?
- **`GraphEntry.badges` is kept:** `row_text.rs`, `row_cells.rs` and `view/preview.rs` still read it.

### Bar per row kind: state tests (`tests/it/row_bars.rs`, `wallet_release` fixture)
- Own row and trunk row: `his_own_row_and_the_trunks_row_take_their_branchs_bar`.
- Folded `Known(4)`: `a_folded_merge_with_its_count_says_what_space_shows`. `Known(3)` words are 03r's `each_row_bar_draws_the_mock_bar_at_120_columns`.
- Opened merge, an inner row of a deleted branch (no `C switch`, no `space`), and `space` there closes the merge with the cursor on the merge row: `the_opened_merge_and_its_rows_from_a_deleted_branch`.
- Inner row of the live `sara/fix-toc`: `an_inner_row_of_a_live_branchs_merge_offers_its_switch`.
- Row verbs (`o take`) sit after `⏎ open` as on any row.
- **Not covered by a state test:** `Pending` → the branch's bar. Only the unit test `row_bar_table` covers it, because the count races the first paint. So box 4 stays unticked. `space` on a `Pending`/`Failed` closed merge still acts; listed for the usability walk.

### Knock-on (Touches)
- Every graph row bar changed app-wide. 67 test-read captures under `docs/shots/` were redrawn (`MWR_UPDATE_SHOTS=1`). I spot-read the bar-line diffs and they read as the mock bars. **Not judged by eye** (rule 6); colour unverified (rule 5).
- **Stale captions (textual, not fixed; old milestones):** these still describe the old bars in their `#` header lines: `docs/shots/m5-hints-{branch,detail,plain}-{80x24,120x40,160x48}.txt` and `docs/shots/m7/bar-amend-row-187x44.txt`.
- Test expectations rewritten for the new bars:
  - `context_menu::tests::heads_row_hints_a_amend`.
  - `hint_bar::{the_tip_row_hints_the_current_branch_context, the_parent_row_hints_its_own_branch_bar (renamed), the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer}`, plus the marked-doorway caption and assert: `? keys` is now drawn at 120.
  - `m5_walk regressions` (current-branch and parent rows).
  - `keymap_edge::remaps::capture_merge_remapped_to_shift_m`. The fixture's only non-own row is a folded merge, so the `M merge` chord is now checked on a `GraphRow Elsewhere` bar through `hints_for`.
- `over_400.txt`: `hint_bar.rs` lowered to 965. MAP.md: `refs::live`, `view::rows::row_bar`, `entries::branch_name`.
- No `.snap` edited.

### Remaining (for the conductor to split)
1. **`BranchRow` removal.** It has no picker left (`context.rs` was its only one). It is still matched in ui `hints/{context,lists,lists/rows,doorway,filters,catalogue,catalogue/forms}.rs`, about 40 ui test sites, app `view/rows/context.rs` (3 matches) and the `dead_keys`/`sweep_screens` sweeps. Mechanical, but the tests that assert BranchRow words must be deleted or moved to `GraphRow`.
2. Settle test 1 (contains read at rest).
3. Second line, `body_line`/`commit_rows`, scroll, hit-tests.
4. The seven captures in box 9.

Commit order: `acd748a6` (settle test 2 pin), then this commit (ruling fix, settle 3, live_branch, row_bar, switch).

Acceptance: 2/9 ticked (boxes 2, 3). Box 1 is waiting on settle 1. Box 4 holds except `Pending` through the choice site. Box 5 is waiting on the removal.
