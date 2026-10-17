# graph-mode-in-help — `GRAPH MODE` in the `?` list: `⏎` switches, the app keeps the pick in its config file

Status: draft
Blocked by: 06h-graph-mode (`MergedBranches`, `as_config`, `History::merged_branches`, `switch_graph_mode`).
Build with: Sonnet — a listing group, a keyless row kind and a small file writer, each pinned by tests; the writer's cases are all observable on a temp dir. Brownfield: scout first (ADR-0035); conductor confirms the model after it.
Needs: from 06h `MergedBranches::{KEY, as_config}`, `History::merged_branches() -> MergedBranches`, `switch_graph_mode(&mut AppState, MergedBranches) -> Vec<Command>`. Existing: `update::palette` (listing, filter, `⏎`), graft-ui `palette` (`RowKind`, `PaletteRow`, `PaletteView`, `palette_row_at`), `config::Loaded` (its path), the job runner (`update` → `Command` → worker → event), the bar's one-shot result line.
Scout: needed — one page: how `update::palette` builds `?`'s groups (`IN PROGRESS`/`HERE`/`EVERYWHERE`/`YOUR CONFIG`) and `⌃P`'s typed groups; how a keyless row is drawn today (commit options over the editor, MAP `update::palette`); every `PaletteRow` field (is there one for muted text after the label?); where `Loaded.path` is held after startup and whether it is set when no file exists; the command/job-event enums a write would use; the one-shot bar line an operation's `✓` uses; every capture and insta snapshot of `?` opened on the graph.

