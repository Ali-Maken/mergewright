# history-place — `L` in blame and `F` on a file: the commits that changed it, the whole file with each one's adds and removes

Status: ready
Blocked by: 18-trace-readers, 20-blame-place
Blocked by: trace-readers (or trace-readers-b), blame-place
Crates: graft, graft-ui.
Source: `docs/design-notes/m7-power-tools.md`: pass 1b "History — the whole file, each change marked"; "Pass 2b" F2,
"The `F` clash — decided", H1, P1, keymap diff (2b) §2.3/§2.5/§2.8/§2.17, "Journey counts (2b)"; field notes :71.
Build with: **Opus**. It is a second place on blame-place's trail, over decided readers, judged by walks and snapshots.
Scout: `scout` first (ADR-0035): what `F` does in the detail pane today. `Build with` is set by the conductor after the
scout.
Needs:
- `graft_git::trace::{enclosing_function, line_history, file_history, whole_file, map_to_head, LineRange, TraceCommit,
  TraceEnd}` (trace-readers);
- `Place`, `Trail`, `trail_back`, `place_key`, `open_blame`, `GraphOverlay::Place` (blame-place).

## Decided (the note; not open)
- **`F` follows the focus:**
  - With the graph focused → Focus mode, unchanged.
  - With the detail pane focused on a file (its row, or any line of its diff, the changes pane included) → that file's
    history.
  - If the scout finds `F` in the detail pane falls through to Focus mode today, that fall-through ends here. Name the
    line in NOTES.
- **`L` in blame:**
  - The range is `enclosing_function` at the blamed rev and line. If that is `None`, the cursor's blame block
    (`LineRange::Block`).
  - The title says which: `fn refresh_token, lines 18–29` / `lines 22–24 · the block blame showed`.
  - The list cursor starts on the commit blame showed for that line.
- **From a working-tree blame:** map the line with `map_to_head`. `None` → status `line <n> isn't committed yet · no
  history`, and nothing opens.
- **`F`:** `file_history` at the commit shown (HEAD for the changes pane). The list cursor starts on the newest row;
  the file pane opens at the line he was on, or the top from a file row.
- **Layout:** pass 1b's mock-up.
  - The list row: `sha7 subject who age +a −r`, with `↳ was <old path>` under a rename.
  - The file pane: `whole_file` of the selected commit.
    - `+`/`−`/blank column; removed rows have no line number.
    - The `┃` bracket covers `range_at` (from `L` only).
    - The header: `<sha7> · Name · age · subject · this file +a −r`.
  - Breakpoints: ≥180 → list 56, file the rest; 120–179 → list 40, who as initials; <120 → one pane, `Tab` swaps.
- **States (H1):**
  - Title `tracing… · esc stops` while rows arrive.
  - `Esc` during a trace stops git and keeps the rows. A second `Esc` goes back.
  - End rows: `the file starts here (<sha7> added it)` / `git stops tracing here: <reason>` / `1 000 shown · older not
    traced` (cap 1 000).
  - Binary: the file pane shows `binary · <old size> → <new size>`.
- **Keys (§2.17):**

  | Key | Does |
  |---|---|
  | `j`/`k` | list: older/newer (the file pane follows, keeping the range in view); file pane: scroll |
  | `Tab` | list ↔ file pane |
  | `n`/`N` | next/previous change in the file pane, from either focus |
  | `B` | blame the file as of the selected commit, at the file pane's line; pushes a place |
  | `⏎` | the graph on the selected commit, its diff on this file; `Esc` comes back |
  | `w` `v` `[` `]` `L` | §2.5, on the file pane |
  | `Esc` | back (P1) |

  Mouse: a click selects, a double-click is `⏎`, and the wheel scrolls the pane under it.
- **Blame's bar** gains `L history of these lines`.

## Read first
- The note: pass 1b "History"; "Pass 2b" F2, the `F` decision, H1, P1, keymap diff (2b), "Journey counts (2b)".
- `docs/design/02-keymap.md` §2.3 (the `F` row), §2.5, §2.6 (resolver `n`/`N` blocks: same behaviour).
  `docs/design/05-visualization.md` §5.2 (Focus mode, to keep it whole).
- `.scratch/m7-power-tools/scouts/blame-place.md` (the detail routing and the trail) and blame-place's NOTES.
- `crates/graft/MAP.md`: Focus mode (`AppState::focus`), `Place`/`Trail`. `crates/graft-ui/MAP.md`: `detail_pane`
  (`diff_body`, the diff line renderer to reuse for the file pane).

