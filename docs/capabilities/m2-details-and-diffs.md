# Capabilities — M2: details and diffs

This milestone made a selected commit (or the working tree, or two picked commits)
readable without leaving the graph: a details pane that lists changed files and
shows their diffs — unified or side by side, syntax-highlighted, word-tinted — and
a Mistake Radar that flags foxtrot merges, backward merges, accidental pull
merges, evil merges, force-pushed remotes and duplicate (cherry-picked) commits
right on the graph row, with a plain-language explanation panel behind them. A
map view gives a whole-repo, zoomed-out picture of branch activity over time, and
an overview strip gives a one-column minimap of the current history.

## Capabilities

### M2-01. Every commit's changed files list in a details pane

- **How:** `Enter` on a history row (or `x`/context menu — see M2-11/M2-12).
- **Behaviour:** Opens the pane, moves focus to the file list, issues one `ListFiles` job for that row's commit. Files are grouped into a directory tree (`assemble_files`): a path list like `a/b/c.rs`, `a/d.rs`, `e.rs` renders rows `a/`, `b/`, `c.rs`, `d.rs`, `e.rs` at depths 0,1,2,1,0. A non-empty commit body becomes a `Message { lines }` row above the files unless the body is only a `Signed-off-by:` trailer, in which case no message row appears. `Space` folds/unfolds a `Dir` row, hiding only its own children. Moving the cursor onto a file issues one `ReadFileDiff`; revisiting a file already read is a cache hit (no new job). A second `Enter` on the same row while already open, or a stale job product with a non-current pending id, does nothing. `Esc` closes the pane.
- **Proven by:**
  `graft::update::detail::tests` (`Enter`/`j`/`Esc`/stale-drop/cache-hit cases per ticket 06's acceptance)
  `graft::view::detail::assemble_files` and `assemble_diff` unit tests (crates/graft/src/view/detail.rs)
- **Looks like:** `docs/shots/m2-detail-pane-breakpoints.txt`
- **Decided in:** DESIGN.md §1.8, §2.2–2.5; ADR-0023 (no hover — details live in this pane)
- **Status:** works

### M2-02. The working tree itself opens in the details pane

- **How:** `Enter` on the pinned uncommitted row (top of history when `HEAD` is the top visible row).
- **Behaviour:** Opens with `focus == Files`, issues `ListFiles(_, DiffSpec::WorkingTreeVsHead, _)`, no `ContainedIn` lookup (there's no commit to look up). The pane header reads `UNCOMMITTED · N staged M unstaged K untracked` / `diff vs HEAD <sha>` with no author/contained line. `k` at the very top selects the row (asks history for nothing); from the row, `j` moves to `HEAD`'s row; `Down(3)` moves to `top + 2`; `PageDown`/`G` clear the "on row" flag and apply the motion; `k`/`PageUp`/`g` at `top == 0` do nothing. A search hit, a click elsewhere, or a refs update with a different `HEAD` all clear the flag. Known limit: the row only appears while `HEAD` is the top visible history row — with a newer branch scrolled above it, the row is unreachable until you scroll back (owned by M3, see below).
- **Proven by:**
  `graft::update::selection::tests::up_one_from_the_top_row_selects_the_virtual_row_and_asks_history_for_nothing`
  `graft::update::selection::tests::down_one_from_the_row_selects_heads_row_and_asks_history_for_nothing`
  `graft::update::selection::tests::down_three_from_the_row_leaves_down_two_for_history`
  `graft::update::selection::tests::reconcile_clears_when_the_tree_goes_clean`
- **Looks like:** `docs/shots/m2-working-tree-row.txt`
- **Decided in:** M2 design note §02 (interfaces); handed to M3 for the scroll-reach gap
- **Status:** works (scroll-reach gap open, see Deliberately not built)

### M2-03. Diffs render as a unified, syntax-highlighted, word-tinted view

- **How:** Default view once a file is selected in the details pane.
- **Behaviour:** One row per diff line, sign glyph plus soft background per line kind (added/removed/context). Gutter width equals the widest old/new line number across the file's hunks (minimum 3); a line present on only one side leaves that gutter half blank. Word-level differences inside a changed-line pair get a visibly stronger background than the line's own soft background (`similar`-based word diff — `word_spans`). Rust/etc. keywords, strings, comments get syntax colour (`syntect`, bottom-up scope matching so nested comment delimiters don't split spans). Scrolling past a hunk's own heading keeps that heading pinned to the body's top row until the next heading would naturally appear. A truncated file (more than `max_lines`, default configurable) ends with `… N more lines · L load all`; `usize::MAX` loads everything and drops the row. A line with no trailing newline shows `␍`/no-newline glyph outside the gutter, not mixed into the text. Under `Palette16`/`NO_COLOR`, no diff cell carries a non-default background — added/removed are bold/dim only, word tint and syntax colour both degrade to that same rule.
- **Proven by:**
  `graft-ui::detail_pane::tests` gutter/word-tint/syntax/pin/truncate/CRLF/16-colour cases (ticket 08 acceptance, `crates/graft-ui/src/detail_pane/tests.rs`)
  `graft-ui::diff_words::word_spans` unit tests
  `graft-ui::syntax::tests::keywords_function_and_string_are_found`, `a_block_comment_spans_the_lines_between_open_and_close`
- **Looks like:** `docs/shots/m2-diff-widget.txt`
- **Decided in:** M2 design note §05 (New UI); PROMPT.md §5 (Diffs)
- **Status:** works. Colour itself is unverified (captures record character grids only, no colour attributes — CLAUDE.md rule 5).

### M2-04. Diffs can render side by side instead of unified

- **How:** `v` while the diff pane is focused (`Files`/`Diff` focus; ignored with focus on `Graph`).
- **Behaviour:** Toggles `AppState.detail.mode` between `Unified` and `SideBySide` and back; emits no command (pure view re-render, no new job). A paired removed/added line pair renders on the same screen row, old on the left, new on the right; an unpaired context or added-only/removed-only line leaves the other half of that row blank rather than shifting rows. Word tint, syntax colour, the no-newline glyph and the 16-colour fallback all match the unified view at the equivalent cell.
- **Proven by:**
  `graft::update::detail::tests::v_toggles_side_by_side_and_back_but_is_ignored_from_graph_focus`
  `graft-ui::detail_pane::tests::paired_rows_share_a_screen_row_and_unpaired_rows_leave_a_blank_half`
- **Looks like:** `docs/shots/m2-side-by-side.txt`
- **Decided in:** M2 design note §05
- **Status:** works

### M2-05. Diff display options: whitespace, word tint, context width, which side of a rename

- **How:** `w` (ignore whitespace), `W` (word-tint on/off, local to the session), `[`/`]` (context lines, presumably narrower/wider), `L` (load-all on a truncated file).
- **Behaviour:** Each key changes exactly its own `DiffOptions` field and issues one `ReadFileDiff`; pressing the same toggle again after the first round-trip is a cache hit (no new job) since the diff for that option combination is already cached. `context: 0` and `context: 8` change hunk line counts the same way `git diff -U0`/`-U8` do. `ignore_whitespace` on a whitespace-only change collapses it to no hunks at all.
- **Proven by:**
  `graft::update::detail::tests::w_toggles_ignore_whitespace_and_capital_w_toggles_word_tint_locally`
  `graft-git` diff-reads acceptance: context/ignore-whitespace cases (crates/graft-git/tests/it — ticket 05 acceptance)
- **Decided in:** M2 design note §02
- **Status:** works

### M2-06. The details pane docks right, below, or as a full tab depending on width

- **How:** Automatic, based on terminal width; `<`/`>` resize it, mouse drag on the divider resizes it, mouse wheel over files/body scrolls that region.
- **Behaviour:** `DetailLayout::compute`: ≥160 columns → `Right` (pane beside the graph); ≥99 and <160 → `Below`; <99 (down to a floor) → `Tab` (pane replaces the graph screen, `shown` follows `focus != Graph`). `<`/`>` move `right_columns`/`below_rows` by ∓4, clamped to the layout's own minimum (48 cols / 8 rows) and the graph's minimum (40 cols / 6 rows); a press at a bound is a no-op (returns false, no state change, no command). `<`/`>` are also a no-op in `Tab` layout or with the pane hidden, but work with focus on `Graph` as well as `Files`/`Diff`. `+`/`-` never resize the pane (they still unfold lanes). Each orientation's chosen size persists independently: resize in `Right`, switch to `Below`, resize there too, switch back — both remembered, round-tripped to `<common_dir>/graft/layout` on disk (non-fatal if the write fails), reloaded at startup. Mouse: `Down(Left)` on the divider's first column/row starts a drag (swallows the click), `Drag(Left)` resizes live and re-saves, `Up` always clears the drag; wheel over `Files` moves the file cursor, wheel over `Body` scrolls the diff, wheel over the divider or elsewhere falls through to the graph's own viewport scroll.
- **Proven by:**
  `graft::update::detail::tests::angle_keys_resize_the_right_pane_and_leave_below_alone`
  `graft::update::detail::tests::angle_keys_resize_rows_in_the_below_layout`
  `graft::update::detail::tests::angle_keys_are_a_no_op_in_the_tab_layout`
  `graft::update::detail::tests::plus_and_minus_never_resize_the_pane`
  `graft::layout` round-trip unit tests (crates/graft/src/layout.rs, 3 tests per ticket 10b notes)
- **Looks like:** `docs/shots/m2-detail-pane-breakpoints.txt`, `docs/shots/m2-detail-keys-resize.txt`
- **Decided in:** M2 design note §05
- **Status:** works. `?` help screen listing `<> resize` is deferred to M8 (no help overlay exists anywhere yet).

### M2-07. Focus and keys inside the details pane

- **How:** `Tab`/`Shift+Tab` cycle focus (`Files`↔`Diff`, wrapping); `Enter` on a file moves focus from `Files` to `Diff`; `Esc` closes the pane.
- **Behaviour:** `Tab`/`Shift+Tab` cycle in order and wrap at the ends. `Enter` only ever moves `Files → Diff`, never the reverse. The focused region's title paints in the accent colour; the other two stay muted.
- **Proven by:** `graft-ui::detail_pane::tests` — focus-title snapshot case (ticket 07 acceptance)
- **Looks like:** `docs/shots/m2-detail-pane-breakpoints.txt`
- **Decided in:** M2 design note §05
- **Status:** works

### M2-08. Compare any two commits directly

- **How:** `x` on a history row to pin it as the compare base; `x` on a second row (or `Shift+click`/`Ctrl+click`) to open the comparison; `x` again on the same row clears the base.
- **Behaviour:** `x` on an unmarked row sets `compare_base` and emits no command. `x` on a different row while a base is set opens `DetailSpec::Compare { base, head }` and keeps the base pinned. Moving the cursor with a base pinned opens a fresh `Compare` against the new row every time (not a plain `Commit` view). Landing on the working-tree row with a base pinned opens `WorkingTree` instead, still keeping the base; `x` on the working-tree row itself is a no-op. `Esc` clears the base and closes the pane in one press (ADR-0023's existing `Esc` order). The pane header reads `COMPARE <base> → <head>` with no `· N files +A −B` summary (that's a known gap, see below); the footer reads `compare base <sha> — move, x again clears`.
- **Proven by:**
  `graft::update::detail::tests::toggle_compare_with_no_base_pins_one_and_emits_nothing`
  `graft::update::detail::tests::toggle_compare_on_the_same_oid_clears_it_and_emits_nothing`
  `graft::update::detail::tests::toggle_compare_on_a_different_oid_opens_compare_and_keeps_the_base`
- **Looks like:** `docs/shots/m2-compare.txt`
- **Decided in:** M2 design note §02
- **Status:** works. `compare_header` never shows file/line totals — deferred (M8 polish, see Deliberately not built).

### M2-09. A right-click / `.` context menu on any row

- **How:** `.` or right-click on a history row (or the working-tree row).
- **Behaviour:** `for_row(marked: bool)` lists `Explain` first only when the row carries a Mistake Radar finding; otherwise it's omitted. `for_working_tree()` lists only `Open details`. The menu opens anchored at the row; `Esc` or a click outside it closes without dispatching. `j`/`k` move the cursor inside the menu clamped (not wrapping); `Enter` runs the selected action and closes. Actions: `CopySha` emits the same `Command::CopyToClipboard` as pressing `y` directly; `CompareWithThis` pins/compares (same as `x`); `OpenDetails` opens the commit (or the working tree, for that row); `Explain` sets a flag and emits no command. Menu width is the longest label's cell width plus 4.
- **Proven by:**
  `graft::update::context_menu::the_menu_runs_what_the_dropped_keys_run`
  `graft::update::context_menu::every_menu_lists_the_dropped_keys`
  `graft::update::context_menu::open_for_history_row_lists_explain_when_the_row_has_a_finding`
  `graft::update::context_menu::open_for_history_row_omits_explain_on_an_unflagged_row`
- **Looks like:** `docs/shots/m2-context-menu.txt`
- **Decided in:** M2 design note §05
- **Status:** works

### M2-10. The Mistake Radar flags foxtrot merges, backward merges, and accidental pull merges

- **How:** Automatic — a marker glyph (`⚠`, ASCII `!`) appears on the graph row for a flagged commit once radar context has loaded, and the pane header shows a radar line when that commit is open.
- **Behaviour:** `classify_commit` runs over every commit against `Policy::default()`. A foxtrot merge (a merge whose real first-parent tip diverges from what a merge subject like "Merge branch 'x'" implies, found either from the default chain or from the branch's previous tips in the reflog) yields one `FoxtrotMerge` finding on that merge commit. A backward merge yields one `BackwardMerge`. An accidental pull merge yields one `AccidentalPullMerge`. Non-merge commits (`parents.len() < 2`) never match, regardless of subject. `merged_branch_from_subject` recognizes six subject forms plus a remote "into" form, and explicitly returns `None` for a `Revert "Merge branch…"` subject and for a lower-cased `merge branch…` (git's own capitalization only). Each of the seven radar kinds can be turned `off`/`info`/`warning` via `[radar]` config; an unrecognised config key is a non-fatal `ConfigWarning`, not a load failure.
- **Proven by:**
  `graft-ops::mistake_radar` fixture-matrix test looping `Shape::ALL` (ticket 04 acceptance; crates/graft-ops/src/mistake_radar/mod.rs or classify.rs)
  `graft::view::radar::tests::a_foxtrot_merge_on_the_default_chain_finds_one_warning`
  `graft::view::radar::tests::a_non_merge_off_the_chain_finds_nothing`
  `graft::view::radar::tests::with_evil_merge_off_no_unchecked_line_is_added` (config-off path)
- **Looks like:** `docs/shots/m2-marker-column.txt`
- **Decided in:** PROMPT.md §1.4; DESIGN.md §1.8; M2 design note §02
- **Status:** works

### M2-11. The marker column and radar lines stay stable as you scroll

- **How:** Automatic — a fixed column beside the graph rows.
- **Behaviour:** The marker column is always reserved (its width never changes, whether or not a finding exists) because it also carries the pinned-compare accent (M2-08). It shows a finding's glyph once radar context is `Ready` and a finding is known for that row, and — the specific guarantee — stays visible after scrolling a marked row off-screen and back, via a sticky `AppState::radar_column_seen` bit (history windowing evicts off-screen rows, so the column can't just re-derive from what's currently laid out). The detail pane's header joins every applicable `RadarLine` for the open commit with " · " into one reserved line (not just the first).
- **Proven by:**
  `graft::view::radar` marker/column tests (ticket 13/13b acceptance)
  `graft-ui::detail_pane::tests` radar-line wording tests, one per `RadarLine` variant, plus the join case (ticket 13c)
- **Looks like:** `docs/shots/m2-marker-column.txt`
- **Decided in:** M2 design note §02
- **Status:** works

### M2-12. Evil-merge probe: a merge that discards changes its subject claims to preserve

- **How:** Automatic once a merge commit's details are opened.
- **Behaviour:** `probe_evil_merge` returns `Evil { files }` (non-empty file list) when a merge silently drops content one of its parents introduced; `Clean` for an ordinary merge; `NotCheckable("not a merge")` for a non-merge. Opening an evil-flagged merge's detail pane issues one `EvilProbe`; revisiting it in the same session is a cache hit. Moving off the merge before the probe answers cancels it; a late answer for a since-abandoned probe changes nothing. An `Evil` verdict becomes one `EvilMerge` finding (no separate `EvilUnchecked`/`EvilClean` line drawn alongside it); while pending the line reads `EvilChecking`; once answered clean, `EvilClean`. With `[radar] evil = off`, no probe is ever issued and no evil-related line appears at all.
- **Proven by:** `graft::update::detail::tests::a_commit_spec_probes_once_and_a_revisit_is_a_cache_hit`, `a_refresh_keeps_the_evil_cache`
  `graft::view::radar::tests::the_evil_line_follows_the_probe_through_each_loading_state`
  `graft::view::radar::tests::an_evil_verdict_is_one_finding_and_no_separate_evil_line`
- **Looks like:** `docs/shots/m2-evil-merge.txt`
- **Decided in:** PROMPT.md §1.4; M2 design note §02
- **Status:** works

### M2-13. Force-push probe: a remote-tracking branch whose history was rewritten

- **How:** Automatic once radar context lands, for every upstream-tracked branch.
- **Behaviour:** `probe_force_push` returns `Rewritten{..}` when a remote-tracking ref's reflog shows a non-fast-forward update; `Clean` for an ordinary fast-forward; `NotCheckable("no remote-tracking reflog")` when there's no reflog entry to judge from. Exactly one probe runs per tracked branch, none for branches with no upstream. A `Rewritten` verdict yields one `ForcePushedRemote` finding at the branch's tip; a `NotCheckable` verdict yields no finding but does yield a `RadarLine::ForcePushNotCheckable` line when that tip is the open commit. Known limit: verdicts are computed only for branches that have entered the visible window, capped at 32 per session — a branch never scrolled to goes unflagged (price of fixing a timeout; see Deliberately not built).
- **Proven by:**
  `graft::update::radar::tests::one_probe_per_tracked_branch_and_only_once`
  `graft::view::radar::tests::a_rewritten_remote_is_a_finding_at_the_branchs_tip`
  `graft::view::radar::tests::a_not_checkable_verdict_is_a_line_and_never_a_finding`
- **Looks like:** `docs/shots/m2-force-push.txt`
- **Decided in:** PROMPT.md §1.4; M2 design note §02
- **Status:** works, narrowed (32-branches-per-session cap; not exhaustive — see Deliberately not built)

### M2-14. Duplicate-commit probe: cherry-picks and other content repeats

- **How:** Automatic once radar context lands.
- **Behaviour:** `probe_duplicates` groups commits with matching content; a group of three yields three findings, each listing the other two as evidence. With no repeated content, `groups: vec![]`. The probe always runs once per session, but with `[radar] duplicate = off`, `findings_from_duplicates` yields nothing even though the probe ran, so no finding or marker appears. Severity is `Info` by design (not `Warning`), so the graph marker is `ⓘ`.
- **Proven by:**
  `graft-ops::mistake_radar::probes::the_cherry_picked_duplicate_is_found_in_its_own_group`
  `graft-ops::mistake_radar::probes::a_fixture_with_no_repeated_content_has_no_groups`
  `graft::update::radar::tests::duplicates_are_asked_for_once_the_context_lands_and_only_once`
  `graft::update::radar::tests::the_duplicate_kind_off_leaves_findings_empty_though_the_probe_ran`
- **Looks like:** `docs/shots/m2-duplicate-commit.txt`
- **Decided in:** PROMPT.md §1.4
- **Status:** works

### M2-15. Explain panel: plain-language reasoning behind a flagged commit

- **How:** `.` → `Explain` from the context menu on a flagged commit.
- **Behaviour:** Opens the pane on that commit and sets a title (e.g. `FOXTROT MERGE`), two-ish paragraphs of explanation, and a 5-line evidence sketch with connectors alternating `──╮`/`──╯` between rows, drawn in the evidence's own order (e.g. `[main] was here` / `this merge` / `first parent`). With Explain open, every graph row not in the explanation's highlighted set fades; closing Explain reverts to the ordinary `F`-ancestry fade rule. `Esc` closes only Explain first (pane stays open on the same commit); a second `Esc` then closes the pane. The row's hint only shows `. explain` when that row actually carries a finding. If the flag is set but no finding is found (stale), the flag clears and no explain view opens — no error.
- **Proven by:**
  `graft-ops::mistake_radar::explain` — sketch/evidence-order tests (ticket 17, `build_sketch`)
  `graft::update::detail` explain-open/close/fade/hint tests (ticket 17 acceptance)
  `graft::update::context_menu::open_for_history_row_lists_explain_when_the_row_has_a_finding`
- **Looks like:** `docs/shots/m2-explain-panel.txt`
- **Decided in:** DESIGN.md §1.8; M2 design note §02
- **Status:** works. No `[ Fix… ]` action button — explicitly deferred to M5.

### M2-16. Map view: whole-repo branch activity over time

- **How:** `M` toggles the map; `Enter` on a branch row jumps back to the graph at that branch's tip; `Esc`/`M` closes it.
- **Behaviour:** `M` opens the map, replacing the graph screen entirely (no graph rows or strip column draw underneath it), and issues one `BuildMap` request whose bucket count is `columns − 24` (minimum 10, using the caller's own clock — never the map job's own clock, to stay pure). Rows: one per local branch, ordered current-branch-then-default-then-by-tip-date; each row carries a per-bucket commit-count histogram (`ramp_glyph`: 0 → space/`.`, max → `█`/`@`, any non-zero count never renders blank). A branch merged into the default branch's first-parent chain gets `merged_into = Some((row, bucket))`, drawn as `●╮` at that bucket; branches not merged (or merged only into another feature branch) get `None` — cross-branch merges into non-default branches are not tracked. A tag on a commit draws `[tag]` beside its bucket's bar. A flagged commit (e.g. a foxtrot merge) draws a `⚠`/`ⓘ` mark at its bucket. An unborn/empty branch draws with no panic; an empty repo gives `branches.is_empty()`. Resizing the terminal while the map is open cancels the in-flight build and issues a new one at the new bucket count. `Enter` on a row closes the map and jumps the graph to that branch's tip commit.
- **Proven by:**
  `graft::map::tests::feature_branches_rows_order_and_merged_into`
  `graft::map::tests::tags_mark_the_bucket_of_their_peeled_commit`
  `graft::map::tests::foxtrot_merge_marks_a_warning_at_the_merge`
  `graft::map::tests::unborn_branch_has_no_current_row_and_no_panic`
  `graft::map::tests::stop_after_the_first_chunk_is_an_error`
  `graft-ui::graph_screen::tests::map_shown_replaces_the_graph_screen_entirely`
- **Looks like:** `docs/shots/m2-map-view.txt`, `docs/shots/m2-map-view-loading.txt`
- **Decided in:** M2 design note §02
- **Status:** works. Merge-elbow tracking is first-parent-into-default only, by design (declined cost); pixel-tier rendered image diffs are not built (PROMPT §4 promise; see Deliberately not built).

### M2-17. Overview strip: a one-column minimap of the current history

- **How:** `O` toggles it (per-run only, no config to start it on).
- **Behaviour:** Reserves one graph column of width when on, none when off; the rest of the row layout (subject/author/date/SHA columns) starts in the same screen column either way. Each strip cell represents a bucket of history rows (`row_for_cell(100 rows, 24 cells, cell 23) == 96`); a cell shows the highest-priority mark among its rows (`Merge` and `Head` marks by priority). Clicking a strip cell selects that bucket's first row.
- **Proven by:**
  `graft::update::strip::tests::o_toggles_strip_shown_on_and_off`
  `graft::update::strip::tests::o_is_ignored_while_the_map_is_shown`
  `graft::view::strip::tests::feature_branches_marks_both_merges_and_head_and_the_viewport`
- **Looks like:** `docs/shots/m2-overview-strip.txt`
- **Decided in:** M2 design note §05
- **Status:** works. No `[ui]` config key to start it on by default (M8 polish).

### M2-18. Git commands are never spawned outside a single runner, with proper cancellation

- **How:** Internal — every git invocation in the app goes through one seam.
- **Behaviour:** `GitInvocation::new(...).arg(...).run(...)` executes and parses results; a non-zero exit surfaces as `Exited { status, stderr_first_line, command }` with the actual command line attached. A `Stop` signal answered mid-run ends a long `git log -p --all` cleanly (`Err(Stopped)`, process actually reaped — no zombie). `output_cap` truncates huge output (`truncated == true`) rather than buffering unbounded. `stdin` can be piped in (e.g. `hash-object --stdin`). Path arguments carrying non-UTF-8 bytes reach git unchanged, byte for byte (skipped on filesystems, like APFS, that can't represent such paths). No `Command::new("git")` exists anywhere outside this runner module (enforced by a repo-wide grep check in the test suite).
- **Proven by:**
  `graft-git::run` tests: `borrow(Shape::Linear)` head-resolution case, `Exited` non-zero-exit case, the `Stop`-mid-log case, `output_cap` truncation case, `stdin` hash-object case, non-UTF-8 path case (ticket 01 acceptance; crates/graft-git/src/run.rs)
- **Decided in:** ADR-0011 (gix primary reader, git2 for named gaps); M2 design note §03
- **Status:** works

### M2-19. Diffs are parsed byte-exact, including renames, binaries and non-UTF-8 paths

- **How:** Internal — feeds M2-03/M2-19's rendering and the file list.
- **Behaviour:** `parse_unified` on every commit of every fixture shape matches `git --numstat`'s added/removed counts exactly (binary files excluded from the count). Handles: rename-with-hunks, copy, mode-only change (no hunks), binary, new file, deleted file, `\ No newline at end of file`, CRLF lines (kept literally with `\r`), quoted non-UTF-8 paths unquoted back to the original bytes, an `@@` literal inside a hunk heading, multiple files in one stream, and a hunk-count mismatch (`ParseError { line }`). `parse_file_list` gives paths byte-equal to `git ls-tree -r -z` for both Unicode and non-UTF-8 names. A `pairs` proptest confirms removed/added line pairing never overlaps and every context line stays outside any pair's range.
- **Proven by:**
  `graft-git::tests::it::diff_fixture::parse_unified_matches_numstat_for_every_shape`
  `graft-git::tests::it::diff_fixture::parse_file_list_paths_match_ls_tree_for_unicode_and_non_utf8_names`
  `graft-git::tests::it::diff_cases` (hand-written cases)
  `graft-git::diff::pairs::tests::properties` (proptest)
- **Decided in:** M2 design note §03
- **Status:** works

### M2-20. Syntax highlighting and word-level diff spans over sanitised text

- **How:** Internal — feeds M2-03/M2-04.
- **Behaviour:** `language_for_path`/`language_for_first_line` resolve a file's language from its extension, exact filename (e.g. `Makefile`), or shebang; unknown extensions resolve to `None` (no highlighting, not an error). Highlighting never panics past 5,001 lines (falls back to `Plain` for the whole file) or past a single 4,001-char line (falls back to `Plain` for that call only) — proven by proptest including combining marks and emoji. `word_spans` computes exact char ranges for changed words between a removed/added line pair; a full replacement with no common word returns empty spans on both sides.
- **Proven by:**
  `graft-ui::syntax::tests::extension_resolves_a_known_language`, `first_line_resolves_a_shebang`, `keywords_function_and_string_are_found`, `a_block_comment_spans_the_lines_between_open_and_close`, `past_5000_lines_everything_is_plain`, `a_line_past_4000_chars_falls_back_to_plain_for_the_call`, `highlight_never_panics_or_overlaps` (proptest)
  `graft-ui::diff_words::word_spans` unit tests
- **Decided in:** M2 design note §03
- **Status:** works. Clean-build time cost measured (~4.2s → ~7.1s with `syntect`), accepted trade per PROMPT.md.

### M2-21. Diff reads support ranges, working-tree comparisons, and truncation

- **How:** Internal — feeds M2-01/M2-02/M2-08.
- **Behaviour:** `list_files` on a single commit matches `git show --stat`'s file set and totals exactly. A merge commit's file list is against its first parent by default; an explicit `against: Some(second_parent)` gives a different set. `Range { base, head }` lists every file the head has relative to base. `WorkingTreeVsHead` reports per-file status (`Modified`/`Added`/`Untracked`) matching what was actually written to the tree (modified, staged-new, untracked). `file_diff` with `max_lines` stops on a hunk boundary and reports `truncated == Some(_)` with the true total line count; `usize::MAX` returns everything untruncated. A pre-set `Stop` short-circuits to `DiffError::Run(RunError::Stopped)` before any parsing.
- **Proven by:** `graft-git` diff-reads acceptance suite (ticket 05; crates/graft-git/tests/it — `list_files`, `WorkingTreeVsHead`, `max_lines`, `context`, `ignore_whitespace`, non-UTF-8-path cases)
- **Decided in:** M2 design note §02
- **Status:** works

### M2-22. Refresh picks up a newly-built commit graph without restarting the walk from scratch

- **How:** Automatic — triggered by `Ctrl+R` (refresh) or when a background commit-graph write finishes.
- **Behaviour:** `refresh()` on a repo with no commit-graph table returns `Refreshed { table_rebound: true }` once one appears (e.g. after `git commit-graph write --reachable`); a second refresh with no change returns `table_rebound: false` and reuses the exact same `Arc` (pointer-equal), not a fresh table. When the graph appears between walks, the next walk restart reanchors on the previously-selected commit rather than losing the cursor. The footer line for "commit-graph written" ends `· Ctrl+R to use it` at ≥120 columns; unchanged wording below that.
- **Proven by:**
  `graft::repos::tests::a_refresh_rebuilds_the_table_when_the_commit_graph_appears`
  `graft::repos::tests::a_refresh_with_no_commit_graph_change_keeps_the_table`
  `graft::jobs::tests::refresh_after_a_commit_graph_write_puts_the_next_walk_on_the_graph_path`
  `graft::update::tests::a_walk_restart_reanchors_on_the_previously_selected_commit`
  `graft-ui::graph_screen::tests::every_commit_graph_wording_fits_under_its_breakpoint`
- **Decided in:** M2 design note §02
- **Status:** works

## Deliberately not built

- **Pixel-tier rendered image diffs.** PROMPT §4 describes a before/after image render in the Kitty/iTerm2 tier; M2 only shows `binary · <old> → <new>` for images, same as any other binary file. Owner: M8 polish.
- **`[ui]` config key to start the overview strip on by default.** `O` only toggles per run. Owner: M8 polish.
- **Map view's merge elbow for feature-into-feature merges.** Only merges into the default branch's first-parent chain draw an elbow; a cross-branch merge search was weighed and declined for cost. Not planned unless asked.
- **Compare pane header's `· N files +A −B` summary.** `compare_header` builds only the base/head SHAs; wiring in `DetailState::files`'s totals is unbuilt. Owner: M8 polish.
- **Force-push probing is not exhaustive.** Verdicts are computed only for branches that entered the visible window in a session, capped at 32 — an unscrolled-to branch goes unflagged with no "not checked" indicator. Price of fixing a real timeout (unbounded probing took 1,275s on the heavy tier; capped, it's 91s). Owner: M7, decides whether to fund an exhaustive background sweep or keep the cap and say so in the UI.
- **`?` help overlay and 80-column footer hints for M2's new keys** (`Space`, `1`, `H`, `<`, `>`, `x`, `.`, `v`, `w`, `W`, `[`, `]`, `L`, `M`, `O`). No help-screen infrastructure exists anywhere yet. Owner: M8 (design already specified in DESIGN.md §1.16).
- **Working-tree row reach when `HEAD` isn't the top visible row.** With a newer branch scrolled above `HEAD`, the row is unreachable without scrolling manually. Owner: M3 (working tree) decides whether to pin it.
- **Heap growth against the M1 budget** (~76 B/commit against a 50 B/commit target after this milestone's changes; peak RSS 214 B/commit against 200). Needs its own investigation into what holds per-commit state beyond the layout window. Owner: unassigned follow-up ticket.
- **No `criterion` benchmarks, no CI.** Heavy-tier fixture tests and a 1,000-case proptest exist; formal benchmarks and continuous integration are out of scope without a hosted repo. Not planned.
- **Gallery/capture coverage gaps**: no pixel-tier or light-theme or ASCII captures except a handful of stops at 80×24; no 100×30 coverage except those same stops; the force-push probe has no gallery stop against a real rewritten remote (only a test-backend fixture); no corpus commit touching two files exists, so the file-cursor `j` motion inside the details pane has no gallery stop. Owner: M2 review follow-up, unassigned.
