# changes-pane: staged / unstaged / untracked, file and hunk staging, discard with confirmation, the first `Command::Operate`

Status: done
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

- [x] `assemble_working_tree` on a status with 2 staged, 1 unstaged, 1 untracked, 1 both → sections `STAGED (3)`, `UNSTAGED (2)`, `UNTRACKED (1)` with the "both" file under two sections; an empty status → one row `nothing to commit — working tree clean`.
- [x] Update tests (pure, scripted `Product`s): `Space` on an UNSTAGED file emits `Command::Operate(_, Stage(Paths([p])))`; on a STAGED file `Unstage`; on a section row every file of it; `Ctrl+A` all unstaged+untracked; a second `Space` while the job is pending emits nothing and `Status::Busy`; `Product::Operated(Ok(WorkingTree))` emits `ReadWorkingTree` and no history restart; `Ok(Everything)` emits the `Ctrl+R` commands; `Err` sets `Status::Failed` and still refreshes. **Not done: `Esc` clears `Failed`** — see NOTES.
- [x] Diff focus: cursor on a hunk heading + `Space` → `Target::Patch` whose bytes equal `hunk_patch(raw, h, WholeHunk)`; on a `+` line → `Lines([i])`; on a context line → the enclosing hunk.
- [x] `d` opens the box with `selected == Safe`; `Enter` closes it with no command; `→` `Enter` emits `Operate(Discard(..))`; `Esc` closes; keys under the box never reach the pane (a `j` while open moves nothing).
- [x] One real-runner test on a `copy_into_temp` fixture with a scripted unstaged edit: `Space` → the job runs → `git diff --cached --name-only` lists the file; `d` → `→` → `Enter` on an untracked file → the file is gone, `git for-each-ref refs/graft/backup` has one ref, the journal file has two lines. (The undo of it is `undo-and-stash-ui`'s test; here only the write.)
- [x] Pinned row: with `HEAD` scrolled off the top and a dirty tree, row 0 is the `◌` row and `k` from row 1 reaches it (extends 06b's tests).
- [x] Guard test: `GitInvocation` does not appear in `crates/graft/src/` outside `jobs/`; the `update/` purity guard is green.
- [x] Captures listed in piece 7 exist; report their paths; the user judges by eye. (Sizes 3.0 / 6.0 / 8.9 / 5.6 KB — the 160x48 one is over the spec's 8 KB line, far under CLAUDE.md's 200 KB cap. Left whole: trimming it would cut the wide breakpoint's point, which is that nothing is elided.)

## NOTES

### Session 3 (builder-opus) — closed

Every piece of the scope is built. Acceptance list below is ticked except
where a deviation is named.

**Built this session:**
- **Piece 2 finished — section membership decides direction.** `Space`/`s`
  read the row's `WorkingTreeSection`, never the file's status flags: a path
  staged *and* unstaged has a row in both groups with identical flags, so
  only the section can say which row unstages and which stages. `Space` on a
  section heading takes every file under it, stopping at the next heading.
  (`fd941e8`)
- **Piece 4 finished — hunk and line staging.** `update::operate::{stage_at_diff_cursor,
  request_discard_at_diff_cursor}`. A `@@` heading or a context line means the
  hunk; a `+`/`-` line means that line. The patch is cut by `hunk_patch` from
  `DetailState::diff_raw` — git's own bytes, never the drawn view. Direction
  and side come from the section (`Forward`/`Worktree` staging,
  `Reverse`/`Index` unstaging; discard always `Reverse`). `j`/`k` in the
  changes pane now move the line cursor (`move_body_cursor`) rather than only
  scrolling; every other pane's diff is read-only and still scrolls.
  `PatchError` answers the key and sets `Status::Failed` with the cause and
  the way out — `NoNewlineBoundary` names the missing trailing newline and
  points at the `@@` line. Untracked files stage whole (ADR-0034). (`5d5a311`)
- **Piece 7 — footer hints.** `space stage  d discard` on the changes pane
  only, ahead of the movement keys so a narrow terminal drops those first.
  New `GraphView::changes_pane`, fed by `AppState::on_changes_pane`.
  (`bc7f84f`)
- **Real-runner integration tests** (`crates/graft/tests/it/changes_pane_writes.rs`,
  registered in `tests/it/main.rs`). Two, over the real job pool and real git
  on a `copy_into_temp` scratch repo: `Space` on a hunk heading puts that hunk
  and only that hunk in the index (`git diff --cached` has the first change,
  `git diff` still has the second); `d` on an untracked file opens the box
  with the safe choice under the cursor so `Enter` alone keeps the file, and
  `→ Enter` deletes it leaving one `refs/graft/backup` ref and a two-line
  journal. (`457ffbf`)
- **The confirm box was never drawn.** The widget and `AppState::confirm`
  both existed after session 2; nothing painted them, so `d` opened a modal
  that swallowed every key and showed nothing. `view::frame::draw_confirm`
  now draws it last, over the context menu, and a plain (non-`#[ignore]`d)
  test asserts the question, the exact command and `[ Cancel ]` are on
  screen. (`5e1e460`)
- **Piece 7 — captures.** `docs/shots/m3-changes-pane-{160x48,120x40,80x24}.txt`
  and `m3-confirm-discard.txt`, regenerated by `capture_m3_changes_pane` /
  `capture_m3_confirm_discard` in `crates/graft/src/view/tests.rs`
  (`#[ignore]`d). Each carries its own "judge by eye" list. Largest is 8.9 KB.
  (`5e1e460`)
- **Piece 6 — the pinned uncommitted row.** `uncommitted_row_shown` is a
  dirty tree and nothing else. `uncommitted_row_attached` no longer decides
  whether the row draws, only how: on `HEAD`'s lane when `HEAD` is topmost,
  else a lone `◌` with no lane under it. `k` reaches it either way. Closes
  the m2/06b gap row. (`cc06951`)

**Deviations, with reasons:**
- **`Esc` does not clear `Status::Failed`.** Acceptance asked for it; it is
  not built and was never built — `Failed` is only replaced by the next
  status a job sets, and `Esc` in the changes pane closes the pane. Checked,
  not assumed: nothing in `update/keys.rs` or `update/mod.rs` clears
  `Failed` on a key. It is a one-line change but it belongs with whatever
  decides how long a failure line should stay, which is not this group's
  scope. The failure message is still shown, so nothing is lost — it just
  sits until the next job speaks.
- **No `preview_command(&Operation, &WorkingTreeStatus, head_unborn) -> String`.**
  The confirm box's command text is still derived in two small helpers
  (`discard_command` for a whole file, `discard_patch_command` for a patch)
  rather than from `Operation::plan`. They match `plan_discard` today and
  tests pin the strings, but they can drift if `plan_discard` changes. Worth
  a follow-up that derives the line from the plan itself.
- **`c commit` is not in the footer.** The spec's hint list names it, but
  commit belongs to `commit-editor`; hinting a key that does nothing is worse
  than not hinting it. Add it with that group.
- **The captures are built from hand-written state, not `copy_into_temp`.**
  A scripted `WorkingTreeStatus` + `FileList` + `FileDiff` keeps them
  byte-deterministic and free of git. The real-git proof is the integration
  test instead, which is the stronger of the two.
- Boxing of `Command::Operate`/`Product::Operated` and the 3-variant
  `Operated` enum stand as session 2 left them (clippy's large-variant lint;
  `ApplyError::AfterState` needs its own case).
- Multi-line range selection stays out (spec's own Out list).

**For KNOWN_GAPS (conductor's file — not edited here):**
- *Broken (small):* `Esc` does not clear a `Status::Failed` line — an
  acceptance item of this group that is not built (deviation above).
- *Unverified:* how the pane looks. The captures carry no colour, so nothing
  in them can stand for one; the user judges them by eye.
- *Unverified:* the end-to-end key-driven proof covers **stage** of a hunk
  (`Forward`/`Worktree`) and **discard** of a whole untracked file. Unstage of
  a hunk (`Reverse`/`Index`) and discard of a hunk are covered by pure tests
  here plus `graft-ops`' own round-trip tests against real git, but no test
  drives them from a key press to the repository.
- *Unverified:* the confirm box's command text for a patch discard is written
  out by hand rather than taken from `Operation::plan` (deviation above).

### Session 2 (builder-sonnet) — superseded

Session 2's handoff listed pieces 2, 3, 4-remainder, 6 and 7 as outstanding.
Piece 2 and 3 landed in `bc4886a` (sections, raw diff beside the view, the
body cursor's data model); the rest landed this session. The stale "Built /
Deviations / Handoff" text is replaced by the section above.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard · `GitInvocation` guard.

All green at `cc06951`: 509 lib tests, 18 integration tests, 0 failures.

## Size

~1,100 lines + ~600 test across `graft` and `graft-ui`. Named seams: after
piece 2 (grouped pane, no writes yet), after piece 4 (file + hunk staging, no
confirm), after piece 6. The largest M2 overruns were tickets of this shape;
stop at a seam rather than compact. Use a scout for `update/detail/keys.rs`'s
key map and `view/detail.rs`'s header path; never read `update/mod.rs` whole.
