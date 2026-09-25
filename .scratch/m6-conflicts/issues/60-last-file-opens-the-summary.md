# last-file-opens-the-summary: finishing a merge's last file opens the summary; otherwise a finish row says `c`

Status: done
From: P11 (approved 2026-09-24, picks item 2; supersedes batch 11); field notes :60, :63 (in the demo the summary
opened by itself and he went on to `y`); KNOWN_GAPS M6/31 :755 (`c finish` shown only after the next key).
Build with: builder-opus (the conductor's call after scout 60, `.scratch/m6-conflicts/scouts/60.md`).
Scout: `.scratch/m6-conflicts/scouts/60.md` (main @ 8be588d5). Line refs below marked `(scout)` come from it.
Needs: `finish-summary-is-a-preview` merged (the summary's `y`; the counts below assume it).
Blocked by: 59, 64. **64 lands first; 60 rebases on 64's result:** 64 marks `d start over` keeps_words in
`resolver_done_list` (`hints/lists.rs:346`, scout) and reorders `fitting.rs` `ladder` (:53); 60 moves `c` to index 0 in
the same lists. Take 64's `lists.rs`, `fitting.rs` and bar captures as the base; don't redo its reorder.
Crates: graft, graft-ui.
Lane: wave 2, second.

## Read first
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24 (:857): J13 (:903-910), J6/J6b/J14 (:911-915), "Where each
  event lands him" (:937-955), "Card and file list" last bullet (:1042-1045), "The finish row" (:1047-1068),
  hard-to-reverse #2 (:1124-1126).
- `docs/design/02-keymap.md` §2.6 the `c` and `Enter` rows (edited 2026-09-24).
- `crates/graft/MAP.md` §update::resolver (:315-346), §update::banner `continue_pressed` (:301), §update::finish
  `open_finish` (:359).
- `crates/graft-ui/MAP.md` lines 12-13 (`DocView.notice`, the file list), 22 (`Resolver { finishable }`).
- Scout 60, whole (one page).

## First thing: find where the post-`Stage` status read is asked (stop if the assumption fails)
The scout could not confirm where the `ReadWorkingTree` after a `Stage` is asked. This spec **assumes** it is asked no
later than the moment the Stage answer's handling returns from `stay_after_last` — i.e. `status_read::owed(state)` is
true when `finish_owed` is set. Candidates, unconfirmed: `reread_set` (`resolved.rs:168`), `operate::operated`.
- Find it; name `file:line` in NOTES.
- Pin it with the first test written: `the_read_after_the_last_stage_is_owed_when_it_lands` (asserts `owed` is true
  right after the last Stage answer is handled).
- **If it doesn't hold** (the read is asked later, or not at all): stop. Report `blocked: post-Stage read asked at
  <file:line>, after the Stage answer` and don't build the trigger — a spend on a read that predates the Stage would
  open the summary on stale status, or cancel it on a stale conflict.

## Scope
**In — graft:**
- **The trigger is the event, never the state.** `stay_after_last` (`resolved.rs:159`, scout; the last file done by
  `⏎`, `O`/`T`, or `o`/`t` on a whole-file card), for a **merge** (begun in Graft or in the shell), sets
  `ResolverState.finish_owed`.
  - **Where it is set:** on the new `ResolverState`, **after** `reopen(done)` (`resolved.rs:166`) returns. `reopen`
    rebuilds the state (`open.rs:89-109`: `take()` + `start`, copies only `files`/`set_job`/`seen`), so a flag set
    before it is lost. Don't copy `finish_owed` across `reopen`: its other callers are file switches
    (`files.rs:192`, `restart.rs:162`, `resolved.rs:131`), and dropping it there is the file-switch clear.
  - **`reopen` refuses** (`open_refusal`, `open.rs:96`; the old resolver kept): `finish_owed` is not set. He sees the
    refusal's notice, unchanged; `c` is the way on.
- **The spend.** `finish_if_owed` spends it at the first moment **all** hold: `finish_owed`; no take `settling`,
  `in_flight` or `queued`; no write out; the resolver's own opening read (`job`/`reading`, started by `reopen`) has
  landed; `!status_read::owed(state)`; the latest status read shows nothing unmerged. Spending clears the flag, then
  calls `banner::continue_pressed(state, commands, ReturnTo::Resolver, next_id)` — the one `c` path, same refusals.
  - **Where it is called:** once, at the tail of the answer path in `update/mod.rs` (every answer, not per kind; the
    builder names the line). The predicate is state-only, so the spend does not depend on which of the Stage answer,
    a take settling or the status read lands last.
- **Cleared, never spent,** by: `Esc`, a file switch (`j`/`k`, a click, `n`/`N` crossing), `d`, `e`, `M`, a failed or
  refused write (`write_not_landed`, `mod.rs:508` / `answers.rs:219`), a status read that still shows a conflict, a
  failed opening read, the resolver closing (every `state.resolver = None`: `banner.rs:456`, `finish/answers.rs:137`,
  `resolver/answers.rs:158`).
- **A failed status read cancels it** (rule 3). Every `status_read::abandon` (`mod.rs:658` the `Err` landing; `:929`,
  `:970`) clears `finish_owed`. On the `Err` path he sees: the resolver; the read's failure words on the notice row, as
  `abandon` shows them today; no summary; **no finish row while that warning stands** (a warning notice outranks the
  finish row — `DocView.finish` is `None` then); the bar still starts `c finish`. The builder names what `:929` and
  `:970` are in NOTES. **If the `Err` path shows no failure words today** (a `CHECKING` notice left standing, or
  nothing): stop and report it — that's a pre-existing swallow and fixing it is a scope change.
- **No open:** a rebase, cherry-pick or revert (the finish row instead; `c` writes); a stash pop (closes to the graph,
  unchanged); after `u` (not bound in the resolver, `resolver.rs:367` scout — the case arises from the banner); on
  relaunch; the last file done outside the resolver (`space` on a `CONFLICTS` row → the banner, unchanged).
- **Status after the last file:** summary opening → `<file> done ✓ · every file done`; no summary (rebase, …) →
  `<file> done ✓ · every file done · c continues the rebase` (the kind's words, `banner::continue_words`,
  `banner.rs:233`). Today these words go on the notice (`resolved.rs:173-178`); they move to the status line.
- **The finish row** (the resolver, every file done, the summary not open, no warning notice: after `Esc` from it, a
  rebase, cherry-pick or revert): replaces `every file done ✓ · c finishes the merge` on the notice row. A click on it
  does what `c` does (`finish_pressed`, `leave.rs:20`).
- **Summary `Esc`** (from `ReturnTo::Resolver`, `finish/keys.rs:286-297`) → the resolver with the finish row; the draft
  kept.

**In — graft-ui:**
- **Finish row:** full `✓ every file done — c review and finish the merge` (merge) / `✓ every file done — c continue
  the <kind>`; when the row is narrower than the full text: `✓ every file done — c review & finish` /
  `✓ every file done — c continue`; then cut with `…`. Style: `accent` BOLD on a full-width `selection` fill; `NO_COLOR`
  REVERSED. Width-driven, not size-tier-driven. Drawn by `draw_page_header` (`conflict_resolver.rs:352-383`) on the
  notice row instead of `notice`.
- **All-done head:** column `FILES · all <n> done ✓` (`files/column.rs:43-48`), strip label `files · all done ✓`
  (`files/strip.rs:26-46`, single-row and grid alike).
- **Bar:** `Resolver { finishable: true }` puts `c finish` (merge) / `c continue` **first** (today `offering_c` inserts
  it before `esc close`, `lists.rs:82-120`). With 64's done list, the floor-3 (`fit_list` :96) becomes `c, d, e`.
- **Hit test** for the finish row (none exists for the notice row today).

**Out:**
- The summary itself (previous spec). `c` from the pane (ticket 52).
- The banner's `every file done ✓ · c finishes the merge` (unchanged).
- New failure words for a status read (see the stop above).

## The failing test first
After the pin test above: graft `tests/it/resolver/`: `the_last_file_of_a_merge_opens_the_summary`.
- Setup: two-files merge begun in the app, `gone.txt` done, `notes.txt` open with one block left. Press `o`, then `⏎`;
  drive the stage and the status read to landing.
- Assert: `finish.is_some()`, `return_to == ReturnTo::Resolver`, focus `Files`, cursor on `README.md` (first row git
  combined); status `notes.txt done ✓ · every file done`.
- Red today: the resolver stays, notice `every file done ✓ · c finishes the merge`.

## Signatures
```rust
// graft/src/update/resolver.rs (ResolverState) and child `resolved`
pub finish_owed: bool,                                                   // new field; set after `reopen` in stay_after_last
pub(crate) fn finish_if_owed(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId);
//   same argument shapes as `banner::continue_pressed` (banner.rs:418, `&mut impl FnMut() -> JobId`);
//   called once at the tail of the answer path in update/mod.rs
// graft-ui/src/conflict_resolver.rs
pub struct FinishRow { pub full: SanitizedText, pub short: SanitizedText }
pub struct DocView { /* existing …, */ pub finish: Option<FinishRow> }   // Some → drawn on the notice row instead of `notice`
#[must_use] pub fn finish_row_at(doc: &DocView, placement: Placement, area: Rect, col: u16, row: u16) -> bool;
```
`DocView` is built by struct literal in **three** places in `graft`; set the new field in all three, this ticket.

## Touches (each entry, and how the builder checks it)
- **`DocView` literals:** `view/resolver.rs:67`, `view/resolver/whole_file.rs:23`, `view/resolver/whole_file.rs:67`
  (scout). Check: the build; `finish_row_on_a_whole_file_card` (the finish row also shows when the last file done was
  a whole-file choice).
- **Post-Stage read.** Check: `the_read_after_the_last_stage_is_owed_when_it_lands` (first; stop rule above).
- **`reopen` drops state.** Check: `the_owed_open_survives_the_reopen` (the flag is on the rebuilt state);
  `a_refused_reopen_opens_nothing`.
- **Takes still landing cannot open it early.** Check: `a_take_still_landing_holds_the_summary` (`T` then `⏎` typed
  faster than the disk: opens once, after both land), `a_failed_last_stage_opens_nothing`.
- **Clears.** Check: `esc_before_the_read_lands_cancels_the_open`, `switching_file_cancels_the_open`,
  `a_failed_status_read_cancels_the_open_and_says_so` (the `Err` path: notice carries the failure, finish row absent,
  bar starts `c finish`).
- **No open where it must not.** Check: `the_last_file_of_a_rebase_shows_the_finish_row`,
  `a_stash_pop_still_closes_to_the_graph`, `undo_after_finish_opens_no_summary` (ticket 41's landing: banner focused),
  `relaunch_on_a_resolved_merge_opens_no_summary`, `space_on_the_last_conflicts_row_lands_on_the_banner`.
- **Shell-begun merge:** opens too (J6b). Check: `the_last_file_of_a_shell_merge_opens_the_summary`.
- **Finish row.** Check: `finish_row_words_by_width` (187, 120, 80: full / full / short), `finish_row_style`
  (buffer: `accent` BOLD on `selection`; `NO_COLOR` REVERSED), `click_on_the_finish_row_is_c`,
  `a_warning_notice_outranks_the_finish_row`.
- **All-done head.** Check: `list_head_when_every_file_is_done` (column and strip).
- **Hints** (on 64's result). `c` first when `finishable`. Check: `every_context` coverage,
  `every_key_the_bar_draws_is_answered`, 64's ladder test still green; `catalogue.rs:142-161` (StashPop skips
  finishable) still covered.
- **Tests that assert "the resolver stays" — rewrite to the new landing, don't delete:**
  `src/update/resolver/tests/finish_here.rs:62 the_resolver_stays_after_the_last_file` (:88); `tests.rs:458`;
  `tests/file_list.rs:436, :448`; `tests/words_and_c.rs:207, :219`; `tests/it/resolver/last_file.rs:19
  last_file_with_a_banner` (:101, :127), `:203 the_last_file_done_draws_one_tick_on_the_footer` (:225, :231);
  `tests/it/m5_walk/conflict_journeys.rs:266 every_file_done()` (:269, :273), `a_finish` :673, `b_finish` :732;
  `m5_walk/resolving.rs:258`. Each rewritten test names what it now asserts in NOTES.
- **Journal and undo:** nothing new (the summary writes nothing). Check: the finish journal tests green.
- **KNOWN_GAPS M6/31 :755** — closed by this (the conductor's row; name the test).
- **The walk:** `one-file-one-conflict`, `three-conflicts-all-theirs`, `two-files-in-order`, `two-files-out-of-order`
  now land on the summary (assert its frame, not "the resolver stays"); `a-finish` and `b-finish` start on the open
  summary and send `y`; new scenario `last-block-to-finished` (two-files, `gone.txt` done, the cursor on `notes.txt`'s
  last open block: `o ⏎ y`, 3). Update `.scratch/m6-conflicts/pty-walk-m6.py` keys (`m6-14` no longer needs `c`).
  - Check: `journey_counts_match_the_note` and `every_scenario::journey` with the rows below applied.
- **Captures** (regenerate on top of 64's bar captures): m6-14 (now the summary, opened by itself; 5 sizes), m6-15
  (now the finish row after `Esc`), `fix-finish-from-the-resolver-120x40`, `fix-every-file-done-footer-120x40`,
  `fix-file-list-*` (the head; 10 files). Regenerate what the tests own; the walk's m6-1x captures are the acceptance
  ticket's.
- **Docs:** `crates/graft/MAP.md` §update::resolver (`finish_owed`, where it is set, `finish_if_owed` and its call
  site, the landings, the clears incl. `abandon`); `crates/graft-ui/MAP.md` lines 12-13 (`FinishRow`, `finish_row_at`,
  the head).

## Acceptance (name the verifying test at tick time)
- [x] `the_read_after_the_last_stage_is_owed_when_it_lands` (`src/update/resolver/tests/finish_here.rs`): green — the read is asked in `operate::operated` → `refresh_scope` (`Refresh::WorkingTree`, operate.rs ~988), before `resolved_advance` runs.
- [x] `the_last_file_of_a_merge_opens_the_summary_once_the_reads_land` (unit, `finish_here.rs`, the old `the_resolver_stays_after_the_last_file` rewritten) and `tests/it/resolver/opens_summary.rs::the_last_file_of_a_shell_merge_opens_the_summary` (real git; the fixture is shell-begun) — green. **Red-first not re-verifiable:** the session compacted and the red run is not in what survived. The app-begun merge against real git: walk `last-block-to-finished`, `one-file-one-conflict` (state: `finish.is_some()` over the resolver).
- [x] every test named under Touches — all green in `cargo nextest run` (3184/3184): `finish_row_on_a_whole_file_card`, `the_owed_open_survives_the_reopen`, `a_refused_reopen_opens_nothing`, `a_take_still_landing_holds_the_summary`, `a_failed_last_stage_opens_nothing`, `esc_before_the_read_lands_cancels_the_open`, `switching_file_cancels_the_open`, `a_failed_status_read_cancels_the_open_and_says_so` (see NOTES Q1), `the_last_file_of_a_rebase_shows_the_finish_row` (`opens_itself.rs`); `undo_after_finish_opens_no_summary`, `relaunch_on_a_resolved_merge_opens_no_summary`, `space_on_the_last_conflicts_row_lands_on_the_banner` (see NOTES Q3), `the_last_file_of_a_shell_merge_opens_the_summary` (`tests/it/resolver/opens_summary.rs`); `a_stash_pop_still_closes_to_the_graph` is `last_file_without_one` inside `tests/it/resolver/last_file.rs::last_file_closes_to_where_it_opened` (not a new name); `finish_row_words_by_width` (see NOTES Q2), `finish_row_style`, `finish_row_at_hits_only_the_row` (graft-ui `conflict_resolver/tests/finish_row.rs`), `click_on_the_finish_row_is_c`, `a_warning_notice_outranks_the_finish_row` (`opens_itself.rs`); `list_head_when_every_file_is_done` (graft-ui `files/tests.rs`); hints `a_finishable_done_file_offers_c` (c first), `every_key_the_bar_draws_is_answered`, 64's ladder test green.
- [x] `journey_counts_match_the_note` with the rows below — green; rows lowered by this builder in `docs/design-notes/m6-conflicts.md` (a-finish 1, b-finish 1, a-whole-one-file 4, a-whole-two-files 5, b-whole-two-files 4, `last-block-to-finished` un-commented at 3); `every_scenario::journey_last_block_to_finished` green.
- [x] Capture (rule 6; colour **unverified**, rule 5) — asserted by `the_last_file_of_a_shell_merge_opens_the_summary` (top border, `▸ README.md`, finish row, `files · all done ✓`, bottom row `c finish`); **the status row is not in frame 2** (NOTES Q4); judged by eye by the user, not by me: `docs/shots/m6/fix-last-file-opens-summary-187x50.txt` (two
      frames: before the last `⏎`; after it, the summary) and `fix-finish-row-120x40.txt` (after `Esc` from it).
  - **What right looks like:** frame 2's top border is `╭ Preview: finish merging feature into main`, the status row
    `notes.txt done ✓ · every file done`. The finish row capture: under the identity row
    `✓ every file done — c review and finish the merge`, list head `files · all done ✓`, bottom row starting `c finish`.
- [x] MAP.md updated (doc, no test): `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`.

## For the conductor (exact text, in this spec's merge commit)
In the `journey-counts` fence, set these `keys` (comments unchanged) and un-comment the last row:
```
a-finish                      1
b-finish                      1
a-whole-one-file              4
a-whole-two-files             5
b-whole-two-files             4
last-block-to-finished        3                     # Amendment 2026-09-24 J13: o ⏎ y (P11+P12) · JetBrains ~4 (?) · VS Code 3–4 · lazygit 2 (bare, rejected)
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium-large. One context. The trigger is the one decision point; the rest is words and a row.

## NOTES

**Compacted once mid-ticket** (context). What remained at the compaction — the integration and walk rewrites, the
new integration tests, the captures, the sheet, the counts, MAP.md, the gates — was done after it; nothing is left.
Scout 60 used; its `impl FnMut` (not `dyn`) point was right.

Rewritten tests and what they now assert:
- `finish_here.rs` `the_resolver_stays_after_the_last_file` → `the_last_file_of_a_merge_opens_the_summary_once_the_reads_land`: summary opens once the stage, done read and status read land, `return_to == Resolver`, status `<file> done ✓ · every file done`.
- `tests.rs` `all_resolved_from_the_pane_offers_c_where_c_continues`: summary opens on its own; `Esc` → finish row; `c` → the summary again, never the commit editor.
- `file_list.rs` :436/:448 → `after_the_last_file_of_a_merge_the_status_says_every_file_done`, `after_the_last_file_of_a_rebase_the_status_says_c_continues`.
- `words_and_c.rs` :207/:219: the finish row, not the notice, offers `c` (notice `None` once the read lands).
- `tests/it/resolver/last_file.rs` `last_file_with_a_banner`: `done_read_landed` asserts the summary open over the done file, notice `None`, status `b.txt done ✓ · every file done`; `captured_both_moments` captures the summary and, after `Esc`, the finish row with the bar starting `c finish` (`fix-finish-from-the-resolver-120x40.txt`). `the_last_file_done_draws_one_tick_on_the_footer` unchanged and green: `Esc` before the read lands cancels the open.
- `tests/it/resolver/file_list.rs::a_rebase_lists_only_this_steps_files`: notice `None`, no summary, the finish row `✓ every file done — c continue the rebase` drawn.
- walk `every_file_done()`: waits `FinishReady`, asserts the summary over the resolver and status `… every file done`, frame `RESOLVED BY YOU`. `a_finish` / `b_finish` / whole rows send `y` only (`y_commit`); `resolving.rs:258` and `u-right-after-continue-is-accepted` (new `last_merge_file_resolved`) the same; rebase `last_file_resolved` asserts the finish row and `c continues the` status. `two-files-in-order`: the list's done states read from `listed_files` (the summary covers the list).
- walk captures: m6-14 keys `m m o T` (the summary), m6-15 `m m o T Esc` (the finish row), m6-16 `m m o T space`, m6-17 `m m o T y`. Retaken by `pty-walk-m6.py` at 5 sizes; the script itself needed no change (keys come from `--stems`). Contact sheet rebuilt: 3 parts, 179 356 / 178 213 / 98 219 B.

Where `finish_owed` clears: `status_read::abandon` (the `Err` path mod.rs ~658, `JobCancelled` ~929, `JobFailed` ~970), `write_not_landed`, `done_unreadable`, keys `Esc j k ↓ ↑ n N d e M` (`resolver::cancel_owed_finish`), `reopen`, the resolver closing.

KNOWN_GAPS M6/31 :755 closed by `after_esc_from_the_summary_the_finish_row_and_c_first` (the finish row is drawn when the read lands, not after the next key).

Questions for the conductor (rule 7: each with its line):
- **Q1** `a_failed_status_read_cancels_the_open_and_says_so`: the spec expects the bar to start `c finish`. It does not: the stale pre-Stage status still calls the file unmerged, so `finishable` is false (`view/rows.rs:509`) and `c` would refuse (`banner.rs:202` `continue_refusal`). The failure words go on the status line (`Status::Failed`), not the notice row; the notice is cleared. Pinned as built.
- **Q2** `finish_row_words_by_width`: the spec says short words at 80. The rule is width-driven, and at 80 and 120 the list is a strip, so the notice row is the frame's full width and the full 49 cells fit: full at 187/120/80; short only on a narrower document (asserted at a 46-wide doc).
- **Q3** `space_on_the_last_conflicts_row_lands_on_the_banner`: observed, `Space` on the last `CONFLICTS` row leaves the keyboard in the changes pane, not the banner (`!banner_focus`; the existing `conflict_interim.rs:535` flow needs `tab_back_to_the_banner` too). The test asserts no summary and no resolver, and pins focus as observed; the name is the ticket's.
- **Q4** Capture frame 2's "status row `notes.txt done ✓ · every file done`": the summary takes the whole frame including the bottom row, so the status is set (asserted on `state.status`) but not drawn while the summary is open. Showing it would be a change to the summary's layout (tickets 19/35/59), not made here.
