# merged-branches-setting — `[graph] merged_branches = "shown"` draws every merged branch on its own lane; folded stays the default

Status: draft
Blocked by: 06b-lanes-on-screen-and-header (its `header_tail` / `Filters.folded_in_view` are asserted here), 06c-fold-seek-reveal (shares `history::state`; its search suffix and `Whereabouts` are asserted here)
Build with: Sonnet — config plumbing and the walk's starting view; no engine change (the unfolded inclusion already exists, `fold: None`); every behaviour below is a state test. Brownfield: scout first (ADR-0035); conductor confirms the model after it.
Needs: from 06b `header_tail(&Filters) -> String`, `Filters.folded_in_view`; from 06c `Included::whereabouts -> Whereabouts`, the search hit summary's `folded: usize`. Existing: `View::folded` (`crates/graft/MAP.md` `history`), `Inclusion { fold: Option<Folding> }` and `opened`/`keep_mut` returning `None` when not folding (`crates/graft-graph/MAP.md` `inclusion`), `Config` `[graph]` (`config::types`), `keymap_from` (every config problem as words), `history::fold::lay_out`, `update::fold_count`.
Scout: needed — one page: where `Config`'s `[graph]` is typed and how a bad value stops today; who builds the problem list the status row and `YOUR CONFIG` read; every production caller of `View::folded()` (walk start, re-walk after an operation, `1`/`H` return); where `History` is built from config.

## Read first
- `docs/design-notes/ease-pass.md`: Hard-to-reverse choice 2 (the "Settled 2026-10-18" paragraph); Mock behaviours G4, G10, M18.
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Scope bullets 1 and 4 (`space`); "Decisions for continuation 2" D2 (HEAD keep). `06b` Decided (K). `06c` Decided (search count and order).
- `docs/design/02-keymap.md` §2.2 :179-195 (config problems: warned, never fatal; status row `config: N problems · ? shows them`), §2.3 :276 and :283 (`Space`, fold default), §2.12 :580-581.
- `README.md` :105-146 (Configuration; the `[graph]` block).
- `crates/graft/MAP.md`: `config`, `config::types`, `config::keys`, `cli::config_keys`, `cli::man`, `history` (`View`, `RowFold`), `history::fold`, `update::fold_count`. `crates/graft-graph/MAP.md` `inclusion`.

## Why
User, 2026-10-18: "no, not as easy as a key, but a setting like a mode... extra keys would be bad for not so important things". He wants the whole graph unfolded — every merged branch on its own lane, the look before the ease pass — as a setting he picks once, not a key.

## Decided here
- **Name and place:** `[graph] merged_branches = "folded" | "shown"`, default `"folded"`. His words: the fold's tail already says "commits from <branch>"; `shown` is what he sees. An enum, not a bool, so a bad value can say both choices, and so a later mode needs no rename. Read once at startup, like every other key; changing it needs a relaunch.
- **Backing:** the unfolded inclusion that already exists — `Inclusion` with `fold: None` (what `View::default()` lays out). **The legacy opt-in `Inclusion.folded` does not come back.** The readers the dispatch cites (`view_change.rs:32,36`, `keys.rs:449`, `entries.rs:375`, `rows.rs:141`) are from 06's first NOTES (:113); continuation 1 retired the field and `Filters.folded` (06 NOTES :138, Touches :168 "grep `inclusion.folded|folded: \[` is empty"), and graft-graph's MAP lists no such field (`inclusion`: "hidden stashes, stash entries and `fold: Option<Folding>`"). Step 1 re-runs that grep; a hit is a stop, not a revival.
- **Shown mode, what follows from `fold: None`** (no new rule, each pinned by a test):
  - No row folds: every `RowFold.folds` is `None`, no `▸` tail, no fold count asked at rest, no `FoldWalk` memory.
  - **`space` on the graph does nothing** on any row, merge rows included, and at the edit stop (§2.6a). It is 06's rule unchanged: `space` acts only on a merge with `folds == Some(true)` or on a row inside an opened merge; in shown mode there are none. No per-merge fold-on-demand (that was the old opt-in; he asked for a mode, not more keys). `space`'s offer in the bar and in `?` follows whatever a plain row in folded mode does today — no special case.
  - **Header (06b):** K counts merge rows with a closed fold mark → 0 → `merged branches folded` never shows. `lanes drawn: N of M` still shows while N < M (more lanes now; the cap and `+` work as today). The header never claims a fold that isn't there.
  - **Search (06c):** `whereabouts` never answers `Folded`, so the count is every hit and the ` · N in folded merges` suffix never shows; `n`/`N` land without opening anything. A hit off the line in `1` keeps `(not in this view)`.
  - **After a merge lands (M18):** the bar drops `space show the N commits inside` — that verb is offered only on a row that folds.
  - **HEAD keep (06 D2):** no fold, no keep: a refs read never changes the view, so launch never re-lays out.
  - **`1` / `H` return** to the setting's view, not to `View::folded()`.
