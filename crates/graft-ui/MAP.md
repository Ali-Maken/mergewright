# graft-ui — map
One line per public item. Update when a public item changes.

## lib.rs
- `diff_words` (mod) — word-level highlighting inside a paired diff line.
- `fit` (mod) — clamp text to an exact cell width.
- `graph_column` (mod) — the Unicode-tier graph column widget.
- `context_menu` (mod) — the `.`/right-click popup: `ContextAction`, `MenuItem`, `ContextMenu` (`for_row`, `for_working_tree`, `move_cursor` clamped, `selected`, `width`); no theme carried, so its `Widget` impl draws only labels and the reversed cursor row, the background painted by the caller (m2-details-and-diffs/12).
- `confirm` (mod) — the discard confirm box: `Choice` (`Cancel`/`Confirm`), `ConfirmView` (`new(title, message, command)` — opens with `Cancel` under the cursor, the safe default; `toggle`; `with_labels(safe, confirm)` renames the two buttons through the `safe_label`/`confirm_label` fields, for boxes where "Discard" names the opposite of what the button does — the undo drift guard asks `[ Stop ]` / `[ Undo ]`, m3-working-tree-undo-and-stash/06), `Confirm<'a>` widget (no theme carried, like `context_menu`: draws the title bold, the consequence in plain words, the exact `git` command it will run, then `[ Cancel ]   Discard` buttons), `confirm_area`/`confirm_rows` centre-and-clamp sizing helpers (never exceed the frame) (m3-working-tree-undo-and-stash/04).
- `commit_editor` (mod) — the commit/amend editor screen: `Field`, `CommitEditorView`, `CommitEditorScreen`, `fits`, `SUBJECT_LIMIT`/`BODY_LIMIT` (m3-working-tree-undo-and-stash/05).
- `detail` (mod) — the detail pane's view-model types, no drawing (m2-details-and-diffs/06).
- `detail_pane` (mod) — the detail pane's layout and widget: breakpoints, regions, plain diff body (m2-details-and-diffs/07).
- `graph_screen` (mod) — the graph screen Graft opens on: header, rows, key hints (every graph key, F3 included; narrow widths drop whole hints from the right, m1-the-graph/33; `y sha` added in 45; `S stash` for the overlay and `H hide stash` for the graph toggle, m3/06 review M8).
- `graph_screen::badges` (mod) — ref badges: kinds, tracking state, plain/Nerd Font text.
- `graph_screen::columns` (mod) — pure formatting for the author initials, relative date and short SHA.
- `journal_screen` (mod) — the undo journal browser (`J`): `RowMark`/`JournalRowView`/`JournalStatus`/`JournalView`/`JournalScreen` (m3-working-tree-undo-and-stash/06).
- `map_screen` (mod) — the Map view widget: `MapView`/`MapRowView`/`MapMarkView`/`MapStatus`/`MapScreen`, `ramp_glyph` (m2-details-and-diffs/19).
- `overview_strip` (mod) — the one-column whole-history strip: `StripMark`/`StripMarks`/`StripCell`, `cell_for`/`row_for_cell`, `OverviewStrip` widget (m2-details-and-diffs/20).
- `paint` (mod) — token-to-style helpers for widgets.
- `stash_screen` (mod) — the stash overlay (`S`): `StashRowView`/`StashPromptView`/`StashStatus`/`StashView`/`StashScreen`, `files_label` (m3-working-tree-undo-and-stash/06).
- `status_screen` (mod) — the status screen view and widget.
- `syntax` (mod) — line-level syntax highlighting (`syntect`) over sanitised text.
- `text_input` (mod) — the commit editor's (and later the stash prompt's) text entry: `TextField`, `TextArea`, `FieldView`/`FieldWidget`, `AreaView`/`AreaWidget` (m3-working-tree-undo-and-stash/05).
- `theme` (mod) — theme kinds, tokens, and colour resolution.

