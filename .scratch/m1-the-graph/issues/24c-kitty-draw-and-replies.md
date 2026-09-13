# m1-the-graph/24c — Draw Kitty strips in the app, drain replies, gfx status

Status: done
Build with: **Opus** (high) — terminal graphics protocol
Blocked by: 24b
Leaves runnable: yes — Ghostty and other Kitty-protocol terminals show the pixel graph.

Split from 24. 24's "Handoff" section (bullet **24c** and Gotchas) is the spec.

## Read first
- Ticket 24, "Scope", "Acceptance" and "Handoff" sections
- ADR-0012, including its Correction
- CLAUDE.md, the status-field rule
- `crates/graft-term/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft/MAP.md`

## Scope
In:
- `graft-ui`: blank graph column at Pixel tier (keep width); export the graph rect (`GraphScreen::body`).
- `event_loop::draw`: after `terminal.draw`, inside `hold_output()` and before END_SYNCHRONIZED_UPDATE, call `KittyImages::frame`.
- Open a `ReplyWaiter`, drain non-blocking each wake, `answer` each reply; status `gfx ok` / `gfx: <msg>` (sanitize_line). Confirm the redetect worker's waiter ignores graphics replies.
- `free_all` on exit; `hide` when leaving the pixel tier.

Out:
- Cache flush wiring, F3 from Pixel → Unicode, `graph.renderer = pixel` forcing, real-Ghostty check and captures (24d). iTerm2 (25). tmux/SSH (29).

## Acceptance
- [x] A simulated `ENOENT` reply through the app path causes a re-transmit and a visible `gfx:` status.
- [x] Exit frees every image (byte-stream test).
- [x] Never `ESC[2J` in steady state (asserted).
- [x] Say what you verified and how; no claims about how it looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.

## Done (2026-09-13, builder-opus, no compaction)

Built:
- `graft-ui`: `GraphView::pixel` leaves the graph column blank at the same width; `GraphView::graphics` / `GraphicsStatus` puts `gfx ok` / `gfx: <msg>` in the header, only while pixels are drawn; `GraphScreen::graph_area(area)` exports the column rect (same layout as `body`, which now shares `body_layout` + `graph_width`).
- `graft::view`: `draws_pixels` (graph screen, pixel tier from override else session, Kitty protocol); `view_with_graphics` returns the graph rect; `view` unchanged for callers. `PixelRows::uncommitted_on_top` added for screen mapping.
- `graft::event_loop::graphics` (private): `Graphics<R: GraphicsReplies>` owns `KittyImages` + `StripCache`; `drain` (non-blocking `ReplyWaiter::read` at `Instant::now()`, `answer` each, status = last error or ok); `frame` (strips → `Place`s cropped to the column, `hide` when no pixels); `release` (`free_all`).
- `event_loop`: `pump` → `pump_with(.., &mut Graphics)`; `run` opens its own `mux.waiter()`. Drain each wake; a status change alone redraws on a `Wake::Terminal`. `draw` writes image commands after `terminal.draw`, inside `hold_output` and before END_SYNCHRONIZED_UPDATE. Uploads freed after the loop however it ends.
- Redetect waiter: confirmed it ignores graphics replies (parsers match CSI/OSC only); test added with an APC `ENOENT` mixed into a cell-size answer.

Verified (tests only, byte streams; nothing looked at):
- `event_loop` tests, pixel session + 100 commits with `strip_rows` set by hand: first frame transmits and places, says `gfx ok`; later frames transmit nothing; no `ESC[2J`, no `q=2` in the whole stream; every transmitted id gets `a=d,d=I` after the last frame. Scripted `ENOENT` for id 1000 → next frame transmits again and header bytes contain `gfx: ENOENT:image not found`. F2 to the status screen → `a=d,d=i` placement removal and no gfx status.
- `graphics::tests`: crop/row mapping with and without the uncommitted row.
- `graph_screen` tests: blank column same width, text columns unchanged; status named by state.
- Not verified: anything in a real terminal (24d). Resize still goes through ratatui's clear with no `forget_uploads` (24d).
