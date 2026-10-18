# graph-mode — two graph modes, `Folded merges` (default) and `Every branch`; read from `[graph] merged_branches`, switchable while running; a live search's folded count re-asked on every view change

Status: ready (reconciled 2026-10-18 with 06c as built: no `OnFold`, `open_merge` pure, `FoldedAsk`/`FoldedHits`, `Search.folded`; gap `ease/06c-suffix-stale` added)
Replaces: `06h-merged-branches-setting.md` (conductor: `git rm` it; this spec and `06i-graph-mode-in-help.md` take its place).
Blocked by: none open. Built and merged: 06r (9394aebb; its open boxes are questions Q1/Q2, no code this ticket reads), 06b (fcc5b7bd, `lane_phrases`/`LaneTally`), 06u (92821959, shape unchanged), 06c (40a10255; its open box is the `m8/search-hint` D3 stop, owned by 14). Runs before 06s/06t (they split files this edits).
Build with: Sonnet — config read, the walk's starting view, one switch through the existing view-change path, and one ask call at that path; no engine change (the unfolded inclusion exists: `fold: None`); every behaviour is a state or worker test. Conductor confirms the model after the scout (one finding below makes it a stop, not a harder build).
Needs: from 06b `lane_phrases(LaneTally) -> Vec<String>` (crate-private, graft-ui `graph_screen::header_tail`), `LaneTally { drawn, open, folded_in_view }`, `LaneTally::of(&Fold, &[GraphEntry])`. From 06c (`crates/graft/MAP.md` `history`, `history::sought`, `search`, `update::search`, `update::keys`): `Included::whereabouts(Oid) -> Whereabouts` (graft-graph); `Found { Row, Folded { behind }, NotInView }`, `Sought { anchor_behind, found }` on `Window.sought`; `History::take_reveal`; `History::ask_folded` / `folded_ask_out` / `ask_again`; `RowRequest.folded: Option<FoldedAsk>`; `HistoryUpdate::FoldedHits { search, folded, unwalked }`; `Search.folded` / `unwalked` / `folded_ask` (`search.rs:278-284`), footer suffix in `Search::line` (`search.rs:353-357`); app ask fns `update/search.rs` `ask_folded` (:260, a finished search with hits only), `ask_folded_now` (:279), `walk_completed` (:295), `folded_hits` (:312, newest ask number only); `view_change::open_merge(&View, Oid) -> Option<View>` (pure); the reveal's own ask at `update/history.rs:88` (`reveal()`), placed **before** its view change. Existing: `View::folded` (MAP `history`), `Inclusion { fold: Option<Folding> }` and `opened`/`keep_mut` → `None` when not folding (graft-graph MAP `inclusion`), `Config` `[graph]` (`config::types`), `keymap_from` (every config problem as words), `update::keys::view_change` (`1`/`H`/`space`, and its `change_view`), `history::fold::lay_out`, `update::fold_count`, graft-ui `Filters` (filled in `graft::view::rows`).
Scout: needed (brownfield, ADR-0035), one page. 06c's NOTES already answer where `open_merge`, `reveal()` and the ask fns are; don't re-scout those. Ask:
1. Where `Config`'s `[graph]` is typed and how a bad value stops today; who builds the problem list the status row and `YOUR CONFIG` read.
2. Every production caller of `View::folded()` (walk start, re-walk after an operation, `1`/`H` return); where `History` is built from config.
3. **`change_view`**: every view change that goes through it (`space` open, `space` close, `1`, `H`), any that doesn't (the reveal doesn't, by design — 06c NOTES b), and where in it the `RowRequest` is built (the ask must be set before that line).
4. **How a view change keeps the cursor's commit across the re-walk**, and how `space`'s close lands the cursor on the merge (G4): an anchor on the request answered through `Sought.anchor_behind`, or a `History::seek`. Since 06c a seek on a folded commit **reveals** (no `OnFold`, 06c NOTES deviation a). If the only route is a seek → report it; the conductor routes it to the planner before the build (the switch must not open a merge).
5. Where render calls `lane_phrases` and how `Filters` reaches it.

