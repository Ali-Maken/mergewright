# graph-mode — two graph modes, `Folded merges` (default) and `Every branch`; read from `[graph] merged_branches`, switchable while running

Status: draft
Replaces: `06h-merged-branches-setting.md` (conductor: `git rm` it; this spec and `06i-graph-mode-in-help.md` take its place).
Blocked by: 06r-fold-fixes, 06b-lanes-on-screen-and-header (`lane_phrases` / `LaneTally` asserted here), 06c-fold-seek-reveal (shares `history::state`; its search suffix and `Whereabouts` asserted here)
Build with: Sonnet — config read, the walk's starting view and one switch through the existing view-change path; no engine change (the unfolded inclusion exists: `fold: None`); every behaviour is a state test. Brownfield: scout first (ADR-0035); conductor confirms the model after it.
Needs: from 06b `lane_phrases(LaneTally) -> Vec<String>` (crate-private, graft-ui `graph_screen`), `LaneTally { drawn, open, folded_in_view }` (built at render from the applied fold and the rows in view); from 06c `Included::whereabouts -> Whereabouts`, the search hit summary's `folded: usize`. Existing: `View::folded` (`crates/graft/MAP.md` `history`), `Inclusion { fold: Option<Folding> }` and `opened`/`keep_mut` → `None` when not folding (`crates/graft-graph/MAP.md` `inclusion`), `Config` `[graph]` (`config::types`), `keymap_from` (every config problem as words), `update::keys::view_change` (`1`/`H`), `history::fold::lay_out`, `update::fold_count`, graft-ui `Filters` (filled in `graft::view::rows`).
Scout: needed — one page: where `Config`'s `[graph]` is typed and how a bad value stops today; who builds the problem list the status row and `YOUR CONFIG` read; every production caller of `View::folded()` (walk start, re-walk after an operation, `1`/`H` return); where `History` is built from config; how `view_change` re-walks and where it keeps the cursor's commit; where render calls `lane_phrases` and how `Filters` reaches it.

## Read first
- `docs/design-notes/ease-pass.md` :20 ("Graph mode in the `?` list"); Hard-to-reverse choice 2 ("Settled 2026-10-18"); Mock behaviours G4, G10, M18.
- Mock: `prototypes/ease-mocks/mergewright-show-all-branches.html` script :70-74 (top line words), :116-117 (bar with/without `space`). Build to it (memory: build to the mocks).
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Scope bullets 1 and 4 (`space`); "Decisions for continuation 2" D2 (HEAD keep). `06b` Decided ("Words", "Fitting", "Shared slot with 06h"). `06c` Decided (search count and order).
- `docs/design/02-keymap.md` §2.2 :179-195 (config problems: warned, never fatal), §2.3 :276, :283 (`Space`, fold default).
- `README.md` :105-146 (Configuration; `[graph]`).
- `crates/graft/MAP.md`: `config`, `config::types`, `config::keys`, `cli::config_keys`, `cli::man`, `history` (`View`, `RowFold`), `history::fold`, `update::fold_count`, `update` (pure), `view::rows`. `crates/graft-graph/MAP.md` `inclusion`. `crates/graft-ui/MAP.md` `graph_screen` (`Filters`, `LaneTally`).

## Why
User, 2026-10-18: "no, not as easy as a key, but a setting like a mode... extra keys would be bad for not so important things"; then "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?" — chose: pick it in the `?` list, the app keeps it (06i). This ticket builds the two modes and the switch; 06i puts them in `?` and writes the pick.

## Decided here
- **Names.** Modes `Folded merges` (default) / `Every branch` (mock :80-81). Stored as `[graph] merged_branches = "folded" | "shown"` — an enum, not a bool: a bad value can name both choices, a later mode needs no rename. Read once at startup; the running app changes it only through `switch_graph_mode` (06i calls it). Hand-editing the file still works (takes effect next launch).
- **Backing:** Every branch = the unfolded inclusion that exists (`fold: None`, what `View::default()` lays out). **The legacy `Inclusion.folded` does not come back** (retired by 06 continuation 1, 06 NOTES :138, Touches :168). Step 1 re-runs `grep -rn 'inclusion.folded\|folded: \[' crates`; a hit is a stop, not a revival.
- **Every branch, what follows from `fold: None`** (no new rule; each pinned by a test):
  - No row folds: every `RowFold.folds` is `None`, no `▸` tail, no fold count at rest, no `FoldWalk` memory.
  - **`space` does nothing** on any graph row and at the edit stop (§2.6a): 06's rule unchanged (acts only on `folds == Some(true)` or inside an opened merge). Bar and `?` offer `space` as a plain row in folded mode does today — mock :117: no `space open` when every branch is shown.
  - **Header (06b + mock :72):** K = 0 (no closed fold mark in view) so `merged branches folded` never shows; in its place, `every branch shown`, always, while this mode is on (render status by state, CLAUDE.md). 06b's slot 2: same `Drop::Lanes` item, same drop order (dropped first, whole). `lanes drawn: N of M` (slot 1) unchanged — 06b's lane drop applies in both modes.
  - **Search (06c):** `whereabouts` never answers `Folded`; no ` · N in folded merges`; `n`/`N` open nothing.
  - **After a merge lands (M18):** no `space show the N commits inside`.
  - **HEAD keep (06 D2):** no fold, no keep; a refs read never re-lays out.
  - **`1` / `H`** return to the mode's view, not `View::folded()`.
