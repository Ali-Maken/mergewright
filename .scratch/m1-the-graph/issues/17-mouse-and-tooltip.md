# m1-the-graph/17 — Mouse and the commit tooltip

Status: ready
Build with: **Sonnet** (medium) — event plumbing over existing selection
Blocked by: 16
Leaves runnable: yes — wheel scrolls, click selects, hover shows a tooltip.

Why: mouse is first-class (DESIGN.md §2.15), and everything the mouse shows must also be reachable from the keyboard.

## Read first
- DESIGN.md §2.15 and §2.3 (the tooltip key, if listed)
- Code: the event loop's input handling in `crates/graft/src/event_loop/`, 16's selection, the pty harness M0 used for termios checks

## Scope
In:
- Wheel scrolls, click selects, hover shows a tooltip with the full message and parents.
- A key opens the same tooltip for the selection.
- Tooltip text arrives as `SanitizedText`.
- "Contained in" is left out, not faked: add a KNOWN_GAPS.md row owned by M2.
- Mouse reporting switched off on every exit path.

Out:
- Anything needing containment reads (M2).

## Acceptance
- [ ] Synthetic mouse events: wheel scrolls, click selects the row under the pointer, hover opens the tooltip.
- [ ] The tooltip opens from the keyboard with identical content (test).
- [ ] After quit, the pty byte stream disables mouse reporting (asserted).
- [ ] A message containing an escape sequence renders inert in the tooltip.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
