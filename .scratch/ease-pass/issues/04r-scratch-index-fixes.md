# 04r — scratch index fixes (review ease-pass-04, Major 1 + 2)

Status: done
Build with: Opus
Review: `docs/reviews/ease-pass-04.md`. Parent: `04-commit-what-goes-in.md`.

## What was wrong

One fixed scratch name, `<names dir>/commit-index`, shared by every window on a worktree. If one window removed it mid-commit, another window's `git commit` read an empty index. That commit then deleted every file he had not chosen.

## Fix (cause)

- **A name per run.** Each `Context` names its copy `commit-index-<pid>-<n>` once (`Context::scratch_index`). The Command Log and the run use the same path.
- **A missing copy is always an error.**
  - The copy is always a file: with no real index yet, `git read-tree --empty` writes an empty one (`TempIndex::copied_as`).
  - A copy that is gone before a later `Scratch` line fails the run as `RunError::NotRun` ("the commit's index copy is gone").
- **Leftovers are swept.**
  - A run holds the journal's writer's lock while its copy exists. `ScratchIndex::ready` refuses otherwise; `fold` claims the journal for a scratch plan.
  - Recovery, holding the journal alone, removes `commit-index` and `commit-index-*`. It reports any it could not remove as `Recovered::scratch_left`.
- **Why per-run names, not one lock.** A lock would make a second window wait or refuse. Unique names cost nothing at run time, and the existing journal lock already says when a sweep is safe.

## Acceptance

- [x] Red first: two concurrent runs reproduce the loss. Red quoted: `left: "" right: "a.rs\nb.rs\ngone.rs"` (HEAD tree empty, entry `Committed`). Now green. Test: `commit_chosen_scratch::another_window_cleaning_up_mid_commit_leaves_this_commit_whole`
- [x] A copy gone mid-plan is a failure in its own words, never the empty index. Test: `apply::run::tests::a_scratch_index_gone_mid_plan_is_a_failure_not_an_empty_index`
- [x] With no real index yet, a chosen first commit holds only the chosen file. Test: `commit_chosen_scratch::a_first_commit_with_no_index_yet_commits_the_chosen_file_alone`
- [x] Recovery sweeps a dead run's copies, the old fixed name too, and leaves other files alone. Test: `commit_chosen_scratch::a_dead_runs_scratch_index_is_never_read_and_recovery_sweeps_it`
- [x] Recovery sweeps nothing while a writer holds the journal. Test: `commit_chosen_scratch::recovery_sweeps_nothing_while_a_writer_holds_the_journal`
- [x] The Command Log names this run's own path. Test: `commit_chosen::the_command_log_names_the_scratch_index_it_ran_on`
- [x] Major 2: `TempIndex::remove` uses a `removed` flag instead of `mem::forget` (`snapshot/temp_index.rs`). Removal is still checked by `commit_chosen::a_failing_pre_commit_hook_leaves_head_and_the_index_and_no_scratch` and the scratch tests.
- [x] Major 2: `command_line_in` uses `graft_git::run::quote_word` and no longer strips "git ". Tests: `run::tests::quote_word_quotes_one_word_as_a_command_line_would` (graft-git) and `operation::tests::chosen::the_logged_line_names_the_real_scratch_path`.
- [x] Major 2: `run_lines` and its two out-params are gone. Replaced by `run_line` plus a `ScratchIndex` holder owned by `run_plan`. Covered by every `commit_chosen*` test.
- [x] Minor 1: a folded edit-stop step no longer drops `cleanup`. It goes into `AfterState` on success, or into the `Aborted` reason on failure. Test: `commit_chosen_scratch::a_chosen_amend_inside_graft_s_rewrite_folds_on_its_own_index`. Its journal claim is load-bearing: with the claim removed, this test failed with "needs the journal held".
- [x] Minor 3: a failed copy reads "could not copy the index for the commit: …", not "could not run git". Test: `apply::run::tests::a_failed_copy_is_worded_as_the_copy`

## NOTES

- Scout: none. The review named every file:line, and I read each one directly.
- Touches:
  - **graft-git:** new `RunError::NotRun` variant. The exhaustive matches in `version.rs` and `commit_graph.rs` got arms. The workspace build shows no other crate matches on it exhaustively.
  - **graft-git:** `quote_word` in `run/exec.rs`, re-exported from `run`. MAP.md updated.
  - **graft-ops `Context`:** gained a private field. It is built only through `Context::open`; grep found no literal construction.
  - **graft-ops `Recovered`:** gained `scratch_left`. The app does not read `plans_left` today either, so `scratch_left` is not shown to the user (gap below).
  - **graft-ops:** `TempIndex` moved to `snapshot/temp_index.rs`, which brought `plumbing.rs` back under 400 lines.
  - **Size ratchet raised:** `graft-git/src/run.rs` 684→690 (the new variant) and `graft-ops/src/journal/file.rs` new at 404 (`Journal::claimed`).
- The `read-tree --empty` step is not in the Command Log. The copy step is not logged either.
- Not verified: two real processes rather than two threads. The test uses two `Journal`s in one process; the journal docs say the locks contend the same way.
