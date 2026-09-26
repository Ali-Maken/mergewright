# diff-line-cursor — every diff has the line cursor the changes pane has; one call lands graph + file + line

Status: ready
Blocked by: none. (Independent of 18/20a; holds graft, graft-ui, so it runs before 20c and 20, never beside them.)
Crates: graft, graft-ui.
Build with: **Opus**. Brownfield state change in the detail pane plus an async three-read landing; judged by tests,
but the landing's drop/fail rules are easy to get subtly wrong.
Scout: done (`.scratch/m7-power-tools/scouts/blame-place.md` P-a, P-g, Q2, Q5, Q6). No second scout.
Needs: none.

## Why this exists (what the scout found)
- A commit's diff has no line cursor: `DetailState::body_cursor()` is `None` unless the spec is `WorkingTree`
  (`crates/graft/src/update/detail/ops.rs:390-392`); `j`/`k` there only scroll (`detail/keys.rs:303-307`). Blame's
  `B` needs "the line he's on" (B1 rows 1–2), and journey `blame-line-to-commit` starts "cursor on line 22".
- Nothing lands on commit + file + line together (scout Q5: seek, open, select file are three steps; `show_cursor`
  zeroes the body cursor, `ops.rs:318-320`). Blame's `⏎`, history's `⏎` (history-place) and the trail's last `Esc`
  (20) all need it.

## Decided
- **The cursor:** every `DetailSpec` (`Commit`, `Compare`, `WorkingTree`) has the body cursor the changes pane has
  today. In a diff body `j`/`k`, `↑`/`↓`, PgUp/PgDn, `g`/`G` move the cursor; the view scrolls only when the cursor would
  leave it (as the changes pane). One behaviour for one key across both panes (UX §3.6 consistency).
  - It opens where the changes pane's opens. `L` (load the whole file), `w`, `[`/`]` keep it on the same side and line
    number, as the changes pane's refresh keeps it (`kept_cursor`, `ops.rs:260-275`).
  - **Non-colour carrier:** the cursor row must be findable in a capture (rule 5). If the changes pane's cursor row
    already carries a glyph, the commit diff uses the same one. If it carries only colour, both panes gain `▸` in the
    first free cell of the line-number gutter, and the changes pane's captures are re-shot (Touches).
- **`diff_cursor`** answers "what is under the cursor" for both panes: path, old path, side and line, binary,
  working-tree section. Rules for `at`:
  - an added or context line → `New`, its new number; a removed line → `Old`, its old number;
  - a hunk heading → the hunk's first changed line (first added, else first removed);
  - a file row → the file's first changed line by the same rule, from the loaded diff; `None` if the diff hasn't
    arrived;
  - side-by-side: a row with a new side → `New`; a removed-only row → `Old`.
