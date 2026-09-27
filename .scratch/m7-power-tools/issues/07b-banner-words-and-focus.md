# 07b — part of 07 rewrite-stops (split by planner, filed by conductor 2026-09-27)

Status: ready. Shared context (settled scout findings, order, Decided, Signatures) is `07-rewrite-stops.md` lines 1-118; read it first. This file holds only part 07b plus Touches and Rules.

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
- [ ] `a_chain_stop_names_the_branch_in_the_banner` (red first);
- [ ] `a_single_branch_stop_has_no_chain_words`; `a_shell_begun_rebase_has_no_chain_words`;
- [ ] graft-ui `banner_line_keeps_step_i_of_n_at_80_columns` (red first) and `banner_ladder_drops_hint_subject_chain_sha_in_order`;
- [ ] `non_rewrite_banners_are_unchanged` (merge, cherry-pick, revert, pull, stash-clash: today's strings);
- [ ] `an_edit_stop_gives_the_keyboard_to_the_graph` (red first: today `operate.rs:1043` focuses the banner) — at all
  three sites: first stop, `stopped_again`, relaunch;
- [ ] `a_conflict_stop_still_focuses_as_today`; `tab_focuses_the_banner_at_an_edit_stop`;
- [ ] `a_at_an_edit_stop_amends_the_stopped_commit` (message kept, no editor); `a_twice_at_an_edit_stop_amends_twice`;
- [ ] `c_after_an_amend_at_an_edit_stop_continues_the_rewrite` (the live entry still found);
- [ ] `a_at_a_conflict_stop_is_still_refused` (app words and ops check);
- [ ] `esc_never_aborts_a_rewrite_stop` — **regression, green on arrival** (settled 1): `Esc` on the banner, in the
  resolver, on the graph; `rebase-merge/` and the live entry still there after each;
- [ ] `a_failed_progress_read_lands_with_todays_banner_and_says_so`;
- [ ] `a_relaunch_mid_rewrite_shows_the_chain` (kill -9 mid-rewrite, relaunch reads the saved plan);
- [ ] hint-bar tests green; `stop_wordings` tests unchanged and green.
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
