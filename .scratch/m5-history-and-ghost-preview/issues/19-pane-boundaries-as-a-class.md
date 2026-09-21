# pane-boundaries-as-a-class: assert on the rects, not the cells

Status: ready
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

- [ ] Panes never overlap, swept across a width range — test:
- [ ] Adjacent panes leave ≥ 1 column, swept across a width range — test:
- [ ] The sweep covers the banner's rows and the preview box's dock — test:
- [ ] The 160-column gutter defect is fixed, and the class test is what
      catches it — test:
- [ ] Footer wording gets no terser as width grows, swept — test:

**Mutation-check each**, and for the class test say **which real defects it
catches** by reintroducing them one at a time. A class test that catches
nothing it was not written against is a class test in name only.

## Captures

None from you — the recaptures belong to ticket 20, which holds the banner
wording. Do not touch `docs/shots/`.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g19`, deleted
before hand-back. Check the baseline on `main` yourself. Commit, never push.

## Report

Cause or symptom per fix. Confirm you watched the class test fail on defect 2
before fixing it. Which real defects the class test catches. Anything
unverified — which is not broken.