## Read first
- `docs/design-notes/ease-pass.md` :20 ("Graph mode in the `?` list"); Hard-to-reverse choice 2 ("Settled 2026-10-18"); Mock behaviours G4, G10, M18.
- Mock: `prototypes/ease-mocks/mergewright-show-all-branches.html` script :70-74 (top line words), :116-117 (bar with/without `space`). Build to it (memory: build to the mocks).
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Scope bullets 1 and 4 (`space`); "Decisions for continuation 2" D2 (HEAD keep). `06b` Decided ("Words", "Fitting", "Shared slot with 06h"). `06c` Decided ("Search folded count"), NOTES "Where this differs" (a–d), NOTES second group, "Gap, not built".
- Scout file (above).
- `docs/design/02-keymap.md` §2.2 :179-195 (config problems: warned, never fatal), §2.3 :276, :283 (`Space`, fold default).
- `README.md` :105-146 (Configuration; `[graph]`).
- `crates/graft/MAP.md`: `config`, `config::types`, `config::keys`, `cli::config_keys`, `cli::man`, `history` (`View`, `RowFold`, `History::ask_folded`), `history::sought`, `history::fold`, `search`, `update::search`, `update::keys`, `update::fold_count`, `update` (pure), `view::rows`. `crates/graft-graph/MAP.md` `inclusion` (`Whereabouts`). `crates/graft-ui/MAP.md` `graph_screen` (`Filters`, `LaneTally`).

## Why
User, 2026-10-18 (field notes :115-116): "no, not as easy as a key, but a setting like a mode... extra keys would be bad for not so important things"; then "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?" — chose: pick it in the `?` list, no key, the app keeps the pick (06i). This ticket builds the two modes and the switch; 06i puts them in `?` and writes the pick.

Gap `ease/06c-suffix-stale` (broken): `/topic` says `4 matches · 3 in folded merges`; `space` opens a merge holding 2 of them and the footer still says 3 until the next reveal (06c NOTES, "Gap, not built"). The graph-mode switch changes the fold set wholesale, so it would leave the same stale count. Fixed here, once, at the path every view change takes.

## Decided here
- **Names.** Modes `Folded merges` (default) / `Every branch` (mock :80-81). Stored as `[graph] merged_branches = "folded" | "shown"` — an enum, not a bool: a bad value can name both choices, a later mode needs no rename. Read once at startup; the running app changes it only through `switch_graph_mode` (06i calls it). Hand-editing the file still works (takes effect next launch).
- **Backing:** Every branch = the unfolded inclusion that exists (`fold: None`, what `View::default()` lays out). **The legacy `Inclusion.folded` does not come back** (retired by 06 continuation 1, 06 NOTES :138, Touches :168). Step 1 re-runs `grep -rn 'inclusion.folded\|folded: \[' crates`; a hit is a stop, not a revival.
- **Every branch, what follows from `fold: None`** (no new rule; each pinned by a test):
  - No row folds: every `RowFold.folds` is `None`, no `▸` tail, no fold count at rest, no `FoldWalk` memory.
  - **`space` does nothing** on any graph row and at the edit stop (§2.6a): 06's rule unchanged (acts only on `folds == Some(true)` or inside an opened merge). Bar and `?` offer `space` as a plain row in folded mode does today — mock :117: no `space open` when every branch is shown.
  - **Header (06b + mock :72):** K = 0 (no closed fold mark in view) so `merged branches folded` never shows; in its place, `every branch shown`, always, while this mode is on (render status by state, CLAUDE.md). 06b's slot 2: same `Drop::Lanes` item, same drop order (dropped first, whole). `lanes drawn: N of M` (slot 1) unchanged — 06b's lane drop applies in both modes.
  - **Search (06c):** `whereabouts` never answers `Folded`, so `Found::Folded` never comes, `take_reveal` stays `None`, `n`/`N` open nothing; every `FoldedHits` answers `folded: 0`, so no ` · N in folded merges` (the suffix shows only while `Search.folded > 0`).
  - **After a merge lands (M18):** no `space show the N commits inside`.
  - **HEAD keep (06 D2):** no fold, no keep; a refs read never re-lays out.
  - **`1` / `H`** return to the mode's view, not `View::folded()`.
  - **Promotion:** `Sought.anchor_behind` is never set (nothing hides).
