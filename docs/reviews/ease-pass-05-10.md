# Review — ease-pass group: the merge screens (05b, 05c, 10)

Fixed hash: `4e6dc034` (main). Diffs: `4ed45107` (05b), `8a12bb6a` (05c), `ca00462f` (10).
Reviewer: `reviewer`, 2026-10-18. Skill: `code-review`.

**Result: 0 Critical · 2 Major · 13 Minor · 2 Questions.** Ticks: **14 spot-checked, 14 held** (one weak, Minor 6).
I did not re-file known gaps already in KNOWN_GAPS (`ease/05c-*`, `ease/10-*`).

## What I verified, and how

- **Gates I ran myself:** `cargo fmt --check` passed. Targeted nextest with its own `CARGO_TARGET_DIR` (since deleted), 3 build jobs and 2 test threads: **166/166 passed**. That covers merge_review, merge_preview, merge_one_file, clash_stops, conflict_solved, live_bar, review_at_y, merge_look, conflict::state, finish::tests, incoming_list, preview_box::merge, dead_keys, screen_keys, the keymap golden list, the sweeps, merge_set_aside, finish_merge and opens_summary. Build took 1 m 08 s and the run 44 s.
- **Gates I did not run:** the workspace clippy, the full nextest and the doctests. The conductor's log says these passed at this hash (`docs/log/ease-pass.md:101`, nextest 6840/6840). I did not repeat them.
- **Not run at all:** the heavy tier, clippy with `--features heavy-tests`, and the release perf run. The conductor asked for light cargo use because another builder is running. These are still owed at the milestone review, which includes `ease/10-spawns` (the merge preview now starts 12 git processes where it used to start 7, and the latency cost is unmeasured).
- **Not measured** in this group run: the 5 slowest tests, peak memory and the size of the full `target/`.
- **Ticks spot-checked.** For each one I read the test body against the tick's claim.
  - 05b, 10 ticks checked, all held:
    - `:63` the bar: `live_bar::a_merge_with_every_clash_solved_bars_y_review_the_merge`
    - `:64` merge_one_file::two_clashes_picked_read_solved_and_stay_unmerged
    - `:65` merge_review::the_review_reads_solved_files_from_a_copy_and_leaves_the_real_index_alone
    - `:66` a_graft_merge_stages_its_solved_files_and_commits_in_one_entry
    - `:67` u_in_the_review_goes_back_to_the_resolver_and_undoes_the_last_pick. This one is weak; see Minor 6.
    - `:68` a_one_clash_merge_is_m_y_o_y_y_and_lands_on_the_merge_commit
    - `:69` esc_enter_in_a_merge_skips_the_solved_file_and_keeps_its_pick
    - `:73` a_hook_refusal_keeps_the_merge_and_the_same_finish_again_commits. It covers both merge origins.
    - `:74` clash_stops::a_rebase_clash_stop_lands_on_the_first_clash
    - `:74` clash_stops::a_cherry_pick_clash_stop_lands_on_the_first_clash
  - 10, 4 ticks checked, all held:
    - the box words: m_on_a_clashing_branch_says_what_comes_in_combines_and_clashes
    - the unrelated withhold: unrelated_histories_withhold_y_from_the_box_and_the_list. `m` after an `m`-opened box is the `m m` alias.
    - the clean merge: a_clean_merge_says_nothing_to_decide_and_m_y_lands_on_the_merge
    - the incoming list: enter_lists_the_incoming_commits_and_esc_comes_back
  - Every test named by any tick exists at the fixed hash, exactly once. I checked this by grepping for `fn <name>`.
- **Journey step counts** (design note `docs/design-notes/ease-pass.md:69-73`), counted in the tests' keys:
  - `merge-one-clash`: 5 keys (`m` and `y` inside `stop_by`, then `o y y`). The note says 5. Held.
  - `merge-clean`: 2 keys. The note says 2. Held.
  - `pause-and-resume`: 2 keys (`esc ⏎`). The note says 2. Held.
  - `merge-three-clashes` (8): not built or walked here. It belongs to ticket 14.
- **Data safety:**
  - The review reads a copy of the index. The real index stays byte-for-byte the same, and the test checks it.
  - Both merge origins stage the files inside the finish's own journal entry, so `u` puts back the stage and the commit together.
  - A file that is not solved is refused at finish time, by the resolver's own rule, re-read from disk (`apply/resume/shell.rs:109-149`).
  - A hook refusal leaves the merge in place.
  - I found no path where work is lost.
