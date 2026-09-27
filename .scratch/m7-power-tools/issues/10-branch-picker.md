# branch-picker — `C` opens a picker of his branches and the remote's; type, `⏎` switches; `C ⏎` goes back

Status: partial (10-a built 2026-09-27; blocked: needs `crates/graft/src/jobs/runner.rs`, held by 30j — see NOTES) — respecified 2026-09-26 against scout 10. The switch rule is **approved** (field notes :79), no longer
awaited; it is built in `10b-switch-rule`, and the picker calls it.
Blocked by: 03-land-on-result, 10a-switch-refs-read, 10b-switch-rule
Crates: graft-ops (`switch_list`, shaping 10a's read); graft (`update/picker.rs` new, `GraphOverlay::Picker`,
`text_field_open`, the modal predicate, graph key `C`); graft-ui (`picker.rs` new).
Source: note "Pass 2a" R5; Decisions 4–5; field notes :73, :79; KNOWN_GAPS :772, :854;
`docs/research/checkout-cherry-pick.md` §1, §4, §7; `docs/shots/m7-proto-4-picker.txt`; scout
`.scratch/m7-power-tools/scouts/10.md`.
Build with: Opus. A new overlay on the `GraphOverlay` pattern plus a shaping read; the switch itself is 10b's.
Scout: done (`scouts/10.md`). No further scout.
Needs: `graft_git::switch_refs` (10a); `switch_to`, `SwitchTarget`, `CheckoutTarget::Track` (10b); `land_on_result`
(03). Keymap rows §2.1 picker, §2.3 `C`, §2.16 are already applied (2026-09-26).

## What changed from the first spec
- **The switch rule left this ticket.** It is `10b-switch-rule`, because it changes two existing routes (`.` Checkout,
  `b ⇥`) with their own tests and captures; the picker only calls `switch_to`. No "if approved" branches remain.
