# 20m landing-in-a-partly-staged-file — `show_line(WorkingTree, …)` lands on the disk line; a folded path is found; four stale diff captures

Status: done (1ab57a3b A+B, 821a621e C)
Milestone: M7 (m7-power-tools)
Blocked by: **20k** (edits `update/detail/landing.rs` and `update/place/trail.rs`, both here) and **20l** (graft crate;
may regenerate diff-focused captures this ticket re-checks). Place after 20l, **before 34** (34 moves `jobs/detail.rs`,
whose lines this spec cites; 34's `Blocked by` now names 20m).
Crates: graft only (`update/detail/landing.rs`, `update/detail/` tests, `update/place/keys.rs`, `update/place/trail.rs`
or wherever `Origin` lives, their tests, graft `MAP.md`) + `docs/shots/` (part C). No graft-ui, no graft-git. Needing
another crate → commit at the seam, report `blocked: needs <crate>`, stop.
Build with: **Opus**. A signature change inside one crate with two callers, and a line mapping judged by an oracle
test, not by reading the diff. Not Fable: no cross-crate interface, and the oracle decides correctness.
Needs: none.
Scout: none. The review names every line (`docs/reviews/m7-20ab-23b.md` M1, m2, m3, at `ffef24f3`). Lines have moved
since (20e–20l): re-read each site before editing.
Source: review m7-20ab-23b Major M1 (gap M7/20b-r-M1), Minor m2 (M7/20b-r-m2), Minor m3 (M7/20b-r-m3). Planner,
2026-09-27.

## Read first
- `docs/reviews/m7-20ab-23b.md` §Major M1, §Minor m2, §Minor m3.
- graft `MAP.md` `update::detail` rows: `Side`, `LineRef`, `DiffCursor`, `show_line`, `LineLanding` (`:592-597`), and
  the `update::place` `Origin` row (`:576`). graft-git `MAP.md` §diff (`Hunk`, `DiffLine`, `FileDiff`, `:120-129`).
- `.scratch/m7-power-tools/issues/20-blame-place.md` §Decided, "Keys in blame" `⏎` and "`Esc`" (the origin restore).
- Then the sites: `landing.rs` `select` (review `:212`), `jobs/detail.rs` changes-pane row order (review `:380-393`),
  `place/keys.rs` blame `⏎` (review `:226`, `:245-252`), the origin restore's `show_line` call.

## A — M1: a disk line in a partly staged file (behaviour; rule 7: **red first, or it closes**)

**The claim, by lines, not run:** `select` takes the first visible row for the path; the changes pane lists
Unmerged → Staged → Unstaged, so a partly staged file matches **Staged** (index vs HEAD, `New` = index numbering). Blame
`⏎` on an uncommitted line sends `LineRef { New, final_line }`, a **disk** number. So the cursor lands by disk number in
an index-numbered diff.

**Second case, same cause (a question, settle it with the same test style):** the origin restore calls
`show_line(origin spec, path, at)` (20's §`Esc`) with no section. An origin in the **Unstaged** section of a partly
staged file would come back on the Staged row, its line read in the wrong numbering.

**Red first, both.** If a test is green before the fix, that half closes: quote in NOTES the line that already
prevents it, and change nothing for that half.

### Ruling: the line says which file it counts in; a disk line is placed by where it lives
The reviewer's "always prefer the Unstaged row" is **rejected**: a line whose uncommitted change is only **staged** is
not in the Unstaged diff, so the cursor would land on the nearest unrelated row. Blame on disk can't tell staged from
unstaged (both are "not committed yet").

`show_line` gains the section the line counts in:
- `section: None`, spec `WorkingTree`: the line counts **on disk**. Placement:
  1. The path has one row only (Staged only: index = disk; Unstaged only: `New` = disk): that row, `at` as given.
  2. The path has Staged **and** Unstaged rows: select Unstaged, wait for its diff, then `disk_line_in(&diff, L)`:
     - `Unstaged(L)` (an added line on the Unstaged diff's `New` side) → place there.
     - `Index(L')` → select the Staged row, place at `LineRef { New, L' }`.
  3. An Unmerged row: as today (first row). Out of scope; say so in NOTES.
- `section: Some(s)`, spec `WorkingTree`: land on the path's row in `s`, `at` in that row's own numbering. No row in
  `s` (the tree changed from outside since) → the path's first row, `at: None` (its first change). No new words.
- `Commit` / `Compare`: `section` must be `None` (`debug_assert!`); behaviour unchanged.
- `at: None` with `section: None` on a partly staged file: the Unstaged row's first change.

`disk_line_in` (pure), for the Unstaged diff (index → disk):
- a `DiffLine` of kind `Added` with `new_no == L` → `Unstaged(L)`;
- a `Context` line with `new_no == L` → `Index(old_no)`;
- outside every hunk → `Index(L − Σ(new_lines − old_lines))` over the hunks that end above `L`. Watch zero-length
  ranges (a pure deletion's `new_lines == 0`).
- `truncated` diff and `L` below the last shown hunk → `Unstaged(L)` (placed by `line_row`'s nearest, as today); NOTES
  says so. `ignore_whitespace` is out of scope (whitespace-only edits don't change line counts).

Callers: blame's `⏎` passes `None` (disk, for `WorkingTree` and `Commit` alike). The origin restore passes the section
recorded at `B` (`Origin` gains it, from `DiffCursor::section`). 21 (history place) will pass `None` with `Commit`.

### Signatures (graft)
```rust
// update/detail/landing.rs — changed: one parameter added, pub path unchanged.
pub fn show_line(
    state: &mut AppState,
    spec: DetailSpec,
    path: &Path,
    at: Option<LineRef>,
    section: Option<WorkingTreeSection>, // WorkingTree only: which file `at` counts in; None = the file on disk
) -> Vec<Command>;

// update/detail/landing.rs (or cursor.rs beside line_row) — new, pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiskLine {
    /// An uncommitted, unstaged line: this `New` number on the Unstaged diff.
    Unstaged(u32),
    /// Unchanged since the index: this `New` number on the Staged diff.
    Index(u32),
}
pub(crate) fn disk_line_in(unstaged: &FileDiff, disk_line: u32) -> DiskLine;

// LineLanding: gains `section: Option<WorkingTreeSection>`. Reuse Selecting → Placing for the re-select onto the
// Staged row; add a LandingStage variant only if the lines show one is needed, and name it in MAP.
// Origin: gains `section: Option<WorkingTreeSection>`.
```
`WorkingTreeSection` is graft-ui's (`graft-ui/MAP.md:61`), already used by `DiffCursor`.

### Tests (red first; name each at tick time)
Fixture (real git, the harness `enter_on_an_uncommitted_line_lands_in_the_changes_pane` uses): `f.txt` with 10 lines
`l1`…`l10` committed. **Staged:** `l8` → `S8`. **Then unstaged:** insert `U` above `l3`, change `l6` → `U6`. On disk:
`U` is line 3, `U6` line 7, `S8` line 9.
**Oracle, independent of the code:** the cursor row's text equals line `L` of `f.txt` read from disk with
`std::fs::read_to_string`, and its kind is `Added`.
1. `enter_on_an_unstaged_line_of_a_partly_staged_file_lands_on_it`: `B` on the file, cursor to disk line 3, `⏎`.
   Unstaged row selected; the oracle holds (`U`). Repeat for line 7 (`U6`).
2. `enter_on_a_staged_line_of_a_partly_staged_file_lands_on_it`: disk line 9. Staged row selected; the oracle holds
   (`S8`).
3. `esc_restores_an_origin_in_the_unstaged_section`: cursor on `U6` in the Unstaged section, `B`, `⏎` on a committed
   line (lands on a commit), `Esc` back to blame, `Esc` out. Unstaged row selected, cursor on `U6`. (The second case.)
4. `disk_line_in_*` unit tests: added, context, above/below/between hunks, a pure-deletion hunk, a pure-add hunk at
   line 1, truncated.
5. Stay green, named at tick time: every existing `landing` test (8 at 20b), `enter_lands_on_the_commit_with_the_diff_at_this_line`,
   `enter_on_an_uncommitted_line_lands_in_the_changes_pane`, every `esc_back` test.

### Capture
`docs/shots/m7/blame-enter-partly-staged-120x40.txt` through its own `check_shot` test: the changes pane after test 2's
`⏎`, the Staged row selected, exactly one `▎`, on `▎+ S8`. The test asserts both.

## B — m2: a path inside a folded directory (latent; red first through `show_line` directly)
`select` searches only visible rows, so a path under a collapsed directory misses and sets the false sticky
`<path> isn't among <c>'s files`. **Ruling:** search all rows for the entry; if it is hidden, unfold its collapsed
ancestors (the pane's existing unfold, `toggle_dir` or its successor), then select. The unfold is the landing's own
move, like its select. "isn't among" stays for a path truly absent.
- `show_line_unfolds_a_folded_directory_to_reach_its_path`: commit pane open, fold `src/`, `show_line(Commit(c),
  "src/a.rs", Some(LineRef{New,1}), None)`. Red today on the "isn't among" status. After: `src/` open, `src/a.rs`
  selected, no status.
- `show_line_on_a_path_not_in_the_commit_still_says_so`: stays green.

## C — m3: four diff captures that predate `▎` (captions/pictures pair; docs only)
Files: `docs/shots/m2-detail-keys-resize.txt`, `m2-detail-pane-breakpoints.txt`, `m2-diff-widget.txt`,
`m5-contact-sheet-2.txt`. **Re-check the list, don't trust it:** grep `docs/shots/` outside `m7/` for files with a
`@@` unified-diff body and no `▎` (side-by-side exempt: it draws no cursor), and list the result in NOTES.
For each file found:
1. Grep `crates/` and `scripts/` for its name. **A writer exists** → regenerate through it, name the test.
2. **No writer, a loose capture** → insert as its first line, verbatim:
   `# Historical: drawn at <milestone>, not regenerated. Since M7 (20b) a unified diff draws ▎ on its cursor row; today's drawing: docs/shots/m7/diff-cursor-commit-120x40.txt`
3. **A contact sheet** (`m5-contact-sheet-*`) → **not edited**: a milestone's sheet is that milestone's record, and it
   is script-generated (`do not hand-edit`, its header). NOTES says so; the conductor closes it on this ruling.
Then grep `TESTING.md` and `docs/` for any citation presenting a marked file as today's drawing; list them, edit none
(a finding for the conductor).

## Acceptance (name the verifying test or command at tick time)
- [x] A: tests 1–3 seen **red** before, green after (quote each red); or, per half, green before with the preventing
      line quoted and nothing changed. Tests 4–5 green, named. — `blame_partly_staged::enter_on_an_unstaged_line_of_a_partly_staged_file_lands_on_it`,
      `…::enter_on_a_staged_line_of_a_partly_staged_file_lands_on_it`, `…::esc_restores_an_origin_in_the_unstaged_section`
      (reds in NOTES); `landing::tests::disk::disk_line_in_*` (8); stay-green list in NOTES.
- [x] A: capture written; its test asserts one `▎` on `+ S8`. — `enter_on_a_staged_line_of_a_partly_staged_file_lands_on_it`
      writes/checks `docs/shots/m7/blame-enter-partly-staged-120x40.txt`.
- [x] B: the red test seen red then green; the absent-path test green. — `landing::tests::select::show_line_unfolds_a_folded_directory_to_reach_its_path`
      (red in NOTES); `landing::tests::show_line_path_missing_says_so` green (the spec's `show_line_on_a_path_not_in_the_commit_still_says_so`
      is this existing test; not duplicated).
- [x] C: the grep list in NOTES; each file's outcome (regenerated by `<test>` / marked / record); `check_shot` green. —
      marked 3, record 1, exempt 1; full `cargo nextest run` green (every `check_shot`).
- [x] Journey `blame-line-to-commit` step count unchanged (`B`, move, `⏎`): the walk's count test green, named. —
      `m7_walk::blame::blame_line_to_commit` green.
- [x] MAP.md graft (edited, read back): `show_line` row (new parameter, the disk rule, the fold rule), `LineLanding`
      (`section`), `Origin` (`section`), `disk_line_in`.
- [x] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
      `cargo test --doc`. — all pass; nextest 4220/4220 (3 skipped).

## Touches (check each; NOTES says how)
- **Every `show_line` caller** (grep + compiler): blame `⏎`, the origin restore, tests. Each passes `None` or the
  recorded section; say which.
- **`Origin` constructions** (grep): each records `DiffCursor::section`.
- **`LandingWatch`**: the re-select onto the Staged row and the unfold happen inside `advance`, not in a key event;
  confirm by test 2 and B's test that neither drops the landing.
- **`files_arrived` / `KeptCursor`** on the changes pane: a working-tree re-read mid-landing keeps behaving (the
  existing landing tests).
- **20b's `diff_cursor`, 20's `blame_from`:** read-only here; unchanged.
- **Keymap, journal/undo, graft-ui, graft-git:** none. The grep for each in NOTES.
- **21 (queued)**: calls `show_line` with `Commit` and `None`; its spec says "via `show_line`", no signature quoted.
- **34 (queued)**: moves `jobs/detail.rs`; this ticket edits it not at all (`git diff --stat` shows it).
- **For the conductor:** closes M7/20b-r-M1, M7/20b-r-m2, M7/20b-r-m3 (the sheet by ruling).

## Size
~50k. One context. Commit after A with gates green, then B, then C.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## NOTES (builder, 2026-09-27)
No compaction. No scout: the ticket named every site; I read them directly (landing.rs, place/keys.rs `enter`,
place/trail.rs `leave`, place.rs `Origin`/`b_key`, graft-ui `FileListView`).

**Harness.** Tests 1–3 are real git in `crates/graft/tests/it/blame_partly_staged.rs` (the `App` harness of
`blame_place.rs`, not `navigation.rs`'s synthetic state: the oracle needs a file on disk). The fixture's commit is
dated `CORPUS_NOW − 1 day` so the capture is byte-stable.

**Reds, before the fix (all three, both halves of A):**
- 1: `line 3: the Unstaged row` — `left: Some(Staged)`, `right: Some(Unstaged)`.
- 2: `the cursor on S8` — `left: (Context, "l9")`, `right: (Added, "S8")` (disk 9 read as index 9).
- 3: `back on the Unstaged row` — `left: Some(Staged)`, `right: Some(Unstaged)`. The second case was real.
- B: with the fix's row search put back to `files.visible()` (the pre-fix search), `left: Sticky { reason: "src/a.rs
  isn't among 0101010's files" }`, `right: Ready`. Restored; green.

**Build.** `select` searches all rows with their section (`FileListView::section_at`); picks per the ruling; unfolds
collapsed ancestors (`unfold_to`, via `toggle_dir`) before `select_file_row`. The disk rule's second step is
`index_line`, run at `Placing` before `place`: on the Unstaged row of a Staged+Unstaged path with its diff in,
`disk_line_in(diff_raw, L)`; `Index(L')` sets `section: Some(Staged)`, `at: New L'`, stage `Selecting` (no new
`LandingStage`). Unmerged rows: out of scope, first row as before (a path with an Unmerged row skips the disk rule).
Truncated Unstaged diff with `L` below its last shown line → `Unstaged(L)`, placed by `line_row`'s nearest.
Also tested (not asked): `select::show_line_section_gone_lands_on_the_first_row_first_change` (`Some(s)` with no row
in `s` → first row, `at: None`).

**Stay green (named):** `landing::tests::*` — the 10 existing (`show_line_lands_on_commit_file_and_line`,
`…puts_the_hunk_heading_on_top`, `…focuses_the_diff_body`, `…working_tree_lands_on_the_uncommitted_row`,
`…path_missing_says_so`, `…unreachable_commit_says_so_and_moves_nothing`, `a_key_during_a_landing_drops_it`,
`a_second_show_line_replaces_the_first`, `…commit_with_a_pinned_base_says_so`, `…on_the_pinned_base_itself_lands`);
`place::tests::navigation::enter_lands_on_the_commit_with_the_diff_at_this_line`,
`…enter_on_an_uncommitted_line_lands_in_the_changes_pane`; every `place::tests::esc_back::*` (7). All in the full
nextest run, 4220/4220.

**C — grep** (`docs/shots/` outside `m7/`, a hunk heading `@@ -n`/`@@ n`, no `▎`): 7 files.
- `m2-detail-keys-resize.txt`, `m2-detail-pane-breakpoints.txt`, `m2-diff-widget.txt`: no writer in `crates/` or
  `scripts/` → **marked** (the Historical line, verbatim, `<milestone>` = M2).
- `m5-contact-sheet-2.txt`: **record**, not edited (the ruling).
- `m2-side-by-side.txt`: exempt (side by side).
- `m5-ui-conflicts.txt`: writer `graft-ui/src/detail_pane/tests.rs` (outside my crate set), green in the full run →
  today's drawing; nothing to do.
- `m6/prototype-journeys.txt`: the M6 prototype's (pipeline step 5) record, no writer; not the app's drawing, so none
  of the three outcomes fits. **Left unedited — conductor's call.**
A second grep for a drawn unified-diff gutter without `@@` found 15 more; every one but `m3-contact-sheet.txt` (a
record) has a `check_shot` writer that is green, so they are today's drawing (no `▎` there: the diff isn't focused).
**Citations presenting a marked file as today's drawing** (edited none): `TESTING.md:1168`
(`m2-detail-pane-breakpoints`); `docs/capabilities/m2-details-and-diffs.md:28, :68, :124, :140` ("Looks like:"), and
`docs/generated/capabilities.html`, generated from it.

**Touches.**
- `show_line` callers (grep `show_line(` + compiler): blame `⏎` (`place/keys.rs`) passes `None`; the trail's leave
  (`place/trail.rs`) passes `origin.section`; landing tests pass `None`, the new section test `Some(Unstaged)`.
- `Origin` constructions (grep `Origin {`): `place.rs` `b_key` records `cursor.section`; `place/tests.rs`
  `trail_over` sets `None`.
- `LandingWatch`: the re-select and the unfold run in `advance` (job answers / `show_line`), not a key — tests 2 and
  B land, so neither dropped the landing.
- `files_arrived`/`KeptCursor`: untouched; the existing landing tests green.
- `diff_cursor`, `blame_from`: not edited (`git diff --stat`).
- Keymap, journal/undo: no edit (`git diff --stat` shows no keys, journal or undo file). graft-ui, graft-git: no edit.
- `jobs/detail.rs` (34's): not edited (`git diff --stat`).
- `detail/tests.rs`: `changes_pane` helper made `pub(super)` for the new section test.
- For the conductor: closes M7/20b-r-M1, M7/20b-r-m2, M7/20b-r-m3 (the sheet by ruling; prototype-journeys open).
