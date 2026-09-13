# m1-the-graph/24 — Pixels on Kitty-protocol terminals

Status: ready
Build with: **Opus** (high) — terminal graphics protocol
Blocked by: 19, 23
Leaves runnable: yes — Ghostty and other Kitty-protocol terminals show the pixel graph; everything else unchanged.

Why: the user's daily pixel terminal is Ghostty. The spike proved it works — and proved how easily it silently doesn't.

## Read first
- ADR-0012, including its Correction
- DESIGN.md §4.2
- CLAUDE.md, the status-field rule
- Code: the Kitty code in `prototypes/graph-spike/` (reference only), 20's reply-waiter API, 23's cache API, 19's renderer override

## Scope
In:
- Transmit strips once with ids; place with a source crop; replace one placement id per frame (`p=1`); free everything on exit.
- Never `ESC[2J` between transmit and placement. Never `q=2`: use `q=1`, route replies through the mux.
- Status field named by state: `gfx ok` / `gfx: <error>`.
- A placement that returns `ENOENT` re-transmits that strip rather than trusting uploads survive.
- Place by cell rectangle, not by pixel-exact cell-size arithmetic.
- `graph.renderer = pixel` added to 19's override.

Out:
- iTerm2 and resize frame cost (25). tmux and SSH (29).

## Acceptance
- [ ] Byte-stream unit tests: chunking, ids, crop per scroll step, placement replacement, free on exit.
- [ ] A simulated `ENOENT` reply causes a re-transmit and a visible `gfx:` status.
- [ ] An automated run against a real Kitty-protocol terminal reads back `q=1` replies with zero errors. A pty capture is not acceptance.
- [ ] Forcing `pixel` on a terminal that never acked attempts it, and 19's tier-drop key recovers.
- [ ] Exact command prepared for the user in Ghostty. **The by-eye box stays unticked until the user looks.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