- **Git on the UI thread:** none. `merge_review` runs in `jobs/finish.rs:27` and `operation_conflicts` in `jobs/conflict.rs:352`.
- **Terminal escapes:** every path, name, subject and author that reaches the box or the list goes through `sanitize_line` (`update/preview/merge_view.rs`).
- **Rule-3 grep** of all three diffs for `let _ =`, `unwrap_or_default`, `Err(_)`, `.ok()` and `_ => {}`: only non-failure defaults turned up, plus one real case (Minor 4).

---

## Critical

None.

## Major

**M1. The merge preview has no prior-art study.** Ticket 10 is a new screen: what comes in, what git combines, what he decides, and an incoming-commit list.
- **Where:**
  - The design note's Prior art table (`docs/design-notes/ease-pass.md:38-58`) has no row for it. Its merge rows cover only the landing, the clash screen and the finish.
  - No file under `docs/research/` studies how tools preview a merge before it runs. I grepped `docs/research/*.md` for "merge preview", "incoming commits", "what comes in", dry-run and merge-tree. `conflict-resolution.md` §1–3 starts after the merge has stopped.
  - The journey row `merge-clean 2 … lazygit 2` (`:70`) gives a count with no source behind it.
- **Why it matters:** CLAUDE.md pipeline step 2. A feature without a prior-art study is Major by the review rule. The user accepted the mock, but the take/reject/invent choices for this screen were never recorded against other tools: counts versus file lists, a separate incoming list, the clash forecast with places, the unrelated-history refusal.
- **Fix:** have the `researcher` write `docs/research/merge-preview.md`. Cover:
  - Sublime Merge's merge dialog
  - GitKraken's conflict detection before a merge
  - Tower's conflict prediction
  - the JetBrains "Merge into" dialog
  - lazygit's merge menu
  - GitHub's "can be merged" panel
  - users' praise and complaints, with links

  Then add the rows (take / reject / invent) to the note's Prior art table. Nothing needs rebuilding unless the study disagrees.

**M2. 05b raised the file-size ratchet on 22 files and added 2 new files over 400 lines.**
- **Where:** `crates/graft/tests/it/over_400.txt`, diff in `4ed45107`. The new or raised rows:
  - `:167` `update/finish/tests.rs` 823→**896** (+73)
  - `:242` `m5_walk/conflict_journeys.rs` 1355→1396
  - `:251` `m5_walk/run.rs` 781→820
  - `:227` `finish/choices.rs` 430→442
  - `conflict_interim.rs` 1994→2006
  - `banner/tests.rs` 1965→1977
  - `resolver/file_list.rs` 705→717
  - graft-ops `it/finish.rs` 443→453
  - graft-ops `it/history_ops.rs` +7
  - **new:** `:270` `tests/it/resolver.rs` **419**
  - **new:** `:273` `tests/it/resolver/file_list/start_over.rs` **407**

  05c and 10 lowered the rows they touched. 05c lowered `view/rows.rs` from 897 to 845, and 10 moved `preview/effects.rs` off the list.
- **Why it matters:** PROMPT.md 6.2 is a hard rule. The user's standing word is "split files over ratchet, don't raise it". A ratchet that only goes up is not a limit.
- **Suggested splits:**
  - move 05b's review tests out of `update/finish/tests.rs` into `update/finish/tests/review.rs` (about 75 lines)
  - move the `two_file_cherry_pick` fixture and its users out of `tests/it/resolver.rs` into `resolver/fixtures.rs`
  - split `start_over.rs` into one file per merge origin
  - move `conflict_journeys.rs`'s +41 lines into a `merge_review_journeys.rs` sibling

  After each split, lower the row or drop it.

## Minor

1. **A fast-forward gets the true-merge box and its words.**
   - **Where:** `graft-ops/src/preview/kinds.rs:81` sets `plan.merge` on the fast-forward path. The box then says `Git combines N files on its own` (`graft-ui/src/preview_box/merge.rs:177`) and `After y the merge is committed.` (`:195`, chosen only by `decide.is_empty()`).
   - **Why it matters:** no merge commit is made and git combines nothing. A fast-forward and a merge commit are different outcomes, and the box draws them the same. 10's NOTES flags the words as a question, but KNOWN_GAPS has no row for it.
   - **Fix:** add a gap row now. Then either give the fast-forward its own words (`After y wallet-release moves up to sara/fix-toc.`, and drop `Git combines`), or ask the user.

