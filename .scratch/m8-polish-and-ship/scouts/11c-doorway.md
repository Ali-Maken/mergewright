# Scout 11c — doorway on overlays, places, marked rows (main c89bf617)

## Where the doorway is decided (graft-ui `graph_screen/hints.rs`)
- `DOORWAY` / `DOORWAY_BY_PALETTE` :357/:359. Fallback `?`→`⌃P`→none in `spelled_bar` :442–461 (trailer dropped if it spells nothing).
- The doorway is a **trailer**, not a list item: `whole_bar` :521 returns `(list, Option<Hint>)`.
  - :546–562 `Journal | Stash | Map | PaneOverOverlay | Blame | History` (with banner, boxes, picker, resolver, finish, rewrite) → `(drawable(..), None)`. Comment :531–545 gives the old "modal swallows `.`" reason.
  - :563–572 `Marked(inner)` → `[EXPLAIN] + drawable(inner)`, trailer `None`; comment :568–570 cites 11b's 92/83 costs.
  - :594–600 `WorkingTreeRow|BranchRow|PlainRow|Rewritable|ChangesPane{..}` → `Some(DOORWAY)`.
  - `BackToBlame/BackToHistory` :589–590 via `led_by` :659–664 — inherits inner's trailer (Blame/History → None today).
- Divider-hint exclusion list :796–810 names the same overlays/places (unchanged by 11c unless the builder finds otherwise).
- Fitting: `fit_list(&list, trailer, …)` :438; trailer is placed by the fitter (floor behaviour lives in `hints/fitting.rs` — not read in detail).
- Lists themselves: `hints/lists.rs` `context_list` :48 (Journal :82, Stash :83/`stash_list` :401, Map :87, PaneOverOverlay :97/:326, Blame :73/`blame_place_list` :259, History :78/`history_place_list` :298, ChangesPane over-overlay :460/:474). None holds the doorway.
- Tests: `hints/tests.rs` own `doorway_of` :1100–1140 (Journal/Stash/Map/PaneOverOverlay/Blame/History → None :1120–1124; Marked → `Some(EXPLAIN)` :1126; ChangesPane → DOORWAY :1138); uses :1206, :1307, :2571; :255 asserts a context lacks DOORWAY. `catalogue.rs` :556–557 pushes both doorways.

## Contexts built (graft `view/rows.rs`)
Map/Journal/Stash :675–677, PaneOverOverlay :707, History :723, Blame :729, Marked :789. 11c need not edit these.

## Key handling
`?` answered before overlays/places: `update/keys.rs:197` `palette::open_key` (comment :194–196, "every place but a live confirm box"). No Marked-specific `?` path found; marked row falls through the same call.

## Contradictions with the ticket
1. **ChangesPane{over_overlay:true} already draws the doorway**: hints.rs:596–600 gives every `ChangesPane` `Some(DOORWAY)`; `actions_unless_over_overlay` :645–656 comment says "the doorway stays". Ticket decision 1 / Touches say it must "gain" it.
2. **Edit site is `whole_bar` trailers in hints.rs, not "appending DOORWAY to the lists" in `hints/lists.rs`** (Signatures line). Lists need no change unless the builder chooses to inline it.
3. `hints/lists/` is a directory holding only `resolver.rs`; lists named are in `hints/lists.rs`.
4. Marked's "ranked last, dropped first" needs the trailer to be droppable; current trailer is placed by `fit_list` as a doorway (floor-ish). Could not confirm whether `fitting.rs` can drop a trailer before list hints — builder reads `fitting.rs`.

## Files 11c would edit
graft-ui: `graph_screen/hints.rs` (whole_bar arms, comments), `hints/tests.rs`, possibly `hints/fitting.rs` (Marked), `catalogue.rs` (only if a coverage slot changes), MAP.md. graft: `tests/it/hint_bar.rs`, `update/tests.rs`, a `view` test for the drawn palette, MAP.md. `docs/design/02-keymap.md`.

## Knock-on
Captures: `docs/shots/m8/changes-over-stash-bar-80x24.txt`, `pane-over-map-bar-80x24.txt`, `polish-blame-whitespace-counted-120x36.txt` (blame bar) plus older milestone shots of journal/stash/map/blame/history/marked bars (not enumerated). No m8 contact sheet file present in `docs/shots/m8` (11 files).

## Overlap with 17c
Source disjoint: 17c is graft-only (`view/frame.rs`, `view/rows.rs`, layout, graphics, `update/mod.rs`); 11c does not need `view/rows.rs`. Shared: `crates/graft/MAP.md`; `update/tests.rs` if 17c adds `a_pixel_preview_requests_no_strips` there; m8 contact sheet re-assembly; `GRAFT_UPDATE_SHOTS` re-shoot runs touch all captures.
