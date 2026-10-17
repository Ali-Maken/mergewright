# Review — ease-pass 05, resolver picks

Reviewed at `7af1cce6` (diff `74f5d13a^1..74f5d13a`, the merge of `edf8c650`). Ticket
`.scratch/ease-pass/issues/05-resolver-picks.md`, scout `.scratch/ease-pass/scouts/05-resolver-picks.md`, note
`docs/design-notes/ease-pass.md` (Journeys, M8–M12, hard choice 3, Q12/Q16/Q17), keymap §2.6.

**Counts: 0 Critical · 3 Major · 9 Minor.**

## What I verified, and how

- **Gates: not run by me.** The conductor said not to run cargo (two builds held the CPU). Per `docs/log/ease-pass.md:55`
  the conductor ran fmt, clippy, nextest and doc after the merge, and all passed. **The heavy tier,
  `clippy --features heavy-tests` and the release perf run were not run for this group.** They must run before the
  milestone closes. Fast-suite time, slowest tests, peak memory and `target/` size were not measured.
- **No work lost: checked by reading the code against the tests.**
  - `apply_untake` (`graft-ops/src/conflict.rs:206-222`) builds the whole document before it writes. It writes once,
    against `file.fingerprint`. `Changed` and `NoSuchBlock` write nothing (`untake_tests.rs:104-141`).
  - Pick, unpick and re-pick each leave the file's bytes equal to the state before (`untake_tests.rs:37-100`;
    `it/resolver/picks.rs:131-210`).
  - The index keeps its 3 conflict stages through a pick and its `u` (`picks/lifetime.rs:24-40`, `ls-files -u`).
    No journal entry is added (same test).
  - A pick's `after` document (`picks.rs:33-35`) is what lets `u` work after a re-read collapsed the taken blocks.
- **Auto-advance stays in the file.** `take.rs` `advance` → `next_open_in_file` on the pending document. When nothing
  is open the cursor stays (`picks.rs:93-109`).
- **`n` goes to the next file.** `walk.rs:46-94` takes the first open clash from the top, which matches the mock's
  `nextOpen` (`mergewright-merge-screen.html:207`).
- **Clashes from a merge begun in the shell:** `clashing_merge` stops on `git merge` run outside, then the app is
  launched (`picks.rs:51-62`).
- **No git work on the UI thread.** The untake is a pool job (`jobs/runner/reads.rs:162-177`). `reconcile`, `seal` and
  `n` work only on memory.
- **No `unwrap`/`expect` added outside tests.** The three `unwrap_or_else` calls added are real defaults, not failures
  being swallowed.
- **Feature pipeline.** The note has Need, Prior art (citing `conflict-resolution.md`) and Journeys.
  - `undo-a-pick` is 1 key (`u`). The build matches: 1 key per pick.
  - `merge-three-clashes` needs 05b's `y`, so it can't be counted here.
- **User answers.**
  - Q16 (a) "re-take replaces" is built (`take.rs:74-85`). `u` restores the earlier take (`picks.rs:157-188`).
  - Q12 (a): built for merge (both origins), rebase and stash put-back. Cherry-pick and revert are untested (m5).
  - Q7 and Q17 belong to 05b. Nothing here contradicts them.

### Ticks: 11 checked, 11 held

| # | Test named | Held? |
|---|---|---|
| 1 | `picks::a_take_moves_to_the_first_open_clash_and_stays…` / `…goes_back_up…` | yes (`picks.rs:93-129`) |
| 2 | `u_puts_each_pick_back_byte_for_byte_newest_first` + op tests | yes (`:131-155`; `untake_tests.rs:37-67`) |
| 3 | `u_after_a_retake_restores_the_earlier_take` | yes (`:157-188`) |
| 4 | `one_u_undoes_a_whole_ctrl_o` | yes (`:190-210`, 3 open, 1 pick) |
| 5 | `n_goes_to_the_first_open_clash_from_the_top` + `first_open` unit tests | yes (`:212-248`; `tests/first_open.rs:25,36,46`) |
| 6 | `a_pick_whose_file_changed_is_dropped_and_u_reaches_the_one_before` | yes (`:268-301`) |
| 7 | `u_with_no_pick_says_so`, stash clause, unit `u_in_the_resolver_says_where_undo_is` | yes (`:303-315`; `lifetime.rs:181-183`; `tests.rs:872`) |
| 8 | `a_pick_in_a_merge_begun_in_the_shell_is_a_file_write_and_no_entry`, relaunch | yes (`lifetime.rs:23-70`) |
| 9 | `a_rebase_stop_takes_and_undoes_and_its_next_stop_starts_clean` | yes (`lifetime.rs:110-163`, `c`, `s`, `a`) |
| 10 | `a_stash_clash_takes_and_undoes_then_closes_as_today` | yes (`lifetime.rs:165-196`) |
| 11 | `mid_merge_u_off_the_resolver_still_refuses_and_keeps_the_pick` | yes (`lifetime.rs:198-235`, graph and banner) |

