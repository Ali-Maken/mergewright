# 30k: code fixes from review 09..21c (Major 2, Minors 3-8, 12, 13)

Status: ready
Build with: Opus. Minor 13 journals a config removal via 30j's `SetBranchConfig` (undo path); the rest are small.
Needs: nothing new.
Blocked by: nothing (all merged). Concurrent: 07b (holds graft `update/operate.rs`, `jobs/runner.rs`, `update/events.rs`,
`view/rows.rs`, graft-ui `graph_screen/mod.rs`, `graph_screen/banner.rs`, graft-ops `operation.rs`, `apply/fold.rs`),
30e (graft-ops `apply.rs`, `apply/error.rs`, `rewrite/dependents.rs` doc comments). Don't touch those. Minors 1 and 2
(in `operate.rs`) went to 07b.
Scout: none. The review names every line (`docs/reviews/m7-09-to-21c.md`, Major 2, Minors 3-8, 12, 13). 30h split
files after the review: re-find each line on main first (`graft-ui/src/history.rs` is now `history/{list,file_pane}.rs`).

Files: graft-ui `src/history/file_pane.rs` (or wherever `binary_words` now lives), `src/graph_screen/render.rs`;
graft `src/update/amend_into.rs`, `tests/it/m7_walk/amend_into.rs`, the preview file drawing "files set aside" (name it);
graft-ops `src/operation/history.rs` (branch delete), tests beside each; the fixture/corpus clock for Minor 7 (name it);
captures their tests re-take; MAP.md per crate. Anything else → NOTES, with the line.

## Fixes (each decided)

1. **Major 2** — `binary_words` re-matches the pane its caller matched. Bind the fields in the outer arm (or pass the
   `Binary` payload) to a function that takes only that; no unreachable arm.
2. **Minor 3** — refs unread: `a` on HEAD's row must take today's amend, not the older-commit path (`amend_into.rs:108`
   vs comment :149). Return `None` when refs are unread.
3. **Minor 4** — `absorb_chosen` gets the same early `live_operation` (P14) check `a_key` has, with `a`'s words.
4. **Minor 5** — footer reveal subtracts the row tag's width (`render.rs:464-468`) so a subject cut only by the tag is
   revealed.
5. **Minor 6** — preview's set-aside count: count what is set aside and put back (the unstaged part); agree with the
   landing's "your unstaged edits". Re-take `amend-into-preview-187x50`, `absorb-preview-120x40`.
6. **Minor 7** — landed capture shows "futu…": pin the rewritten commits' dates to the corpus clock. Re-take
   `amend-into-landed-120x40`.
7. **Minor 8** — binary header: drop `+0 −0` for binary files. Re-take `history-binary-140x40`.
8. **Minor 12** — move the two `#[ignore]` put-in perf tests (`m7_walk/amend_into.rs:567`, `:741`) into the heavy tier
   (`heavy-tests` feature), so every review runs them. Don't run them yourself (measurement never runs in a helper).
9. **Minor 13** — deleting a branch leaves `branch.<b>.*` config: remove the section on delete and journal it through
   30j's `SetBranchConfig`, so undo restores it and a later branch of the same name inherits nothing.

## Acceptance

- [ ] Major 2: no fallback arm; `history_binary_*` snapshots unchanged (name them).
- [ ] Red first: Minor 3 test (refs unread, `a` on HEAD → amend path).
- [ ] Red first: Minor 4 test (absorb during a live op refused with `a`'s words).
- [ ] Red first: Minor 5 test (subject cut only by the tag is revealed).
- [ ] Minor 6: red first on the count; captures re-taken, lines named.
- [ ] Minor 7: capture shows a date, not `futu…`; test named.
- [ ] Minor 8: capture line 2 without `+0 −0`; snapshot named.
- [ ] Minor 12: both tests under `heavy-tests`, not `#[ignore]`; compile under `--features heavy-tests` shown.
- [ ] Red first: Minor 13 (real git): delete branch → config gone; undo → config back; new same-name branch has no
      upstream.
- [ ] MAP.md each crate touched. Gates workspace-wide.

## Touches (check each, say how)

Other callers of `binary_words`; every capture with a binary pane, a row tag in the footer, or the set-aside count;
every undo of a branch delete; journal read of old entries. KNOWN_GAPS rows this closes (list; don't edit).

## NOTES (builder)
