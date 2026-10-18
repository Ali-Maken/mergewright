# graft-ui — map
The screens: widgets, layout, theme and the view models they render from. Draws and nothing else: no repository handle, no `git` or `ops`.
Depends on `graph`, `term`, `core` (ARCHITECTURE.md crate table). Repo-derived strings arrive only as `SanitizedText`.
One line per public item. Update when a public item changes.

## lib.rs
- `blame` (mod) — the blame place's picture; pure draw.
- `commit_editor` (mod) — the commit screen: message, file list, changes.
- `confirm` (mod) — the confirm box with N choices.
- `conflict_resolver` (mod) — the conflict resolver screen: document with cards, file list, three panes.
- `context_menu` (mod) — the `.`/right-click popup of row actions.
- `credential_prompt` (mod) — the modal askpass answers a username, password or host-key question in.
- `detail` (mod) — the detail pane's view-model types; no drawing.
- `detail_pane` (mod) — the detail pane's layout and widget.
- `diff_words` (mod) — word-level highlighting inside a paired diff line.
- `empty_state` (mod) — the screen outside a repository: recent repositories, open and clone prompts.
- `finish_summary` (mod) — the screen that sums up an operation before it is finished.
- `fit` (mod) — clamp text to an exact cell width.
- `graph_column` (mod) — the Unicode-tier graph column widget.
- `graph_screen` (mod) — the graph screen Graft opens on: header, banner, rows, footer hints.
- `history` (mod) — the file history place's picture; pure draw.
- `journal_screen` (mod) — the journal overlay.
- `keymap` (mod) — every key binding as data, and the translate that remaps them.
- `map_screen` (mod) — the Map view widget.
- `name_prompt` (mod) — the one-field name/create prompt.
- `overview_strip` (mod) — the one-column whole-history strip.
- `paint` (mod) — token-to-style helpers for widgets.
- `palette` (mod) — the command palette and help box.
- `picker` (mod) — the branch picker's box.
- `preview_box` (mod) — the Ghost Preview box and its dock.
- `rewrite_editor` (mod) — the rewrite editor's draw.
- `stash_screen` (mod) — the stash overlay.
- `status_screen` (mod) — the status screen view and widget.
- `still_to_apply` (mod) — the still-to-apply block at a rewrite stop.
- `syntax` (mod) — line-level syntax highlighting over sanitised text.
- `text_input` (mod) — text entry: single-line, secret and multi-line fields.
- `theme` (mod) — theme kinds, tokens and colour resolution.
- `tour_card` (mod) — the practice tour's card.
- `whitespace_tail` (mod) — the title tail saying whitespace is ignored, for blame, detail and history.
- `Cells` (struct) — `x`, `y`, `width`, `height` (`u16` each): a rectangle by name, for a caller that must not name the rendering library.
- `cells_of` (fn) — a `Rect` as [`Cells`]; with `rect_of`, the only conversions.
- `rect_of` (fn) — [`Cells`] as a `Rect`.
- `PendingRowView`, `PendingView`, `draw_still_to_apply` (re-exports from `still_to_apply`) — the still-to-apply block's view and draw.

## blame
The blame place's picture; pure draw, no state.
- `BlameRev` (enum) — `Now` / `AsOf(sha7)` / `Before(sha7)`: the title's last words.
- `Who` (struct) — `sha7`, `name`, `author_time`, `subject`: a gutter block's commit.
- `BlockPos` (enum) — `Single` / `First` / `Middle` / `Last`: a row's place in its commit's block.
- `RowMark` (enum) — `Plain` / `NotCommitted` / `Skipped` / `Unblamable` / `Shallow`: what the gutter says instead of a commit.
- `BlameRowView` (struct) — `line_no`, `text`, `who: Option<Who>`, `block`, `mark`.
- `BlameBody<'a>` (enum) — `Reading` / `Failed(&str)` / `Lines { rows, cursor, top }`.
- `IgnoreLine` (struct) — `count`, `applied`: the `.git-blame-ignore-revs` line, drawn only when `Some`.
- `FindTitle<'a>` (struct) — `query`, `at` (1-based), `of` (0 = no match): the title's find tail.
- `BlameView<'a>` (struct) — `path`, `rev`, `body`, `ignore`, `find`, `heatmap`, `now`, whitespace flags: everything the place draws.
- `draw_blame` (fn) — `(frame, area, view, keys: &Keymap)`: title, ignore line, then gutter · number · code rows.
- `blame_row_at` (fn) — `(area, view, y) -> Option<usize>`: mouse hit-test into rows; `None` off the rows.

