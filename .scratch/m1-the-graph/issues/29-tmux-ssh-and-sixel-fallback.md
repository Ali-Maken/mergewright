# m1-the-graph/29 — Pixel tier inside tmux and over SSH; Sixel-only terminals

Status: ready
Build with: **Opus** (high) — terminal graphics protocol passthrough
Blocked by: 24, 25
Leaves runnable: yes — inside tmux the graph is pixels when passthrough answers, clean Unicode otherwise.

Why: the retired pixel-protocols ticket (14, text in 35bec3e) owned tmux passthrough, SSH and the Sixel-only gap. The re-scope pointed 24 and 25 at "26" for them, which was never written.

## Read first
- DESIGN.md §4.3, §4.4
- ADR-0014 §4 and §5 only; ADR-0016 (the Sixel paragraphs only)
- KNOWN_GAPS.md rows "tmux passthrough and SSH untested", "No Sixel encoder is planned", "Sixel and plain-text rendering tiers unproven"
- Code: `MAP.md` of `graft-term`; 24's placement layer; `detect::tier`

## Scope
In:
- tmux: wrap Kitty and iTerm2 sequences in the DCS passthrough; keep ADR-0014's "pixel fails closed inside tmux unless the re-probe answered"; otherwise Unicode.
- SSH: detection over SSH where env vars lie — the longer probe window (DESIGN.md §4.4) and a clean fallback.
- Sixel-only terminal with no truecolor: ADR-0016 keeps no Sixel encoder, so `detect::tier` must not select a pixel tier with nothing behind it. Make it fall to Unicode (or ASCII) explicitly, and say which.
- Fill TESTING.md's tmux and SSH rows with the exact commands.

Out:
- A Sixel encoder. Changing the probe sequence.

## Signatures
Expose (in `graft-term`, update MAP.md):
- `pub fn tmux_passthrough(sequence: &[u8], out: &mut Vec<u8>)` — doubles every `ESC` inside, wraps in `ESC P tmux; … ESC \`.
Call: 24's placement/encoder API; `detect::tier`; 20's reply-waiter API.

## Acceptance
- [ ] Byte-stream tests: passthrough wrapping, ESC doubling, chunked Kitty upload wrapped per chunk.
- [ ] Inside `$TMUX` with no passthrough answer → Unicode, asserted; with an answer → pixel.
- [ ] Sixel-only + no truecolor → the named text tier, asserted; the KNOWN_GAPS row closed.
- [ ] **The user looks at it** inside tmux and over one SSH session — box stays unticked until then. Exact commands prepared.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
