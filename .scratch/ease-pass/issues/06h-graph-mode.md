# graph-mode — two graph modes, `Folded merges` (default) and `Every branch`; read from `[graph] merged_branches`, switchable while running; a live search's folded count re-asked on every view change

Status: done (2026-10-18, partial boxes: edit-stop `space` and `n`-visits in Every branch untested — see Acceptance). Was: ready (reconciled 2026-10-18 with 06c as built: no `OnFold`, `open_merge` pure, `FoldedAsk`/`FoldedHits`, `Search.folded`; gap `ease/06c-suffix-stale` added. Re-planned 2026-10-18 after the scout's stop: the cursor landing is a request anchor answered through `anchor_behind`, not a seek; `1`/`H` need no change; the problem list has two sources; `space` close's reveal question is a test.)
Replaces: `06h-merged-branches-setting.md` (conductor: `git rm` it; this spec and `06i-graph-mode-in-help.md` take its place).
Blocked by: none open. Built and merged: 06r (9394aebb; its open boxes are questions Q1/Q2, no code this ticket reads), 06b (fcc5b7bd, `lane_phrases`/`LaneTally`), 06u (92821959, shape unchanged), 06c (40a10255; its open box is the `m8/search-hint` D3 stop, owned by 14). Runs before 06s/06t (they split files this edits).
Build with: Sonnet — config read, the walk's starting view, one switch through the existing view-change path, one ask call per view-change site, and one request field the worker answers by the code promotion already uses (`anchor_behind`); no graft-graph change; every behaviour is a state or worker test. Scout (`.scratch/ease-pass/scouts/06h-graph-mode.md`) put it at Sonnet once the landing was settled; it is settled below.
Needs: from 06b `lane_phrases(LaneTally) -> Vec<String>` (crate-private, graft-ui `graph_screen::header_tail`), `LaneTally { drawn, open, folded_in_view }`, `LaneTally::of(&Fold, &[GraphEntry])`. From 06c (`crates/graft/MAP.md` `history`, `history::sought`, `search`, `update::search`, `update::keys`): `Included::whereabouts(Oid) -> Whereabouts` (graft-graph); `Found { Row, Folded { behind }, NotInView }`, `Sought { anchor_behind, found }` on `Window.sought`; `History::take_reveal`; `History::ask_folded` / `folded_ask_out` / `ask_again`; `RowRequest.folded: Option<FoldedAsk>`; `HistoryUpdate::FoldedHits { search, folded, unwalked }`; `Search.folded` / `unwalked` / `folded_ask` (`search.rs:278-284`), footer suffix in `Search::line` (`search.rs:353-357`); app ask fns `update/search.rs` `ask_folded` (:260, a finished search with hits only), `ask_folded_now` (:279), `walk_completed` (:295), `folded_hits` (:312, newest ask number only); `view_change::open_merge(&View, Oid) -> Option<View>` (pure); the reveal's own ask at `update/history.rs:88-90` (`reveal()`), placed **before** its view change. From 06 (built): the promotion anchor — `request.promote.anchor` answered as `Sought.anchor_behind: (anchor, behind)` in `history/serve.rs:331-337`, landed in `history/scroll/apply.rs:143-150`; test `history::reveal_tests::a_promoted_anchor_the_new_order_folds_lands_on_its_merge_and_opens_nothing`. Existing: `View::folded` (MAP `history`), `Inclusion { fold: Option<Folding> }` and `opened`/`keep_mut` → `None` when not folding (graft-graph MAP `inclusion`), `Config` `[graph]` (`config/types.rs:71-93`), `keymap_from` (`config/keys.rs:70`) and `theme_problems`, joined at `event_loop/startup/arrival.rs:45-46`, seeded `state.seed_keys` (:63), `History::change_view` (`history/scroll/apply.rs:247`; `reanchor` :251, landing :52-58), `update::keys::apply_view_change` (`keys.rs:436`, `change_view` at :447, `space`-close seek at :453), `history::fold::lay_out`, `update::fold_count`, graft-ui `Filters` (`graph_screen/mod.rs:189-197`, filled in `graft::view::rows`).
Scout: done — `.scratch/ease-pass/scouts/06h-graph-mode.md` (at 2d542fbd). Its stop (no view-change route lands on a merge) is answered in Decided, "Cursor landing". Re-read it for line numbers; don't re-scout.

