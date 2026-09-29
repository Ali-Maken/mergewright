# Review: m7-power-tools, merges 4df7fa65..71fb9494 (08-a, 08-b, 07d, 30l, 24-0, rebase-preview-keeps-drops)

The reviewer was read-only against **71fb9494**. Built and tested in the review worktree, which was at that hash with
a clean tree apart from four untracked m6 scratch files. It used its own `CARGO_TARGET_DIR` in the scratchpad, which
was removed afterwards.

**Counts: Critical 0 · Major 3 · Minor 11 · Question 3.**

## Gates (run one at a time, at 71fb9494)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` (workspace) | 4782 / 4782 pass, 4 skipped |
| `cargo test --doc` | pass |

- **Fast suite:** 286.1 s nextest wall (286.8 s total). Peak RSS 272 MB (`/usr/bin/time -l`). `target/` 4.5 GB.
  **Under load:** loadavg was about 7 throughout. That is over PROMPT 7.1's 3 min. The quiet-window re-measure is
  already owed (log :425).
- **5 slowest** (second full run, 311.7 s, 4782/4782 pass, same load):
  - graft-ui `hints::tests::the_bar_never_draws_a_fragment_of_itself` 16.1 s
  - `no_hint_loses_ground_as_the_room_for_it_grows` 14.2 s
  - `a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 14.0 s
  - `the_bare_keys_are_a_tail_not_a_scatter` 13.9 s
  - graft-ops `undo_cancel::cancel_at_every_step_boundary_is_clean` 12.4 s

  The four hint sweeps grew about 1 s each since the last review (15.0/13.0/13.0/13.6). 08-b added
  `HintContext::Rewritable`, `RewriteEditor` and `RewriteMessage`, and each new context multiplies these sweeps.
- **Heavy tier, release perf run, fuzzing and benchmarks: not run**, on the conductor's word ("do not run perf";
  fseventsd pegged, log :431). They are owed at the quiet window before 26.

## Ticks spot-checked: 28 checked, 28 held

For each one I read the test body against what its box says.

- **08-a/08-b (14):**
  - `capital_r_opens_on_the_cursor_row`, `capital_r_on_another_branch_is_refused_in_words`, `esc_writes_nothing`
  - `y_launches_the_planned_rewrite` (asserts that the op run is the plan's `runs`)
  - `an_edit_replans_and_the_last_wins`, `a_superseded_after_answer_is_dropped`, `a_moved_branch_refuses_y_in_words`
  - `a_refused_write_restores_the_editor_with_the_words`, `u_while_the_editor_is_up_does_nothing`
  - `the_bar_offers_capital_r_on_his_commit_rows`, `the_bar_hides_capital_r_on_default_branch_rows`
  - the walks `squash_last_three` (`R s s y`, fenced, the message = the drawn block, cursor on the new HEAD, `u`),
    `swap_adjacent` (`R Alt+↑ y`, fenced) and `rewrite_look_and_leave_writes_nothing` (refs, index, files and
    journal)
- **07d (5):**
  - `space_on_the_stopped_row_stages_every_change` and `space_on_the_uncommitted_row_at_an_edit_stop_stages`. Both
    compare against `stage_all` run on a copy, which asserts that it stages. Not vacuous.
  - `space_on_a_merge_row_at_an_edit_stop_still_folds`
  - `progress_names_each_follower_with_author_and_count`
  - `edit_stop_amend_continue` (`space a c`, `within`)
- **30l (2):**
  - `o_on_an_unborn_head_says_check_picks_words`. It holds against its box, but the words are false; see Minor 3.
  - Minor 3's comments (`running.rs:210`, `:232`, `:284` say "six" and match the arms)
- **24-0 (2):** `a_stash_drop_has_one_name`, `the_drop_after_a_clash_carries_its_why`
- **rebase-preview-keeps-drops (5):**
  - `from_rebase_unedited_matches_the_rebase_todo`. It compares against git's own `rev-list`.
  - `keep_a_suggested_drop` (route `k r R p y` = 5, `within`, then `main..feature-a` = `[a3]`, parent, `u` per
    branch)
  - `r_is_unbound_in_other_previews`, `capture_keep_drops_editor`, `footer_r_keep_whole_or_absent_at_80`

## Journey fence vs the walks

| Journey | Note | Walk route | Holds |
|---|---|---|---|
| squash-last-three | 4 | `R s s y` | yes |
| swap-adjacent | 3 | `R Alt+↑ y` | yes |
| edit-stop-amend-continue | 3 | `space a c` | yes |
| abort-from-a-chain-stop (new row) | 3 | `esc a y` | yes |
| keep-a-suggested-drop (new row) | 5 | `k r R p y` (fixture puts main one row down) | yes |

Both new fence rows landed in the same commit as their walks, as the conductor's notes asked.

## Keymap check against `docs/design/02-keymap.md`

- §2.3 `R`: `update/keys.rs` `write_key` sends it only when the graph has focus and there is no overlay.
- §2.3b `R`:
  - `update/preview.rs:430` acts only through `keeps_drops`;
  - the footer shows `R keep`, and every other preview capture stays free of it (`grep -rl "R keep" docs/shots` →
    only the re-taken file).
- §2.7: the keys are `update/rewrite_editor/keys.rs:157-183`.
  - `p e s f d`, `r`, `Alt+↑/↓`, `y`, `⏎`, `Tab` <120, `Esc`, and a drag. Plus `j/k`, the universal motion.
  - `Ctrl+C` passes through (`:39`).
- §2.6a (07d D1 clause): `update/keys/edit_stop.rs`.

No key in this range is bound outside its row. The one consistency point is Question 3 (`r` vs `R`).

---

## Critical

None.

## Major

### M1. The m7 contact sheet is stale: the keep-drops capture is missing and the squash-merged preview is the old one

- **Where:**
  - `docs/shots/m7/m7-contact-sheet.txt`, last written by 08-b (`git log` → db964b10);
  - `docs/shots/m7/m7-contact-sheet-captions.tsv`, which has no `rebase-keep-drops-editor-120x40.txt` row.
- **What:**
  - rebase-preview-keeps-drops re-took `rebase-squash-merged-preview-120x40.txt` and added
    `rebase-keep-drops-editor-120x40.txt`, but did not regenerate the sheet.
  - `grep -c "R keep\|R to keep\|p keeps it" m7-contact-sheet.txt` → 0. The sheet shows the pre-change box (no
    `· R to keep them`, no `R keep` in the footer) and has no editor capture.
- **Why it matters:** ADR-0037 and rule 6. The user judges by scrolling the sheet once, so he would be judging a
  picture the app no longer draws, and would miss the one capture this ticket exists for. The ticket's Files list
  left out the TSV and the sheet (spec defect). It said "nothing else", so the builder was right not to touch them.
- **Fix:** add the TSV row after `rebase-squash-merged-preview-120x40.txt` with its Right line (TESTING.md already has
  it). Then regenerate with `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`, and check the result with
  `grep "R keep"`. The planner's Files template should name the TSV and the sheet whenever a ticket adds or re-takes a
  capture.

### M2. New files born over the ~400-line rule (PROMPT.md 6.2)

The ticket allowed child modules (`keys`, `after`, `words`), and they exist. The parents are still over.

| File (non-test lines unless noted) | Lines | Note |
|---|---|---|
| `crates/graft-ui/src/rewrite_editor.rs` | 939 | tests already in a child file |
| `crates/graft/src/update/rewrite_editor.rs` | 716 | |
| `crates/graft/src/view/rewrite_editor.rs` | 420 | |
| `crates/graft/src/update/rewrite_editor/keys.rs` | 413 | |
| `crates/graft-ops/src/rewrite/plan.rs` | 288 → 571 | its new tests are inline (`#[cfg(test)] mod tests` at :336), unlike the crate's `x/tests.rs` convention |
| `crates/graft/src/update/keys.rs` | 398 → 438 | crossed the line |
| `crates/graft-ui/src/graph_screen/hints/lists.rs` | 406 → 485 | |