- **The switch** (`switch_graph_mode`, through `view_change`'s re-walk): changes `inclusion.fold` only — `follow` and every other filter kept (a switch inside `1` stays in `1`). `OpenedMerges` cleared both ways (none opened after a switch to folded). Cursor keeps its commit; switching to folded with the cursor on a commit that folds away → lands on the merge that holds it (06c `Whereabouts`). Same mode as now → no re-walk, no command. Not an operation: no journal entry, `u` does not undo it.
- **Bad value → a config problem, never fatal.** `= "open"`, `= true`, `= 3`: one problem, `graph.merged_branches = "open" — takes "folded" or "shown"; using folded`; status row `config: N problems · ? shows them`; listed in `?` under `YOUR CONFIG` like `[keys]`/`[theme]` problems (§2.2). Opens folded. Departs from README :110-111 ("a value a key does not take stops Mergewright…"): read off the raw table as `config::keys` reads `[keys]`, still on the loader's known-key list. README gains the exception.

## Scope
In (graft `config::types`, `config::load` known keys, `keymap_from`'s problem list, `cli::config_keys`/`cli::man`, `history::state`, every production `View::folded()` caller, `update::keys::view_change`, the M18 bar verb if offered unconditionally, `view::rows` (`Filters.every_branch` fill); graft-ui `graph_screen` `Filters` + `lane_phrases` and its render call; `README.md` config block and :110-111).
Out: graft-graph changes; new `space` behaviour; live reload of the file; the `?` group, the file write and the flash (06i); `LaneTally` and the lane counts (06b); design-doc wording — text in NOTES for the conductor: §2.2 :188-189, §2.12 :581, `01-view-mockups.md` :525 "`[keys]`/`[theme]`" → "`[keys]`, `[theme]` or `graph.merged_branches`"; §2.3 :283 "unless the graph mode is Every branch"; PROMPT §1.3 same clause.

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
/// Switch the graph mode for this run (06i calls it on ⏎). Same mode → `vec![]`.
pub(crate) fn switch_graph_mode(state: &mut AppState, to: MergedBranches) -> Vec<Command>;
```
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
- Every production `View::folded()` caller; `1`/`H`; promotion (`anchor_behind` never set when nothing hides).
- `with_head_kept` (06 D2), `m7_walk::pull::pull_ff_over_edits` (phase stays `Complete`).
- `space` handler and edit stop (`edit_stop_tests`); `RowBar::{FoldedMerge, Opened, JustMerged}`.
- `update::fold_count::at_rest`, `update::rest_owed`; `history::fold::lay_out` on a non-folding view.
- 06b header (graft-ui `lane_phrases` table, its render caller, the fitting table); every `Filters { .. }` literal (new field); 06c search footer and seek routes.
- Pixel strips (`view::strips`): unfolded = the pre-ease-pass picture apart from 06b's node-less lane drop; say if a strip key changes.
- Lane cap, rail, `+`.
- Tour: honours the mode; grep its steps for a folded-row or `space` dependency.
- AppState size cliff (06 NOTES :116) only if a field is added there; the mode lives in `History`.
- Keymap: none (no key). Journal/undo: none (test). `docs/capabilities/m1-the-graph.md` M1-09: cite the mode's test if the checker requires it.

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
- [ ] Shown: search, 4 hits on merged-branch commits → `4 matches`, no suffix; `n` visits all 4, `OpenedMerges` empty. — names its test.
- [ ] Shown: after a merge lands the bar has no `space` verb; folded keeps `space show the N commits inside`. — names its test.
- [ ] `switch_graph_mode`: folded → shown keeps the cursor's commit; shown → folded with the cursor on a folded-away commit lands on its merge; inside `1` keeps `follow`; an opened merge is closed after; same mode → no commands; journal unchanged. — names its tests.
- [ ] No config: every 06/06b/06c/06e/06g test passes unchanged. — names the run.
- [ ] Captures `docs/shots/ease-pass/main-merged-shown-120.txt` (`wallet_release`, shown, 120×36, cursor on the first merge; top line ends `every branch shown`) and `config-merged-branches-problem-120.txt` (`"open"`: status row, then `?` open), in the contact sheet beside `main-folded-120.txt`. Look unjudged (rule 6), colour unverified (rule 5).
- [ ] NOTES: the step-1 grep (expected empty), the design-doc wording, each re-shot capture with its D3 class.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~40k.