## history
The file history place's picture; pure draw, no state.
- `LIST_WIDE`, `LIST_MIDDLE`, `ONE_PANE_BELOW` (consts) — 56 / 40 / 120: the list's width by frame width; below 120 one pane shows.
- `RANGE_MARK`, `SELECTED` (consts) — `┃` and `▸`.
- `HistoryFocus` (enum) — `List` / `File`.
- `HistoryRow<'a>` (struct) — `sha7`, `subject`, `who`, `when`, `added`, `removed`, `was` (a rename's `↳ was <path>` row).
- `HistoryList<'a>` (enum) — `Rows { rows, selected, top, end }` / `Failed(&str)`.
- `FilePane<'a>` (enum) — `Empty` / `NoChange` / `Reading` / `Failed` / `Binary` / the file's text: what the file pane shows.
- `HistoryView<'a>` (struct) — `title`, `tracing`, `list`, `file`, `focus`, `now`, `theme`, `whitespace_ignored`.
- `Columns` (struct), `columns` (fn) — `columns(width, focus)`: where the list and file pane sit; shared with the mouse hit-test.
- `file_rows`, `changed_rows`, `row_of_new_line`, `row_of_old_line` (fns) — the file's rows from its one hunk, which changed, and a line's row.
- `list_index_at`, `rows_through` (fns) — list hit-test and height, counting a rename's `↳ was` row.
- `draw_history` (fn) — `(frame, area, view, keys: &Keymap)`: title, then list | file pane.

## detail
View-model types for the detail pane; no widget. Every string is `SanitizedText`.
- `Subject` (enum) — `None`, `Commit { sha }`, `Compare(CompareHeader)`, `WorkingTree { staged, unstaged, untracked }`: what the pane is about.
- `CompareHeader` (struct, re-exported from `detail/compare.rs`) — `base`, `head`, `state`: a compare's title.
- `CompareState` (enum) — `Reading`, `IsBase`, `Since { commits, tally }`, `AlreadyIn`, `NoSharedHistory`, `Failed`.
- `FileTally` (struct) — a compare's `files`, `added`, `removed`.
- `CompareSummary` (enum) — `Since { split, commits }` or `NoSharedHistory`: what a compare's file-list read learnt.
- `DetailHeader` (struct) — subject, summary, author, dates, committer, co-authors, parents, contained, radar lines.
- `ContainedIn` (enum) — `NotAsked`, `Checking`, `Known { branches, tags, more }`, `Failed`.
- `RadarLine` (enum) — one Mistake Radar line in the header: a finding, or an evil-merge / force-push state.
- `FileStatusView` (enum) — `Added`, `Deleted`, `Modified`, `Renamed`, `Copied`, `TypeChanged`, `Unmerged`, `Submodule`…: a file's status as listed.
- `SubmoduleView` (struct, re-exported from `detail/submodule.rs`) — one submodule's pointer move and inside changes.
- `SubmoduleView::move_words`, `SubmoduleView::row_words` (fns) — `(ids)`: the pointer's move, and the row's words after the path.
- `SubmoduleLogView` (enum) — `Commits(Vec<SubmoduleCommitView>)` / `NotHere`: the diff body on a submodule path.
- `SubmoduleCommitView` (struct) — one commit a pointer move pulled in or rewound.
- `WorkingTreeSection` (enum) — `Unmerged`, `Staged`, `Unstaged`, `Untracked`; `WorkingTreeSection::title` is the heading word.
- `FileRowKind` (enum) — `Message { lines }`, `Dir { collapsed }`, `File { status, added, removed }`, section rows.
- `FileRow` (struct) — `depth`, `name`, `kind`, `entry` (index into the git list; `None` for message/dir).
- `FileListView` (struct) — rows, cursor, top, totals, `message`, `compare`: the changed files as a folding tree.
- `FileListView::visible`, `FileListView::toggle_dir`, `FileListView::move_cursor` (fns) — rows not under a folded dir; fold a dir; move, clamped.
- `FileListView::selected_entry` (fn) — the cursor row's git entry, if a file.
- `FileListView::section_at`, `FileListView::selected_section` (fns) — the changes-pane group a row, or the cursor, is in.
- `FileListView::section_entries` (fn) — `(row) -> Vec<u32>`: every git entry under the section row at `row`.
- `LineKindView` (enum, re-exported from `detail/diff.rs`) — `Context`, `Added`, `Removed`, `HunkHeading`.
- `LineView` (struct) — kind, old/new numbers, text, word ranges, `SyntaxSpan`s, `pair` (its partner line).
- `HunkView` (struct) — one hunk; `lines[0]` is its heading.
- `DiffMode` (enum) — `Unified`, `SideBySide`.
- `DiffOptionsView` (struct) — context, ignore_whitespace, word_tint; `Default`.
- `DiffView` (struct) — path, old path, status, hunks, `binary`, `truncated`, `untracked`: one file's diff.
- `BodyRow` (enum) — what a row of the drawn diff body is: a hunk heading or a hunk's line.
- `DiffView::body_len`, `DiffView::body_row` (fns) — rows the body draws; what row `row` is, `None` past the end.
- `BodyView` (enum) — `Empty`, `Message`, `Loading`, `Failed`, `Diff(Arc<DiffView>)`: what the pane's body shows.
- `DetailFocus` (enum) — `Graph`, `Files`, `Diff`.
- `DetailView` (struct) — everything the pane draws: header, files, body, body_top, focus, mode, options, explain.
- `ExplainView` (struct) — the explain panel's title, severity, paragraphs, sketch.

## detail_pane
Draws a `detail::DetailView` beside, below or in place of the graph. No I/O.
- `DetailLayout` (enum) — `Right { columns }` (160+), `Below { rows }` (100-159), `Tab { shown }` (<100).
- `DetailLayout::compute` (fn) — the breakpoint for a size and focus; `override_cells` replaces the percentage, clamped to the minimum.
- `split_area` (fn) — `(layout, area) -> (Option<Rect>, Option<Rect>)`: graph rect and pane rect; `None` for the side `Tab` hides.
- `DetailAreas` (struct) — `header`, `files`, `body` rects for one layout.
- `detail_areas` (fn) — `(pane_area, layout, files_count) -> DetailAreas`, the files region capped to what it needs.
- `DiffFocus` (struct) — `focused`, `word_tint`, `cursor: Option<usize>`: how `diff_body` paints.
- `diff_body` (fn) — `(hunks, top, truncated, area, buf, theme, focus)`: the unified diff body, with a load-all footer when cut.
- `diff_body_side_by_side` (fn) — `(hunks, top, area, buf, theme, focused)`: the two-column diff over the same hunks.
- `DetailPane` (struct, `Widget`) — `view`, `layout`, `theme`, `keys: &Keymap`: draws the whole pane.

## diff_words
- `WordSpans` (struct) — `removed`/`added`: per-line char ranges that changed against the paired line.
- `word_spans` (fn) — word diff of `removed[i]` against `added[i]`; unpaired lines get empty spans.

## fit
- `MAX_MARKS` (const) — combining marks kept per base character.
- `fit` (fn) — text laid into exactly N cells: padded, or cut with `…`; wide chars never split.

## graph_column
- `CELLS_PER_LANE` (const) — 2: a lane's own cell, then the gap.
- `STUB_BELOW`, `STUB_ABOVE`, `RAIL` (consts) — `▾` / `▴` / `⋮`: folded edges and the rail folded lanes sit behind.
- `NodeKind` (enum, re-exported from `graph_column/kinds.rs`) — commit, merge, root, HEAD, stash, uncommitted, `StashOnly`, `Absent`.
- `NodeKind::of`, `NodeKind::glyph` (fns) — the shape from git's parents (`Row::is_merge`: a folded merge is still `◉`), stash-blind; the node's character.
- `RowPaint` (enum) — `Normal`, `Dimmed`, `Highlighted`, `DropTarget`…: how a row is painted.
- `GraphLine` (struct) — `row`, `node`, `paint`, `thread`, ghost lanes: one row as the widget draws it.
- `GraphLine::plain`, `GraphLine::owned`, `GraphLine::ghost` (fns) — a borrowed row, a synthesised row, a Ghost Preview row.
- `GraphColumn` (struct, `Widget`) — draws consecutive rows, filling its area exactly.
- `GraphColumn::new`, `GraphColumn::ascii`, `GraphColumn::through` (fns) — lines in a theme; the ASCII glyph table; drawn through a `Fold`.
- `GraphColumn::width_for`, `GraphColumn::width_through` (fns) — cells for N lanes; cells for a window through a `Fold`.

## graph_screen
- `SHA_WIDTH` (const) — 7: cells the short-SHA column takes.
- `AUTHOR_BREAKPOINT` (const) — 100: below this width the author chip drops.
- `CHROME_ROWS`, `SPACED_FROM_ROWS` (consts) — 2 non-graph rows under 30 rows; from 30 rows the chrome is spaced.
- `spaced`, `chrome_rows` (fns) — `(rows)`: whether the blank row and rule show; non-graph rows (2, or 4 spaced).
- `GUTTER`, `RIGHT_MARGIN` (consts) — 2 cells left of every list row (cursor slot + gap), 1 right.
- `Location` (struct) — `repo`, `path`, `branch`, `practice`: where the header says the screen is.
- `GraphView` (struct) — everything the graph screen draws: rows, header state, banner, tour, pending, footer state, hint context.
- `ProblemLine` (struct) — `reason`, `keys`: a failure the footer shows in place of the hints.
- `MarkerView` (struct) — a Mistake Radar marker's `severity`, painted in the row's marker slot.
- `GraphicsStatus` (enum) — `Ok` / `Failed(text)`: drawn `gfx ok` / `gfx: <error>`.
- `Filters` (struct) — `first_parent`, `stash_hidden`, `focused`; `Filters::text(keys)` is the header's words.
- `Compaction` (struct) — `max_lanes`, `expanded`, `keep`; `Compaction::at(max_lanes)` is collapsed with nothing kept.
- `GraphEntry` (struct) — one row: `line`, `badges`, `subject`, `author`, `when`, `sha`, `compare_marked`, `marker`, `tag`, `fold`.
- `FoldMark` (struct) — `open`, `count`, `from`: a folding merge's tail; `words(ascii)` is `▸ 4 commits from PR 16021`; `fitting(ascii, cells)` its longest form that fits (`▸ 4 commits`, then `▸`).
- `FoldCount` (enum) — `Pending`, `Known(n)`, `Failed`: the tail's count, none until it lands, `?` if it failed.
- `RowTag` (enum) — `Changed`: a row's tag after its subject; `RowTag::words(ascii)` per tier.
- `TAG_MIN_SUBJECT` (const) — 8: the fewest subject cells a tag leaves.
- `AuthorChip` (struct) — `initials` and the `identity` that colours them.
- `OrderNotice` (enum) — `Sorting` / `Provisional`: the header's word on row order.
- `WalkView` (enum) — opening, reading, jumping, paused, complete, cancelled: where the history walk is.
- `GraphScreen` (struct, `Widget`) — the graph screen; `GraphScreen::new(view, theme)`.
- `SearchLine` (enum) — `Prompt(text)`: the footer's ` /text_` while search is open.
- `SearchTail` (struct) — `text`, `failed`: a running or finished search in the header's tail.
- `CommitGraphLine` (enum) — `Offered { keys }`, …: the commit-graph offer's footer line.
- `APP_NAME` (const, from `header.rs`) — the product's name where the app names itself: `Generation::CURRENT.title()` (`Mergewright`).
- `HeaderView` (struct), `header_line`, `tail_room` (fns) — the header row's words; drawn at a width; cells left for the tail.
- `BannerView` (struct, from `banner.rs`) — `kind`, `step`, `conflict`: the operation banner.
- `RewriteBannerView` (struct), `BannerChain` (enum) — what a rewrite stop adds to the banner, and its chain words.
- `banner_line` (fn) — `(banner, width, keys) -> String`: the banner's text in its room.
- `CarveInputs` (struct, from `carve.rs`) — what the row carve reads of a `GraphView`, as plain values; `CarveInputs::of(view)`.
- `RowCarve` (struct) — `ScreenRows` in `Cells`: same bands, same names.
- `row_carve` (fn) — `(area: Cells, inputs) -> RowCarve`: the one carve of the screen's rows, shared with `graft::update`.
- `ScreenRows` (struct, from `layout.rs`) — header, gap, banner, tour, pending, body, offer, rule, footer rects.
- `screen_rows` (fn) — `(view, area) -> ScreenRows`; also `GraphScreen::screen_rows(area)`.
- `GraphScreen::graph_area`, `GraphScreen::column_boundaries` (fns) — where the graph column lands; every x where a row column ends.
- `RowLayout` (struct), `row_layout` (fn) — `body` rect and resolved `Columns` for a frame.
- `Columns` (struct) — where each trailing column of a row sits and how wide it is.
- `row_at` (fn) — `(view, area, screen_row) -> Option<(&GraphEntry, Columns)>`: the entry drawn at a screen row.
- `lanes_that_fit`, `max_lanes_for` (fns) — the most lane columns a width holds; the frame's lane cap (configured, else by width).
- `snap_to_columns` (fn) — `(rect, boundaries, bounds) -> Option<Rect>`: grows `rect` so both edges sit on column boundaries.
- `badge_spans` (fn, from `render.rs`) — the screen columns each of a row's badges occupies, as drawn.
- `badges_that_fit` (fn) — `columns / 2 + 1`: the most badges a row can draw.
- Re-exports here: `badges` and `columns` items below, and the `hints` items the footer uses.

## graph_screen::badges
- `Icons` (enum) — `Plain` (bracketed) or `Nerd` (icon-prefixed).
- `Badge` (struct) — one ref on a commit: `name`, `kind`; `Badge::text(icons, keys)` is how it reads.
- `BadgeKind` (enum) — branch, remote, tag, detached HEAD, stash, `CompareBase`, `OnlyInStash`.
- `Tracking` (enum) — untracked, in sync, or apart by ahead/behind counts.
- `separator` (fn) — what goes between two badges in an icon set.

## graph_screen::columns
- `initials` (fn) — two letters standing in for a name, for the author chip.
- `relative_date`, `relative_date_long` (fns) — `(then, now)`: `"2m"`, `"3h"`…; the same spelled for a sentence.
- `absolute_date` (fn) — a commit's date in its own offset, `"2026-09-13 14:02 +0500"`.
- `short_sha` (fn) — the first N hex characters of an object id.

## graph_screen::hints
Decides which hints the footer draws, per context; `graph_screen`'s footer draws them. Words come from the keymap, never literals.
- `HintContext` (enum) — where the cursor or focus is, for the footer's hint list; one variant per screen and state (`Picker` / `PickerOnCurrent`: the picker, its cursor off / on HEAD's branch).
- `RowVerbs` (struct) — which of a commit row's own verbs act on the cursor's row; `RowVerbs::NONE`, `RowVerbs::ALL`.
- `RowBar` (enum) — `HintContext::GraphRow`'s row kind: `Own`, `Elsewhere { branch }`, `FoldedMerge { inside }`, `OpenedMerge` (the merge row whose fold is open), `JustCommitted`, `JustMerged { inside }`.
- `ClashBar` (struct), `ClashAt` (enum) — `Clash`'s state `{ at, roles }`; `is_merge` reads `roles`; the next file is drawn by its last segment.
- `bar_name` (fn) — a branch's or path's bar form, its last path segment (`sara/fix-toc` → `fix-toc`; `hints/context/row_bar.rs`).
- `WayToBanner` (enum, `hints/context/live_op.rs`) — `Forward` / `Back`: which one press reaches the banner from the graph.
- `LiveC` (enum) — what `c` does while an operation is live: finish, continue, and the rest.
- `TabWord` (enum, `hints/context/tab_word.rs`) — `Graph`, `Diff`, `Banner`: where `⇥` lands from the changes pane; `TabWord::word`, `TabWord::ALL`.
- `ConfirmYes` (enum) — the button `y` confirms in a confirm box; `ConfirmYes::label`, `ConfirmYes::word`, `ConfirmYes::from_label`, `ConfirmYes::ALL`.
- `HintState` (enum) — `Live` / `Planned`: whether the key does the verb today.
- `Rung` (enum) — `Key` < `Short` < `Full`: how much of a hint the bar had room for.
- `HintKey` (enum) — one key a hint names, spelled from the keymap.
- `NOT_A_BINDING` (const) — the words a hint may name that are not a binding, each with why.
- `Hint` (struct) — keys, verb (`Cow`: owned when it names a branch or count), state, rung; `Hint::keeps_words`, `Hint::key_text`, `Hint::text`, `Hint::is`, `Hint::drawn_verb`.
- `MARK_RESOLVED`, `MARK_RESOLVED_SHORT` (consts) — `"mark resolved"` / `"mark"`: `space`'s verb on a conflicts row.
- `Divider` (enum) — whether a detail pane is on screen, so its divider can be dragged.
- `hints_for` (fn) — `(context, divider, columns)`: as many whole hints for `context` as fit.
- `HINT_SEPARATOR` (const), `hints_text` (fn) — `" · "`; hints as the footer draws them.
- `StopWording` (struct) — why an operation stopped, fullest wording first; `StopWording::new`, `StopWording::full`, `StopWording::before_keys`, `StopWording::is_before_keys`.
- `stopped_row`, `stopped_row_before_keys` (fns) — the bottom row while an operation is stopped: reason and bar.
- `fit_status_line` (fn) — `(reason, keys, columns)`: a `reason · keys` status line cut to fit.
- `doorway_words` (fn) — `(keys) -> Option<String>`: the bar's doorway, `? keys` as the keymap spells it.
- `blame_place_list`, `history_place_list` (fns) — the blame and history places' bars.
- `every_context`, `every_hint` (fns) — every footer context and every hint, for sweeps.


