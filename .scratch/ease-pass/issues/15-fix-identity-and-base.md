# fix round: one identity per tip; merged-in branch is not a base

Status: ready
Blocked by: —
Build with: Sonnet (builder-sonnet, model sonnet); fails twice → Opus

From the combined review (`docs/reviews/ease-pass-07-14.md`). Causes and fixes are named there; read those sections, not the whole file.

## Scope
1. **Major 1** (review :100-122; usability U1): `history/seeds.rs:44-51` lets a remote ref's identity win a tip a local branch holds. One identity per tip, chosen once: local branch first, remote only where no local sits. Then drop the patch at `view/entries/branch_column.rs:112-120` and anything else that only exists to undo it (grep).
2. **Major 3** (review :144-158; usability U3): `mergewright-ops/src/branch_status.rs:121` accepts a branch merged in through a second parent as a base. An ordinary base must be on the walker's first-parent line. Mind 09p's budget (`docs/perf/branch-pane.txt`): one `rev-list --first-parent` per contender top, not per candidate.

## Acceptance (tick each with its test)
- [ ] In-sync `ops/ci` row on 14's mock fixture → pane heading `ops/ci`, `C` targets ops/ci.
- [ ] HEAD in sync with its remote → `wallet-release (you)` and `(you're here)`.
- [ ] branch_column's patch removed; column still names the lane by the local branch (existing tests green).
- [ ] sara's route on 14's fixture reads `main ─◆ wallet-release ─◆ sara/fix-toc`; no `up to date with dana/receipts`.
- [ ] 09p spawn count for the pane not raised (say the number before/after).
- [ ] Captures this changes re-shot; list them (user judges by eye).

Touches to check: lane identity feeds branch column, row bar, second line, pane, `C` picker, `not merged`, graph captures; KNOWN_GAPS rows ease/14-ops-ci-not-on-branch, ease/14-merged-branch-as-base.