## Read first
- `docs/design-notes/ease-pass.md` :20 ("Graph mode in the `?` list"); Hard-to-reverse choice 2 ("Settled 2026-10-18"); Mock behaviours G4, G10, M18.
- Mock: `prototypes/ease-mocks/mergewright-show-all-branches.html` script :70-74 (top line words), :116-117 (bar with/without `space`). Build to it (memory: build to the mocks).
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Scope bullets 1 and 4 (`space`); "Decisions for continuation 2" D2 (HEAD keep); :130 (`anchor_behind` boxed for the AppState size), :144. `06b` Decided ("Words", "Fitting", "Shared slot with 06h"). `06c` Decided ("Search folded count", "Promotion re-seek"), NOTES "Where this differs" (a–d), NOTES second group, "Gap, not built".
- Scout file (above), whole (one page).
- `docs/design/02-keymap.md` §2.2 :179-195 (config problems: warned, never fatal), §2.3 :276, :283 (`Space`, fold default).
- `README.md` :105-146 (Configuration; `[graph]`).
- `crates/graft/MAP.md`: `config`, `config::types`, `config::keys`, `cli::config_keys`, `cli::man`, `history` (`View`, `RowFold`, `RowRequest`, `History::ask_folded`), `history::sought`, `history::fold`, `history::serve`, `history::scroll`, `search`, `update::search`, `update::keys`, `update::fold_count`, `update` (pure), `view::rows`, `event_loop::startup`. `crates/graft-graph/MAP.md` `inclusion` (`Whereabouts`). `crates/graft-ui/MAP.md` `graph_screen` (`Filters`, `LaneTally`).

## Why
User, 2026-10-18 (field notes :115-116): "no, not as easy as a key, but a setting like a mode... extra keys would be bad for not so important things"; then "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?" — chose: pick it in the `?` list, no key, the app keeps the pick (06i). This ticket builds the two modes and the switch; 06i puts them in `?` and writes the pick.

Gap `ease/06c-suffix-stale` (broken): `/topic` says `4 matches · 3 in folded merges`; `space` opens a merge holding 2 of them and the footer still says 3 until the next reveal (06c NOTES, "Gap, not built"). The graph-mode switch changes the fold set wholesale, so it would leave the same stale count. Fixed here, at every view-change site.