## commit_editor
- `SUBJECT_LIMIT` (const) — 72, where the subject's guide stands and past which its counter turns `Warning`.
- `BODY_LIMIT` (const) — 72, the body's guide column.
- `FILES_WIDTH` (const) — 44, the file list's width beside the changes.
- `DIFF_COLUMNS` (const) — 100; under it the changes pane is hidden and the files take the width.
- `CommitFocus` (enum) — what has focus: `Subject`, `Body`, `Files`.
- `next` (fn) — the next part `⇥` moves to: subject → body → files → subject.
- `previous` (fn) — the previous part, for `⇧⇥`.
- `types` (fn) — whether the focus is a text field (`?` is typed there, not the palette).
- `FileGroup` (enum) — `GoingIn` or `StayingOut`.
- `CommitFileRow` (struct) — one file row: path, kind letter, group, line counts, secrets mark.
- `Onto` (enum) — where the commit goes: a new commit on a branch, or an amend of a commit.
- `Summary` (enum) — the sentence above the files: what goes in (files, lines, onto, how many stay out) or nothing.
- `Refusal` (enum) — why `⏎` would do nothing: `NoMessage`, `NoFile`.
- `CommitHeader` (struct) — the top row: committing to / amending, the push state, the options that are on.
- `CommitScreenView` (struct) — everything the screen draws: header, subject/body, summary, files and cursor, the cursor file's changes, focus, refusal, status, busy, and a live operation's words.
- `CommitScreen` (struct) — `view`, `theme`, `keys`; `Widget`: the whole-area commit screen (header, message box, summary, files | changes, bar).
- `bar` (fn) — the bar's words for the focused part, keys spelled from the keymap.
- `summary_line` (fn) — the sentence above the files.

