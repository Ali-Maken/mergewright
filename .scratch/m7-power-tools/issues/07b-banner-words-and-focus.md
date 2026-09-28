# 07b — part of 07 rewrite-stops (split by planner, filed by conductor 2026-09-27)

Status: done. Shared context (settled scout findings, order, Decided, Signatures) is `07-rewrite-stops.md` lines 1-118; read it first. This file holds only part 07b plus Touches and Rules.

## 07b — banner words, the ladder, focus, `a` at an edit stop, `Esc` (graft + graft-ui)
Build with: **Opus**. An amend written inside a live rewrite through `apply` must leave the rewrite's live `Pending`
entry the one `resume_target` finds; wrong here is structural and not visible in a diff.
Needs: 07a (`EditStop`, `edit_stop`, the check); `rewrite::progress` (02); 09's `RowTag`/`RowTags` (the
`HEAD · stopped here` tag rides on them: add `RowTag::StoppedHere`); `land_on_head` (03-a).
Blocked by: 07a, 09, 03b.
Files:
- graft-ui: `src/graph_screen/banner.rs` (**new**, move-only commit first from `graph_screen/mod.rs:205-253`, then
  the ladder); `src/graph_screen/mod.rs` (the move: removal + `mod`/`pub use` lines only); the file declaring 09's
  `RowTag` (one variant + its draw words); every `BannerView {` literal in graft-ui, literal-only; their tests.
- graft src: `update/banner.rs` (`banner_view` :184, `banner_focusable` :229 + `stop_takes_focus`, `a`/Esc arms
  untouched); `update/banner/stop.rs` (`Stop`, `RewriteStop`, `stopped_again` :66); `update/operate.rs` (:1043 only);
  `update/relaunch.rs` (:80 and the relaunch read); `update/events.rs` + `jobs/runner.rs` (the stop-producing
  products — apply, resume, relaunch read — carry `RewriteStop`, read on the job thread by `progress` + `edit_stop`;
  name them in NOTES); `update/keys/commit.rs` (`commit_key` at an edit stop); `update/commit_editor.rs` (the P14 arm,
  `:390-393` per 09, gains the edit-stop exception read from `Stop.rewrite`; `operation_refusal` :133 untouched);
  `view/rows.rs` (bar label `a amend it` at an edit stop); every `Stop {` / `BannerView {` literal, literal-only.
- graft tests: `update/banner/tests.rs`; `update/banner/rewrite_stop_tests.rs` (**new**).
- Docs: MAP.md (graft, graft-ui).

