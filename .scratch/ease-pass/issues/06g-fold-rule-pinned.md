# fold-rule-pinned — tests that pin "merged = merged into any shown line" on teammates' dependent branches

Status: ready; was draft
Build with: Sonnet — tests only, in modules 06 already built (`inclusion::tests::fold_by_default`, app `fold_by_default`); no production change, no design call. A failing test is a stop, not a fix.
Needs: none (06, 06d, 06e merged). Calls only existing: `Inclusion::folded_by_default`, `OpenedMerges`, `FoldWalk::{folds, behind}`, `Included::fold`, the 06e repo-fixture helpers.
Scout: not needed — test-only; 06's NOTES name the modules and helpers.

## Read first
- `docs/design-notes/ease-pass.md` Hard-to-reverse choice 2, the "Settled 2026-10-18" paragraph.
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md` Scope, first bullet; NOTES "Deviations" (no `ref_tips`: why a tip no merge reaches shows).
- `crates/graft-graph/MAP.md` `inclusion`. `crates/graft/MAP.md` `history::fold`.

## Why
The note said refs "not reachable from trunk" stay shown; 06 and the code fold any ref tip a shown merge reaches through a second parent. Settled for the code's rule (the main mock folds branches merged into the unmerged `integration`). Nothing names a test for the cases that differ — the ones his team hits daily. Pin them.

## Scope
In: graft-graph `inclusion` tests (fixture histories, topo order and date-skewed); one graft integration test through the worker in 06e's `fold_by_default` module.
Out: any change to `FoldWalk`, `Included`, the app. If a case fails, stop: report the failing assertion and the line of the rule that produced it (rule 7), `blocked: fold rule disagrees with the note`.

## Signatures
None new. Test names below are the spec.

## Acceptance
- [ ] `teammate_branch_merged_into_an_unfinished_branch_folds_under_its_merge`: refs `development` (trunk), `sara/report` (unmerged), `omar/fix` merged into `sara/report` only (ref kept). `omar/fix`'s commits hidden, `behind` = sara's merge; sara's merge `folds == Some(true)`; `sara/report`'s tip shown.
- [ ] `a_merged_branch_with_a_new_commit_shows_again_with_its_lane`: same, then one commit on `omar/fix`. Its tip and every commit down its first-parent chain shown; sara's merge `folds == Some(false)` (nothing hidden behind it, so no tail).
- [ ] `a_base_merged_into_its_dependent_folds_until_it_moves`: `wallet-release` (not HEAD), `sara/fix-toc` cut from it, then `wallet-release` gains 2 commits and is merged into `sara/fix-toc`. Those 2 hidden behind the merge; with `wallet-release` as HEAD (keep) they are shown and the merge folds nothing.
- [ ] `keep_tips_never_fold`: HEAD's branch and an integration branch each merged into an unmerged teammate branch stay shown, every commit on their first-parent chains shown.
- [ ] `a_branch_merged_into_heads_unmerged_branch_folds`: `pay-571` merged into HEAD's `wallet-release`, which is not in trunk → `pay-571` hidden behind that merge (the mock's rows 115–118 shape).
- [ ] Each of the five also passes emitted date-skewed, then promoted: after promotion, shown equals the topo answer (reuse the skew helper the property tests use).
- [ ] App, through the worker: `fold_by_default::a_teammates_merged_branch_folds_and_its_tail_names_it` — the first case as a real repo; first window has no `omar/fix` commit; sara's merge row tail `▸ N commits from omar/fix`.
- [ ] NOTES says whether `in_topological_order_the_fold_is_exact`'s oracle already uses "reached by a shown merge's second parent" (file:line), or "not reachable from trunk".
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~25k.
