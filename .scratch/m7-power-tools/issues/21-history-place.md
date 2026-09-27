# history-place — `L` in blame and `F` on a file: the commits that changed it, the whole file with each one's adds and removes

Status: ready
Blocked by: 18-trace-readers, 20-blame-place (both landed)
Crates: graft, graft-ui.
Source: `docs/design-notes/m7-power-tools.md`: pass 1b "History — the whole file, each change marked"; "Pass 2b" F2,
"The `F` clash — decided", H1, P1, keymap diff (2b) §2.3/§2.5/§2.8/§2.17, "Journey counts (2b)"; field notes :71.
Build with: **Opus** (planner recommends; conductor confirms). The three open design points the scout raised are ruled
below, so nothing left is structural; the rest is judged by tests, snapshots and walks. Fails Fable test 1.
Scout: done — `.scratch/m7-power-tools/scouts/21.md` (main @ df51a8d7). Read it first.
Needs:
- `graft_git::trace::{enclosing_function, line_history, file_history, whole_file, map_to_head, LineRange, TraceCommit,
  TraceEnd, TraceError, SHALLOW_END}` (trace-readers; scout §"Trace readers" has the exact signatures);
- `Place`, `Trail`, `trail_back`, `esc_goes_back`, `place_key`, `open_blame`, `GraphOverlay::Place` (blame-place).

