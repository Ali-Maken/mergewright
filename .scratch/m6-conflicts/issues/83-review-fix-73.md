# 83 — review fix for 73 (Critical C1; Major M1, M2; Minor m1, m2, m3, m8, m9)

Status: done
Blocked by: 73 (merged)
Crates: graft-ops only (src and tests). Runs beside 74 (graft-ui, graft): disjoint.
Source: `docs/reviews/m6-final-fixes-73.md`. Every finding names its lines; re-check each against main.
Build with: builder-opus (git-write tests and one pure count; no new design)
Scout: none. The review names every line, and the 73 scout covers the module.

## Decided (conductor, 2026-09-26)
- m4 (early `?` exits after step 4 don't name `S`), m5 (put back after a step-5 stop?), m6 (`Aborted` for an op that
  took effect): gap rows, question/unverified, M7. Not here.
- m7 (README says a dirty rebase is refused): 74, which owns the words.
- **M1 changes what `SetAside::staged` means.** 74 words the preview row from it. 74 is told; if 74 merged first
  with a test that counts a staged new file as "comes back unstaged", fix that test here only if it is in graft-ops,
  otherwise report `blocked: needs graft` naming it.

## Acceptance
- [x] **C1:** `set_aside_round_trips` asserts, after the put-back (before any undo), per tree: every edited file's
      bytes are back, and the index shape git actually gives (`git status --porcelain` per row: `MM` → ` M`, staged
      deletion → unstaged, staged rename → `A b` + ` D a`, staged new file stays `A`). 73's tick 3 and NOTES
      (`73-…md:144-145`, `:227-228`) corrected to say what is proven: the edits come back, and exactly this much
      staging is lost. Name the test.
      **Test:** `round_trip` (shared check) behind the eight `set_aside::{rebase,checkout}_round_trips_*` tests
      (`tests/it/set_aside.rs`): after apply and before undo it asserts each file in `Tree::edits` (bytes, or gone)
      and `porcelain(repo) == Tree::after_put_back()` (the four rows above, exact lines). 73's tick and NOTES
      "Round trip" rewritten to that.
- [x] **M1:** `SetAside::staged` = staged entries whose staging the put-back loses (modifications, deletions; a
      rename counts its old-path deletion; `Added` does not). Doc at `operation.rs:274-276` says so.
      `sets_aside_table` moves `b.rs` to "stays staged". A real-repo test compares `sets_aside(facts).staged` with
      what `git status` shows after the put-back, per C1's trees. Red first.
      **Tests:** `operation::history::tests::sets_aside_table` (b.rs `A` not counted; d.rs `D` and e.rs `R` added;
      asserts `files: 5, staged: 3`) — red first (`staged: 4`), then green. Real repo: the same `round_trip` asserts
      `staging_lost(before, after) == op.sets_aside(&read_facts(..)).staged`, `staging_lost` counting from git's
      porcelain alone. Red shown by reverting the count to `is_staged()`: both `*_round_trips_a_staged_new_file`
      failed (promised 1, git lost 0). Type changes also count (not exercised on a real repo — see NOTES).
