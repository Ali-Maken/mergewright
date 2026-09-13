# m1-the-graph/29 — Pixel tier inside tmux and over SSH; Sixel-only terminals

Status: done (built; the user's tmux/SSH look is still open)
Build with: **Opus** (high) — terminal graphics protocol passthrough
Blocked by: 24d, 25
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
- [x] Byte-stream tests: passthrough wrapping, ESC doubling, chunked Kitty upload wrapped per chunk.
- [x] Inside `$TMUX` with no passthrough answer → Unicode, asserted; with an answer → pixel.
- [x] Sixel-only + no truecolor → the named text tier, asserted; the KNOWN_GAPS row closed.
- [ ] **The user looks at it** inside tmux and over one SSH session — box stays unticked until then. Exact commands prepared.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Result (2026-09-13, Opus)

- `graft_term::tmux`: `tmux_passthrough(sequence, out)` and `Transport { Direct, TmuxPassthrough }`
  (`Transport::for_tmux`: anything but `TmuxStatus::Outside` wraps, so a forced pixel tier inside
  tmux is at least attempted through the passthrough). `KittyImages::with_transport` wraps every
  APC command alone, upload chunks included; `ItermImages::with_transport` wraps each band; cursor
  save/move/restore and erases stay outside for tmux. `Graphics::with_transport` set at startup
  from `capabilities.tmux`.
- **Bug fixed:** the detection probe's wrapper ended `ESC ESC \`, so the passthrough never
  terminated in a real tmux; the re-probe now uses the shared wrapper (single `ESC \`).
- Sixel-only: **ASCII** without truecolor (Unicode with truecolor or inside tmux); protocol still
  reported `Sixel`. `view::draws_pixels` already kept a forced pixel tier off Sixel.
- SSH: no code change needed; the 500 ms budget and silence fallback exist and are asserted
  (`ssh_buys_the_terminal_an_extra_round_trip`).
- Tests: tmux.rs (wrap, doubling, append, round trip, transport), kitty (per-chunk wrapping and
  unwrap-equals-direct, hide/free wrapped), iterm2 (per band, erases unwrapped). Detect tier
  tests already covered tmux answer/no answer; the Sixel test now asserts ASCII.
- Captures: `docs/shots/29-probe-fake-tmux-no-answer.txt` (fake `$TMUX`, no tmux binary →
  `VersionUnknown`, Unicode), `docs/shots/29-probe-fake-ssh-no-answer.txt` (SSH env → budget
  500ms, Ascii). Bare `script` pty: stdin is not a live terminal, so "TimedOut in µs" is the
  harness, not the budget; these show the tier decision only.
- **Not verified:** any real tmux (not installed here) or SSH session; image placement in a
  split tmux pane; whether tmux forwards `q=1` error replies. Exact commands in TESTING.md
  "Must degrade, not break". Fourth acceptance box stays unticked until the user runs them.

NOTES: no compaction.
