# m0-foundations/04 — Terminal-capability detection (`term::detect`)

Status: todo
Build with: Sonnet
Blocked by: 01

Why: the tier decision drives everything the UI draws; M0 must detect and display it so M1 can build renderers on top. Biggest M0 ticket — stop early with a handoff note rather than rush.

## Read first
- DESIGN.md §4 (all of it — this ticket implements it)
- ADR-0012 ("What worked": probe chain, Kitty round-trip detection; "What didn't": probe/typeahead conflict)
- `prototypes/graph-spike/` probe code as a working reference — throwaway quality; adapt ideas, don't copy blindly
- DESIGN.md §8.2 for which types land in `graft-core`

## Goal
`graft-term`: `Capabilities::detect(...)` implementing DESIGN.md §4.1–§4.4 — env hints, batched active queries with a deadline, per-query fallback with provenance, tmux handling, and the §4.2 tier decision.

## Scope
In:
- `graft-core` types: `Capabilities`, `RenderingTier` (pixel/unicode/ascii), a theme hint (dark/light/unknown), cell size with provenance (reported / winsize-derived / assumed), colour depth (truecolor/256/16/none), `NO_COLOR` flag.
- Probes per §4.1: Kitty APC `a=q` (round-trip, DA1-fenced per ADR-0012), Sixel via DA1 param 4, `CSI 16 t` cell size, `CSI ? u` kitty keyboard, DECRQM 2026, OSC 11 background. 200 ms deadline, 500 ms over SSH (§4.4).
- Per-query fallback exactly per §4.1 step 3 (cell size → TIOCGWINSZ → assumed 8×16 marked approximate; OSC 11 silence → dark, flagged unknown).
- iTerm2 by env hint alone (§4.1 step 4, the one permitted exception).
- tmux (§4.3): version check, passthrough-wrapped re-probe, else Unicode tier — a decision made once at startup.
- Tier decision per §4.2 table. Config override precedence is NOT applied here — `detect` returns what it detected; the app layer combines it with config (ticket 11).
- I/O behind a small transport trait (write probes / read bytes with deadline) so the driver runs against scripted fakes in tests; the real impl reads the tty directly, before the event loop starts.

Out:
- Image-protocol encoding, rasterising (M1).
- Re-probe on SIGWINCH / focus (§4.5) and the stdin input mux — recorded M1 gaps in KNOWN_GAPS.md already; detection runs once pre-event-loop, and startup typeahead loss is the accepted M0 state.
- The sanitiser (ticket 05).

## Acceptance
- [ ] Parser unit tests per reply type, incl. malformed and truncated replies (use byte strings captured in the spike/ADR as fixtures).
- [ ] Driver test with a scripted fake: full-answer terminal → pixel tier; silent terminal → falls back per-query and still returns within budget; tmux-too-old → unicode.
- [ ] Every fallback value carries provenance (visible in the struct, later in diagnostics).
- [ ] All tests laptop-safe, no real terminal needed; total added test time well under 10 s.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
