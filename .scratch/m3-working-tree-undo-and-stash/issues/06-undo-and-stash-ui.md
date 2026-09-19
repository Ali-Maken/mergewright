# undo-and-stash-ui: `u`/`U`, the journal browser, the drift guard, interrupted-entry notice, the stash overlay, and refresh on focus

Status: ready
Blocked by: 01-git-reads-and-patches, 02-ops-model, 03-ops-execution, 04-changes-pane, 05-commit-editor
Build with: Opus — undo is on Opus by CLAUDE.md's table; the drift guard and the aborted-recovery path decide what a wrong keypress costs the user.
Needs: `ops-execution` (`undo`, `redo`, `UndoError`, `Drift`, `Journal::{open, entries, unresolved, mark_interrupted}`, `journal_path`, `Applied`), `ops-model` (`next_undo`, `next_redo`, `Operation::Stash*`, `StashParams`, `Refusal`), `git-reads-and-patches` (`stash::list`, `StashEntry`), `changes-pane` (`Command::Operate`, `update::operate::operated`, `Status::*`, the confirm box), `commit-editor` (`TextField`, `FieldWidget`).

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — Promise; New UI "Stash overlay", "Journal browser", "Confirm box", "Status line"; Proposed changes 2, 4, 5, 7; Hard choice 4
- `docs/adr/0031` (no time-machine key), `docs/adr/0002` (`Ctrl+Z`), `docs/adr/0033`
- `docs/design/01-view-mockups.md` §1.10, §1.11; `docs/design/02-keymap.md` §2.2 (`u`, `U`, `Ctrl+R`), §2.3 (`S`, `H`), §2.9, §2.11
- `docs/design/06-undo-journal-design.md` §6.6 (what M3 does *not* build), §6.7
- `PROMPT.md` §3 "Undo and redo", §4 "Stash", Technology "Auto-refresh" (line 58)
- `KNOWN_GAPS.md` "Only the newest stash entry has a badge" (line 164)
- `crates/graft/MAP.md` §update (`Command`, `Product`, `Status`, `AppState::{refs, working_tree, map}`), §update::map (the `M` overlay precedent: open/close/key/event shape to copy), §refs (`RefBadges::stash`, `read`, `badges`), §event_loop (`input::spawn_reader` — `FocusGained` already reaches it), §jobs
- `crates/graft-ui/MAP.md` §map_screen (overlay widget precedent), §graph_screen::badges (`BadgeKind::Stash`), §confirm (from `changes-pane`), §text_input (from `commit-editor`)

## Scope

In — in this order, one commit each, turns reported:

