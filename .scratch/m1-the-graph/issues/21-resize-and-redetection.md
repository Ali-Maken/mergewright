# m1-the-graph/21 — Resize and re-detection

Status: done
Build with: **Opus** (high) — signals and terminal re-queries through the mux
Blocked by: 20
Leaves runnable: yes — resizing and switching the terminal's theme are picked up live.

Why: the pixel tier needs a trustworthy cell size after every resize, and a theme switched while Graft was in the background should not leave it painting the old one.

## Read first
- docs/design/04-terminal-capability-detection.md §4.5
- ADR-0012 (the tiny-terminal and text-budget findings only)
- Code: 20's reply-waiter API, the cell-size provenance type in `crates/graft-term/src/detect/`, the SIGWINCH path in the event loop

## Scope
In:
- SIGWINCH: re-query cell size (`CSI 16 t`, then `TIOCGWINSZ`, then the assumed value), keeping M0's provenance; an assumed size is marked degraded, not equal.
- `OSC 11` re-queried on focus regain; a silent terminal leaves the theme unchanged.
- Explicit guards for 0×0 and tiny sizes: no panic, no one-byte text budget.
- The change is an observable event the pixel renderer (23) can subscribe to.
- The tier is still decided once per session.

Out:
- What the pixel renderer does with a new size (23–25).

## Acceptance
- [x] A scripted resize changes the reported cell size and its provenance.
- [x] An unanswered re-query keeps the previous value and marks it assumed.
- [x] Focus regain updates the theme hint; a silent terminal leaves it unchanged (test).
- [x] 0×0 and 1×1 under a pty never panic.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Notes (builder, Opus)

- `graft-term::detect::{remeasure_cell_size, requery_theme}`: fenced single-query rounds; silence keeps the previous size marked `Assumed`, and keeps the previous theme unchanged. `WindowPixels::cell_size` now rejects zero-pixel cells (fewer pixels than cells).
- `graft::event_loop::redetect::Redetector`: worker thread; resize thread asks `CellSize`, input forwarder asks `Theme` on `FocusGained`; bursts coalesce; a changed answer arrives as `Wake::Terminal(TerminalChange)`, applied by `Session::apply` (tier untouched), then redraw. That wake is what the pixel renderer (23) subscribes to: read `session.capabilities.cell_size` after `apply` returns true.
- Query writes and whole frames share one stdout lock (`redetect::hold_output`): crossterm writes one sequence in several pieces, so an unlocked query could land inside it.
- pty: `.scratch/m1-the-graph/pty-redetect-check.py` (steps / tiny / tiny1); captures `docs/shots/21-*.txt`.
- Not handled: the 50 ms split-reply gap (now also mid-session, where split replies become keys); late-reply misattribution. Both in KNOWN_GAPS.md.
- No compaction.
