# 25 — Failed setting bar has no ✗; help ranks HERE rows above a setting named by the query

Status: done · Build with: Sonnet · Blocked by: none
Source: KNOWN_GAPS ease/06i-failed-no-cross, ease/06i-graph-query-steps (both broken, rulings in the rows).

## Need / scope
1. A failed setting write draws its bar without `✗` (`view/status.rs:108` builds the line from the message alone).
   Ruling: a failure shows `✗`, as every other failure bar. Red-first test on the drawn bar.
2. Typing `graph` in help: three HERE rows match first; `? graph ↓ ⏎` lands on "Write the commit-graph file"; switching the
   graph setting takes 7 steps vs ≤4 (`? every ↓ ⏎` = 4). Ruling: a setting whose name starts with the query lists before
   HERE rows. Red-first test on the ranking; the journey counted in a test (≤4 steps).
3. Captures that show the help list or a setting failure: re-shoot and list; user judges by eye.

## Acceptance
- [x] ✗ on a failed setting write; test named: `the_bar_says_kept_or_why_not_with_its_mark_drawn` (update/tests/graph_mode_help.rs).
- [x] Ranking: setting prefix-match before HERE rows; tests `a_setting_named_by_the_query_lists_before_here`, `the_graph_switch_journey_is_four_steps`; walk `typed_graph_lists_the_group_after_here_at_120_and_90`.
- [x] Captures re-shot and listed (≤200 KB), or re-shot: help-graph-mode-120/90.txt, ease-pass-contact-sheet-1.txt (177 KB). No capture shows the setting failure bar (grep 'not kept' docs/shots: only m7 staging ones).
- [x] Five gates + `scripts/walks.sh`; Touches: (keymap doc, help catalogue, other screens drawing status bars).

## NOTES
- No scout (two small known spots). Not compacted.
- Failure `✗` is put in the setting's message (`graph_mode_kept`), not in `Status::Failed` globally: no other failure bar draws `✗` today (the ruling's "as every other" does not hold), a global change would re-shoot many captures. Conductor: decide if wanted globally.
- Rank: a Setting group whose header or choice title starts with the query sorts after the chord group, before others (`listing.rs filter`).
- walks: `m7_walk::fork_rows::a_stop_inside_a_fork_names_it_on_the_banner` failed once in a full run, passed alone and on rerun: flaky, not investigated.
- Touches: keymap doc (no key changes), help catalogue (ordering only, ⌃P same filter; test `typed_graph_fold_and_branch...` holds), status bars (only the setting message; other Failed bars unchanged), captures (grep above).