- **Why it matters:** 6.2 is a hard rule, and these are new files, not legacy growth. 30m was scoped before them and
  lists none of them.
- **Fix (add to 30m; pure moves):**
  - graft-ui `rewrite_editor.rs` → `rewrite_editor/view.rs` (the view types, :81-240), `layout.rs` (:262-445) and
    `draw.rs` (:447-939);
  - graft `update/rewrite_editor.rs` → move `ViewFacts`/`RowFacts`/`MessageFacts`/`AfterFacts` (:169-247) into
    `words.rs`, and `owns`/`after_planned`/`files_listed`/`message_returned`/`job_ended`/`write_answered` (:545-700)
    into `routing.rs`;
  - `plan.rs` tests → `rewrite/plan/tests.rs`.

### M3. Names and plain code in the new private types (PROMPT.md 6.2 "Names", "Plain over clever")

- **`update/operate.rs:1017-1026`:**
  - `struct Stop(String, InProgress, Vec<PathBuf>, Option<String>, EntryId, Option<Result<RewriteStop, String>>)`
    has six positional fields, and the only record of their order is a comment.
  - In the same function, 12 lines later (`:1044`), `let stop = super::state::Stop { … }` is a *different* type with
    the same name, and `graft_git::run::Stop` is a third in the crate.
  - A reader of `stopped` meets two unrelated `Stop`s in one screen.
  - Fix: a struct with named fields and a distinct name, e.g. `ConflictedAnswer { description, kind, paths, stalled,
    entry, rewrite }`.
