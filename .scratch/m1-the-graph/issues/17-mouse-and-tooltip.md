# m1-the-graph/17 — Mouse and the commit tooltip

Status: done
Build with: **Sonnet** (medium) — event plumbing over existing selection
Blocked by: 16
Leaves runnable: yes — wheel scrolls, click selects, hover shows a tooltip.

Why: mouse is first-class (docs/design/02-keymap.md §2.15), and everything the mouse shows must also be reachable from the keyboard.

## Read first
- docs/design/02-keymap.md §2.15 and §2.3 (the tooltip key, if listed)
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
- [x] Synthetic mouse events: wheel scrolls, click selects the row under the pointer, hover opens the tooltip.
- [x] The tooltip opens from the keyboard with identical content (test).
- [x] After quit, the pty byte stream disables mouse reporting (asserted).
- [x] A message containing an escape sequence renders inert in the tooltip.

## Notes

**A context compaction happened mid-ticket.** All production code (scroll.rs,
graph_screen/mod.rs, view.rs, update.rs) was written before the compaction;
after it, I resumed by writing the tests, running the gates, adding the
`KNOWN_GAPS.md` row, updating MAP.md, and capturing the pty shots. Nothing
was left unfinished — the ticket is complete — but flagging per CLAUDE.md.

Design/judgement calls made and not pre-specified in DESIGN.md:
- **Keyboard tooltip key: `Enter`.** docs/design/02-keymap.md §2.3 only reserves it for a
  sidebar row, which M1 has none of. Free on the graph pane.
- **Wheel moves the viewport only** (`History::scroll`, changes `top`, not
  `cursor`) — distinct from `j`/`k`, matching every other terminal pager.
- **Click moves the selection directly** (`History::select_at`), clamped to
  rows laid out so far.
- **`Esc` closes the tooltip first**, before withdrawing a `G` jump, before
  leaving Focus mode, before cancelling the walk — the lightest thing on
  screen goes first.
- `AppState.tooltip: Option<usize>` is a single absolute history row index,
  set identically by hover and by `Enter`, and read through one
  `view::tooltip_view` lookup — this is what makes "identical content"
  hold trivially rather than by convention.

Verified: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --workspace` all pass (200 tests in `graft`, full workspace
green). New tests: `update.rs` (click, wheel, hover/Enter parity, Esc
priority), `view.rs` (escape-sequence-inert tooltip, hover/keyboard content
equality), `graph_screen/tests.rs` (tooltip overlay renders). Mouse-off-on-
exit is covered by the pre-existing `terminal.rs` test
`teardown_undoes_every_step_in_reverse_order`, which already asserts the
`DisableMouseCapture` sequence on every teardown path — no new test needed.

Pty captures (real release binary, real pty, byte-level SGR mouse input,
rendered with pyte — not a TestBackend snapshot):
`docs/shots/17-mouse-select-and-tooltip.txt` (hover tooltip, `Enter` opening
the same content, `Esc` closing it, click moving the selection, wheel
scrolling the viewport while the selection stays put) and
`docs/shots/17-tooltip-sanitizes-escape.txt` (a commit whose subject and
body carry a raw `\x1b[31m...\x1b[0m` — both the graph row and the tooltip
show the sanitizer's placeholder, never a live escape).

"Contained in" deferred to M2: `KNOWN_GAPS.md` row added.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