- **The switch** (`switch_graph_mode`, through `change_view`'s re-walk): changes `inclusion.fold` only — `follow` and every other filter kept (a switch inside `1` stays in `1`). `OpenedMerges` cleared both ways (none opened after a switch to folded). Cursor keeps its commit; switching to folded with the cursor on a commit that folds away → lands on the merge that holds it (`Whereabouts::Folded { behind }`), **nothing opened** — the same landing as `space` closing a merge (G4) and as the promotion anchor (06c). **Never through `History::seek`**: since 06c a seek reveals (scout item 4). Same mode as now → no re-walk, no command. Not an operation: no journal entry, `u` does not undo it.
- **Folded count re-asked on every view change (gap `ease/06c-suffix-stale`).** `change_view` calls `update::search::ask_folded` (the :260 fn: does nothing unless a search has finished with hits) **before** it builds its `RowRequest`, so the ask rides on the view change's own request — the pattern `reveal()` uses at `update/history.rs:88`. One site covers `space` open, `space` close, `1` both ways, `H` both ways and `switch_graph_mode` both ways (the last goes through `change_view`). **No extra row request** (the view change sends one anyway; `ask_folded_now`'s `ShowRows` is not used here — it would set `outstanding` and read as reading, 06c NOTES Touches). The worker answers under the **new** view (the request carries it). `folded_hits` keeps only the newest ask, so an ask the change superseded is dropped. A view change that doesn't reach `change_view` (scout item 3) gets the same call at its own request site, named in NOTES. The reveal keeps its own ask and gains none (06c's reveal tests assert one row request — unchanged). Each re-ask is the view's answer: `space` opening a merge holding 2 of 3 folded hits → `· 1 in folded merges`; closing it → `· 3`; `1` → no suffix (off-line hits are `Filtered`, 06c box); `1` again → `· 3`; switch to Every branch → no suffix; back → `· 3`.
- **Bad value → a config problem, never fatal.** `= "open"`, `= true`, `= 3`: one problem, `graph.merged_branches = "open" — takes "folded" or "shown"; using folded`; status row `config: N problems · ? shows them`; listed in `?` under `YOUR CONFIG` like `[keys]`/`[theme]` problems (§2.2). Opens folded. Departs from README :110-111 ("a value a key does not take stops Mergewright…"): read off the raw table as `config::keys` reads `[keys]`, still on the loader's known-key list. README gains the exception.

## Scope
In: graft `config::types`, `config::load` known keys, `keymap_from`'s problem list, `cli::config_keys`/`cli::man`, `history::state` (`View::starting`, `History::merged_branches`), every production `View::folded()` caller, `update::keys::view_change` (`switch_graph_mode`, the ask in `change_view`), the M18 bar verb if offered unconditionally, `view::rows` (`Filters.every_branch` fill); graft-ui `graph_screen` `Filters` + `lane_phrases` and its render call; `README.md` config block and :110-111; tests for the re-ask in `update::search` / `update::keys` and the worker.
Out: graft-graph changes; new `space` behaviour; live reload of the file; the `?` group, the file write and the flash (06i); `LaneTally` and the lane counts (06b); the ask fns themselves and `FoldedHits` counting (06c, built); the `m8/search-hint` re-shoot (14); the Esc-closed search's ask riding until answered (06c review minor; not this gap); design-doc wording — text in NOTES for the conductor: §2.2 :188-189, §2.12 :581, `01-view-mockups.md` :525 "`[keys]`/`[theme]`" → "`[keys]`, `[theme]` or `graph.merged_branches`"; §2.3 :283 "unless the graph mode is Every branch"; PROMPT §1.3 same clause.

