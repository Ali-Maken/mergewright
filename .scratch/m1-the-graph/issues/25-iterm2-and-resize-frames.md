# m1-the-graph/25 — iTerm2 and resize without retransmitting the viewport

Status: ready
Build with: **Opus** (high) — terminal graphics protocol and frame-time work
Blocked by: 24d
Leaves runnable: yes — iTerm2 shows the pixel graph; resizing rebuilds only what changed.

Why: the spike spent 32–41 ms per resize retransmitting the whole viewport against a 16 ms budget, and 7 ms re-encoding unchanged iTerm2 bands.

## Read first
- ADR-0012 (the iTerm2 and resize findings only)
- DESIGN.md §3.8 (the 16 ms frame budget paragraphs)
- Code: 24's placement layer, 23's cache API, 21's resize event

## Scope
In:
- iTerm2 inline images re-sent per frame, with encoded bands cached by (offset, viewport) so an unchanged band is not re-encoded.
- On SIGWINCH, for Kitty and iTerm2: re-place and rebuild only strips whose key changed.
- Measure the resize frame against 16 ms and report it honestly.

Out:
- tmux and SSH (29).

## Acceptance
- [ ] iTerm2 encoder byte-stream tests.
- [ ] An unchanged band is not re-encoded (counter asserted).
- [ ] A resize does not retransmit the whole viewport (strip count asserted); measured frame time reported, including if it still exceeds 16 ms.
- [ ] Exact command prepared for the user in iTerm2. **The by-eye box stays unticked until the user looks.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
