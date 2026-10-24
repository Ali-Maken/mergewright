# Scout 29r: GraphicsStatus::Failed and the graph column

Answer: yes, column stays blank (pixel tier kept, text skipped). No code path drops the tier on Failed.

## Where Failed is set (all in crates/mergewright/src/event_loop/graphics.rs)
- :150 `drain()`: any Kitty reply with `is_error()` (terminal error reply e.g. ENOENT). Status = last error in the batch; reset to Ok (:146) when a drain has none, so it self-clears.
- :197 iTerm2 path: `self.bands.frame(..)` Err (write/encode error).
- :252 strip rasterise/layers Err in `drawn` (PNG encode etc.).
- :289 same, background pre-raster.
- No path for "no ack" or "timeout": drain only reads replies that arrive (`replies.take()`, :144); silence sets nothing. Upload "refused" = only via an error reply (:150). Write error to stdout: could not confirm whether it propagates as io::Error out of the loop rather than Failed.
- Consumed: running/turn.rs:244,270.

## Tier is never touched by Failed
- `draws_pixels` view/frame.rs:129-131 reads session+state only; no status. Doc comment :122-126 says F3 is the escape.
- render.rs:149: text only if `!view.pixel || gutter.is_none()`; at pixel tier cells stay blank (comment :144-147).
- Header: layout.rs:255-256 `gfx: {message}`; Ok shows `gfx ok` (:254).
- Only tier-drop: F3 (update/state.rs:143 "forced this tier for the rest of the run"); not tied to Failed.
- The ENOENT case re-sends the strip (event_loop/tests.rs:532-545), so a transient error may recover; other errors persist as blank.

## Design
- docs/design/04-terminal-capability-detection.md:77: config `graph.renderer` "always wins ... still attempts it ... with a one-key escape hatch to drop a tier if the output looks garbled". Escape = manual F3. No text says auto-fallback on Failed.
- :107-111: tier is "decided once at startup, not a per-frame guess" (stated for the tmux probe).
- Phrase "config wins" is in frame.rs:125 comment, not found verbatim in DESIGN.md/docs/design (grep). DESIGN.md §4.2 itself: could not confirm beyond the doc above (design lives in docs/design/04).
- That row covers a forced `pixel` config; whether it covers an auto-detected pixel tier that Fails: could not confirm, doc silent.

## Tests
- Header text only: graph_screen/tests.rs:897-899, tests/problem_line.rs:59-70, event_loop/tests.rs:541 (frame contains `gfx: ENOENT...` and strip re-sent).
- No test found asserting the graph column (blank or text) after Failed (grep `Failed(` / `gfx:` over crates). Could not confirm absence in walk-tier tests beyond that grep.

## Not confirmed
- Real terminal behaviour (not run). KNOWN_GAPS row 1472 claim "builder saw no glyphs" not reproduced.
