# Review: m6-conflicts 35 (finish summary: the words for what you chose, the real `$EDITOR`, the captures)

Fixed hash `4b344bff`. Diff `3de12dfe..4b344bff`, one commit. Ticket 35 ticks ticket 19's boxes `:161`, `:169`–`:172` and `:182`. Findings already filed from `docs/reviews/m6-amend-19-34.md` (ticket 36) are not repeated here.

**Counts: Critical 0 · Major 2 · Minor 9.**

## What I verified, and how

- **I did not run the gates.** The conductor runs them. Every claim below comes from reading the code at the hash. None of the behaviour described was observed by running it.
- **Tick spot-check: I checked all 6 ticks, and all 6 held.** For each one I read the named test and matched its assertions against the box's words.
  - `:161` `it::finish::a_hash_line_from_the_editor_lands` (`tests/it/finish.rs:191`).
    - **Does a real `$EDITOR` process run? Yes.** A script set as `core.editor` is spawned by `run_the_fixture_editor` (`:161-182`), through the app's own `event_loop::outside::editor_command`, on `.git/graft/COMMIT_EDITMSG`. That is the app's `MESSAGE_FILE` (`outside.rs:83`).
    - It asserts the appended line three times: in the file, in the body, and in `%B`.
    - The tick states what it leaves out (the terminal suspend). See Minor 1 for what else it leaves out.
  - `:169` `the_summary_words_what_you_chose` (`:440`).
    - With real git, `Ctrl+T ⏎` then `o` draw `theirs · feature in 2 of 2 conflicts` and `kept · yours · main` at 120×40.
    - After a relaunch, `choices` is empty and the rows show the `ResultIs` words.
  - `:170` `shift_t_records_its_choices` (`:477`): the `T` row reads the take-count words, not `same as theirs`. So `T` records.
  - `:171` `a_start_over_forgets_the_choices` (`:493`):
    - the entry exists after `t`;
    - the restart lands with 2 blocks open;
    - the entry is gone.
  - `:172` `choices_and_draft_clear_when_the_merge_ends` (`:571`), with the app's own merge:
    - after `⏎`, `MERGE_HEAD` is gone, `choices` is empty and `finish_draft` is `None`;
    - after `a` → confirm, the same holds.
  - `:182` `summary_captures` (`:692`).
    - **Pinned:** `check_shot` → `graft_fixtures::capture` compares the frame with the committed file and fails on drift (`graft-fixtures/src/capture.rs:145-150`, `check_text`).
    - **Size:** the four files are 592, 650, 742 and 716 bytes, far under the 200 KB cap.
    - It asserts the title, both section counts, `▸ README.md modified +1 −1`, the first line of `MERGE_MSG`, that the bar starts `⏎ commit`, that there is no `load all`, and that the diff rows are indented.
- **`O`/`T` record through `take_landed`.** `record_landed` is called at `resolver/answers.rs:287`, before `open.file` is replaced (`:294`) and before `in_flight` is cleared (`:293`).
  - `taken` checks `open.job == id` (`:184`) before `take_landed`, so a stale answer records nothing.
  - A queued take is sent against the document that landed before it (`:301-309`), so `open.file` at landing is the document that request was applied to. The `Every` diff is right.
  - `shift_t_records_its_choices` shows it end to end.
- **`ResolverState.whole_side` cannot go stale today.** I grepped every writer of `staging` (`whole_file.rs:92`, `resolved.rs:52`, `:78`, `answers.rs:397`). Each sets or clears `whole_side` in the same block. `resolved_advance` reads it only when `staging == Some(id)` (`resolved.rs:68-73`). A refused launch leaves `operating` `None`, so nothing is recorded (`whole_file.rs:94`). The invariant rests on discipline, though: Minor 5.
- **`AppState.conflict_editing` cannot go stale in the real loop.**
  - `OpenConflictEditor` is carried out inside the same command drain that `edit_conflict` fed. Its answer is `update`d on the spot (`event_loop/running.rs:188-199`), and there is no other event in between.
  - The headless path answers `HandedOver(Err)` (`:230`), which takes the path too.
  - The resolver arm (`update/mod.rs:289`) has precedence, but the resolver is modal, so banner or changes-pane `e` cannot fire under it.
  - Design note: Minor 4.
- **Errors.**
  - No new `let _`, ignored `Result` or `unwrap`/`expect` in non-test code. The removed `let _ = state;` (`banner.rs:559`) is now a real use.
  - `was.map(..).unwrap_or_default()` (`choices.rs:46`) is on an `Option` that means "no earlier document", and it is commented.
  - No git work on the UI thread was added. Nothing new is drawn, so there is no new sanitising surface.
