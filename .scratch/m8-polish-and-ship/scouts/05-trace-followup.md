# Scout: 05 trace follow-up (per-turn cost)

Code at a873f1aa. Paths under `crates/` unless noted. No cargo run; raw traces not in repo (only summaries).

## What tail_ms measures — contradiction with the ticket's reading
- `scripts/m8-trace-summary.sh:43-46,62-63`: `tail_ms` = `at_us` of the **last frame in the whole trace** minus the last frame with `motions>0`. One number per run, not per key release, and not "frames kept coming": a single frame drawn 1.7 s after the last motion (rest follow, job result, re-detect reply, quit) yields 1748 ms. Whether Ghostty drew 1 or many frames in that window: **could not confirm** (raw trace not committed).
- Ghostty `motions p50=0` (ghostty.txt:4) vs Zed `p50=1`: over half the Pixel frames apply no motion. What drives them: **could not confirm** from summaries.

## Frame pacing (still true from frame-loop.md, renamed)
- Frame bytes: `graft/src/event_loop/running/frame.rs:93-125` — SYNC_BEGIN, ratatui draw, `graphics.frame` images, SYNC_END, fence, flush. `draw_traced` :31; `draw_us` excludes `write_us` (`graft/src/trace.rs:70`).
- Pacer `event_loop/pace.rs`: `FENCE_TIMEOUT` 500 ms :25, `FRAME_CAP` 16 ms :28, gives up fencing after 3 misses :31; deadline :116-122. Turn waits `turn.rs:196-198` (min of rest/offer deadline and pacer).
- Rest (settle): `event_loop/rest.rs:1-20` — follow deferred in a burst, `AppEvent::Rested` after quiet window 50–150 ms; deadlines `turn.rs:123-135, 341, 362-367`. Idle `prefetch` `turn.rs:182` → `running.rs:157-181` rasterises uncached strips (no output written; could not confirm it never owes a frame).

## Pixel bytes per frame
- `graft/src/event_loop/graphics.rs:145-212`: every frame walks visible strips (`STRIP_ROWS`=32, `graft-graph/src/raster/cache.rs:52`), builds layers :306-323, then `KittyImages::frame` (`graft-term/src/kitty/mod.rs:211-228`).
- Kitty `frame` **re-places every shown image every frame** (`put_placement` :219) and deletes placements no longer shown :221-226. p50 479 B (ghostty.txt:9) fits placements-only frames.
- Transmit happens in `ensure_uploaded` :278-313 when no upload for (first_row, layer) or its key changed; a changed key frees the old id :290 and sends the full frame :294.
- Backdrop key has no selection in it (`cache.rs:25, 213`) → cached across cursor moves; new strip entering view on scroll → transmitted once.
- Emphasis layer keyed by `paint_key` incl. selection (`graphics.rs:313, 320-322`) → **any cursor move inside a strip changes its key and re-transmits that strip's emphasis image** (unverified as the p95 77 KB source; emphasis image size not measured).
- Upload capacity `2*STRIPS_KEPT` = 32 (`graphics.rs:34, 97`); LRU eviction of unplaced only (`kitty/mod.rs:337-352`). Viewport 70 rows ≈ 3–4 strips.
- `forget_uploads` (`kitty/mod.rs:249`) on clear/resize re-sends all.

## Overview strip search-hit rebuild (ticket's named candidate)
- `graft/src/view/strip.rs:24-57` `strip_marks`: per call builds `HashSet<Oid>` of **all** search hits (:38-42, O(hits)), then iterates `history.window.rows` (resident window, not visible viewport) :44-63, calling `radar::marker_for` per row :54 and decoding subject :52. Runs every frame the strip is drawn, no change check. With no active search, set is empty (cheap). Callers: grep found none outside strip.rs with `--include`; **could not confirm** call site (run `grep -rn strip_marks crates/graft/src`).
- Test: `a_search_hit_on_a_resident_row_is_marked` strip.rs:180.

## Contradictions
- Ticket "tail after release" ≠ script's definition (script :15-17, 62-63).
- Ticket's strip candidate is O(hits)+O(resident rows), not O(history) unless search hits ~ history; at 3494 rows (ghostty.txt:1) unlikely to cost ms — Pixel draw_us p95 4 ms vs Unicode 1 ms points at `graphics.frame` (rasterise/encode) rather than strip_marks (unverified).
