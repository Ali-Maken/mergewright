# undo-and-stash-ui: `u`/`U`, the journal browser, the drift guard, interrupted-entry notice, the stash overlay, and refresh on focus

Status: done — all seven pieces; seams 4–6 finished the stash overlay, the badges and the captures
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
- [x] Journal browser: with three scripted entries `[A, B, Undo(B)]` the rows mark A `← u` and `Undo(B)` `← U`; `Space` on A shows its `commands`; `Enter` on B (not next) sets the M5 notice and emits nothing; `Enter` on A emits `Undo`; an Aborted entry with a backup reads `failed: <reason> · backup saved` and is marked `← u` when it is the next; an interrupted one reads `interrupted`; the footer notice shows once until `J`.
- [x] Stash overlay: `S` opens and emits `ReadStashes`; rows render from a scripted product; `Enter` emits `Operate(StashApply { index: sel, commit })` with the commit from the row (never an index alone); `d` → box → `StashDrop`; `s` opens the prompt, typing lands in `message`, `Tab` flips `staged_only`, `Enter` emits `StashPush { message: Some(..), include_untracked: true, staged_only }`; on a clean tree `Enter` emits nothing and the prompt's `refusal` reads `nothing to stash`.
- [x] **End-to-end on a `copy_into_temp` fixture through the real runner** (this is where the promise is first exercised by keys): (1) scripted untracked file with bytes `b"\x00\xff\n"` → `Enter` on the changes pane's untracked row … no: `d` → `→` → `Enter` (discard) → file gone → `u` → file bytes identical, journal has 3 entries (discard, undo) with the undo's `reverses` set. (2) stage a file, `c` … (commit is `commit-editor`'s; here: `Operate(Commit)` injected as a command) → `u` → `git rev-parse HEAD` is back, `git diff --cached --name-only` lists the file again → `U` → the commit is back. (3) `S` `s` type `wip` `Enter` → `git stash list` has one entry and the tree is clean → `u` → the tree is dirty again and `git stash list` is empty → `U` → stashed again → `Enter` (apply) → `u` → clean. (4) `d` on `stash@{0}` → drop → `u` → `git stash list` shows it.
- [x] Focus: a scripted `FocusGained` emits `ReadWorkingTree`; a second within 500 ms emits nothing; none emit `ShowRows` or a history restart.
- [x] Every stash commit carries a `stash@{n}` badge on a fixture with two stashes; `H` hides both.
- [x] Captures from piece 7 exist (the 8 KB figure is a spec defect, see NOTES; the standing cap is 200 KB and the largest here is 8.7 KB); `← u` / `← U` marks and the expanded `$ git …` lines are visible in the grid.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard · `GitInvocation` guard.

## Size

~1,200 lines + ~600 test. Seams: after piece 2 (undo works by key, no browser), after piece 3, after piece 4. If piece 4 (stash) is not reached in one context, stop there and say so — the conductor files it as its own group with this spec's piece 4–5 text verbatim.

## NOTES

### Progress

Three seams committed.

- Seam 1 — pieces 1 and 2: `u`/`U`/`Ctrl+Z`, the drift guard, the journal read
  at launch, the interrupted notice.
- Seam 2 — piece 3: the journal browser (`J`).
- Seam 3 — piece 6 (focus refresh), piece 7's hints and the captures that exist.

**Not started: piece 4 (stash overlay) and piece 5 (stash badges)**, and with
them the stash halves of piece 6 (`ReadStashes` on focus) and piece 7 (the
`S stash` hint is already in the footer from M1's `H stash`; the four stash
captures and the two stash acceptance walks are not). The spec's own seam rule
said to stop here and let the conductor file piece 4–5 as its own group. See
*Handoff* below.

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

### What was run for the lines ticked in seams 2 and 3

- *Journal browser*: `crates/graft/src/view/journal.rs` (5 tests — newest-first
  order and the `← u` mark, an undo entry as the `← U` target with what it
  reversed reading `undone`, an interrupted entry, an aborted entry leading with
  git's first line and still marked `← u` because it kept its backup, the empty
  and failed states); `crates/graft/src/update/journal.rs` (8 tests — `J` opens
  and re-reads, the notice is taken up, `Esc` closes and cancels a read in
  flight, `j`/`k` clamp, `Space` expands one row at a time, `Enter` on the
  `← u` row is `u` and closes, `Enter` on an older row sets the M5 line and asks
  for nothing, an open browser swallows `q`);
  `crates/graft-ui/src/journal_screen.rs` (6 widget tests over a `TestBackend`
  grid — the mark is text not colour, the expanded log, an aborted row's reason,
  the empty state's wording, a failed read's reason, and 1/2/3-row terminals).
- *Focus*: `crates/graft/src/update/tests.rs` —
  `coming_back_to_the_window_re_reads_the_working_tree_and_nothing_else` (asserts
  no `ReadHistory` and no `ShowRows`),
  `a_second_focus_report_within_half_a_second_is_ignored` (and that 600 ms later
  it reads again), `a_focus_report_before_the_repository_opened_asks_for_nothing`.
- *Captures*: `docs/shots/m3-journal-160x48.txt`, `m3-journal-80x24.txt`,
  `m3-journal-expanded.txt`, `m3-undo-drift.txt`, regenerated by the `#[ignore]`d
  `capture_m3_journal` / `capture_m3_undo_drift` in
  `crates/graft/src/view/tests.rs`. The four stash captures do not exist.
  **The spec's "under 8 KB each" is not met and should not be**: a bare 160×48
  grid is 7,824 bytes before any content, so the figure is a spec defect (the
  conductor's brief says so). Largest here is 8.7 KB; the standing cap is 200 KB.
  Nothing in a capture proves colour — the grids carry none.

### Deviations added in seams 2 and 3

- `Enter` in the browser also acts on the `← U` row (running `U`), not only the
  `← u` row. Same rule as ADR-0031 — it acts only where a key already would —
  and saying "multi-step revert comes with Ghost Preview" over the redo target
  would be wrong.
- `graft_ops::journal::in_force_all` added (public): `in_force` for every entry
  in one walk. The browser marks every row, and asking `in_force` per row walks
  the journal once per row. The marks come from `ops`, not from a rule restated
  in the view.
- `Ctrl+R` and `J` both re-read the journal; the browser opens on whatever it
  already has and replaces it when the read lands.
- `Clock::now_millis` added: the 500 ms debounce cannot be measured against a
  clock that only reports seconds.
- `graft_ui::graph_screen::footer`'s `LEADING_HINTS` went from 3 to 4 so
  `u undo` sits among the hints a narrow terminal keeps. Insta snapshots for the
  footer and the graph screens were regenerated; the only change in them is the
  hint line.
- The drift guard wraps `Drift::explain()` over several lines instead of letting
  the 64-column box clip it. The capture caught this: the box read "the index
  changed; the …", and a box whose purpose is to say what moved must not elide
  it.

### Handoff: what piece 4 and piece 5 still need

Nothing of theirs was started, so the spec's piece 4–5 text stands as written.
What the three finished seams have already put in place for them:

- `Command::Operate(JobId, Box<Operation>)` and `update::operate::launch` take
  any operation, `StashPush`/`StashApply`/`StashPop`/`StashDrop` included.
- The confirm box takes its own button words (`ConfirmView::with_labels`), which
  `d` on a stash needs (`[ Keep ] / [ Drop ]`).
- `PendingConfirm::then` is `ConfirmThen`, so a stash confirmation is a new
  variant rather than a second mechanism.
- `update::journal` and `view::journal` are the overlay shape to copy for
  `update::stash` / `view::stash`: state, `<name>_key` returning `bool` with a
  wildcard that swallows, a `Loading<Arc<…>>` field read by a job, a
  `Screen::Graph if state.<name>.shown` arm in `view::frame`.
- `Command::ReadStashes` / `Product::Stashes` / `Task::ReadStashes` were written
  during seam 1 and then **removed** to keep that seam compiling. They are not
  in the tree; write them fresh.
- Piece 6's stash half is one line in `update::mod::focus_gained`: push
  `Command::ReadStashes` too while the overlay is open.

### For the conductor

- `crates/graft/tests/it/app_harness.rs` is a shared harness. `changes_pane_writes.rs` and
  `commit_editor_writes.rs` still carry their own older copies of the same `App`; folding them in
  is not this group's scope.
- `App::pump` prints each event under `GRAFT_TEST_TRACE=1`. It earned its keep finding the id
  rewind above.

---

## NOTES — seams 4, 5 and 6 (pieces 4, 5, the stash halves of 6 and 7)

### Progress

- Seam 4 — `a9dc35e` the stash overlay, its keys and the drop box.
- Seam 5 — `9ef8705` save, apply, pop and drop, proved against real git.
- Seam 6 — `c0b36e0` a badge for every stash, and a box that agrees with itself.
- Seam 7 — this commit: the three captures, MAP.md, the ticket.

No compaction in seams 4–6; this session resumed after one, with the code
already committed and only captures, MAP.md and the ticket left.

### What was actually run for the newly ticked lines

- *Stash overlay* (scripted): `crates/graft/src/update/stash/tests.rs`, 20 tests
  — `S` opens and asks for the list; the cursor clamps at both ends and is
  pulled back when a shorter list lands; `Enter`/`p` carry the row's **commit**,
  not an index alone; `d` opens the box and runs nothing until it is answered;
  the box's title, its two button words and its safe default; typing lands in
  `message`; `Tab` flips `staged_only`; an empty message sends `None`, not
  `Some("")`; a clean tree's refusal text for both scopes and `Enter` emitting
  nothing; `Space` asks for the files and `k`/`j` cancel that read; `q` is
  swallowed; every key is a no-op while a write is in flight.
  Widget: `crates/graft-ui/src/stash_screen.rs`, 7 tests over a `TestBackend`
  grid (header count, the singular `1 file`, the prompt's `(•)` scope mark, the
  empty and failed states, a 3-row terminal, nothing drawn outside the `Rect`).
  View model: `crates/graft/src/view/stash.rs` tests.
- *End-to-end, parts (3) and (4)*: `crates/graft/tests/it/stash_by_key.rs`, over
  the real job pool and real `git` on a `copy_into_temp` fixture, each assertion
  read back out of git (`git stash list`) or off the disk — never out of
  `AppState`:
  - `s_saves_the_working_tree_and_u_takes_the_stash_back` — `s`, `wip`, `Enter`;
    one entry in `git stash list` carrying `wip`, `sample.txt` back to its
    committed bytes; `u` empties the list and restores the edit; `U` stashes
    again.
  - `enter_applies_a_stash_and_u_takes_the_application_back` — apply leaves the
    entry on the list (only pop takes it off); `u` cleans the tree again.
  - `p_pops_a_stash_and_u_puts_the_entry_back` — and it comes back at
    `stash@{0}`, asserted.
  - `d_drops_a_stash_behind_the_box_and_u_puts_it_back_at_the_top` — `j` onto
    `stash@{1}`, `d`, `→`, `Enter`; the right one of the two is gone; `u` brings
    it back at `stash@{0}`, which the test asserts by message, not by index.
- *Every stash badged*: `crates/graft/tests/it/stash_by_key.rs::every_stash_entry_is_badged_on_a_real_repository`
  (two real stashes, read through the real refs job, one `RefLabel::Stash(n)`
  per entry) and `crates/graft/src/refs/tests.rs::every_stash_entry_is_badged_by_its_own_index`
  / `a_repository_with_no_stash_has_no_stash_badges`. `H` hiding both:
  `update::tests::capital_h_hides_the_stash_and_shows_it_again`.
- *Captures*: `docs/shots/m3-stash-list.txt`, `m3-stash-save.txt`,
  `m3-stash-empty.txt`, from the `#[ignore]`d `capture_m3_stash` in
  `crates/graft/src/view/tests.rs`. 4.4–4.8 KB each. **The user judges these by
  eye**; each file's header says what to look at. They carry no colour, so
  nothing in them is evidence about colour.

### Deviations from the spec

- **`StashView` carries `notice` / `notice_is_failure`**, which the spec's
  signature does not. The overlay covers the graph screen's footer, and that
  footer is the only place `AppState::status` is drawn. Without this a write
  started from the overlay would have had no acknowledgement at all and a
  refused apply or pop would have shown nothing. The fields are fed from
  `AppState::status`, so the overlay cannot disagree with the footer under it.
  **The journal browser still has this defect** (`Status::Done(OLDER_ENTRY)`,
  set by `Enter` on an older row, is invisible while `J` is open). Not fixed —
  out of this group's scope, and it is the conductor's to file.
- **The refusal wording is `ops`', not the spec's.** The spec asked for
  `Status::Failed("stash touches files you changed: a.rs, b.rs — commit or
  stash them first")`, but `graft_ops::operation::Refusal::TouchesChangedFiles::explain`
  already names the files. Restating it in `app` would put the same rule in two
  places, against the seam rule, so the overlay shows `ops`' own words.
- **The preview is a file list with counts and no diff body**, which the spec
  allows if the builder says which — this says which. It is a one-call reuse of
  the detail pane's path: `Command::ListFiles(id, DiffSpec::Commit { commit,
  against: None }, DiffOptions::default())`, answered by `Product::Files` and
  routed by `update::stash::claims` (which asks whether the overlay's preview,
  not the pane, is waiting for that id). `against: None` is the stash's first
  parent, i.e. `stash@{n}^` — the spec's `Some(stash^)` spelled the same thing
  the long way.
- **`graft_git::stash::commits` added** (reflog only, no `diff-tree`). The badge
  path runs at startup and on every refresh, and `stash::list` spawns one
  `diff-tree` per entry, up to 50. The overlay still uses `list`, which needs
  the paths.
- **`RefLabel::Stash(usize)`** replaces the unit variant, and
  `refs::badges` takes a `stashes` argument (5 arguments now). `RefBadges::stash()`
  is kept as "the newest", so existing callers are unchanged.
- **`StashState::read` and the key handler take a `next_id` closure**, like every
  other handler after seam 1's id-rewind fix.
- `Tab`/`BackTab` are never claimed by an overlay, so focus can always reach the
  detail pane.

### A bug this found, fixed at the cause

`Enter`, `s`, `d` and `Space` in the stash overlay ran the **detail pane's**
commands instead of the overlay's: `keys::key_event` dispatched
`detail::detail_key` before either overlay, and the pane claims `Enter` (open
the selected graph row) and `s`/`d`/`Space` (the changes pane's write keys)
whenever it is shown. Two integration tests caught it, failing at
`settle_write`'s `operating.is_some()`.

Fixed at the cause, not by reordering one key: an open overlay stands where the
graph does, so it now takes the graph's keys **before** the pane whenever
`detail.focus == DetailFocus::Graph` (`keys::pane_and_overlay_key`). **The same
defect was live for the journal browser's `Enter`** since seam 2 and is fixed by
the same change. Cost to the user: none — no behaviour was wanted from the old
order; `Tab` still reaches the pane because no overlay claims it.

A second cause, same seam: a stash push moves `refs/stash`, so it refreshes
through `Refresh::Everything` → `keys::refresh_key`, which did not re-read the
stash list — the overlay went stale right after the write it had just started.
`refresh_key` now refreshes the stash list too, which also makes `Ctrl+R`
re-read it.

### For the conductor

- KNOWN_GAPS line 164 ("Only the newest stash entry has a badge") is closed by
  seam 6, proved on a real repository — yours to move.
- **New, for KNOWN_GAPS, unverified (not broken):** the journal browser's own
  status line is invisible while `J` is open, for the reason above. Cheap fix,
  same shape as `StashView::notice`.
- **New, unverified:** nothing in this group proves any colour. The captures are
  character grids.
- A stash restored by `u` re-lists at `stash@{0}`. The drop box says so in plain
  words before it runs, and the integration test asserts it.
