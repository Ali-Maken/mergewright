# Scout 07 — cursor row + dark B (main @ 6f82acaa)

## Theme
- Types: `Rgb` = `graft_core::capabilities::Rgb` (theme/chrome.rs:1); `Token` enum theme/mod.rs:70 (`Selection` :86, `Text` :90, `TextMuted` :92); `Theme::token` → `TokenColor` (mod.rs:237, :315), not `Rgb` — `legible_on(fg: Rgb, bg: Rgb)` needs the caller to unwrap.
- Dark hues: theme/chrome.rs:37-50. Today sel `#5A6478`, text `#D7DBE0`, muted `#8A919C`. B per polish.py:62: sel `#5C667B`, text `#F0F2F5`, muted `#9AA1AC`. Light :64-71, HC :92-99.
- No contrast helper exists: grep `luminance|fn contrast|CONTRAST|legible_on` finds none. chrome.rs:34 comment cites ratios computed in `theme::tests` (theme/tests.rs; 3 `Selection` hits) — existing test asserts today's 4.28 shortfall reasoning; will change.
- Shot B numbers (m8-proto-4a-dark-a-b.txt:21-24): 3.14 / 5.14 / 6.95, match ticket.

## Cursor row (graph screen)
- `row_emphasis` graft-ui/src/graph_screen/render.rs:341-376: `Highlighted|DropTarget` → bold + `Selection` bg over `Rect{y,height:1,..area}` (:363-371). `area` = body, already inside 06's 2-cell gutter + 1-cell margin (layout.rs:44-46). So gutter already ground; **graph column is filled** today (fill starts at area.x, graph drawn from area.x :140-151).
- 16 colour / NO_COLOR skip the fill (:352-355).
- Marker slot: `draw_marker_slot` :270-302, rule from `row_rule` :580-586: cursor `▏`/`|`, DropTarget `▸`/`>`, compare base `▏`.
- Badges/subject/author/date/sha drawn after with `style`/`trailing_style` (:179-200); author fg = `theme.branch(identity)` (:190) — no legibility adjustment.
- Pixel: cells left blank (:137-138) but the bg fill still covers them; strip compositing in graft/src/view/strips.rs has no cursor fill (Backdrop graft-graph/src/raster/emphasis.rs:30 is alpha only; strips.rs:70 "cursor move leaves every strip").

## Other `Selection` painters (need `legible_on` check)
journal_screen.rs:309, stash_screen.rs:199, detail_pane/files.rs:179, finish_summary/list.rs:143, map_screen/render.rs:187, rewrite_editor/draw.rs:233, conflict_resolver/finish_row.rs:58. Status screen: no `Selection` hit. Picker: no hit (could not confirm picker's cursor style).

## Contradictions with ticket
1. Ticket "drop target draws →/=, never ▸": code draws `▸`/`>` for DropTarget at render.rs:583 — change, plus m5/35 drag tests asserting `▸` (grep before).
2. Ticket puts `▸` "in the gutter"; code's cursor mark is `▏` in the marker slot (render.rs:581,584). Ticket says "compare base keeps ▏" but is silent whether cursor's `▏` in the slot goes — **could not confirm** intent; ADR-0058 not read in full.
3. `>` in ASCII for cursor collides with DropTarget's current ASCII `>` (:583); ticket moves drop to `=`.
4. Signature assumes `Theme::token` yields `Rgb`; it yields `TokenColor`.

## 06 nearby
06 (merge 00345448) added header.rs, the gutter/margin in layout.rs, render.rs (34 lines), and re-shot graph_screen, ascii_tier, lanes_without_hue snaps — all of which include cursor rows and will move again.

## Captures/goldens likely to change
graft-ui snapshots: graph_screen__feature-branches-{80x24,120x40,200x60}, ascii_tier__*, lanes_without_hue__*, graph_screen tests badges-icons-{nerd,plain}; graft-ui/tests/selection.rs; theme snapshots (theme/snapshots/). Keys: none claimed.