- **The feature pipeline** was checked by the 19+34 review, and this ticket changes no journey. `:181` (the step counts) is still ticket 21's.

## Critical

None. 6 of 6 ticks held.

## Major

### 1. The choices and the draft outlive a merge that ended outside Graft, and the next merge shows them
`crates/graft/src/update/mod.rs:605-612`, `crates/graft/src/update/undo.rs:130-131`.

- **What's wrong:**
  - `choices` and `finish_draft` are cleared only in `undo::operation_ended` (`undo.rs:130-131`). `git grep` finds no other clearer.
  - `operation_ended` runs only on Graft's own resume answering `Done` (`banner.rs:620`, `:682`).
  - The working-tree read that finds no operation at all (`mod.rs:605`) clears `banner_focus`, `stopped` and `live_landing_owed`. It does not clear `choices` or `finish_draft`.
  - The code's own comment says this branch is where "the operation ended, in another terminal".
- **How it is reached:**
  1. A merge begun in the shell (`git pull` stopped) is resolved in Graft. This records choices.
  2. The summary's `⏎` answers `nothing Graft started is in progress; an operation begun in the shell is finished there` (`graft-ops/src/operation.rs:582`, observed by 35's builder; until ticket 20 lands P4).
  3. So the user commits in the shell.
  4. The next conflicted merge in the same session touches a file with the same path, which is common for `CHANGELOG`s and lock files on a team's dependent branches.
  5. Its `RESOLVED BY YOU` row reads the last merge's `theirs · feature in 2 of 2 conflicts`.
  6. If he typed a message and pressed `Esc` last time, `open_finish` puts that old draft into the new merge's fields (`update/finish.rs:145`). An unedited `⏎` then commits it with `ContinueWith`.
- **Why it matters:** the summary exists to say truthfully what you chose ("you see the result before you confirm"). Words from another merge are false. A message he wrote for merge A lands on merge B unless he reads the field.
- **Scope:** spec item 3 named only `operation_ended` and the abort's answer, so this is a spec gap as well as a build gap. The MAP line (`MAP.md:346`, "Cleared in `undo::operation_ended` (Done and abort)") is accurate about the code. The 19+34 review's Minor 6 raised the same trigger for `finish` itself. This is its data half, and it is now reachable because 35 records.
- **Not run.** This is from reading. I did not drive a shell commit followed by a second merge.
- **Suggested fix:** in `mod.rs:605`'s `in_progress.is_none()` branch, clear `choices` and `finish_draft` (and close `finish`, which covers 19+34 Minor 6). Better: move the three "belongs to the operation" clears into one `undo::forget_operation(state)` called from both places. Test with real git: record a take, commit with `git commit` outside, pump the re-read, then assert `choices` is empty and `finish_draft` is `None`.

