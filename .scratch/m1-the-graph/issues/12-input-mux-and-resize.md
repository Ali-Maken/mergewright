# m1-the-graph/12 — One owner of stdin, and re-detection on resize

Status: ready
Build with: Opus
Blocked by: 10

Why: two readers on fd 0 is the hack ADR-0012 flagged and M0 shipped: keys pressed during the probe window are eaten, and cell size is never re-queried when the font size or display changes. The pixel tier (13/14) cannot be correct on resize until this exists, and every tier loses typeahead until it does.

## Read first
- ADR-0012, "What didn't" (probe/typeahead conflict, crossterm can't own stdin during probes, degenerate window sizes) and "What M1 must do differently"
- DESIGN.md §4.5 (re-detection triggers)
- KNOWN_GAPS.md M0 row "The probe eats typeahead"
- `crates/graft-term/src/detect/tty.rs` and `crates/graft/src/event_loop/`

## Goal
One component owns stdin for the process's life: probe replies go to the prober, everything else becomes a key event, and nothing is lost.

## Scope
In:
- The input mux: a single reader that classifies incoming bytes, routes terminal replies to whoever is waiting for one, and preserves everything else as input. Detection stops reading the fd itself.
- Typeahead pressed during startup detection arrives after it, in order.
- `SIGWINCH`: re-query cell size (`CSI 16 t`, then `TIOCGWINSZ`, then the assumed value, keeping M0's provenance), and treat an assumed cell size as degraded rather than equal.
- `OSC 11` re-queried on focus regain, so a terminal theme switched while Graft was in the background is picked up (DESIGN.md §4.5).
- Explicit guards for 0×0 and tiny terminal sizes — no panic, no one-byte text budget (ADR-0012).
- Tier itself still decided once per session; this ticket does not make the tier change mid-run.

Out:
- What the pixel renderer does with a new cell size (13/14) — but make the change observable to it.
- Changing the probe sequence or the tier table.

## Acceptance
- [ ] Under a pty: send keys during the probe window and assert every one arrives as an event afterwards, in order (the M0 harness style — record the measurement).
- [ ] A scripted resize changes the reported cell size and its provenance; an unanswered re-query keeps the previous value and marks it assumed.
- [ ] Focus-regain re-query updates the theme hint; a silent terminal leaves it unchanged rather than flipping to a guess.
- [ ] 0×0 and 1×1 terminals draw something or draw nothing, but never panic.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