- **`update/operate.rs:842-846`:**
  - `incomplete_answered` carries two stacked doc paragraphs. The first ("`operated`'s `Incomplete`: the write ran,
    only part of its after-state could not be read back") is `incomplete`'s old doc.
  - `fn incomplete` (:860) now has none.
  - Fix: move that paragraph back onto `incomplete`.
- **`update/rewrite_editor.rs:104`:**
  - `by_name: HashMap<Oid, MarkedBy>` now also holds rebase-suggested drops (`MarkedBy::AlreadyIn`). The name is kept
    only to avoid editing `keys.rs` (ticket NOTES).
  - `keys.rs:205`'s comment "not the one its name gave it" is stale for the same reason.
  - Fix: rename it `marked_by` and reword the comment.

## Minor

### m1. `R` acts in a preview whose footer does not name it (the `undrawable` box)

- **The key:** `update/preview.rs:430-438` acts whenever `keeps_drops_here` is `Some`.
- **The footer:** `footer_keys` (:530-531) returns early, with no `keep`, when `!preview.acts()`. `acts()` (:239-241)
  is false when the plan landed but `undrawable` is set. That box draws `preview cannot be drawn here: … · esc`
  (:895-902).
- **The result:** `R` there closes the box and opens the editor, with no key named.
- **The claim it breaks:** `keeps_drops`'s doc at :472-476 says "the key, the row and the footer all ask this, so none
  of them names a key the others would not answer".
- **Fix:** have `keeps_drops_here` also require `preview.acts()`, and add the undrawable case to
  `r_is_unbound_in_other_previews`.

### m2. `from_rebase` copies the engine's drops-into-rows loop

- **Where:** `graft-ops/src/rewrite/plan.rs:109-118` is `apply/engine.rs:162-172` line for line, including the `Moved`
  refusal for a drop with no row.
- **Why it matters:** the ticket's correctness anchor ("unedited, the editor runs the rebase's own todo") now rests on
  two copies staying equal.
- **Fix:** one `pub(crate) fn rows_with_drops(repo, params, stop) -> Result<Vec<TodoLine>, Refusal-ish>` in
  `rewrite`, called by both.

### m3. `o` on an unborn HEAD now says HEAD moved, which is false

- **The words:** `update/preview.rs:1026` now uses `Refusal::BranchChanged.explain()`, which is "HEAD moved to another
  branch since it was read — look again". The test pins that sentence for `HeadState::Unborn`
  (`context_menu/tests.rs:1184-1191`).
- **Why they are false:** graft-ops `operation/history.rs:523-526` maps `Unborn` to `BranchChanged`. But HEAD did not
  move, and "look again" can never help.
- **The trade:** the old words ("o needs a commit on HEAD to take it onto") were true. 30l traded a true wording for
  one wording, and the ux-principles "recovery from errors" check is lost.
- **Fix (graft-ops):** `head_is` refuses `Unborn` with its own refusal (e.g. `Refusal::HeadUnborn` → "o needs a commit
  on HEAD to take it onto"), and both `check_pick` and `take_operation` say it. That keeps one wording and makes it
  true. Rare path (an orphan branch).

### m4. The editor re-says the preview's AFTER words in a second place

- **The copies:** 08-b NOTES admits `view/rewrite_editor.rs` has its own copies of preview words. The duplicated
  strings are:
  - `built on yours · follows` (:285)
  - `on origin: … · P will need force after` (:387)
  - `no conflicts expected` (:392)
  - `conflicts: not checked (…)` (:396)
- **Also re-implemented:**
  - `short_ref` (:418);
  - `short_name` (`jobs/rewrite_open.rs:189`);
  - the refs-pending string (`update/rewrite_editor.rs:435`), which ignores `remote/push_box.rs:22`'s `REFS_PENDING`.
- **Why it matters:** H4 is one wording, and two sources drift.
- **Fix:** make the preview's word helpers `pub(crate)` (or move them to a shared `words` module in 30m) and call them.
  Use `history::short_ref` and `REFS_PENDING`.

### m5. A placeholder `After` that claims a real state

`update/rewrite_editor.rs:282`: `RewriteEditor::new` starts with `after: After::Invalid(TodoProblem::EverythingDropped)`.
Only because `opened` calls `after::replan` straight after (:533) does the editor not claim "every commit dropped".
Fix: have `new` take the first `After`, or add an explicit `After::NotAsked`.

### m6. `space stage` pressed with nothing to stage says nothing

- **Where:** `update/keys/edit_stop.rs:36-38` swallows `stage_all`'s `false` by design, the same as `Ctrl+A` (07d
  NOTES).
- **Why it matters:** the bar now offers `space stage` on the edit stop's two rows, and pressing it with nothing
  stageable, or with a write running, gives no sign (ux-principles: visibility of system status).
- **Fix:** have `stage_all` return why, and set a status line for both callers.

### m7. ARCHITECTURE.md has no entry for the rewrite editor

- **Evidence:** `grep -i rewrite ARCHITECTURE.md` gives only :134 (the put-in job).
- **The parts it doesn't name:** a new component across three crates:
  - graft-ui `rewrite_editor.rs`;
  - graft `update/rewrite_editor/`, `view/rewrite_editor.rs`, `jobs/rewrite_open.rs`;
  - graft-ops `RewriteRequest::FromRebase`.
- **Also missing:** the edit-stop `space` (`update/keys/edit_stop.rs`).
- **Fix:** add a "Changing the rewrite editor →" entry and an edit-stop entry, as the picker and `o` have (:145-151).

### m8. 24-0: the CONTEXT.md check was skipped, and the term is not there

- **What the ticket said:** ticket 24 :406-407 says "CONTEXT.md already has 'set-aside copy'; check".
- **What is there:** `grep -i "set.aside" CONTEXT.md` finds nothing, and NOTES 24-0 is silent on it.
- **Why it matters:** the user-facing journal words now say "drop the set-aside copy (stash@{n})"
  (`graft-ops/src/operation.rs:1443-1444`), with no glossary term behind them.
- **Fix:** add "set-aside copy" to CONTEXT.md.

### m9. Some "red first" boxes were not seen red

- **24-0:** the two (red) tests were "red by construction": they didn't compile without `why` (NOTES 24-0). A compile
  error is not a red of the words.
- **rebase-preview-keeps-drops:** 7 graft tests were written with the code and mutation-checked instead (said).
- **07d:** box 4 and the Minor 7 test were written after the change (said).
- **Risk:** low. Each asserts exact words or commands.
- **Fix:** none owed. Noted because the ticks say "(red)".

### m10. No gap row for the unedited-rebase-editor behaviour

rebase-preview-keeps-drops NOTES "Behaviour to know" says that on a squash-merged HEAD branch the editor opens with
every row dropped, so `y` is refused until `p`. KNOWN_GAPS has no row for it. Question 1 asks whether that is right.
Until someone answers, it belongs in KNOWN_GAPS as **unverified (question)**.

### m11. 08-a's Question 3 was never carried to KNOWN_GAPS

- **Where:** ticket 08 NOTES (08-a) :388-389.
- **The question:** the bar hides `R` on default-chain rows, but `R` there still sends the open and `plan` decides,
  while R1 says `R` deeper opens more rows. It is open, and it is not in `M7/08-words` or elsewhere.
- **Fix:** add a row.

## Questions (rule 7: lines given; what I couldn't settle is said)

### Q1. The editor opened from a rebase refuses its own unedited todo whenever every row is a suggested drop

- **Where:**
  - `graft-ops/src/rewrite/todo.rs:87` (`validate` → `EverythingDropped`);
  - `update/rewrite_editor.rs:300-301` (`params()` runs `validate`);
  - the capture `docs/shots/m7/rebase-keep-drops-editor-120x40.txt`, which shows `every commit dropped · the branch
    would be its base again` and no `y rewrite`.
- **When:** every time HEAD's branch is the squash-merged one, which is the case the suggestion exists for.
- **Why it's a question:**
  - Decided 3 says "the editor opened unedited must run the same todo the preview's `y` would". The preview's `y` runs
    that todo (the branch becomes `onto`); the editor refuses it.
  - It is harmless, because he pressed `R` to keep something and `p` clears it. But the first thing he reads on
    arrival is worded as a refusal.
- **Options:**
  - exempt `FromRebase` from `EverythingDropped`;
  - keep the refusal but word it for this entry (`nothing kept yet · p keeps a row`);
  - accept as is (and add the gap row, m10).

### Q2. `R` in a preview while an open read is still out closes the preview and does nothing

- **Where:** `update/preview.rs:435` closes the preview. Then `update/rewrite_editor.rs:459-461` returns silently when
  `rewrite.opening.is_some()`, with no status. The earlier read then opens the editor on the *graph row*, not on the
  rebase.
- **What I couldn't confirm:** whether this is reachable in practice. It is a race on how long the read takes.
  - `grep -rn "rewrite\.opening" crates/graft/src` finds no reader outside `update/rewrite_editor*`, so nothing stops
    `r` from opening a preview while the `R` read is out.
  - The same race has a second, also unconfirmed, effect: when the read answers, `opened` (`rewrite_editor.rs:527-534`)
    sets the editor while that preview is still up. I did not trace which modal then owns the keys and the frame.
  - The fix for both: `open_rewrite`/`open_rewrite_from_rebase` and the history keys each refuse or cancel the other
    (`cancel_opening`), with words.

### Q3. `r` and `R` sit side by side in the footer with opposite outcomes

- **The footer:** `y confirm  r rebase  R keep  ←→ choose  ⏎ act  esc cancel`
  (`graft-ui/src/preview_box.rs:74-83`).
- **The risk:** missing Shift on `R` confirms the rebase that drops the commits he meant to keep. `u` recovers it.
- **Where it stands:** the keymap decided this (§2.3b), so it is not a defect. It is for the usability walk
  (ux-principles: error prevention before keystroke economy on a destructive path).

---

## Spec defects noted in the tickets (for the planner)

- **rebase-preview-keeps-drops:**
  - Its Files list missed `update/rewrite_editor/words.rs` and `view/rewrite_editor.rs`. It guessed graft-ui for the
    `marked by its name` words. The builder was blocked mid-build and the conductor granted them.
  - It also missed the captions TSV and the contact sheet (M1).
  - `plan.rs` had no tests module and TESTING.md had no 05 row, though the ticket assumed both.
- **08-b:** needed forced arms outside Files (graft-ui `hints.rs`, `catalogue.rs`, `hints/tests.rs`,
  `graph_screen/tests.rs`; graft `bound.rs`, `view/mod.rs`, `rewrite_editor/keys.rs`). All literal, and all said.
- **08:**
  - The 187 capture's Right line has the message order backwards against the engine's rule (`rewrite/todo.rs:44-67`;
    gap `M7/08-order`).
  - Ruling 6's `Moved` words match no existing wording (gap `M7/08-words`).
- **07d:**
  - Its Touches said `edit_stop_lands`' three picks were one branch. They were three (builder fixed the test).
  - Its Files list missed four graft-ui hint files (the scout caught them).
- **24:** the Touches line asserted a CONTEXT.md term that isn't there (m8).

## Touches lines

| Group | Touches line | Checked by me |
|---|---|---|
| 08-a | present | grep `Char('R')` outside tests → only `keys/predicates.rs:115` (confirmed) |
| 08-b | present | `grep -rl "R keep" docs/shots` → one file |
| 07d | present | |
| 30l | present | |
| 24-0 | present | the files list and the scout's reason, but the CONTEXT.md check is missing (m8) |
| rebase-preview-keeps-drops | present (NOTES) | the sheet was missed (M1) |

## Swallowed errors

None found in the diff's non-test code.

- The `other =>` arm in `update/mod.rs` `rewrite_answer` has a `debug_assert` and ends the job in words.
- `files_listed` and `message_returned` put `Err` on screen (`Loading::Failed`, the notice).
- `opened` puts `Err` on `Status::Failed`.
- `progress::followers` returns `Err` on an unparsable tip or a missing author (no partial followers).
- `edit_stop::space_key`'s ignored `stage_all` bool is commented (m6 is about the silence, not a swallowed error).
- The `let _removed`/`let _stale` in `plan.rs` tests are commented.

No `unwrap`/`expect` outside tests. Sanitisation: every drawn editor string goes through `sanitize_line`
(`view/rewrite_editor.rs:81-105`, `:143-144`, `:170-173`), commit subjects and `$EDITOR` text included.
