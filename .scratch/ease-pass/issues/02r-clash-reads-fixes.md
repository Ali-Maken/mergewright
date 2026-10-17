# 02r — clash-reads review fixes

Status: done
Crate: graft-git. Review: `docs/reviews/ease-pass-02.md` (Major 1, Minor 1, Minor 2).

## Acceptance

- [x] `merge_forecast` under ~60 lines (now 52, `history_reads/merge_check.rs`); record parse pulled to `read_records`. Verified by the existing fixtures staying green: `tests/it/merge_forecast.rs` all 7 (`a_clean_merge_lists_what_theirs_changed_and_no_clashes` … `a_raised_stop_is_stopped`).
- [x] Files under 400: `forecast.rs` 220, `merge_check.rs` 241, `history_reads.rs` 359 (`wc -l`).
- [x] Tree-id parse once (`forecast::tree_id`, used by `merge_tree_step` and `read_records`); old-git reason once (`forecast::OLD_GIT_MERGE`, used by `forecast_merge` and `merge_forecast`). Pinned by `merge_forecast::an_old_git_is_not_checkable_with_forecast_merges_reason` (old-git fixture test comparing both readers) and the `forecast_merge`/`merge_tree_step` tests in `tests/it/history_reads.rs`.
- [x] `line_origins` aligns each side's added lines in order (LCS) instead of text-set membership — red first: `diff::origins::tests::a_common_text_is_claimed_where_its_side_put_it_not_wherever_it_appears` (blank line, lone `}`; failed with `Both`/`Yours` misattributions before the fix).
- [x] `changed_paths` stop: new `changed_paths_until(…, stop)`; `merge_forecast` uses it. `changed_paths` keeps its signature (graft-ops callers untouched) and delegates with `&()`. Red first: `merge_forecast::changed_paths_with_a_raised_stop_is_stopped` (did not compile before).

## NOTES

- Cause fixes, all three. Minor 1 cost: a side over 4 M result×side line pairs claims nothing (no attribution rather than a wrong one); 16 MiB DP table at the cap.
- Not verified: a stop raised *between* merge-tree and diff-tree inside one `merge_forecast` call — no deterministic hook for mid-call timing; covered by the direct `changed_paths_until` test plus the call site reading `stop`.
- Minor 3, 4, 5 of the review not in scope.
- No scout: three named functions, read directly.