- **Bad value → a config problem, never fatal.** `merged_branches = "open"`, `= true`, `= 3`: one problem, `graph.merged_branches = "open" — takes "folded" or "shown"; using folded`, counted in the status row (`config: N problems · ? shows them`) and listed first in `?` under `YOUR CONFIG`, as `[keys]`/`[theme]` problems are (§2.2). The graph opens folded. **Departs from README :110-111** ("a value a key does not take stops Mergewright with the line and column"), which binds the typed `[ui]`/`[graph]` keys today: a display preference must not keep him out of his repo, and the dispatch asked for `YOUR CONFIG`. So this key is read off the raw table (as `config::keys` reads `[keys]`), not by serde's typed enum; it is still on the loader's known-key list (no "unknown key" warning). README's sentence gains the exception (builder edits README).
- **No new key, no palette action** (his words). Not an operation: no journal entry, nothing to undo.

## Scope
In (graft `config::types`, `config::load` (known-key list), the problem list `keymap_from` returns, `cli::config_keys`/`cli::man` tables, `history::state` (`View::starting`), every production `View::folded()` caller, `update::keys::view_change` (`1`/`H` base view, `with_head_kept`), the M18 bar verb if it is offered unconditionally today; `README.md` config block and :110-111):
- Parse, default, problem words as Decided.
- Walk start (launch, every re-walk) uses `View::starting(mode)`.
- Tests and captures below.
Out: any graft-graph change; any new `space` behaviour; a live reload; a palette toggle; design-doc wording (text in NOTES for the conductor: §2.2 :188-189 and §2.12 :581 and `01-view-mockups.md` :525 "`[keys]`/`[theme]`" → "`[keys]`, `[theme]` or `graph.merged_branches`"; §2.3 :283 add "unless `graph.merged_branches = "shown"`"; PROMPT §1.3 fold default, same clause).

## Signatures
graft `config::types`:
```rust
/// `[graph] merged_branches`: does a merged branch fold into its merge or keep its own lane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MergedBranches {
    /// Behind their merge, `▸ N commits from …`; `space` opens one. The default.
    #[default]
    Folded,
    /// Every merged branch on its own lane; nothing folds.
    Shown,
}
impl MergedBranches {
    pub const KEY: &'static str = "graph.merged_branches";
    /// Unset → `Folded`. A value it doesn't take → `Err(problem words)`; the caller uses `Folded`.
    pub fn from_config(raw: Option<&toml::Value>) -> Result<Self, String>;
}
```
`Config`'s `[graph]` gains `merged_branches: MergedBranches`; the `Err` words join the `Vec<String>` `keymap_from` returns (the list the status row and `YOUR CONFIG` read — if the scout finds another, NOTES names the line).
graft `history::state`:
```rust
impl View {
    /// The view a walk starts in: `Folded` → `View::folded()`; `Shown` → no fold (`inclusion.fold == None`).
    pub fn starting(merged: MergedBranches) -> Self;
}
```
`History` holds the mode, set where it is built from `Config` (NOTES names the line). After this ticket, `grep -n 'View::folded()' crates/graft/src` outside tests shows only `View::starting`'s body.

