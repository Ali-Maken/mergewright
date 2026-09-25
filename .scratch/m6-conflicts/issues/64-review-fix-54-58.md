# 64 — review fix for 54–58 (Major M1, Major M2; Minor m1, m2, m3, m4, m8)

Status: done
Crates: graft-ui, graft, graft-ops + docs
Source: `docs/reviews/m6-look-finish-54-58.md` M1, M2, m1, m2, m3, m4, m8.
Build with: builder-opus
Needs: 59 merged (same crates, graft-ui and graft). Runs before 60 (60 adds `c finish` to the done list; m8 must land first).
Scout: none needed. The review names every line; re-check each against main at build time, since 59 lands first.

## Decisions (conductor, 2026-09-25)
- **M1 → option (a): the order ADR-0044 §4 states** (`docs/adr/0044-conflict-colours-and-the-confirm-key.md:96-99`):
  1. Every hint placed at its bare key first.
  2. Then the marked (`keeps_words`) hints raised to `Full`.
  3. Then the rest raised, in the existing fixed sequence.

  Why: the ADR is the approved text, and the user's pick 5 was "drop words from everyday keys first". The code follows the ticket's reading, and at 80 columns it hides `j/k` and `M` outright, which loses visibility (`docs/research/ux-principles.md`). No ADR edit is needed. Monotonicity must still hold.

