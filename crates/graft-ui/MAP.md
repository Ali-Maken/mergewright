# graft-ui — map
One line per public item. Update when a public item changes.

## lib.rs
- `fit` (mod) — clamp text to an exact cell width.
- `graph_column` (mod) — the Unicode-tier graph column widget.
- `graph_screen` (mod) — the graph screen Graft opens on: header, rows, key hints.
- `graph_screen::badges` (mod) — ref badges: kinds, tracking state, plain/Nerd Font text.
- `graph_screen::columns` (mod) — pure formatting for the author initials, relative date and short SHA.
- `paint` (mod) — token-to-style helpers for widgets.
- `status_screen` (mod) — the status screen view and widget.
- `theme` (mod) — theme kinds, tokens, and colour resolution.

## fit
- `MAX_MARKS` (const) — combining marks kept per base character.
- `fit` (fn) — text laid into exactly N cells: padded, or cut with `…`; wide chars never split.

## graph_column
- `CELLS_PER_LANE` (const) — cells per lane: its own, then a gap.
- `NodeKind` (enum) — node shape: commit, merge, root, HEAD, stash, uncommitted.
- `NodeKind::of` (fn) — commit/merge/root from a row's parents.
- `NodeKind::glyph` (fn) — the node's character.
- `RowPaint` (enum) — normal, dimmed, highlighted.
- `GraphLine` (struct) — `row` (borrowed or synthesised), `node`, `paint` for one drawn row.
- `GraphLine::plain` (fn) — a row drawn normally, borrowing a real `Row`.
- `GraphLine::owned` (fn) — a row drawn normally, owning a synthesised `Row` (the uncommitted-changes row).
- `GraphColumn` (struct) — widget drawing consecutive rows; fills its area exactly. At 16 colours and `NO_COLOR`, odd-column lanes draw dashed (`╎`, `╌`).
- `GraphColumn::new` (fn) — draw lines in a theme.
- `GraphColumn::width_for` (fn) — cells for N lanes.

## graph_screen
- `CHROME_ROWS` (const) — screen rows that are not graph rows (header + footer).
- `SHA_WIDTH` (const) — cells the short-SHA column takes, and hex digits to call `short_sha` with.
- `AUTHOR_BREAKPOINT` (const) — width below which the author chip drops (DESIGN.md §1.2/§1.3).
- `GraphView` (struct) — everything the graph screen draws.
- `GraphView::location` (field) — the repository's path, once open.
- `GraphView::walk` (field) — where the history walk is.
- `GraphView::problem` (field) — an unsuperseded error; replaces the key hints while it lasts.
- `GraphView::rows` (field) — viewport rows, top first; the cursor's row is painted highlighted.
- `GraphView::icons` (field) — which glyphs badges draw with (`ui.icons`).
- `GraphEntry` (struct) — one row: `line`, `badges`, sanitised `subject`, `author` chip, `when`, `sha`.
- `AuthorChip` (struct) — an author's chip: sanitised `initials` and the `identity` that colours it.
- `WalkView` (enum) — walk state for the header: opening, reading, jumping, complete, cancelled, failed.
- `GraphScreen` (struct) — the graph screen, ready to render.
- `GraphScreen::new` (fn) — draw `view` in `theme`.

## graph_screen::badges
- `Icons` (enum) — `Plain` (default, bracketed) or `Nerd` (icon-prefixed).
- `Badge` (struct) — one ref on a commit: sanitised `name` and `kind`.
- `Badge::text` (fn) — the badge as it reads on screen in an icon set.
- `BadgeKind` (enum) — branch (identity, current, tracking), remote, tag, detached HEAD, stash.
- `Tracking` (enum) — untracked, in sync with a remote, or apart by ahead/behind counts.
- `separator` (fn) — what goes between two badges in an icon set.

## graph_screen::columns
- `initials` (fn) — two letters standing in for a sanitised name, for the author chip.
- `relative_date` (fn) — how long ago a Unix timestamp was, in short form (`"2m"`, `"3h"`, `"1d"`).
- `short_sha` (fn) — the first N hex characters of a full object id.

## paint
- `color` (fn) — resolve a `TokenColor` to a ratatui `Color`.
- `foreground` (fn) — a style that paints text in one of the theme's tokens.

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
- `Theme::branch` (fn) — the colour a branch identity draws in, independent of column.
