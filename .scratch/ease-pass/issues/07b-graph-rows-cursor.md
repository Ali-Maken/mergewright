# graph-rows-cursor — the cursor row's second line, scroll and hit-test around it, the bar per row kind

Status: ready (split from 07 at its old (a)/(b) point, planner spec-only pass 2026-10-18, after scout `.scratch/ease-pass/scouts/07-graph-rows.md` at e9b08d12; mismatches 2, 4, 5 and the two *could not confirm* items settled below)
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
- [ ] `live_branch`: local hit, remote-only hit, deleted → `None` (unit tests); built from an index, not a per-call scan (NOTES names where `badges` builds it).
- [ ] `row_bar` table test: every `MergeFold` × {own, elsewhere, none}.
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
