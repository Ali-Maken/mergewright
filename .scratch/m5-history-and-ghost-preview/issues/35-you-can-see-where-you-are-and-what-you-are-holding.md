# you-can-see-where-you-are-and-what-you-are-holding

Status: ready
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

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] The cursor row differs from its neighbours in the rendered buffer by a
      named attribute — test: name it. Say whether the old `selection.rs`
      asserted something weaker, and what.
- [ ] Which of the two causes (A) was — assertion too weak, or style not
      applied — stated with the line.
- [ ] A drag in progress is visible in the buffer **before release**: the
      target row, the hint bar, and whatever you chose for the source badge —
      test: name each. Say where the chain was broken.
- [ ] Nothing regressed for a reader with no colour — `lanes_without_hue.rs`
      still passes, and the cursor is still findable without hue. Name the test.
- [ ] Captures regenerated where the grid changed. **Colour itself still ships
      for the user's eye** — you do not tick "looks right."

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g35`.
Never bare `git stash`. Commit, never push.

## Report

Which of A's two causes it was. Where B's chain broke. What you chose for the
source badge and why. Anything left that only the user's eye can settle.
