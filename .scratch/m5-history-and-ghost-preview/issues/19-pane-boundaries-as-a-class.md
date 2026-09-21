# pane-boundaries-as-a-class: assert on the rects, not the cells

Status: **done** (m5-g19, 2d8ac22)
Blocked by: none — `graft-ui` only, runs alongside ticket 18 (`graft`)
Build with: **Opus.** The test design is the deliverable; the two defects are
the proof it works.
Crates: **`graft-ui` only.** Another builder holds `graft` right now. If you
need it, commit at the seam, report `blocked: needs graft`, and stop.
Scout: **not needed.** `layout.rs` is named below; the two defects are named
with their exact rendered text.

## 1. The test, which is the point of this ticket — the user's design

Nothing in the tree asserts pane boundaries **as a class**. The two opacity
tests in `preview.rs` are hand-keyed to specific labels in specific widgets at
two hardcoded sizes, so every new surface needs its own copy — and that
technique **cannot express defect 2 at all**, because that fault is a *missing
column between two panes that are both drawing legitimately*, not extra text
on a known row.

`crates/graft-ui/src/graph_screen/layout.rs` returns a `Rect` per pane
(`graph_area`, `body_layout`, and the rest). **Assert on the rects, not the
cells:**

- Panes never overlap.
- Adjacent panes leave **at least one column** between them.
- **Swept across a range of widths**, not two fixed ones — the defects below
  appear at 160 and not at 120, which is exactly what a two-size test misses.

One test for the class instead of one per instance. Sweep rows too where a
pane's height is computed. Include the banner, which now takes rows of its own
(ticket 16), and the preview box, which docks over the bottom of the graph.

This is the structural answer to a milestone in which six defects shipped
because the tests agreed with the code rather than with the requirement. A
property swept over a range cannot be satisfied by agreeing with one arrangement.

## 2. Defect — no gutter between the graph and the changes pane, at 160

Found by the user by eye in `docs/shots/m5-ui-banner-conflict-160x48.txt`:

```
●       Correct the note                       7y   a82960bCONFLICTS (2)
```

The sha column and the pane's first column touch, so `a82960bCONFLICTS` reads
as one token. Your new boundary test must **go red on this before you fix it** —
if it does not, the test does not yet express the property, and the test is
this ticket's deliverable. Say in your report that you watched it fail.

## 3. Defect — footer verbosity is inverted

At 120 columns:

```
no commit-graph file: history sorts on every open · B build one (git commit-graph write --reachable) · X never here
```

At 160 columns, with **forty more columns available**:

```
no commit-graph file · B build one · X never here
```

A wider terminal gets terser text. Whatever picks the wording is keyed on
something other than available width, or the ordering is reversed. Fix at the
cause and check it against the open width-pressure gap in `KNOWN_GAPS.md`
rather than special-casing 160.

## Acceptance (every tick names its verifying test inline, at tick time)

Unless said otherwise the test is in
`crates/graft-ui/tests/pane_boundaries.rs`; the footer ones are in
`crates/graft-ui/src/graph_screen/tests.rs`. Every sweep runs widths
20..=240 and heights 3, 8, 24, 40, 48.

- [x] Panes never overlap, swept across a width range — test:
      `the_graph_and_the_detail_pane_never_abut_at_any_width` and
      `the_detail_pane_s_own_regions_never_abut_the_graph_or_each_other`
      (`boundary_fault` checks overlap and gutter in one pass).
      Mutation: widening the graph rect by one column so the two rects
      overlap → both fail, "the graph Rect { width: 94 } overlaps the
      detail pane Rect { x: 93 }".
- [x] Adjacent panes leave ≥ 1 column, swept across a width range — test:
      the same two. Mutation: `split_area`'s `Right` arm back to flush →
      "the graph ends at column 93 and the detail pane starts at column 93".
- [x] The sweep covers the banner's rows and the preview box's dock — test:
      `the_banner_never_takes_a_row_another_part_of_the_screen_draws`
      (header / banner / body / offer / hint bar, disjoint, inside the
      screen, in reading order, banner outranks the offer, and
      `graph_area` agrees with the carve) and
      `the_preview_box_docks_inside_the_frame_and_above_the_footer`.
      Mutations: the offer's row condition back to ignoring the banner →
      "the banner ... overlaps the commit-graph offer" at 20x3;
      `preview_box_area` no longer reserving the footer row → "the preview
      box ... overlaps the screen footer".
- [x] The 160-column gutter defect is fixed, and the class test is what
      catches it — test:
      `the_graph_and_the_detail_pane_never_abut_at_any_width`. **Watched it
      fail first**, on the untouched tree: "160x3, Right { columns: 67 },
      override None: the graph ends at column 93 and the detail pane starts
      at column 93: no gutter between them". Also checked at the cell level
      in a throwaway render (not committed): at 160x48 the sha now ends at
      column 91, column 92 is blank, the pane still starts at 93.
