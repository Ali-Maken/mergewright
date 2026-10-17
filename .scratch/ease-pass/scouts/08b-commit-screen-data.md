# Scout 08b: commit screen data paths (crates/graft unless noted)

## 1. File list + file diff
- Commands (`update/events/command.rs`): `Command::ListFiles(JobId, DiffSpec, DiffOptions)` :107; `Command::ReadFileDiff(JobId, DiffKey, FileEntry)` :113. The caller picks the id (`local_jobs.next_id()`). (`update/events.rs:131,136` are `Task` labels, not commands.)
- Runner: `jobs/runner/queue.rs:107-119` → `jobs/detail.rs:55 read_files` → `Product::Files(Result<(Arc<FileListView>, Arc<FileList>),String>)` (events.rs:281); `jobs/detail.rs:115 read_file_diff` → `Product::FileDiff(DiffKey, Result<(Arc<DiffView>, Arc<FileDiff>),String>)` (events.rs:295).
- Routing (`update/answer.rs:81-93`): a `Files` answer is first checked by `stash::claims` (stash.rs:136, `state.stash.preview == Loading::Pending(id)`). Otherwise every `Files`/`FileDiff` goes to `detail::detail_event` (detail/control.rs:29), then to `files_arrived` / `diff_arrived` (detail/ops.rs:457). Stale answers are dropped at :463 (`self.diff != Loading::Pending(id)`). The results are stored in `DetailState.diff`, `.diff_raw` and `.diff_cache`.
- Precedent for issuing the command under your own id: the stash preview, stash.rs:311 (issues ListFiles with its own id) plus the guard arm at answer.rs:81. No such guard exists for `FileDiff`. A FileDiff with a foreign id reaches `diff_arrived` and is dropped there. That is harmless, but the answer never reaches your state unless a new arm is added.
- Minimal set: push `ListFiles(id, DiffSpec::WorkingTreeVsHead, opts)`; push `ReadFileDiff(id2, DiffKey{spec: DetailSpec::WorkingTree, entry, section, options, max_lines}, entry)` (DiffKey: detail/state.rs:192); add guard arms in answer.rs before :87.
- **Contradiction (counts):** `FileEntry` does carry `added/removed: Option<u32>` (graft-git diff/mod.rs:54-64). But for WorkingTreeVsHead, `read_files` does NOT diff HEAD against the working tree. It lists Staged and Unstaged separately (jobs/detail.rs:63-66) and concatenates them by section (jobs/detail/working_tree.rs:188-209). A file that is both staged and unstaged appears twice, with split counts. Untracked entries carry `None`/`None` (graft-git diff/read.rs:163, :298-299), so they have no line counts.
- **Contradiction (diff):** `read_file_diff` takes its spec from `diff_spec_for(spec, section)` (jobs/detail/working_tree.rs:20). Staged reads the index against HEAD; Unstaged and Untracked read the worktree against the index. Only `section: None` (:38) or `Unmerged` (:36) gives WorkingTreeVsHead.

## 2. Drawing
- `graft_ui::detail_pane::diff_body(hunks:&[HunkView], top:usize, truncated:Option<(usize,usize,&Keymap)>, area:Rect, buf:&mut Buffer, theme:&Theme, focus:DiffFocus) -> usize` (graft-ui detail_pane/unified.rs:87; re-exported at mod.rs:41). It returns the total row count, for scroll clamping. `DiffFocus{focused, word_tint, cursor:Option<usize>}`: line_render.rs:22. Side-by-side variant: side_by_side.rs:200. The finish summary already calls `diff_body` directly (finish_summary/list.rs:213). The `BodyView` enum (graft-ui detail/diff.rs:173) is only drawn through `DetailPane` (body.rs:92 → `render_diff_body` :99, which is private).

## 3. Undo landed
- `update/undo/answers.rs:193 undone` (stale answers dropped at :199, `state.operating != Some(id)`) → `Undone::Done` → `undid` :154. Status is set at :168; the cursor lands via `finished_lands_on` at :180. Deferred landing: `land_after_undo` :68, called on the newest working-tree read.

## 4. Op landed
- The `✓` text: `update/operate/outcome.rs:45 done_status` returns the Status; `done_answered` assigns it at :296. Cursor: `landed` :121 calls `banner::finished_lands_on` (banner/stop.rs:179), which sets `land_on_head` and `head_landing`. `Landing::Head` is chosen by `operate.rs:195 landing_of`. The landing is spent once both the refs and the walk answer (facts.rs ~108; mod.rs:144-162).

## 5. Ahead of upstream
- No dedicated field. Use `state.refs: Option<Arc<RefBadges>>` (update/state.rs:90). `refs.branches()` yields `(tip, name, current, &Upstream)` (used at update/radar.rs:136). `Upstream::{None, InSync{remote}, Apart(AheadBehind)}`: refs/mod.rs:158. To read: take the entry with `current`, match `Apart(c)`, then `c.ahead`.

## 6. Row bar
- `view/rows/context.rs:107 cursor_row_context(state:&AppState, cursor:&GraphEntry<'_>) -> HintContext`. It returns `StashRow`, `BranchRow{current,verbs}` or `PlainRow{verbs}`, wrapped in `Rewritable`/`Marked`. Top-level picker: `view/rows.rs:772 hint_context`.
- `HintContext::GraphRow{row:RowBar, verbs:RowVerbs}`: graft-ui graph_screen/hints/context.rs:94. `RowBar` (hints/context/row_bar.rs:12) variants: `Own`, `Elsewhere{branch:SanitizedText}`, `FoldedMerge{inside:u32}`, `OpenedMerge`, `JustCommitted`, `JustMerged{inside:u32}`.
- **Note:** grep finds no `HintContext::GraphRow` or `RowBar::` constructed anywhere in non-test crates/graft/src. The app does not produce this context yet.
