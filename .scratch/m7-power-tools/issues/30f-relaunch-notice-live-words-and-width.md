# 30f: relaunch notice — live stash words, and fitted to the footer it is drawn in

Status: done
Build with: Sonnet. Both fixes are spelled out in the review; no new decisions.
Needs: nothing new.
Blocked by: **01b** (holds `crates/graft/src/update/undo.rs`). Wave: after 01b merges.
Scout: none. The review names every line (`docs/reviews/m7-30c-to-10c.md` Major 1, Major 2).

Files: crates/graft/src/update/undo.rs (+ its tests); crates/graft/src/view/frame.rs, view/frame/bars.rs,
view/frame/layout.rs; crates/graft-ui/src/graph_screen/* only if the notice is fitted there instead (say so in NOTES);
the K4 walk file (builder names it) for the two new steps; captures a fast-tier test re-takes (listed); MAP.md (graft);
TESTING.md.

## Problem

Review m7-30c-to-10c:
- **Major 1** (`update/undo.rs:718-727`): `interrupted_words` words the stash clause from `AppState::interrupted_edits`,
  read once at launch. After `git stash drop` + `Ctrl+R` the footer still says `a copy stays in stash@{0}`. The same
  staleness hits `Stash { n }` after `S p`.
- **Major 2** (`view/frame.rs:178`, `bars.rs:316`, `layout.rs:46-68`): the notice is fitted to the frame's width, but with
  the detail pane open at ≥160 columns the footer is drawn into the narrower `graph_area`. At 160x50, pane open,
  `stash@{0}` and `J to inspect` are cut.

## Decided (from the review's Fix lines)

1. Keep recovery's kind (`Stash` vs `Back`). Take `n` from `state.refs.stashes()` on every draw, looking up
   `entry.put_back.stash` as the fallback branch at `:732-745` does. Stash no longer listed → drop the clause.
2. Fit the notice to the width of the row it is drawn in: the graph area's width after `split_for_detail`, or let the
   graph screen fit it. Pick one and say why in NOTES.

## Acceptance

- [x] Red first: `the_notice_drops_a_stash_that_is_gone` (unit, beside `the_notice_words_where_recovery_found_the_edits`)
      — `interrupted_edits` = `Back { copy: Some(0) }`, refs list no stash → notice has no `stash@`.
      Test: `update::undo::tests::set_aside::the_notice_drops_a_stash_that_is_gone`. Red: `left: "… your edits are back
      · a copy stays in stash@{0} · J to inspect"`.
- [x] Red first: `the_stash_clause_follows_the_list` — `Stash { n }` and the stash has moved to another index → the
      notice names the live index. Test: `update::undo::tests::set_aside::the_stash_clause_follows_the_list`. Red:
      `left: "… your edits are in stash@{0} · S p …"`, right `stash@{1}`.
- [x] Red first: K4 walk step `git stash drop` + `Ctrl+R` → footer has no `stash@`. Test:
      `it::m6_stash_put_back::set_aside::the_relaunch_notice_fits_the_pane_and_follows_the_stash_list` (second half).
      Red (width fix in, words fix out): ` interrupted: checkout fix · your edits are back · a copy stays in stash@{0} · J to inspect`.
- [x] Red first: K4 step at 160x50 with the detail pane open (`⏎`) → the whole rung line is on the bottom row,
      `J to inspect` included. Same test, first half. Red: ` an operation was interrupted: checkout fix · your edits
      are back · a copy stays in stash@{…`.
- [x] Every width from 80 to 187, pane shown and hidden: the notice's never-cut clause is whole (sweep test:
      `it::m6_stash_put_back::set_aside::the_relaunch_notice_is_whole_at_every_width_pane_or_not`).
- [x] Captures re-taken by fast-tier tests are listed in NOTES; none hand-edited. ≤200 KB, colour unverified. (See NOTES.)
- [x] MAP.md (graft) for the width source (`interrupted_notice_in` row). Gates workspace-wide (see NOTES).

## Touches (check each, say how)

The notice's other words (`edits in stash@{n}`, set-aside, unreadable-journal): unchanged except the live index. The
footer's other occupants at those widths (hint bar). Every existing K4 capture. `S p` from the notice's words.
Known and out of scope: with a set-aside clause **and** the unreadable-journal warning, the never-cut clause alone
is over 80 (`undo.rs:645-652`). Stays a gap row.

## NOTES (builder, 30f)

- Scout: none — the review named every line.
- **Words (Major 1), cause:** `undo.rs` `live_edits_clause`. Recovery's kind kept; the index is read from
  `refs.stashes()` by `put_back.stash` every draw. Gone: `Back { Some }` → `your edits are back`; `Stash` → no clause;
  `Backup { Some }` → `u restores the backup`. Refs unread, or no put-back → recovery's own words (a launch-time fact,
  nothing newer to read).
- **Width (Major 2), cause — picked `view::frame`, not `GraphScreen`:** `footer_notice` gets
  `primary_pane_area(state, area)`'s width, the pure split the draw and the overlays already share, so no new
  geometry and no graft-ui change. Fitting in `GraphScreen` would move ladder knowledge (graft) into graft-ui.
- Sweep finding: with the pane **below** (Bottom layout) the footer is not the frame's last row but the graph half's;
  the sweep finds the notice row rather than assuming the bottom row. Held at every width.
- Captures: none re-taken changed (`git status docs/shots` clean after the run); no new capture — the checks are
  asserted in the walk. The 30c captures (`m7/30c-relaunch-edits-back-{120x40,80x24}.txt`) unchanged. Colour unverified.
- Not compacted.
