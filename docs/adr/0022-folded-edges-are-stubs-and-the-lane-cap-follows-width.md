# Folded edges are stubs, the count leaves the rows, the lane cap follows width

Status: **Proposed** (planner-fable, 2026-09-13; needs the user's approval at
the M1 graph design checkpoint). Supersedes DESIGN.md §3.5's third and fourth
bullets and the "default 8" in §3.2 step 5; rewords PROMPT.md §1.2 "compress
them behind a `+N` indicator that expands on demand".

## Context

The user walked git/git in the pixel tier
(`docs/shots/user-37-git-git-pixel-after-scroll.png`): every row read `+54`,
every merge's second-parent curve crossed the kept lanes and stopped at the
`+54` column, and "the 8-lane default is wrong for merge-heavy history".

Two facts explain it. `Fold::plan` decides which *columns* fold for a whole
window, so N is a per-window number that was being printed on every row.
And a column holding a node in the window is never folded, so every folded
lane is nodeless where it is folded: a curve into the marker says only
"the second parent is behind the fold", forty times per screen.

## Decision

**The default view stays all-parents with a lane cap.** Neither auto-folding
merged branches nor auto-switching to first-parent view is the default: both
hide rows, both flip the picture under the user at a threshold, and both
erase the one thing a merge-heavy history has to show — that merges curve
in. `1` (first-parent) and `Space` (fold a merge) stay one key away.

What a fold *draws* changes:

1. **A folded edge is a stub on its node, not a curve to the marker.** In the
   gap cell right of the node: `▾` for an outgoing edge (the merged branch
   continues below, behind the fold), `▴` for an incoming edge (a folded
   branch forked here, above), in the branch's identity colour. ASCII: `v`
   and `^`. Pixel tier: a tail one lane long, curving the same way, whose
   alpha fades to zero. Several folded edges on one row draw one stub in the
   first edge's colour. Nothing crosses kept lanes to reach the fold and
   nothing ends in open space.
2. **The marker column is a rail, not a counter.** `⋮` in `text-muted` on
   every row (ASCII `:`, pixel a 1 px dotted vertical in the muted hue). The
   count moves to the header, after the filter words: `+54 lanes`, shown only
   while something folds. `+` expands and collapses as before; with nothing
   folded, no rail and no count (a status field shows only when there is
   something to report).
3. **The cap follows width.** `graph.max_lanes` unset means: 8 lanes under
   100 columns, 10 at 100–159, 12 at 160 and above — DESIGN.md §1's
   breakpoints. The ceiling is 12 because the lane palette has 12 hues;
   past that, colour stops telling lanes apart, so more lanes add width
   without meaning. An explicit `graph.max_lanes = N` holds at every width.

Fold ranking (least-recently-touched first; never lane 0, a reserved lane,
a pinned column or a column with a node in the window) is unchanged.

## Consequences

- `Fold::view` returns a `DrawnRow { row, folded: Vec<FoldedEdge> }` instead
  of remapping folded edges onto the marker; `Row.lanes` at the rail column
  is always `None`. `raster::RasterRow` gains `stubs`; `raster::Marker` loses
  `folded`; the strip cache's layout hash covers `stubs`.
- The pixel tier no longer rasterises digits (the 3×5 bitmap font goes),
  which also keeps the count out of cached strips: a per-window number baked
  into strips would change the key on every scroll.
- The header count is the text tier's fold (visible rows). The pixel tier
  plans one fold across every strip drawn (KNOWN_GAPS); the two may differ
  by a lane or two at strip edges. Accepted; the header is the screen's.
- Clicking the rail to expand (DESIGN.md §3.5) is not built; `+` is. Logged
  in KNOWN_GAPS by ticket 39.
- Config: `Graph::max_lanes` becomes `Option<usize>`; `DEFAULT_MAX_LANES`
  (8) is the narrow-width value. Values below 2 stay a parse error.
- The curve label (merged branch name, DESIGN.md §3.4) has no room on a
  stub. The label is recovered from the merge's subject, which sits on the
  same row, so nothing is lost on a folded edge.

## What would reopen this

The user's eye on `docs/shots/39-*`, `40-*`. If stubs still read as
"dangling", the fallback within this decision is a dim dashed connector from
the stub to the rail (`◉▾┄┄⋮`), which reintroduces crossings but not
ambiguity. If 12 lanes at 160+ columns read as a barcode, drop the wide
value to 10.
