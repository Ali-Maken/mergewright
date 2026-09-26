# blame-place — `B` opens blame at the line he's on; who, when and why trail that line; `^`, `⏎`, `Esc` back

Status: draft (respecified 2026-09-26 against the scout; was `ready`)
Blocked by: 18-trace-readers (landed), 20a-blame-commit-path, 20b-diff-line-cursor, 20c-blame-draw
Crates: graft.
Build with: **Opus**. App wiring and a modal place on a decided design, judged by walks and tests. The one structural
piece (the places stack) the note decides (2b hard choice 4); the key-routing choices are decided below.
Scout: done, `.scratch/m7-power-tools/scouts/blame-place.md`. No second scout.
Needs:
- `graft_git::blame::{blame, Blame, BlameCommit (with path, 20a), BlameOptions, Rev, LineMark, BlameError}`;
- `graft::{Side, LineRef, DiffCursor, diff_cursor, show_line}` (20b);
- `graft_ui::blame::{BlameView, BlameRowView, BlameBody, …, draw_blame, blame_row_at}` and the hint flags (20c);
- the "session" fixture (18).

## What changed from the first spec
- **Split.** The diff's line cursor and the graph+file+line landing are 20b; the picture and bar words are 20c; the
  commit's own path is 20a. This ticket is the wiring, the trail and the walks.
- **Line cursor** (scout P-a): 20b gives commit and compare diffs the changes pane's cursor. Journey counts unchanged:
  the note's start state ("the diff pane, cursor on line 22") is now reachable; the keys that put the cursor there are
  setup, as for tig/Magit's count.
