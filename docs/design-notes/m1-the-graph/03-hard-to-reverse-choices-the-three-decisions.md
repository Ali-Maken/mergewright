# M1 design pass: the graph after the gallery walk (2026-09-13) — Hard-to-reverse choices (the three decisions)

[See also: m1-the-graph/README.md](README.md) for the full section index.
Previous: [Interfaces and data shapes](02-interfaces-and-data-shapes.md) · Next: [Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)](04-where-id-differ-from-designmd-promptmd-proposed-changes.md)

---

## Hard-to-reverse choices (the three decisions)

### 1. Merge-heavy history by default

Options:
- **(a) Auto-fold merged branches** (PROMPT §1.3 fold) when their lane would
  exceed the cap. Narrow result; but it hides *rows* by default (topic
  commits vanish, `/` finds what the view won't show), which merge fold is
  order-dependent (the 9th merge folds, the 8th doesn't), and the fold
  decision would move into layout — DESIGN §3.5 made compaction a
  rendering-time lens on purpose. Rejected.
- **(b) First-parent above a threshold, with a hint.** Machinery exists
  (`1`). But the threshold is only known once enough rows are laid out, so
  the view flips under the cursor mid-scroll (`1` goes to the top); it hides
  rows too; and it erases the very thing the user expects on git/git
  ("merges curve in"). Rejected as a default; stays one key away.
- **(c) Keep all-parents and the lane cap; change what a fold draws.** The
  cap is right; the drawing is wrong. Three changes:
  1. A folded edge draws as a **stub on the node**, not a curve to the
     marker: `◉▾` (branch continues below, folded), `●▴` (branch came from
     above, folded), in the branch's colour, in the gap cell right of the
     node. Pixel tier: a one-lane tail that fades to alpha 0. No crossings,
     nothing ends in open space.
  2. The marker column becomes a **quiet rail** `⋮` (text-muted), no digits.
     The count moves to the header: `+54 lanes` beside the filters, only
     while something folds. `+` still expands; the rail is its affordance.
  3. **The cap follows width**: `graph.max_lanes` unset → 8 under 100 cols,
     10 at 100–159, 12 at 160+ (ceiling 12 = the palette's hue count; past
     it colour stops telling lanes apart). An explicit number overrides.

**Recommendation: (c).** Honest about what a 50-topic history is (wide),
keeps every row, keeps the reductions (`1`, `Space`) explicit, and removes
both complaints at their cause. ADR-0022.

### 2. The tooltip

Options:
- **(a) Keep and fix:** border, `esc close` title, anchored to the right
  third (over author/date/sha, never over the subject), pin keyed by commit
  id (36). Two tickets of work on a widget M2's pane makes redundant.
- **(b) The user's proposal:** no popup. Truncated subject → full subject in
  the hint bar on hover. Details → M2's pane. Copy SHA → a key and `.`.
- **(c) (b) plus a keyboard peek:** `Enter` shows the cursor row's full
  subject in the same footer line, one-shot (any other key clears it), so
  the reveal isn't mouse-only (PROMPT §5: nothing hover-only).

**Recommendation: (c).** Honest evaluation of (b): it is right. The tooltip
duplicates the detail pane PROMPT §1.1/§1.3 already specify, covers the
graph the milestone is about, fights terminal text selection, and its one
real bug (36) exists only because it has state. The cost is a one-milestone
gap: until M2, no way in Graft to read a commit body or its parents. M2 is
next and is where PROMPT puts them; KNOWN_GAPS names M2. Copy SHA: `y`
(vim yank, unassigned in DESIGN §2.2/§2.3; lazygit precedent) via OSC 52,
plus `.` when the context menu exists (M2+). ADR-0020 superseded. ADR-0023.
Amendment to the proposal: (c)'s `Enter` peek, so keyboard users get the
reveal too; M2 rebinds `Enter` to open the pane (DESIGN §2.2 "drill in").

### 3. Lane 0's weight

Cause (hypothesis, ticket 42 confirms first): thread stroke = heavier, and
the thread is always on. Options: (a) keep heavier, calibrate down; (b)
thread never widens; emphasis by **alpha only** — thread full colour,
off-thread 0.5, Focus-faded rows 0.3; (c) lane 0 always heavy as identity.
**Recommendation: (b).** DESIGN §5.2 already says off-thread is "a
low-saturation, dimmed render"; width was ticket 16/22's substitute. A 2 px
line at full colour among half-alpha lines is a line to follow. Base stroke
`cell_width/8` clamped 1.5–2.5 px (2.1 at the user's 17 px). Unicode keeps
bold/dim (its only tools). ADR-0024.
