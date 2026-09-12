# m1-the-graph/14 — Getting pixels to the terminal: Kitty, iTerm2, tmux

Status: ready
Build with: Opus
Blocked by: 03, 12, 13

Why: this is where the step-3 failure happened and where it must not happen again. The spike's pty captures proved the bytes were emitted and the graph still drew nothing, because Ghostty frees image **data** on `ESC[2J` and `q=2` hid every `ENOENT`. Every rule below is paid for.

## Read first
- ADR-0012 **including the Correction** — read the Correction first
- DESIGN.md §3.8 and §4.3 (tmux passthrough)
- KNOWN_GAPS.md prototype rows: "Is ED-frees-image-data Ghostty-specific?", "Sixel and plain-text rendering tiers unproven", "tmux passthrough and SSH untested"
- `prototypes/graph-spike/` as a reference, not a source to copy

## Goal
`term` encodes `RasterFrame`s into real pixels on Kitty-protocol and iTerm2 terminals, inside tmux and over SSH, and says so out loud when the terminal rejects something.

## Scope
In:
- Kitty: transmit strips once with ids, then place with a source crop; replace one fixed placement id per frame (`p=1`); free everything on exit. **Never `ESC[2J` between transmit and placement.**
- **Never `q=2`.** Use `q=1` and surface errors in the status field, named by state per CLAUDE.md (`gfx ok` / `gfx: <error>`), never a permanent error-looking label.
- **Do not assume an upload survives a clear on any terminal.** Only Ghostty 1.3.1 was ever tested. Detect a lost image (`ENOENT` on placement) and re-transmit that strip, rather than trusting one terminal's behaviour everywhere.
- iTerm2: re-send per frame, with encoded bands cached by (offset, viewport) so an unchanged band is not re-encoded (the spike wasted 7 ms doing exactly that).
- Placement by **cell rectangle**, not by pixel-perfect cell-size knowledge — that is what made alignment survive the assumed-cell-size fallback.
- tmux: wrap in the passthrough sequence, honour M0's already-decided "pixel fails closed inside tmux unless the re-probe answered" (ADR-0014), and fall back cleanly to Unicode otherwise. The pass condition is PROMPT.md §7.1's: a correct pixel graph **or** a clean fallback, never a broken screen or stray escapes.
- SIGWINCH: re-place and rebuild only the affected strips (ticket 13 owns the cache side).
- Sixel **only if** ticket 03's ADR kept it. If the ADR says Sixel is used only where it beats Unicode and that is unproven, leave it out and put a row in KNOWN_GAPS.md — do not build an unproven tier on spec.
- `TESTING.md`: fill in the pixel rows of the manual checklist (Ghostty, Kitty, WezTerm, iTerm2, tmux with and without passthrough, SSH).

Out:
- Rasterising (13).
- Changing capability detection (M0's, plus ticket 03's diagnostic fix).

## Acceptance
- [ ] Encoder unit tests over the byte stream: chunking, ids, crop arithmetic per scroll step, placement replacement, free-on-exit, tmux wrapping.
- [ ] A simulated `ENOENT` reply causes a re-transmit and a visible `gfx:` status, not a silently blank graph.
- [ ] Tested against a terminal that actually answers — at minimum an automated run that reads back graphics replies with `q=1` and asserts zero errors (the standard the step-3 fix set). **A pty capture is not acceptance** (ADR-0012 Correction).
- [ ] **The user looks at it** in Ghostty and iTerm2, and inside tmux: this box stays unticked until then. Prepare the exact commands.
- [ ] Resize does not retransmit the whole viewport — report the measured frame time on resize against the 16 ms budget, and say honestly if one resize frame still exceeds it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