## Acceptance
- [x] **M1:** at 80 columns the resolver bar shows every key, bare at least. `j/k` and `M` are present in `docs/shots/m6/m6-13-after-ctrl-t-80x24.txt`, retaken. Assert this in a test, and see the test red first against today's order. — `graph_screen::hints::tests::resolver_bar_shows_every_key_at_80` (merge and rebase roles; seen red before the fix: `j/k`, `M` missing from the 80 keys). Capture retaken by `pty-walk-m6.py 80x24 three-conflicts-all-theirs`, then a plain run `settled`; bottom row now `… ⏎  j/k  M  d start over  esc`.
- [x] **M1:** at 120 columns the bar still keeps `d start over` and `esc close` (56's test stays green). Name it. — `graph_screen::hints::tests::resolver_bar_keeps_the_way_out_at_120` and `…_at_120_in_a_rebase`, unedited, green; `graft::it resolver::enter_on_conflicts_row_opens_the_resolver` (120 footer, `resolver.rs:147-160`) green; `m6-13-after-ctrl-t-120x40.txt` settled unchanged.
- [x] **M1:** the monotonicity test still holds (name it). — `graph_screen::tests::no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`, unedited, green.
- [x] **M2:** `crates/graft-ui/src/theme/mod.rs` ≤ ~400 lines: the conflict-token table plus `pinned`/`tint` go to `theme/conflict.rs`. `conflict_resolver/files.rs` ≤ ~400: the words split (`WordsTone`, the cell's two-style draw) goes to `files/words.rs`. Pure moves; the existing tests stay green (name the module tests). MAP.md updated. — `theme/mod.rs` 490 → 396 lines, `theme/conflict.rs` 112 (`TINT_ALPHA`, `mix`, `Token::is_tint`/`is_conflict`, `Theme::tint`, `tints_dropped`, `pinned`, `reduce_conflict`; re-exported, paths unchanged). `files.rs` 427 → 385, `files/words.rs` 65 (`WordsTone`, `FileState::tone`, `words_at_end`, `paint_words`). Module tests unedited and green: `theme::tests` (30, incl. `existing_tokens_reduce_as_before`, `tint_is_none_at_sixteen_colours_under_no_color_and_in_high_contrast`), `conflict_resolver::files::tests` (10, incl. `file_list_text_is_unchanged_but_the_mark`), `conflict_resolver::tests::card_colours`. graft-ui MAP.md: theme and file-list rows.
- [x] **m1:** fix the comment at `crates/graft/src/update/banner.rs:376-378` to say what the code does in a shell merge. Behaviour stays unchanged; that question is in KNOWN_GAPS M6/57. Doc edit: no test. — comment now says: in a shell merge there is no refusal, `undo_key` runs, the push box can open first and "Undo locally" then meets ops' `OperationLive`.
- [x] **m2:** `NotInvertible::OperationLive` gets one wording function, called by both `graft-ops/src/preview.rs:715` and `undo/execute.rs:231`. `operation_live_words_match_ops` (or a new test) covers both call sites. — `graft_ops::undo::operation_live_words(InProgress)`; both sites call it. New `graft-ops` `preview::tests::operation_live_has_one_wording_in_ops` checks `explain()` and `not_invertible()` against it for all five kinds; `operation_live_words_match_ops` (app vs ops) still green. graft-ops MAP.md row added.
- [x] **m3:** word the doc comments at `crates/graft/src/update/resolver/restart.rs:17` and `:27` by kind (markers vs keep/delete). Doc edit: no test. — `:17` names both commands by kind; `:27` says the box would promise the markers or the keep-or-delete choice.
- [x] **m4:** add TESTING.md journey rows for `b-abort-shell-merge` (3) and `b-undo-a-take-in-the-shell` (1), next to `walk-back-a-whole-file-choice` (~:2333). Give each its start, keys, count and "what right looks like". `journey_counts_match_the_note` stays green. — rows at `TESTING.md` after `walk-back-a-whole-file-choice`, worded from the walks' own asserts (`conflict_journeys.rs` `b_abort_shell_merge`, `b_undo_a_take_in_the_shell`) and the `fix-shell-merge-abort-and-undo-120x40` capture; `m5_walk::conflict_journeys::…::journey_counts_match_the_note` green.
- [x] **m8:** `graph_screen/hints/lists.rs:346` → `hint("d", "start over").keeps_words()`. Test: the done list's `d` is marked. — `graph_screen::hints::tests::done_list_marks_start_over` (seen red first); `only_the_resolver_marks_words` and `done_bar_keeps_the_way_out_at_100` green.
- [x] Captures retaken wherever the bar changed. Rebuild the contact sheet with `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`; each part must be ≤ 200 000 B. Update the captions line for m6-13 80x24 (`m6-contact-sheet-captions.tsv:87`). Colour is **unverified**. — retaken: `m6-13-after-ctrl-t-80x24` (pty), `fix-card-names-numbers-80x24` and `fix-stop-lands-in-resolver-80x24` (`GRAFT_UPDATE_SHOTS=1 cargo nextest run`, bottom row only). Captions `:87` rewritten. Sheet rebuilt: parts 179 977 / 170 759 / 85 895 B. Colour unverified.

## Touches (check each, report how)
- Every capture showing the resolver bar at 80 or 100: `grep -rl` the old bar words in `docs/shots/m6`.
- The finish and done hint lists: 59 changed them. Re-read them after the merge.
- The walk tests that read the bar.
- MAP.md for graft-ui theme and files.
- KNOWN_GAPS is the conductor's; don't edit it.

## NOTES

- Scout: none, per the ticket; every review line re-read against the tree after 59 (lists.rs `:346` is now `:346-348`, the done list; the finish list 59 changed has no marked hint and is untouched).
- **M1 result at 80:** `o yours  t theirs  b both  ⌃O/⌃T  O/T  B  e  n  N  ⏎  j/k  M  d start over  esc`. `esc close` loses its words at 80 (marked hints are raised in list order, `d` first, and `esc close` no longer fits). That is the ADR's order as written; flag for the user's eye.
- **Out of scope, done because the pty walk failed on it:** `m6-10-stop-lands-in-resolver` at 80x24, 120x40 and 160x48 had drifted before this ticket (committed `notes.txt …`, drawn `notes.txt •` — the unread file's own glyph, `files.rs` `Glyphs::unread`, which the capture predates; 187/200 unaffected). Retaken, plain run `settled`, captions `:68` and `:72` updated. Stale since some ticket after 43; the conductor may want to know why no gate caught it (pty captures are not run by nextest).
- Flake seen once: `graft-git conflict::tests::first_marker_line_agrees` failed in one full run, passed in three reruns and the final full run. Not in this crate set; not investigated.
- Pty walk re-run at every size (80, 120, 160, 187, 200): nothing else drifted.
- Touches: every 80-column capture with resolver bar words (`grep -l "d start over\|esc close" docs/shots/m6/*80x24.txt`: 7 files; m6-02/03/04 card bars settled unchanged, the other four retaken); no 100-column captures exist. Done and finish lists re-read after 59. Walk tests reading the bar: `resolver::enter_on_conflicts_row_opens_the_resolver` (120 footer), `every_key_the_bar_draws_is_answered`, all green. MAP.md: graft-ui (ladder order, `keeps_words`, theme, file list), graft-ops (`operation_live_words`). Doc comments on `Hint::keeps_words` and `only_the_resolver_marks_words` reworded to the new order. KNOWN_GAPS not edited.
