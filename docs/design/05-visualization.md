# Mergewright: Design — Visualization

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Terminal-capability detection](04-terminal-capability-detection.md) · Next: [Undo journal design](06-undo-journal-design.md)

---

## 5. Visualization

Three optional overlays on top of the graph pane from §1.1–§1.3: Map view,
Focus mode, and the Overview strip. All three are off by default and cost
nothing when off — no extra column, no extra pass, no permanent chrome added
to the quiet default view PROMPT.md §1.1 describes. Each is a toggle, not a
mode you back out of through a menu: press the key again, or `Esc`, to
return to exactly the view you left.

### 5.1 Map view

Zooms all the way out: one line per branch, plotted against a time axis
instead of one line per commit. It answers "what's been happening, and
where" before committing to reading any single commit's history — the
question the per-commit graph is deliberately bad at, since it shows
everything at the same resolution.

```
╭─ MAP · main ─ last 4 months ──────────────────────────────────────────────╮
│ BRANCH                                                              NOW   │
│ ▸ main             a  ▁▁▂▁▃▁▁▂▇▁▁▁▂▁▃▁▁▂▁▁▅▁▁▂▁▁▁▂▁▁▇▁▁▂▁▁▁▂▁▁▁▂▁▁▁▁▁▁  ● │
│   feature/auth     b     ▁▂▃▇▁▁●╮                                         │
│   feature/billing  c         ▁▁▂▃▁▇▁▁▁▂▁▁▁▁▁▁  (active)                   │
│   release/2.3      d                       ▁▂▇  [v2.3.0] ⓘ                │
│ ⋮                                                                          │
╰ ↑↓ select branch  ⏎ zoom to commits here  M / Esc: back ──────────────────╯
```