## Signatures
graft `config::types`:
```rust
/// `[graph] merged_branches`: the graph mode. Folded merges, or every branch on its own lane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MergedBranches {
    /// `"folded"`: behind their merge, `▸ N commits from …`; `space` opens one. The default.
    #[default]
    Folded,
    /// `"shown"`: every merged branch on its own lane; nothing folds.
    Shown,
}
impl MergedBranches {
    pub const KEY: &'static str = "graph.merged_branches";
    /// Unset → `Folded`. A value it doesn't take → `Err(problem words)`; the caller uses `Folded`.
    pub fn from_config(raw: Option<&toml::Value>) -> Result<Self, String>;
    /// The value the file spells: `"folded"` / `"shown"` (06i writes it).
    pub fn as_config(self) -> &'static str;
}
```
`Config`'s `[graph]` gains `merged_branches: MergedBranches`; the `Err` words join `keymap_from`'s `Vec<String>` (scout names the line if another list feeds the status row).
graft `history::state`:
```rust
impl View {
    /// The view a walk starts in: `Folded` → `View::folded()`; `Shown` → `inclusion.fold == None`.
    pub fn starting(merged: MergedBranches) -> Self;
}
impl History {
    pub fn merged_branches(&self) -> MergedBranches;
}
```
graft `update::keys::view_change`:
```rust
/// Switch the graph mode for this run (06i calls it on ⏎). Through `change_view`, so a live
/// search's folded count is re-asked on its request. Same mode → `vec![]`.
pub(crate) fn switch_graph_mode(state: &mut AppState, to: MergedBranches) -> Vec<Command>;
```
`change_view` keeps its signature; it gains one call to `update::search::ask_folded` (06c, unchanged signature) before its `RowRequest` is built. `open_merge(&View, Oid) -> Option<View>` unchanged.
graft-ui `graph_screen`: `Filters` gains `pub every_branch: bool` (filled in `graft::view::rows` from `History::merged_branches() == Shown`). 06b's fn gains the mode:
```rust
/// Slot 1 `lanes drawn: N of M` (while N < M); slot 2 `every branch shown` when `every_branch`,
/// else `K merged branches folded` (while K > 0).
pub(crate) fn lane_phrases(tally: LaneTally, every_branch: bool) -> Vec<String>;
```
Render passes `view.filters.every_branch`.
After this ticket `grep -n 'View::folded()' crates/graft/src` outside tests shows only `View::starting`'s body.

## Touches
Each checked, and how, in the report's **Touches:** line:
- config: `config::types`, `config::load` known keys (no unknown-key warning — test), `cli::config_keys` pairing, `cli::man` CONFIGURATION, `README.md` :110-111 and :125-129.
- Config problems: status row, `?` `YOUR CONFIG` (`m8-keymap-config-problem-120x36` re-shot only if its fixture's count changes; D3 rule).
- Every production `View::folded()` caller; `1`/`H`; promotion (`Sought.anchor_behind` never set when nothing hides).
- `change_view`'s callers (grep `change_view(`): each now re-asks; list each. `reveal()` (`update/history.rs`): untouched, its one-request tests pass. `ask_folded_now` / `walk_completed`: unchanged.
- The worker's `folded_hits` (`history/serve.rs`) on a request that also changes the view: answers under the new view (test below; if it answers under the old one, stop and report the line — that is 06c's code).
- Header `is_reading` / the `Esc` ladder: no new request, so unchanged (the request-count test below shows it).
- `with_head_kept` (06 D2), `m7_walk::pull::pull_ff_over_edits` (phase stays `Complete`).
- `space` handler and edit stop (`edit_stop_tests`); `RowBar::{FoldedMerge, Opened, JustMerged}`.
- `update::fold_count::at_rest`, `update::rest_owed`; `history::fold::lay_out` on a non-folding view.
- 06b header (graft-ui `lane_phrases` table, its render caller, the fitting table); every `Filters { .. }` literal (new field); 06c search footer and seek routes (`.seek(` callers in 06c NOTES).
- Captures that search then press `space`, `1` or `H` (grep the capture tests for `/` followed by those keys): the suffix may now change after the key — each re-shot with its D3 class, or "none" with the grep. `m8/search-hint` / `m8-bar-after-search`: not re-shot here (14).
- Pixel strips (`view::strips`): unfolded = the pre-ease-pass picture apart from 06b's node-less lane drop and 06u's connector columns; say if a strip key changes.
- Lane cap, rail, `+`.
- Tour: honours the mode; grep its steps for a folded-row or `space` dependency.
- AppState size cliff (06 NOTES :116) only if a field is added there; the mode lives in `History`.
- Keymap: none (no key). Journal/undo: none (test). `docs/capabilities/m1-the-graph.md` M1-09: cite the mode's test if the checker requires it.
- Gap `ease/06c-suffix-stale`: NOTES names the tests that close it (conductor moves the row).

