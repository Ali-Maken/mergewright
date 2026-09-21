# ADR-0040 — A box covers whole columns, or it clears

Date: 2026-09-21. Decided by the user mid-M5, judging
`docs/shots/m5-ui-confirm-over-graph-80x24.txt` and
`docs/shots/m5-ui-credential-over-graph-80x24.txt` by eye. Status: accepted.
Completes the clause m5/27 left pending in
`docs/design-notes/m5-history-and-ghost-preview.md` §"Pending: the
border/background distinction".

## The decision

> **A non-clearing box covers whole columns only. It never splits one. If no
> snap position exists that covers whole columns and still fits its content,
> it clears.**

Three clauses make it total.

1. **Both edges move.** The rule is about splitting, not about the right
   edge. A box whose left edge lands mid-subject fails on the left however
   tidy its right edge is.
2. **Snap outward if it fits, else clear.** When the nearest boundary would
   grow the box past the screen, it clears instead. **Never shrink the box's
   content to reach a boundary** — truncating the text to satisfy a layout
   rule trades a legible fragment beside the box for an illegible one inside
   it.
3. **The snap is computed at the layout site, not inside the widget.**
   `ConfirmView` and the credential prompt must not learn the graph's column
   geometry; that would couple a generic widget to one screen and break
   PROMPT.md 6.1. Whoever lays out the screen already knows both, so the
   rect arrives computed and the widget stays dumb.

## Why a geometric rule and not a per-box ruling

A border makes a box's edge explicit, so content beside it reads as
*background* rather than as part of the box — **and that is only true while
whole columns survive beside the box. Fragment a column and the border does
not save you**: a reader cannot tell a clipped column from a box that leaked
into it. Both halves of that sentence are the decision; neither stands alone.

The accident that exposed it, in one sentence: at 80 columns the credential
prompt's right border landed inside the short-SHA column and left ` b8c5`,
`68ff`, `00bd`, `3e93`, `1ece` against it — the tails of five real SHAs,
reading as four-character identifiers that are not — while the confirm box
beside it kept whole SHAs only because "Push main → origin/main" happens to
be the length it is. Same surface class, same border, opposite outcome,
decided by a string length. A ruling on either box would have rested on that
accident, so the rule is stated over the geometry instead.

## What each box does

- **The credential prompt clears its band, at every width.** Two reasons,
  either sufficient: it fragments the SHA column, and nothing behind it
  serves the task — `docs/design/02-keymap.md` §2.14 makes it **modal**, and
  the footer already carries the context (`push main → origin/main · esc
  cancels`). The graph behind it is decoration next to a secret. It clears
  **the band it occupies**, full width — not the screen. The footer stays.
- **The confirm box does not clear. It snaps.** The rows beside it are the
  graph the user is about to act on, and seeing them is M5's promise ("you
  see the result before you confirm"), not a leak.

## How it is built

- `graft_ui::graph_screen::GraphScreen::column_boundaries(area)` hands out
  the x boundaries the rows were actually drawn to — the same
  `Columns::compute` the body and the footer already share, exposed as data
  rather than recomputed, so the snap and the draw cannot disagree about
  where a column is.
- `graft_ui::graph_screen::snap_to_columns(rect, boundaries, bounds)` is the
  whole of the rule and is pure: it knows nothing about the graph and
  nothing about either box. Both halves arrive as numbers.
- `graft::view::frame` composes them. That is the layout site clause 3 names.

**Measured consequence, for the record:** with the frame's own edges in the
boundary set, an outward snap always exists inside the frame, so the confirm
box never actually reaches the clearing branch at any width from 20 to 240.
The branch is the rule's floor, not dead weight — a future overlay anchored
somewhere other than the frame's centre can reach it — but nothing today
does, and this ADR should not be read as claiming otherwise.

## Also decided here: text never touches a border's rule

Every bordered surface puts a space on **both** sides of a label in its
border, as `frame_box::border_row` has always done. The credential prompt
kept a private copy of that helper that put one only on the left, which drew
`╭ Credentials — 127.0.0.1:8443─────╮` and
`╰ ⏎ send  ⌃U clear  esc cancel the push────╯`. A rule running straight into
a port number reads as part of the host. The copy is gone; the four
surfaces that draw a label into a border share one implementation and are
checked as a class (`crates/graft-ui/tests/border_rules.rs`).

## Considered and not taken

- **Ruling on each box separately.** The two boxes differ by a few cells of
  title. A rule that turns on that is a rule that will be wrong the next
  time a title changes.
- **Clearing both boxes' bands.** Simpler, and it throws away the graph
  under the confirm box — which is the one thing the confirm box exists to
  show.
- **Shrinking a box to reach the boundary below it.** Rejected by clause 2:
  it moves the illegible fragment from outside the box to inside it.