- [x] **M2:** `a_stop_before_each_step_loses_nothing` asserts per step what that step leaves: entries written and
      status, stash list length, the entry's last command, tree dirty or clean. `|| step == 6` removed. 73's NOTES
      row 5 corrected (the op never ran at 5; tree clean, edits in `S`). Red by moving a stop one step (say how).
      **Test:** `set_aside::a_stop_before_each_step_loses_nothing` asserts `Left::read == Left::expected(step)`
      (entry aborted-or-none, last command's step name, stash count, `base.txt` dirty) plus the old "edit somewhere"
      checks; `latched` asserted for every step. Red: step 4's stop re-keyed to step 5's trigger (`base.txt` back to
      `base`) → fails at "Rebase, stop before step 4", left `{last: reset --hard, stashes 1, dirty: false}` vs
      right `{last: stash store, stashes 1, dirty: true}`; reverted.
- [x] **m1:** a journal failure after step 6 carries the put-back's outcome (kept `S` / clash) in its words. Test.
      **Test:** `set_aside::a_journal_failure_after_a_clash_names_the_stash` — a one-shot post-checkout hook
      replaces the journal file with a directory; asserts `ApplyError::Journal` whose `explain()` contains
      `stash@{0}` and `clash`, S still listed. Red first (words were `journal file: Is a directory`).
- [x] **m2:** an unreadable status after the put-back names `stash@{n}` in the error. Test.
      **Test:** `set_aside::an_unreadable_status_after_the_put_back_names_the_stash` — hook sets
      `status.showUntrackedFiles bogus` (git status exits 128); asserts `explain()` contains `stash@{0}` and
      `could not be read`, edit in tree or list. Red first (was `ApplyError::Facts`, no stash words).
- [x] **m3:** `Dropped::Unlisted` gets its own `Kept` arm: "your edits are back; the set-aside copy could not be
      found to drop". Test.
      **Test:** `set_aside::an_unlisted_set_aside_after_a_clean_put_back_says_the_edits_are_back` — hook
      `chmod 000` on the stash reflog; asserts `explain()` contains that phrase, not "status", and `base.txt`'s edit
      is back. Red first (was `Facts(Stash(Reflog("Permission denied")))`).
- [x] **m8:** `u` after a kept put-back (the hook test, `set_aside.rs:376`) and after a stop at 4 and at 5: tree,
      index and stash list back to before. Tests.
      **Tests:** `set_aside::undo_after_a_refused_put_back_restores_everything` (`whole()` = refs, HEAD, index,
      files, stash list, equal to before) and `set_aside::undo_after_a_stop_at_4_or_5_restores_everything` (both
      kinds, checks the stop landed via `Left::expected`, then `whole()` equal to before).
- [x] **m9:** `set_aside_round_trips` split per kind (one `#[test]` each over a shared check); none over 5 s alone.
      Report before/after.
      **Measured (nextest, this worktree):** before 10.6 s (review's number, one test); after eight tests,
      0.80–0.82 s each. Slowest set-aside test now `a_stop_before_each_step_loses_nothing` 3.9 s.
- [x] graft-ops MAP.md updated where the `staged` meaning and `Kept` arms changed. (Doc, no test: `SetAside`,
      `apply` set-aside, `ApplyError::Journal`, `AfterState` lines.)

## Touches (check each, report how)
- `sets_aside` callers: `preview.rs:363`, `apply` (grep). 74's preview row reads `staged` (graft; not yours).
- Existing set-aside tests, the stash undo suite, `live_clash` table (82) — green.
- ADR-0045 Consequences ("A put-back loses the staged/unstaged split"): unchanged; the tests now prove it.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit. A known failure from the base: none (a1c86055 fixed `published_repo`).

## NOTES

No compaction. Scout: none (ticket says so; review names every line; files read directly).

### Shape changes (public, graft-ops)
- `ApplyError::Journal` gains `kept: Option<String>` (where a set-aside's edits are when kept or clashed; Display
  appends ` · <kept>`). graft matches it with `{ .. }` (`graft/src/jobs/operate.rs:215`), so no graft change.
- `AfterState::SetAsideKept { error: FactsError, words }`: m2/m3 now answer `ApplyError::AfterState` (op ran and is
  journaled → caller refreshes everything) instead of `ApplyError::Facts`, which graft reads as "nothing written"
  (`refresh_after_failure`, no refresh) — wrong after a checkout that moved HEAD. No graft code matches `AfterState`
  variants (grep).
- `Kept::Unlisted` names the copy by commit (`it is stash commit <short>`), not by a second read of the list that
  just failed.

### Test-side finding
- The first m8 hook-undo test failed: the post-checkout hook ran again on undo's own checkout and rewrote `n.txt`.
  Test artifact, not a code defect: the hook is now one-shot (deletes itself) and lives in the git dir, so it is not
  one of the tree's files.

### Not verified
- `Change::TypeChanged` counted as "staging lost": by reasoning from the no-`--index` apply, not a real repo.
- `Change::Copied` not counted: same.
- A staged new file with further unstaged edits (`AM`) comes back `A` with the worktree bytes (staged, split lost);
  not counted and not exercised.

### Touches
- `sets_aside` callers: `preview.rs:363` (sets `PreviewPlan.set_aside` from it — meaning changes, code doesn't),
  `apply` (`set_aside::create` uses only `is_none()`). graft's `r_on_a_dirty_tree_opens_the_preview` builds a
  synthetic `SetAside { files: 1, staged: 0 }` — unaffected. **74 words its row from `staged`: must be told the
  new meaning** (staged new files no longer counted).
- Existing set-aside tests, stash undo suite, `live_clash`: workspace nextest 3425/3425 green.
- ADR-0045 Consequences: unchanged; the round trips now prove it.
