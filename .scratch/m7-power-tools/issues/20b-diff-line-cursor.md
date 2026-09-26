# diff-line-cursor — every diff has the line cursor the changes pane has; one call lands graph + file + line

Status: done
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
- [x] **Cursor:** `commit_diff_j_moves_the_line_cursor` (red observed first) · `commit_diff_view_scrolls_only_when_the_cursor_leaves_it`
      · `compare_diff_has_the_line_cursor` · `load_whole_file_keeps_the_commit_diff_cursor_line` ·
      `changes_pane_cursor_is_unchanged`, plus `w_keeps_the_cursor_on_its_line_from_the_cache` — all in
      `crates/graft/src/update/detail/cursor/tests.rs`. Existing changes-pane tests green, named in NOTES.
- [x] **`diff_cursor`:** `diff_cursor_added_line_is_new_side` · `diff_cursor_removed_line_is_old_side_with_the_old_path` ·
      `diff_cursor_heading_is_the_hunks_first_changed_line` · `diff_cursor_file_row_is_the_files_first_changed_line` ·
      `diff_cursor_file_row_before_the_diff_arrives_is_none` · `diff_cursor_binary_file_says_binary` ·
      `diff_cursor_untracked_row_has_its_section` · `diff_cursor_is_none_with_the_graph_focused`, plus
      `diff_cursor_side_by_side_removed_row_with_a_pair_is_new_side` — `cursor/tests.rs`.
- [x] **`show_line`:** `show_line_lands_on_commit_file_and_line` (red observed first, with `advance` stubbed out; commit
      200 of a 30-row window, so the seek is a `Wanted::Find`) · `show_line_puts_the_hunk_heading_on_top` ·
      `show_line_focuses_the_diff_body` · `show_line_working_tree_lands_on_the_uncommitted_row` ·
      `show_line_path_missing_says_so` · `show_line_unreachable_commit_says_so_and_moves_nothing` ·
      `a_key_during_a_landing_drops_it` · `a_second_show_line_replaces_the_first` — all in
      `crates/graft/src/update/detail/landing/tests.rs`, driven through `update`.
- [x] **Capture** `docs/shots/m7/diff-cursor-commit-120x40.txt` — `crates/graft/tests/it/diff_cursor.rs::capture_the_commit_diff_cursor`
      (in-process harness, real git, real `view::view`; not a pty). It asserts `diff_cursor` = New 2 and exactly one row
      carries `▎`, the row `▎+ B`. Colour of the bar: unverified (rule 5). Looks-right: the user's.
- [x] graft MAP.md: `Side`, `LineRef`, `DiffCursor`, `diff_cursor`, `show_line`, `LineLanding`/`LandingStage`/`advance`/
      `LandingWatch`, `AppState::line_landing`, `KeptLine`; `body_cursor`, `set_options`, `load_all`, `scroll_body` rows
      updated. Carrier and key table: NOTES.

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

## NOTES (builder, 2026-09-26)
- **Context compacted** once, after all code, tests and captures were green. Remains after compaction: the ticket
  ticks and NOTES, the commit. Both done; nothing of the build remains.
- **Scout:** the conductor's (`scouts/blame-place.md`); no second one.
- **Carrier:** the changes pane already drew a glyph, `▎` (U+258E, `CURSOR_GLYPH`,
  `graft-ui/src/detail_pane/line_render.rs:61`, drawn at `:145` on a heading and `:192` on a line). Commit and compare
  diffs use the same one. No graft-ui drawing change, doc comments only. Side by side draws no cursor in either pane
  (existing, `side_by_side.rs:277 cursor: false`); the cursor still moves there, unseen, as in the changes pane.
- **Key table as built** (diff body focused, every spec): `j`/`↓` +1, `k`/`↑` −1, PgDn/PgUp ±10, `g`/`G` first/last —
  all move the cursor, the view follows only when it leaves (`keys.rs` `diff_key`, scroll branch removed). `L`, `w`,
  `[`/`]` keep the cursor's side + line and its offset in the view (`DetailState::kept_line`, spent when the diff
  arrives, at once on a cache hit). This also changes the changes pane: its cursor used to go back to row 0 on these
  keys. No new key.
- **Mouse:** unchanged. Wheel over the body scrolls `body_top` without moving the cursor (`mouse.rs:151`); a click in the
  body does nothing — same in both panes.
- **Re-shot captures** — the one change in each is `▎` on row 0 of a commit diff (its first hunk heading), because
  `DetailState::body_cursor()` (`ops.rs:418`) lost its `WorkingTree` guard: `m6/fix-finish-shell-merge-120x40`,
  `m6/fix-landing-after-continue-rebase-{120x40,187x50}`, `m6/fix-shell-merge-abort-and-undo-120x40`,
  `m6/fix-undo-shell-merge-120x40`, `m6/stash-clash-done-120x40`, `m7/landed-after-merge-120x40`.
- **Stale, not regenerated:** `docs/shots/m6/m6-contact-sheet-{1,3,4}.txt` embed copies of those captures without `▎`.
  `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6` would fix them, but I ran it and it also pulls in drift
  from before this ticket (changed captions, other captures), so I reverted it — the conductor's call.
- **Unverified:** diff captures with no test behind them, so not regenerable: `m2-side-by-side`,
  `m2-detail-keys-resize`, `m2-detail-pane-breakpoints`, `m2-diff-widget`, `m5-contact-sheet-2`,
  `m6/prototype-journeys`. Whether they would change: not checked.
- **Touches:**
  - `j`/`k` elsewhere: `diff_key` has one caller, `keys.rs:126` (`detail_key`, Diff focus). File list, resolver, map,
    journal, stash, picker, editor, `.` menu have their own handlers; full nextest green.
  - Write keys: `operate::diff_unit` and `cursor_line` now check `spec == WorkingTree` themselves (they relied on
    `body_cursor()` being `None` elsewhere). Green: `operate.rs` `space_on_a_hunk_heading_stages_the_whole_hunk`,
    `space_on_an_added_line_stages_that_line_alone`, `space_on_a_context_line_stages_the_hunk_around_it`,
    `d_in_the_diff_region_confirms_before_discarding_the_hunk`, `d_on_one_line_names_the_line_and_counts_it`,
    `e_in_the_diff_carries_the_cursors_new_side_line`; `detail/tests.rs`
    `a_write_leaves_the_changes_cursor_on_the_same_file`, `staging_the_file_under_the_cursor_keeps_the_cursor_in_its_own_group`,
    `g_and_capital_g_jump_the_diff_body_to_its_ends`.
  - `show_cursor` zeroing / `kept_cursor`: the landing places the cursor in `settle` after `follow_selection`, so it
    runs after both; covered by the landing tests.
  - Hint bar: `grep -rni scroll crates/graft-ui/src/graph_screen/hints/` → nothing.
  - Journal/undo: none.
- **Design calls to check:**
  - Landing status lines are sticky. `WorkingTree` with a clean tree refuses: `can't show the uncommitted changes:
    there are none` (wording not in the spec).
  - A `Commit` landing while a compare base is pinned: `follow_selection` makes the pane a `Compare`, and the landing
    drops silently at `Selecting`. Not tested; 20 may want the base cleared.
  - `binary`: an untracked file also has no counts, so it is excluded; once the diff arrives its own binary flag decides.
