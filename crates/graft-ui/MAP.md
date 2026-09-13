# graft-ui — map
One line per public item. Update when a public item changes.

## lib.rs
- `paint` (mod) — token-to-style helpers for widgets.
- `status_screen` (mod) — the status screen view and widget.
- `theme` (mod) — theme kinds, tokens, and colour resolution.

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
- `Theme::token` (fn) — the colour for one chrome token.
- `Theme::lane` (fn) — the colour for one graph lane; shape/dash separate same-hue lanes.
