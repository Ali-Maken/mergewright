# graph-mode-in-help — `GRAPH MODE` in the `?` list: `⏎` switches, the app keeps the pick in its config file

Status: partial — built and gated; two boxes open on spec rulings (the `✗` the bar does not draw; `graph` does not reach the group in 4 steps). See NOTES › Open.
Blocked by: 06h-graph-mode (`MergedBranches`, `as_config`, `History::merged_branches`, `switch_graph_mode`, `AppState.graph_mode`, the folded-count re-ask and the `keep` landing). Build after 06h merges.
Build with: Sonnet — a listing group, a keyless row kind drawn by kind, one column type widened, and a small file writer the loop runs as it runs `SaveLayout`; each pinned by tests, the writer's cases all observable on a temp dir. Scout (`.scratch/ease-pass/scouts/06i-graph-mode-in-help.md`) held Sonnet pending three rulings; they are made below.
Needs: from 06h `MergedBranches::{KEY, as_config}`, `History::merged_branches() -> MergedBranches`, `switch_graph_mode(&mut AppState, MergedBranches) -> Vec<Command>` (through `apply_view_change`: lands the cursor by `RowRequest.keep`, re-asks a live search's folded count — 06i adds no ask), `AppState.graph_mode`. Existing (scout): `update/palette/groups.rs:14-77` `groups()` (`HERE` :58-62, `everything()` app rows :100-136); `Entry { row, target, search: &'static [&'static str] }` (`palette/mod.rs` ~:290); `PaletteTarget` (`mod.rs:240-262`), `AppAction { Theme, Tour }` (:228-237); graft-ui `palette.rs` `RowKind` (:56-74), `PaletteRow` (:78-89, `config: Option<ActionId>` :86), row draw `palette/row.rs:60-103`; `config/load.rs` `Loaded.path` (:26, `None` with no file :159-160), `config::default_path()` (:177), `Session.config: Loaded` (`session.rs:27`); `Command` (`update/events/command.rs:18`), `AppEvent` (`update/events.rs:27`), loop-carried writes `SaveLayout` (:94) / `FirstLaunchDone` (:101), `jobs/runner/dispatch.rs:98` `not_a_job`; bar `Status` (`update/state.rs:1188`), `Status::done` (:1282).
Scout: done — `.scratch/ease-pass/scouts/06i-graph-mode-in-help.md`. Re-read it for line numbers; don't re-scout.

## Read first
- `docs/design-notes/ease-pass.md` :20 ("Graph mode in the `?` list").
- Mock: `prototypes/ease-mocks/mergewright-show-all-branches.html` script :76-107 (rows, group, words), :137 (`⏎` and the flash). Build to it.
- `06h-graph-mode.md` Decided ("Where the mode lives", "Cursor landing", "The switch", "Folded count re-asked") and Signatures.
- Scout file (above), whole (one page).
- `docs/design/02-keymap.md` §2.12 :576-592 (palette and help). `docs/design/01-view-mockups.md` §1.16 :505-532 (row form: key 6 cells · label · config id, id dropped below 100 columns).
- `README.md` :105-111 (config path, how it is read).
- `crates/graft/MAP.md`: `config`, `config::load`, `config::folder`, `update` (pure: no file), `update::palette`, `jobs`, `layout` (an existing loop-carried write, for form), `event_loop`. `crates/graft-ui/MAP.md` `palette`.

## Why
User, 2026-10-18 (field notes :116): "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?" — chose: pick the graph mode inside the app, in the `?` list; no new key; he never opens a file.

## Checked before this spec (rule: check the specific)
- **Keyless palette rows are not new**, but the precedent is `RowKind::App` (themes, tour, notes; `PaletteTarget::App`), not the commit-editor options — those carry `ActionId`s (`editor_tests.rs:147`; scout mismatch 6). **New here:** a row that is a setting's choice — `●`/`○` in the key column, `(on)` after the chosen one, a muted detail, the config key in the id column — and a group not read from the key table.
- **The app never writes its config today.** `crates/graft/Cargo.toml` :63-66: "Graft reads config and never writes it" (`toml` 1.1.6 built without `display`). The app's other writes (`SaveLayout`, `FirstLaunchDone`) go to its own state folder and are **carried by the loop, not jobs** (`dispatch.rs:98`). **The writer is new code; the way it runs is not.**
- **Typing in `?`:** 06r Q1 (open, owned by 14): the space key typed in `?` goes into the query. This journey types letters (`graph`), unaffected.

## Decided here
- **Where it lists.** `?` opened on the graph (`ContextId::Graph`): group `GRAPH MODE` pushed in `groups()` right after `HERE` (`groups.rs:58-62`; mock :78-81), before `EVERYWHERE`. `⌃P`/`:` with a query: the same rows in `everything()` beside the theme/tour app rows (:100-136), shown only when one matches. Not on the start screen (no graph). Hidden words through `Entry.search` (the slot exists; app rows pass `&[]` today): `graph mode view fold folded merges merged every branch all lanes` (mock :83) — `graph`, `fold`, `branch` all find it.
- **Target.** `PaletteTarget::App(AppAction::GraphMode(MergedBranches))` — beside `Theme`/`Tour`, the keyless precedent. Not an `ActionId`.
- **Config column type (scout mismatch 1).** `PaletteRow.config` widens from `Option<ActionId>` to `Option<ConfigId>`, `enum ConfigId { Action(ActionId), Key(&'static str) }` (graft-ui `palette`). Every existing producer becomes `ConfigId::Action(id)`; the drawer prints either as text, muted, dropped below 100 columns as today. **Rejected: a made-up `ActionId` `graph.merged_branches`** — it would enter the action table, `tests/keymap_ids.txt`, `[keys]` binding and `cli::config_keys`' pairing as something bindable, which it isn't. Rejected: no id on setting rows — the mock and §1.16 show it, and it tells him the file key.
- **Row** (mock :100, §1.16 form): `RowKind::Setting { on }`, `key: None`. The drawer draws the key column **by kind** for `Setting`: `●` accent+bold when `on`, `○` muted when not (scout mismatch 4: today any key is accent+bold, `row.rs:68-73`; other kinds unchanged). Title padded (`Folded merges`, `Every branch`), then `detail` muted (`merged branches fold into their merge row` / `every branch on its own lane, as before`), then `   (on)` muted after the chosen one; id column `graph.merged_branches`. Room order: id dropped first (below 100 columns, as every row's), then the detail cut from its end, then the title — `(on)` never cut (it is the state).
- **`⏎` / click** on the off row: palette closes → `switch_graph_mode` (06h) → `Command::KeepGraphMode(mode)`. On the on row: palette closes, nothing else (mock :137). Not recorded as a recently run command (ADR-0061 records actions; this is a setting). No journal entry.
- **Where he lands:** the graph, cursor on the same commit (06h; switching to folded with the cursor on a commit that folds away → on its merge, nothing opened), top line `every branch shown` / `K merged branches folded`. A live search stays live; its footer is re-counted under the new mode by 06h's re-ask. Bar, after the write: `✓ Graph: every branch · kept for next launch` / `✓ Graph: folded merges · kept for next launch` (mock :137), `Status::done`'s one-shot line. Write failed: `✗ Graph: every branch for this run · not kept: <reason>` — the switch stands, the file is as it was. The failure uses the existing one-shot failure variant (`Failed`, not `Sticky`: the app works on, only the keep failed). **Whether `Failed` draws `✗` is unconfirmed (scout) — a test pins it; if it doesn't, stop and report the line: no new `Status` variant without a spec change.**
- **Who writes (scout mismatch 3): the loop, as `SaveLayout`.** `KeepGraphMode` is listed in `dispatch.rs:98` `not_a_job`; the loop's command arm calls `config::write::keep_merged_branches` and feeds `AppEvent::GraphModeKept { mode, result }` back to `update` by the route `SaveLayout`'s arm uses (NOTES names it; if `SaveLayout` reports nothing back, the arm queues the event on the loop's own event channel — NOTES says which). A worker job was rejected: no precedent for app writes, and the write is one small file (milliseconds). `update` stays pure.
- **The path (scout mismatch 2).** Resolved in the loop arm, which holds `Session.config`: `Loaded.path` when set (the file actually read), else `config::default_path()` (`load.rs:177`, names `$XDG_CONFIG_HOME/mergewright/config.toml`, else `~/.config/mergewright/config.toml`, whether or not the file exists). Both absent (no home) → `KeepError::NoFolder`. The pty walk (no file) takes the `default_path()` branch.
- **The writer** (`config::write`, new):
  - Missing file → create it (and the folder) holding `[graph]` + the key. Read fresh at write time, not the startup copy: edits he made while running are kept.
  - Edits with **`toml_edit`** (new dependency, graft only; absent from `Cargo.lock` today; same toml-rs family as `toml` 1.x, which already pulls `toml_parser`/`toml_writer`/`toml_datetime`): every comment, blank line, key order and other table kept byte for byte; only the key's value changes, its trailing comment kept; absent key → added at the end of `[graph]`; absent `[graph]` → appended; a dotted `graph.merged_branches` or inline `graph = { … }` edited in place. A bad value (`"open"`) is replaced. Pick the `toml_edit` version sharing `toml_parser`/`toml_datetime` with `toml` 1.1.6; `cargo tree -d` showing a duplicate of either is a stop, reported.
  - **Refuses, file untouched:** a file that no longer parses (his edit mid-run) → `config.toml no longer reads (line L, column C) · left as it is`; `graph` not a table → `graph in config.toml isn't a table · left as it is`.
  - **Atomic:** temp file in the same folder as the real file (`tempfile::NamedTempFile::new_in`; `tempfile` moved from dev- to normal dependencies, already in the tree), written, `sync_all`, renamed over. **Symlink** (dotfiles): resolve to the target and replace the target; the link stays a link. Permissions of the old file kept. No temp file left after success or failure.
  - Two instances picking at once: last write wins over the whole file, each built from a fresh read. Accepted; no lock (a millisecond window).
  - `Cargo.toml` :63-66 comment rewritten (the app now writes one key).

## Journey (counted; acceptance asserts)
Switch to every branch: `?` · `graph` · `↓` · `⏎` = **4** (typed word = 1 step), lands on the graph, same commit, flash. Back: `?` · `graph` · `⏎` = 3 (Folded merges is the first row). Next launch opens in the kept mode: 0 steps. Before (06h as a file setting): leave the app, open the file, type the line, save, relaunch. Best tool: Fork, collapse/expand all from a menu (`docs/research/graph-readability.md` :31; its click count not recorded there). ux-principles: recognition over recall (the choice is listed, not remembered as a key), visibility of system state (dot + `(on)` + top line).

## Scope
In: graft `update::palette` (group, rows, target, `⏎`), `Command::KeepGraphMode`, `AppEvent::GraphModeKept` and its `update` arm (bar line), `dispatch.rs` `not_a_job`, the loop's command arm (path + write), `config::write` (new); graft-ui `palette` (`RowKind::Setting`, `PaletteRow.detail`, `ConfigId`, row drawing, `palette_row_at` on the new rows); `Cargo.toml` (graft), `tests/it/dependency_edges.rs` (:65 table); README :105-111 (the app writes this one key when picked in `?`).
Out: the modes themselves, the landing and the search re-ask (06h); other settings in the list; live reload; 06r Q1 (space typed in `?`); a new `Status` variant; design-doc wording — text in NOTES for the conductor: keymap §2.12 :580-581 add "`GRAPH MODE` after `HERE` on the graph: two keyless rows, `⏎` picks and keeps it"; `01-view-mockups.md` §1.16 :524-528 the group and the `●`/`○` row form; §1.16's drawing gains the group.

## Signatures
graft-ui `palette`:
```rust
pub enum RowKind {
    // … existing variants …
    /// One choice of a setting: `●` (on, accent) / `○` (muted) drawn in the key column by kind,
    /// `(on)` after the chosen one; `⏎` picks it. Its `PaletteRow.key` is `None`.
    Setting { on: bool },
}

/// What the id column shows: an action's config id, or a setting's config key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigId {
    Action(ActionId),
    Key(&'static str),
}

pub struct PaletteRow {
    // … existing fields; `config` retyped:
    pub config: Option<ConfigId>,
    /// Muted text after the label (setting rows); cut from its end before the label is.
    pub detail: Option<&'static str>,
}
```
graft `update::palette`:
```rust
/// One row of a setting group the `?` list offers.
pub(crate) struct Choice { pub value: MergedBranches, pub title: &'static str, pub detail: &'static str }
pub(crate) const GRAPH_MODE: [Choice; 2];                  // Folded merges, Every branch — in this order
pub(crate) const GRAPH_MODE_WORDS: &[&str];                // the hidden words, into `Entry.search`
// AppAction gains:
GraphMode(MergedBranches),
```
graft `config::write` (new module):
```rust
/// Sets `[graph] merged_branches` in the config file at `path`, every other byte kept.
/// Creates the file and folder if missing; refuses a file that no longer parses or whose `graph` isn't a table;
/// writes through a symlink to its target; atomic (temp in the same folder, then rename); keeps permissions.
pub fn keep_merged_branches(path: &Path, mode: MergedBranches) -> Result<(), KeepError>;

#[derive(Debug)]
pub enum KeepError {
    Unparsable { line: usize, column: usize },
    NotATable,
    /// No config folder could be named (no `Loaded.path`, `default_path()` gave none).
    NoFolder,
    Io(std::io::Error),
}
impl std::fmt::Display for KeepError; // the reason words in Decided; Io: `can't write <path>: <error>`
```
graft `update/events/command.rs`: `Command::KeepGraphMode(MergedBranches)` (loop-carried, in `not_a_job`). `update/events.rs`: `AppEvent::GraphModeKept { mode: MergedBranches, result: Result<(), String> }` (the `KeepError`'s words). No graft-core type gains a config type. Every exhaustive match the new command/event reaches (e.g. `event_loop/label.rs`, as 06c found for `FoldedHits`) gains its arm; the compiler confirms.

## Touches
Each checked, and how, in the report's **Touches:** line:
- Every `PaletteRow { config: … }` producer and every `.config` reader (grep `config: Some(` / `.config` in graft and graft-ui palette code): each producer → `ConfigId::Action`; each reader handles `Key` (draw) or matches `Action` only where it needs the action — list each.
- Every `PaletteRow { .. }` literal (new `detail` field); `muted_prefix` (`row.rs:93-119`) unchanged for File/Commit.
- Row draw `row.rs:60-103`: other kinds' key column still accent+bold (existing row-draw tests pass unchanged).
- Captures and snapshots of `?` on the graph (scout list): `m8-help-graph-120x36.txt`, `m8-help-80x24.txt`, `m8-look-help-graph-187x50.txt`, `m8-keymap-config-problem-120x36.txt`, 06h's `config-merged-branches-problem-120.txt` — each re-shot with its D3 class; `m8-contact-sheet-{1,2,3}.txt` reassembled from them; `m8/help-at-map-120x36.txt` (map: no group) — unchanged, shown by a diff; prototype `m8-proto-2*` — historical, not re-shot (NOTES says so). No insta `.snap` contains `EVERYWHERE` (scout grep) — re-run the grep.
- `palette_row_at` (group header → `None`, rows hit), `row_count`, scrolling in a short box, `nothing matches`.
- `⌃P` typed ranking and the recently-run list (not recorded — test); theme/tour app rows in `everything()` unchanged.
- Start-screen palette (`empty_state::palette`): no group (test).
- `first_launch` (opening the palette still counts as today).
- Tour steps that open `?` on the graph (grep).
- A live search across the switch: re-counted by 06h; 06i sends no ask of its own (grep `ask_folded` in the files it edits → none).
- The loop's command arms and `dispatch.rs:98` (`not_a_job` list); `SaveLayout`'s route for its result (named in NOTES).
- `cli::config_keys` / man page: the key is unchanged; the man page notes the app may write it.
- Dependencies: `toml_edit` added, `tempfile` promoted; `tests/it/dependency_edges.rs` :65 table; no `deny.toml` (scout).
- Keymap: no key added; `tests/keymap_ids.txt` unchanged (rows aren't actions — test/diff). Journal/undo: none.

## Acceptance
- [x] `?` on the graph, empty query: `HERE`, then `GRAPH MODE` with `● Folded merges … (on)` and `○ Every branch`, then `EVERYWHERE`; id column at 120, dropped at 90; `(on)` whole. — `update::tests::graph_mode_help::help_on_the_graph_lists_graph_mode_after_here_the_on_row_marked` (order, kinds, targets); `mergewright-ui palette::tests::setting_tests::the_graph_mode_group_sits_after_here_with_its_key_at_120`, `…::narrow_the_id_drops_then_the_detail_is_cut_and_on_stays_whole` (90: no id, `(on)` whole; 50: detail cut, `(on)` whole); pty `graph_mode_keep::typed_graph_lists_the_group_after_here_at_120_and_90`. **Deviation:** at 120 the detail is cut (`…merge …   (on)  graph.merged_branches`) while the id stays: the spec's "id dropped first" and "id column at 120" can't both hold in a 96-cell box (title 16 + detail 41 + `(on)` 7 + id 23 > 85). Kept the id, cut the detail (NOTES › Open 3).
- [x] Setting row draw: `●` accent+bold, `○` muted (not bold), an action row's key accent+bold; detail and `(on)` muted — buffer cells. — `palette::tests::setting_tests::setting_rows_draw_their_dot_by_kind_and_action_keys_stay_accent_bold`.
- [x] `ConfigId`: action rows draw as before; a `Key` row draws the key. — every pre-existing `mergewright-ui palette::tests::*` passes unchanged (23/23 in the run); `setting_tests::the_graph_mode_group_sits_after_here_with_its_key_at_120` (both `graph.merge` and `graph.merged_branches` drawn).
- [x] Typed `graph`, `fold`, `branch` keep both rows; `⌃P` typed `graph` lists them; start screen never. — `graph_mode_help::typed_graph_fold_and_branch_each_keep_both_rows_and_ctrl_p_finds_them`; `empty_state::palette_tests::the_start_screen_never_lists_graph_mode`.
- [x] `⏎` on Every branch: closed, `merged_branches() == Shown`, one `KeepGraphMode(Shown)`, row request `keep` = the cursor commit, not recent, only `ShowRows` + `KeepGraphMode` emitted (no journal command). On row: closed, no command. Double-click = `⏎` (a single click only moves the cursor, §2.12 decision 6). — `graph_mode_help::enter_on_every_branch_switches_keeps_and_records_nothing`, `…::enter_on_the_on_row_closes_and_does_nothing`, `…::a_double_click_on_a_mode_row_is_enter`.
- [x] Finished search, 3 folded: `⏎` on Every branch → the request carries a `FoldedAsk`; fed `folded: 0`, no suffix, search live. — `graph_mode_help::a_live_search_is_recounted_by_the_switch_and_stays_live`.
- [ ] Event ok → `✓ Graph: every branch · kept for next launch` (**holds**); event err → drawn line starts `✗ Graph: …` — **does not hold: `Status::Failed` draws no `✗`** (`crates/mergewright/src/view/status.rs:108`, the `ProblemLine` built from the bare message). Drawn: `Graph: every branch for this run · not kept: <reason> · esc dismiss`. Stopped as the spec says; no new variant. Mode stays `Shown` (holds). — `graph_mode_help::the_bar_says_kept_or_why_not_with_its_mark_drawn` (pins the words as drawn today).
- [x] Loop arm: not a job; `Loaded.path = Some(p)` writes `p`; `None` writes `default_path()`; neither → `NoFolder`'s words. — `jobs/runner/dispatch.rs` `not_a_job` arm (compiler-exhaustive); `config::write::write_tests::the_loop_writes_the_file_read_else_the_default_else_no_folder` (`keep_in`, the function the arm calls with `outside.config_path` and `config::default_path()`); pty walk (`Loaded.path` `None`).
- [x] Writer cases on temp dirs, each `load_from`s to the mode with no warning, no temp left. — `write_tests::a_missing_folder_and_file_are_created_holding_only_the_key`, `…::no_graph_table_appends_one_after_every_original_byte`, `…::a_graph_table_with_other_keys_gains_only_the_line`, `…::an_existing_value_changes_alone_its_comment_kept_and_a_bad_one_replaced`, `…::a_dotted_key_is_edited_in_place`.
- [x] Refusals: unparsable and `graph = 3` → `Err`, bytes unchanged; read-only folder → `Io`, unchanged; no temp left. — `write_tests::an_unparsable_file_and_a_graph_that_isnt_a_table_are_refused_untouched`, `…::a_read_only_folder_is_an_io_error_and_the_file_is_unchanged`.
- [x] Symlink stays a link, target updated; 0600 stays 0600. — `write_tests::a_symlink_stays_a_link_its_target_updated_and_0600_stays_0600`.
- [ ] Pty walk ≤ 4 steps — **holds only with `every`, not `graph`**: `? · every · ↓ · ⏎` = 4 → `every branch shown`, flash, file `"shown"`; relaunch opens shown, no key; `? · every · ⏎` → folded, file `"folded"`. With `graph` the list keeps three HERE rows first (renderer, commit-graph offer's two), so `? · graph · ↓×4 · ⏎` = 7, and `? · graph · ⏎` runs `F3` (Cycle the graph renderer), not Folded merges. `graph mode` matches nothing. — `graph_mode_keep::picking_a_graph_mode_in_help_switches_keeps_and_reopens_in_it` (asserts `steps.taken <= 4` with `every`); the `graph` list asserted and captured in `graph_mode_keep::typed_graph_lists_the_group_after_here_at_120_and_90`.
- [x] `cargo tree -d -e normal`: no duplicate `toml_parser`/`toml_datetime`/`toml_writer`; `toml_edit 0.25.15+spec-1.1.0` shares them with `toml 1.1.6`. — `docs/perf/06i-cargo-tree-d.txt`; `dependency_edges` passes (it checks internal crate edges only; no row to add — NOTES).
- [x] Captures `help-graph-mode-120.txt`, `help-graph-mode-90.txt`, `graph-mode-switched-120.txt` under `docs/shots/ease-pass/`, captioned after 06h's; sheet regenerated (`scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass`, now 2 parts). Look unjudged (rule 6), colour unverified (rule 5). — written by `graph_mode_keep::*`.
- [x] NOTES below. — document box.
- [x] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k (the column retype touches every palette producer; the rest as before).

## NOTES (builder, 2026-10-18)

Scout: used `.scratch/ease-pass/scouts/06i-graph-mode-in-help.md` (no re-scout). Not compacted.

### Open (conductor / user rulings)
1. **`✗` not drawn.** `Status::Failed` → `view/status.rs:108` builds the problem line from the message alone. The failure bar reads `Graph: every branch for this run · not kept: <reason> · esc dismiss`. Options: prefix `✗ ` in the message (words only, no variant), or draw `✗` for every `Failed` (touches every failure capture). Spec said stop; stopped.
2. **Journey count.** `graph` also fuzzy-matches `Cycle the graph renderer`, `Write the commit-graph file`, `Decline the commit-graph offer` (HERE, listed before GRAPH MODE by fixed group order, decision 4): `? graph ↓ ⏎` lands on the commit-graph offer row. Real counts with `graph`: switch `? graph ↓×4 ⏎` = 7, back `? graph ↓×3 ⏎` = 6; with `every`: 4 and 3, as the spec counts. `fold` keeps one HERE row first (`Expand / collapse folded lanes`); `graph mode` matches nothing (space in the query). Options: GRAPH MODE before HERE when the query matches its words; rank a group whose hidden word matches exactly first; or teach `every` as the word.
3. **Room order at 120.** The 96-cell box can't hold title + detail + `(on)` + id; built: id kept at ≥100 columns, detail cut. Spec's "id dropped first" would drop the id at 120 too, contradicting the 120 box.

### Decisions / deviations
- `KeepError::Io { path, error }` instead of `Io(io::Error)`: `Display` must say `can't write <path>`.
- `NoFolder` words (spec gave none): `no config folder could be found`.
- `Outside` gains `config_path: Option<&Path>` (`event_loop/outside.rs`), set from `Loaded.path` in `startup::run_repo`; `listen::outside` takes it. `Session` has no `config` field (scout's `session.rs:27` is `Startup.config`).
- Click: one click moves the cursor (decision 6); a second click on the row runs it, as `⏎`.
- `setting.rs` (new, `update/palette/`) holds `AppAction` (moved out of `mod.rs`), `GRAPH_MODE`, `GRAPH_MODE_WORDS`, `graph_mode_rows`, `graph_mode_kept`.

### `.config` readers / producers
- Producers: `listing.rs` action rows → `ConfigId::Action(binding.action)`; every other producer (`groups.rs` notes/themes/tour, `find.rs`, `listing.rs` find placeholders) `config: None`, `detail: None`; setting rows `ConfigId::Key("graph.merged_branches")`.
- Readers: `mergewright-ui palette/row.rs` draw (`ConfigId::as_str`, either variant, dropped below `WIDE_FROM`); `update/resolver/tests/help_words.rs` (matches `ConfigId::Action`). No other reader (compiler-checked: the retype broke only these).

### Result route
`SaveLayout` reports nothing back (`event_loop/running/carry.rs`, `drop(save_split)`). `KeepGraphMode`'s arm in the same `carry_out` calls `config::keep_in(outside.config_path, config::default_path(), mode)` and feeds `AppEvent::GraphModeKept` straight to `update`, queuing what it answers — the route `handed_over` events take. No event channel.

### Captures (D3 class)
- New: `ease-pass/help-graph-mode-120.txt`, `help-graph-mode-90.txt`, `graph-mode-switched-120.txt`.
- Re-shot: `m8-look-help-graph-187x50.txt` — list content (GRAPH MODE added; `u`/`U`/`⌃R` scrolled below the box).
- Unchanged (their check passes): `m8-help-graph-120x36.txt`, `m8-help-80x24.txt`, `m8-keymap-config-problem-120x36.txt`, `ease-pass/config-merged-branches-problem-120.txt` (the group falls below their box's fold), `m8/help-at-map-120x36.txt` (map: no group). Proto `m8-proto-2*` historical, not re-shot.
- `m8-contact-sheet-{1,2,3}`: **not** reassembled — `scripts/contact-sheet.sh m8` rewrites 3304 lines of them (stale from earlier tickets' re-shots), out of scope; conductor's call.
- No insta `.snap` holds `EVERYWHERE` (grep).

### Dependencies
`toml_edit = 0.25.3` (locked 0.25.15+spec-1.1.0, features `parse`, `display`), `tempfile` dev → normal. `cargo tree -d -e normal` (`docs/perf/06i-cargo-tree-d.txt`): no toml duplicate. `dependency_edges.rs` checks only internal edges against ARCHITECTURE.md; nothing to add.

### Size ratchet
Over-400 files kept from growing by splits: `update/events/progress.rs` (Progress out of `events.rs`, 584→551), `event_loop/outside/stop.rs` (`stop_this_process`, 625→608), `update/palette/setting.rs` (`mod.rs` 408→402); `command.rs` held at 459 by tightening two doc comments. `over_400.txt` rows lowered. `startup::run_repo` held under 60 lines by `graphics_for`.

### Design-doc wording (for the conductor)
- keymap §2.12 :580-581: add "`GRAPH MODE` after `HERE` on the graph: two keyless rows, `⏎` picks and keeps it in config".
- `01-view-mockups.md` §1.16 :524-528: the group and the `●`/`○` row form (`● title(16) detail (on)  graph.merged_branches`); the drawing gains the group.

### Pre-existing failure
`published_repo::nothing_a_commit_would_carry_names_this_machine` fails on `docs/release/export-inventory.md:114,121` (home directory), a file this ticket does not touch.