## Read first
- `docs/design-notes/ease-pass.md` :20 ("Graph mode in the `?` list").
- Mock: `prototypes/ease-mocks/mergewright-show-all-branches.html` script :76-107 (rows, group, words), :137 (`⏎` and the flash). Build to it.
- `06h-graph-mode.md` Decided and Signatures.
- `docs/design/02-keymap.md` §2.12 :576-592 (palette and help). `docs/design/01-view-mockups.md` §1.16 :505-532 (row form: key 6 cells · label · config id, id dropped below 100 columns).
- `README.md` :105-111 (config path, how it is read).
- `crates/graft/MAP.md`: `config`, `config::load`, `config::folder`, `update` (pure: no file), `update::palette`, `jobs`, `layout` (an existing write to the app's own folder, for form). `crates/graft-ui/MAP.md` `palette`.

## Why
User, 2026-10-18: "introducing [a config file] for just one feature doesn't feel right, how about a mode selector?" — chose: pick the graph mode inside the app, in the `?` list; no new key; he never opens a file.

## Checked before this spec (rule: check the specific)
- **Keyless palette rows are not new.** Over the commit editor the palette lists keyless options (`commit.toggle_amend` …; MAP `update::palette` :277; keymap §2.12 / §2.10 :538-540). **New here:** a row that is a setting's choice, not an action — no `ActionId`, a `●`/`○` in the key column, `(on)` on the chosen one — and a group not read from the key table. MAP lists no such `RowKind`; scout confirms.
- **The app never writes its config today.** `crates/graft/Cargo.toml` :63-66: "Graft reads config and never writes it" (`toml` built without `display`). Under `crates/graft/src/config/` only `folder.rs` touches the disk to write, and that moves the folder (grep `fs::write|File::create|rename(`, paths only). Other writes (`layout::save_split`, `first_launch`, `recent`) go to the app's own state folder, not `config.toml`. **The writer is new code.**

## Decided here
- **Where it lists.** `?` opened on the graph (`ContextId::Graph`): group `GRAPH MODE` right after `HERE` (mock :78-81), before `EVERYWHERE`. `⌃P`/`:` with a query: the same group after actions when a row matches. Not on the start screen (no graph). Typing filters on the title plus hidden words `graph mode view fold folded merges merged every branch all lanes` (mock :83) — `graph`, `fold`, `branch` all find it.
- **Row** (mock :100, §1.16 form): key column `●` on / `○` off (accent on the `●`), title padded (`Folded merges`, `Every branch`), muted detail (`merged branches fold into their merge row` / `every branch on its own lane, as before`), `   (on)` after the chosen one; config id column `graph.merged_branches` (muted, dropped below 100 columns as every row's). Below 100 columns the detail is cut before the title is.
- **`⏎` / click** on the off row: palette closes → `switch_graph_mode` (06h) → `Command::KeepGraphMode`. On the on row: palette closes, nothing else (mock :137). Not recorded as a recently run command (ADR-0061 records actions; this is a setting). No journal entry.
- **Where he lands:** the graph, cursor on the same commit (06h), top line `every branch shown` / `K merged branches folded`. Bar, after the write: `✓ Graph: every branch · kept for next launch` / `✓ Graph: folded merges · kept for next launch` (mock :137), the bar's existing one-shot line. Write failed: `✗ Graph: every branch for this run · not kept: <reason>` — the switch stands, the file is as it was.
- **The writer** (`config::write`, new; runs on a worker, `update` stays pure):
  - Path: the one `config::load` reads (`$XDG_CONFIG_HOME/mergewright/config.toml`, else `~/.config/mergewright/config.toml`). Missing file → create it (and the folder) holding `[graph]` + the key. Read fresh at write time, not the startup copy: edits he made while running are kept.
  - Edits with **`toml_edit`** (new dependency, graft only; same toml-rs family as `toml` 1.x; `cargo tree -d` shows no duplicate of `toml_parser`/`toml_datetime`): every comment, blank line, key order and other table kept byte for byte; only the key's value changes, its trailing comment kept; absent key → added at the end of `[graph]`; absent `[graph]` → appended; a dotted `graph.merged_branches` or inline `graph = { … }` edited in place. A bad value (`"open"`) is replaced.
  - **Refuses, file untouched:** a file that no longer parses (his edit mid-run) → `config.toml no longer reads (line L, column C) · left as it is`; `graph` not a table → `graph in config.toml isn't a table · left as it is`.
  - **Atomic:** temp file in the same folder as the real file (`tempfile::NamedTempFile::new_in`, `tempfile` moved from dev- to normal dependencies, already in the tree), written, `sync_all`, renamed over. **Symlink** (dotfiles): resolve to the target and replace the target; the link stays a link. Permissions of the old file kept. No temp file left after success or failure.
  - Two instances picking at once: last write wins over the whole file, each built from a fresh read. Accepted; no lock (a millisecond window).
  - `Cargo.toml` :63-66 comment rewritten (the app now writes one key).

## Journey (counted; acceptance asserts)
Switch to every branch: `?` · `graph` · `↓` · `⏎` = **4** (typed word = 1 step), lands on the graph, same commit, flash. Back: `?` · `graph` · `⏎` = 3 (Folded merges is the first row). Next launch opens in the kept mode: 0 steps. Before (06h as a file setting): leave the app, open the file, type the line, save, relaunch. Best tool: Fork, collapse/expand all from a menu (`docs/research/graph-readability.md` :31; its click count not recorded there). ux-principles: recognition over recall (the choice is listed, not remembered as a key), visibility of system state (dot + `(on)` + top line).

## Scope
In: graft `update::palette` (group, rows, `⏎`), the command and job event, `jobs` (the write job), `config::write` (new), the bar line; graft-ui `palette` (`RowKind::Setting`, row drawing, `palette_row_at` on the new rows); `Cargo.toml` (graft); README :105-111 (the app writes this one key when picked in `?`).
Out: the modes themselves (06h); other settings in the list; live reload; design-doc wording — text in NOTES for the conductor: keymap §2.12 :580-581 add "`GRAPH MODE` after `HERE` on the graph: two keyless rows, `⏎` picks and keeps it"; `01-view-mockups.md` §1.16 :524-528 the group and the `●`/`○` row form; §1.16's drawing gains the group.

## Signatures
graft-ui `palette`:
```rust
pub enum RowKind {
    // … existing variants …
    /// One choice of a setting: `●` (on) / `○` in the key column, `(on)` after the chosen one; `⏎` picks it.
    Setting { on: bool },
}
```
`PaletteRow` gains `detail: Option<String>` (muted, after the title) unless the scout finds a field that already draws that; NOTES says which.
graft `update::palette`:
```rust
/// One row of a setting group the `?` list offers.
pub(crate) struct Choice { pub value: MergedBranches, pub title: &'static str, pub detail: &'static str }
pub(crate) const GRAPH_MODE: [Choice; 2];          // Folded merges, Every branch — in this order
pub(crate) const GRAPH_MODE_WORDS: &str;           // the hidden search words above
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
    /// No config folder could be named (no `$XDG_CONFIG_HOME`, no `$HOME`).
    NoFolder,
    Io(std::io::Error),
}
impl std::fmt::Display for KeepError; // the reason words in Decided; Io: `can't write <path>: <error>`
```
graft jobs: `Command::KeepGraphMode(MergedBranches)` and its event `GraphModeKept { mode: MergedBranches, result: Result<(), String> }` in the graft-local enums the scout names (no graft-core type gains a config type).

## Touches
Each checked, and how, in the report's **Touches:** line:
- Every `?`-on-graph capture and insta snapshot (3 more lines: header + 2 rows) — list them, re-shoot each with its D3 class; `m8-keymap-config-problem-120x36`.
- `palette_row_at` (group header → `None`, rows hit), `row_count`, scrolling in a short box, `nothing matches`.
- `⌃P` typed ranking and the recently-run list (not recorded — test).
- Start-screen palette (`empty_state::palette`): no group (test).
- `first_launch` (opening the palette still counts as today).
- Tour steps that open `?` on the graph (grep).
- `cli::config_keys` / man page: the key is unchanged; the man page notes the app may write it.
- Dependencies: `toml_edit` added, `tempfile` promoted; `tests/it/dependency_edges.rs`; licence/deny config if one exists.
- Keymap: no key added; `tests/keymap_ids.txt` unchanged (rows aren't actions). Journal/undo: none.

## Acceptance
- [ ] `?` on the graph, empty query: `HERE`, then `GRAPH MODE` with `● Folded merges … (on)` and `○ Every branch`, then `EVERYWHERE`; id column `graph.merged_branches` at 120, dropped at 90. — names its test.
- [ ] Typed `graph`, `fold`, `branch` each keep both rows; `⌃P` typed `graph` lists the group; start-screen palette never does. — names its tests.
- [ ] `⏎` on `Every branch`: palette closed, `History::merged_branches() == Shown`, one `KeepGraphMode(Shown)` command, cursor commit unchanged; `⏎` on the on row: closed, no command; neither recorded as recently run; journal unchanged. Click on a mode row = `⏎`. — names its tests.
- [ ] Event ok → bar `✓ Graph: every branch · kept for next launch`; event err → `✗ Graph: every branch for this run · not kept: <reason>`, mode stays `Shown`. — names its test.
- [ ] Writer, temp dir each: missing folder + file → file is exactly `[graph]\nmerged_branches = "shown"\n`; a file with comments, `[ui]`, `[keys.graph]` and no `[graph]` → original bytes then the appended table; `[graph]` with other keys and comments → only the inserted line differs; existing key with a trailing comment, and `= "open"` → only the value differs; dotted `graph.merged_branches` edited in place; each result `config::load`s to the mode with no warning. — names its tests.
- [ ] Writer refusals: an unparsable file and `graph = 3` → `Err`, file bytes unchanged. Read-only folder → `Io`, file unchanged. No temp file left in the folder after any case. — names its tests.
- [ ] Symlinked `config.toml` → still a symlink, target updated; 0600 file stays 0600. — names its tests.
- [ ] Pty walk (`XDG_CONFIG_HOME` = temp dir, no file): `?` · `graph` · `↓` · `⏎` → top line `every branch shown`, flash; **≤ 4 steps**; relaunch → opens shown with no keys pressed; `?` `graph` `⏎` → folded, file says `"folded"`. — names its test.
- [ ] Captures: `docs/shots/ease-pass/help-graph-mode-120.txt` (`?` on `wallet_release`, typed `graph`, cursor on `Every branch`), `help-graph-mode-90.txt` (same, id column dropped), `graph-mode-switched-120.txt` (after `⏎`: top line and flash), in the contact sheet after 06h's. Look unjudged (rule 6), colour unverified (rule 5).
- [ ] NOTES: the scout's `PaletteRow` answer, every re-shot capture with its D3 class, the design-doc wording, `cargo tree -d` result.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~40k.