1. **Undo/redo keys** (`crates/graft/src/update/undo.rs`, `crates/graft/src/jobs/undo.rs`, new): `u`/`Ctrl+Z` → `Command::Undo(JobId, force: false)`; `U` → `Command::Redo(JobId, false)`; the job opens the journal, builds a `Context`, calls `graft_ops::undo`/`redo`; `Product::Undone(Result<Applied, UndoOutcome>)` where `UndoOutcome::Drift(Drift)` opens the confirm box (`Since this <description>, <drift.explain()>. Undo anyway?`, `[ Stop ]` safe, `[ Undo ]` → `Command::Undo(_, true)`), `Nothing` → `Status::Done("nothing to undo")`, any other error → `Status::Failed`. Success → `Status::Done("undid: <description>")` + refresh by `Applied::refresh`. One write at a time: `operation_in_flight` covers undo too.
2. **Journal on launch:** `Command::ReadJournal(JobId)` at startup and after every `Operated`/`Undone` product; `Product::Journal(Result<Arc<Vec<JournalEntry>>, String>)` → `AppState::journal: Loading<Arc<Vec<JournalEntry>>>`. Unresolved Pending entries are `mark_interrupted` by the job before it answers, and the footer shows `an operation was interrupted: <description> · J to inspect` until `J` is pressed once.
3. **Journal browser** (`crates/graft-ui/src/journal_screen.rs`, `crates/graft/src/update/journal.rs`, `crates/graft/src/view/journal.rs`, new; copy the Map view's open/close/key/event shape): `J` opens over the graph area; rows newest first per the note (`2m ago · commit "Fix race" · main moved · backup saved · ← u`); `j/k` move; `Space` expands the Command Log lines (`$ git commit -F -`) and, for an Aborted entry, its reason in full; `Enter` on the `← u` row = `u`; on any other row `Status::Done("multi-step revert comes with Ghost Preview (M5)")`; `Esc`/`J` close. Empty state per the note.
4. **Stash overlay** (`crates/graft-ui/src/stash_screen.rs`, `crates/graft/src/update/stash.rs`, `crates/graft/src/view/stash.rs`, new): `S` opens over the graph area; `Command::ReadStashes(JobId)` → `Product::Stashes(Result<Arc<Vec<StashEntry>>, String>)` (also re-read after every operate/undo product); rows per the note; `Space` asks `Command::ListFiles(DiffSpec::Commit { commit: stash, against: Some(stash^) })` and shows the file list under the rows (reuse `DetailState`'s list path only if it is a one-call reuse; otherwise a plain file list with counts and no diff body — say which); `Enter` → `Operate(StashApply { index, commit })`, `p` → `StashPop`, `d` → confirm box → `StashDrop`; `s` → the save prompt: one `TextField` row `message`, one option row `( ) all incl. untracked  ( ) staged only` toggled by `Tab`, `Enter` → `Operate(StashPush { message, include_untracked: true, staged_only })`, `Esc` back to the list; a clean tree shows `nothing to stash` in the prompt's title and `Enter` does nothing. A `Refusal::TouchesChangedFiles` from apply/pop shows as `Status::Failed("stash touches files you changed: a.rs, b.rs — commit or stash them first")`.
5. **Stash badges for every entry** (KNOWN_GAPS line 164): `RefBadges` carries every stash commit from `stash::list` (job side, `refs::read`), `BadgeKind::Stash` badges draw `stash@{n}`; `H` hides them all as it hides the newest today.
6. **Refresh on focus** (note change 7): `FocusGained` → `Command::ReadWorkingTree` (+ `ReadStashes` while the overlay is open), debounced to one per 500 ms in `update`; no history restart. `Ctrl+R` unchanged.
7. **Hints, MAP.md, captures:** footer `u undo  S stash  J journal` (width permitting); `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`; `docs/shots/m3-journal-{160x48,80x24}.txt`, `m3-journal-expanded.txt`, `m3-stash-list.txt`, `m3-stash-save.txt`, `m3-undo-drift.txt` via `TestBackend` with scripted products.

Out: multi-step revert / Ghost Preview (M5), "branch from stash" (M5), selected-files stash (M7), the `notify` watcher (M4; KNOWN_GAPS row), `Ctrl+Shift+Z` (documented as unreliable; `U` is the binding), "clear backups older than…" (§6.8 maintenance; M7/M8 — KNOWN_GAPS row saying backups accumulate until then).

## Signatures

```rust
// crates/graft/src/update/events.rs (additions)
pub enum Command { /* … */ Undo(JobId, bool), Redo(JobId, bool), ReadJournal(JobId), ReadStashes(JobId) }
pub enum Product { /* … */ Undone(Result<Applied, UndoOutcome>), Journal(Result<Arc<Vec<JournalEntry>>, String>), Stashes(Result<Arc<Vec<StashEntry>>, String>) }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum UndoOutcome { Nothing, Drift { description: String, drift: Drift, redo: bool }, Failed(String) }

// crates/graft/src/update/undo.rs (pure)
pub fn undo_key(state: &mut AppState, redo: bool, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
pub fn undone(state: &mut AppState, result: Result<Applied, UndoOutcome>, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;   // opens the drift box or sets the status; refreshes
pub fn interrupted_notice(state: &AppState) -> Option<String>;   // the footer line, until dismissed

// crates/graft/src/jobs/undo.rs
pub fn run_undo(repos: &RepoStore, redo: bool, force: bool, cancel: &Cancel) -> Result<Applied, UndoOutcome>;
pub fn read_journal(repos: &RepoStore) -> Result<Arc<Vec<JournalEntry>>, String>;   // marks unresolved entries interrupted first

// crates/graft/src/update/journal.rs (pure; the Map view's shape)
pub struct JournalState { pub shown: bool, pub selected: usize, pub expanded: Option<usize>, pub notice_seen: bool }
pub fn journal_key(state: &mut AppState, key: &KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn journal_event(state: &mut AppState, product: &Product) -> bool;

// crates/graft-ui/src/journal_screen.rs
pub struct JournalRowView { pub age: SanitizedText, pub description: SanitizedText, pub summary: SanitizedText, pub mark: RowMark, pub commands: Vec<SanitizedText>, pub failure: Option<SanitizedText>, pub expanded: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum RowMark { None, NextUndo, NextRedo, Interrupted, Undone }
pub struct JournalView { pub location: SanitizedText, pub rows: Vec<JournalRowView>, pub selected: usize, pub status: JournalStatus }
pub enum JournalStatus { Loading, Empty, Failed(SanitizedText), Ready }
pub struct JournalScreen<'a> { pub view: &'a JournalView, pub theme: &'a Theme }   // Widget

// crates/graft/src/update/stash.rs (pure)
pub struct StashState { pub shown: bool, pub selected: usize, pub entries: Loading<Arc<Vec<StashEntry>>>, pub preview: Loading<Arc<FileListView>>, pub prompt: Option<StashPrompt> }
pub struct StashPrompt { pub message: TextField, pub staged_only: bool }
pub fn stash_key(state: &mut AppState, key: &KeyPress, next_id: &mut impl FnMut() -> JobId) -> bool;
pub fn stash_event(state: &mut AppState, product: &Product) -> bool;

// crates/graft-ui/src/stash_screen.rs
pub struct StashRowView { pub name: SanitizedText /* stash@{n} */, pub message: SanitizedText, pub age: SanitizedText, pub files: usize }
pub struct StashPromptView<'a> { pub title: SanitizedText, pub message: FieldView<'a>, pub staged_only: bool, pub refusal: Option<SanitizedText> }
pub struct StashView<'a> { pub rows: Vec<StashRowView>, pub selected: usize, pub preview: Option<&'a FileListView>, pub prompt: Option<StashPromptView<'a>>, pub status: StashStatus }
pub enum StashStatus { Loading, Empty, Failed(SanitizedText), Ready }
pub struct StashScreen<'a> { pub view: &'a StashView<'a>, pub theme: &'a Theme }

// crates/graft/src/refs (addition)
impl RefBadges { pub fn stashes(&self) -> &[(Oid, usize)]; }   // every stash commit with its index; `stash()` stays as index 0
```

## Acceptance

- [x] Update tests: `u` emits `Command::Undo(_, false)`; `Product::Undone(Ok(..))` sets `Status::Done("undid: …")` and refreshes per `Applied::refresh`; `Err(Drift)` opens the confirm box whose consequence line contains `drift.explain()`, `→` `Enter` emits `Undo(_, true)`, `Enter` on Stop emits nothing; `Err(Nothing)` → `Status::Done("nothing to undo")`; `u` while a write is in flight emits nothing (`Busy`); `U` mirrors with `Redo`.
- [ ] Journal browser: with three scripted entries `[A, B, Undo(B)]` the rows mark A `← u` and `Undo(B)` `← U`; `Space` on A shows its `commands`; `Enter` on B (not next) sets the M5 notice and emits nothing; `Enter` on A emits `Undo`; an Aborted entry with a backup reads `failed: <reason> · backup saved` and is marked `← u` when it is the next; an interrupted one reads `interrupted`; the footer notice shows once until `J`.
- [ ] Stash overlay: `S` opens and emits `ReadStashes`; rows render from a scripted product; `Enter` emits `Operate(StashApply { index: sel, commit })` with the commit from the row (never an index alone); `d` → box → `StashDrop`; `s` opens the prompt, typing lands in `message`, `Tab` flips `staged_only`, `Enter` emits `StashPush { message: Some(..), include_untracked: true, staged_only }`; on a clean tree `Enter` emits nothing and the prompt's `refusal` reads `nothing to stash`.
- [~] **End-to-end on a `copy_into_temp` fixture through the real runner** (this is where the promise is first exercised by keys): (1) scripted untracked file with bytes `b"\x00\xff\n"` → `Enter` on the changes pane's untracked row … no: `d` → `→` → `Enter` (discard) → file gone → `u` → file bytes identical, journal has 3 entries (discard, undo) with the undo's `reverses` set. (2) stage a file, `c` … (commit is `commit-editor`'s; here: `Operate(Commit)` injected as a command) → `u` → `git rev-parse HEAD` is back, `git diff --cached --name-only` lists the file again → `U` → the commit is back. (3) `S` `s` type `wip` `Enter` → `git stash list` has one entry and the tree is clean → `u` → the tree is dirty again and `git stash list` is empty → `U` → stashed again → `Enter` (apply) → `u` → clean. (4) `d` on `stash@{0}` → drop → `u` → `git stash list` shows it.
- [ ] Focus: a scripted `FocusGained` emits `ReadWorkingTree`; a second within 500 ms emits nothing; none emit `ShowRows` or a history restart.
- [ ] Every stash commit carries a `stash@{n}` badge on a fixture with two stashes; `H` hides both.
- [ ] Captures from piece 7 exist, under 8 KB each; `← u` / `← U` marks and the expanded `$ git …` lines are visible in the grid.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard · `GitInvocation` guard.

## Size

~1,200 lines + ~600 test. Seams: after piece 2 (undo works by key, no browser), after piece 3, after piece 4. If piece 4 (stash) is not reached in one context, stop there and say so — the conductor files it as its own group with this spec's piece 4–5 text verbatim.

## NOTES

### Progress

Seam 1 (pieces 1 and 2) committed. Pieces 3–7 not started.

### What was actually run for the ticked lines

- *Update tests*: `crates/graft/src/update/undo.rs`, 14 tests — `u_asks_for_an_undo_and_says_so`,
  `capital_u_asks_for_a_redo`, `u_while_a_write_is_running_asks_for_nothing`,
  `a_finished_undo_says_what_it_undid_and_re_reads_the_working_tree`,
  `an_undo_that_moved_refs_re_reads_everything`, `an_answer_for_another_job_is_dropped`,
  `nothing_to_undo_is_an_answer_not_a_failure`, `a_failed_undo_stays_on_screen_past_its_own_refresh`,
  `drift_opens_a_box_that_quotes_what_moved_and_answers_nothing_yet`,
  `enter_on_stop_changes_nothing_and_enter_on_undo_forces`, `a_redo_drift_box_re_runs_the_redo`,
  plus three for the interrupted notice.
- *End-to-end*, parts (1) and (2) only, in `crates/graft/tests/it/undo_by_key.rs` over the real
  job pool and real `git`: `u_brings_a_discarded_untracked_file_back_byte_for_byte` (discard an
  untracked file holding `b"\x00\xff\n"`, `u`, read the file back off disk and compare bytes),
  `u_takes_a_commit_back_and_capital_u_puts_it_again` (`git rev-parse HEAD` and
  `git diff --cached --name-only` read out of git, not out of `AppState`),
  `u_on_a_repository_nothing_has_written_says_there_is_nothing_to_undo`.
  Parts (3) and (4) are stash and wait on piece 4.

### Deviations from the spec

- `jobs::undo::Undone` is a five-variant enum (`Done`/`Incomplete`/`Nothing`/`Drift`/`Failed`)
  rather than the spec's `Result<Applied, UndoOutcome>`: a `Result` cannot carry
  `RestoreError::AfterState`, which holds a real `Applied` — the undo happened. Mirrors
  `jobs::operate::Operated`, which was changed for the same reason in group 04.
- `undo::undone` takes the answering `JobId` as well, so an answer for a job that is no longer
  the one in flight is dropped rather than overwriting the status line.
- The status line says the entry's own description (`undo: stage src/a.rs`), not
  `undid: <description>`: `InversePlan::description` already prefixes `undo:`/`redo:`, so the
  spec's wording read `undid: undo: …`.
- An undo failure is `Status::Sticky`, not `Status::Failed`: the refresh the failure itself asks
  for would otherwise wipe the reason a moment after it appeared (m3/05 review Major 7).
- The launch read calls `graft_ops::apply::recover_interrupted`, not `Journal::mark_interrupted`:
  only the former seals the recovered entry's trees behind its backup ref (group 03's review fix).
  It had no caller in `graft` before this.
- `PendingConfirm::then` is now `ConfirmThen::{Operate, Undo}`; it was an `Operation`.
- `Ctrl+R` re-reads the journal too — another Graft on the same worktree appends to it, and an
  entry this run has not seen is one `u` would not offer to take back. This keeps
  `Refresh::Everything` byte-identical to `Ctrl+R`, which m3/04 review M8 asked for.

### A bug this found, fixed at the cause

`u` straight after a commit answered under another job's name and vanished: `operating` cleared,
status `Ready`, nothing said. Cause: a caller that borrows `local_jobs` out into a closure
(`answer`, `settle`, every key handler) writes its **copy** back when the callee returns, so ids
the callee minted from `state.local_jobs` were thrown away and the next mint reissued an id still
in flight. `keys::refresh_key` and `operate::write_job_ended` both minted that way.
Both now take a `next_id` closure, so one pass has one counter.
Regression test: `operate::tests::a_refresh_never_mints_from_the_states_own_counter`.
Cost to the user: none — no behaviour was wanted from the rewind. It was live before this ticket;
it only ever collided detail-pane ids with each other, which the pane drops as stale.

### For the conductor

- `crates/graft/tests/it/app_harness.rs` is a shared harness. `changes_pane_writes.rs` and
  `commit_editor_writes.rs` still carry their own older copies of the same `App`; folding them in
  is not this group's scope.
- `App::pump` prints each event under `GRAFT_TEST_TRACE=1`. It earned its keep finding the id
  rewind above.