- **`C` is built here from nothing.** The first spec assumed a graph `C` switch path to copy; there is none (scout:
  the only `C` is the commit editor's, `commit_editor.rs:608`).
- **`b ⇥` was not "previewed, becomes instant"** — it was already instant and **refused** a dirty tree
  (`history.rs:510`). 10b turns that refusal into the set-aside preview.
- **The read split out** to `10a-switch-refs-read` (graft-git): `@{-1}` is read nowhere and no read lists local and
  remote together. `switch_list` here only shapes it. 10a and 10b run concurrently (disjoint crates).
- **Remote-only switch** needs a new `CheckoutTarget::Track` (none exists, `history.rs:178-183`) — in 10b.
- **`SwitchRow` signature changed** from the note: `tracks: Option<RefName>` becomes `relation: Relation`
  (`Tracks` / `BuiltOn` / `None`), and `RemoteOnly` carries its remote and local name. Why: `built on …` had no source
  in the note's struct; git's own record of a stacked branch is an upstream under `refs/heads/`, so it is read, not
  guessed. Different things get different variants.
- **Words with a set-aside:** `your edits are back` (the existing clause, as for a rebase), not
  `your N edited files put back`.
- **Sidebar `Enter` checkout** is out: no sidebar exists in code. The first spec's "unchanged, check its tests" had
  nothing to check.
- **`Esc` from the switch preview returns to the picker**, query and row kept (as `J` and `S` do). New.

## Decided (note R5, amended above)
- **`C` on the graph** (graph focus, no modal up) opens `GraphOverlay::Picker`. `switch_list` runs as a job; rows fill
  when it lands. While reading: `reading branches…`. Closing cancels the job.
- **Rows and order** (`switch_list`):
  - Nothing typed: `↩ before` (10a's `previous`, if it still exists and isn't HEAD's branch) first; then `● local`
    newest tip first; then `◌ remote` with no local copy, newest first; `· here` (HEAD's branch) last.
  - A remote ref is left out when any local branch's upstream is it, **or** a local branch has its local name (the
    switch would be refused `RefExists`).
  - A remote-only row's local name = the ref less `refs/remotes/<remote>/`, `<remote>` the longest name in
    `graft_git::remote::list` that prefixes it. A remote ref under no listed remote is left out (MAP.md says so).
  - The previous branch shows once, as `↩ before`, not again under `● local`. When typing, it ranks like any row.
  - Right column: `tracks origin/main` (upstream under `refs/remotes/`), `built on feature-a` (upstream under
    `refs/heads/`), `no copy of yours yet` (remote-only), else blank.
- **Filter:** case-insensitive over the short name. Segment-prefix match (start, or after `/` or `-`) ranks above a
  substring; ties to the newest tip. Empty: `no branch matches "<q>" · esc closes`.
- **Keys** (§2.16; modal, a text field): printable keys are typed (`j`, `k`, `q`, `C` included); `Backspace`;
  `↑`/`↓` move; `⏎` → `switch_to` (10b) with `opened_by` `"C"`; `Tab` closes and puts the graph cursor on the tip,
  writing nothing — outside the loaded history it seeks as the Map's `Enter` does (`map.rs:190-205`
  `seek_selected`, `History::seek`, `Command::ShowRows`); `Esc` closes; `Ctrl+C` quits. Mouse: click selects,
  double-click switches.
- **`⏎` targets:** `● local`/`↩ before` → `SwitchTarget::Branch`; `◌ remote` → `SwitchTarget::Track { remote: Tip {
  name, oid: row.tip }, local }`; `· here` → `you're on <b>`, nothing written, picker stays.
- **`⏎` while a write runs or git is mid-operation:** the picker stays; its bottom line shows `preview_refusal`'s
  words. `Tab` still works.
- **A preview opened from the picker** hides the picker (not closed); `Esc` on the preview shows it again with query and
  row; `y` writes and lands on the graph (`land_on_result`), picker closed.
- **Lands:** graph, cursor on the new HEAD; words are 10b's. A put-back clash lands on the resolver (ADR-0045).
- **Box:** width min(cols − 4, 150), over the lower 60% of the graph (graph above stays visible). ≥120 cols: kind,
  name, who, age, subject, relation. <120: kind glyph, name, age. At 24 rows: 10 rows, scrolling. Footer
  `<n> branches`. Bar: `type to find  ⏎ switch to it  tab show it on the graph  ↑↓ row  esc close`.
- **Error:** the read fails → `can't list branches · <error>` (sanitized, one line) in the box, `esc closes`. No
  branches (unborn HEAD) → `no branches yet · esc closes`.

## Read first
- This spec. Note "Pass 2a" R5 (the mock, rows, words), "Journey counts" rows `switch-*` and
  `cherry-pick-other-branch`.
- `docs/design/02-keymap.md` §2.1 (text fields; the modal list), §2.3 (`C`), §2.16.
- `10a-switch-refs-read.md` and `10b-switch-rule.md` Signatures.
- `crates/graft/MAP.md`: `GraphOverlay`, `AppState::text_field_open` (+ the modal predicate below it),
  `update::map` (`map_key`, `seek_selected`), `update::preview` (`open_preview`, `return_to_journal`),
  `update::journal` (`journal_key`: the hidden-under-a-preview shape). `crates/graft-ops/MAP.md` history reads.
  `crates/graft-git/MAP.md` `remote::list`.

## Signatures
```rust
// graft-ops
pub enum SwitchKind { Local, RemoteOnly { remote: String, local: String } }
pub enum Relation { Tracks(RefName), BuiltOn(RefName), None }
pub struct SwitchRow { pub name: RefName, pub kind: SwitchKind, pub tip: Oid, pub subject: String,
                       pub author: String, pub when: i64, pub relation: Relation, pub current: bool }
pub struct SwitchList { pub previous: Option<SwitchRow>, pub rows: Vec<SwitchRow> }  // rows in drawn order, `here` last
pub fn switch_list(repo: &Repo, git: &Git, stop: &dyn Stop) -> Result<SwitchList, FactsError>;
/// Pure: rank `list` for `query`; empty query = the drawn order.
pub fn filter_switch_rows<'a>(list: &'a SwitchList, query: &str) -> Vec<&'a SwitchRow>;
// graft
pub fn open_picker(state: &mut AppState) -> Vec<Command>;
pub fn picker_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
// graft-ui
pub fn draw_picker(frame: &mut Frame, area: Rect, view: &PickerView);
```
(`Repo`/`Git`: the types `switch_list`'s neighbours in graft-ops take, as the note wrote them; match them.)

## Acceptance
- [x] **graft-ops:** (10c, all eight in `crates/graft-ops/tests/it/switch_list.rs`, green) `switch_list_puts_the_previous_branch_first`; `a_remote_with_a_local_copy_shows_once`;
      `a_remote_whose_name_a_local_branch_has_is_left_out`; `remote_only_branches_are_listed_with_their_local_name`
      (a remote named `a/b`); `an_upstream_under_heads_reads_built_on`; `here_is_last`;
      `filter_ranks_segment_prefix_above_substring`; `switch_list_honours_stop`.
- [x] **App:** (all in `crates/graft/src/update/picker/tests.rs` unless named; scripted state through `update::update`,
      so the real key routing; the list injected as `Product::SwitchList` because the runner arm is missing)
      `c_opens_the_picker`; `typing_filters_segment_prefix_first`; `enter_on_a_clean_tree_switches_at_once`;
      `enter_with_edits_opens_the_set_aside_preview`; `esc_on_that_preview_returns_to_the_picker_with_the_query`;
      `enter_on_here_writes_nothing`; `enter_while_a_write_runs_refuses_in_the_box`;
      `tab_puts_the_cursor_on_the_tip_and_writes_nothing`; `tab_on_an_unloaded_tip_seeks`;
      `esc_closes_and_cancels_the_read`; `the_picker_owns_every_letter` (`j k q C u` typed, no quit, no undo);
      `a_remote_only_branch_creates_its_tracking_branch` (asserts the `Track` target launched — the branch being
      created on disk is 10b's write, proved there, not re-run here); `a_failed_read_says_so_in_the_box`;
      `update::search::tests::slash_belongs_to_every_modal_that_is_up` (picker added to `text_fields()`; a mutation
      dropping it from `text_field_open` fails this and `c_opens_the_picker`). Extra: `enter_on_before_goes_back`,
      `y_on_that_preview_closes_the_picker`, `c_needs_the_graph_focused_and_nothing_over_it`,
      `c_in_the_commit_editor_is_typed_there`, `a_stale_list_is_dropped`, `ctrl_c_still_quits`.
      **Not built:** mouse (click selects, double-click switches) — not in the Acceptance list, in the Decided list.
- [ ] **Walks** (`m7_walk`, counted against the note's `noted_keys`): `switch_named_branch` = `C login ⏎` (3);
      `switch_back` = `C ⏎` (2); `switch_with_edits` = `C login ⏎ y` (4); `cherry-pick-other-branch` = `C <type> Tab
      motion o y` (6, if `11-cherry-pick-key` has landed; else NOTES says it waits). After each: HEAD on the target,
      cursor on its row; after `switch_with_edits` the edits are back; then `u` goes back.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/picker-empty-query-187x50.txt`. **Right:** `↩ before  feature-a` first; `● local` / `◌ remote` /
    `· here` in their own column; `tracks …`, `built on …`, `no copy of yours yet` on the right; the bar as above.
  - `docs/shots/m7/picker-filtered-100x30.txt`. **Right:** glyph, name, age only; the segment-prefix match on top.
  - `docs/shots/m7/switch-landed-120x40.txt`. **Right:** `▸` on the new HEAD; `✓ on login-page · u goes back`.
  - `docs/shots/m7/picker-remote-landed-120x40.txt`. **Right:** `✓ on sara/login-page (new, tracks
    origin/sara/login-page) · u goes back`.
- [ ] Hint-bar tests green: `C` on the graph bar (at least its `more` list, §2.1 ranking;
      `every_bound_action_is_hinted_live_or_is_named_here`). MAP.md for the three crates. TESTING.md section
      "Switching" + captions + contact sheet rows.

## Touches (check each, report how)
- **Other overlays:** a new `GraphOverlay` variant. Check: every match site compiles and the overlay tests green.
- **`text_field_open` + modal predicate:** the picker joins both. Check: `slash_belongs_to_every_modal_that_is_up`;
  `u`/`Esc` globals don't fire while typing.
- **Graph key `C`:** unrouted today; the commit editor's `C` (`commit_editor.rs:608`) must stay the editor's. Check:
  the commit editor's tests green; `C` inside the editor does not open the picker.
- **Map seek** (`seek_selected`): reused, not changed. Check: the Map tests green.
- **Preview return:** the journal's `return_to_journal` mechanism is paralleled or generalized (builder's choice, named
  in NOTES). Check: the journal and stash `Esc`-back tests green.
- **Journal/undo:** entries are 10b's Checkout entries. Check: `u` in the walks.
- **Captures:** new only; no existing capture has a picker. Check: `GRAFT_UPDATE_SHOTS` pass changes no other file.

## Rules / Gates
Workspace-wide gates. Throwaway repos only.

## Size
Large (~65k). Split if needed: **-a** `switch_list` + filter + overlay + keys; **-b** the ui draw, walks, captures.

Conductor note (2026-09-27, from 10c): the read landed as `graft_ops::switch_list(repo: &RepoHandle, stop: &dyn Stop)` — no `git` argument, matching `rewrite::progress`. Filter matches the short ref name, so a remote-only row matches as `origin/x`.

## NOTES (builder, 2026-09-27)

**Partial — stopped at the seam.** Split used: **10-a** (overlay, keys, filter wiring, ui draw) done; **10-b** (runner
arm, walks, captures, bar, TESTING.md) blocked.

- **Blocked: needs `crates/graft/src/jobs/runner.rs`** (30j's). `Command::ReadSwitchList` has no dispatch arm, so in the
  real binary `C` reaches runner.rs's `other => not_a_job` → **`unreachable!` panic on the job thread. Do not ship
  this commit alone.** The arm to add (copy of `read_stashes`, runner.rs ~:1011 and ~:1161):
  `Command::ReadSwitchList(id) => self.read_switch_list(id),` and
  `fn read_switch_list(&self, id: JobId)` → `queue_as(id, Some(Task::ReadSwitchList), …)` → if cancelled
  `Outcome::Cancelled`, else `Outcome::Produced(Product::SwitchList(super::switch_list::read_switch_list(&repos,
  cancel)))`. Then drop the `#[expect(dead_code)]` on `jobs/switch_list.rs::read_switch_list`. 30j's exhaustive
  `not_a_job` will fail to compile on merge until `ReadSwitchList` is handled — good, it forces the arm.
- **Blocked too: the bar.** `C switch` on the graph `more` list and the picker's own bar (`type to find ⏎ switch to
  it tab show it on the graph ↑↓ row esc close`) need `graft-ui/src/graph_screen/hints/lists.rs` (30j) plus a
  `HintContext::Picker`. Meanwhile: `("C","switch")` in `keys/bound.rs` with a named `NEVER_HINTED` row
  (`update/tests.rs`, "a seam, not a decision"), and `view::rows::hint_context` answers `HintContext::Preview`
  (empty bar) while the picker is shown — empty, not wrong.
- **Remaining (10-b):** runner arm; walks `switch_named_branch`, `switch_back`, `switch_with_edits` in
  `tests/it/m7_walk/` (+ `cherry-pick-other-branch` if 11 landed: **11 has not landed** — only 11a is in the log, so
  that walk waits); the four captures; hint-bar entries; TESTING.md "Switching" + contact sheet rows.
- **Scout used:** `scouts/10.md` (given) and `scouts/10-build.md` (mine: overlay/job/routing template).
- **Deviations, named:**
  - **Not a `GraphOverlay` variant.** `GraphOverlay` is "an overlay that replaces the graph screen" (`state.rs`), and
    the box sits over the lower 60% with the graph drawn above it; a variant would stop the graph drawing and the
    banner riding. It is `AppState::picker: Option<PickerState>`, drawn in the graph arm after the preview and `.`
    menu (`view/frame/boxes.rs::draw_over_graph`). No `GraphOverlay` match site changed.
  - **`land_on_result` is not a function** (scout C2): an instant switch lands through 03-a's `lands_on_head`; nothing
    to call.
  - **Preview return: paralleled, not generalized.** `picker::preview_closed` / `preview_confirmed`, called beside the
    journal's `return_to_journal` restore in `preview::close_preview`, `refused`, `confirm_preview`. `switch_into`
    (30j's file) hard-codes `return_to_journal = false`; a generalized flag would need that file.
  - `map::seek_selected`'s seek body split out as `map::seek_to` (behaviour unchanged; Map tests green).
- **Touches:** Other overlays — none changed (not a variant; grep `GraphOverlay::` unchanged). `text_field_open` +
  modal predicate — picker added to `text_field_open` (so `modal_owns_keys` too); slash sweep extended; `u`/`Esc`
  typed not fired (`the_picker_owns_every_letter`). Graph `C` — routed in `keys::key_event` after the menu, before
  the commit editor; editor's `C` untouched (`c_in_the_commit_editor_is_typed_there`, commit_editor tests green).
  Map seek — `seek_to` extracted (Map tests green). Preview return — journal/stash Esc-back tests green in the
  workspace run. Journal/undo — entries are 10b's Checkout; `u` after a picker switch **not walked** (needs the
  runner). Captures — none taken; no existing capture changed (workspace nextest green, no shot diffs). `keys::key_event`
  — commit-editor claim moved to a helper `editor_key` (clippy line count), same order.
- **Cause vs symptom:** new feature; the one workaround is the empty bar under the picker (symptom-level stand-in
  until lists.rs), stated above.
- **Test-first:** tests written before the code for both new modules (compile-red), then green; not red-per-test.
- Workspace gates: see report. One flake seen once in a full run: `graft-fixtures
  remote::tests::http_remote_drop_ends_a_wedged_backend` (crate untouched; passes alone).
- No compaction.
