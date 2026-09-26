# 23b review-fixes — review 22-23's Critical and Majors

Status: done
Blocked by: 03-land-on-result (holds graft-ops)
Crates: graft-ops only (`preview/recover.rs`, `apply.rs` `guard_untracked`, `journal_list/steps.rs`, `rows.rs`,
`recover.rs`, `edits.rs`, and their tests). If a fix needs another crate, commit at the seam and report
`blocked: needs <crate>` with the signature.
Build with: Opus (unchanged from 23: git writes on the recovery path).
Scout: none. The review cites every site with file:line against `b09274ea`; re-read each site before you edit it.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-22-journal-recover.md`. No new decision.

## Scope
Each fix is the one the review states, at the cause. Read the review's section for each.
- **C1** (§Critical C1): decide "in the way" from the disk, not from status. For each path `tree_diff(expected, to)`
  adds or changes that the index does not track, a file on disk (ignored included) refuses with `UnsavedEditsTo`, in the
  preview and in `guard_untracked` alike. Precedent: `preview/stash.rs` `occupied`. The review's probe is the red-first
  test.
- **C1 open question:** one probe of whether Graft's `Checkout` overwrites an ignored file the target tracks. Report
  it in NOTES with the test or probe output. **Don't fix it here** if it is exposed; the conductor files it.
- **M1** (§Major M1): `after_head_record` takes a `to` as a branch only when the logs prove it; `record_hint` reads a
  hex `from` as detached (`Hint::Head`); `recover_for` never `Recreate`s a name that exists under `refs/remotes/` or
  `refs/tags/`, offering `restored` instead. Oracle tests: detached at `origin/main`, detached at a tag.
- **M2** (§Major M2): `an_edit_to_a_file_the_undo_never_wrote_is_not_lost`, plus a test for the one-side-unknown arm
  (`edits.rs:102`). Each seen red under the review's mutation.
- **M3** (§Major M3): the three-way match in `recover_scope`; `let left = left_at(…)` in `lost_commit`. No behaviour
  change; existing tests green.

Not in scope: M4 (a spec question, with the planner) and the Minors (they go to KNOWN_GAPS).

## Acceptance
- [x] C1: the review's probe as a test, red first then green; the preview refuses and `y` refuses. Name the test(s).
  `tests/it/recover.rs` `move_ref_checked_out_refuses_over_an_ignored_file_it_would_overwrite`. Red seen twice on the
  old code: the preview returned `Ok(… come_back: ["c.txt"] …)`; with the preview assert softened to a print, `y` ran
  and `c.txt` read `c\n` (his bytes gone). Green after. Plus `a_tracked_file_the_put_back_changes_is_not_in_the_way`
  (a tracked path in the diff is the set-aside's, not refused) and the existing untracked test, still green.
- [x] C1 open question answered in NOTES, with the evidence.
- [x] M1: both oracle tests red first, then green; the list and `recover_for` output named in the test.
  `tests/it/journal_list_oracle.rs` `a_detached_checkout_of_a_remote_branch_is_not_a_branch`,
  `a_detached_checkout_of_a_tag_is_not_a_branch` (rows `✗ Detached work · HEAD`, `committed "Detached work" on HEAD`;
  `recover_for` = `OfferBranch { name: "restored", why: "no branch held it, only HEAD" }`). Defence in depth:
  `journal_list/tests/recover.rs` `a_name_a_remote_branch_or_tag_has_is_never_made_a_branch` (`$` and `✗` rows, remote
  and tag). All three red on the original `steps.rs`/`rows.rs`/`recover.rs` (files restored from git, run, put back).
- [x] M2: both tests named, each red under the stated mutation. `tests/it/recover.rs`
  `an_edit_to_a_file_the_undo_never_wrote_is_not_lost` — the only test red under `edits.rs:103` →
  `(Some(drifted), Some(_written)) => drifted` (36 run, 1 failed).
  `a_reversed_entry_with_no_recorded_worktree_claims_every_path_the_undo_wrote` — the only test red under `:102` →
  `(Some(_), None) | (None, Some(_)) => return Ok(None)`. `edits.rs` unchanged after (git diff empty).
- [x] M3: done; name the tests that cover the rewritten code. `recover_scope`: `checked_out_main_is_moved_back` (scope
  order, updated for M1's two new refs), `a_name_a_remote_branch_or_tag_has_is_never_made_a_branch`; `lost_commit`'s
  `left`: `checked_out_main_is_moved_back`, `under_a_graft_row_main_is_moved_back_from_its_after_value`,
  `moved_on_offers_main_restored`, `a_gone_branch_is_recreated` (all `journal_list/tests/recover.rs`).
- [x] Gates workspace-wide: fmt ok, clippy `--all-targets -D warnings` ok, nextest 3799/3799 (1 skipped), `test --doc` ok.

## Touches
Every caller of `untracked_in_the_way` and `guard_untracked` (grep), the journal-list oracle tests, 22b's lost_work
fixture tests, the recover tests. 24 draws these rows: the row words change for the M1 case only. Name how each
was checked.

## NOTES (builder, 2026-09-26)
No scout: the ticket says none; the review cites every site, each re-read before editing. No compaction.

**C1 open question — Graft's `Checkout` IS exposed.** Throwaway probe (not committed) in a copy of
`tests/it/recover.rs`: `main` at two (no `c.txt`), `other` at three (tracks `c.txt`), `c.txt` in `.git/info/exclude`
holding `HIS IGNORED FILE`, then `Operation::Checkout` to `other`. Output:
`preview: Ok((["git switch other"], Updated { paths: ["c.txt"] }))` — no refusal; `run ok: true`;
`c.txt after y: "c\n"`; `undo ok: true`; `c.txt after u: "c\n"`. His bytes are gone and `u` does not bring them
back. Not fixed here, per the ticket; for the conductor to file.

**Cause fixes:**
- C1: `preview::in_the_way` (was `untracked_in_the_way`) decides from the disk: paths `tree_diff(expected, to)` adds or
  changes, plus untracked paths `to` holds, minus `ls-files`, then `symlink_metadata` (the stash `occupied` rule). One
  read for preview and `guard_untracked`. New `FactsError::OnDisk { path, source }` for a disk read that fails other than
  not-found. Cost: one `diff-tree` + one `ls-files` per checked-out `MoveRef` preview and `y`.
- M1: `after_head_record` takes a `checkout:` `to` as a branch only when the next newer switch leaves `from` that name
  (`next_switch_from`); a twin branch record still sets it. `record_hint` returns `Hint::Head` for a hex `from`.
  `recover_scope` adds `refs/remotes/<b>` and `refs/tags/<b>`; `recover_for` offers `restored` instead of `Recreate`
  when either exists (`"<b> is a remote branch, not one of yours"` / `"<b> is a tag, not a branch"`).

**Touches:** `untracked_in_the_way`/`guard_untracked` callers: grep — only `preview/recover.rs` and `apply.rs`; renamed
in `preview.rs`. `recover_scope`/`recover_for`/`journal_list`/`FactsError::` outside graft-ops: grep over `crates/`,
none (24 unbuilt). Oracle, lost_work fixture, recover and journal_list unit tests: all green in the workspace nextest.
MAP.md: Recover op line, `recover_scope` line, new `FactsError::OnDisk` line.