## Acceptance
- [ ] Unset, `"folded"`, `"shown"` → `Folded`, `Folded`, `Shown`; `as_config` round-trips. — names its test.
- [ ] `"open"`, `true`, `3` → `Err` with the exact words in Decided; the app starts folded. — names its test.
- [ ] Config `merged_branches = "open"`: status row `config: 1 problem · ? shows them`; `?` lists `YOUR CONFIG` first with the words; the graph folds (a `▸` tail). — names its test.
- [ ] `"shown"` raises no unknown-key warning; `cli::config_keys` pairing passes. — names its test.
- [ ] Shown, 06e's `wallet_release` through the worker: first window holds every merged-branch commit; row ids equal a `View::default()` layout; every `RowFold.folds == None`; no tail. — names its test.
- [ ] Header: `lane_phrases` table extended — (5,8,0, every) → `["lanes drawn: 5 of 8", "every branch shown"]`; (8,8,0, every) → `["every branch shown"]`; (8,8,3, folded) → `["3 merged branches folded"]`; (8,8,0, folded) → `[]`. 06b's fitting table: `every branch shown` dropped whole at the room `K merged …` drops. Shown at 120 → top line ends `every branch shown` (no `merged branch` phrase). — names its tests.
- [ ] Shown: `space` on a merge row, a plain row, at an edit stop → view unchanged, no command. — names its test.
- [ ] Shown: rests ask no `CountFolded`; no rest wake owed. — names its test.
- [ ] Shown: a refs read leaves view and phase `Complete`. — names its test.
- [ ] Shown: `1` `1` and `H` return to the unfolded view. — names its test.
- [ ] Shown: search, 4 hits on merged-branch commits → `4 matches`, no suffix; `n` visits all 4, `OpenedMerges` empty, `take_reveal` `None` at every step. — names its test.
- [ ] Shown: after a merge lands the bar has no `space` verb; folded keeps `space show the N commits inside`. — names its test.
- [ ] `switch_graph_mode`: folded → shown keeps the cursor's commit; shown → folded with the cursor on a folded-away commit lands on its merge with `OpenedMerges` empty and no `Find` sent; inside `1` keeps `follow`; an opened merge is closed after; same mode → no commands; journal unchanged. — names its tests.
- [ ] **Re-ask, app (gap `ease/06c-suffix-stale`):** search done, 4 hits, 3 behind two merges (06c's `folded` fixture shape), footer `4 matches · 3 in folded merges`. Each of `space` open (merge holding 2), `space` close, `1`, `1`, `H`, `switch_graph_mode(Shown)`, `switch_graph_mode(Folded)`: exactly one `RowRequest` sent (as before the change), carrying a `FoldedAsk` with a new number over the 4 ids; fed the answer for the new view, the line reads, whole, `· 1 …`, `· 3 …`, no suffix, `· 3 …`, (`H`: unchanged count), no suffix, `· 3 …`; an answer to the superseded number is dropped. No search, or a search still running → no ask on the request. — names its test(s) in `update::search` or `update::keys`.
- [ ] **Re-ask, worker:** a `RowRequest` that changes the view and carries a `FoldedAsk` answers `FoldedHits` under the new view — `space`-opened merge: its hits not counted; first-parent: 0; `fold: None`: 0 — and lays out no further than the request needs (06c's no-further rule). — names its test in `history::idle_tests` or beside it.
- [ ] `reveal()` unchanged: 06c's `update::history::tests::reveal` tests pass as they are (one row request per reveal). — names the run.
- [ ] No config: every 06/06b/06c/06e/06g/06u test passes unchanged. — names the run.
- [ ] Captures `docs/shots/ease-pass/main-merged-shown-120.txt` (`wallet_release`, shown, 120×36, cursor on the first merge; top line ends `every branch shown`) and `config-merged-branches-problem-120.txt` (`"open"`: status row, then `?` open), in the contact sheet beside `main-folded-120.txt`. Look unjudged (rule 6), colour unverified (rule 5).
- [ ] NOTES: the step-1 grep (expected empty), the scout's item-4 answer (how the switch keeps the cursor without a seek), the design-doc wording, each re-shot capture with its D3 class, the tests closing `ease/06c-suffix-stale`.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~45k (the re-ask adds one call and two tests).