- **`show_line`:** moves the graph cursor to the spec's row (`Commit(c)` → `c`; `Compare{head}` → `head`, base kept
  pinned; `WorkingTree` → the uncommitted-changes row), opens the detail pane on the spec, selects `path`, and when the
  diff arrives puts the cursor on `at` (nearest row at or before it on that side if the line isn't shown) with **focus
  on the diff body**. Scroll: the hunk heading at the viewport's top if the cursor row then fits, else the cursor row a
  third of the way down.
  - One pending landing at a time (a new `show_line` replaces it). A key or click that moves the graph cursor or the
    detail selection drops it.
  - The seek can't reach the commit → status `can't show a1b2c3d on the graph: <seek's words>`; nothing moves.
  - The path isn't in the spec's files → the detail pane on the spec as it opens by default; status
    `src/x.rs isn't among a1b2c3d's files`.

## Read first
- The scout file above: P-a, P-g, Q2, Q5, Q6.
- `crates/graft/MAP.md`: `DetailSpec`, `KeptCursor`, `AppState::selected_spec`, the detail-pane `update::detail` rows,
  the history seek. `crates/graft-ui/MAP.md`: `detail_pane` / `DiffView` / `BodyRow`.
- `docs/design/02-keymap.md` §2.2 (movement), §2.5.
- `docs/research/ux-principles.md` §3.6.

## Scope
- In: the cursor for commit and compare diffs; `diff_cursor`; `show_line` and its pending landing; the cursor-row
  carrier; tests; one capture.
- Out: `B`, blame, the trail (20); any new key; the changes pane's write keys (unchanged; they already read its cursor).

## Signatures (exact)
```rust
// graft (app) — update::detail, re-exported where 20 and history-place reach it
pub enum Side { Old, New }
pub struct LineRef { pub side: Side, pub line: u32 }                 // 1-based, in that side's file
pub struct DiffCursor {
    pub spec: DetailSpec,
    pub section: Option<WorkingTreeSection>,   // Some only in the changes pane
    pub path: PathBuf,                         // new-side path
    pub old_path: Option<PathBuf>,             // Some when renamed (FileEntry.old_path)
    pub binary: bool,                          // FileEntry added/removed are None
    pub at: Option<LineRef>,                   // rules above; None: diff not arrived
}
pub fn diff_cursor(state: &AppState) -> Option<DiffCursor>;          // None: the pane isn't focused on a file or diff
pub fn show_line(state: &mut AppState, spec: DetailSpec, path: &Path, at: Option<LineRef>) -> Vec<Command>;
```
`show_line`'s pending state lives on `AppState`; its name and fields are the builder's.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Cursor:** `commit_diff_j_moves_the_line_cursor` (red first) · `commit_diff_view_scrolls_only_when_the_cursor_leaves_it`
      · `compare_diff_has_the_line_cursor` · `load_whole_file_keeps_the_commit_diff_cursor_line` ·
      `changes_pane_cursor_is_unchanged` (its existing tests green, named in NOTES).
- [ ] **`diff_cursor`:** `…_added_line_is_new_side` · `…_removed_line_is_old_side_with_the_old_path` ·
      `…_heading_is_the_hunks_first_changed_line` · `…_file_row_is_the_files_first_changed_line` ·
      `…_file_row_before_the_diff_arrives_is_none` · `…_binary_file_says_binary` · `…_untracked_row_has_its_section` ·
      `…_is_none_with_the_graph_focused`.
- [ ] **`show_line`:** `show_line_lands_on_commit_file_and_line` (red first; the commit off the loaded window, so the
      seek is async) · `show_line_puts_the_hunk_heading_on_top` · `show_line_focuses_the_diff_body` ·
      `show_line_working_tree_lands_on_the_uncommitted_row` · `show_line_path_missing_says_so` ·
      `show_line_unreachable_commit_says_so_and_moves_nothing` · `a_key_during_a_landing_drops_it` ·
      `a_second_show_line_replaces_the_first`.
- [ ] **Capture** (pty, `GRAFT_UPDATE_SHOTS=1`, ≤ 200 KB): `docs/shots/m7/diff-cursor-commit-120x40.txt`. Right: a
      commit's diff, the cursor on an added line after three `j`, that row carrying the carrier glyph and no other row
      carrying it. Colour of the bar: unverified (rule 5).
- [ ] graft MAP.md: `Side`, `LineRef`, `DiffCursor`, `diff_cursor`, `show_line`, the pending landing; the
      `body_cursor` row updated. NOTES: the carrier chosen and why; the key table as built.

## Touches (check each, report how)
- **`j`/`k` meaning in a commit diff** changes from scroll to cursor. Other `j`/`k` users (file list, resolver, map,
  journal, stash, picker, commit editor, `.` menu) must not change: their tests green; grep the `diff_key` `j`/`k` arm's
  callers and name them.
- **`operate::cursor_line`** (`operate.rs:379-391`) and every write key reading the changes-pane cursor (`Space`, `s`,
  `d`, `e`): unchanged; their tests green.
- **`show_cursor`'s zeroing** (`ops.rs:318-320`) and `kept_cursor`/`put_cursor_back`: the landing must survive them;
  covered by the landing tests.
- **Captures of any diff body:** grep `docs/shots/**` for commit-diff and changes-pane captures (m2 details, m3, m6);
  every one whose rows change (carrier glyph, first-view scroll) is re-shot; list them in NOTES.
- **Hint bar:** grep `graft-ui/src/graph_screen/hints/` for words that call `j`/`k` "scroll" in a diff context; fix
  them and their tests if found, else report the grep.
- **Mouse:** none new. A click in a commit diff does what it does in the changes pane; say which in NOTES.
- **Journal/undo:** none (no write).

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. A key the keymap
doesn't give you is not free: ask.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~55k. If it compacts, split: **-a** the cursor + `diff_cursor` + capture; **-b** `show_line`.
