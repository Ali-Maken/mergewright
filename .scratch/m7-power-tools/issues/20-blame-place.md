# blame-place — `B` opens blame at the line he's on; who, when and why trail that line; `^`, `⏎`, `Esc` back

Status: ready
Blocked by: 18-trace-readers
Blocked by: trace-readers (or trace-readers-a)
Crates: graft, graft-ui.
Source: `docs/design-notes/m7-power-tools.md`: pass 1b "Blame — the name at the line"; "Pass 2b" B1, P1, the keymap
diff (2b), "Journey counts (2b)", "Where he lands" (2b); field notes :70.
Build with: **Opus**. It is app wiring and a new pane on a decided design, judged by walks and snapshots. The places
stack is the one structural piece, and the note decides it (hard choice 4).
Scout: `scout` first (ADR-0035, brownfield: the detail pane's key routing and `Esc` chain). `Build with` is set by the
conductor after the scout.
Needs:
- `graft_git::blame::{blame, Blame, BlameOptions, Rev, LineMark, BlameError}` (trace-readers);
- the "session" fixture shape (trace-readers).

## Decided (the note; not open)
- **`B` from the detail pane** (diff pane or file list; a commit's or the changes pane's). The table in B1:

  | Pressed on | Blames | Cursor on |
  |---|---|---|
  | an added or context line | as of the commit | that line |
  | a removed line | the parent; title `before <sha7>` | the old line |
  | a file row | as of the commit | its first changed line |
  | the changes pane | `Rev::WorkingTree` | that line |

  - The refusals go on the status line, with no screen: `<path> isn't committed yet · no blame` and `<path> is binary ·
    no blame`.
  - `B` with the graph focused is unchanged: whatever §2.3 says today, or unbound.
- **Layout:** pass 1b's mock-up, exactly.
  - The gutter: `┌ │ └` brackets, one-row blocks `─`, hash/name/age/subject on the block's first row, and the cursor
    row always names its author.
  - Then the `│` rule and the code.
  - The trailer on the cursor row only: `◂ Name · age · subject · sha7`, 4 cells after the code's last character.
    - Short of room, shed in order: the sha, then cut the subject with `…`, then drop it.
    - If even `◂ Name · age` won't fit, the gutter carries the name (it already does on the cursor row).
    - An uncommitted line: `◂ not committed yet`. A skipped line: `◂ (skipped commit) Name …`.
  - Breakpoints (gutter width): ≥180 → 46; 120–179 → 22, with the subject dropped from the gutter; <120 → 10 (bracket,
    initials, age).
  - Age is relative (`7mo`), as DESIGN §1.12.
- **Title and states (B1):**
  - `BLAME · <path> · as it is now` / `as of <sha7>` / `before <sha7>`.
  - Reading: `blaming <path>… · esc cancels`; `Esc` kills git.
  - Failed: the body reads `git blame stopped: <first line> · esc back`.
  - The ignore-revs line shows only when the repo has the file: `skipping N commits listed in .git-blame-ignore-revs ·
    i shows them` / `showing every commit · i skips the N in .git-blame-ignore-revs`.
- **Keys** (keymap diff 2b, §2.8):

  | Key | Does |
  |---|---|
  | `j`/`k`, `↑`/`↓`, PgUp/PgDn, `g`/`G` | move |
  | `⏎` | the graph at the line's commit, detail pane on it, the file selected, the diff scrolled to the hunk holding this line; bar leads `esc back to blame` |
  | `^` | blame `previous` (the parent and old path); cursor on the first old-side line of the hunk that made this line; title `before <sha7>`; at a root: `<sha7> added this file · nothing before it` |
  | `/`, `n`/`N` | find in the file; next/previous match |
  | `Space` | age heatmap (as §2.8) |
  | `w` | `-w` toggle (on by default) |
  | `i` | ignore-revs toggle, bound only when the file exists |
  | `L` | history-place's; this ticket leaves it unbound, and the bar doesn't show it until history-place |
  | `Esc` | back (P1) |

  Mouse: a click selects, a double-click is `⏎`, the wheel scrolls.
- **Places and the trail (P1), built here** (history-place adds `Place::History`):
  - `Trail{places, shown, origin}` on `AppState`.
  - `B` starts a new trail and records `origin`: pane, file, line, graph cursor.
  - `^` pushes. `Esc` pops; the last `Esc` restores `origin` exactly.
  - `⏎` sets `shown=false` and moves the graph.
  - On the graph, `Esc` with nothing else open (menu, box, prompt, search, Focus, Explain) sets `shown=true`.
  - Any write (an operation starts), or a new `B`/`F`, clears the trail. More than 16 places drops the oldest.
  - The graph's verbs are inert while a place is shown.

## Read first
- The note: pass 1b "Blame"; "Pass 2b" B1, P1, keymap diff (2b), "Journey counts (2b)", "Interfaces" (graft app and
  graft-ui); "Hard-to-reverse choices (2b)" 4.
- `docs/design/02-keymap.md` §2.1 (modal panes), §2.2 (`Esc`), §2.5, §2.8.
- `docs/design/01-view-mockups.md` §1.12. `docs/research/ux-principles.md` §3.1, §3.6, checklist 16–17.
- `crates/graft/MAP.md`: `AppState`, `DetailFocus`, `GraphOverlay`, the key routing (`detail_key` or its equivalent),
  jobs/Command/Product.
- `crates/graft-ui/MAP.md`: `detail_pane`, the overlay draw entry.

## Scout brief (code, not intent; ≤ 1 page; `.scratch/m7-power-tools/scouts/blame-place.md`)
1. Where a key pressed with the detail pane focused is routed. What `B`, `F`, `L`, `^`, `/`, `n` do there today, with
   lines. Does any fall through to the graph?
2. How the diff pane knows the line under its cursor: the new/old line number, removed or context, and the commit and
   path it belongs to, for both a commit's diff and the changes pane.
3. The `Esc` chain on the graph: the order in which Esc closes menu, box, prompt, search, Focus and Explain, with lines.
   Where a new "back to place" step slots in last.
4. `GraphOverlay` variants and where each is matched (draw, keys, hint bar, mouse), with lines. What a new variant
   must add.
5. How the app moves the graph cursor to a commit **and** selects a file **and** scrolls its diff to a line. Is there
   a function, or is it three steps?
6. How a cancellable job is started and its product lands (one example: the diff job), and how `Esc` cancels one.
7. The hint-bar tests (the ones asserting every bar's words) and the captures that show the detail pane's bar; the
   files.
8. What "any write" is in code: the one place every operation launch passes through (for clearing the trail).

## Signatures (exact)
```rust
// graft (app)
pub enum Place { Blame(Box<BlameState>) }          // history-place adds History(Box<HistoryState>)
pub struct Trail { pub places: Vec<Place>, pub shown: bool, pub origin: Option<Origin> }   // AppState::trail
pub enum BlameFrom { Commit { commit: Oid, path: PathBuf, line: u32 },
                     Parent { commit: Oid, path: PathBuf, line: u32 },
                     WorkingTree { path: PathBuf, line: u32 } }
pub fn open_blame(state: &mut AppState, from: BlameFrom) -> Vec<Command>;
pub fn place_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
pub fn trail_back(state: &mut AppState) -> bool;   // Esc; false = nothing to go back to
// GraphOverlay::Place (reads state.trail); Command/Product variants for the blame job (builder names them).
// graft-ui
pub fn draw_blame(frame: &mut Frame, area: Rect, view: &BlameView);
```
`BlameState`, `Origin` and `BlameView` fields are the builder's. `Origin` must hold enough to restore the pane, file,
line and graph cursor exactly.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Open:**
  - `b_on_a_diff_line_opens_blame_at_that_line` (red first).
  - `b_on_a_removed_line_blames_the_parent_at_the_old_line`.
  - `b_on_a_file_row_lands_on_the_first_changed_line`.
  - `b_on_the_changes_pane_blames_the_working_tree`.
  - `b_on_an_untracked_file_says_so_and_opens_nothing`.
  - `b_on_a_binary_file_says_so_and_opens_nothing`.
  - `b_with_the_graph_focused_is_unchanged`.
- [ ] **Trailer** (graft-ui unit tests on `draw_blame`):
  - `the_trailer_is_on_the_cursor_row_only`.
  - `the_trailer_sheds_sha_then_subject`.
  - `a_long_line_moves_the_name_to_the_gutter`.
  - `mid_block_cursor_row_prints_its_name`.
  - `uncommitted_line_trails_not_committed_yet`.
- [ ] **Navigation:**
  - `caret_opens_the_parent_and_esc_comes_back_to_the_same_line`.
  - `caret_lands_where_the_line_came_from`.
  - `caret_at_the_root_says_nothing_before_it`.
  - `enter_lands_on_the_commit_with_the_diff_at_this_line`.
  - `esc_after_enter_returns_to_blame_same_line`.
  - `esc_on_the_graph_closes_search_before_returning_to_blame`.
  - `esc_from_blame_returns_where_b_was_pressed` (pane, file, line, graph cursor).
  - `a_write_clears_the_trail`.
  - `the_trail_holds_16`.
- [ ] **Keys:**
  - `find_and_n_move_between_matches`.
  - `i_toggles_ignore_revs_and_the_line_says_which`.
  - `i_is_unbound_without_the_file`.
  - `w_toggles_whitespace`.
  - `graph_verbs_are_inert_in_blame` (`m`, `r`, `a`, `c`, `u` do nothing).
  - `mouse_double_click_is_enter`.
- [ ] **States:**
  - `blame_reading_title_then_lines`.
  - `esc_while_reading_cancels_the_job`.
  - `blame_failed_shows_gits_words`.
- [ ] **UI snapshots:** the fixture's `src/auth/session.rs`, cursor on line 22, at 187×50, 140×40 and 100×30. The rows
      match pass 1b's mock-up and the breakpoints above.
- [ ] **Walks** (`m7_walk`, the fence's ids; `len(keys) <= noted_keys(id)`):
  - `blame_line_to_commit`: `B ⏎` = 2;
  - `commit_back_to_blame`: `Esc` = 1;
  - `blame_who_at_line`: `j` = 1, then asserts the trailer names the new row's author;
  - `blame_line_before`: `B ^` = 2.

  The walk harness also reads the 2b fence (2a's `parse_counts` extension reads every `journey-counts` fence; if it
  reads only the first, extend it here and add `a_duplicate_id_across_fences_fails`).
- [ ] **Captures** (pty, `GRAFT_UPDATE_SHOTS=1` as ticket 08's `check_shot`), each ≤ 200 KB. Each has a "Right:" line in
      its test comment, which the acceptance ticket copies:
  - `docs/shots/m7/blame-187x50.txt`: line 22 selected; `▸`, and `◂ Lina · 7 months ago · Retry refresh once on 401 ·
    a1b2c3d` on that row only.
  - `blame-140x40.txt`: gutter 22, the subject absent from the gutter, the trailer whole.
  - `blame-100x30.txt`: gutter 10, the trailer as far as it fits.
  - `blame-long-line-187x50.txt`: cursor on line 27; the trailer gone, the name in the gutter.
  - `blame-then-commit-120x40.txt`: after `B ⏎`, the graph on Lina's commit, the diff at line 22's hunk, the bar leading
    `esc back to blame`.
- [ ] graft MAP.md: `Place`, `Trail`, `open_blame`, `place_key`, `trail_back`, `GraphOverlay::Place`. graft-ui MAP.md:
      `draw_blame`, `BlameView`. NOTES: the scout answers used, and the key table as built.

## Touches (check each, report how)
- **Detail-pane keys:** `B` is new there. List what it did before (scout Q1). The hint-bar tests and every capture
  showing the detail pane's bar gain `B blame` → re-shoot or update, named in NOTES.
- **The graph's `Esc` chain:** a new last step. Check: the existing `Esc` tests stay green; the new test above.
- **`GraphOverlay` matches:** every site from scout Q4. Check: the build, plus the draw and hit-test tests.
- **Mouse hit-testing:** blame's area must not pass clicks to the graph. Check: a test clicking in blame.
- **Journal/undo:** blame writes nothing. The trail clears on writes (the test above).
- **Keymap doc:** the conductor applies the 2b diff; this ticket binds only what it names.
- **Other screens drawing the detail bar:** the resolver, the changes pane. Check: their bar snapshots.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. A key the keymap
doesn't give you is not free: ask.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~75k. If it compacts, split:
- **blame-place-a:** logic: open, the trail, keys, jobs, and the tests with a stub view.
- **blame-place-b:** `draw_blame`, the breakpoints, snapshots, walks, captures.
