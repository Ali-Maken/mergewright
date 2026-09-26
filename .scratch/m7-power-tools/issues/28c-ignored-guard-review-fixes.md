# 28c ignored-guard-review-fixes — review 28's Majors and fixable Minors

Status: done
Blocked by: none (06c merged).
Crates: graft-ops (`in_the_way.rs` and its split, `operation.rs` StashInTheWay words, tests), plus docs: ADR-0049's table and `docs/design/02-keymap.md:425`.
Build with: Opus (unchanged from 28; git writes).
Scout: none. The review cites every site with file:line. Re-read each one before editing, because 06b and 06c touched graft-ops.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-28-ignored-guard.md`. No new decision: each fix is the review's.

## Scope (each at the cause, red first)
- **M1** (§M1): pull diffs `HEAD` against the tracking tip (`in_the_way.rs:210`). Diff the incoming side instead, from the merge base to the tip. Amend ADR-0049's table to say so, in an amendment line dated today.
- **M2** (§M2): a case-only rename on a case-insensitive disk is refused and called "(ignored)" (`:87-91`, `:607`).
- **M3** (§M3): `in_the_way.rs` is 718 lines; bring it under 400. Split the two functions over ~60 lines. Replace the runtime word template with plain formatting. No behaviour change.
- **m1 / M7/28skip** (§m1): model git's skip of unchanged picks, so that `R` and `a` which leave the untracking commit unchanged are not refused.
- **m4** (§m4): when any of `StashInTheWay`'s paths are ignored, the refusal says `(ignored)` and gives the next step, in the same words as every other kind. Edit the keymap line at `02-keymap.md:425`.
- **m5** (§m5): time the read for a switch on the 100k synthetic fixture, with a script writing `docs/perf/m7-28.txt`. Read only its summary line. If it is over budget, list the index only when `on_disk` found candidates.
- **m6** (§m6): split the test file into `in_the_way/{one_shot,stash,pull,replay}.rs`, each ≤ 400 lines. Add one absorb case.

## Acceptance
- [x] M1: `in_the_way::pull::pull_while_ahead_by_an_untracking_commit_is_not_refused` — red first (`remote-0.txt (ignored) is in the way of pulling origin/main`); now preview not refused, apply runs, bytes `MINE`, HEAD unmoved, and the same test runs plain `git merge --ff-only --no-overwrite-ignore origin/main` and asserts `Already up to date`. `pull_over_an_ignored_file_is_refused` still refuses.
- [x] M2: `in_the_way::one_shot::a_case_only_rename_is_not_in_the_way` — red first here (APFS, case-insensitive: `README.md (ignored) is in the way of switching to other`); now switches, bytes kept, index `README.md`. On a case-sensitive disk it returns early and prints `skipped: this disk is case-sensitive` (doc comment says so).
- [x] M3: src `in_the_way.rs` 176, `in_the_way/{disk 178, reads 129, replay 244, written 194}`; `written_paths` now ~35 lines, the rewrite and revert arms are `replay::{rewritten, reverted}`; the runtime template replaced by a four-arm `match` (`operation.rs` `in_the_way_of`). Words unchanged: `operation::tests::in_the_way_words` and all 18 refusal tests passed on the split commit `323c9709` before any fix.
- [x] m1: `in_the_way::replay::rewrite_that_leaves_the_untracking_commit_unchanged_runs` (R, Y reworded) and `amend_into_the_untracking_commit_runs` (`a` into X) — both red first (`.env (ignored) is in the way of rewriting feature`); each first runs plain `git rebase -i` (the second with `--fixup` + `--autosquash`) and asserts `.env` is `MINE`, then Graft. Refusing case still refuses: `rewrite_over_an_ignored_file_is_refused` (X reworded), `amend_into_over_an_ignored_file_is_refused`.
- [x] m4: `in_the_way::stash::{stash_apply_over_an_ignored_file_is_refused_at_y, stash_whose_staged_new_file_meets_an_ignored_file}` name `c.txt (ignored) is in the way · move it aside first` (red first: `c.txt is in the way`); shapes in `operation::tests::stash_in_the_way_words_name_an_ignored_file`; `stash_preview::an_untracked_file_in_the_way_refuses_the_preview` updated for the ignored half. Keymap `02-keymap.md:425` edited.
- [x] m5: summary line quoted in NOTES; under the assumed budget, so the index listing stays eager (no change).
- [x] m6: tests `in_the_way.rs` 182 (shared helpers), `in_the_way/{one_shot 323, stash 43, pull 178, replay 369, perf 103}`; absorb case `in_the_way::replay::absorb_over_an_ignored_file_is_refused` (passed on first run — it pins a covered path, not a fix).
- [x] Gates workspace-wide: fmt ok; clippy `--all-targets -D warnings` ok (also with `graft-ops/heavy-tests`); nextest 3939/3939 passed, 1 skipped; `cargo test --doc` ok.

## NOTES (28c, builder)
- No compaction. No scout (ticket: none; the review cites every site, each re-read before editing).
- m5: `m7-28 summary: blocking for a switch, median of 7: 29.1 ms (100k commits), 55.0 ms (20000 files)` (release, `scripts/m7-28-perf.sh`). No budget is written for this read; I assumed 100 ms (the file header says so). The 100k fixture's tree is 3 files, so a 20 000-file tree was added to measure `ls-files`. Not measured: 100k files.
- M2 fix: the on-disk spelling is found per component by inode (`read_dir`, `DirEntry::ino`), not by case folding, so Unicode normalisation is covered by the same code. Unverified: I tested case only, not a decomposed/precomposed name.
- m1 fix needs `rewrite::git_words` (the word git is handed per row); `git_todo` now uses it, so words and todo text have one source. Rebase (`r`) not modelled (ADR amendment says why).
- ADR-0049 has no table of its own; the amendment (dated 2026-09-26) states the changed rows of ticket 28's §4 table.
- Removed the rewrite arm's `made` extend: `made` commits are already todo rows (fixups), so it double-counted.
- Not done (out of scope): review m2, m3 (conductor's gap rows), Q1, Q2.

## Touches
Every caller of the guard (apply, preview, the engine), the pull path used by 03b, and ADR-0049. Name how each was checked.
