# 30f: relaunch notice — live stash words, and fitted to the footer it is drawn in

Status: ready
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

- [ ] Red first: `the_notice_drops_a_stash_that_is_gone` (unit, beside `the_notice_words_where_recovery_found_the_edits`)
      — `interrupted_edits` = `Back { copy: Some(0) }`, refs list no stash → notice has no `stash@`.
- [ ] Red first: `the_stash_clause_follows_the_list` — `Stash { n }` and the stash has moved to another index → the
      notice names the live index.
- [ ] Red first: K4 walk step `git stash drop` + `Ctrl+R` → footer has no `stash@`.
- [ ] Red first: K4 step at 160x50 with the detail pane open (`⏎`) → the whole rung line is on the bottom row,
      `J to inspect` included.
- [ ] Every width from 80 to 187, pane shown and hidden: the notice's never-cut clause is whole (sweep test; name it).
- [ ] Captures re-taken by fast-tier tests are listed in NOTES; none hand-edited. ≤200 KB, colour unverified.
- [ ] MAP.md (graft) for the width source. Gates workspace-wide.

## Touches (check each, say how)

The notice's other words (`edits in stash@{n}`, set-aside, unreadable-journal): unchanged except the live index. The
footer's other occupants at those widths (hint bar). Every existing K4 capture. `S p` from the notice's words.
Known and out of scope: with a set-aside clause **and** the unreadable-journal warning, the never-cut clause alone
is over 80 (`undo.rs:645-652`). Stays a gap row.
