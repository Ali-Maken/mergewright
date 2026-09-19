# changes-pane: staged / unstaged / untracked, file and hunk staging, discard with confirmation, the first `Command::Operate`

Status: partial — see NOTES (whole-file stage/unstage/discard + confirm box built; hunk staging, sectioned model, pinned row, footer hints, captures, real-runner test not done)
Blocked by: 01-git-reads-and-patches, 02-ops-model, 03-ops-execution
Build with: Sonnet — wiring of an existing pane, a list widget, a confirm box, key maps and tests; the writes themselves are `graft_ops::apply` (mechanical by CLAUDE.md's table). Escalates to Opus on a second failure.
Needs: `git-reads-and-patches` (`status::read`, `WorkingTreeStatus`, `DiffSpec::{Staged, Unstaged}`, `hunk_patch`, `Selection`), `ops-execution` (`Context`, `Journal`, `journal_path`, `apply`, `Applied`, `ApplyError`, `Refresh`), `ops-model` (`Operation`, `Target`, `Refusal`).

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — Seam rule; New UI "Changes pane", "Confirm box", "Status line"; Proposed changes 6, 8
- `docs/design/01-view-mockups.md` §1.4; `docs/design/02-keymap.md` §2.2 (`Space`, `.`), §2.4
- `PROMPT.md` §3 "Ease-of-use rules" (destructive confirmation: consequence, exact command, safe default), §4 "Working changes"
- `KNOWN_GAPS.md` "m2-details-and-diffs/06b — working-tree row reach" (line 347)
- `crates/graft/MAP.md` §update (`Command`, `Product`, `Task`, `AppState::{detail, on_working_tree, working_tree, selected_spec, uncommitted_row_shown}`, `Status`), §update::detail (`DetailState`, `DetailSpec::WorkingTree`, `Loading`, `DiffKey`, `LocalJobs`), §update::selection, §jobs (`detail` mod, `JobRunner`, `queue_as`), §view (`detail_view`, `entries`), §working_tree, §repos
- `crates/graft-ui/MAP.md` §detail (`FileRowKind`, `FileRow`, `FileListView`, `DiffView`, `HunkView`, `LineView`, `DetailFocus`), §detail_pane (`DetailAreas`, `diff_body`, `DiffFocus`, `DetailPane`), §context_menu, §paint, §fit
- `.scratch/m2-details-and-diffs/issues/06b-working-tree-row-selectable.md` and `10-detail-keys-and-focus.md` "Built" sections — what exists, what the purity guard forbids in `update/`

## Scope

In — in this order, one commit each, turns reported:

1. **The operate seam in `app`** (`crates/graft/src/jobs/operate.rs`, new): `Command::Operate(JobId, Operation)`, `Task::Operate`, `Product::Operated(Result<Applied, String>)` + `Progress::Operating { description }` — one job; the runner opens `Journal` at `journal_path`, builds `Context` (`now` from the wall clock in the job, `random` from `getrandom`/`rand` — pick whichever is already a dependency), calls `graft_ops::apply`, stringifies the error with `explain()`. `update::operate::operated(state, result) -> Vec<Command>`: `Ok` → `Status::Done(description)` + refresh (`Refresh::WorkingTree` → `Command::ReadWorkingTree` + `DetailState::refresh`; `Everything` → the `Ctrl+R` path); `Err` → `Status::Failed(text)` (stays until `Esc`) + the same refresh. The guard test from the note: no `GitInvocation` mention under `crates/graft/src/` outside `jobs/`.
2. **Grouped working-tree model:** `Command::ReadWorkingTree` now reads `status::read` (the whole `WorkingTreeStatus`, `Arc`) and derives counts; `DetailSpec::WorkingTree` lists from it, not from `list_files(WorkingTreeVsHead)`. `FileRowKind::Section { title, count }`; `jobs::detail::assemble_working_tree(&WorkingTreeStatus) -> FileListView` (STAGED / UNSTAGED / UNTRACKED sections, files sorted by path within each; a file both staged and unstaged appears under both). `FileRow::entry` gains a `WorkingTreeRef { path, section }`; the diff for a row uses `DiffSpec::Staged` under STAGED, `Unstaged` otherwise (untracked: the hunk-less `FileDiff` M2 shows as added lines — keep). The pane header `Subject::WorkingTree` counts come from the status.
3. **Raw diff beside the view:** `Product::FileDiff` carries `(Arc<DiffView>, Arc<FileDiff>)`; `DetailState` caches both under the same `DiffKey` (`diff_raw`). `DetailState::body_cursor: usize` (line index into the flattened body), moved by `j/k/↑/↓` when `DetailFocus::Diff` (the old scroll-by-line becomes cursor-follows-scroll); `diff_body` draws `▎` in the sign cell of the cursor row and bolds it when focused (`DiffFocus` gains `cursor: Option<usize>`).
4. **Keys → operations** (`update/detail/keys.rs` + new `update/operate.rs`): in Files focus, `Space`/`s` on a file row → `Operate(Stage(Paths))` for UNSTAGED/UNTRACKED, `Unstage(Paths)` for STAGED; on a Section row → all of its files; `Ctrl+A` anywhere in the pane → `Stage(Paths(all unstaged + untracked))`. In Diff focus, `Space`/`s` → `hunk_patch(raw, hunk_of(cursor), selection)` where a hunk heading row → `WholeHunk`, a `+`/`−` row → `Lines([that line])`, a context row → `WholeHunk` of the enclosing hunk; direction by the file's section. `d` on a file row or diff row → open the confirm box for `Discard` (same targets). A `PatchError` or an unstageable row → `Status::Failed` with the reason, no command. Every key is a no-op while an `Operate` job is in flight (one write at a time; the status line says `<description>…`).
5. **Confirm box** (`crates/graft-ui/src/confirm.rs`, new): `ConfirmView { question, consequence, command, safe: SanitizedText, danger: SanitizedText, selected: Choice }`, `Confirm` widget centred over the graph area; `update::confirm::{open, confirm_key}` (`←→`/`Tab` move, `Enter` acts, `Esc` closes; modal like the context menu); `AppState::confirm: Option<PendingConfirm { view, then: Operation }>`. `d`'s consequence text: `"N added / M removed lines in the working tree; a backup is saved first, u undoes it"`; untracked: `"deletes the file; a backup is saved first, u undoes it"`; command = `Invocation::command_line()` of the op's plan (call `Operation::plan` with the current facts — `update` can't read facts (purity): show the command the plan *would* be for the common case, computed by a pure helper `operate::preview_command(&Operation, &WorkingTreeStatus, head_unborn: bool) -> String`).
6. **Pinned uncommitted row** (note change 6): `AppState::uncommitted_row_shown` is true whenever the tree is dirty; `view::entries`/`view::strips` draw it at row 0 attached to `HEAD`'s lane only when `HEAD` is the top row, else as a lone `◌` with no lane line; the `on_working_tree` selection rules unchanged. Closes the m2/06b gap row.
7. **Hints, MAP.md, captures:** footer gains `space stage  d discard  c commit` when the pane shows the working tree (width permitting; drop from the right as M1 does); `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`; `docs/shots/m3-changes-pane-{160x48,120x40,80x24}.txt` and `m3-confirm-discard.txt` via `TestBackend` on a `copy_into_temp` fixture with a scripted dirty state (a test that writes the files, deleted after, as m2/09 did).

Out: commit/amend (`commit-editor`), undo keys and the browser, stash (`undo-and-stash-ui`), `e` `$EDITOR` (`commit-editor`), `i` .gitignore (M7), hunks inside untracked files (M7), multi-line range selection (single line per `Space` in M3), mouse on section rows beyond click-to-select.

## Signatures

```rust
// crates/graft/src/update/events.rs (additions)
pub enum Command { /* … */ Operate(JobId, graft_ops::operation::Operation), ReadWorkingTree /* now reads the full status */ }
pub enum Product { /* … */ Operated(Result<graft_ops::apply::Applied, String>), WorkingTree(Result<Arc<WorkingTreeStatus>, String>), FileDiff(DiffKey, Result<(Arc<DiffView>, Arc<FileDiff>), String>) }
pub enum Progress { /* … */ Operating { description: String } }
pub enum Status { /* … */ Done(String), Failed(String), Busy(String) }

// crates/graft/src/update/operate.rs (new; pure)
pub fn operated(state: &mut AppState, result: Result<Applied, String>) -> Vec<Command>;
pub fn stage_selected(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;    // Space/s: file, section, hunk or line by focus + cursor
pub fn stage_all(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
pub fn request_discard(state: &mut AppState) -> bool;                                               // opens the confirm box; false when nothing discardable is under the cursor
pub fn preview_command(op: &Operation, status: &WorkingTreeStatus, head_unborn: bool) -> String;
pub fn operation_in_flight(state: &AppState) -> bool;

// crates/graft/src/update/confirm.rs (new; pure)
pub struct PendingConfirm { pub view: graft_ui::confirm::ConfirmView, pub then: Operation }
pub fn confirm_key(state: &mut AppState, key: &KeyPress, next_id: &mut impl FnMut() -> JobId) -> Option<Vec<Command>>;   // Some while the box is open (modal)

// crates/graft/src/jobs/detail.rs (addition; pure)
pub fn assemble_working_tree(status: &WorkingTreeStatus) -> FileListView;

// crates/graft/src/jobs/operate.rs (new; the job)
pub fn run_operation(repos: &RepoStore, op: Operation, cancel: &Cancel) -> Result<Applied, String>;

// crates/graft-ui/src/detail.rs (additions)
pub enum FileRowKind { Message { lines }, Dir { collapsed }, File { status, added, removed }, Section { title: SanitizedText, count: usize } }
pub enum WorkingTreeSection { Staged, Unstaged, Untracked }
pub struct WorkingTreeRef { pub path: Vec<u8>, pub section: WorkingTreeSection }   // FileRow::entry becomes an enum: `Commit(usize)` (index into the git list) | `WorkingTree(WorkingTreeRef)`

// crates/graft-ui/src/confirm.rs (new)
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Choice { Safe, Danger }
pub struct ConfirmView { pub question: SanitizedText, pub consequence: SanitizedText, pub command: SanitizedText, pub safe: SanitizedText, pub danger: SanitizedText, pub selected: Choice }
pub struct Confirm<'a> { pub view: &'a ConfirmView, pub theme: &'a Theme }   // Widget; 7 rows × min(72, width−4), centred in the area given
pub fn confirm_area(area: Rect) -> Rect;

// crates/graft-ui/src/detail_pane (addition)
pub struct DiffFocus { pub focused: bool, pub word_tint: bool, pub cursor: Option<usize> }
```

## Acceptance

- [ ] `assemble_working_tree` on a status with 2 staged, 1 unstaged, 1 untracked, 1 both → sections `STAGED (3)`, `UNSTAGED (2)`, `UNTRACKED (1)` with the "both" file under two sections; an empty status → one row `nothing to commit — working tree clean`.
- [ ] Update tests (pure, scripted `Product`s): `Space` on an UNSTAGED file emits `Command::Operate(_, Stage(Paths([p])))`; on a STAGED file `Unstage`; on a section row every file of it; `Ctrl+A` all unstaged+untracked; a second `Space` while the job is pending emits nothing and `Status::Busy`; `Product::Operated(Ok(WorkingTree))` emits `ReadWorkingTree` and no history restart; `Ok(Everything)` emits the `Ctrl+R` commands; `Err` sets `Status::Failed` and still refreshes; `Esc` clears `Failed`.
- [ ] Diff focus: cursor on a hunk heading + `Space` → `Target::Patch` whose bytes equal `hunk_patch(raw, h, WholeHunk)`; on a `+` line → `Lines([i])`; on a context line → the enclosing hunk.
- [ ] `d` opens the box with `selected == Safe`; `Enter` closes it with no command; `→` `Enter` emits `Operate(Discard(..))`; `Esc` closes; keys under the box never reach the pane (a `j` while open moves nothing).
- [ ] One real-runner test on a `copy_into_temp` fixture with a scripted unstaged edit: `Space` → the job runs → `git diff --cached --name-only` lists the file; `d` → `→` → `Enter` on an untracked file → the file is gone, `git for-each-ref refs/graft/backup` has one ref, the journal file has two lines. (The undo of it is `undo-and-stash-ui`'s test; here only the write.)
- [ ] Pinned row: with `HEAD` scrolled off the top and a dirty tree, row 0 is the `◌` row and `k` from row 1 reaches it (extends 06b's tests).
- [ ] Guard test: `GitInvocation` does not appear in `crates/graft/src/` outside `jobs/`; the `update/` purity guard is green.
- [ ] Captures listed in piece 7 exist, each under 8 KB; report their paths; the user judges by eye.

## NOTES (builder-sonnet, this session — partial, handoff below)

**Built** (pieces 1, 5, and a reduced piece 4/2):
- `jobs::operate::{Operated, run_operation}`, `Command::Operate`, `Task::Operate`, `Product::Operated`, `Progress::Operating`, `update::operate::{operation_in_flight, stage_selected, stage_all, request_discard, operated}`, `update::confirm::{PendingConfirm, confirm_key}`, `graft-ui::confirm::{Choice, ConfirmView, Confirm, confirm_area, confirm_rows}`.
- Changes-pane write keys: `Space`/`s` stage or unstage the cursor's **whole file**, `Ctrl+A` stages every path, `d` opens the confirm box, all wired in `update/detail/keys.rs::changes_pane_write_key`.
- Seam rule holds: `app` never builds a write `GitInvocation` or touches the journal (`tests/it/seam_rule.rs` green).
- `ApplyError::AfterState` contract honored: `Operated::Incomplete` still refreshes (`update::operate::operated`), never treated as "nothing happened".
- Gates green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` (482 lib tests + all binaries, 0 failures).

**Deviations from spec, with reasons:**
- **No grouped/sectioned model (piece 2).** Spec asked for `assemble_working_tree` building STAGED/UNSTAGED/UNTRACKED sections and a `FileRowKind::Section`/`WorkingTreeRef` row model. Instead reused the *existing* M2 pipeline (`DetailSpec::WorkingTree` → `DiffSpec::WorkingTreeVsHead` → `list_files`/`assemble_files`, already wired and tested) and derived stage/unstage direction and discard's command text by cross-referencing the cursor's selected path (`update::operate::selected_path`) against `AppState::working_tree`'s `StatusEntry` (`status_entry`). This avoided ~130 lines of new parallel file-list machinery. **Consequence, not cosmetic: untracked files do not appear as rows in the changes pane at all** (`WorkingTreeVsHead`'s `diff-index` structurally excludes them) — bigger than the design note assumed. A file both staged and unstaged shows once, not under two sections.
- **No hunk-level staging (part of piece 4).** Only whole-file `Stage`/`Unstage`/`Discard` (`Target::Paths`) built. `hunk_patch`, `Target::Patch`, cursor-driven hunk/line selection, and the diff-cursor infrastructure (piece 3: `body_cursor`, `▎` cursor row, `Product::FileDiff` carrying raw `FileDiff`) were **not started**.
- **No per-section diff routing.** Every row's diff still comes from the single `DiffSpec::WorkingTreeVsHead`, not `DiffSpec::Staged`/`Unstaged` per section (follows from the piece-2 deviation above).
- **No pinned uncommitted row (piece 6).** Left `uncommitted_row_shown`/`uncommitted_row_attached` exactly as M2 behaved (HEAD must be the topmost visible row for the row to show) — restores a regression introduced pre-compaction, does not build the "pin at row 0, detached lone `◌`" behavior the design note asks for. `uncommitted_row_attached` exists but only backs the old gate.
- **No footer hints, no captures, no real-runner integration test (piece 7).** `space stage  d discard  c commit` footer text not added. `docs/shots/m3-changes-pane-*.txt`/`m3-confirm-discard.txt` not produced this session (session ran long across a compaction; stopped at a clean, gate-green seam instead of pushing further uncaptured surface). The acceptance list's real-runner test (`copy_into_temp`, `git diff --cached`, backup ref, journal lines) was not written — coverage this session is pure-logic unit tests only (`update::operate`: 10, `update::confirm`: 5).
- **Signature deviations (clippy-driven, not scope):** `Product::Operated(Box<Operated>)` and `Command::Operate(JobId, Box<Operation>)` are boxed (spec had them unboxed) — clippy's enum-size lint (`Command`/`Product` gained a large-variant warning once wired). `Operated` is a 3-variant enum (`Done`/`Incomplete`/`Failed`), not the spec's plain `Result<Applied, String>` — needed to carry `ApplyError::AfterState`'s "happened but unread" case distinctly from a clean success, per as-built.md's contract.
- `commit/amend` (piece "c" hint) correctly out of scope per spec's own Out list.

**Handoff — what's left, in spec order:**
1. Piece 2 proper: grouped STAGED/UNSTAGED/UNTRACKED sections, `FileRowKind::Section`, `WorkingTreeRef`, untracked files as real rows. Biggest gap.
2. Piece 3: diff-cursor (`body_cursor`, `DiffFocus::cursor`, raw `FileDiff` cached alongside `DiffView`).
3. Piece 4 remainder: hunk/line-level `Space`/`s` in Diff focus, `Target::Patch` via `hunk_patch`, `PatchError::NoNewlineBoundary` surfaced as `Status::Failed` not a dead key.
4. Piece 6: pinned row at row 0 when HEAD scrolled off (currently unbuilt; old M2 gating restored instead).
5. Piece 7: footer hints, `docs/shots/` captures (4 files), the real-runner integration test.
6. `preview_command`/plan-based confirm command text: this session's `discard_command` hand-derives the three git command strings inline rather than calling `Operation::plan` — matches spec intent but doesn't reuse `plan_discard`'s own formatting; worth checking they stay in sync if `plan_discard` changes.

Given the size of what remains (roughly pieces 2–4, 6, 7 of 7), recommend the conductor split this into a follow-on group rather than one more session.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard · `GitInvocation` guard.

## Size

~1,100 lines + ~600 test across `graft` and `graft-ui`. Named seams: after piece 2 (grouped pane, no writes yet), after piece 4 (file + hunk staging, no confirm), after piece 6. The largest M2 overruns were tickets of this shape; stop at a seam rather than compact. Use a scout for `update/detail/keys.rs`'s key map and `view/detail.rs`'s header path; never read `update/mod.rs` whole.
