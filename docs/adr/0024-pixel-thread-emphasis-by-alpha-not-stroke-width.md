# Pixel tier: the ancestry thread is emphasised by alpha, never by stroke width

Status: **Accepted** (user, 2026-09-13, judged from the file). Was: Proposed (planner-fable, 2026-09-13; needs the user's approval at
the M1 graph design checkpoint). Reverses the judgement call in tickets 16
and 22 ("on-thread heavier"). Changes no PROMPT.md or DESIGN.md text: it
brings the pixel tier back to docs/design/05-visualization.md §5.2, which says off-thread rows
"drop to a low-saturation, dimmed render" and says nothing about width.

## Context

The user: "Lane 0 renders as a thick slab, not a line to follow." In
`docs/shots/user-37-git-git-pixel-after-scroll.png` lane 0 is about twice
the width of the other lanes, and so is the lane the top merge's second
parent takes.

The likely cause is not a lane-0 rule. The selection's ancestry thread is
drawn with a heavier stroke (ticket 22's `Emphasis`/`thread`), the cursor
starts on the top row, and the top row's ancestry is all of lane 0 below it.
So "heavier" is the graph's default state, not a highlight — every fresh
open shows a slab. Ticket 42 confirms the cause before changing anything.

## Decision

- **Stroke width is one number for every line in the graph:**
  `cell_width / 8`, clamped to 1.5–2.5 px (2.1 px at the user's 17 px
  cell). Lanes, curves, tails and node rings all use it. The thread does
  not widen; lane 0 does not widen.
- **Emphasis is alpha:** on-thread lanes at full alpha; off-thread lanes at
  `OFF_THREAD_ALPHA = 0.5`; rows faded by Focus mode (`Emphasis::Dimmed`) at
  `FADED_ALPHA = 0.3`. `Emphasis::Highlighted` no longer changes width; if
  nothing distinguishes it from `Normal` afterwards, it is removed.
- **The Unicode tier is unchanged** (bold thread, dim others): text has no
  alpha, and bold `│` is not a slab.

## Why alpha and not a smaller width ratio

A 1.25× stroke is invisible under anti-aliasing; a 1.5× stroke at 2 px is
the slab. There is no ratio that reads as "a line to follow" and also reads
as emphasis. Contrast does both: a full-colour 2 px line among half-alpha
lines is the thread, and it is still a line.

## Consequences

- Raster snapshots (`tests/it/snapshots/it__raster__*`) and
  `docs/shots/22-*.png` change; ticket 42 regenerates and the user judges
  `docs/shots/42-*.png` by eye.
- KNOWN_GAPS row "Focus and the ancestry highlight are bold/dim only" stays
  for the Unicode tier; the pixel half of it closes.
