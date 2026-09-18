# m1-the-graph/25 — iTerm2 and resize without retransmitting the viewport

Status: done (by-eye check in iTerm2 pending — user)
Build with: **Opus** (high) — terminal graphics protocol and frame-time work
Blocked by: 24d
Leaves runnable: yes — iTerm2 shows the pixel graph; resizing rebuilds only what changed.

Why: the spike spent 32–41 ms per resize retransmitting the whole viewport against a 16 ms budget, and 7 ms re-encoding unchanged iTerm2 bands.

## Read first
- ADR-0012 (the iTerm2 and resize findings only)
- docs/design/03-graph-layout-algorithm.md §3.8 (the 16 ms frame budget paragraphs)
- Code: 24's placement layer, 23's cache API, 21's resize event

## Scope
In:
- iTerm2 inline images re-sent per frame, with encoded bands cached by (offset, viewport) so an unchanged band is not re-encoded.
- On SIGWINCH, for Kitty and iTerm2: re-place and rebuild only strips whose key changed.
- Measure the resize frame against 16 ms and report it honestly.

Out:
- tmux and SSH (29).

## Acceptance
- [x] iTerm2 encoder byte-stream tests (`graft-term/src/iterm2.rs`, 8 tests).
- [x] An unchanged band is not re-encoded (`ItermImages::encoded` asserted, unit + loop test).
- [x] A resize does not retransmit the whole viewport (0 transmits, rasterised count unchanged, 0 flushes asserted); frame time measured below.
- [x] Exact command prepared for the user in iTerm2 (below).
- [ ] **By eye in a real iTerm2 — user.** Unticked until the user looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Result (builder, 2026-09-13)

What changed:
- `graft-term::iterm2::ItermImages`: OSC 1337 bands re-sent per frame, PNG (`Compression::Fast`) cached by (strip pixels, crop, cell rect); last frame's cells erased with `ECH` first; `doNotMoveCursor=1`. New dep `png 0.18` in `graft-term`.
- Root cause of resize retransmit: ratatui's `ESC[2J` on a size change, not the resize. `event_loop::row_clear::RowClear` backend turns `ClearType::All` into per-row `ESC[2K`. Loop no longer flushes strips or forgets uploads on resize (lane columns are in the layout hash).
- `view::draws_pixels` accepts `PixelProtocol::Iterm2`; `Graphics` picks the encoder by protocol.

Verified, and how:
- Real Ghostty 1.3.1, `examples/kitty_check` (extended): row erase then placements only → 0 errors; full `ESC[2J` then placements only → 2/2 ENOENT. `docs/shots/25-kitty-resize-check-report.txt`.
- pty capture of the real binary, iTerm2 tier forced (100x30 → SIGWINCH 80x30 → q): 1 band before and 1 after the resize, byte-identical, 0 `ESC[2J`, 28 ECH, 0 Kitty bytes. `docs/shots/25-iterm2-capture.txt`, `.esc`, band `25-iterm2-capture-band.png` (16x448). Script `.scratch/m1-the-graph/iterm2_capture.py`. A pty proves bytes, not pixels.
- Frame time, release, in-memory backend (terminal write cost excluded): Kitty first frame 8.8 ms, **resize 0.30 ms**; iTerm2 first 4.2 ms, unchanged 0.26 ms, **resize 0.25 ms**. Debug: Kitty resize 0.97 ms, iTerm2 0.77 ms. Under 16 ms as measured; real-terminal cost with write back-pressure not measured (ticket 30).

Not verified:
- No one has looked at the iTerm2 output in a real iTerm2 (installed, but the builder cannot see the screen).
- An actual Ghostty window resize was not driven (the clear path was). If Ghostty drops uploads on a real resize, `q=1` ENOENT retransmits next wake.

Proposed change: docs/design/03-graph-layout-algorithm.md §3.8 lists terminal resize as a wholesale strip flush. The key already carries lane columns, so the flush is unnecessary; propose dropping resize from that list (`Flush::Resize` left in the API, unused by the app).

User command (iTerm2 ≥ 3.5; forces the pixel tier, since iTerm2 with truecolor settles on Unicode by ADR-0016):

```
cd /Users/devuser/dev/sandbox/gitter && cargo build -p graft && mkdir -p /tmp/graft-iterm/graft && printf '[graph]\nrenderer = "pixel"\n' > /tmp/graft-iterm/graft/config.toml && XDG_CONFIG_HOME=/tmp/graft-iterm ./target/debug/graft .
```

Look for: lane graph drawn as an image beside the commit text, rows aligned; j/k scroll keeps alignment; resize the window — graph redraws in place, no leftover image fragments; F3 drops to Unicode and the image disappears.

NOTES: not compacted.

