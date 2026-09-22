# the-box-has-no-left-border

Status: ready
Blocked by: **32**, then **33** (shares `graft-ui`).
Crates: `graft-ui`, `graft`.
Build with: **Opus.**
Scout: not needed.

## Found by the user, in the running app, at his own width

**This is the fourth instance of the fault ticket 30 was supposed to have
killed, and it was found by eye in five minutes after the sweep passed.**

`docs/field-notes.md` 2026-09-22. Preview box over the graph, `m` on a
`feature/demo` row, terminal ~200 columns:

- **The box has no left border.** Top rule, right `│` and bottom are drawn;
  the left column is graph. Lane glyphs occupy the cell the border belongs in.
- **A character is eaten.** The undo line reads `ndo: u takes this back` — the
  `u` of `undo:` overwritten by a lane glyph.

## The mechanism (conductor's hypothesis — **verify before building to it**)

`crates/graft-ui/src/graph_screen/layout.rs::snap_to_columns`:

- `:597-608` — the **right** boundary maps through `x.checked_sub(1)`, so it
  **steps back one cell**: that is the gutter ticket 29 added.
- `:596` — the **left** boundary is `boundaries.iter().filter(|x| *x <= rect.x).max()?`,
  a bare maximum with **no step at all**.

So the left edge snaps flush onto a column start while the right edge is inset
by one. The box's left border then occupies the first cell of that column,
which the graph also draws into.

**Verify this against a drawn frame before fixing it.** The conductor reasoned
it from the source and the user's capture and did **not** draw it — rule 7:
a report you could not confirm is a stop, not a premise. If the real cause is
different, say so and fix that instead.

## What to build

1. The fix at the seam, symmetric: **whatever the right edge does for the
   gutter, the left edge does too.** One rule, not two.
2. **The sweep has to reach the real app.** Ticket 30's `border_rules.rs`
   passes while the running app fails, so the sweep is not testing what ships.
   Find why and say so plainly — candidates: it never draws the box over a
   *real* graph with lanes; it never uses the widths a real terminal has; it
   snaps against synthetic boundaries. **Widen it until this case is red
   before the fix**, and quote the failure.
3. **ADR-0040 says "a box covers whole columns, or it clears."** Here it does
   neither — it covers part of a column. Say which clause was violated.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] A box drawn over a graph with lanes at ~200 columns has a left border,
      and no glyph from underneath is inside it — test: name it, **red before
      the fix**, and quote the `undo:` line before and after.
- [ ] Left and right edges are computed by one rule — evidence: quote it.
- [ ] Why ticket 30's sweep missed this, stated in the test file itself.
- [ ] Every shipped capture that draws a box re-checked at a wide width.

Rule 6: the user judges it. He already did.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g34`.
Never bare `git stash`. Commit, never push.