- **Routing** (P-b, P-d, P-e): a shown place is a modal pane (`modal_owns_keys`), so the graph search no longer takes
  `/`, `n`, `N` from it, and `L`/`w` never reach `detail_key`. `B` in the pane yields to the commit-graph offer while
  the offer is actionable (§2.1's named exception). `F` is untouched here (history-place).
- **`n`/`N`** = next/previous find match (2b proposal 7, **approved by the user 2026-09-26**).
- **B1 table extended:** compare diffs (P-f), a hunk heading, the changes pane's removed lines, and a file with both
  staged and unstaged edits.
- **Landing** (P-g): `⏎` calls 20b's `show_line`; after it, focus is the diff body, and one `Esc` there returns to
  blame (the pane doesn't close while a place is hidden).
- **"Any write"** (Q8, unconfirmed): not hooked in `claim_write`. The trail clears where `update` returns commands, by
  an exhaustive `is_write` match, so every operation launch is caught whatever path it takes.
- The first spec's `u` in `graph_verbs_are_inert_in_blame` stands: `u`/`U` are inert in a place (it writes nothing
  and the graph isn't shown).

## Decided (the note, plus this respec; not open)
- **Opening — `blame_from`** (pure), from 20b's `DiffCursor`:

  | Cursor | Blames | Title | Cursor on |
  |---|---|---|---|
  | commit diff, `New` | `Commit{c, path}` | `as of <c>` | that line |
  | commit diff, `Old` (removed; a deleted file) | `Parent{c, old_path or path}` (first parent) | `before <c>` | the old line |
  | compare, `New` / `Old` | `Commit{head, path}` / `Commit{base, old_path or path}` | `as of <head>` / `as of <base>` | that line |
  | changes pane, `New` | `WorkingTree{path}` | `as it is now` | that line |
  | changes pane, `Old` | `Commit{HEAD, old_path or path}` | `as of <HEAD>` | the old line |
  | changes pane, the file has staged **and** unstaged edits and the side isn't disk- or HEAD-numbered (unstaged `Old`, staged `New`) | `WorkingTree{path, line: None}` | `as it is now` | its first uncommitted line; status `staged and unstaged edits both move this file's lines · on its first uncommitted line` |

  A file row or hunk heading uses 20b's "first changed line". Refusals, status line, no screen:
  `src/new.rs isn't committed yet · no blame` (untracked section, no HEAD, or the job's `BlameError::Untracked`) ·
  `pic.bin is binary · no blame` (`binary`, or `BlameError::Binary`) · `wait for the diff to load · then B` (`at` is
  `None`). A job-side refusal pops the place it opened and shows the same words.
- **`B` in the detail pane:** claimed by `detail_key` on a file row, heading or diff line, both panes — **except**
  while the commit-graph offer is actionable, when `B` goes to the offer as today (keymap §2.1: the banner claims the
  keyboard as a modal pane does). The walk fixtures must not show the offer. Graph focused: `B` unchanged.
- **The place is a modal pane** (§2.1, 2b's keymap diff "places"). It claims keys in `key_event` after the
  boxes/finish/resolver/banner steps (`keys.rs:163-167`) and before the context menu (`:178`), and is in
  `modal_owns_keys` (`state.rs:765-772`), so `search::search_key` (`mod.rs:857`) never sees its keys.
  - Its keys: `j`/`k`, `↑`/`↓`, PgUp/PgDn, `g`/`G` move · `⏎` · `^` · `/` · `n`/`N` · `Space` heatmap · `w` · `i` (only
    with the file) · `Esc`.
  - Live globals: `?`, `q`/`Ctrl+C`, `Ctrl+R`. Everything else is swallowed silently: the graph's verbs (`m r a c o b
    t x`), `u`/`U`/`Ctrl+Z`, `Tab`, `.`, `:`/`Ctrl+P`, `B`, `F`, and `L` (until history-place).
  - Mouse (full width): a click selects a row (`blame_row_at`), a double-click is `⏎`, the wheel scrolls. No event
    reaches the graph (`mouse.rs:36-48` pattern).
- **Keys in blame:**
  - `⏎` on a committed line → `show_line(Commit(line.commit), commit.path, LineRef{New, orig_line})`, `shown=false`.
    On an uncommitted line → `show_line(WorkingTree, path, LineRef{New, final_line})`.
  - `^` → blame `previous` (parent, old path), title `before <c>`, pushed. One job reads the parent's blame **and** `c`'s
    diff of the file; the cursor goes to `came_from(hunks, orig_line)`: the first old-side line of the hunk that made
    the line (a pure add: its old start, at least 1). At a root: `a1b2c3d added this file · nothing before it`. On an
    uncommitted line: `line 22 isn't committed yet · nothing before it`. Neither opens anything.
  - `/` opens the graph's search prompt on the bottom row (its keys and case rule); `⏎` goes to the first match at or
    after the cursor. `n`/`N` next/previous match with the graph search's wrap words; with no find, they do nothing and
    aren't on the bar.
  - `w` / `i` re-run the job (title reading), the cursor kept on the same final line, clamped.
- **`Esc`:**
  - In a shown place, in order: the find prompt → an active find (as the graph's finished search) → a held failure
    (§2.2) → reading: cancel the job (`Command::CancelJob`, the walk's pattern `keys.rs:530-534`) and pop → pop. `Esc`
    in a place never cancels a running write.
  - Place hidden, graph focused: a new last step in `escape_key`, after the reading walk (`keys.rs:530`) → `shown=true`.
  - Place hidden, pane focused: in `detail_key` after Explain and the compare base (`detail/keys.rs:26-41`) and
    **before** the pane close (`:110`) → `shown=true`.
  - Popping the last place restores `origin`: if nothing moved since `B`, just drop the trail; else the recorded graph
    cursor, focus, then `show_line(origin spec, path, at)`.
- **Trail (P1):** `B` starts a new one and records `origin` (graph cursor, focus, spec, path, `at`). `^` pushes. More
  than 16 drops the oldest. After `update` returns commands, any `is_write` command clears the trail.
- **Jobs:** one cancellable job per blame (command/product names the builder's), stale ids dropped (the diff job's
  pattern, `ops.rs:314-352`).
- **Bar:** 20c's flags: blame context (find active, ignore file); graph bar leads `esc back to blame` while hidden;
  pane bar `B blame` on a blamable row, off while the offer is actionable.

## Read first
- The scout file (all of it; ≤ 1 page).
- The note: pass 1b "Blame — the name at the line"; "Pass 2b" B1, P1, keymap diff (2b), "Journey counts (2b)",
  "Where he lands" (2b); "Hard-to-reverse choices (2b)" 4; "Where I'd differ" (2b) 7.
- `docs/design/02-keymap.md` §2.1 (modal panes; the `B`/`X` offer exception), §2.2, §2.5, §2.8 (as the researcher
  applies the 2b diff).
- 20a, 20b, 20c's Signatures sections.
- `crates/graft/MAP.md`: `AppState`, `modal_owns_keys`, `GraphOverlay`, `DetailFocus`, `claim_write`, the key chain,
  jobs/Command/Product. `crates/graft-ui/MAP.md`: `blame`.

## Scope
- In: `blame_from`, `B`, the place and trail, `place_key`, `trail_back`, `is_write`, `came_from`, the jobs,
  `GraphOverlay::Place`, mouse, bar flags, walks, captures, re-shoots.
- Out: history (`L`, `F`: history-place); keymap doc edits (the researcher's).

## Signatures (exact)
```rust
// graft (app)
pub enum Place { Blame(Box<BlameState>) }                     // history-place adds History(Box<HistoryState>)
pub struct Trail { pub places: Vec<Place>, pub shown: bool, pub origin: Option<Origin> }   // AppState::trail
pub enum BlameFrom { Commit { commit: Oid, path: PathBuf, line: u32 },
                     Parent { commit: Oid, path: PathBuf, line: u32 },       // path: in the parent
                     WorkingTree { path: PathBuf, line: Option<u32> } }      // None: first uncommitted line
pub enum NoBlame { Untracked(PathBuf), Binary(PathBuf), DiffNotLoaded }
pub fn blame_from(cursor: &DiffCursor, head: Option<Oid>, staged_and_unstaged: bool) -> Result<BlameFrom, NoBlame>;
pub fn open_blame(state: &mut AppState, from: BlameFrom) -> Vec<Command>;   // B, ^
pub fn place_key(state: &mut AppState, key: KeyEvent) -> Vec<Command>;
pub fn trail_back(state: &mut AppState) -> bool;                             // Esc; false = nothing to go back to
pub fn is_write(command: &Command) -> bool;                                  // exhaustive, no `_` arm
pub fn came_from(hunks: &[Hunk], new_line: u32) -> u32;                      // pure
// GraphOverlay::Place (reads state.trail); blame Command/Product variants (builder names them).
```
`BlameState` and `Origin` fields are the builder's; `Origin` restores graph cursor, focus, spec, file and line exactly.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Open:** `b_on_a_diff_line_opens_blame_at_that_line` (red first) · `b_on_a_removed_line_blames_the_parent_at_the_old_line`
      · `b_on_a_renamed_files_removed_line_uses_the_old_path` · `b_on_a_file_row_lands_on_the_first_changed_line` ·
      `b_on_a_heading_lands_on_the_hunks_first_changed_line` · `b_on_a_compare_blames_head_or_base` ·
      `b_on_the_changes_pane_blames_the_working_tree` · `b_on_a_changes_pane_removed_line_blames_head` ·
      `b_with_staged_and_unstaged_edits_says_the_lines_moved` · `b_on_an_untracked_file_says_so_and_opens_nothing` ·
      `b_on_a_binary_file_says_so_and_opens_nothing` · `a_job_side_refusal_pops_and_says_so` ·
      `b_with_the_graph_focused_is_unchanged` · `b_while_the_commit_graph_offer_is_up_goes_to_the_offer`.
- [ ] **Routing:** `slash_in_blame_finds_in_the_file_not_the_graph` · `graph_verbs_are_inert_in_blame` (`m r a c o u`)
      · `l_and_w_in_blame_never_reach_the_diff_pane` · `a_click_in_blame_never_reaches_the_graph` ·
      `esc_in_blame_never_cancels_a_running_write`.
- [ ] **Navigation:** `caret_opens_the_parent_and_esc_comes_back_to_the_same_line` · `caret_lands_where_the_line_came_from`
      (+ `came_from` unit tests: mid-hunk, pure add at line 1) · `caret_at_the_root_says_nothing_before_it` ·
      `caret_on_an_uncommitted_line_says_so` · `enter_lands_on_the_commit_with_the_diff_at_this_line` (the commit's own
      path after a rename) · `enter_on_an_uncommitted_line_lands_in_the_changes_pane` ·
      `esc_in_the_diff_after_enter_returns_to_blame_same_line` · `esc_on_the_graph_closes_search_before_returning_to_blame`
      · `esc_from_blame_returns_where_b_was_pressed` (graph cursor, focus, file, line; after `⏎`+`Esc` too) ·
      `a_write_clears_the_trail` · `is_write_covers_every_operation_launch` (one per launching `Command` variant) ·
      `the_trail_holds_16`.
- [ ] **Keys:** `find_and_n_move_between_matches` · `n_without_a_find_does_nothing` · `esc_clears_the_find_before_popping`
      · `i_toggles_ignore_revs_and_the_line_says_which` · `i_is_unbound_without_the_file` · `w_toggles_whitespace` ·
      `mouse_double_click_is_enter`.
- [ ] **States:** `blame_reading_title_then_lines` · `esc_while_reading_cancels_the_job` · `blame_failed_shows_gits_words`.
- [ ] **Hints:** `every_bound_action_is_hinted_live_or_is_named_here` green with the new keys (`NEVER_HINTED` unchanged)
      · `pane_bar_hides_b_while_the_offer_is_up` · `graph_bar_leads_esc_back_to_blame_while_hidden`.
- [ ] **Walks** (`m7_walk`, the 2b fence's ids; `len(keys) <= noted_keys(id)`; counts unchanged): `blame_line_to_commit`
      `B ⏎` = 2 (setup puts the diff cursor on line 22; asserts the landing) · `commit_back_to_blame` `Esc` = 1 (focus in
      the diff body) · `blame_who_at_line` `j` = 1 (asserts the trailer names the new row's author) · `blame_line_before`
      `B ^` = 2. The harness reads every `journey-counts` fence; if it reads only the first, extend it and add
      `a_duplicate_id_across_fences_fails`. Each walk asserts the commit-graph offer is not shown.
- [ ] **Captures** (pty, `GRAFT_UPDATE_SHOTS=1` as ticket 08's `check_shot`, ≤ 200 KB, "Right:" line in each test
      comment):
  - `docs/shots/m7/blame-187x50.txt`: line 22 selected; `▸`, and `◂ Lina · 7 months ago · Retry refresh once on 401 ·
    a1b2c3d` on that row only.
  - `blame-140x40.txt`: gutter 22, no subject in the gutter, the trailer whole.
  - `blame-100x30.txt`: gutter 10, the trailer as far as it fits.
  - `blame-long-line-187x50.txt`: cursor on line 27; no trailer, the name in the gutter.
  - `blame-then-commit-120x40.txt`: after `B ⏎`, the graph on Lina's commit, the diff's cursor carrier on line 22 in
    its hunk, the bar leading `esc back to blame`.
  - `blame-find-187x50.txt`: after `/retry⏎ n`, the title ending `· find "retry" 2 of N`.
- [ ] graft MAP.md: `Place`, `Trail`, `BlameFrom`, `NoBlame`, `blame_from`, `open_blame`, `place_key`, `trail_back`,
      `is_write`, `came_from`, `GraphOverlay::Place`, the key-chain rung. NOTES: the key table as built; the re-shot
      captures.

## Touches (check each, report how)
- **Key chain:** `detail_key` (`B` claim, the new Esc rung before `:110`), `commit_graph_key` (`keys.rs:243`; offer
  precedence), `search_key` (`mod.rs:857`; via `modal_owns_keys`), `escape_key` (new last rung after `:530`). Check:
  every existing key test green; the routing tests above.
- **`GraphOverlay` sites** (scout Q4): `graph_overlay()` precedence (Place after Finish and Resolver), the frame arm
  (`view/frame.rs:131-152`), `overlay_context` (`view/rows.rs:542-555`), banner hide (`banner.rs:115`), undo
  (`undo.rs:222`), boxes (`view/frame/boxes.rs:271`), mouse (`mouse.rs:36-48`). Check: build + the draw and hit tests.
- **`modal_owns_bottom_row`:** decide whether the place's bottom row is the frame's bar (it should be, with status
  words); every rung that asks it (MAP: `graph_view`, `footer_notice`, `draw_status_or_hints`, `modal_blanked`) checked.
- **Hint-bar tests:** `graft-ui/src/graph_screen/hints/tests.rs`, `update/keys/tab_words_tests.rs`, `view/tests.rs`.
- **Captures showing the pane's bar** gain `B blame`: the scout named `docs/shots/m6/fix-three-panes-{180,187,200}x50.txt`,
  `fix-c-in-pane-finishes-187x50.txt`, `stash-clash-take-esc-pane-120x40.txt` and could not confirm the full set.
  Grep `docs/shots/**` for the pane bar's leading words; re-shoot every hit; list them in NOTES.
- **Other `j`/`k` users** (file list, resolver, map, journal, stash, picker): unchanged; the place claims keys only
  while shown. Their tests green.
- **Writes:** every `Command` variant classified in `is_write`; the list in NOTES. Journal/undo: blame writes nothing.
- **Keymap doc** (2b diff, applied by the researcher): check §2.1 places, §2.2 `Esc`, §2.5 `B`, §2.8 rows against what
  is built; report a mismatch, don't edit.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. A key the keymap
doesn't give you is not free: ask.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~75k. If it compacts, split: **-a** open, routing, trail, keys, jobs and their tests; **-b** walks, captures,
re-shoots, hint tests.
