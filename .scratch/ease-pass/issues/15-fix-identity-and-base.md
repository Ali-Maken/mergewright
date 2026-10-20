# fix round: one identity per tip; merged-in branch is not a base

Status: done
Blocked by: —
Build with: Sonnet (builder-sonnet, model sonnet); fails twice → Opus

From the combined review (`docs/reviews/ease-pass-07-14.md`). Causes and fixes are named there; read those sections, not the whole file.

## Scope
1. **Major 1** (review :100-122; usability U1): `history/seeds.rs:44-51` lets a remote ref's identity win a tip a local branch holds. One identity per tip, chosen once: local branch first, remote only where no local sits. Then drop the patch at `view/entries/branch_column.rs:112-120` and anything else that only exists to undo it (grep).
2. **Major 3** (review :144-158; usability U3): `mergewright-ops/src/branch_status.rs:121` accepts a branch merged in through a second parent as a base. An ordinary base must be on the walker's first-parent line. Mind 09p's budget (`docs/perf/branch-pane.txt`): one `rev-list --first-parent` per contender top, not per candidate.

## Acceptance (tick each with its test)
- [x] In-sync `ops/ci` row on 14's mock fixture → pane heading `ops/ci`, `C` targets ops/ci. Test: `ease_walk::identity::in_sync_ops_ci_row_is_on_a_branch`
- [x] HEAD in sync with its remote → `wallet-release (you)` and `(you're here)`. Test: `ease_walk::identity::head_in_sync_is_you` (ops/ci checked out; red not observed separately, same cause as the first)
- [x] branch_column's patch removed; column still names the lane by the local branch (existing tests green). Test: existing `graph_rows_tests`, `ease_walk::graph` column tests green
- [x] sara's route on 14's fixture reads `main ─◆ wallet-release ─◆ sara/fix-toc`; no `up to date with dana/receipts`. Test: `ease_walk::identity::merged_in_branch_is_not_sara_s_base` (red, then green); oracle `equivalence::features_merged_one_after_another_answer_as_the_oracle` updated
- [x] 09p spawn count for the pane not raised (say the number before/after). Test: `scripts/branch-pane-cost.sh`: focus spawns 8 -> 8, scaling 6 -> 6 (others=10 and 500); window spawns 3337 -> 2753; budget OVER window-ms/window-spawns was already OVER before. Where a walker has ordinary contenders the step costs one extra `rev-list --first-parent` (not in the perf fixture)
- [x] Captures this changes re-shot; list them (user judges by eye). Test: 20 under docs/shots (ease-pass: graph-mode-switched, help-graph-mode, main-100/120/90, main-colours-off, main-cursor-own, main-cursor-teammate, main-folded, main-header, main-merged-shown, main-opened, main-opened-merge, main-search-folded, merge-landed, both contact sheets; m7: menu-slot-two-dim, picker-empty-query, picker-filtered, picker-remote-landed, switch-landed, take-preview, take-refused-merge); user judges by eye

Touches to check: lane identity feeds branch column, row bar, second line, pane, `C` picker, `not merged`, graph captures; KNOWN_GAPS rows ease/14-ops-ci-not-on-branch, ease/14-merged-branch-as-base.

## NOTES
- No compaction. Scout not used (files named by the ticket and review).
- New: `mergewright_git::history_reads::on_first_parent_line` (one `rev-list --first-parent --parents line ^tip...`); seeds drop a remote seed at a tip a local branch holds (`seeds_for`, `preview_seeds`).
- Not fixed (review Major 2): HEAD's wallet-release row still has a blank column on the mock (stacked lane).
- KNOWN_GAPS rows ease/14-ops-ci-not-on-branch, ease/14-merged-branch-as-base: conductor to close.