## Scout brief (≤ 1 page; `.scratch/m7-power-tools/scouts/history-place.md`)
1. What `F` does with the detail pane focused today: routed, ignored, or falling through to Focus mode. Give the line.
2. Every test and capture asserting `F` toggles Focus mode, and from which focus.
3. Whether `graft-ui`'s diff body renderer can draw a `FileDiff` with an extra left gutter column (the `┃`) without
   copying it. Which function, which args.
4. How the resolver's `n`/`N` find the next block (to reuse, not copy).
5. Whether `run` streams stdout. That decides whether rows arrive in batches.

## Signatures (exact)
```rust
// graft (app)
pub enum Place { Blame(Box<BlameState>), History(Box<HistoryState>) }   // adds History
pub enum HistoryFrom { Lines { at: Rev, path: PathBuf, line: u32, blamed: Option<Oid> },
                       File { at: Rev, path: PathBuf, line: Option<u32> } }
pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command>;
// Command/Product for the trace (streamed batches) and the whole-file read (builder names them).
// graft-ui
pub fn draw_history(frame: &mut Frame, area: Rect, view: &HistoryView);
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Open:**
  - `l_in_blame_opens_history_of_the_function` (red first).
  - `l_with_no_function_uses_the_block_and_says_so`.
  - `l_on_an_uncommitted_line_says_so_and_opens_nothing`.
  - `the_list_cursor_starts_on_the_commit_blame_showed`.
  - `f_on_a_file_row_opens_file_history` (red first).
  - `f_on_a_diff_line_scrolls_the_file_pane_to_that_line`.
  - `f_in_the_changes_pane_opens_history_of_that_file`.
  - `f_with_the_graph_focused_is_still_focus_mode`.
- [ ] **Keys:**
  - `j_steps_older_and_the_file_pane_follows`.
  - `the_range_stays_in_view_as_it_moves`.
  - `n_goes_to_the_next_change_from_either_focus`.
  - `b_in_history_blames_that_commit_at_that_line`, and `esc_comes_back_to_history`.
  - `enter_shows_the_commit_and_esc_comes_back`.
  - `esc_during_trace_stops_and_keeps_rows`.
  - `esc_from_history_returns_to_blame_same_line`.
  - `tab_swaps_below_120`.
- [ ] **Rows:**
  - `a_rename_row_says_was`.
  - `the_last_row_says_the_file_starts_here`.
  - `the_cap_row_says_how_many`.
  - `a_binary_commit_shows_sizes`.
- [ ] **UI snapshots** at 187×50, 140×40, 100×30: removed rows have no number, the `┃` sits on the range, `+`/`−` sit in
      their column.
- [ ] **Walks** (`m7_walk`): `function_history` `B L` = 2 (start: the diff pane on line 22); `file_history` `F` = 1
      (start: the detail pane on the file's row). `len(keys) <= noted_keys(id)`.
- [ ] **Captures**, each ≤ 200 KB, with a Right line:
  - `docs/shots/m7/history-lines-187x50.txt`: 4 rows, Lina's selected; the file pane at line 18 with `┃` on 18–29; line
    19 `+`, the old line 22 `−` with no number.
  - `history-file-140x40.txt`: after `F`; a `↳ was src/session.rs` row; list 40 wide.
  - `history-100x30.txt`: one pane; the bar shows `tab file`.
- [ ] MAP.md (graft, graft-ui) entries. NOTES: the scout's `F` answer with its line, and the Focus-mode tests kept
      green.

## Touches (check each, report how)
- **Focus mode (`F`):** M1's key, its tests and captures. Check: every test from scout Q2 green. No Focus-mode capture
  changes; the ones that do are named, with why.
- **The detail pane's bar** gains `F history`. Check: the hint-bar tests, and every capture with that bar re-shot and
  named.
- **Blame's bar** gains `L`. Check: blame-place's snapshots and captures re-shot.
- **The diff renderer (graft-ui):** reused, not forked. Check: the existing diff snapshots stay byte-identical.
- **The trail:** `Place::History` in every match blame-place added. Check: the build, plus the trail tests.
- **Journal:** none (read-only).

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~70k. If it compacts, split:
- **history-place-a:** logic, the `F` routing, the jobs, the tests.
- **history-place-b:** `draw_history`, snapshots, walks, captures.

## Note (conductor, 2026-09-26, from the 20 respec)
- history-place must place via `show_line`, as 20 does.
- `BlameFrom::WorkingTree.line` is now `Option<u32>` (20a/20b): read the respec before you build.
