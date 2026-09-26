# blame-draw — the blame place's picture: gutter, trailer, breakpoints, title states, its bar words

Status: ready
Blocked by: 20b-diff-line-cursor (shares graft-ui; sequence only, no signature needed)
Crates: graft-ui.
Build with: **Sonnet**. A pure draw function against a mock-up that fixes every cell, judged by unit tests and
text snapshots; no state, no routing.
Needs: none (`BlameView` is defined here; 20 fills it).

## Read first
- The note `docs/design-notes/m7-power-tools.md`: pass 1b "Blame — the name at the line" (mock-up, trailer, gutter,
  distinctions, breakpoints); "Pass 2b" B1 (title, states, ignore-revs line, "At 24 rows").
- `docs/design/01-view-mockups.md` §1.12 (relative age, heatmap).
- `crates/graft-ui/MAP.md`: the age formatter, `detail_pane` (`DiffView::body_row` as the hit-test model), the hint
  catalogue and contexts (`graph_screen/hints/`).

## Decided (the note; not open)
- **Layout**, top to bottom inside `area`: title (1 row); the ignore-revs line (0–1 row); the body. The bar is the
  frame's bottom row, drawn by the hint catalogue, not here.
- **Body row:** gutter · `│` rule · line number · code. Gutter per pass 1b: `┌ │ └` brackets, `─` for a one-row block;
  sha7, name, age (`7mo`), subject on a block's first row only; the cursor row always prints its name; `▸` replaces
  the bracket on the cursor row.
- **Gutter width:** ≥180 cols → 46; 120–179 → 22, subject dropped; <120 → 10 (bracket, initials, age).
- **Trailer**, cursor row only, 4 cells after the code's last character: `◂ Name · 7 months ago · subject · sha7`.
  Short of room: drop the sha, then cut the subject at a word with `…`, then drop it. If `◂ Name · age` won't fit, no
  trailer (the gutter already names him). Not committed: `◂ not committed yet`. Skipped (`LineMark::Skipped`):
  `◂ (skipped commit) Name …`. Unblamable: `◂ git can't say who`.
- **Title:** `BLAME · <path> · as it is now` / `· as of a1b2c3d` / `· before a1b2c3d`; while a find is active it ends
  `· find "retry" 2 of 5` (`· find "retry" · no match` at 0).
- **States:** reading → the title reads `blaming <path>… · esc cancels`, empty body. Failed → the body's first row
  reads `git blame stopped: <first line> · esc back`.
- **Ignore-revs line** (only when `Some`): `skipping 3 commits listed in .git-blame-ignore-revs · i shows them` /
  `showing every commit · i skips the 3 in .git-blame-ignore-revs`.
- **At 24 rows:** title 1, ignore 0–1, bar 1 (the frame's); the rest is code.
- **Heatmap** (`Space`): per-block age colour, §1.12. Colour only → unverified (rule 5); off, the body is unchanged.
- **Bar words** (hint catalogue; gated by flags 20 sets, so nothing shows until 20 binds the keys):
  - Blame context: `⏎ that commit  ^ before it  / find  n N match  space ages  w whitespace  i ignore list  esc back`.
    `n N match` only while a find is active (2b proposal 7, approved: next/previous match, as the graph's search).
    `i …` only when the repo has the file. No `L` (history-place adds `L history of these lines` after `^`).
  - Graph context with a hidden place: the bar **leads** `esc back to blame`.
  - Pane-focused context on a blamable row: `B blame`, placed before the §2.5 view options.

## Signatures (exact)
```rust
// graft-ui::blame (new)
pub enum BlameRev { Now, AsOf(String), Before(String) }            // sha7
pub struct Who { pub sha7: String, pub name: String, pub author_time: i64, pub subject: String }
pub enum BlockPos { Single, First, Middle, Last }
pub enum RowMark { Plain, NotCommitted, Skipped, Unblamable }
pub struct BlameRowView { pub line_no: u32, pub text: String, pub who: Option<Who>, pub block: BlockPos,
                          pub mark: RowMark }
pub enum BlameBody<'a> { Reading, Failed(&'a str), Lines { rows: &'a [BlameRowView], cursor: usize, top: usize } }
pub struct IgnoreLine { pub count: usize, pub applied: bool }
pub struct FindTitle<'a> { pub query: &'a str, pub at: usize, pub of: usize }   // at is 1-based; of 0 = no match
pub struct BlameView<'a> { pub path: &'a str, pub rev: BlameRev, pub body: BlameBody<'a>,
                           pub ignore: Option<IgnoreLine>, pub find: Option<FindTitle<'a>>, pub heatmap: bool,
                           pub now: i64 }
pub fn draw_blame(frame: &mut Frame, area: Rect, view: &BlameView);
pub fn blame_row_at(area: Rect, view: &BlameView, y: u16) -> Option<usize>;    // mouse: index into rows, or None
// hints: the contexts gain the flags above (names the builder's); catalogue entries in the order given.
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Trailer:** `the_trailer_is_on_the_cursor_row_only` (red first) · `the_trailer_sheds_sha_then_subject` ·
      `a_long_line_drops_the_trailer_and_the_gutter_names_him` · `mid_block_cursor_row_prints_its_name` ·
      `uncommitted_line_trails_not_committed_yet` · `skipped_line_trails_skipped_commit`.
- [ ] **Gutter:** `gutter_is_46_at_187` · `gutter_is_22_without_subject_at_140` · `gutter_is_10_with_initials_at_100` ·
      `one_row_block_is_a_dash` · `block_brackets_open_and_close`.
- [ ] **Title and states:** `title_says_now_as_of_before` · `reading_title_says_esc_cancels` ·
      `failed_body_shows_gits_first_line` · `ignore_line_says_skipping_or_showing` · `no_ignore_line_without_the_file` ·
      `find_title_counts_matches` · `at_24_rows_code_fills_the_rest`.
- [ ] **Mouse:** `blame_row_at_maps_y_to_row` (title and ignore rows → `None`).
- [ ] **Hints:** `blame_bar_words_in_order` · `n_n_only_while_finding` · `i_only_with_the_file` ·
      `graph_bar_leads_esc_back_to_blame` · `pane_bar_offers_b_blame`; `context_lists_match_the_design_note_order`
      and `whole_list` updated, not weakened.
- [ ] **Text snapshots** (TestBackend; hand-built rows matching pass 1b's mock-up, lines 18–27, cursor on 22 with Lina
      `Retry refresh once on 401` `a1b2c3d` 7 months old): 187×50, 140×40, 100×30, and 187×50 with the cursor on the
      long line 27. Each snapshot's header comment carries a "Right:" line.
- [ ] graft-ui MAP.md: the `blame` module's items, the new hint flags.

## Touches (check each, report how)
- **Hint catalogue tests** (`hints/tests.rs` `all_contexts` :24, `context_lists_match_the_design_note_order` :428,
  `whole_list` :812): every context gains its flag defaulting off; existing bars unchanged with the flags off (the
  existing expectations stay byte-identical; say so).
- **The age formatter:** reuse §1.12's; if no long form (`7 months ago`) exists, add it beside the short one and test
  both.
- **No capture changes** here: the flags are off until 20.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~40k.