## Critical

None.

## Major

### M1 — No key walks forward to a taken block, so in a file with every clash taken, the clashes below the cursor can't be reached by keyboard (the builder's question)

- **Where:**
  - `crates/graft/src/update/resolver.rs:385`: `n` is now `first_open`.
  - `crates/graft/src/update/resolver/walk.rs:158-174`: `N` walks backward, crosses files first, then wraps only to
    an open block.
  - `walk.rs:208-216`: `wrap_target` keeps only blocks in `BlockState::Open`.
- **What happens (step by step):** a merge with one conflicted file and three clashes.
  1. `o o o` solves it. The cursor stays on clash 3.
  2. `N N` reaches clash 1. A re-take there (`t`) leaves the cursor on clash 1, because `advance` finds nothing open.
  3. To change clash 3 now: `N` has no file to cross to and nothing open to wrap to, so it says
     `no more conflicts this way`.
  4. Clash 3 can be reached only by mouse (`scroll.rs:109`). `j`/`k` re-read the file, which turns taken clashes into
     plain text.

  With several files, the only keyboard route is `N` through every block of every other file.
- **Why it matters:**
  - It breaks ux-principles §2, heuristic 3 (user control and freedom) and heuristic 7 (efficiency).
  - It breaks the reason the user picked Q16 (a): "changing your mind about an earlier clash is 1 key".
  - `no more conflicts this way` is wrong on the screen where it shows: the clashes are there.
  - Every studied tool has both next and previous: KDiff3's prev/next buttons (`conflict-resolution.md:175`),
    smerge `C-c ^ n`/`p`.
- **Fix: a spec decision, not a build one** (CLAUDE.md: keys come from `02-keymap.md`). Two options for the conductor:
  - (a) No new key. When `N` has no file to cross to and nothing open, it wraps to the file's last block, taken ones
    included. One branch in `walk_blocks`.
  - (b) Give a forward block walk its own key in `02-keymap.md` §2.6.

  Either way, add an `it` test: one file, all clashes taken, cursor on clash 1, reach clash 3 by keyboard.

### M2 — A re-read drops a file's picks without saying so, so the next `u` undoes a pick in a different file

- **Where:** `crates/graft/src/update/resolver/answers.rs:71-74`. `state.picks.reconcile(..)` returns `true` when it
  dropped picks (`picks.rs:136-148`), and the result is thrown away. No production code reads it
  (`git grep '\.reconcile('` finds only this call and unit tests).
- **What happens:**
  1. He picks in `b.txt`, then in `a.txt`.
  2. He presses `e` on `a.txt`, edits it, and returns. The re-read finds other bytes, so `a.txt`'s picks are dropped.
     Nothing says so.
  3. He presses `u`, expecting the pick in `a.txt` to come back. Instead the resolver crosses to `b.txt` and reopens a
     clash he had solved. Only the status line, after the fact, says `undid the last pick in b.txt`.
- **Why it matters:**
  - It breaks heuristic 1 (visibility of system status). The meaning of `u` changed and nothing on screen showed it.
  - It breaks the spirit of rule 3: a dropped state is computed, then ignored.
  - The ticket decided the drop ("`e` returning with the file's bytes changed drops that file's picks too"), but not
    that it happens in silence.
  - This path is tested only at unit level (`picks/tests.rs:129`). No app-level test does a re-read after an edit.
- **Fix:** when `reconcile` returns `true`, put words on the notice row, e.g.
  `a.txt changed since your picks · u now reaches earlier files`. Mark the function `#[must_use]`. Add an `it` test:
  pick, edit the file outside, re-read (`j`/`k` back, or the `e` return path), check the words, then `u` crosses.

### M3 — The guard against writing markers back over a file already staged in the shell has no test

- **Where:** `crates/graft/src/jobs/conflict/write.rs:93-99`. The untake reads the status and refuses a path that is
  not unmerged (`is not conflicted any more`).
