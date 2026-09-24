# Draft KNOWN_GAPS rows — review m6-amend-14 Minors (for the conductor)

Minor 5 left out: ticket 15 owns it. Format matches `KNOWN_GAPS.md:666-668`.

| M6/14 | **`three_panes_capture`'s "= 180" assertion cannot fail.** `panes` is built from `bars` and 180, then summed back to 180 (`tests/it/resolver/three_panes.rs:93-102`); only `bars.len() == 3` (:92) and `≥ 48` (:103) test anything. `? open · o t b` is asserted anywhere in the frame (:77), not in block 2's result pane. Source: `docs/reviews/m6-amend-14.md` Minor 1 | **broken** (test proves less than it claims) | M7 |
| M6/14 | **"One width source" holds only while the frame's area equals `state.size`.** Layout uses `resolver_split(state.size)` (`view/frame/conflicts.rs:46-48`); render re-splits the frame's area (`graft-ui/src/conflict_resolver.rs:289`). Nothing asserts they agree. Question: can `terminal.draw` (`event_loop/running.rs:353`) run between a resize and its `Resize` event, giving one frame at the old width? Source: `docs/reviews/m6-amend-14.md` Minor 2 | **unverified** | M7 |
| M6/14 | **Any resize re-scrolls to the cursor block, not only a 180 crossing** (`update/mod.rs:744` → `geometry.rs:82-90`); `top` unclamped when `cursor_block` is `None` (`geometry.rs:83-85`). Question: is the snap wanted inside one arrangement? Ticket item 7 says "across 180". Source: `docs/reviews/m6-amend-14.md` Minor 3 | **unverified** (question, intent not settled) | M7 |
| M6/14 | **Drawn words with no test:** three-pane ASCII tier (`resolver_ascii_tier_is_ascii`, `conflict_resolver/tests.rs:340-353`, loops `Card` only; branches `panes/draw.rs:37, :58, :146, :179`); D3 `base` head (`panes/draw.rs:148-149`); binary `? open · o t` (`:171`); D2 `(not on …)` and `absent_on` mapping (`view/resolver.rs:108-123`). Source: `docs/reviews/m6-amend-14.md` Minor 4 | **unverified** | M7 |
| M6/14 | **Ticket 13's "what right looks like" at 200 is stale:** `13-resolver-file-list.md:125` says "then the card (three panes are ticket 14's)"; `docs/shots/m6/fix-file-list-200x50.txt` now shows three panes. Fix before ticket 21's contact sheet quotes it. Source: `docs/reviews/m6-amend-14.md` Minor 6 | **broken** (textual) | M7 |
| M6/14 | **`pane_number_widths` recomputed per drawn row** (`panes/draw.rs:60`, inside `draw_pane_row`): O(visible rows × regions) per frame. Unmeasured; `TooLarge` caps file size. Source: `docs/reviews/m6-amend-14.md` Minor 7 | **unverified** (perf, unmeasured) | M7 |

Counts: 6 rows — 2 broken, 4 unverified.

## Existing row to close
- `KNOWN_GAPS.md:668` (M6/14, `three_panes.rs` 467, `draw.rs` 402 over ~400) — to be closed by ticket 27 (`.scratch/m6-conflicts/issues/27-review-fix-amend-14.md`); move to `docs/gaps/closed.md` when 27 lands. Not verified here that 27's scope covers it — dispatcher's word.
