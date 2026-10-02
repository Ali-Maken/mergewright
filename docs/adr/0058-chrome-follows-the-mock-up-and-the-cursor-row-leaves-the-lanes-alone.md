# ADR-0058: The chrome follows the mock-up; the cursor row's fill leaves the lanes alone; dark theme B

Status: accepted (user, 2026-10-02: "the real app should look like the mock-up"; dark A vs B left to the planner, to
be picked by measured contrast). Amends ADR-0039 (the bar's bare-key rung), DESIGN.md §1.1–§1.3 (borders, header,
footer), §1.15 (empty state). Evidence: `.scratch/m8-polish-and-ship/real-vs-mockup.md` (six differences),
`docs/shots/m8-proto-*.txt`.

## Context

On his phone the prototype looked better than the product (field notes 2026-10-02). Side by side, same font and
renderer: the real header is `graft` and a full path cut with `…`; the bar is ~35 bare letters running off the edge;
no gutter, no space between regions; outside a repo, two lines of error text; the branch label on the cursor row is dim
magenta on grey.

Contrast (prototype scene 4, `docs/shots/m8-proto-4a-dark-a-b.txt:21-24`, WCAG formula):

| | today | A (ground darker) | **B (text brighter)** | floor |
|---|---|---|---|---|
| cursor row vs ground | 3.04 | 3.05 | **3.14** | 3.0 |
| text on cursor row | 4.28 | 4.61 | **5.14** | 4.5 |
| muted text on ground | — | 6.15 | **6.95** | 4.5 |
| blue lane on cursor row | — | 1.33 | 1.20 | 3.0 |

No flat fill clears 3:1 against all twelve lane hues (KNOWN_GAPS row 542: lanes span luminance 0.12–0.60; lane-3
`#AA3377` on today's selection ≈ 1.02:1). Picking A or B cannot fix the lane; only where the fill is drawn can.

## Decision

1. **Dark theme B**: `text` brighter, ground (`bg` `#14161B`) unchanged, `selection` raised to 3.14:1. B beats A on
   every floor it can meet, keeps the ground his other tools share, and its one loss (the lane on the fill) goes away
   with decision 2.
2. **The cursor row's fill starts after the graph column.** The selection fill covers the marker slot to the right
   edge; the left gutter and the graph column keep the pane's ground, at every tier (a Pixel strip's transparent
   pixels show ground). Lanes on the cursor row then contrast as on every other row (worst: lane-3 on ground ≈ 2.97:1,
   unchanged from today, ADR-0003's band). The cursor is found by a `▸` in the left gutter (accent, bold), the thread
   emphasis (ADR-0024) and the fill.
3. **Text on the fill is legible by rule.** Any foreground drawn on `selection` that is under 4.5:1 against it is
   mixed toward white (dark) or black (light) in 5% steps until it clears 4.5:1 (`Theme::legible_on`). A branch badge
   keeps its hue family and becomes readable. At 16 colours and `NO_COLOR` it draws `text`, bold.
4. **Chrome** (all screens with a repository open; DESIGN.md §1.0):
   - Header: ` ◆ <repo name>  <~/path> · <branch>`, repo name bold accent, path muted and cut from the left; the
     informational tail (walk, filters, gfx, operation banner) right-aligned. No product name in it.
   - A two-cell left gutter on list rows (cursor glyph, gap); header and bar indented one cell; one cell right margin.
   - At 30 rows and taller: a blank row under the header and a rule (`─`, border token) above the bar. Below 30 rows,
     neither: the rows go to the list.
   - Bar: hints joined with ` · `. **Outside modal contexts, a hint is drawn with its words or not at all**: the bare-key
     rung is gone; what does not fit is behind the doorway (`. more`, then `? keys` once `?` lands — ADR-0056). Modal
     contexts (resolver, banner, boxes, prompts, preview) keep ADR-0044 §4's ladder: there the keys are the point.
5. **The drop target's slot glyph becomes `→`** (ASCII `=`), so the cursor's `▸` and a drop target never look alike
   (m5/35's rule).
6. **Outside a repository**: the empty state of the mock-up (`docs/shots/m8-proto-3a-empty.txt`), DESIGN.md §1.15.

## Why not the alternatives

- **Dark A:** lower margins on all three floors; changes the ground he matches against Zed and Ghostty.
- **Per-lane contrast tweaks on the cursor row:** recolours a lane on one row only; at the Pixel tier the strip is
  an image drawn without knowing the cursor (ADR-0053).
- **Keep bare keys in the bar:** what he saw as ~35 letters running off the screen; `?` now lists every key here.

## Consequences

- Every capture with a bar, a header or a cursor row goes stale; the look-parity group re-shoots them.
- `graph_screen::CHROME_ROWS` grows by two at ≥ 30 rows; every geometry mirror (`update::geometry`, mouse row math,
  `pane_boundaries` sweeps) reads the one `ScreenRows` split.
- Colour ships **unverified** (CLAUDE.md rule 5); ratios are asserted by tests, legibility is his eye.
