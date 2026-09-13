# m1-the-graph/24d — Cache flush, F3 from pixels, real-Ghostty check

Status: ready
Build with: **Opus** (high) — terminal graphics protocol
Blocked by: 24c
Leaves runnable: yes — pixel graph survives resize/theme changes; F3 recovers; user has a command to judge by eye.

Split from 24. 24's "Handoff" section (bullet **24d**) is the spec. Closes 24's remaining acceptance boxes.

## Read first
- Ticket 24, "Acceptance" and "Handoff" sections
- ADR-0012, including its Correction
- CLAUDE.md, "Visual results are judged by eye"
- `crates/graft-term/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft/MAP.md`

## Scope
In:
- `StripCache::flush` from `Session::apply` (CellSize/Theme) and resize; resize also `forget_uploads`.
- F3 from Pixel drops to Unicode (today always Ascii, `update.rs` `is_tier_drop`); `update` needs the settled tier in `AppState`.
- `graph.renderer = pixel` with no protocol ack → attempt Kitty; F3 recovers.
- Automated real-Kitty-protocol check: transmit + place at `q=1`, DA1 fence, count APC errors, write a report file. A pty capture is not acceptance.
- Raw escape capture + decoded summary under `docs/shots/`; exact Ghostty command for the user written into this ticket.

Out:
- iTerm2 and resize frame cost (25). tmux/SSH (29).

## Acceptance
- [ ] Theme / cell-size / resize flush the cache (tests); resize retransmits.
- [ ] Forcing `pixel` on a terminal that never acked attempts it, and F3 recovers to Unicode.
- [ ] Automated real-terminal run reads back `q=1` replies with zero errors — or, if no Kitty-protocol terminal is reachable from the builder, say so plainly and leave the box unticked.
- [ ] Capture + decoded summary committed; Ghostty command in this ticket. **By-eye box stays unticked until the user looks.**
- [ ] When all boxes are met, mark 24 done too.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.