## Decided here
- **Names.** Modes `Folded merges` (default) / `Every branch` (mock :80-81). Stored as `[graph] merged_branches = "folded" | "shown"` — an enum, not a bool: a bad value can name both choices, a later mode needs no rename. Read once at startup; the running app changes it only through `switch_graph_mode` (06i calls it). Hand-editing the file still works (takes effect next launch).
- **Backing:** Every branch = the unfolded inclusion that exists (`fold: None`, what `View::default()` lays out). **The legacy `Inclusion.folded` does not come back** (retired by 06 continuation 1, 06 NOTES :138, Touches :168). Scout's step-1 grep `grep -rn 'inclusion.folded\|folded: \[' crates` was empty at 2d542fbd; the builder re-runs it; a hit is a stop, not a revival.
- **Where the mode lives.** Derived, not stored, inside a walk: `History::merged_branches()` reads `view.inclusion.fold` (`Some` → `Folded`, `None` → `Shown`; in folded mode the fold is never `None`, scout item 2: the only production `View::folded()` is `History::started`). Before a walk exists the mode sits in one `AppState` field, `graph_mode: MergedBranches` (1 byte), seeded at `arrival.rs` beside `state.seed_keys` (:63); `History::started` (`state.rs:337`, called from `tree_answers.rs:97`) takes it and opens `View::starting(mode)`. A re-walk copies `previous.view` (`tree_answers.rs:105-107`), so the mode carries over by itself. `switch_graph_mode` sets the field too, so a repository opened later in the same run starts in the picked mode.
- **Every branch, what follows from `fold: None`** (no new rule; each pinned by a test):
  - No row folds: every `RowFold.folds` is `None`, no `▸` tail, no fold count at rest, no `FoldWalk` memory.
  - **`space` does nothing** on any graph row and at the edit stop (§2.6a): `ViewChange::Fold` exits at `fold_at(cursor)?` / `opened_mut()?` (scout, "space"; untested today — the test is new). Bar and `?` offer `space` as a plain row in folded mode does today — mock :117: no `space open` when every branch is shown.
  - **Header (06b + mock :72):** K = 0 (no closed fold mark in view) so `merged branches folded` never shows; in its place, `every branch shown`, always, while this mode is on (render status by state, CLAUDE.md). 06b's slot 2: same `Drop::Lanes` item, same drop order (dropped first, whole). `lanes drawn: N of M` (slot 1) unchanged — 06b's lane drop applies in both modes.
  - **Search (06c):** `whereabouts` never answers `Folded`, so `Found::Folded` never comes, `take_reveal` stays `None`, `n`/`N` open nothing; every `FoldedHits` answers `folded: 0`, so no ` · N in folded merges` (the suffix shows only while `Search.folded > 0`).
  - **After a merge lands (M18):** no `space show the N commits inside`.
  - **HEAD keep (06 D2):** no fold, no keep; a refs read never re-lays out.
  - **`1` / `H`** — no code change. They toggle `follow` / `hidden_stashes` on a clone of the current view (`view_change.rs:38, 61-79`, scout item 2) and already keep `inclusion.fold`, so in Every branch they stay unfolded. (The earlier "they reset to `View::folded()`" was wrong; struck.) `update/tests/graph_toggles.rs:27,143,198` assert `View::folded()` with no config — unchanged and still right.
  - **Promotion:** `Sought.anchor_behind` is never set (nothing hides).
- **Cursor landing on a view change (the scout's stop).** Today `change_view` sets `reanchor` (`apply.rs:251`); on the answer the cursor moves to it if it is in the new rows, else keeps its row index (`apply.rs:52-58`). A commit the new view folds away gets the index, not its merge. Decided: **the view change's request carries the reanchor commit, and the worker answers it exactly as it answers a promotion's anchor.**
  - `RowRequest` gains `keep: Option<Oid>`. Filled from `History.reanchor` on **every** request built while a reanchor is pending (as the `FoldedAsk` rides on every request until answered), so the `space`-close seek's request, which replaces `change_view`'s (`keys.rs:453`, `seek(..).or(request)`), still carries it.
  - `history/serve.rs:331-337`: the anchor it resolves becomes `request.promote.anchor` **or** `request.keep` (promotion first; both are never set together — a debug assertion). Same resolution (whereabouts / `fold::landing`), same lay-out-far-enough rule, same `Sought.anchor_behind = (anchor, behind)`. No second rule in the worker.
  - `history/scroll/apply.rs:52-58`: reanchor not in the new rows and `anchor_behind == Some((reanchor, behind))` with `behind` in the rows → cursor on `behind`. Otherwise unchanged (keeps its index — e.g. `1` hiding an off-line commit: `Filtered`, no `anchor_behind`). The promotion landing (`apply.rs:143-150`) and this one share one fn in `history::scroll` (name in NOTES); 06c's promotion test passes unchanged.
  - Nothing opens: no `Reveal`, no seek, `take_reveal` stays `None`. Same landing as `space` closing a merge (G4) and as the promotion anchor (06c). **Never through `History::seek`** (since 06c a seek on a folded commit reveals).
  - Applies to every `change_view`, not only the mode switch: closing a merge, the HEAD keep, the refs read. In every case today's landing is either unchanged (commit still shown) or improved (commit folded away → its merge, where it was a stale index).
  - **`space` close's own seek** (`keys.rs:453`) stays: out of scope to remove. With `keep` it is redundant in the common case (both land on `inside.merge`); NOTES says so for the conductor.
- **`space` close leaves no reveal owed (scout: unconfirmed).** The seek targets the merge, which the new view shows; `whereabouts` answers `Row`, so no `Reveal`. Pinned by a test with the merge row **off the window on hand** (a long opened merge, cursor at its bottom, so the seek sends a find): after the answer, cursor on the merge, `OpenedMerges` empty, `take_reveal()` `None`, exactly one `RowRequest`. If it fails (a reveal owed, or a merge re-opened), stop and report the line — that is 06c's code.
- **The switch** (`switch_graph_mode`, through `apply_view_change` → `History::change_view`): changes `inclusion.fold` only — `follow` and every other filter kept (a switch inside `1` stays in `1`). `OpenedMerges` cleared both ways (none opened after a switch to folded). Shown → folded: `View::starting(Folded)`'s fold with the current `follow`/filters, so the HEAD keep (06 D2) applies as on a fresh start. Cursor per "Cursor landing". Same mode as now → no re-walk, no command. Not an operation: no journal entry, `u` does not undo it. Sets `AppState.graph_mode`.
- **Folded count re-asked on every view change (gap `ease/06c-suffix-stale`).** Every `history.change_view(` call in `update` except `reveal()`'s is preceded by `update::search::ask_folded` (the :260 fn: does nothing unless a search has finished with hits), so the ask rides on the view change's own request — the pattern `reveal()` uses at `update/history.rs:88-90`. Sites (scout item 3): `update/keys.rs:447` (`apply_view_change`: `space` open/close, `1`, `H`, and `switch_graph_mode`), `update/tree_answers.rs:57` (`head_known`, HEAD keep), `update/tree_answers.rs:287` (refs read, stash/HEAD view). `reveal()` (`update/history.rs:92`) keeps its own ask and gains none (06c's reveal tests assert one row request — unchanged). **No extra row request** (each site sends one anyway; `ask_folded_now`'s `ShowRows` is not used — it would set `outstanding` and read as reading, 06c NOTES Touches). The worker answers under the **new** view (the request carries it). `folded_hits` keeps only the newest ask, so a superseded ask is dropped. If `tree_answers.rs:287` sends no `RowRequest` (a re-walk follows instead), `walk_completed`'s re-ask covers it — the test asserts whichever holds and NOTES says which. Each re-ask is the view's answer: `space` opening a merge holding 2 of 3 folded hits → `· 1 in folded merges`; closing it → `· 3`; `1` → no suffix (off-line hits are `Filtered`, 06c box); `1` again → `· 3`; switch to Every branch → no suffix; back → `· 3`.
- **Bad value → a config problem, never fatal.** `= "open"`, `= true`, `= 3`: one problem, `graph.merged_branches = "open" — takes "folded" or "shown"; using folded`; status row `config: N problems · ? shows them`; listed in `?` under `YOUR CONFIG` like `[keys]`/`[theme]` problems (§2.2). Opens folded. The `[graph]` struct holds the raw value (`Option<toml::Value>`), so serde never fails on it; `MergedBranches::from_config` turns it into the mode or the words. **The words join the list at its join site, `arrival.rs:45-46`, after `keymap_from`'s and `theme_problems`'** — not inside `keymap_from` (the list has two sources today, scout item 1; `keymap_from` stays keys-only). Departs from README :110-111 ("a value a key does not take stops Mergewright…"): still on the loader's known-key list (`load.rs:350`). README gains the exception.

## Scope
In: graft `config::types` (`MergedBranches`, `[graph]` raw field), `config::load` known keys, `event_loop/startup/arrival.rs` (problem join, `graph_mode` seed), `cli::config_keys`/`cli::man`, `history::state` (`View::starting`, `History::started(.., merged)`, `History::merged_branches`, `RowRequest.keep`), `history::serve` (anchor `promote.anchor` or `keep`), `history::scroll` (`apply.rs` reanchor landing, shared with promotion; `keep` filled on requests), `AppState.graph_mode`, `update::tree_answers` (`History::started` caller :97; ask before :57 and :287), `update::keys` (ask before :447) and `view_change` (`switch_graph_mode`), the M18 bar verb if offered unconditionally, `view::rows` (`Filters.every_branch` fill); graft-ui `graph_screen` `Filters` + `lane_phrases` and its render call (`layout.rs:247`); `README.md` config block and :110-111; tests listed in Acceptance.
Out: graft-graph changes; new `space` behaviour; removing `space` close's seek (`keys.rs:453`); `1`/`H` code; live reload of the file; the `?` group, the file write and the flash (06i); `LaneTally` and the lane counts (06b); the ask fns themselves and `FoldedHits` counting (06c, built); the `m8/search-hint` re-shoot (14); the Esc-closed search's ask riding until answered (06c review minor; not this gap); design-doc wording — text in NOTES for the conductor: §2.2 :188-189, §2.12 :581, `01-view-mockups.md` :525 "`[keys]`/`[theme]`" → "`[keys]`, `[theme]` or `graph.merged_branches`"; §2.3 :283 "unless the graph mode is Every branch"; PROMPT §1.3 same clause.

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
// in the existing `[graph]` struct (types.rs:71-93):
/// Raw, so a bad value is a problem, not a load failure. Read through `MergedBranches::from_config`.
#[serde(default)]
pub merged_branches: Option<toml::Value>,
```
graft `event_loop/startup/arrival.rs` (:45-46): `let (mode, problem) = match MergedBranches::from_config(cfg.graph.merged_branches.as_ref()) { Ok(m) => (m, None), Err(w) => (MergedBranches::Folded, Some(w)) };` — `problem` appended after the keys and theme problems; `state.graph_mode = mode` beside `seed_keys` (:63). (Field path `cfg.graph` as the struct names it.)
graft `history::state`:
```rust
impl View {
    /// The view a walk starts in: `Folded` → `View::folded()`; `Shown` → `inclusion.fold == None`.
    pub fn starting(merged: MergedBranches) -> Self;
}
impl History {
    /// Existing params unchanged; gains `merged`, opens `View::starting(merged)`.
    pub fn started(/* existing */, merged: MergedBranches) -> Self;
    /// From `view.inclusion.fold`: `Some` → `Folded`, `None` → `Shown`. No stored field.
    pub fn merged_branches(&self) -> MergedBranches;
}
pub struct RowRequest {
    // existing fields unchanged
    /// The commit a view change keeps the cursor on (`History.reanchor`). The worker answers
    /// `Sought.anchor_behind` for it as for a promotion's anchor, so a folded-away commit
    /// lands on its merge. `None` on `RowRequest::new`.
    pub keep: Option<Oid>,
}
```
graft `AppState`: `pub(crate) graph_mode: MergedBranches` (default `Folded`). The AppState size test (06 NOTES :116) must pass; `RowRequest` grows by `Option<Oid>` — if a size test trips, `Option<Box<Oid>>`, named in NOTES.
graft `update::keys::view_change`:
```rust
/// Switch the graph mode for this run (06i calls it on ⏎). Through `apply_view_change`, so the
/// cursor lands by `keep` and a live search's folded count is re-asked on its request.
/// Same mode → `vec![]`.
pub(crate) fn switch_graph_mode(state: &mut AppState, to: MergedBranches) -> Vec<Command>;
```
`History::change_view` and `apply_view_change` keep their signatures. `update::search::ask_folded` (06c) unchanged; called before each of the three `change_view` sites. `open_merge(&View, Oid) -> Option<View>` unchanged.
graft-ui `graph_screen`: `Filters` gains `pub every_branch: bool` (filled in `graft::view::rows` from `History::merged_branches() == Shown`). 06b's fn gains the mode:
```rust
/// Slot 1 `lanes drawn: N of M` (while N < M); slot 2 `every branch shown` when `every_branch`,
/// else `K merged branches folded` (while K > 0).
pub(crate) fn lane_phrases(tally: LaneTally, every_branch: bool) -> Vec<String>;
```
Render (`layout.rs:247`) passes `self.view.filters.every_branch`.
After this ticket `grep -n 'View::folded()' crates/graft/src` outside tests shows only `View::starting`'s body; `grep -n 'change_view(' crates/graft/src/update` shows each hit preceded by `ask_folded` except `reveal()`'s (which has its own at :88-90).

## Touches
Each checked, and how, in the report's **Touches:** line:
- config: `config::types`, `config::load` known keys (no unknown-key warning — test), `cli::config_keys` pairing, `cli::man` CONFIGURATION, `README.md` :110-111 and :125-129.
- Config problems: both existing sources (`keymap_from`, `theme_problems`) unchanged; the join at `arrival.rs:45-46`; status row, `?` `YOUR CONFIG` (`m8-keymap-config-problem-120x36` re-shot only if its fixture's count changes; D3 rule).
- `History::started` callers (`tree_answers.rs:97`; grep `History::started(` incl. tests — each passes a mode); every production `View::folded()` caller; `1`/`H` (no code change; `graph_toggles` tests pass as they are); promotion (`Sought.anchor_behind` never set when nothing hides).
- **The landing:** `RowRequest` literals and `RowRequest::new` callers (grep `RowRequest {` / `RowRequest::new`); `serve.rs:331-337`; `apply.rs:52-58` and `:143-150`; 06c's promotion test unchanged; every `change_view` caller's landing (`keys.rs:447` `space`/`1`/`H`, `tree_answers.rs:57`, `:287`, `history.rs:92` reveal — reveal's seek lands it, `keep` must not move the cursor off the revealed target: test); `space` close's seek `keys.rs:453` (still lands on the merge).
- `change_view`'s callers (grep `change_view(`): each of the three now re-asks; list each. `reveal()` (`update/history.rs`): untouched, its one-request tests pass. `ask_folded_now` / `walk_completed`: unchanged.
- The worker's `folded_hits` (`history/serve.rs`) on a request that also changes the view: answers under the new view (test below; if it answers under the old one, stop and report the line — that is 06c's code).
- Header `is_reading` / the `Esc` ladder: no new request, so unchanged (the request-count tests below show it).
- `with_head_kept` (06 D2), `m7_walk::pull::pull_ff_over_edits` (phase stays `Complete`).
- `space` handler and edit stop (`edit_stop_tests`); `RowBar::{FoldedMerge, Opened, JustMerged}`.
- `update::fold_count::at_rest`, `update::rest_owed`; `history::fold::lay_out` on a non-folding view.
- 06b header (graft-ui `lane_phrases` table, its render caller `layout.rs:247`, the fitting table, `graph_screen/tests/lanes_in_view.rs:73-78`); every `Filters { .. }` literal (new field); 06c search footer and seek routes (`.seek(` callers in 06c NOTES).
- Captures that search then press `space`, `1` or `H` (grep the capture tests for `/` followed by those keys): the suffix may now change after the key — each re-shot with its D3 class, or "none" with the grep. Captures that close a merge or toggle `1` with the cursor on a commit that then hides: the cursor row may now move to the merge — grep and re-shoot with D3 class, or "none". `m8/search-hint` / `m8-bar-after-search`: not re-shot here (14).
- Pixel strips (`view::strips`): unfolded = the pre-ease-pass picture apart from 06b's node-less lane drop and 06u's connector columns; say if a strip key changes.
- Lane cap, rail, `+`.
- Tour: honours the mode; grep its steps for a folded-row or `space` dependency.
- AppState size test (new `graph_mode` field; `RowRequest.keep` if held there).
- Keymap: none (no key). Journal/undo: none (test). `docs/capabilities/m1-the-graph.md` M1-09: cite the mode's test if the checker requires it.
- Gap `ease/06c-suffix-stale`: NOTES names the tests that close it (conductor moves the row).

## Acceptance
- [x] Unset/"folded"/"shown" → modes; `as_config` round-trips. — `config::merged_tests::unset_folded_and_shown_read_as_their_modes_and_round_trip`.
- [x] `"open"`, `true`, `3` → `Err` with the exact words; file still loads. — `config::merged_tests::a_value_it_does_not_take_is_a_problem_in_words_and_the_file_still_loads`; starts folded: `event_loop::tests::theme_problems::a_graph_mode_it_does_not_take_is_the_last_problem_and_the_app_starts_folded`.
- [x] Three problems, status row `config: 3 problems · ? shows them`, `YOUR CONFIG` first, graph problem last, walk starts `View::folded()`. — `event_loop::tests::theme_problems::a_graph_mode_it_does_not_take_is_the_last_problem_and_the_app_starts_folded`. Note: the "graph folds (a ▸ tail)" half is asserted as `History::started(state.graph_mode).view == View::folded()` on a synthetic history, not a drawn `▸`.
- [x] `"shown"` no unknown-key warning; `cli::config_keys` pairing. — `config::merged_tests::shown_raises_no_unknown_key_warning`, `cli::docs_tests::config_keys_and_the_loader_name_the_same_keys`.
- [x] Shown `wallet_release` through the worker: every merged commit in first window, view == `View::default()`, every `folds == None`, no tail. — `it::graph_mode::every_branch_first_paint_draws_every_merged_commit_and_says_so` (row-id equality shown as view equality with `View::default()`).
- [x] `started(Shown)`/`started(Folded)`; re-walk keeps mode. — `history::keep_tests::a_walk_starts_in_its_mode_and_the_mode_is_read_from_the_view`, `update::tests::graph_mode::the_same_mode_again_or_no_walk_sends_nothing`.
- [x] Header `lane_phrases` table and fitting; top line at 120 ends `every branch shown`. — mergewright-ui `graph_screen::tests::lanes_in_view::{the_lane_words_say_only_what_there_is_to_say, every_branch_shown_takes_the_folded_counts_slot_always, every_branch_shown_drops_whole_with_the_lane_phrases}`; `it::graph_mode::every_branch_first_paint_…`.
- [ ] Shown `space`: merge row and plain rows → nothing — `update::tests::graph_mode::every_branch_space_does_nothing_on_any_row`. **Edit stop (§2.6a) not tested**: open.
- [x] Shown: no `CountFolded` at rest, none owed. — `update::tests::graph_mode::every_branch_asks_no_fold_count_at_rest_and_offers_no_space_after_a_merge` (with a folded contrast at the same size).
- [x] Shown: refs read leaves view and `Complete`. — `update::tests::graph_mode::every_branch_a_refs_read_relays_nothing`.
- [x] Shown `1 1`, `H H` keep `fold == None`; folded `graph_toggles` unchanged. — `update::tests::graph_mode::every_branch_first_parent_and_stash_toggles_stay_unfolded`; full nextest run.
- [ ] Shown search: suffix none when the count answers 0 — `update::tests::graph_mode_reask::every_view_change_asks_the_folded_count_again` (switch to Shown → `4 matches`); worker counts 0 under `fold: None` — `history::idle_tests::keep_tests::a_folded_ask_on_a_view_change_is_counted_under_the_new_view`. **`n` visiting all 4 with `take_reveal` None at each step not tested**: open.
- [x] After a merge lands, Shown: no `space` verb (no count is ever asked, so `just_merged_bar` is `None`). — `update::tests::graph_mode::every_branch_asks_no_fold_count_at_rest_and_offers_no_space_after_a_merge`. See NOTES Q1.
- [x] Landing, worker. — `history::idle_tests::keep_tests::a_kept_commit_the_view_folds_away_is_answered_with_its_merge`; promotion: `history::reveal_tests::a_promoted_anchor_…` unchanged, `history::keep_tests::a_promotion_request_carries_no_keep`.
- [x] Landing, app. — `update::tests::graph_mode::{folded_to_every_branch_keeps_the_cursors_commit_and_is_not_an_operation, every_branch_to_folded_lands_a_folded_away_commit_on_its_merge, a_switch_inside_first_parent_stays_in_it_and_closes_every_opened_merge, the_same_mode_again_or_no_walk_sends_nothing}`; history level `history::keep_tests::a_view_change_that_folds_the_cursors_commit_away_lands_on_its_merge`.
- [x] Landing, `1`: keeps index (top, as `Follow` change sets), no `anchor_behind`. — `history::keep_tests::first_parent_hiding_the_cursors_commit_keeps_its_index`.
- [x] `space` close off the rows on hand: one find, lands on merge, closed, no reveal. — `update::tests::graph_mode::space_closing_a_merge_off_the_rows_on_hand_owes_no_reveal`; on hand: existing `update::tests::graph_toggles::space_inside_an_opened_merge_closes_it_and_lands_on_the_merge`.
- [x] Reveal unchanged. — `history::keep_tests::a_reveal_lands_on_its_target_not_on_the_kept_commit`; 06c reveal tests pass in the full run.
- [x] Re-ask, app. — `update::tests::graph_mode_reask::every_view_change_asks_the_folded_count_again` (seen red with the ask removed), `…::a_view_change_with_no_finished_search_asks_nothing`. `H` not in the sequence: H with no stash is no view change.
- [x] Re-ask, tree_answers: refs read carries the ask; none without a search. — `update::tests::graph_mode_reask::a_refs_read_view_change_asks_the_folded_count_again`. `head_known` (:57) with no search: not separately tested (code path: `ask_folded` returns early with no search).
- [x] Re-ask, worker under the new view (fold None → 0; opened merge → fewer). — `history::idle_tests::keep_tests::a_folded_ask_on_a_view_change_is_counted_under_the_new_view`; first-parent 0 is 06c's `hits_off_the_first_parent_line_are_not_counted_folded`. No-further rule: not re-tested here.
- [x] No config: every earlier test passes. — full workspace nextest run.
- [x] Captures `docs/shots/ease-pass/main-merged-shown-120.txt` (`it::graph_mode::every_branch_first_paint_…`), `config-merged-branches-problem-120.txt` (`event_loop::tests::theme_problems::a_graph_mode_it_does_not_take_…`), in the contact sheet after `main-opened-120.txt`. Look unjudged (rule 6), colour unverified (rule 5).
- [x] NOTES below.
- [x] Each tick names its test.

## NOTES (builder, 2026-10-18)
- Scout used: `.scratch/ease-pass/scouts/06h-graph-mode.md` (not re-scouted). No compaction.
- Step-1 grep `inclusion.folded\|folded: \[` over crates: empty.
- Shared landing fn: `history::scroll::apply::landed_at(&Window, Oid) -> Option<usize>`.
- `keep` is filled from `History.reanchor` in `ask()` **only when no promotion rides** (a promotion's anchor is the selection already); so the worker's debug assertion holds.
- `space` close's seek (`view_change::send`, was `keys.rs:453`): still needed when the merge row is off the rows on hand; with it on hand `keep` would land the same. Not removed.
- `tree_answers.rs` refs read sends a `RowRequest` (asserted); the ask rides on it.
- `apply_view_change`'s body moved to `update::keys::view_change::send` (keys.rs 487 → 474, row lowered); `switch_graph_mode` goes through it.
- Deviations: `[graph] merged_branches` held as `Option<RawValue>` (newtype over `toml::Value`, manual `Eq`) — `toml::Value` is not `Eq` and `Config` derives it. `History::started(MergedBranches)` callers (37) updated. `AppState.graph_mode` is `pub` (integration test sets it before the walk starts). `Carried` (event_loop/places) carries the config's mode to every place; a **switch made at runtime does not carry to the next place** (each place gets a fresh `AppState`) — gap for 06i/conductor.
- `switch_graph_mode` has `#[cfg_attr(not(test), expect(dead_code))]` until 06i calls it.
- Q1 (question, not defect): in Every branch the M18 `JustMerged` bar never shows at all (it needs a fold count, never asked) — so `u undo the merge · P push` is also missing after a merge in that mode. `finish.rs:208-224`.
- Captures re-shot: none needed — no existing capture test failed (full run). New: two above.
- Design-doc wording for the conductor: §2.2 :188-189, §2.12 :581, `01-view-mockups.md` :525 "`[keys]`/`[theme]`" → "`[keys]`, `[theme]` or `graph.merged_branches`"; §2.3 :283 add "unless the graph mode is Every branch"; PROMPT §1.3 same.
- Gap `ease/06c-suffix-stale` closed by `update::tests::graph_mode_reask::every_view_change_asks_the_folded_count_again` and `history::idle_tests::keep_tests::a_folded_ask_on_a_view_change_is_counted_under_the_new_view`.
- Flaky seen once in a full run, passed on rerun, untouched code: `it::m7_walk::fork_rows::a_stop_inside_a_fork_names_it_on_the_banner`.
- README: `[graph] merged_branches` line and the bad-value exception. Man page regenerated.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k (the landing adds one request field, one worker branch, one apply branch and four tests; the re-ask one call per site and three tests).
