# changes-pane: staged / unstaged / untracked, file and hunk staging, discard with confirmation, the first `Command::Operate`

Status: ready
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

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard · `GitInvocation` guard.

## Size

~1,100 lines + ~600 test across `graft` and `graft-ui`. Named seams: after piece 2 (grouped pane, no writes yet), after piece 4 (file + hunk staging, no confirm), after piece 6. The largest M2 overruns were tickets of this shape; stop at a seam rather than compact. Use a scout for `update/detail/keys.rs`'s key map and `view/detail.rs`'s header path; never read `update/mod.rs` whole.