## Touches
Each checked, and how, in the report's **Touches:** line:
- config: `config::types` `[graph]`, `config::load` known-key list (no unknown-key warning — test), `cli::config_keys` pairing test, `cli::man` CONFIGURATION, `README.md` :110-111 and :125-129 (the documented key list lives there and in the man page, not in a design doc).
- Config problems: status row count, `?` `YOUR CONFIG` group (`m8-keymap-config-problem-120x36` capture — re-shot only if its fixture's problem count changes; D3 rule in 06).
- Every production `View::folded()` caller; `1` / `H` base view; promotion (`anchor_behind` never set when nothing hides — test via the shown walk).
- `with_head_kept` (06 D2) and `m7_walk::pull::pull_ff_over_edits` (phase stays `Complete`).
- `space` handler and the edit stop's `space` (§2.6a, `edit_stop_tests`).
- `update::fold_count::at_rest` and `update::rest_owed` (no count owed in shown mode).
- `history::fold::lay_out` on a non-folding view (equals plain `advance`; existing tests cover `View::default()`).
- 06b header tail; 06c search footer and seek routes; hint bar `RowBar::{FoldedMerge, Opened, JustMerged}` (`?`/bar offers of `space`).
- Pixel strips (`view::strips`): unfolded = the pre-ease-pass picture; no strip-key change expected — say if one appears.
- Lane cap, rail and `+` (more live lanes in shown mode).
- Tour: honours his config; grep its steps for a folded-row or `space` dependency, fix the step if one exists (06 D3 "the tour is product").
- AppState size cliff (06 NOTES :116) only if a field is added there; the mode lives in `History`.
- Keymap: none — `docs/design/02-keymap.md` keys unchanged, `no_literal_keys` unchanged. Journal/undo: none.
- `docs/capabilities/m1-the-graph.md` M1-09 (fold): add the setting's test citation if the checker requires it.

## Acceptance
- [ ] Unset, `"folded"`, `"shown"` read as `Folded`, `Folded`, `Shown`. — names its test.
- [ ] `"open"`, `true`, `3` each → `Err` with the exact words in Decided; the app starts folded. — names its test.
- [ ] A config with `merged_branches = "open"`: status row `config: 1 problem · ? shows them`; `?` opens with `YOUR CONFIG` first, its row reading the problem words; the graph folds (a `▸` tail on the fixture). — names its test.
- [ ] `merged_branches = "shown"` raises no unknown-key warning; `cli::config_keys` pairing passes with the key in README and the man page. — names its test.
- [ ] Shown, 06e's `wallet_release` fixture through the worker: first window holds every merged-branch commit; row ids equal a `View::default()` layout of the same repo; every `RowFold.folds == None`; no row has a tail. — names its test.
- [ ] Shown: header tail has no `merged branch` phrase at any width; `lanes drawn: N of M` present iff N < M (06b's table extended with K = 0). — names its test.
- [ ] Shown: `space` on a merge row, on a plain row, and at an edit stop → view unchanged, no reinclude, no job. — names its test.
- [ ] Shown: rests after scrolling ask no `CountFolded`, and no rest wake is owed for counts. — names its test.
- [ ] Shown: a refs read leaves the view unchanged and the phase `Complete`. — names its test.
- [ ] Shown: `1` then `1` (and `H`) returns to the unfolded view. — names its test.
- [ ] Shown: search with 4 hits on merged-branch commits → footer `4 matches`, no suffix; `n` visits all 4, `OpenedMerges` empty throughout. Detail pane parent 2 on a merge → lands, view unchanged. — names its tests.
- [ ] Shown: after a merge lands, the bar has no `space` verb; folded mode keeps `space show the N commits inside` (both asserted). — names its test.
- [ ] No config: every 06/06b/06c/06e/06g test passes unchanged. — names the run.
- [ ] Captures `docs/shots/ease-pass/main-merged-shown-120.txt` (`wallet_release`, shown, 120×36, cursor on the first merge) and `docs/shots/ease-pass/config-merged-branches-problem-120.txt` (`"open"`: status row, then `?` open), each in the contact sheet beside `main-folded-120.txt`. Look unjudged (rule 6), colour unverified (rule 5).
- [ ] NOTES: the step-1 grep for `Inclusion.folded` (expected empty), the design-doc wording for the conductor, each re-shot capture with its D3 classification.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~35k.