Acceptance:
- [x] `a_chain_stop_names_the_branch_in_the_banner` (**not** red first: red never observed) — `graft/src/update/banner/rewrite_stop_tests.rs::a_chain_stop_names_the_branch_in_the_banner`; real repo `tests/it/rewrite_stop.rs::a_relaunch_mid_rewrite_shows_the_chain`. Red first **not observed**: written after compaction, code already in (NOTES).
- [x] `rewrite_stop_tests::a_single_branch_stop_has_no_chain_words`; `tests/it/rewrite_stop.rs::a_shell_begun_rebase_has_no_chain_words` (real `git rebase -i --update-refs` edit stop).
- [x] graft-ui `graph_screen/tests.rs::banner_line_keeps_step_i_of_n_at_80_columns` (red first, observed) and `graph_screen/banner/tests.rs::banner_ladder_drops_hint_subject_chain_sha_in_order`;
- [x] `graph_screen/banner/tests.rs::non_rewrite_banners_are_unchanged` (merge, cherry-pick, revert, pull, stash-clash);
- [x] `rewrite_stop_tests::an_edit_stop_gives_the_keyboard_to_the_graph` — three sites (`operate::operated`, `resumed`→`stopped_again`, relaunch working-tree read). Mutation-checked: `stop_takes_focus` reverted to `banner_focusable` turns it red. Plus `an_edit_stop_lands_the_cursor_on_head`.
- [x] `rewrite_stop_tests::a_conflict_stop_still_focuses_as_today`; `rewrite_stop_tests::tab_focuses_the_banner_at_an_edit_stop`;
- [x] `rewrite_stop_tests::a_at_an_edit_stop_amends_the_stopped_commit`; `rewrite_stop_tests::a_twice_at_an_edit_stop_amends_twice`; real repo `tests/it/rewrite_stop.rs::a_twice_then_c_amends_the_stopped_commit_and_the_rewrite_goes_home` (message kept, no editor, HEAD's parent unchanged).
- [x] `rewrite_stop_tests::c_after_an_amend_at_an_edit_stop_continues_the_rewrite` (app); graft-ops `tests/it/edit_stop.rs::an_amend_inside_graft_s_rewrite_folds_and_c_still_continues_it` (the entry `resume_target` finds); real repo `a_twice_then_c_…` (rewrite goes home, feature-c built on the amend).
- [x] `rewrite_stop_tests::a_at_a_conflict_stop_is_still_refused` (app words); ops check is 07a's `edit_stop_is_none_at_a_conflict_stop`.
- [x] `tests/it/rewrite_stop.rs::esc_never_aborts_a_rewrite_stop` — green on arrival; `Esc` on the graph, the banner (edit stop), in the resolver (conflicted Graft rewrite); after each: no `Abort` command, `.git/rebase-merge/` present, `live_entries` non-empty.
- [x] `rewrite_stop_tests::a_failed_progress_read_lands_with_todays_banner_and_says_so`;
- [x] `tests/it/rewrite_stop.rs::a_relaunch_mid_rewrite_shows_the_chain` — Graft's rewrite applied in-process and dropped at the stop (the disk state a kill leaves), then a fresh `App::open` reads the saved plan. Not a literal `kill -9`.
- [x] hint-bar tests green (`update::tests::every_key_the_bar_draws_is_answered`, `every_planned_hint_is_still_unbound`, graft-ui coverage slots 90–95); `stop_wordings` tests unedited and green in the workspace run.
- [x] Conductor notes: Major 1 `graft-ops tests/it/edit_stop.rs::a_shell_begun_edit_stop_amend_is_recorded_with_a_backup`; Minor 1 `amend_into/tests.rs::staging_not_kept_after_a_put_in_tags_and_spends_its_words`, `incomplete_after_a_put_in_forgets_its_words`; Minor 2 `amend_into/tests.rs::a_write_claimed_while_the_tags_are_read_drops_their_answer` (all red first, observed).
Size: ~55k.


## Touches (each part checks its own, reports how)
- **M6 banner and resolver** for merge, pull, stash clash: words and focus unchanged. Check: every m6 banner/resolver
  test and capture byte-identical (`check_shot`), or each change listed in NOTES. (07b, 07c)
- **Footer `stop_wordings`:** unchanged. Check: its tests. (07b)
- **`u`/`U` refusal while live:** unchanged; the amend at an edit stop is refused for `u` like any live step. Check:
  its test. (07b)
- **P14 for merge / cherry-pick / revert and conflict stops:** unchanged. Check: existing amend-refusal tests. (07a, 07b)
- **`resume_target` / `live_entries`:** an amend entry landing inside a live rewrite must not hide the rewrite's
  `Pending` entry. Check: `c_after_an_amend_at_an_edit_stop_continues_the_rewrite`. (07b)
- **`pane_c_acts`** (one place; `commit_key` and `view::rows::live_c` read it): now true for shell-begun rebase and
  cherry-pick. Check: `live_c_tests.rs`, `shell_begun.rs`. (07c)
- **Relaunch after kill -9:** reads the saved plan through `progress`. Check: `a_relaunch_mid_rewrite_shows_the_chain`. (07b)
- **Keymap:** §2.6a already amended; no doc edit here. Check: hint-bar tests.
- **Journal/undo:** the edit-stop amend is an ordinary `Amend` entry; the shell hand-over writes none (git's
  operation, begun outside, as M6). Nothing else new.
- **KNOWN_GAPS :545:** closed by the conductor (settled 1), not by a builder.

## Rules / Gates
As rewrite-engine. Workspace-wide gates. Each part commits at its seam.

## Conductor note (2026-09-27, from 07a)
07a probe: an Amend at an edit stop folds into the rewrite span (`graft-ops/src/apply/fold.rs:40`) and gets no journal entry of its own; `fold.rs:63` doc is stale. The Touches line here expects an ordinary Amend entry. **07b decides** (verify first, rule 7): keep the fold (then the amend undoes with the rewrite; say so in the banner/words) or journal it apart. Say which in NOTES with the line.

## Conductor note (2026-09-28, review m7-09-to-21c Major 1)

An edit-stop amend in a rebase begun in the shell is admitted (`graft-ops/src/operation.rs:1537`) but unrecorded: `apply/fold.rs:55` finds no span, so there is no backup and no journal entry, and `u` can't take it back. 07a's Touches line missed it. This is the same fold path as the earlier note; 07b fixes it: record it, or refuse it with words. Verify first; red test first.

## Conductor note (2026-09-28, review m7-09-to-21c Minors 1, 2 — you hold `operate.rs`)

- Minor 1: `operate.rs:1081-1088` `StagingNotKept` calls only `landed(...)`; call `amend_into::landed` too (the one that `take`s `asked`, `amend_into.rs:292-295`) and forget `asked` on `Incomplete`. Red test first.
- Minor 2: `claim_write` (`operate.rs:202`) clears `row_tags` but not `amend_into.tagging`, so `tags_resolved` (`amend_into.rs:342-350`) can set stale tags. Clear it in `claim_write`. Red test first.
Both fixes are in `operate.rs`. Do not edit `amend_into.rs` (30k holds it); if you must, report `blocked: needs amend_into.rs`.

## NOTES (builder, 2026-09-28)

**Compacted once mid-ticket.** Nothing left from the list. Scouts: yes, two.

**Fold (note a):** kept for a rewrite Graft began — the amend undoes with the rewrite
(`graft-ops/src/apply/fold.rs:40` `folded_step`; the folded step now re-reads refs, `fold.rs:126`). Verified by
`an_amend_inside_graft_s_rewrite_folds_and_c_still_continues_it` (two amends, journal holds only the rewrite entry,
`resume` finds it). **Question:** the spec status `✓ amended the stopped commit · c continues` does not say it undoes
with the rewrite; Touches' "ordinary `Amend` entry" now holds only for a shell-begun rebase.

**Major 1 (note b), cause:** `fold.rs:60` — a spanless step touching worktree or refs no longer folds; ordinary
`apply` path (own entry, backup, `u` works). Red first: `a_shell_begun_edit_stop_amend_is_recorded_with_a_backup`.

**Minor 1, cause:** `operate.rs` `StagingNotKept` calls `amend_into::landed` (its line then yields to the staging
line); `Incomplete` forgets a put-in's words. Red first, in `update/amend_into/tests.rs` (appended — may conflict
with 30k at merge): `staging_not_kept_after_a_put_in_tags_and_spends_its_words`,
`incomplete_after_a_put_in_forgets_its_words`.