2. **A refused banner `c` or resolver `y` now opens the review and then closes it.**
   - **Where:** `update/banner/pressed.rs:39` skips `continue_refusal` for every merge. `:57` then calls `open_finish`, which sets `finish = Some(.. Pending)` and sends the read. `update/finish/answers.rs:31-36` closes the review when the read comes back `StillOpen`.
   - **Why it matters:** before 05b a file still open was refused at once. Now every refusal waits on a job that opens and parses every unmerged file, and the review is open while it waits. I did not observe whether a frame draws the empty review in between.
   - **Fix:** in a merge, refuse at once when the resolver knows of a clash still open (`review::clashes_open` already answers this). Ask the read only when unread files are left.

3. **The same key, two units in its words.**
   - **Where:** resolver `y` says `N clashes still open · n goes to the next` (`update/resolver/review.rs:86`). When the review's read refuses instead, it says `N files still open · n goes to the next` (`update/finish/answers.rs:75`).
   - **Why it matters:** the user cannot tell which count he is reading. The spec gives `N clashes still open`.
   - **Fix:** have the read return the clash count, or word both as files.

4. **A failed scratch-index removal is dropped when the read also fails.**
   - **Where:** `graft-ops/src/finish/review.rs:136-138`. `let removed = copy.remove(); let summary = summary?;` returns before `removed` is looked at.
   - **Why it matters:** rule 3. The comment at `:133-134` promises a copy that would not go is "said rather than left behind in silence". That holds only when the read succeeds.
   - **Fix:** report both errors when both fail, or comment why the removal error is safe to drop there. It is inert scratch under a unique name, as `snapshot/temp_index.rs:140-145` argues.

5. **`finish_preview` is public, has no non-test caller, and trusts its `solved` list.**
   - **Where:** `graft-ops/src/finish/review.rs:90`. The app goes through `merge_review`.
   - **Why it matters:** `finish_preview` stages whatever it is given on the copy, markers included. A future caller that passes an unsolved file gets a review of a marker-filled result.
   - **Fix:** make it `pub(crate)` and go through `merge_review`, or check each path with `file_state` as `unmerged_to_stage` does.

6. **One review `u` tick is only partly proved.**
   - **Where:** `update/resolver/tests/review_at_y.rs:263-269` asserts that the review closed, the resolver is open and an untake was sent. The tick's "cursor on it" is not asserted, and no integration test drives review `u` through to its landing.
   - **Why it matters:** the tick claims more than its test checks. The tick still holds on 05's untake-landing tests, so it is weak, not false.
   - **Fix:** add an app test: review, `u`, pump, then assert `cursor_block` is on the undone clash and `open == 1`.

7. **`u` means two things on the review.**
   - **Where:** the review box still reads `undo: u takes this back` (`docs/shots/m6/fix-finish-from-the-resolver-120x40.txt:4`), meaning `u` undoes the merge after it lands. Inside the review, `u` now undoes the last pick (`update/finish/keys.rs:103`).
   - **Why it matters:** this is a consistency heuristic (`ux-principles.md`).
   - **Fix:** route to 12 (Fr), which owns the review's words. The mock's review footer says `u undo last pick`.

8. **Departure from the mock: the solved file's word.**
   - **Where:** the mock's file list marks a file with no clash open as `✓ … done` (`prototypes/ease-mocks/mergewright-merge-screen.html:145`). The build draws `solved ✓` (`docs/shots/m6/fix-finish-row-120x40.txt:2`) and keeps `done ✓` for staged files.
   - **Why it matters:** the distinction is 05b's Decided, but it changes words the user accepted in the mock ("build to the mocks").
   - **Fix:** take it back to him through 11 (L), which owns the list's look, before 11 builds.

9. **One state, three names.**
   - **Where:**
     - `graft_ops::conflict::FileState` (`graft-ops/src/conflict/state.rs:11`: Open, Solved, Done)
     - `graft_ui::conflict_resolver::FileState` (`graft-ui/src/conflict_resolver/files.rs:47`: AllTaken, Solved, Done …)
     - the app's `RowState::AllTaken`, which stands for "solved" in a merge (`update/resolver/files.rs:101`)
   - **Why it matters:** CONTEXT.md has `Done (file)` but no `Solved`. PROMPT.md 6.2 asks for glossary terms used exactly.
   - **Fix:** add `Solved (file)` to CONTEXT.md, and rename one of the two `FileState` types (for example the ui one to `FileRowLook`) so a reader of graft code that imports both is not misled.