## confirm
- `ConfirmView` (struct) — Everything the confirm box draws: what is about to happen, in plain words, and the exact command that will run.
- `new` (fn) — A confirm box, cursor on `Cancel`, choices `Cancel`/`Confirm`.
- `with_max_lines` (fn) — Wrapped message rows this box may draw; `new` sets [`MAX_MESSAGE_LINES`] (4).
- `with_choices` (fn) — The same box with its own choices — `labels[0]` is always the safe one and is where the cursor starts (ADR-0038 §1): the undo-after-push box asks `Cancel` / `Undo locally` / `Re-push old tip`
- `with_labels` (fn) — A shim over the two-choice case: `[ safe ]  [ confirm ]`, where "Discard" (the old default) would name the opposite of what the button does for some callers (the `[ Stop ]` / `[ Undo ]` box).
- `left` (fn) — Moves the cursor one choice left, clamped at the first (no wrap: the safe choice is never one key-repeat away from "one past the end, wrapped onto the dangerous one").
- `right` (fn) — Moves the cursor one choice right, clamped at the last.
- `toggle` (fn) — Cycles the cursor to the next choice, wrapping past the last back to the first — the two-choice case's `Tab`.
- `selected` (fn) — The choice the cursor is on.
- `confirm_rows` (fn) — Rows the box needs to draw itself: the top border (title), the message wrapped to `width` (capped at the view's own line cap, 4 unless raised), a blank row, the command, a blank row, the choices, the bottom border.
- `confirm_area` (fn) — A box centred in `area`, sized to `view` (width capped so the command line stays readable), never larger than `area` itself.
- `confirm_area_at` (fn) — The same box at a horizontal span the caller chose rather than the centred one [`confirm_area`] computes: same vertical rule, height taken again at `width` because a wider box wraps its message into fewer lines.
- `Confirm` (struct) — The confirm box itself: a rounded-border frame on a cleared area, so it never shows the cells that were underneath it (row 484).

## conflict_resolver
- `SIDE_BY_SIDE_MIN` (const) — Columns at or above which a block's OURS/THEIRS draw side by side.
- `THREE_PANE_MIN` (const) — Frame columns at or above which the resolver is three panes beside a file-list column (ADR-0043 §1); below it, a strip above the card.
- `Arrangement` (enum) — The resolver's two arrangements (ADR-0043 §1).
- `arrangement` (fn) — Which arrangement a frame this wide gets.
- `HEADER_ROWS` (const) — Screen rows the widget reserves above the document itself: the title row and one more for the notice (blank while none is set).
- `MIN_COLUMNS` (const) — The narrowest the resolver draws its document at; below this it draws [`TOO_SMALL`] instead.
- `MIN_ROWS` (const) — The shortest the resolver draws its document at, on the same terms as [`MIN_COLUMNS`].
- `DocView` (struct) — Everything the resolver draws for one file.
- `RegionView` (enum) — One region of the document.
- `BlockView` (struct) — One conflict block.
- `BlockState` (enum) — Whether a block still has a decision to make.
- `TakeView` (enum) — Which side a taken block kept.
- `SideKind` (enum) — Which side a [`ScreenRow::Side`] row draws.
- `LabelOf` (enum) — Which section a [`SideKind::Label`] row names.
- `ResolverScreen` (struct) — The resolver, ready to render.
- `FileRow` (struct) — One file in the list.
- `FileState` (enum) — Where one listed file stands; `Solved` (a merge's file with every clash picked, not staged) reads `solved ✓`, muted.
- `FileListView` (struct) — The list, ready to draw.
- `Placement` (enum) — Where the list sits.
- `placement` (fn) — Where the list goes at this frame width: a column at [`super::THREE_PANE_MIN`] and up, else a strip as tall as it needs.
- `ResolverAreas` (struct) — The resolver's area split between the list and the document.
- `resolver_areas` (fn) — Splits `area`: a strip, one rule row, the document; or a column, one `│` column, the document at full height.
- `file_at` (fn) — The file drawn at `(dx, dy)` from the list's origin, in a list `width` x `height` cells — `None` on the label, a blank, or `… N more`.
- `FileList` (struct) — The list widget.
- `FinishRow` (struct) — The finish row's words, full and short: `✓ every file done — c review and finish the merge` / `✓ every file done — c review & finish`.
- `finish_row_at` (fn) — Whether (`col`, `row`) is on the finish row of a resolver drawn in `area` with `placement` — a click there does what `c` does.
- `ScreenRow` (enum) — One row of the document, as [`layout`] plans it.
- `Layout` (struct) — `doc` laid out at `columns`: every row the widget can draw, and where each block's header row is.
- `layout` (fn) — `doc` laid out in `arrangement` at `columns` (the document's own width).
- `row_at` (fn) — The row `top + screen_row` names, or `None` past the end.
- `SideRoles` (enum) — The role words for one operation's two sides — a closed set, so the hint bar can carry them as `&'static str` (ADR-0043 §4).
- `ours` (fn) — The ours side's role word.
- `theirs` (fn) — The theirs side's role word.
- `SideNames` (struct) — The one name each side goes by everywhere the resolver names it: role word, then what it is (`yours · main`, `picked · 3f2a1b7 Add OAuth`).
- `BlockNumbers` (struct) — This region's first line in each projection, 1-based (`graft_git::conflict::ConflictFile::first_lines`): a block's, or a plain region's (`RegionView::Plain::numbers`).
- `number_width` (fn) — Digits in the widest line number any projection reaches in `doc` — the number column's width (at least 1).
- `LineRef` (enum) — Which line a [`Cell::Line`] draws.
- `Cell` (enum) — One pane's share of a [`ScreenRow::Aligned`] row.
- `three_panes` (fn) — `doc`'s area split into its three panes, `[yours, result, theirs]`, with a one-column `│` between each two.

## context_menu
- `ContextAction` (enum) — An action a context-menu item dispatches.
- `MenuItem` (struct) — One row of the menu.
- `ContextMenu` (struct) — The popup itself: its items, an optional heading, and which item the cursor is on.
- `with_key` (fn) — `label (key)`, the key spelled from `keys` so the menu teaches the key his keymap gives `action`.
- `for_row` (fn) — A commit row's menu.
- `for_working_tree` (fn) — The uncommitted-changes row's menu: absorb first (its only verb with no key of its own), then `Open details`, then the dropped keys every menu has. Nothing `staged`: absorb stays, dim, reading `nothing staged`.
- `for_drop` (fn) — The menu a dropped branch badge opens over the row it was dropped on (`docs/design/02-keymap.md` §2.15).
- `rows` (fn) — rows the popup draws: its items, plus the title row when it has one.
- `item_at` (fn) — The item `offset` rows down from the popup's top edge, or `None` when that row is the title (which is not selectable) or past the end.
- `selected_refusal` (fn) — Why the item under the cursor cannot act, or `None` when it can.
- `move_cursor` (fn) — Move the cursor by `delta` items, clamped to the ends (the files list's own convention, `graft_ui::detail::FileListView::move_cursor` — no wrap).
- `selected` (fn) — The action the cursor is on.
- `width` (fn) — The longest label's cell width, plus 4 (design note, "Context actions").

## credential_prompt
- `MIN_COLUMNS` (const) — Narrower than this and the box does not fit — draw one line instead.
- `MIN_ROWS` (const) — Shorter than this and the box does not fit — draw one line instead.
- `CredentialView` (struct) — Everything the credential prompt draws.
- `CredentialPrompt` (struct) — [`CredentialView`] drawn into whatever `Rect` [`credential_area`] gives it.
- `credential_area` (fn) — Where the box goes in `frame`: centred, width `min(frame.width - 4, 72)`, [`BOX_ROWS`] tall, never larger than `frame`.

## empty_state
- `RecentRow` (struct) — One remembered repository as drawn.
- `EmptyPrompt` (enum) — The prompt open over the list, if any.
- `EmptyStateView` (struct) — Everything the empty state draws.
- `EmptyState` (struct) — [`EmptyStateView`] drawn over a whole frame.
- `first_launch_words` (fn) — The first-launch line fitted to `cells`: `first time? ? keys here · ⌃P anything`, keys spelled from `keys`.
- `recent_age` (fn) — How long ago, in the words the mock-up uses: `today`, `yesterday`, `3 days ago`, `last week`, `2 weeks ago`, `last month`, `5 months ago`, `last year`, `3 years ago`.
- `bar_keys` (fn) — The bar row's hints for `view`, as `(key, label)` pairs — the keys the screen answers in its current form, and only those.
- `bar_fitted` (fn) — [`bar_keys`] as drawn in `width` cells: the tour clause is the first shed when the whole bar does not fit (it is opt-in, and the actions row still names `t`); past that, the bar is cut at the edge as before.
- `CloneView` (struct) — The clone prompt as drawn (rulings R6): `url`, `cursor` (character index into `url`), `into` (the `~`-abbreviated folder), `shown` (typing, running or ended).
- `CloneShown` (enum) — Where the clone prompt's clone is, as drawn.

## finish_summary
- `FINISH_MIN` (const) — The smallest area the summary draws in, `(columns, rows)`.
- `TWO_COLUMN_MIN` (const) — From this width the file lists take the left 60 % and the message the right; below it, one column.
- `DIFF_ROWS_MAX` (const) — The most rows an expanded diff draws under its row.
- `SummaryRow` (struct) — One file row: its path and what the summary says about it.
- `SummaryFocus` (enum) — What has the keyboard in the summary.
- `ExpandedDiff` (enum) — The diff drawn under an expanded row.
- `SummaryState` (enum) — Whether the summary itself has been read.
- `FinishView` (struct) — Everything the summary draws.
- `row_count` (fn) — Rows in both lists together: what the cursor counts over.
- `FinishScreen` (struct) — [`FinishView`] drawn into the area under the header and above the bar.
- `preview_title` (fn) — The top border's words for a frame `width` cells wide.
- `preview_line` (fn) — The first line inside the frame: what lands, and that nothing does until `y` (`confirm.yes`'s key, spelled from `keys`).
- `footer_hints` (fn) — The keys the bottom border draws in a frame `width` cells wide: the `HintContext::Finish` list for the focus, fitted to the border.
- `footer_spans` (fn) — The bottom border's words as `(text, style)` spans, built from the hints themselves: the first hint's key in `strong`, everything after it in `muted`.
- `undo_line` (fn) — The second line inside the frame: `undo: u takes this back`, `app.undo` spelled from `keys`.
- `ListLine` (enum) — One line of the scrolled file list.
- `Section` (enum) — Which section heading a [`ListLine::Section`] is.
- `FinishLayout` (struct) — Where each part of the summary goes in an area.
- `fits` (fn) — Whether `area` is big enough for the summary ([`FINISH_MIN`]).
- `list_lines` (fn) — The file list, line by line, from the top: section headings, rows, and the diff under the expanded row.
- `layout` (fn) — Where each part goes in `area`, or `None` below [`FINISH_MIN`].
- `FinishHit` (enum) — What a click at `(column, row)` landed on.
- `hit` (fn) — What a click at `(column, row)` in `area` landed on, if anything the summary answers.
- `list_height` (fn) — The list's height in `area`, or 0 below [`FINISH_MIN`].
- `max_top` (fn) — The largest `top` that still fills the list: the wheel stops there.
- `top_for_cursor` (fn) — The `top` that puts the cursor row — and the diff under it, as much as fits — on screen, moving `view.top` as little as it must.

## incoming_list
The commits a merge brings in (ease-pass 10), `⏎` from the merge preview; docked where the box was.
- `IncomingRow` (struct) — `subject`, `who` (first name), `age` (graph format), `is_merge`.
- `IncomingView` (struct) — `from`, `rows` (newest first), `capped`, `withheld`, `cursor`, `ascii`; `header` / `down` / `up` / `rows_wanted` (fns).
- `IncomingList` (struct, `Widget`) — `view`, `theme`, `keys`: header, rows (cursor `▸`, scrolled into view), footer `esc back to the preview  y merge`.

## journal_screen
- `JournalScreen` (struct) — The browser itself: `view`, `theme`, `keys: &Keymap` (footer, legend, empty state and row marks are spelled from it). The caller paints the surface underneath.
- `legend` (fn) — What each glyph means, on screen for as long as the rows are (J1), with `app.undo`'s key spelled from `keys` (its first chord); unbound, the legend says it in words. The `⇢` clause is appended only when it all fits.
- `NO_TRACE` (const) — The list's last line, muted: what the list cannot show (`reflog.md` takeaway 12).
- `COMMANDS_HEADING` (const) — The heading over an expanded row's Command Log.
- `moved_heading` (fn) — the heading over an expanded row's `moved` lines: `refs the fetch moved · u does not move them back`, `app.undo` spelled from `keys`; unbound, `refs the fetch moved` alone.
- `cut_at_word` (fn) — `(text, width) -> String`: cut at the last whole word that fits, then `…`; a hard cut only when no word fits. Keeps the text's own spacing.
- `RowMark` (enum) — What a row is to `u` and `U`, in words rather than colour: `None` (default), `NextUndo`, `NextRedo`, `Interrupted`, `Undone`, `NothingWritten`; `label(keys)` draws them as `← u`, `← U` (spelled from `keys`).
- `label` (fn) — The mark as it is drawn, at the right of the row: `← u`, `← U`, the keys spelled from `keys` (`← next undo` when he left it unbound).
- `source_word` (fn) — `RowKind -> &'static str`: the source column's word, `Graft` rows `Generation::CURRENT.title()` (`Mergewright`), `Outside` rows `outside`, the `✗`/`⇢` rows `""` (ADR-0057 §10).
- `RowKind` (enum) — `Graft` (`●`), `Outside` (`$`), `Lost` (`└ ✗ lost`, a commit), `LostEdit` (`└ ✗ lost`, an edit in an undo's backup), `Replaced` (`└ ⇢ replaced`) / `Deleted` (`└ ⇢ deleted`), muted: a commit `u` brings back.
- `JournalRowView` (struct) — `age`, `description`, `summary`, `mark`, `commands` (the Command Log, drawn while expanded), `failure` (the whole reason, same), `moved`, ADR-0062: the refs a fetch or a pull's fetch moved, one line each
- `JournalStatus` (enum) — `Loading`, `Empty` (`nothing yet — every write lands here and u takes it back`), `Failed(SanitizedText)`, `Ready`.
- `JournalStatusRow` (struct) — `words`, `problem` (Warning ink when true): J's own status row, above the legend; `None` draws no row.
- `JournalFindView` (struct) — `query: SanitizedText`, `at: Option<usize>` (`None` = no match), `of: usize`.
- `JournalView` (struct) — `location`, `rows` (newest first), `selected`, `status`, `esc_cancels` (a write takes `Esc`: bar `esc cancels` … `J close`)

## keymap
Decides, never draws: which action a chord means in which place (`ContextId` scope), and how a key is spelled. Hint bars and menus draw what it spells.
- `KeyCode` (re-export) — `graft_core::event::KeyCode`, so callers name keys without `core`.
- `ContextId` (enum) — A place keys are bound in, 22 (`Global` … `Menu`); `ALL`, `config_name()` (`app`, `graph`, `operation` for `Banner`, …), `scope()` (innermost first, `Global` last), `borrows()`, `stands_down()`, `is_text_field()`.
- `ALL` (const) — Every place, `Global` first.
- `config_name` (fn) — The `[keys.<name>]` table his config writes for this place, and the prefix of every action id in it.
- `scope` (fn) — Where a key pressed in this place may still land, innermost first, `Global` last.
- `borrows` (fn) — The rows of other places this place answers itself, ahead of anything it falls through to: a key resolves in its own rows, then these, then the rest of `scope()`. Empty for `Graph`, `Global` and text fields.
- `stands_down` (fn) — In-scope rows this place answers with nothing: the `Global` rows a modal overlay swallows.
- `is_text_field` (fn) — A text field: typed keys stay as they are.
- `KeySwitches` (struct) — `suspend_on_ctrl_z`; on, `⌃Z` is `app.suspend` and undo keeps `u`.
- `ActionId` (struct) — `<context>.<verb>`; his config's API, never renamed after release.: `tour.leave` (`Esc`, context `Tour`, scope `[Tour, Global]`) — the tour's one row.
- `Binding` (struct) — `action`, `context`, `default: &[KeyChord]` (first is the one spelled), `label`, `search`.
- `KeyChord` (struct) — `code`, `ctrl`, `alt`, `shift`; normal form: a `Char` carries its case, `shift` false; Shift+Tab is `BackTab`.
- `ChordError` (struct) — `text` that names no key.
- `KeyLabel` (struct) — a spelled key (`⌃P`, `⏎`, `esc`, `PgUp`, `alt+↑`, `⇧⇥`), `—` when unbound.
- `key` (fn) — A plain character key: `KeyChord::key('m')`.
- `plain` (fn) — A key with no modifiers.
- `ctrl` (fn) — Control with a character: `KeyChord::ctrl('a')` is `⌃A`.
- `ctrl_code` (fn) — Control with a non-character key: `⌃⏎`.
- `alt` (fn) — Alt with a key: `alt+↑`.
- `QUIT_NOW` (const) — `Ctrl+C`: always quits, never remapped, never given away.
- `parse` (fn) — Read one chord from his config, or from any [`KeyChord::label`].
- `label` (fn) — The chord as the hint bar, the `.` menu and `?` spell it today.
- `from_press` (fn) — The chord a press is, or `None` when Command / Super is held: no binding uses it, and the terminal or the OS owns those keys.
- `to_press` (fn) — The press a dispatcher matches for this chord: the shape the input decoder delivers it in, so a translated default acts exactly as pressing that default does.
- `Translated` (enum) — `Default(KeyChord)` / `Unbound` / `Pass`.
- `Keymap` (struct) — [`defaults()`] with his overrides applied, resolved per context. `Default` is the table as shipped (no overrides, switches off); `with_overrides` builds the rest. `Clone + Debug + PartialEq + Eq`.
- `with_overrides` (fn) — Apply his `[keys.<context>]` lines in his file's order; an override replaces the action's whole default list. A bad line is not applied and says why (a `KeyWarning`). `switches` apply first.
- `translate` (fn) — What the app hands the panes for `pressed` in `ctx` (module docs of [`Translated`]).
- `shipped` (fn) — The table as shipped, built once: what a text field spells (its keys pass untranslated, so they do what their defaults do) and what tests spell against.
- `chords` (fn) — The action's chords as he has them; empty when unbound or unknown.
- `reachable` (fn) — Every row a key pressed in `ctx` may land on, in the order a chord resolves there: its own rows, then [`ContextId::borrows`], then the rest of its scope (D1).
- `in_context` (fn) — The context's own rows, in table order, with their chords as he has them.
- `acting` (fn) — The action `chord` acts as when pressed in `ctx`, his lines applied: the palette's per-place spelling (after `merge = "j"`, `j` on the graph is merge, not move). `None` when no row a key there reaches holds it.
- `spell` (fn) — The action's key as `?` prints it: its first live chord, or `—` when he unbound it (or it is not an action, or every chord it had was taken by another in a collision).
- `spell_keys` (fn) — The hint bar's and the footers' spelling of several actions: each one's first live chord, joined with `/`, an unbound action left out (`j/k` for `app.down`, `app.up`).
- `spell_every` (fn) — Every live chord of one action, joined with `/` (`⏎/space` for `finish.toggle_diff`); `None` when it has none.
- `key_verb` (fn) — A footer's `<keys> <verb>` (`esc back`, `j/k move`): the keys as [`Keymap::spell_keys`] spells them, then the verb.
- `key_verbs` (fn) — A footer's `<key> <verb>` pairs, two spaces apart, each spelled by [`Keymap::key_verb`] from one action id, an unbound one left out (`⏎ apply  p pop  esc close`).
- `after` (fn) — `words` with ` · <key> <verb>` after it (`finish needs 60x20 · esc back`), or `words` alone when the key is unbound.
- `live_labels` (fn) — The action's live chords, spelled: every chord it holds, less any another action of its own context took in a collision, and less any a collision anywhere names it as losing D7: after `merge = "j"`
- `defaults` (fn) — the table, grouped by context in `ContextId::ALL` order (§2.6 and §2.10 rows in `keymap/table/`, joined in place). Adding a row edits `tests/keymap_ids.txt`.
- `KeyWarning` (enum) — One problem with one override line: `UnknownAction`, `BadKey`, `Collision { line, places: Vec<Clash> }` (every place a key changed hands), `Reserved`, `Fixed`, `SwitchOff`, `Follows { action, leader }`.
- `Clash` (struct) — `key`, `context`, `kept`, `lost`, `keeps: Option<KeyChord>` (the loser's key left there, for the words).
- `words` (fn) — The problem in one lowercase line, keys spelled as the app spells them.

## map_screen
- `MapMarkView` (enum) — `Tag(SanitizedText)` or `Radar(Severity)`: a mark on one bucket, sanitised for drawing.
- `MapRowView` (struct) — one branch's row: `name`, `identity`, `counts`, `is_current`, `merged_into`, `marks`, `active`, `complete`.
- `MapStatus` (enum) — `Loading { done, total }`, `Empty`, `Failed(SanitizedText)`, `Ready`: what draws instead of, or with, the rows.
- `MapView` (struct) — everything the screen draws: `title_branch`, `span_days`, `rows`, `max_count`, `buckets`, `selected`, `more`, `complete`, `status`, `ascii`, `esc_cancels` (a write takes `Esc`: footer `esc cancels` … `M back`).
- `MapScreen` (struct) — `view`, `theme`, `keys: &Keymap`: the footer (`j/k select  ⏎ zoom to commits  M/esc back`, loading `esc cancels`) is spelled from it.

## name_prompt
- `MIN_COLUMNS` (const) — Narrower than this and the box does not fit — draw one line instead.
- `MIN_ROWS` (const) — Shorter than this and the box does not fit — draw one line instead.
- `NamePromptView` (struct) — Everything the name prompt draws.
- `NamePrompt` (struct) — [`NamePromptView`] drawn into whatever `Rect` [`name_prompt_area`] gives it.
- `name_prompt_area` (fn) — Where the box goes in `frame`: centred, width `min(frame.width - 4, 60)`, [`BOX_ROWS`] tall, never larger than `frame`.

## overview_strip
- `StripMark` (enum) — `Merge`, `Radar(Severity)`, `SearchHit`, `Head`: one thing worth marking about a row.
- `StripMarks` (struct) — `rows_total`, `viewport`, `marks: Vec<(usize, StripMark)>` — what the widget draws from.
- `StripCell` (enum) — `Blank`, `Rail`, `Viewport`, `Merge`, `Head`, `SearchHit`, `Radar(Severity)`: one cell, already resolved to a priority.
- `cell_for` (fn) — the highest-priority mark in `cell`'s bucket; blank once the bucket starts past `rows_total`.
- `row_for_cell` (fn) — the first row of `cell`'s bucket, for the click.
- `OverviewStrip` (struct) — `marks`, `theme`, `ascii`; `Widget::render` draws one glyph per cell, Unicode or ASCII.

## paint
- `color` (fn) — resolve a `TokenColor` to a ratatui `Color`.
- `foreground` (fn) — a style that paints text in one of the theme's tokens.
- `on_selection` (fn) — `style` as drawn on the `selection` fill: its foreground lifted to stay legible there ([`Theme::on_selection`], ADR-0058 §3).
- `lift_on_selection` (fn) — `on_selection` over every cell of a rect already drawn (map row, rewrite-editor band: pieces drawn by several helpers).

## palette
- `MAX_WIDTH` (const) — The widest the box gets (§1.16).
- `WIDE_FROM` (const) — Below this many frame columns the box takes the frame's full width and rows drop their config id (§1.16).
- `NOTHING_MATCHES` (const) — The one line drawn when the query keeps no row; the key that closes is spelled after it (`nothing matches · esc close`).
- `PaletteMode` (enum) — Which way the palette was opened.
- `source_word` (fn) — `RowKind -> &'static str`: the source column's word, `Graft` rows `Generation::CURRENT.title()` (`Mergewright`), `Outside` rows `outside`, the `✗`/`⇢` rows `""` (ADR-0057 §10).
- `RowKind` (enum) — What a row stands for: `Action`, `App`, `Branch`, `Tag`, `File`, `Commit`, `Recent` (drawn as an action row), `More` (a find group's muted `+N more` / `reading…` line).
- `PaletteRow` (struct) — One row, worded.
- `PaletteView` (struct) — Everything the box draws.
- `row_count` (fn) — How many rows there are, groups flattened.
- `palette_area` (fn) — The most the box can take: over the focused pane `over`, at most [`MAX_WIDTH`] wide and centred on it; the full width of `frame` when `frame` is narrower than [`WIDE_FROM`].
- `palette_row_at` (fn) — The row a click at (`column`, `row`) landed on, in a box drawn in `area` (from [`palette_area`]): `None` on a border, the query, a header or below the last row.
- `Palette` (struct) — The palette as a widget: draws `view` into the area it is given (from [`palette_area`]), clearing only the box it takes.

## picker
- `TITLE` (const) — The box's title, in its top border.
- `PickerKind` (enum) — Which kind of branch a row is: its own glyph and word.
- `glyph` (fn) — The glyph, as the narrow layout draws it alone.
- `label` (fn) — Glyph and word, as the wide layout draws them.
- `PickerRow` (struct) — One branch row, worded.
- `PickerBody` (enum) — What stands under the query.
- `PickerView` (struct) — Everything the box draws.
- `picker_area` (fn) — Where the box goes over `graph`: as wide as `graph` less 4, at most [`MAX_WIDTH`], centred; the lower 60% of it, so the graph above stays in view.
- `picker_box` (fn) — [`picker_area`] in plain numbers ([`Cells`]), for a caller that must not name the rendering library (`graft::update::geometry` mirrors the frame so a click can find the row it landed on).
- `first_drawn` (fn) — The first row drawn in a box `height` rows tall with `selected` under the cursor: the list scrolls only as far as keeps the cursor in view.
- `row_at` (fn) — The row a click `line` rows below the box's top border landed on, in a box `height` rows tall showing `count` rows with `selected` under the cursor.
- `draw_picker` (fn) — Draws `view` into `area` (from [`picker_area`]), clearing it first.
- `render_picker` (fn) — [`draw_picker`] into a bare buffer.

## preview_box
- `PREVIEW_MIN` (const) — Below this width or this height **the screen** does not fit the box: "a picture that cannot be seen is never confirmed" (design note).
- `MIN_BOX_ROWS` (const) — The fewest rows a box can be: the title border, one content row, the choices row, the footer border.
- `PreviewBox` (struct) — Everything the Ghost Preview box draws.
- `new` (fn) — An empty box titled `title`, choices `Cancel`/`Confirm`, cursor on `Cancel` — the only way to build one, so the safe choice is first and under the cursor in every preview, whatever the plan does (ADR-0038 §1).
- `with_choices` (fn) — The same box with its own choices — `labels[0]` is the safe one and is where the cursor goes back to (ADR-0038 §1).
- `with_merge` (fn) — fills the box's `merge` field: no lines, no choices (keys only), footer `y merge  esc cancel  ⏎ see the N commits`. Its `incoming` field draws the incoming list in the box's place.
- `MergeParts` (struct) — what the merge box says, in counts: incoming commits and who, files changed, clashes with places, his set-aside `edits`; `not_checkable` or `withheld` replace the counts.
- `choices` (fn) — The choices, left to right.
- `selected` (fn) — The choice the cursor is on.
- `left` (fn) — Moves the cursor one choice left, clamped at the first (no wrap).
- `right` (fn) — Moves the cursor one choice right, clamped at the last (no wrap).
- `preview_rows` (fn) — Rows the box takes at `width` — the caller's row budget for docking it above the footer, without the footer row it must never cover.
- `preview_box_area` (fn) — Where the box goes in `frame`: docked to the bottom, at its own height ([`preview_rows`]), with the frame's last row left to the screen footer — and, from 30 rows
- `dock_cells` (fn) — [`preview_box_area`] in plain numbers, for a caller that must not name the rendering library (`graft::update` hit-tests the box's buttons against it).
- `PreviewBoxWidget` (struct) — [`PreviewBox`] drawn into whatever `Rect` the caller docks it to — bottom of the graph area, never over the footer row.
- `set_aside_row` (fn) — The set-aside row a rebase or checkout over a dirty tree adds under `working tree:` (ADR-0045 §4, amended 2026-09-26 b): `files` tracked files set aside, wordings longest first for [`fitting_line`].
- `fitting_line` (fn) — The longest of `wordings` (longest first) that fits the box's text column at `width` columns — the borders and their one-cell gutters taken off.
- `Again` (struct) — The footer's verb slot: which key confirms, and the word after it.
- `confirm` (fn) — `Again`'s confirm slot: `confirm.yes`'s first live chord, then `verb` (`y apply`).
- `opening` (fn) — The opening key again: `action` is `preview.confirm_merge` or `preview.confirm_rebase` (`m merge`).
- `text` (fn) — `<key> <verb>` spelled from `keys`; `None` when the action has no live chord.

## rewrite_editor
- `WIDE_MIN` (const) — From this width the AFTER side stands right of the rows, past a `│`.
- `NARROW_BELOW` (const) — Below this width the AFTER side is a card `Tab` swaps to.
- `LEFT_WIDTH` (const) — The rows' width at [`WIDE_MIN`] and above; the `│` is the next column.
- `SHORT_HEIGHT` (const) — At this many area rows or fewer the message shows [`SHORT_MESSAGE_LINES`] lines and `…`.
- `SHORT_MESSAGE_LINES` (const) — The message lines shown on a short screen.
- `draw_rewrite_editor` (fn) — Draw the editor into `area` (the whole frame but the hint bar).
- `RewriteEditorWidget` (struct) — The editor as a widget, for [`draw_rewrite_editor`] and tests.
- `rewrite_row_at` (fn) — The row (index into `view.rows`) drawn at `(column, row)`, as [`draw_rewrite_editor`] laid it out in `area`; `None` off the rows (the field's line included).
- `RewriteEditorView` (struct) — Everything the editor draws.
- `RowMark` (enum) — A row's action, which picks its word and glyph: `Pick`, `Reword` (`✎`), `Edit` (`⏸`), `Squash` (`↓`), `Fixup` (`↓`), `Drop` (`✗`, subject muted), `Base { label }` (the row under his, not editable).
- `EditorRow` (struct) — One row on the left.
- `FieldView` (struct) — `r`'s one-line field: `row` (the row it edits; the field is the line under it), `text`, `caret` (character index).
- `MessageBlock` (struct) — The message block.
- `RightSide` (enum) — What the right-hand side shows.
- `AfterView` (enum) — The AFTER plan's state.
- `AfterFact` (struct) — One fact under the AFTER rows, and how loud it is drawn.
- `plain` (fn) — A fact in the plain `Text` tone.
- `FactTone` (enum) — How an [`AfterFact`] is drawn.
- `AfterRow` (struct) — One AFTER row.
- `AfterMark` (enum) — An AFTER row's mark.
- `ChangesView` (enum) — `⏎`'s file list.

## stash_screen
- `StashRowView` (struct) — one entry's row: `name` (`stash@{0}`), `message` (quoted, first line), `age`, `files`.
- `StashPromptView` (struct) — the save prompt: `title`, `message` (a `text_input::FieldView`), `staged_only`, `refusal` (what `s` would refuse, shown before `⏎` is pressed).
- `StashStatus` (enum) — `Loading`, `Empty` (`no stashes — s stashes the working tree`), `Failed(SanitizedText)`, `Ready`.
- `StashView` (struct) — `rows` (newest first), `selected`, `preview` (the selected stash's `detail::FileListView`, drawn under the list), `prompt`, `status`, `notice`/`notice_is_failure`, `esc_cancels` (a write has `Esc`)
- `StashScreen` (struct) — `view`, `theme`, `keys`: footer and empty line spelled from his keymap; the save prompt, a text field, spells from `Keymap::shipped()` (`⏎ save  ⇥ scope  esc back`).
- `files_label` (fn) — `1 file` / `3 files`: the count with the noun it agrees with.

## status_screen
- `StatusView` (struct) — everything the status screen draws.
- `HistoryView` (struct) — how far the history walk has got.
- `HistoryState` (enum) — walk state; worded so "fine" never reads as an error.
- `RepositoryView` (struct) — the repository Graft opened.
- `HeadView` (enum) — where `HEAD` points, worded for a person.
- `RenderingView` (struct) — the tier/theme as the status screen shows them, with provenance.
- `TierSource` (enum) — where the rendering tier came from.
- `StatusScreen` (struct) — the status screen, ready to render.
- `new` (fn) — Draw `view` in `theme`.

## still_to_apply
- `TITLE` (const) — The box's title, in its top border.
- `MAX_ROWS` (const) — Pick rows shown before the rest is counted as `+N more`.
- `PendingRowView` (struct) — One pending todo line.
- `PendingView` (struct) — What the block draws.
- `is_empty` (fn) — Nothing pending: no block.
- `still_to_apply_height` (fn) — Rows the whole block wants: two borders plus its content.
- `still_to_apply_lines` (fn) — The block as plain text rows, exactly `area` sized (rows beyond the box are not produced).
- `draw_still_to_apply` (fn) — Draws the block into `area` (the rows between the banner and the graph).

## syntax
- `SyntaxRole` (enum) — a highlighted token's category: `Plain`, `Comment`, `String`, `Keyword`, `Type`, `Function`, `Number`, `Punctuation`, `Attribute`.
- `Language` (struct) — a resolved `syntect` syntax; opaque.
- `language_for_path` (fn) — the language a path's extension or exact file name (`Makefile`, `Dockerfile`) implies.
- `language_for_first_line` (fn) — the language a shebang or modeline implies, for extension-less files.
- `SyntaxSpan` (struct) — one highlighted run: a half-open, ascending, non-overlapping **char** range and its `SyntaxRole`.
- `highlight` (fn) — highlight every line of a file in one language; parser state carries across lines; past 5,000 lines or a 4,000+ char line, every line comes back `Plain`; never panics.

## text_input
- `GUIDE` (const) — `┊`, the length-guide glyph (column 50 for a subject, 72 for a body).
- `FOCUS` (const) — The glyph marking whatever has focus.
- `TextField` (struct) — One line of editable text (the commit subject, and later the stash message): `new`, `text`, `cursor`, `len`, `is_empty`, `insert`, `insert_str` (a bracketed paste), and cursor moves.
- `new` (fn) — A field holding `text`, with the cursor at its end — where someone prefilled a message would carry on typing.
- `text` (fn) — What has been typed.
- `cursor` (fn) — The cursor's character index.
- `len` (fn) — How many characters long the text is — what the `n/50` counter counts.
- `is_empty` (fn) — Nothing typed.
- `insert` (fn) — Type one character at the cursor.
- `insert_str` (fn) — Type a run of characters — a bracketed paste, which lands exactly as if it had been typed.
- `backspace` (fn) — Delete the character before the cursor.
- `delete` (fn) — Delete the character under the cursor.
- `left` (fn) — One character left.
- `right` (fn) — One character right.
- `home` (fn) — To the start of the line.
- `end` (fn) — To the end of the line.
- `word_left` (fn) — To the start of the word the cursor is in, or of the one before it.
- `word_right` (fn) — To the start of the next word, or the end of the line.
- `SecretField` (struct) — The credential prompt's typed password or passphrase — never a plain `String`.
- `clear` (fn) — Empties the field (`Ctrl+U`) — zeroes the whole capacity in place, not just the live length, and keeps the allocation.
- `pulse` (fn) — The one bit the masked row shows: it flips on **every** edit — insert, paste (once per character), backspace, delete, `Ctrl+U` — so each keystroke visibly registers, which is the first half of ADR-0038 §2.
- `into_bytes` (fn) — The only way out: what was typed, as UTF-8 bytes, in a buffer that zeroes on drop.
- `TextArea` (struct) — several editable lines, always at least one.
- `lines` (fn) — The lines as they are drawn.
- `newline` (fn) — Break the line at the cursor.
- `up` (fn) — One line up, keeping the column where the shorter line allows.
- `down` (fn) — One line down, keeping the column where the shorter line allows.
- `append_line` (fn) — Put `line` on a line of its own at the end, and leave the cursor at the end of it — what `Ctrl+Shift+C` does with a co-author trailer.
- `FieldView` (struct) — `label`, `text`, `cursor` (character index), `guide_at`, `counter` (`(typed, limit)`; drawn in `Warning` past the limit), `focused`.
- `SECRET_GLYPHS` (const) — How many glyphs a secret field draws, whatever the real typed length (ADR-0038 §2, m4-07 review Minor 9).
- `display_text` (fn) — What a field actually draws for its text: the text itself, or [`SECRET_GLYPHS`] mask glyphs — one of them flipped to the pulse glyph — while `secret` is `Some` — the single place that decision is made
- `FieldWidget` (struct) — `view`, `theme`; `Widget` over one row: label, text, guide, right-aligned counter.
- `LABEL_WIDTH` (const) — cells the `Subject`/`Body` label column takes, so both text regions start in the same column.
- `AreaView` (struct) — `label`, `lines`, `cursor` (`(line, character)`), `guide_at`, `focused`.
- `AreaWidget` (struct) — `view`, `theme`; `Widget` over the rows it is given: the label row, then as many lines as fit, scrolled so the cursor's line is always one of them.

## theme
- `TINT_ALPHA` (const) — 0.30, the one α every conflict tint is mixed at (ADR-0044 §1).
- `is_tint` (fn) — true for the three conflict tints (backgrounds).
- `tint` (fn) — a tint token's background, `None` at 16 colours, under `NO_COLOR` and in high contrast; widgets ask this, never the depth.
- `mix` (fn) — `fg` over `bg` at `alpha`, per channel, rounded; ties the tint literals to the rule.
- `contrast` (fn) — WCAG 2.x ratio of two `Rgb`s, `(L1+0.05)/(L2+0.05)`, symmetric (`theme/contrast.rs`).
- `ContrastRule` (struct) — one floor: `fg` token on `bg` token at `min`:1.
- `ContrastMiss` (struct) — a floor missed: `rule` and the `ratio` reached. `words()` → `selection on bg 1.0:1 — under 3.0`, ratio floored to 0.1 (never rounds up to its floor).
- `words` (fn) — `selection on bg 1.0:1 — under 3.0`.
- `CONTRAST_RULES` (const) — the seven floors every theme passes: text/bg 4.5, text/selection 4.5, selection/bg 3.0, text-muted/bg 4.5, role-this/under/trunk on bg 3.0.
- `legible_on` (fn) — `fg` unchanged if ≥ 4.5:1 on `bg`, else mixed toward white (dark bg) / black (light bg) in 5% steps until it is; target after 20 steps.
- `on_selection` (fn) — `fg` as drawn on this theme's `selection` fill, at this terminal's depth — what every widget calls for text it draws on the cursor row.
- `TOKEN_COUNT` (const) — 22: PROMPT.md §5's twelve, the six conflict roles, the four graph roles.
- `LANE_COUNT` (const) — number of lane hues; lanes beyond reuse them.
- `Token` (enum) — a chrome colour role; ends with the conflict roles `Ours`, `Theirs`, `Open`, `OursTint`, `TheirsTint`, `OpenTint` (ADR-0044 §1), then the graph roles `RoleThis`, `RoleUnder`, `RoleTrunk`, `RoleOther` (ADR-0063).
- `ALL` (const) — every token, in PROMPT.md §5's order.
- `name` (fn) — The token's name as PROMPT.md §5 and the config file spell it.
- `ThemeKind` (enum) — dark, light, or high contrast (override only).
- `select` (fn) — resolve theme from an override and a terminal hint; unknown → dark.
- `TokenColor` (enum) — a resolved colour, `NO_COLOR`-aware.
- `Theme` (struct) — a resolved theme; no path hands a widget a colour the terminal can't show.
- `new` (fn) — Resolve `kind` against what the terminal can do.
- `kind` (fn) — which theme this is (diagnostics, config round-trips).
- `color_depth` (fn) — How much colour this terminal has, or [`ColorDepth::None`] if `NO_COLOR` was set — the same reduction [`Theme::new`] applied to every token.
- `token` (fn) — the colour for one chrome token. Conflict tokens use private `pinned` (fixed 256/16 entries, dark and high contrast) then `depth::nearest_256_cube` (16–231, never the grey ramp); a dropped tint is `TokenColor::Default`.
- `lane` (fn) — the colour for one graph lane; shape/dash separate same-hue lanes.
- `lane_muted` (fn, lane methods live in `theme/lanes.rs`) — a lane's hue mixed 45% toward `bg` for off-thread cells at truecolor/256; the lane's own colour at 16/none (caller keeps DIM).
- `branch_muted` (fn) — `lane_muted` by `BranchIdentity`.
- `fades_by_hue` (fn) — truecolor or 256: off-thread lanes fade by hue, not DIM.
- `raster_lanes` (fn) — truecolor lane hues for the pixel tier at any depth; one muted hue when colour is off.
- `raster_muted` (fn) — the unreduced `text-muted` hue (his override if any, from the `rgb` table), for a pixel `+N` marker.
- `branch` (fn) — the colour a branch identity draws in, independent of column.
- `syntax` (fn) — The colour for one [`SyntaxRole`], from the same tokens as everything else — a diff's syntax colouring is chrome, not a second palette.
- `with_overrides` (fn) — the constructor: `kind`'s built-in table with `[theme.<kind>]` colours written over it (kept privately as `rgb`), each token reduced from that table.
- `from_name` (fn) — inverse of `Token::name` over `Token::ALL`; `lane-N` and typos → `None`.
- `contrast_misses` (fn) — `(ThemeKind, &[(Token, Rgb)]) -> Vec<ContrastMiss>`: pure, on the pre-reduction table (built-ins with overrides written over), `CONTRAST_RULES` order; empty for every built-in (`theme/overrides.rs`).

## tour_card
- `TOUR_CARD_ROWS` (const) — Rows the card takes: its two borders and the line between.
- `TICKS_FROM` (const) — Narrower than this, the ticks are not drawn (the note: "< 100 cols drops the ticks").
- `TourCardView` (struct) — What the card says.
- `TourCard` (struct) — [`TourCardView`] drawn into its rows.
- `ticks_text` (fn) — The ticks as one string: `✓ stage ✓ commit · undo stash …` — the done ones marked, a `·` where the done run ends.
- `card_lines` (fn) — The three rows as text, `width` cells each: what [`TourCard`] draws, and what a test reads.

## whitespace_tail
- `whitespace_tail` (fn) — `whitespace_tail(ignored, default_ignored)`: the title tail for blame, detail and history.