**Minor 2, cause** (after 30k released `amend_into.rs`, main merged at 73191e72, both sides' tests kept):
`amend_into::write_claimed` clears `tagging`; `operate::claim_write` calls it. Red first, observed:
`amend_into/tests.rs::a_write_claimed_while_the_tags_are_read_drops_their_answer`.

**Fold status words (question, open):** the edit-stop amend inside Graft's rewrite **undoes with the rewrite** (one
`u` takes back the rewrite and every amend in it), but `✓ amended the stopped commit · c continues` does not say so.
Should the words say it (e.g. `· undoes with the rewrite`)? Not changed here; spec wording is the conductor's call.

**Products carrying `RewriteStop`:** `Operated::Conflicted.rewrite` (apply + resume, read in `jobs/operate.rs`
`in_context`); `working_tree::WorkingTree.rewrite` (every working-tree read while a rebase is live; the relaunch's).
`events.rs`/`runner.rs` unchanged.

**Deviations:** `HEAD · stopped here` derived in `view/entries.rs` from the stop, not `RowTags` (`claim_write`
clears those; the tag must survive the amend). Cursor on HEAD: `edit_stop_lands_on_head` (first stop, again),
`relaunch_lands_on_head` (replaces `land_on_the_work` at an edit stop: no changes pane). `head_message_arrived` takes
`commands`, `next_id`. `keys/bound.rs` gains `("a","amend it")`. Shell-begun edit stop: `c` refuses until 07c, so no
`a amend it` in its bar (key works).

**Captures:** new `docs/shots/m7/rewrite-edit-stop-120x40.txt` (user judges; colour unverified). Changed m6:
`fix-tab-words-80x24.txt`, `fix-tab-words-120x40.txt`, `fix-file-list-rebase-step2-120x40.txt` (`REBASING step 1 of
1` → `REBASING  step 1 of 1 · pick 9d44f42 Change every file on feature`); m6 contact sheets 1, 4 still old text.
For 07c (textual): new capture's bar reads `⇥ banner  a amend it  ⏎ open  . more  b  r  t  c …` — `c` bare, not
`c continue`.