10. **Two capture names mislead, and no gap row says so.**
    - **Where:** `docs/shots/m6/fix-finish-row-120x40.txt` shows no finish row. `fix-last-file-opens-summary-187x50.txt` shows `y` opening the review, not the last file. 05b's NOTES says so, 05c repeats it, and KNOWN_GAPS has no row.
    - **Fix:** rename the files with their test, or file a gap.

11. **Six source files sit at the 400-line ceiling.**
    - **Where:**
      - `graft-ui/src/graph_screen/hints/lists.rs` 400
      - `graft/src/update/resolver.rs` 398
      - `graft-ui/src/conflict_resolver/files.rs` 397
      - `graft-ops/src/preview/effects.rs` 395
      - `graft/src/update/preview/answer.rs` 391
      - `graft/src/update/finish/keys.rs` 390
    - **Why it matters:** the next ticket to touch any of them (11, 12 and 14 will) must split first, and should not raise the ratchet (M2).
    - **Fix:** name the splits in 11's and 12's specs now. For example `finish/keys.rs` into its text-field and file-list halves, and the `Clash` arms of `hints/lists.rs` into `lists/clash.rs`.

12. **Missing Touches lines.**
    - **Where:** 10's Touches section (`.scratch/ease-pass/issues/10-merge-preview.md`, bullet 3) required grepping `\.conflicts` across the workspace and listing each reader. 10's NOTES has no such list, and no `Touches:` line. 05b's NOTES has no `Touches:` line either. 05c's has one.
    - **Why it matters:** CLAUDE.md "check the knock-on effects" requires a Touches line in every builder report.
    - **Fix:** the conductor asks 10's builder, or a scout, for the reader list. I saw no wrong reader, but I did not do the grep for it.

13. **The keymap doc does not say `y` is withheld.**
    - **Where:** `docs/design/02-keymap.md:304` says the merge preview's `y` merges. It does not mention that `y` is withheld over unrelated histories. 10's NOTES lists the doc edits it proposes, but they were not made and no gap row records them.
    - **Fix:** add the sentence to §2.3b (and §1.7 of `01-view-mockups.md`), or file a gap.

## Questions (for the conductor)

- **Q1.** 05c was built by `builder-opus` (`docs/log/ease-pass.md:87`), but its ticket says `Build with: Fable` (`.scratch/ease-pass/issues/05c-merge-review-rest.md:5`). Was this a deliberate re-route? The log does not say.
- **Q2.** 05b edited the journey counts in `docs/design-notes/m6-conflicts.md`: seven merge rows +1 for the review's `y`. Its NOTES asks the conductor to confirm the edit. I found no confirmation in the log (`:77-78`). Confirmed?

## Checked and fine (no finding)

- **One staging path for both merge origins.** `solved` rides on `Resume` (`apply/resume.rs:36-60`). In the shell origin it is copied into `FinishMergeParams` (`resume/shell.rs:59-70`) and is `FinishMerge`'s first plan line (`operation/finish_merge.rs:52-55`). `operation/check.rs:38-40` lets `FinishMerge` past `Unmerged` only when every unmerged path is in `solved`.
- **The finish re-judges `solved` from disk** with `file_state`, so a stale app list cannot stage a file with markers (`resume/shell.rs:129-147`).
- **The paused banner's `s skip`** follows `skippable = kind != Merge` (`update/banner.rs:219`, `hints/lists.rs:330`).
- **The incoming list** swallows unbound keys, as the box does. A merge preview's `⏎` is chosen by op kind (`merge_view.rs:look`), not by `opened_by`. Rebase and other previews are untouched (`a_rebase_preview_keeps_its_buttons_and_enter`).
- **One forecast per merge preview.** The spawn log shows one `merge-tree` (`merge_set_aside/spawns.rs:11-52`).
- **The no-merge-base fix** (`in_the_way/written.rs:39-48`) fixes the cause, not the symptom.
- **Captures** are 2.7–3.8 KB, far under the 200 KB cap. Colour is unverified (rule 5), and the look is the user's to judge (rule 6).
