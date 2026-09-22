# you-can-see-where-you-are-and-what-you-are-holding

Status: done
Blocked by: **34** (same crate, same files — one builder takes both).
Crates: `graft-ui`, `graft`.
Build with: **Opus.**
Scout: not needed — both faults were found in the running app and are below.

## Both findings are the user's, from five minutes in the real app

`docs/field-notes.md` 2026-09-22. **Both are attribute/colour faults, which is
the one class the whole test suite cannot see** (rule 5: the pty pipeline and
the `.txt` captures record character grids with no attributes). He found both
almost immediately. That is the measure of how big the hole is.

### A. "Really and I mean really hard to notice which row I'm on."

The cursor row does not read as selected at a glance in his terminal and
theme. **This is the most basic affordance in the application** — every key he
presses acts on that row.

`crates/graft-ui/tests/selection.rs` exists and passes. So either it asserts a
style that is too weak to see, or it asserts a style the real frame does not
apply. **Find out which, and say which — they need opposite fixes.**

### B. A drag gives no feedback until you let go.

His words: *"when I'm dragging there is no indication that I have held
something, until I release and a popup shows up."*

Two things were built for exactly this and **neither reached him**:
- `RowPaint::Highlighted` on the row under the pointer (ticket 10)
- `HintContext::Dragging` → `release drop on a row  esc cancel` (ticket 10,
  swept by ticket 31)

Both are tested. Both are invisible in use. **Establish where the chain breaks
before changing anything** — candidates: the state is never set on a real drag;
it is set and never drawn; it is drawn in an attribute his terminal renders
indistinguishably. Say which, with the line.

Note the asymmetry worth thinking about: **nothing indicates the badge itself
has been picked up.** Even a correct target highlight leaves the user unsure
they are holding anything. Consider what the *source* should look like while in
the air — that is a design call, and it is yours to make, but say what you
chose and why.

## The bigger point, which is the real deliverable

**Rule 5 has been over-applied for the whole project.** It says *captures*
cannot prove colour — true, the `.txt` format drops attributes. It has been
read as *colour cannot be tested*, and every milestone has shipped "colour
unverified" as a standing gap. That is false:

- `TestBackend`'s `Buffer` carries **fg, bg and modifier per cell**
- **six test files already assert style** — `graft-ui/tests/lanes_without_hue.rs`,
  `selection.rs`, `graph_column.rs`, and three more

So: **write these as style assertions, not as notes.** A test that the cursor
row differs from its neighbours by an actual rendered attribute, and a test
that a drag in progress is visible in the buffer. If a fault above turns out to
be "the style is applied but too subtle", say so plainly — that part is the
user's call and **you do not tick it** (rule 6).

## What was found, and what was built

### A — which of the two causes: **the assertion was too weak.**

The style was applied and is applied still:
`graph_screen/render.rs::row_emphasis` painted `Modifier::BOLD` on the text
and filled the row with `Token::SurfaceRaised`
(`crates/graft-ui/src/graph_screen/render.rs:236-249` before this ticket).
It was applied **too faintly to find**: `#14161B` → `#232730` in the dark
theme (`crates/graft-ui/src/theme/chrome.rs:36`), about a six-percent step
in lightness, with the foreground colour unchanged and no shape at all.

`crates/graft-ui/tests/selection.rs` could not have caught either a weak
background or a missing one: its only helper, `code()` at
`selection.rs:104-115`, reads `cell.modifier` and maps every cell to
`b`/`d`/`.`. **`cell.bg` and `cell.fg` are read nowhere in the file.** A
cursor row with no background at all would have kept every snapshot green.
The only tests in the repo that did assert the fill were unit tests inside
the crate (`graph_screen/tests.rs:173-183`), and they assert that it *is*
`SurfaceRaised` — which is true and says nothing about whether it can be
seen.

**Built:** the `▏` accent rule in the marker slot is now drawn at **every**
colour depth, not only at `Palette16`/`None` where the terminal had no
background to give (`graph_screen::render::row_rule`). A shape survives every
theme, every depth and a black-and-white screenshot. The fill stays.

### B — where the chain broke: **the last hop, the rendered attribute.**

Everything upstream was sound and the scout confirmed it with lines: `Drag`
events decode (`graft-term/src/input/decode/mouse.rs:38-55`), reach
`drag_mouse` (`graft/src/update/drag.rs:68-79`), set `drag.over`
(`drag.rs:74`), and the loop redraws unconditionally after every event
(`event_loop/running.rs:135-148` — no dirty flag exists anywhere in the
crate). `hint_context` returns `Dragging` first (`view/rows.rs:325-327`).
None of that was wrong.

It broke here, `view/entries.rs:129-135` as it was:

    if crate::update::drag::drop_target(state) == Some(index) {
        entry.line.paint = RowPaint::Highlighted;
    }

Two faults in three lines. The drop target took **the cursor row's own
paint**, and it took it *instead of* the cursor's — the comment above it said
so on purpose. So the whole of a drag, on screen, was one six-percent
background raise moving from one row to another. And before the pointer first
moved, `over` is `None`, so there was no target row either, and the footer was
the only thing that had changed.

Nothing marked the source. `drag.from_row` is stored (`drag.rs:98`) and read
only by `release()`; it appears in no rendering file.

**Built:** `RowPaint::DropTarget`, its own variant, drawn as `▸` in the
marker slot in `Token::Accent` — a different shape from the cursor's `▏`, so
both rows are on screen and neither is mistaken for the other.

### The source badge — the design call, and why

**Chosen: the row the badge came from fades (`RowPaint::Dimmed`).**

