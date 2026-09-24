# M6/32: the key hints at the bottom of the resolver, at 120 columns

- **What happens:** in a merge that lets you view the base (`B base`) and start over (`d start over`), all the hints together need 135 columns (137 in a rebase). The screen has 120. So the last two lose their words: the bar ends `… ⏎ done  j/k file  M mergetool  d  esc`. Only the key letters show, with no `start over` or `close` after them (ticket 32 NOTES; KNOWN_GAPS.md:728).
- **When it fits:** without `B`/`d`, the bar is 113 columns and every word shows. The design drew `d start over` and `esc close` in full at 120 (`docs/design-notes/m6-conflicts.md:164`, `:692`).
- **See it:** `docs/shots/m6/fix-stop-lands-in-resolver-120x40.txt`, `fix-take-all-120x40.txt`, `fix-file-list-many-120x40.txt`, `fix-file-list-rebase-step2-120x40.txt`. Compare with the wider screen, `fix-stop-lands-in-resolver-187x50.txt`.
- **Option A: drop other words first.** Keep `start over` and `close`, and cut the words from keys you already know (e.g. `j/k file`, `M mergetool`). Trade-off: the risky key and the way out get explained, and the everyday keys go bare.
- **Option B: shorter words.** E.g. `mergetool` → `tool`, `start over` → `restart`, so that everything fits at 120. Trade-off: every word shows, but some are less clear.
- **Option C: accept as is.** `d` and `esc` go bare at 120 and show in full on wider screens. Trade-off: no change, but at 120 the one key that throws away your work shows no warning word.

## Row

| M6/21 | **Question: `begun outside Graft` is not drawn in the footer at 120x40 (nor, per earlier captures, at 187).** The footer draws the fullest `stop_wordings` rung that fits (`crates/graft/src/update/banner/stop.rs:197-215`), budgeted on the graph side's width (`crates/graft-ui/src/graph_screen/footer.rs:87`, `crates/graft-ui/src/graph_screen/hints/fitting.rs:229`), which a launch splits with the changes pane (`crates/graft/src/update/relaunch.rs:32-33`). Could not confirm the pane's width at 187, or whether sizing to the graph side is intended. The walk asserts the origin on the stop's full reason (state) and `merge in progress` + `CONFLICTS` in the frame. Source: ticket 21 NOTES Findings 3 | **question** (conductor) | M7 |
