# ADR-0041 — Feedback is function; presentation is later

Date: 2026-09-22
Status: accepted (the user's decision, 2026-09-22)

## Decision

**Build the functionality first and completely. Defer presentation research
to M8.** One carve-out, and it is the whole point of this record:

> **"Can the user tell it happened?" is functionality, not presentation, and
> is never deferred.**

Two groups, and the line between them:

| Not deferrable — ships with the feature | Deferred to M8 |
|---|---|
| Where the cursor is | Whether the layout is the best layout |
| That a drag picked something up | Whether a pane should follow the cursor |
| That a key registered | Colour schemes, spacing, aesthetics |
| That an operation is in flight or refused | What competitors do better |

The M8 research is the conductor's to do, not the user's, and the user's
question about Sublime Merge's cursor-following detail pane is its first
input (`docs/design-notes/detail-on-cursor-move.md`).

## Why

The obvious split — "functionality now, UI later" — does not cut cleanly, and
M5 is the evidence. In **five minutes** in the running app the user found:

- the cursor row is *"really and I mean really hard to notice"*
- a drag shows nothing until release — *"no indication that I have held
  something"*
- the preview box has no left border, and a lane glyph eats a character

**All three passed a fully green suite.** None is presentation. A drag with no
feedback is not a working drag; it is a working drag *in the tests*. Every key
acts on the cursor row, so a cursor you cannot find is not a finished feature.

Filing these as "UI polish, later" would have meant building M6's conflict
resolver and M7 on top of a view where you cannot tell which row you are on —
every later milestone inheriting the fault and each one making it dearer to
fix. The cost of not deferring is one ticket (m5/35).

## Consequences

- **Rule 5 is narrower than it has been read.** It says *captures* cannot prove
  colour — true, `.txt` grids drop attributes. It has been read as *colour
  cannot be tested*, and every milestone has shipped "colour unverified" as a
  standing gap. `TestBackend`'s `Buffer` carries fg, bg and modifier per cell,
  and six test files already assert style. **Feedback and visible state get
  buffer assertions.** What stays the user's (rule 6) is whether a colour is
  *legible in his terminal and theme* — not whether an attribute was applied.
- A feature is not done when its logic is tested. It is done when a test shows
  the user could tell. Builders should expect that in acceptance lists.
- Presentation findings still get recorded when spotted — in
  `docs/field-notes.md`, as input to M8 — but they do not open tickets now.

## What this does not say

It does not license shipping something ugly. It says: when the two compete for
the same hour before M8, the hour goes to *can the user tell*, and the rest is
written down for the pass that will do it properly.