- [x] Footer wording gets no terser as width grows, swept — tests:
      `no_footer_line_gets_terser_as_the_room_for_it_grows` (the property
      as the ticket words it, over every `CommitGraphLine`),
      `the_commit_graph_line_spends_the_room_it_is_given` (the widest
      wording that fits is the one drawn), and
      `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` (the same
      property over every `HintContext`, which already held). Mutations:
      picking the narrowest above 128 columns → the first fails, "129
      columns draws 50 cells, 128 draws 116"; the old width tier restored →
      only the second fails, "Offered at 116: room going spare"; a hint tier
      that drops to four hints above 200 columns → the third fails, "200
      columns draws 5 hints, 199 draws 19".

## What the class test caught, by putting each fault back one at a time

Eight mutations, eight caught. Three were **real defects in the tree**, not
mutations — the class test went red on all three before anything was fixed:

1. **The ticket's own** (§2): graph and detail pane flush at 160 columns.
2. **Not filed before**: under `Below` (100..160 columns) the file list and
   the diff body are also flush — a 30-cell file name runs straight into the
   diff text beside it, the same fault at the other breakpoint.
3. **Not filed before**: on a screen with one row to spare, the banner, the
   commit-graph offer and the hint bar all claimed it; the offer and then
   the footer drew over the banner, which is the modal one. Found because
   the sweep runs heights as well as widths.

## Fixes: cause or symptom

All three at the cause.

- **Gutter**: `split_area` (`Right`) and `detail_areas` (`Below`) each leave
  one blank column between side-by-side regions, taken out of the **left**
  region's width, never out of the right one's `x`. That keeps every
  boundary position where it was, which matters because
  `graft::update::geometry::detail_split` / `detail_hit_areas` mirror this
  arithmetic in plain `u16` for mouse hit-testing and cannot call into
  `graft-ui`. Cost to the user: one column of subject, and one of the file
  list's thirty.
- **Row carve**: `GraphScreen::screen_rows` carves header / banner / body /
  offer / footer once and everything draws from it. Cost: on a screen too
  short for both, the commit-graph offer is not shown. It is a suggestion;
  the banner is modal.
- **Footer wording**: `commit_graph_segments` draws the **widest wording
  that fits**, replacing the 120/100/under width tiers. The tiers were
  calibrated against the screen's width while the function is handed the
  graph area's, which loses 42% of the screen to a right-hand pane. Cost:
  none — every wording is one the line already had.

## NOTES

- No scout: the ticket named `layout.rs` and gave both defects with their
  exact rendered text, as its `Scout: not needed` line says.
- No compaction.
- Gates workspace-wide at `2d8ac22`: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`
  (2194 passed, 35 skipped, up from 2187 on `main`), `cargo test --doc`.
  Baseline on `main` checked first: 2187/2187, all four green.
- **Defect 3 is fixed at the cause but the user will still see a shorter
  line at 160 than at 120, and this is not the same bug.** At 160 the graph
  area is 92 columns wide because `DetailLayout::Right` gives the detail
  pane 42% of the screen; the line now spends all 92 (87 cells, up from 47),
  but 120 columns of graph area still fit 116. Closing that gap means
  either the footer spanning the whole frame under the pane, or the pane
  taking less — both are `graft`'s frame composition and a design decision,
  not this line's.
- **Same miscalibration, unfixed, in the hint bar.** `hints_for`'s tiers
  (`<100` → 3 hints, `100..160` → 6, `≥160` → all, ADR-0037 §4) are read
  against the graph area's width too, so a 160-column terminal with the pane
  open gets the three-hint tier where 120 gets six. `hints_for` is monotone
  in its own argument (pinned by the new sweep), so this is not a defect in
  it — the tier numbers are a design decision about the *screen*, and
  changing them is a spec change through the conductor, not a build
  decision. Left alone deliberately.
- Public API added: `graph_screen::ScreenRows` and
  `GraphScreen::screen_rows`. `crates/graft-ui/MAP.md` updated for those,
  for `split_area`, `detail_areas` and `CommitGraphLine`.
- One hand-keyed test updated rather than deleted:
  `detail_pane::tests::below_splits_files_and_body_horizontally_beside_a_two_line_header`
  now expects a 29-cell file list, with the reason in a comment.
- `docs/shots/` untouched, as the ticket says. The three fixes change what
  `m5-ui-banner-conflict-120x40.txt` and `-160x48.txt` show, so ticket 20's
  recapture is now required, not optional.
- **Unverified, not broken**: colour (rule 5 — nothing here touches it), and
  whether the one-column gutter reads well by eye at 160 (rule 6 — the
  user's call, on ticket 20's recapture). The cell-level check above proves
  the column is blank, not that it looks right.