- **Shows:** every local branch (a remote collapses into its upstream's row
  unless it's diverged) as one row: a density sparkline bucketed by commit
  date, the branch's identity colour and lane letter, a merge elbow
  (`●╮`/`╯`) where its line joins the branch it merged into, and the
  Mistake Radar/tag markers already attached to the commits it's built from.
  The current branch is prefixed `▸`, its row running to a `NOW` marker at
  the right edge.
- **Toggle:** `M` in the Graph/history pane (§2.3). `Enter` on a selected
  branch row zooms into the ordinary per-commit graph (§1.1–§1.3), scrolled
  to that branch's tip; `M` or `Esc` returns to Map view from there, `Esc`
  alone exits Map view back to the normal graph.
- **Rendering tiers (§4):** tier-aware like the main graph. Pixel tier
  rasterises a real density curve per branch through the same
  Kitty/iTerm2/Sixel path the pixel graph renderer already uses. Unicode
  tier buckets commit counts into block-density characters (`▁▂▃▄▅▆▇█`), one
  per column, coloured by identity. ASCII tier drops to a bounded character
  ramp (` .:-=+*#%@`, clamped to what a bucket's count maps to) so density
  stays legible with no colour and no block glyphs.
- **Cost:** deliberately cheaper than the graph layout algorithm (§3) it
  sits next to. Map view never assigns lanes or resolves crossings — it only
  needs, per branch, a count of commits per time bucket, a single walk of
  that branch's history with no lane-occupancy state to track. It's built on
  toggle-on, not maintained continuously, and discarded on toggle-off.

### 5.2 Focus mode

Selecting a commit fades every lane that isn't an ancestor or descendant of
it, so a tangled multi-branch history collapses, for a moment, to just the
one thread the selected commit sits on.

```
│ ●a      [main]          Fix token refresh race            RM   2m   a1b2c│
│ │ ●b    [feature/auth]  Add OAuth provider                 ·    ·    ·   │
│ ◉a──╯b  ⚠ [main]        Merge 'feature/login'             RM   3h   3c2b1│
│ ●a                      Bump deps                          ·    ·    ·   │
│ ◉a──╯a  ⓘ               Merge branch 'main' of origin     RM   2d   9f8e7│
╰ Focused on a1b2c3d — ancestors/descendants stay lit, rest fades ──────────╯
```

- **Shows:** full-strength lane colour and detail text on the selected
  commit's ancestor/descendant path; every other row keeps its shape (lanes
  don't disappear — the graph's topology stays legible) but drops to a
  low-saturation, dimmed render. In the pixel tier this dimming is alpha
  only, never a wider or heavier stroke on the on-thread rows (ADR-0024:
  a heavier stroke read as the graph's default state, not a highlight,
  since the cursor's own ancestry is often lane 0's full length). The mockup stands in for that dimming the
  same way §1.7's ghost nodes stand in for a lower alpha: `·` marks columns
  that would render muted, not blank. This is a deliberately different
  visual vocabulary from Ghost Preview's dashed `┊`/`┈` (§1.7) — a faded row
  is a real commit that's simply off the thread you asked about; a ghost row
  isn't a real commit at all.
- **Toggle:** `F` in the Graph/history pane (§2.3), applied to the currently
  selected row (selecting the topmost row first if nothing is selected yet).
  Moving the selection while Focus mode is on recomputes the fade from the
  new row; `F` or `Esc` turns it off and restores full-strength rendering
  everywhere.
- **Rendering tiers (§4):** applies at every tier — it's a paint-time
  attribute on rows the layout algorithm already produced, not a layout
  change, so it costs the same one extra bit of per-row state (`Row.faded`
  or equivalent) whether the renderer is pixel, Unicode or ASCII.
- **Cost:** answers the same question the detail pane's "Contained in" field
  (§1.1) already answers — is this row an ancestor, descendant, or neither,
  of the selection — over the rows currently laid out and visible. It's an
  O(visible rows) walk on selection change, not a new pass over history.

### 5.3 Overview strip

A compressed, non-1:1 column at the graph pane's scroll edge — its own right
edge in the narrow/tab layout (§1.3), just inside the detail pane's border
in the wide/medium layout (§1.1–§1.2) — showing the whole history's shape in
one column, independent of how much of it is currently laid out or on
screen.

```
╭─ ◆ mwr ─── ⎇ main ↑2 ↓0 ────────────────────────────────────────── ⌃P ──╮
│ GRAPH   REF          SUBJECT                     AUTHOR  WHEN   SHA   ┆ │
│ ●a      [main]       Fix token refresh race        RM    2m    a1b2c ┆ │
│ │ ●b    [feature/..] Add OAuth provider            AK    1h    7f6e5 █ │ ← viewport
│ ◉a──╯b  ⚠ [main]     Merge 'feature/login'         RM    3h    3c2b1 █ │
│ ●a                   Bump deps                     RM    1d    0a9b8 ┆ │
│ ⋮                                                                    ⚠ │ ← radar hit further down
│                                                                       ◆ │ ← search hit
│                                                                       ┆ │
│                                                                      ●h │ ← HEAD
╰────────────────────────────────────────────────────────────────────────╯
```

- **Shows:** one glyph per bucket of the full commit range for whichever
  markers fall there — `◉` for a merge, `⚠`/`ⓘ` for a Mistake Radar hit, `◆`
  for a search hit, `●` plus the branch's lane letter for HEAD/the current
  branch's tip — over an otherwise-blank `┆` column, plus a solid block
  (`█`) marking which rows the main pane currently has on screen. It's a map
  of "where are the interesting things," not a second copy of the graph.
- **Toggle:** `O` in the Graph/history pane (§2.3). Independent of Map view
  and Focus mode; toggling Map view on hides the strip automatically, since
  Map view already shows the whole history's shape, and restores it on
  toggling Map view back off if it was on before.
- **Rendering tiers (§4):** plain text/glyphs at every tier — there's no
  pixel variant, since a one-cell-wide column gains nothing from
  rasterizing. It reads whichever tier the main graph is already using only
  to draw its own glyphs consistently (lane letters, marker characters).
- **Cost:** piggybacks on data the incremental layout (§3.6) is already
  producing rather than triggering a new history walk. Merge and Mistake
  Radar markers come from `Row.marker` on whatever's already in the `rows`
  buffer; search hits come from the search job already scanning history for
  the active search (§3.6's "search hit at row 900,000" example is exactly
  this). For a huge repo whose full history hasn't been walked yet, the
  strip is honestly incomplete above/below what's been laid out so far and
  fills in as background streaming catches up — a navigational aid, not a
  guarantee every marker in history is already plotted.

---