## Files (ADR-0051)
Edit only these. Anything else → commit at the seam, report `blocked: needs <file>`, stop.
- **graft:**
  - `src/update/detail/keys.rs` — the `F` arm (scout §1: none today, `:93`).
  - `src/update/place.rs`, `src/update/place/{keys.rs,trail.rs,write.rs,answer.rs}`; new
    `src/update/place/{history.rs,history_keys.rs,history_answer.rs}`; new `src/update/place/tests/history*.rs`
    (+ the `mod` line in `place/tests.rs`); `src/update/place/tests/{hints.rs,esc_back.rs}` (20i's table).
  - `src/update/keys/bound.rs` — the bound-keys table (scout: `:44`).
  - `src/view/frame/overlays.rs` — `draw_place` (scout: `:77`) dispatches on the variant; `src/view/rows.rs` —
    `place_context` (`:614`) and `back_to_blame` (→ `back to history` when the top is a history place).
  - `src/jobs/detail/history.rs` (new) + the `mod` line in `src/jobs/detail.rs` (or `detail/mod.rs`).
  - The `Command`/`Task`/`Product` definitions and `JobRunner`'s dispatch arm: **the file(s) where `ReadBlame` /
    `Product::Blame` live** (grep; name them in NOTES before editing). If that is `src/update/mod.rs` (16b's) or a
    `src/jobs/` file 09 edits, stop: see Overlaps.
  - `tests/it/history_place.rs` (new) + its `mod` line; the `m7_walk` journey (grep `noted_keys`).
  - `MAP.md`.
- **graft-ui:**
  - new `src/history.rs` (+ `src/history/snapshots/`), the `mod`/`pub use` lines in `src/lib.rs`.
  - `src/detail_pane/line_render.rs` — **visibility only** (`pub(crate)` on helpers history reuses); no behaviour change.
  - `src/graph_screen/hints/lists.rs` (`blame_bar` `:145`, the detail bar, `:106`'s `F focus`), `hints/tests.rs`.
  - `MAP.md`.
- **Read only, never edit:** `src/detail_pane/unified.rs` (`diff_body`), `side_by_side.rs`, `graft-ui/src/blame.rs`,
  `graft/src/update/keys.rs`, `update/resolver/walk.rs`, `update/tests.rs`, every `graft-git` file.

### Overlaps
- **10b** (`context_menu.rs`, `operate.rs`, `update/preview.rs`): none.
- **16b** (`update/remote*`, `update/mod.rs`, graft-ui `confirm.rs`): none, **unless** the product routing arm lives in
  `update/mod.rs`. Then 21 starts after 16b merges (conductor sequences).
- **09** (`context_menu`, `operate`, `jobs`, `view/entries.rs`, graft-ui `context_menu.rs`): **overlap on `jobs`**.
  21's new job file is disjoint; the shared `Command`/`Task`/`Product` enums and `JobRunner` dispatch are not, if 09
  adds arms to them too. Recommend: 21 after 09 merges, or the conductor confirms 09's jobs edits don't touch those
  files. `graft-ui/src/lib.rs` and both `MAP.md`s: one-line / separate-section edits, merge-trivial.

## Rulings (planner, 2026-09-27, after scout 21)

### (a) Rows arrive in one batch, not streamed
- Fact: the runner is buffered (`graft-git/src/run.rs:332`); `file_history` runs git then parses the finished output
  (`trace/log.rs:105`, doc `:117`). The sink is called after git exits.
- Ruling: **keep the design; no reader change.** The note already allows it (H1 `:1141`: "or all at once if the runner
  can't stream"). Streaming would mean changing `graft-git`'s runner, shared by every read, for a monthly task
  (need, `:167`). Not worth it now.
- What he sees while tracing: the title `tracing… · esc stops`; the list and file pane blank until the batch lands. No
  count, since there is none to give. Visibility (ux-principles #1) holds through the title; step counts unchanged
  (`function-history` 2, `file-history` 1; the wait adds no key).
- **`Esc` while tracing → back where he pressed `L`/`F`**, in one key: `trail_back` (it cancels the place's job) → the
  last place's `show_line`, or the blame place under it. No "stopped, empty" screen: there are no rows to keep, and a
  screen showing nothing that needs a second `Esc` breaks "fewest steps that are still enough". **This changes the
  note's landing row `:1267`** ("History, with the rows so far") → "Where he pressed `L`/`F`, same line". Conductor:
  amend the note; tell the user if you judge it his call.
- The trace answer arriving after that `Esc` is dropped (job id no longer the place's): nothing reopens.
- If daily use shows the wait hurts, streaming is a follow-up (conductor: a KNOWN_GAPS row, **unverified**, not broken:
  "history arrives in one batch; `run` is buffered, `run.rs:332`").

### (b) `n`/`N`: a small function of its own, same contract as the resolver's
- Fact: `next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>` (`resolver/walk.rs:175`)
  is typed on conflicts.
- Ruling: **don't reuse or generalise `next_block`.** Widening a resolver function to serve a diff couples two screens
  for ~10 lines. Write `next_change` (Signatures) over the file pane's rows, **with `next_block`'s contract**:
  `from: None` → first/last; `None` past the end.
- On `None`: the status reads `no more changes this way`; the cursor stays. No wrap: `j`/`k` already step commits, and
  "open block" (what the resolver wraps to) doesn't exist here. The words match the resolver's `no more conflicts this
  way` (consistency, ux-principles).
- Moves the file pane so the change's first row sits at the top, under the header.

### (c) The file pane's unified body: history's own, built on the shared row helpers
- Fact: `diff_body(hunks, top, truncated, area, buf, theme, focus: DiffFocus) -> usize` (`detail_pane/unified.rs:85`)
  takes no gutter. It also draws **two** number columns, old and new (graft-ui MAP `detail_pane`), but the ticket's file
  pane has **one**: removed rows have no number.
- Ruling: **`diff_body` stays untouched.** `graft-ui::history` draws its own unified body: `┃` column · new-number
  column · sign column · text. Each row is drawn by the helpers `detail_pane/line_render.rs` already shares between both
  diff bodies (sign, backgrounds, word tint, syntax fg, `␍`, the 16-colour fallback), made `pub(crate)` where needed.
  The body is new; the row look is reused, not copied. If a helper needed isn't in `line_render.rs` but inside
  `diff_body`, stop and report `blocked: needs unified.rs` with the name. Don't copy it.
- `v` (§2.5) on the file pane: `diff_body_side_by_side` unchanged, in the pane's full width. **No `┃` in side by side**:
  its paired rows don't map one-to-one onto a range strip. The title still names the range (`fn refresh_token, lines
  18–29`). Conductor: KNOWN_GAPS row, marked as a scoped limit, not broken.
- Why not add a gutter argument to `diff_body`: it would change the detail pane's renderer and every caller for one
  place, and it still wouldn't give the one-number column.

### Focus-mode captures (scout could not confirm)
- `docs/shots/16-criss-cross-focus{,-esc,-moved}.txt` and `21-redetect-resize-and-focus.txt` are graph-focused Focus
  mode. `F` with the graph focused is unchanged, so **they must not change.**
- If your change alters any of them, find the producing test first: grep the file name in `crates/`, `scripts/`,
  `xtask/`, `TESTING.md`, `docs/log/`. Found → re-shoot with it and name it in NOTES. Not found → **don't hand-edit;
  report `could not confirm: producer of <file>`** and stop that part.

## Decided (the note; not open)
- **`F` follows the focus:**
  - With the graph focused → Focus mode, unchanged (`update/keys.rs:292`, pinned by `update/tests.rs:448`).
  - With the detail pane focused on a file (its row, or any line of its diff, the changes pane included) → that file's
    history. The arm goes in `detail/keys.rs` before the fall-through. Today it falls through to Focus mode
    (`detail/keys.rs:93` → `update/keys.rs:292`); name both lines in NOTES.
  - With the detail pane focused but on no file (`diff_cursor` → `None`, no file row) → falls through as today.
- **`L` in blame:**
  - The range is `enclosing_function` at the blamed rev and line. `Ok(None)` → the cursor's blame block
    (`LineRange::Block`). `Err` → the failure below, **not** the fallback.
  - The title says which: `fn refresh_token, lines 18–29` / `lines 22–24 · the block blame showed`.
  - The list cursor starts on the commit blame showed for that line.
  - `enclosing_function` is I/O: it runs **inside the trace job**, before `line_history`. The answer carries the range
    chosen, for the title.
- **From a working-tree blame:** map the line with `map_to_head`. `None` → status `line <n> isn't committed yet · no
  history`, and nothing opens. (`BlameFrom::WorkingTree.line` is `Option<u32>` since 20a/20b.)
- **`F`:** `file_history` at the commit shown (HEAD for the changes pane). The list cursor starts on the newest row;
  the file pane opens at the line he was on, or the top from a file row.
- **Placing** goes through `show_line`, as 20 does.
- **Layout:** pass 1b's mock-up.
  - The list row: `sha7 subject who age +a −r`, with `↳ was <old path>` under a rename.
  - The file pane: `whole_file` of the selected commit, per ruling (c).
    - `+`/`−`/blank column; removed rows have no line number.
    - The `┃` bracket covers `range_at` (from `L` only; unified only).
    - The header: `<sha7> · Name · age · subject · this file +a −r`.
    - While a whole-file read runs: the header and a dim `reading…`. `j`/`k` cancel it and ask for the new one.
  - Breakpoints: ≥180 → list 56, file the rest; 120–179 → list 40, who as initials; <120 → one pane, `Tab` swaps.
- **States (H1):**
  - Tracing: ruling (a).
  - End rows: `the file starts here (<sha7> added it)` / `git stops tracing here: <reason>` / `1 000 shown · older not
    traced` (cap 1 000).
  - Binary: the file pane shows `binary · <old size> → <new size>`.
  - **Zero rows** (review 18bcd Minor 6). On a depth-1 clone the sink gets nothing and the end is
    `GitStops(SHALLOW_END)`. The list shows the end row alone: `git stops tracing here: this clone is shallow; nothing
    older was fetched`. No row is selected, and no code may index row 0 or assume a newest row (from `L`, the start
    commit has nothing to land on). The file pane shows `no change to show`, dim. `j`/`k`, `n`/`N`, `B` and `⏎` do
    nothing, and the bar doesn't offer them. `Esc` goes back. Same words as the blame place's (`update/place/keys.rs:192`).
  - **A `TraceError` is a failure, drawn** (H1, H9): `Run`, `Parse`, `Shallow`, `TooLarge`. The list area shows
    `couldn't trace: <Display>` on one line, styled as the blame place's failure line (20e). The title drops
    `tracing…`, the file pane is empty, and the bar offers `esc back`. A `whole_file` `DiffError` shows the same way, in
    the file pane only: `couldn't read <path> at <sha7>: <Display>`.
- **Keys (§2.17):**

  | Key | Does |
  |---|---|
  | `j`/`k` | list: older/newer (the file pane follows, keeping the range in view); file pane: scroll |
  | `Tab` | list ↔ file pane |
  | `n`/`N` | next/previous change in the file pane, from either focus: ruling (b) |
  | `B` | blame the file as of the selected commit, at the file pane's line; pushes a place |
  | `⏎` | the graph on the selected commit, its diff on this file; `Esc` comes back |
  | `w` `v` `[` `]` `L` | §2.5, on the file pane (`v`: ruling (c)) |
  | `Esc` | while tracing: ruling (a). Otherwise back (P1) |

  Mouse: a click selects, a double-click is `⏎`, and the wheel scrolls the pane under it.
- **`Esc` on the graph after `⏎`** (20i, Major 1): one predicate, `place::esc_goes_back`, is place-generic. Build
  history on it; don't add a second rung. The open pane is not closed first.
- **Bars:** blame's bar gains `L history of these lines`. The detail bar gains `F history` when the pane is focused on a
  file. The graph bar's `F focus` stays.

## Read first
- `.scratch/m7-power-tools/scouts/21.md` (whole, 1 page).
- The note: pass 1b "History"; "Pass 2b" F2, the `F` decision, H1 (`:1121–1145`), P1 (`:1147–1164`), keymap diff (2b),
  "Journey counts (2b)" and "Where he lands" (`:1245–1269`).
- `docs/design/02-keymap.md` §2.3 (the `F` row, `:222`), §2.5, §2.6 (resolver `n`/`N`, `:296`, for the contract),
  §2.17 (`:467–480`). `docs/design/05-visualization.md` §5.2 (Focus mode, to keep it whole).
- `20i-blame-review-fixes.md` Ruling — Major 1; the 20 respec (`BlameFrom::WorkingTree.line`).
- `crates/graft/MAP.md` "update::place" and `AppState::focus`; `crates/graft-ui/MAP.md` "detail_pane" and "blame".
  MAP.md before source.

## Signatures (exact)
```rust
// graft — update/place.rs, place/history.rs
pub enum Place { Blame(Box<BlameState>), History(Box<HistoryState>) }
pub enum HistoryFrom { Lines { at: Rev, path: PathBuf, line: u32, blamed: Option<Oid> },
                       File { at: Rev, path: PathBuf, line: Option<u32> } }
pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command>;
/// First row of the next run of changed rows. `next_block`'s contract: `from: None` → first/last; `None` past the end.
pub(crate) fn next_change(changed: &[bool], from: Option<usize>, forward: bool) -> Option<usize>;

// graft — jobs. Same id/error shape as `ReadBlame` / `Product::Blame`; one answer per trace (ruling a).
pub enum TraceRequest { Lines { at: Oid, path: PathBuf, line: u32, block: LineRange }, // block: the Ok(None) fallback
                        File { at: Oid, path: PathBuf } }
pub struct TraceAnswer { pub range: Option<LineRange>,  // the range used (Lines only), for the title
                         pub rows: Vec<TraceCommit>, pub end: TraceEnd }
// Command::TraceHistory { id: JobId, request: TraceRequest }  → Product::Trace(Result<TraceAnswer, TraceError>)
// Command::ReadWholeFile { id: JobId, commit: Oid, path: PathBuf, old_path: Option<PathBuf> }
//                                                              → Product::WholeFile(Result<FileDiff, DiffError>)
// Both cancellable; `trail_back` cancels the top place's jobs. `is_write` → false for both.

// graft-ui — src/history.rs
pub struct HistoryView<'a> {
    pub title: &'a str, pub tracing: bool,
    pub list: HistoryList<'a>,             // rows, selection, top, end row
    pub file: FilePane<'a>,
    pub focus: HistoryFocus, pub now: i64,
}
pub enum HistoryList<'a> { Rows { rows: &'a [HistoryRow<'a>], selected: Option<usize>, top: usize,
                                  end: Option<&'a str> },
                           Failed(&'a str) }
pub struct HistoryRow<'a> { pub sha7: &'a str, pub subject: &'a str, pub who: &'a str, pub when: i64,
                            pub added: u32, pub removed: u32, pub was: Option<&'a str> }
pub enum FilePane<'a> { Empty, NoChange, Reading { header: &'a str }, Failed(&'a str),
                        Binary { header: &'a str, old: Option<u64>, new: Option<u64> },
                        Lines { header: &'a str, hunks: &'a [HunkView], top: usize,
                                range: Option<RangeInclusive<u32>>, side_by_side: bool, word_tint: bool } }
pub enum HistoryFocus { List, File }
pub fn draw_history(frame: &mut Frame, area: Rect, view: &HistoryView);
```
Fields may be added (say so in NOTES and MAP.md); none of these may be removed or retyped without the conductor.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Open:**
  - `l_in_blame_opens_history_of_the_function` (red first).
  - `l_with_no_function_uses_the_block_and_says_so`.
  - `l_when_the_function_lookup_fails_draws_the_failure` (an `Err`, not the block).
  - `l_on_an_uncommitted_line_says_so_and_opens_nothing`.
  - `the_list_cursor_starts_on_the_commit_blame_showed`.
  - `f_on_a_file_row_opens_file_history` (red first).
  - `f_on_a_diff_line_scrolls_the_file_pane_to_that_line`.
  - `f_in_the_changes_pane_opens_history_of_that_file`.
  - `f_with_the_graph_focused_is_still_focus_mode`, and `update/tests.rs:448` stays green.
- [ ] **Keys:**
  - `j_steps_older_and_the_file_pane_follows`.
  - `the_range_stays_in_view_as_it_moves`.
  - `n_goes_to_the_next_change_from_either_focus`; `n_past_the_last_change_says_so_and_stays`.
  - `next_change_matches_next_blocks_contract` (unit: `None` from, past the end, both directions, an all-`false` slice).
  - `b_in_history_blames_that_commit_at_that_line`, and `esc_comes_back_to_history`.
  - `enter_shows_the_commit_and_esc_comes_back`.
  - `esc_during_trace_stops_git_and_goes_back` (red first: the trace job's cancel is in the commands, the place is
    popped, he is on the line he pressed `L`/`F` on). Replaces `esc_during_trace_stops_and_keeps_rows`.
  - `a_trace_answer_after_esc_is_dropped`.
  - `esc_from_history_returns_to_blame_same_line`.
  - `esc_on_the_graph_with_the_pane_open_returns_to_history` (red first; 20i). After `⏎` from history, `Tab` to the
    graph with the pane still shown: the bar leads `esc back to history`, and one `Esc` shows history with the same
    row selected and the file pane at the same line.
  - History's rows in 20i's `the_bar_leads_esc_back_to_blame_exactly_where_esc_goes_back` table, with the trail's top a
    history place; the bar reads `back to history`.
  - `tab_swaps_below_120`.
- [ ] **Rows:**
  - `a_rename_row_says_was`.
  - `the_last_row_says_the_file_starts_here`.
  - `the_cap_row_says_how_many`.
  - `a_binary_commit_shows_sizes`.
  - `a_depth_one_clone_shows_the_end_row_alone` (red first: a `--depth 1` clone. No panic, nothing selected, the end
    row's words, the bar without `j`/`k`/`B`/`⏎`).
  - `a_trace_error_is_drawn_as_a_failure` (a `shallow` that is a directory → `Shallow`; the list shows its `Display`;
    `Esc` goes back).
- [ ] **UI snapshots** (`graft-ui/src/history/snapshots/`) at 187×50, 140×40, 100×30: removed rows have no number, the
      `┃` sits on the range, `+`/`−` sit in their column. Plus `history_tracing_140x40` (title only, blank panes) and
      `history_side_by_side_187x50` (no `┃`, title names the range).
- [ ] **Walks** (`m7_walk`): `function-history` `B L` = 2 (start: the diff pane on line 22); `file-history` `F` = 1
      (start: the detail pane on the file's row). `len(keys) <= noted_keys(id)`.
- [ ] **Captures**, each ≤ 200 KB, with a Right line:
  - `docs/shots/m7/history-lines-187x50.txt`: 4 rows, Lina's selected; the file pane at line 18 with `┃` on 18–29; line
    19 `+`, the old line 22 `−` with no number.
  - `history-file-140x40.txt`: after `F`; a `↳ was src/session.rs` row; list 40 wide.
  - `history-100x30.txt`: one pane; the bar shows `tab file`.
- [ ] MAP.md (graft, graft-ui) entries. NOTES: the `F` lines (`detail/keys.rs:93`, `update/keys.rs:292`); the files
      holding `Command`/`Product`; the Focus-mode tests kept green; the `line_render.rs` helpers widened.

## Touches (check each, report how)
- **Focus mode (`F`):** M1's key. Check: `update/tests.rs:448` green. The four Focus captures (ruling above) unchanged,
  shown by `git diff --stat docs/shots/16-criss-cross-focus* docs/shots/21-redetect-resize-and-focus.txt` empty.
- **The detail pane's bar** gains `F history`. Check: the hint-bar tests (`tests/it/hint_bar.rs`,
  `tests/it/pane_bar_space.rs`, `hints/tests.rs`); captures with that bar (`docs/shots/m5-hints-detail-*.txt`,
  `m2-detail-*.txt`) re-shot and named, each with its producing test.
- **Blame's bar** gains `L`. Check: `blame_bar_words_in_order` (`hints/tests.rs:1849`), `place/tests/hints.rs`,
  `tests/it/blame_place.rs`; `graft_ui__blame__tests__blame_*.snap` and `docs/shots/m7/blame-*.txt` re-shot where the bar
  shows.
- **The bound-keys table** (`bound.rs:44`): `F` keeps `focus` for the graph; add history for the pane. Check its test.
- **The diff renderer (graft-ui):** `diff_body` and `diff_body_side_by_side` unedited. Check: `git diff` on
  `unified.rs`/`side_by_side.rs` empty; every existing `detail_pane` snapshot byte-identical.
- **The trail:** `Place::History` in every match blame-place added (`trail.rs`, `keys.rs`, `overlays.rs`, `rows.rs`).
  Check: the build, plus `place/tests/{esc_back,navigation}.rs` green.
- **`SHALLOW_END` words:** the blame place (`update/place/keys.rs:192`) and history say the same. Check by test or grep.
- **Journal:** none (read-only). `is_write` false for both new commands.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~80k: borderline. The conductor may split it up front along the Files line (disjoint crates, runnable concurrently
once the Signatures are fixed):
- **21a (graft-ui):** `draw_history`, the file pane body over `line_render.rs`, the snapshots, `hints/lists.rs`.
- **21b (graft):** state, `F`/`L` routing, jobs, keys, tests, walks, captures. Needs 21a's `draw_history`; walks and
  captures after 21a merges.
If it compacts unsplit, split the remainder the same way.