The alternatives considered and rejected. Naming the held badge in the hint
bar would mean `Hint.key`/`Hint.verb` stop being `&'static str`
(`graph_screen/hints.rs:67-79`), which is the table ticket 31 built its
invariant on — too much churn for a line in the footer, which is not where a
hand on the mouse is looking anyway. Carrying the badge itself onto the row
under the pointer is the truest metaphor and the largest change; it wants the
badge pipeline in `entries.rs`, and it is the thing to do next if fading is
not enough.

Fading is the honest picture of what has happened: the badge is not on that
row any more, it is in the user's hand. It needs no new variant, no new
plumbing, and it is the mark that lands on the button **down**, before any
motion — which is precisely the moment he said nothing happened.

**Its one cost, stated:** drag a badge off the row the cursor is on — the
common case, and what `docs/shots/m5-ui-dragging-*.txt` captures — and the
fade takes the place of the cursor's own `▏`. One row, one state. The
capture's caption says so rather than claiming otherwise.

### Found by the capture, not by reasoning

Taking `m5-ui-dragging-80x24.txt` showed the cursor row carrying `ⓘ` and **no
rule**: the Mistake Radar's marker had precedence in the slot, so on any row
with a finding the fix switched itself off and the cursor was back to the
background alone. The rule now takes the slot's first cell and the marker its
second (`▏⚠`), and the columns after it do not move —
`graft_ui::graph_screen::tests::a_row_with_a_marker_still_shows_the_cursor_rule`.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The cursor row differs from its neighbours in the rendered buffer by a
      named attribute — test:
      `graft_ui::selection::the_cursor_row_differs_from_its_neighbours_by_three_named_attributes`,
      which asserts **three**, each against the row above *and* the row
      below: `bg == Token::SurfaceRaised` across the full width (columns 0,
      20, 40 and 79), a `▏` in `Token::Accent` with `Modifier::BOLD`, and
      `Modifier::BOLD` on the subject. The old `selection.rs` asserted
      something weaker: `code()` (`selection.rs:104-115`) reads
      `cell.modifier` only — BOLD/DIM — and never `cell.bg` or `cell.fg`, so
      the background was asserted **nowhere in the file**.
- [x] Which of the two causes (A) was — **the assertion was too weak**, and
      the line is `crates/graft-ui/tests/selection.rs:104-115`. The style was
      applied (`graph_screen/render.rs::row_emphasis`) and remains applied;
      it was too faint to find. Written up under "A" above and in the doc
      comment of the new test.
- [x] A drag in progress is visible in the buffer **before release** — three
      tests, one per mark:
      `graft::view::tests::a_drag_in_progress_is_visible_in_the_buffer_before_the_button_comes_up`
      (the target row's `▸` in `Token::Accent`, and that it does *not* also
      carry `▏`; the source row's subject DIM against an untouched row's; the
      cursor's own rule still drawn),
      `graft::view::tests::the_hint_bar_names_the_gesture_while_a_badge_is_in_the_air`
      (`release`, `drop on a row`, `esc` in the footer), and
      `graft::view::tests::picking_a_badge_up_changes_the_screen_before_the_pointer_moves`
      (the press alone, `over: None`, changes the frame).
      `graft_ui::graph_screen::tests::a_row_with_a_marker_still_shows_the_cursor_rule`
      holds the slot-sharing. Where the chain broke: `view/entries.rs:129-135`
      as quoted under "B" — not input, not state, not redraw.
- [x] Nothing regressed for a reader with no colour — `lanes_without_hue.rs`
      passes (its 256-depth snapshots regenerated with the rule, one row
      each), and the cursor is findable without hue by
      `graft_ui::selection::without_colour_the_cursor_row_is_still_marked_by_its_shape`,
      which runs at `ColorDepth::None` and `Palette16`, finds the `▏` on the
      cursor row and not on the row above, and asserts there is **no**
      background step at those depths — so the shape is carrying it alone.
- [x] Captures regenerated where the grid changed — `m5-ui-context-menu-verbs-*`,
      `m5-ui-name-prompt-80x24`, `m5-ui-drop-menu-*` (the cursor rule), and a
      new pair `m5-ui-dragging-{80x24,160x48}` for the mid-drag frame that
      nothing captured before, with caption rows added to
      `m5-contact-sheet-captions.tsv` and the sheet re-run (62 captures, 0
      without a caption). Held by `capture_a_drag_in_flight_for_docs_shots`.
      **Not ticked: whether any of it is legible in his terminal and theme.**
      That is rule 6 and it is his. What is ticked is that the attributes are
      applied and distinct in the buffer.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g35`.
Never bare `git stash`. Commit, never push.

## Report

Which of A's two causes it was. Where B's chain broke. What you chose for the
source badge and why. Anything left that only the user's eye can settle.

## NOTES

- **Scout used** (`Explore`, sonnet) for both chains — the selection styling
  and the drag path end to end. Both reported code with lines, and that is
  what made "the assertion was too weak, not the style" answerable without
  guessing. Ticket 34's half used no scout: it needed a drawn frame, not a
  summary.
- **Rule 5 was being over-applied, and the ticket is right about it.**
  `TestBackend`'s `Buffer` carries `fg`, `bg` and `modifier` per cell and
  nothing stopped us asserting them. The new helper
  `graft::view::tests::drawn_cells` is `draw_cells`'s sibling for exactly
  this and says so in its doc comment. What rule 5 actually forbids is
  treating a `.txt` capture as evidence about colour — unchanged.
- **Still only the user's eye can settle:** whether the raised fill is
  legible in his terminal and theme; whether `▏` and `▸` are distinct enough
  at a glance; whether fading the source row reads as "picked up" rather than
  as Focus mode's fade; and whether `▏⚠` is legible in two cells.
- **Left open, deliberately:** the dragged badge is not drawn on the row
  under the pointer. If fading the source is not enough feedback, that is the
  next step, and it wants the badge pipeline in `view/entries.rs`.
