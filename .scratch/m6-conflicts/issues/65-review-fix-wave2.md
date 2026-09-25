# 65 — review fix for wave 2 (Major M1, M2, M3; Minor m1, m4, m6, m8, m9)

Status: done
Crates: graft-ui, graft (src and tests) + docs
Source: `docs/reviews/m6-look-finish-wave2.md` M1, M2, M3, m1, m4, m6, m8, m9.
Build with: builder-opus
Needs: 63 merged. 63 edits graft tests and captures in the same files.
Scout: none needed. The review names every line; re-check each against main after 63 lands.

## Decisions (conductor, 2026-09-25)
- **M2: the built landing stands.** After `space` on the last `CONFLICTS` row, the pane keeps the keyboard. Since 52, `c y` finishes from the pane, so `⇥` to the banner would be a wasted key. The note is already corrected (`docs/design-notes/m6-conflicts.md`, "The last file done outside the resolver"). Only the test name is left to fix.
- **M3: fix the cause, not the tolerance.** The settle waits for the radar's answer. The check does not learn to accept a missing `ⓘ`.

## Acceptance
- [x] **M3:** `capture_fix_c_live_frames`: `bisect_frame`'s settle now also waits for the radar (`radar_answered`, `shell_begun.rs`): `state.radar.context` is `Loading::Ready` and `state.radar.built_from` is `Some` (the ref findings the `ⓘ` draws from are built), and neither `radar.duplicates` nor any `radar.force_push` is `Pending`. `details_open_frame` (frame 2, which also draws `ⓘ`) waits the same way. Test: `resolver::shell_begun::capture_fix_c_live_frames`.
  - Red first, run alone 5 times before the fix: **2 of 5 failed** (`drawn │ ●    [feature] …`, no `ⓘ`), not 5 of 5 as the review saw; timing-dependent either way.
  - After the fix, run alone 5 times: **5 of 5 pass**.
  - Capture unchanged: `check_shot` matched `docs/shots/m6/fix-c-live-frames-187x50.txt` every run, so no retake.
  - Heavy tier: green: `cargo test --features heavy-tests --no-fail-fast -- --test-threads=2`, 3271 passed, 0 failed.
- [x] **M1:** pure moves, existing tests unedited (the diff removes no line from any test file) and green in `cargo nextest run`. Line counts: `finish_summary.rs` 610 → 255 (`frame.rs` 152, `list.rs` 251); `conflict_resolver.rs` 449 → 396 (`finish_row.rs` 70: `FinishRow`, `finish_row_at` and `draw_finish_row`); `hints/lists.rs` 434 → 273 (`lists/resolver.rs` 171); `update/resolver.rs` 415 → 397 (`cancel_owed_finish` now in `resolved.rs`, 321). Module tests: `finish_summary::tests::preview_title_shortens_then_cuts`, `preview_frame_styles`; `conflict_resolver::tests::finish_row::finish_row_words_by_width`, `finish_row_style`, `finish_row_at_hits_only_the_row`; `graph_screen::hints::tests::a_finishable_done_file_offers_c`, `done_list_marks_start_over`, `resolver_bar_shows_every_key_at_80`; `update::resolver::tests::opens_itself::switching_file_cancels_the_open`, `esc_before_the_read_lands_cancels_the_open`. MAP.md updated for graft-ui and graft.
- [x] **M2:** renamed to `space_on_the_last_conflicts_row_opens_no_summary_and_keeps_the_pane` (`tests/it/resolver/opens_summary.rs`); assertions unchanged, doc comment now cites the decided landing.
- [x] **m1:** `banner::continue_pressed` refuses while `operation_in_flight`: sticky `a write is already running` (banner) or the resolver's notice row, nothing sent, before the resolver would close. Covers pane, both graph frames and the banner's own `c`. Test: `update::keys::live_c_tests::c_while_a_write_runs_refuses_in_words` (seen red: status stayed `Ready`).
- [x] **m4:** note line reworded to `c y` **2** (lowered by 52, as the fence says). Test: `journey_counts_match_the_note` green.
- [x] **m6:** `⌃E $EDITOR` in both (the summary moved to the commit editor's word; `bound.rs` table row too). `esc cancel` vs `esc back` kept apart; why is in the note ("Bar by focus"). Test: `commit_editor::tests::ctrl_e_reads_the_same_in_both_subject_fields` (seen red: `⌃E editor`).
- [x] **m8:** keymap §2.3c now says the hint bar leads with `y <verb>`, the box has no footer of its own. Doc edit.
- [x] **m9:** `finish_summary::footer_spans(&[Hint], strong, muted)` builds the spans from the hints (new `Hint::drawn_verb()`, which `Hint::text` now uses too). Test: `finish_summary::tests::footer_spans_take_the_first_key_from_the_hint` (every focus, plus a key with a space in it). Colour **unverified**: styles compared as values only.
- [x] Captures: none changed (every `check_shot` in the fast tier matched). m6 sheet needs no rebuild; parts 179 678 / 177 038 / 140 530 B. No earlier-milestone capture retaken. No machine path in the diff (grep of the diff for home, temp and user-name paths: 0).

## Touches (check each, report how)
- Every caller of the moved items: the build proves them.
- Captures of the summary footer and both subject editors: `grep -rl` them in `docs/shots`.
- The walk tests that read those footers.
- MAP.md for graft-ui and graft.
- KNOWN_GAPS is the conductor's; don't edit it.

## NOTES
- Scout: none; the review named every line, each re-checked against this tree after 63.
- M3 state field pinned: `state.radar.context` (`Loading::Ready`) with `state.radar.built_from.is_some()`; the probes (`radar.duplicates`, `radar.force_push`) also waited out so no later radar mark can land after the frame.
- m1: the bar still offers `c continue` while a write runs; `c` now answers in words. Whether the bar should drop it meanwhile is open (not in scope).
- m1 added 11 lines to `update/banner.rs`, already 793 lines before this ticket: over PROMPT 6.2's bound, pre-existing, not split here.
- M1: `lists/resolver.rs` takes the whole resolver family (`resolver_context_list`, `offering_c`, `resolver_bar`, `resolver_list`, `resolver_card_list`, `resolver_done_list`) with `finish_list`, not only the finish and done lists: a module named for the resolver holding two of its six tables would read wrong. Still a pure move.
- M1: `draw_finish_row` moved with `FinishRow` into `finish_row.rs`; without it the file stayed at 425.
- No compaction.
