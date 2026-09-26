# 31 patch-extended-headers — a deleted file's patch writes `dev/null` into history

Status: done
Blocked by: none (18d merged).
Crates: graft-git only (`diff/patch.rs`, its tests, MAP.md).
Build with: Opus (a git write path; absorb, and the app's hunk stage and discard use it).
Scout: none. Source: `docs/reviews/m7-06b-absorb-staged.md` §C1 (Critical) and Question 1.
Split by: conductor, 2026-09-26. No new decision: the fix is the review's.

## Why
`crates/graft-git/src/diff/patch.rs:282-318` writes `diff --git a/b.txt b/b.txt`, `--- a/b.txt`, `+++ /dev/null` with no `deleted file mode` line. `git apply` then reads `/dev/null` as a path: `b.txt` goes, and an empty `dev/null` is added. Through absorb, history gains a `dev/null` file and the M3 drop never fires (probe q8). `u` restores it.

## Scope
- `hunk_patch` writes git's extended header lines: `deleted file mode <old>` for `Deleted`, `new file mode <new>` for `Added`.
- Test by **applying** the patch to a temp index (`git apply --cached`), not only by asserting the text (`patch/tests.rs:201-202`): deleted, added, and modified.

## Acceptance
- [x] Deleted-file patch applied: red first (`dev/null` in the index), then green, with the path gone and no `dev/null`. — `a_deleted_files_patch_applied_to_the_index_removes_it_and_adds_no_dev_null`; red seen: `left: ["a.txt", "dev/null"]`.
- [x] Added-file patch applied: green (today git rejects it with `dev/null: does not exist in index`; say what you saw). — `an_added_files_patch_applied_to_the_index_creates_it`; red seen: `error: dev/null: does not exist in index`, as the review said.
- [x] The existing text tests updated to the new header, each named. — `added_and_deleted_files_use_dev_null` renamed `added_and_deleted_files_use_dev_null_under_their_extended_header`, now asserts the full header prefix. `tests.rs:201-202` assert git's own `diff-tree` output (the oracle), not ours; unchanged and still true. Modified: `a_modified_files_patch_applied_to_the_index_changes_only_it` (applied, green before and after).
- [x] Gates workspace-wide. — fmt, clippy `-D warnings`, nextest 3986/3986, doctests: pass.

## NOTES (builder, 2026-09-26)
- Fix (cause): `patch.rs` `write_git_headers` writes `new file mode`/`deleted file mode` for `Added`/`Deleted`. Mode unknown (only hand-built `FileDiff`s; git's diff always prints it): the `diff --git` line is left out, so git reads a traditional patch where `/dev/null` means no file — no guessed mode. Tests: `a_creation_or_deletion_of_unknown_mode_is_written_without_the_diff_git_line`, `a_deletion_of_unknown_mode_still_applies_as_a_deletion`, `a_creation_of_unknown_mode_still_applies_as_a_creation`.
- **Question 1 answered, worse than asked:** unstage was hit too. Red before the fix: `unstaging_a_staged_addition_takes_the_file_out_of_the_index` left `dev/null` in the index; `unstaging_a_staged_deletion_puts_the_file_back_in_the_index` failed with `dev/null: does not exist in index`. Both green now. `discarding_a_worktree_deletion_brings_the_file_back` green (not run red).
- **Gap (new behaviour, not corruption):** a *line* selection on a wholly deleted file (Forward) or wholly added file (Reverse) now carries the creation/deletion header, and git refuses it (exit non-zero; message not captured) — the index is unchanged (`staging_part_of_a_deletion_is_refused_by_git_and_leaves_the_index_alone`; before the fix git accepted it, meaning a `dev/null` write). Doing it properly means writing such a selection as a modification (`+++ b/path`, fixed hunk starts). Not built; for the conductor to file.
- Touches: every `hunk_patch` caller — `crates/graft/src/update/operate.rs:466` (`patch_for`: stage, unstage, discard) and its tests at 1549-1606; `crates/graft-ops/src/absorb/build.rs:201` (`with_context`). Checked by grep and the workspace nextest run (all pass); none of their tests exercises a deleted/added file end to end — follow-ups `M7/31-app` and 30's absorb case. `hunk_portion` in the oracle tests starts at `@@`, so the new header lines do not touch the byte-for-byte corpus comparisons (both still pass).
- Scout: none — one file named by the ticket, ~40 lines changed.

## Touches
Every `hunk_patch` caller, including absorb (`graft-ops` `absorb` `with_context`) and the app's hunk stage, unstage and discard (`crates/graft/src/update/operate.rs:466`). Test through graft-git only. Name the callers and say that their end-to-end tests are the follow-ups `M7/31-app` and 30's absorb case.