## detail
View-model types for the detail pane (m2-details-and-diffs/06); no widget. Every string is `SanitizedText`.
- `Subject` (enum) — `None`, `Commit { sha }`, `Compare { base, head }`, `WorkingTree { staged, unstaged, untracked }`.
- `DetailHeader` (struct) — subject, summary, author, dates, committer, co-authors, parents, against, contained, radar lines.
- `ContainedIn` (enum) — `NotAsked`, `Checking`, `Known { branches, tags, more }`, `Failed`.
- `RadarLine` (enum) — a finding (`Severity`, title) or the evil-merge / force-push states, including `EvilResolved { files }` ("evil merge: clean (N files hand-resolved)").
- `FileStatusView` (enum) — added, deleted, modified, renamed, copied, type-changed, unmerged, untracked.
- `FileRowKind` (enum) — `Message { lines }`, `Dir { collapsed }`, `File { status, added, removed }`.
- `FileRow` (struct) — depth, name, kind, `entry` (index into the git list; `None` for message/dir).
- `FileListView` (struct) — rows, cursor, top, totals, `message` (the `✉` row's body lines).
- `FileListView::visible` (fn) — rows not under a collapsed dir, with their indices.
- `FileListView::toggle_dir` (fn) — collapse or expand a dir row.
- `FileListView::move_cursor` (fn) — move among visible rows, clamped.
- `FileListView::selected_entry` (fn) — the cursor row's git entry, if a file.
- `LineKindView` (enum) — `Context`, `Added`, `Removed`, `HunkHeading`.
- `LineView` (struct) — kind, old/new numbers, text, word ranges, `SyntaxSpan`s, `pair` (index of its partner line).
- `HunkView` (struct) — heading and lines.
- `DiffMode` (enum) — `Unified`, `SideBySide`.
- `DiffOptionsView` (struct) — context, ignore_whitespace, word_tint; `Default`.
- `DiffView` (struct) — path, old path, status, hunks, `binary`, `truncated`, `untracked`.
- `BodyView` (enum) — `Empty`, `Message`, `Loading`, `Failed`, `Diff(Arc<DiffView>)`.
- `DetailFocus` (enum) — `Graph`, `Files`, `Diff`.
- `DetailView` (struct) — everything the pane draws: header, files, body, body_top, focus, mode, options, explain.
- `ExplainView` (struct) — the explain panel's title, severity, paragraphs, sketch (built by `explain-panel`).

## detail_pane
Draws a `detail::DetailView` beside, below or in place of the graph (m2-details-and-diffs/07). No I/O; `Theme` is the only extra it needs over the ticket's literal signature (undocumented deviation, see module doc). Split into files, public items unchanged (m2 review M8): `layout.rs` (breakpoints, `DetailAreas`), `line_render.rs` (row-drawing helpers shared by both diff bodies), `unified.rs` (`diff_body`), `side_by_side.rs` (`diff_body_side_by_side`), `header.rs`, `files.rs`, `body.rs` (`DetailPane` widget, Explain panel).
- `DetailLayout` (enum) — `Right { columns }` (160+ cols), `Below { rows }` (100-159), `Tab { shown }` (<100).
- `DetailLayout::compute` (fn) — picks the breakpoint for a size and focus; `override_cells` replaces the percentage, still clamped to the minimum.
- `split_area(layout, area)` (fn) — `area` split into the graph rect and the pane rect for `layout` (`None` for whichever side `Tab` hides); pure geometry, shared by `view/frame.rs`'s draw and mirrored (plain `u16` math, not this fn) by `update/mod.rs`'s mouse hit-testing, which cannot depend on this crate's rendering library (`detail-resize-and-persistence`).
- `DetailAreas` (struct) — `header`, `files`, `body` rects for one layout.
- `detail_areas` (fn) — splits a pane rect into `DetailAreas`, capping the files region to what `files_count` needs.
- `DiffFocus { focused, word_tint, cursor: Option<usize> }` (struct) — `diff_body`'s two independent flags bundled into one param to stay under clippy's 7-argument limit; `word_tint` actually gates the tint now (m2-details-and-diffs review M1: it used to be read nowhere).
- `diff_body(hunks, top, truncated, area, buf, theme, focus: DiffFocus)` (fn) — full diff body: right-aligned old/new gutter (width = widest number, min 3), sign cell, soft `Surface` bg on added/removed rows, stronger `Added`/`Removed` bg on word-tinted ranges when `focus.word_tint` (fg forced to `Bg` for contrast), syntax-role fg elsewhere regardless of `focus.focused` (review M2: body colour no longer depends on focus; only the gutter does — accent when focused, muted otherwise), hunk heading pinned to the top row while scrolling within its hunk — **except when `focus.cursor` is the row at `top`**, since the pin takes that row's slot and `Space`/`d` must never act on a line that is not drawn (m3/04 review C1) — trailing `␍` glyph, and a muted "… N more lines · L load all" footer row when `truncated: Some((shown, total))`. At `Palette16`/`None`: no backgrounds/tint, sign+bold(added)/dim(removed) only. Returns total content rows (real rows + footer) for scroll clamping. Deviation: adds a `truncated: Option<(usize, usize)>` param beyond the ticket's literal signature — reported as a spec gap, not silently worked around (see module doc).
- `diff_body_side_by_side(hunks, top, area, buf, theme, focused)` (fn) — the `v`-toggled two-column diff over the same `HunkView` data: old numbers/text left, new right, `│` rule between (plain at `Palette16`/`None`); `LineView::pair` keeps a removed/added run aligned row for row, an unpaired line leaves the other half blank (never shifted); hunk heading pins full-width above both halves; word tint, syntax fg, `␍`, 16-colour fallback match `diff_body` (m2-details-and-diffs/09).
- `DetailPane` (struct, `Widget`) — `view`, `layout`, `theme`; draws header, files and body for one frame. When `view.explain` is `Some`, the files+body area (`detail_areas().files.union(.body)`) draws the Explain panel instead, at every breakpoint: severity glyph + title, wrapped paragraphs, a blank line, the sketch verbatim, `Esc close` (m2-details-and-diffs/17).

## diff_words
- `WordSpans` (struct) — `removed`/`added`: per-line char ranges that changed against the paired line on the other side.
- `word_spans` (fn) — word-diff (`similar`) `removed[i]` against `added[i]`; extra lines get empty `Vec`s; a pair sharing no word gets no spans on either side.

## fit
- `MAX_MARKS` (const) — combining marks kept per base character.
- `fit` (fn) — text laid into exactly N cells: padded, or cut with `…`; wide chars never split.

## graph_column
- `CELLS_PER_LANE` (const) — cells per lane: its own, then a gap.
- `NodeKind` (enum) — node shape: commit, merge, root, HEAD, stash, uncommitted.
- `NodeKind::of` (fn) — commit/merge/root from a row's parents.
- `NodeKind::glyph` (fn) — the node's character.
- `RowPaint` (enum) — normal, dimmed, highlighted.
- `GraphLine` (struct) — `row` (borrowed or synthesised), `node`, `paint`, `thread` (real columns on the selection's thread: bold; other lanes dim; `None` = no selection) for one drawn row.
- `GraphLine::plain` (fn) — a row drawn normally, borrowing a real `Row`.
- `GraphLine::owned` (fn) — a row drawn normally, owning a synthesised `Row` (the uncommitted-changes row).
- `GraphColumn` (struct) — widget drawing consecutive rows; fills its area exactly. At 16 colours and `NO_COLOR`, odd-column lanes draw dashed (`╎`, `╌`); at the ASCII tier, every glyph collapses onto `* | / \ - +` instead (m1-the-graph/19).
- `GraphColumn::new` (fn) — draw lines in a theme.
- `GraphColumn::ascii` (fn) — draw with the ASCII glyph table instead of Unicode box-drawing.
- `GraphColumn::width_for` (fn) — cells for N lanes.
- `GraphColumn::through` (fn) — draw through a `Fold`: kept lanes in their drawn columns, a `RAIL` in the last column on every row (text-muted), folded edges as one stub in the gap right of the node (first edge's colour); no count in the column; dashes still key on the real column.
- `STUB_BELOW` / `STUB_ABOVE` / `RAIL` (const) — `▾` / `▴` / `⋮` (ASCII `v` / `^` / `:`).
- `GraphColumn::width_through` (fn) — cells for a window drawn through a `Fold` (kept lanes plus the rail, two cells each).

## graph_screen
- `CHROME_ROWS` (const) — screen rows that are not graph rows (header + footer); a `GraphView::banner` adds one row per line.
- `SHA_WIDTH` (const) — cells the short-SHA column takes, and hex digits to call `short_sha` with.
- `AUTHOR_BREAKPOINT` (const) — width below which the author chip drops (DESIGN.md §1.2/§1.3).
- `GraphView` (struct) — everything the graph screen draws.
- `GraphView::location` (field) — the repository's path, once open.
- `GraphView::walk` (field) — where the history walk is.
- `GraphView::problem` (field) — an unsuperseded error; replaces the key hints while it lasts.
- `GraphView::rows` (field) — viewport rows, top first; the cursor's row is painted highlighted.
- `GraphView::icons` (field) — which glyphs badges draw with (`ui.icons`).
- `GraphView::compaction` (field) — lane limit, expansion flag and `Keep` priorities; the screen plans one `Fold` per frame over the rows it draws, and its header ends ` · +N lanes` while that fold hides lanes (not when expanded).
- `GraphView::filters` (field) — what the history shown leaves out; said in the header.
- `GraphView::reveal` (field) — index into `rows`: the footer shows that row's full subject in place of the hints, but only when the row's subject actually needed truncation (`None` for no reveal) (m1-the-graph/41, ADR-0023).
- `GraphView::order` (field) — the order notice while rows are not in their final order, or `None`.
- `GraphView::ascii` (field) — draw the graph column with the ASCII glyph table (`graph.renderer`, m1-the-graph/19).
- `GraphView::notice` (field) — what the app layer did to the renderer tier, shown in the footer under `problem`, above the hints; `None` for nothing to say.
- `GraphView::pixel` (field) — the pixel tier draws the graph: the column keeps its width, left blank for the images (m1-the-graph/24c).
- `GraphView::search` (field) — the search footer line (`SearchLine`): below `problem`, above `notice`; `None` when no search.
- `GraphView::banner` (field) — `Vec<SanitizedText>` drawn under the header, above the rows, each fitted to the width (first line accent, rest info); each line costs a body row, so `graph_area` moves down with it; empty in the shipped binary (m1-the-graph/38).
- `GraphView::commit_graph` (field) — `Option<CommitGraphLine>`: one footer row above the hints, below a notice; takes a graph row while `Some` (m1-the-graph/54).
- `CommitGraphLine` (enum) — `Offered`, `NoGitToBuild`, `Writing`, `Written { commits }`; wording per width (≥120, 100–119, <100), key letters in the accent. `Written` at ≥100 cols adds ` · Ctrl+R to use it` (m2-details-and-diffs/21).
- `SearchLine` (enum) — `Prompt(text)` draws ` /text_`; `Status{text, failed}` draws the search's line, warning-coloured only when failed.
- `GraphView::graphics` (field) — `gfx ok` / `gfx: <message>` in the header while pixels are drawn; `None` otherwise.
- `GraphView::detail_shown` (field) — the detail pane is on screen; the footer adds a `<> resize` hint only then, since that's the only time its divider exists to drag (`detail-resize-and-persistence`).
- `GraphView::changes_pane` (field) — the pane on screen is the changes pane; the footer adds `space stage` and `d discard`, ahead of the movement keys so a narrow terminal keeps them. Never set for a commit's pane: the keys do nothing there (m3-working-tree-undo-and-stash/04).
- Footer hints add `. explain` whenever the cursor row (`RowPaint::Highlighted`) carries a `GraphEntry::marker` (m2-details-and-diffs/17).
- `GraphView::compare_footer` (field) — the pinned compare base's footer line (`compare base <sha> — move, x again clears`); same footer slot as `search`/`notice` (m2-details-and-diffs/11).
- `GraphicsStatus` (enum) — `Ok` or `Failed(SanitizedText)`: what the terminal said about the graph images.
- `Filters` (struct) — `first_parent`, `folded` (merge count), `stash_hidden`, `focused` (Focus mode on); `Filters::text` is the header's words, empty for none.
- `Compaction` (struct) — `max_lanes` (the resolved cap for this frame, see `max_lanes_for`), `expanded` (the `+` key), `keep` (`graft_graph::fold::Keep`: selected column, reserved integration lanes, the selection's thread — budget priorities, not vetoes, ADR-0025).
- `Compaction::at` (fn) — collapsed, `Keep::default()`.
- `lanes_that_fit` (fn) — the most lane columns, rail included, the graph column can hold at a screen width; never below 2 (m1-the-graph/44).
- `max_lanes_for` (fn) — `graph.max_lanes` if configured, else 8/10/12 by the width breakpoint (`< 100`, `100..160`, `160+`), then clamped to `lanes_that_fit` so a configured number larger than the screen never cuts the rail (ADR-0022, ADR-0025; m1-the-graph/44).
- `GraphEntry` (struct) — one row: `line`, `badges`, sanitised `subject`, `author` chip, `when`, `sha`, `compare_marked`, `marker`.
- `GraphEntry::compare_marked` (field) — this row is the pinned base of a live compare; draws the accent `▏` left rule (m2-details-and-diffs/11).
- `GraphEntry::marker` (field) — `Option<MarkerView>`: this row's Mistake Radar marker, painted at draw time (`view::radar::marker_for`, m2-details-and-diffs/13).
- `MarkerView` (struct) — a marker's `severity`, all a paint-time lens needs; draws `⚠`/`ⓘ` (ASCII `!`/`i`) in the marker slot, only while `GraphView::marker_column` and only in the row's colour token.
- `GraphView::marker_column` (field) — the marker slot may draw a glyph this session: sticky once a finding is known, never toggles off on scroll (m2-details-and-diffs/13).
- `AuthorChip` (struct) — an author's chip: sanitised `initials` and the `identity` that colours it.
- `OrderNotice` (enum) — header notice: `Sorting` (date order while sorting) or `Provisional` (sort stopped; order stays).
- `WalkView` (enum) — walk state for the header: opening, reading, jumping, complete, cancelled, failed.
- `GraphScreen` (struct) — the graph screen, ready to render.
- `GraphScreen::new` (fn) — draw `view` in `theme`.
- `GraphScreen::graph_area` (fn) — the graph column's rectangle inside the screen area, every body row tall: where pixel images are placed.

## graph_screen::badges
- `Icons` (enum) — `Plain` (default, bracketed) or `Nerd` (icon-prefixed).
- `Badge` (struct) — one ref on a commit: sanitised `name` and `kind`.
- `Badge::text` (fn) — the badge as it reads on screen in an icon set.
- `BadgeKind` (enum) — branch (identity, current, tracking), remote, tag, detached HEAD, stash.
- `Tracking` (enum) — untracked, in sync with a remote, or apart by ahead/behind counts.
- `separator` (fn) — what goes between two badges in an icon set.

## graph_screen::columns
- `initials` (fn) — two letters standing in for a sanitised name, for the author chip.
- `relative_date(then, now)` (fn) — how long ago `then` was relative to `now` (both Unix seconds), in short form (`"2m"`, `"3h"`, `"2d"`, `"2w"`, `"2mo"`, `"1y"`), rounding to match `git log --date=relative`'s cascade exactly (m1-the-graph/48) — units below "now" never read `1`, only years can; `then > now` reads `"future"` (m1-the-graph/53).
- `absolute_date` (fn) — a commit's recorded date in its own offset, `"2026-09-13 14:02 +0500"` (Hinnant's `civil_from_days`, integer-only, no date/time crate) (m1-the-graph/34).
- `short_sha` (fn) — the first N hex characters of a full object id.

## journal_screen
The undo journal browser (`J`, DESIGN.md §1.10): every write this repository
has taken, newest first (m3-working-tree-undo-and-stash/06). An overlay over
the graph area, like `map_screen`: no state, no I/O, and
`view::journal::journal_view` (crate `graft`) builds its input. Every
field-relevant fact is in the text, because a capture carries no colour and
neither does a `NO_COLOR` terminal.
- `RowMark` (enum) — `None`, `NextUndo`, `NextRedo`, `Interrupted`, `Undone`; `label` (fn) draws them as `← u`, `← U`, `interrupted`, `undone` and the empty string.
- `JournalRowView` (struct) — `age`, `description`, `summary`, `mark`, `commands` (the Command Log, drawn while expanded), `failure` (the whole reason, same), `expanded`.
- `JournalStatus` (enum) — `Loading`, `Empty` (`nothing yet — every write lands here and u takes it back`), `Failed(SanitizedText)`, `Ready`.
- `JournalView` (struct) — `location`, `rows` (newest first), `selected`, `status`.
- `JournalScreen` (struct) — `view`, `theme`; `Widget`: header row, rows (cursor row painted to the right edge, mark right-aligned, expanded rows' `$ git …` lines under them), footer. Draws nothing outside its `Rect` at any size.

## map_screen
The Map view (`M`, DESIGN.md §5.1): one row per local branch, a density
sparkline against a shared time axis (m2-details-and-diffs/19). Owns no
state or I/O; `view::map::map_view` (crate `graft`) builds its input from
`map::MapData`.
- `MapMarkView` (enum) — `Tag(SanitizedText)` or `Radar(Severity)`: a mark on one bucket, sanitised for drawing.
- `MapRowView` (struct) — one branch's row: `name`, `identity`, `counts`, `is_current`, `merged_into`, `marks`, `active`, `complete`.
- `MapStatus` (enum) — `Loading { done, total }`, `Empty`, `Failed(SanitizedText)`, `Ready`: what draws instead of, or with, the rows.
- `MapView` (struct) — everything the screen draws: `title_branch`, `span_days`, `rows`, `max_count`, `buckets`, `selected`, `more`, `complete`, `status`, `ascii`.
- `MapScreen` (struct) — `view`, `theme`; `Widget::render` draws the header (branch/span, then `BRANCH`/`NOW`), rows (prefix, name, bar, marks, elbow, tag annotations, current/active suffix), the state body, and the footer.
- `ramp_glyph(count, max, ascii)` (fn) — the density glyph for a bucket: blank only at `count == 0`, otherwise the ramp's ceiling-rounded level (never a false blank for a nonzero count).
- Split into files, public items unchanged (m2 review M8): `render.rs` has `MapScreen`'s `Widget` impl and every draw helper (`ramp_glyph` included, now `pub(crate)`); this file keeps the view model types and `MapScreen` itself.

## overview_strip
The overview strip (`O`, DESIGN.md §5.3): one column, the whole history
bucketed onto the screen's height (m2-details-and-diffs/20). Owns no
state or I/O; `view::strip::strip_marks` (crate `graft`) builds its input.
- `StripMark` (enum) — `Merge`, `Radar(Severity)`, `SearchHit`, `Head`: one thing worth marking about a row.
- `StripMarks` (struct) — `rows_total`, `viewport`, `marks: Vec<(usize, StripMark)>` — what the widget draws from.
- `StripCell` (enum) — `Blank`, `Rail`, `Viewport`, `Merge`, `Head`, `SearchHit`, `Radar(Severity)`: one cell, already resolved to a priority.
- `cell_for(marks, height, cell)` (fn) — the highest-priority mark in `cell`'s bucket; blank once the bucket starts past `rows_total`.
- `row_for_cell(marks, height, cell)` (fn) — the first row of `cell`'s bucket, for the click.
- `OverviewStrip` (struct) — `marks`, `theme`, `ascii`; `Widget::render` draws one glyph per cell, Unicode or ASCII.

## paint
- `color` (fn) — resolve a `TokenColor` to a ratatui `Color`.
- `foreground` (fn) — a style that paints text in one of the theme's tokens.

## stash_screen
The stash overlay (`S`, DESIGN.md §1.11): every stash entry, the selected
one's files, and the save prompt (m3-working-tree-undo-and-stash/06). An
overlay over the graph area like `journal_screen`: no state, no I/O, and
`view::stash::stash_view` (crate `graft`) builds its input. Every fact is in
the text — the scope is `(•)`/`( )`, not a colour — because a capture
carries none and neither does a `NO_COLOR` terminal.
- `StashRowView` (struct) — one entry's row: `name` (`stash@{0}`), `message` (quoted, first line), `age`, `files`.
- `StashPromptView` (struct) — the save prompt: `title`, `message` (a `text_input::FieldView`), `staged_only`, `refusal` (what `s` would refuse, shown before `⏎` is pressed).
- `StashStatus` (enum) — `Loading`, `Empty` (`no stashes — s stashes the working tree`), `Failed(SanitizedText)`, `Ready`.
- `StashView` (struct) — `rows` (newest first), `selected`, `preview` (the selected stash's `detail::FileListView`, drawn under the list), `prompt`, `status`, `notice`/`notice_is_failure` — the overlay covers the graph footer, so a write's acknowledgement and a refusal are drawn on its own row above the footer or nowhere.
- `StashScreen` (struct) — `view`, `theme`; `Widget`: header (`STASHES (n)`), rows (cursor row painted to the right edge), the preview or the prompt, the notice, the footer (`⏎ apply  p pop  s save  d drop  space files  esc close`, or the prompt's own). Draws nothing outside its `Rect` at any size.
- `files_label` (fn) — `1 file` / `3 files`: the count with the noun it agrees with.

## status_screen
- `StatusView` (struct) — everything the status screen draws.
- `StatusView::repository` (field) — the repository, once open; `None` while opening.
- `StatusView::rendering` (field) — what Graft decided to draw with, and how sure.
- `StatusView::notes` (field) — advisories (old git, unrecognised config key); empty draws nothing.
- `StatusView::problem` (field) — an unsuperseded error; absent when nothing is wrong.
- `StatusView::history` (field) — the history walk view, once one has been started.
- `HistoryView` (struct) — how far the history walk has got.
- `HistoryView::state` (field) — where the walk is.
- `HistoryView::rows` (field) — rows the layout engine has laid out so far.
- `HistoryState` (enum) — walk state; worded so "fine" never reads as an error.
- `RepositoryView` (struct) — the repository Graft opened.
- `RepositoryView::path` (field) — working tree root, or the repo's own dir if bare.
- `RepositoryView::bare` (field) — no working tree.
- `RepositoryView::head` (field) — where `HEAD` points.
- `HeadView` (enum) — where `HEAD` points, worded for a person.
- `RenderingView` (struct) — the tier/theme as the status screen shows them, with provenance.
- `RenderingView::tier` (field) — tier in effect: detection's answer unless config overrode it.
- `RenderingView::tier_source` (field) — where the tier came from.
- `RenderingView::cell_size` (field) — size of one character cell, and whether it was measured.
- `RenderingView::theme` (field) — the theme in effect.
- `RenderingView::theme_forced` (field) — theme came from `ui.theme`, not terminal background.
- `RenderingView::theme_unknown` (field) — terminal never reported background; theme is a default.
- `RenderingView::color_depth` (field) — how much colour the terminal has.
- `RenderingView::no_color` (field) — `NO_COLOR` set: chrome drawn without colour regardless.
- `TierSource` (enum) — where the rendering tier came from.
- `StatusScreen` (struct) — the status screen, ready to render.
- `StatusScreen::new` (fn) — draw `view` in `theme`.

## syntax
- `SyntaxRole` (enum) — a highlighted token's category: `Plain`, `Comment`, `String`, `Keyword`, `Type`, `Function`, `Number`, `Punctuation`, `Attribute`.
- `Language` (struct) — a resolved `syntect` syntax; opaque.
- `language_for_path` (fn) — the language a path's extension or exact file name (`Makefile`, `Dockerfile`) implies.
- `language_for_first_line` (fn) — the language a shebang or modeline implies, for extension-less files.
- `SyntaxSpan` (struct) — one highlighted run: a half-open, ascending, non-overlapping **char** range and its `SyntaxRole`.
- `highlight` (fn) — highlight every line of a file in one language; parser state carries across lines; past 5,000 lines or a 4,000+ char line, every line comes back `Plain`; never panics.

## theme
- `LANE_COUNT` (const) — number of lane hues; lanes beyond reuse them.
- `Token` (enum) — a chrome colour role.
- `Token::ALL` (const) — every token, in PROMPT.md §5's order.
- `Token::name` (fn) — the token's name as PROMPT.md §5 and config spell it.
- `ThemeKind` (enum) — dark or light.
- `ThemeKind::select` (fn) — resolve theme from an override and a terminal hint; unknown → dark.
- `ThemeKind::name` (fn) — the theme's name as the config file spells it.
- `TokenColor` (enum) — a resolved colour, `NO_COLOR`-aware.
- `Theme` (struct) — a resolved theme; no path hands a widget a colour the terminal can't show.
- `Theme::new` (fn) — resolve a theme for a kind and terminal capabilities.
- `Theme::kind` (fn) — which theme this is (diagnostics, config round-trips).
- `Theme::color_depth` (fn) — how much colour this terminal has, `NO_COLOR`-reduced like every token; widgets use it to reach for a non-colour separator at `Palette16`.
- `Theme::token` (fn) — the colour for one chrome token.
- `Theme::lane` (fn) — the colour for one graph lane; shape/dash separate same-hue lanes.
- `Theme::raster_lanes` (fn) — truecolor lane hues for the pixel tier at any depth; one muted hue when colour is off.
- `Theme::raster_muted` (fn) — the unreduced `text-muted` hue, for a pixel `+N` marker.
- `Theme::branch` (fn) — the colour a branch identity draws in, independent of column.
- `Theme::syntax` (fn) — the colour for a `syntax::SyntaxRole`, from the chrome tokens; collapses to `Token::Text` at `Palette16` and under `NO_COLOR`.

## text_input
Text entry (m3-working-tree-undo-and-stash/05). The state halves are pure —
no width, no theme — and the cursor is a **character** index, so a wide
character or a combining mark is never cut in half. Horizontal and vertical
scrolling are worked out at draw time from the cursor rather than stored, so
no offset can go stale behind a resize.
- `GUIDE` (const) — `┊`, the length-guide glyph (column 50 for a subject, 72 for a body).
- `LABEL_WIDTH` (const) — cells the `Subject`/`Body` label column takes, so both text regions start in the same column.
- `TextField` (struct) — one editable line. `new`, `text`, `cursor`, `len`, `is_empty`, `insert`, `insert_str` (a bracketed paste; control characters, newlines included, are refused at the door so the cursor index can never disagree with the sanitiser), `backspace`, `delete`, `left`, `right`, `home`, `end`, `word_left`, `word_right`.
- `TextArea` (struct) — several editable lines, always at least one. `new`, `text` (joins with `\n`; round-trips `new`), `lines`, `cursor`, `is_empty`, `insert`, `insert_str` (`\n` and `\r\n` break the line), `newline`, `backspace` (joins upwards at a line start), `delete` (pulls the next line up at a line end), `up`, `down`, `left`, `right`, `home`, `end`, `word_left`, `word_right`, `append_line` (a line of its own at the end, cursor after it; on an empty area it becomes the first line, never a blank one followed by it).
- `FieldView` (struct) — `label`, `text`, `cursor` (character index), `guide_at`, `counter` (`(typed, limit)`; drawn in `Warning` past the limit), `focused`.
- `FieldWidget` (struct) — `view`, `theme`; `Widget` over one row: label, text, guide, right-aligned counter.
- `AreaView` (struct) — `label`, `lines`, `cursor` (`(line, character)`), `guide_at`, `focused`.
- `AreaWidget` (struct) — `view`, `theme`; `Widget` over the rows it is given: the label row, then as many lines as fit, scrolled so the cursor's line is always one of them.

## commit_editor
The commit/amend editor's screen (m3-working-tree-undo-and-stash/05). Draws
into whatever rect it is given and carries no state of its own: the caller
owns the `TextField`/`TextArea` halves and passes a view.
- `SUBJECT_LIMIT` (const) — 50, where the subject's guide stands and past which its counter turns `Warning`.
- `BODY_LIMIT` (const) — 72, the body's guide column.
- `Field` (enum) — what has focus: `Subject`, `Body`, `Amend`, `SignOff`, `NoVerify`, `AllowEmpty`, `CommitButton`. `OPTIONS` (const) lists the four checkboxes in row order. `is_option`, `label`, `next`/`previous` (the `Tab` order, wrapping), `sibling_option(forward)` (`←`/`→` along the options row, `None` at either end).
- `CommitEditorView` (struct) — `title`, `subject`/`body` (the `FieldView`/`AreaView` halves), `focus`, `amend`/`sign_off`/`no_verify`/`allow_empty`, `refusal` (the reason drawn dimmed beside a dimmed `[ Commit ]`, `None` when the commit may go ahead), `status` (why the last write failed — a hook's refusal above all; takes the footer row from the key hints, in `Token::Warning`, until `Esc` dismisses it; m3/05 review M1), `busy`.
- `CommitEditorScreen` (struct) — `view`, `theme`; `Widget`: title row, subject row, body block, the options row pinned above the footer, then the key footer. Nothing scrolls off: below `fits` it is not drawn at all.
- `fits` (fn) — whether an area has the rows (`CHROME_ROWS` = 6, plus a body line) and columns the screen needs. The caller shows a "too small" message instead.
