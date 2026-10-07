# Scout — 17 look polish rows (main @ ae74b302)

## Per item: what exists
- **2 fold `+N`**: graph_column has no per-row `+N` after subject (grep `+{` in graph_column/graph_screen columns: none). Fold counts live in `crates/graft/src/view/rows.rs:134` (`folded: inclusion.folded.len()`), fold priority `rows.rs:369-395,975`. Row text drawn in `crates/graft-ui/src/graph_screen/columns.rs` (349 lines). Which merge's side is folded → per-row count: *could not confirm* a ready source.
- **3 fade**: Unicode fade is SGR DIM: `graph_column/mod.rs:177-183` (`RowPaint::Dimmed => DIM`), `graph_column/style.rs:44-69` (`off_thread`, `on_thread` add DIM). Lane hue: `theme/mod.rs:309 Theme::lane`; `theme/lanes.rs` (70), `theme/depth.rs` (depth). No `lane_muted`.
- **4 path/notice**: `graph_screen/header.rs` (301) — not inspected in depth; *could not confirm* closed.
- **5 overview**: `Ui` struct `crates/graft/src/config/types.rs:61-66` = theme, icons only. Startup seeding in `event_loop/startup.rs` (741) — no `overview` there.
- **6 ghost at Pixel**: strips built by `view/strips.rs:196 pixel_rows`, reads `history.window` rows only (`:201-202`), no preview reference in strips.rs or `event_loop/graphics.rs` (call sites `:167,:223`). Preview rows built in `view/preview.rs` (306). So strips today paint the live window rows, not the preview's ghost slice — on-screen result *could not confirm* (no capture read). `rows.rs:91` sets `pixel: false` in some view path; `rows.rs:128` says pixel tier reads none of row geometry. Ghosts are tied to strips only by both reading the same viewport; not deep, but a strips/preview seam is new code.
- **7 ASCII**: literals `detail_pane/files.rs:29` (`\u{25a3}`), `:66` `\u{2192}`, `:72` `\u{b1}`, `:74` `?`. Precedent: `finish_summary/frame.rs:190` replaces `→`→`->`.
- **8 whitespace tail**: state: detail `update/detail/keys.rs:385` (`options.ignore_whitespace`), blame `update/place/keys.rs:64`, history `update/place/history_keys.rs:168`, `place/history.rs:160`. No title tail in `detail_pane/header.rs`, `blame.rs`, `history.rs` (grep: no hits).

## Files 17 touches
graft-ui: graph_column/{mod,style}.rs, graph_screen/{columns,header}.rs, theme/{mod,lanes}.rs, detail_pane/{files,header}.rs, blame.rs, history.rs. graft: config/types.rs (+load.rs), event_loop/startup.rs, view/{strips,preview,rows}.rs, event_loop/graphics.rs.
**Gap file sizes:** touches **startup.rs (741 now, gap says 704)**, **view/rows.rs (1103 now, gap says 984)** and graph_screen/**render.rs (677)** only if `+N`/tails go there; empty_state not touched. Gap numbers are stale against main.

## Overlap
- 11b (bar hints, mouse, captures): shares graph_screen/ and captures showing folded merges; hints/ files not touched by 17.
- 09e (bar `/ search` under overlays): `view/rows.rs:94-99 modal_blanked` — same file as 17's preview/strip work.
- 12 (palette): no shared file found.

## Size / kind
Items 2,3,5,7,8 mechanical; 6 design-heavy (strips↔preview seam, frame-bytes test). Two contexts advisable only if 6 grows: split 6 off; rest one context.
