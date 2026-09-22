# the-box-has-no-left-border

Status: done
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

## The mechanism, verified — **the hypothesis did not hold**

Drawn, not reasoned. Two findings:

1. **`snap_to_columns` is not on this path at all.** `draw_preview`
   (`crates/graft/src/view/frame.rs:691`) never calls it;
   `preview_box_area` (`crates/graft-ui/src/preview_box.rs:148-164`) returns
   `x: frame.x, width: frame.width` — the box is **always the full frame
   width**. The left/right asymmetry at `layout.rs:596`/`:604` is real and
   is what the *confirm* box uses; it cannot be what the user saw.
2. **The character grid was never wrong.** A sweep of the real app frame
   over a graph with lanes, widths 20..=240 × heights {24,30,40,50} ×
   detail on/off × strip on/off, finds a border glyph in the box's own left
   column on every row, at every size —
   `graft::view::tests::the_preview_box_keeps_its_own_left_border_over_a_graph_with_lanes`.
   Green **before** the fix as well.

**The real cause: the pixel layer.** At the pixel tier the lane column is
not drawn into the buffer. `view_with_graphics` hands back the rectangle the
lane images go in and `event_loop::running::draw`
(`crates/graft/src/event_loop/running.rs:352-360`) writes those images
*after* the whole frame, so the terminal composites them on top of every cell
the frame wrote. Measured at 200x50: lanes in
`Rect { x: 0, y: 1, width: 4, height: 48 }`, preview box in
`Rect { x: 0, y: 38, width: 200, height: 11 }`, overlapping on
`Rect { x: 0, y: 38, width: 4, height: 11 }`. Lanes sit at columns 0 and 2
with gaps at 1 and 3 — which is the box's `│` at 0 and the `u` of `undo:` at
2, the `n` at 3 surviving. `│ undo:` reads `ndo:`. Exactly the report.

**ADR-0040 is not violated.** The box covers whole columns; it covers all of
them. The fault is that the graph is drawn *over the box*, not the box over
part of a column. ADR-0040 governs one layer and the fault is between two.

**The fix, one rule for every overlay:** `frame::pixels_above_overlays` —
each `draw_*` returns the rectangle it cleared, and the lane rect is cut back
to the rows above the topmost overlay that overlaps it. Applied on the graph
screen and in `draw_map`.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] A box drawn over a graph with lanes at ~200 columns has a left border,
      and no glyph from underneath is inside it — **two tests, because the
      fault turned out to be in two layers**:
      `graft::view::tests::the_preview_box_keeps_its_own_left_border_over_a_graph_with_lanes`
      (the grid; green before and after — it is the floor, not the evidence)
      and
      `graft::view::tests::the_lane_images_never_cover_a_box_drawn_over_the_graph`
      (the pixel geometry; **red before the fix at every size**, first
      failure `20x24: … they share Rect { x: 0, y: 22, width: 4, height: 1 }`,
      quoted in the test's own doc comment together with the 200x50 numbers).
      The `undo:` line before: `│ undo: u takes this back` in the buffer,
      with the lane image over columns 0 and 2 → `  ndo: u takes this back`
      on screen. After: the lane rect stops at row 37, the box's rows are
      the box's.
- [x] Left and right edges are computed by one rule — **the finding is that
      this box computes neither**: `preview_box_area` takes `frame.x` and
      `frame.width` whole (`crates/graft-ui/src/preview_box.rs:158-163`), so
      there is no asymmetry to remove. The one rule this ticket did add is
      `pixels_above_overlays`, which every overlay goes through —
      `graft::view::tests::the_lane_images_never_cover_the_confirm_box_or_the_credential_prompt`
      holds it for the confirm box and the credential prompt as well.
- [x] Why ticket 30's sweep missed this, stated in the test file itself —
      three reasons, in the doc comment of
      `the_preview_box_keeps_its_own_left_border_over_a_graph_with_lanes`:
      `border_rules.rs` draws each widget into a bare buffer with nothing
      behind it; `pane_boundaries.rs` checks only the cells *beside* the box
      and skips the left check entirely when `rect.x == frame.x`, which is
      where this box always is; and neither can see the pixel layer, which
      is where the fault was. **Not width** — both already sweep 20..=240.
- [x] Every shipped capture that draws a box re-checked at a wide width —
      the sweep above is that check, at every width to 240 and with the
      detail pane and the strip both ways. The committed captures
      (`docs/shots/m5-*`) are all regenerated and green under
      `capture_*_for_docs_shots` and the walk.

Rule 6: the user judges it. He already did.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g34`.
Never bare `git stash`. Commit, never push.


## NOTES

- **No scout** for this half: the work was a geometry question in two named
  files, and the answer had to come from a drawn frame rather than a summary.
  A scout was used for ticket 35's two chains.
- **The pixel layer is invisible to every test we have**, and this is the
  general form of the finding: `.txt` captures are character grids, buffer
  sweeps read the same grid, and the images are written to the terminal
  afterwards. The only thing a test can hold is the *geometry* — that the
  rect handed to the graphics layer and the rect a widget drew into do not
  overlap. That is what the new tests hold. **Whether the images then land
  where the rect says is still unverified by anything but the user's eye.**
- Related: `draw_journal` and `draw_stash` take the overlay rectangles and
  drop them, because those screens draw no pixels at all. Named, not silent.
