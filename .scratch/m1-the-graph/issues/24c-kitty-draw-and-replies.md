# m1-the-graph/24c — Draw Kitty strips in the app, drain replies, gfx status

Status: ready
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
- [ ] A simulated `ENOENT` reply through the app path causes a re-transmit and a visible `gfx:` status.
- [ ] Exit frees every image (byte-stream test).
- [ ] Never `ESC[2J` in steady state (asserted).
- [ ] Say what you verified and how; no claims about how it looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.