- **Why it matters:** `git add a.txt` in the shell does not change the file's bytes, so the pick's fingerprint still
  matches. This status check is the only thing that stops `u` from writing conflict markers into a file whose
  resolution git already holds. It is a data-safety guard, and no test exercises it
  (`git grep 'not conflicted any more'` finds only `write.rs:98`).
- **Also untested:**
  - `UntakeFailure::Failed` → `put_back` (`unpick.rs:189-196`).
  - `untake_ended` on a cancelled or failed job (`unpick.rs:204-209`; wired at `dispatch.rs:230,282`).
- **Fix:** add an `it` test, `clashing_merge`:
  1. Pick in `a.txt`, then `j` to `b.txt`.
  2. Run `git add a.txt` in the shell, then press `u`.
  3. Check: `a.txt` bytes unchanged, `ls-files -u a.txt` empty, the sticky words, and
     `picks.in_file("a.txt") == 0`.

  Add a unit test for `untake_ended`, checking the pick is back on top.

## Minor

1. **Two sets of words for one wait.** `crates/graft/src/update/resolver.rs:307-315`: `write_refusal` asks
   `busy_words` before `picks.undoing()`. `undo_pick` sets `open.job` (`unpick.rs:63-66`), so while an untake runs,
   `o`/`t`/`b`/`⏎`/`d`/`j`/`k` say `still writing the last take`, but `esc`/`e`/`M` say `still undoing the last pick`
   (`handover.rs:21-23`). This breaks heuristic 4 (consistency). Fix: ask `picks.undoing()` first, or teach
   `busy_words` about the untake.
2. **Stale comment.** `crates/graft/src/update/resolver/take.rs:78` says `Question 16 (open)`. The user answered 16 (a)
   on 2026-10-17 (`ease-pass.md:254`). Reword it to "answered (a); a refuse answer would branch here".
3. **Keymap doc drift (textual).**
   - `docs/design/02-keymap.md:352` still gives `n` its old meaning: "Next / previous block, taken ones included".
   - `:353` still says "the document scrolls with `n`/`N`".
   - The Decided row says `n` = "Next **open** clash". The mock and the build both mean "first open clash from the
     top", so the row should say that.
   - Owner: researcher/conductor.
4. **`U` mid-merge words.** `U` in the resolver mid-merge says `finish or abort the merge first`
   (`unpick.rs:129-131`). That reads as "undo is refused here", while `u` one key over undoes picks (heuristic 4).
   For the usability walk.
5. **Cherry-pick and revert untested.** Q12 (a) puts them in scope (ticket Scope line 22), but only rebase is
   tested. They share `stopped_again` (`banner/stop.rs:127-129`), so the risk is low, but untested.
6. **Sealing after `O`/`T` has no test of its own.** It is reached through `resolved_advance` (`resolved.rs:92`,
   called from `answers.rs:260`), and no test asserts `picks.in_file(..) == 0` after `O`/`T`.
7. **Untake perf (unverified).** Each untake reads the whole status (`write.rs:94`) to check one path. This is
   unmeasured on his work repo. A per-path check (`ls-files -u -- <path>`) would be enough. Release perf run item.
8. **Mixed-document stack untested.** The sequence: pick in a file, leave it, come back (the re-read collapses the
   taken blocks and renumbers the rest), pick again, then `u`, `u`.
   - The second `u` works only if rendering a parsed file gives back exactly the bytes it was parsed from, across two
     documents (`picks.rs:12-15`). If that fails, `u` refuses (safe), but the refusal would confuse him.
   - `picks_outlive_a_closed_resolver_but_not_a_relaunch` covers only one pick. Add the two-pick case.
9. **Gaps not yet filed.** The ticket's unverified gaps have no rows in `KNOWN_GAPS.md`: memory per pick
   (`Arc<ConflictFile>`), a rebase continued in the shell, and the stale `m6-01`/`m6-12` captures (keys changed in
   `resolving.rs:663`, `conflict_journeys.rs:84`). The conductor files them.

## Notes, not findings

- **Architecture.**
  - `Restore`/`apply_untake` sit in graft-ops beside `apply_take`, and the job stays thin.
  - `Command::UntakeSide` carries an `update::resolver::Pick` into `jobs`, the same pattern as the existing
    `TakeRequest`.
  - The crate rules are kept, and the MAP.md lines were updated for both crates.
- **Readability.** The new files are under 300 lines. `first_open` is about 50 lines, nests shallowly and is
  documented. Names are plain.
- **Moved code.** The 3 foreign test files changed for `large_stack_arrays` (makers in arrays) are test-only.
  At merge the conductor took 05's version over 06's `vec!` (`docs/log/ease-pass.md:55`).