### 2. Files grown past the ~400-line rule, and one new crossing
PROMPT.md 6.2 (a readability rule; Major by this review's brief). Line counts are `wc -l` at the hash, with the base in parentheses:

| File | Lines | Note |
|---|---|---|
| `crates/graft/tests/it/finish.rs` | **788** (299) | +489 in this ticket, about 2× the rule |
| `crates/graft/src/update/resolver/answers.rs` | **403** (387) | crosses 400 in this ticket |
| `crates/graft/src/update/mod.rs` | 969 (952) | |
| `crates/graft/src/update/resolver.rs` | 412 (406) | |
| `crates/graft/tests/it/resolver/file_list.rs` | 680 (656) | |

- **What's wrong:** no `KNOWN_GAPS` size row names any of these files. Test files are held to the rule: gap M6/07 names `m5_walk/*.rs`, and M5/18 names `it/context_menu.rs`.
- **Why it matters:** `tests/it/finish.rs` now holds three unrelated families: commit and message, the choice words, and the captures. The next ticket (21) will add to it.
- **Suggested fix (`codebase-design`, pure moves):**
  - Split `tests/it/finish.rs` into `finish/message.rs` (`:1-363`), `finish/choices.rs` (`:367-677`, with its helpers `stopped_on_gone_and_notes`, `advanced_to`, `row_for`, …) and `finish/captures.rs` (`:686-788`), following `it/resolver/`'s precedent.
  - For `answers.rs`, move `record_landed` and `landed_words` next to `take_landed` in a `resolver/landed.rs`, or record the size in a `KNOWN_GAPS` row with the counts above.

## Minor

1. **`:161` runs a real editor, but not the app's `edit_message`.** `tests/it/finish.rs:161-182` re-implements `Handover::edit_message` (`event_loop/outside.rs:96-101`): the file write, the run, the read-back. It also re-implements `git_editor`'s lookup.
   - The tick holds, and its doc says "what `Handover::edit_message` does".
   - But S3 ("nothing strips before the product") is guarded at `outside.rs:100`, and the test runs its own copy of that line. If someone added a strip there, this test would still pass.
   - Fix: split `edit_message` into the terminal-free part (write, run through a given closure, read back) and the `run_outside_terminal` wrapper, then have the test call the first.
2. **Question: `edited` outranks takes whatever the order, and whether or not the editor changed anything.**
   - `record_edited` fires on any `Ok` handover (`update/mod.rs:291-294`), and `by_you_words` checks `edited` before takes (`update/finish.rs:790`).
   - Example: `e` in the resolver to look, quit vim (exit 0), then `T`. The row reads `edited in your editor`, not `theirs · feature in 2 of 2 conflicts`.
   - This follows spec item 3 as written. Conductor: should a take that lands after an edit clear `edited`, or should the order be kept?
3. **A file resolved outside after in-app takes keeps its entry, so its row reads the takes.**
   - Example: `t` on block 1, then `git checkout --ours notes.txt && git add` in the shell. `closed_resolved_outside` (`resolver/answers.rs:166-171`) does not `forget`, so the row says `theirs · feature in 1 of 2 conflicts` about a file that is all yours.
   - It also contradicts the MAP line "a file resolved outside reads `ResultIs` words" (`MAP.md:346`).
   - Fix: call `finish::forget` in `closed_resolved_outside`, or narrow the MAP wording. From reading; not run.
4. **`AppState.conflict_editing` is state standing in for a missing product field.**
   - `Product::HandedOver` carries no path, so the path is parked on `AppState` for one synchronous round trip (`update/state.rs:327-332`, `update/mod.rs:305-311`).
   - The precedent is `Product::Mergetool { path, result }` (`event_loop/running.rs:245-248`).
   - A `Product::ConflictEdited { path, result }` answered at `running.rs:241-244` would remove the field, the extra match arm and the ordering dependence on the resolver arm.
5. **`whole_side` must be written beside `staging` at four sites** (`whole_file.rs:92-94`, `resolved.rs:52-53`, `:78-79`, `answers.rs:397-398`).
   - All four are paired today.
   - Gap M6/07 already names the same shape for `reading`/`job`. Folding them into `staging: Option<(JobId, Option<Side>)>` makes a stale side impossible rather than avoided.
6. **Paths with no test:**
   - a refused or failed whole-file write records no `whole` (`write_not_landed` clearing it, `answers.rs:398`);
   - a failed **resolver** `e` handover marks nothing (only the banner route's `Err` is tested, `tests/it/finish.rs:645`);
   - the banner route's `Err` still reaching `commit_editor_answer`'s sticky status (not asserted);
   - a mergetool path recorded under a key the summary's row actually looks up (only the map key is checked).

   `choices::tests::whole_edited_and_forget` also tests three functions in one test.
7. **Readability, small.**
   - `mod choices;` and its `pub(crate) use` sit at the bottom of `update/finish.rs:830-831`, below the functions. Move them to the top with the other items.
   - `record_take`'s `Block` arm drops an out-of-range index silently (`choices.rs:38-41`). It can't happen, since the take landed on that block, but say so in a comment (CLAUDE.md rule 3).
8. **The TESTING.md line for `:182` is handed to ticket 21, but 21 doesn't say so.**
   - The NOTES defer the line that quotes `head -1 .git/MERGE_MSG` (`Merge branch 'feature'`) to 21's checklist.
   - `21-acceptance-conflict-journeys.md:8` names only `fix-finish-summary-screen` in general.
   - Add the quote and the four capture names to 21, or it will be lost.
9. **For the user's eye (textual, from the captures):**
   - In `fix-finish-summary-160x48.txt` the subject's 50-column guide `┊` would sit at column 157, under the `22/50` counter (columns 155-159). At 160 the guide is hidden, while at 120 and 200 it shows.
   - In `…-expanded-120x40.txt` the first row under `▸ README.md` is blank (the hunk heading with no function context). The builder flagged this.
   - Colour is unverified (rule 5).
   - The sibling fixture `gone_notes_and_readme_merge` (a deviation from "extend `gone_and_notes_merge`") is sound: it leaves already-judged captures untouched. Conductor to accept or not.

## Gates

I did not run them; the conductor runs them. 35 reports that fmt, clippy `-D warnings`, nextest (2949/2949) and doctests were green in its worktree on 2026-09-24. I have not observed that. The heavy tier and the release perf run are for the milestone review.
