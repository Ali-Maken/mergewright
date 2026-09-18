# m0-foundations/04 — Terminal-capability detection (`term::detect`)

Status: done (2026-09-12, Opus)
Build with: Sonnet
Blocked by: 01

Why: the tier decision drives everything the UI draws; M0 must detect and display it so M1 can build renderers on top. Biggest M0 ticket — stop early with a handoff note rather than rush.

## Read first
- docs/design/04-terminal-capability-detection.md §4 (all of it — this ticket implements it)
- ADR-0012 ("What worked": probe chain, Kitty round-trip detection; "What didn't": probe/typeahead conflict)
- `prototypes/graph-spike/` probe code as a working reference — throwaway quality; adapt ideas, don't copy blindly
- docs/design/08-architecture.md §8.2 for which types land in `graft-core`

## Goal
`graft-term`: `Capabilities::detect(...)` implementing docs/design/04-terminal-capability-detection.md §4.1–§4.4 — env hints, batched active queries with a deadline, per-query fallback with provenance, tmux handling, and the §4.2 tier decision.

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
- [x] Parser unit tests per reply type, incl. malformed and truncated replies (use byte strings captured in the spike/ADR as fixtures).
- [x] Driver test with a scripted fake: full-answer terminal → pixel tier; silent terminal → falls back per-query and still returns within budget; tmux-too-old → unicode.
- [x] Every fallback value carries provenance (visible in the struct, later in diagnostics).
- [x] All tests laptop-safe, no real terminal needed; total added test time well under 10 s.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## What was built

- `graft-core::capabilities` — `Capabilities` and the provenance types (`CellSizeSource`, `ThemeSource`, `Supported`, `TmuxStatus`, `ProbeOutcome`). Plain data, no behaviour (ADR-0007).
- `graft-term::detect` — `detect(&Environment, &mut dyn TerminalHost) -> Capabilities`, with `reply` (escape-sequence scanner + per-reply parsers), `probe` (the six queries, the `DA1` fence, tmux passthrough wrapping) and `tty` (the only module that touches the real terminal: `poll`, `read`, `TIOCGWINSZ`, `tmux -V`).
- `cargo run -p graft --example capability_probe` — manual check; prints the whole report, no alternate screen.
- 46 tests in `graft-term`, all under 10 ms.

## Deviations from the letter of the spec (flag at the checkpoint)

1. **`Capabilities::detect(...)` is a free function, `graft_term::detect::detect(...)`.** `Capabilities` lives in `graft-core` (docs/design/08-architecture.md §8.2) and Rust cannot put an inherent `impl` on a foreign type — and ADR-0007 forbids the behaviour living in `core` anyway. Same call, different spelling.
2. **Detection takes the environment as a value, not from `std::env`.** `Environment::from_process()` reads the variables once; everything below is a pure decision over that value, which is what makes the tier table testable without `unsafe { set_var }`.
3. **Inside tmux, `iterm2_images` is false unless the passthrough re-probe answered.** tmux blocks `OSC 1337` the same way it blocks the Kitty protocol, so an env hint that survives into tmux would otherwise promise images that never draw.
4. **A tmux session with no reachable protocol gets Unicode even when §4.2's table would say ASCII** (e.g. no `COLORTERM`). §4.3 is the more specific rule and says so explicitly.
5. **The tmux re-probe gets a fresh budget**, so a tmux session can spend up to 2x the deadline at startup. §4.3 mandates the second round and §4.4 only sizes a single one.

## Verified how

- Scripted-fake tests: the tier table, both fallback chains, all four tmux outcomes, the SSH budget (asserted on the deadline handed to `read`, so no test waits).
- One run in a bare pty (`script -q /dev/null`): the six queries go out in order with `DA1` last; a terminal that answers nothing times out at 201 ms against the 200 ms budget and falls back per query, with every fallback flagged.
- **Not verified: any real terminal answering.** See the M0 rows in KNOWN_GAPS.md — the manual runs (Ghostty, iTerm2, Kitty, WezTerm, Terminal.app, Zed, VS Code, tmux with and without passthrough, SSH) are outstanding.
