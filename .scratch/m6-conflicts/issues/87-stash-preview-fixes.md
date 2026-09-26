# 87 — stash preview: review 86 fixes

Status: done · Build with: Opus · Blocked by: 86 (merged f68f0857)
Source: `docs/reviews/m6-final-fixes-86.md` M1, M2, m1–m7. Crates: graft-ops, graft (graft-ui only for M2's bar, `stash_screen.rs:175`).

Fix each at its cause, per the review's *Fix* lines (file:line there):

- **M1** — the forecast must follow git's `--index` rule: when `stash^2`'s tree differs from both `stash^1` and the
  index-now tree, check `diff-tree --binary stash^2^ stash^2 | apply --cached --check` against the same temp index
  copy. Failure → refusal `the staged part of stash@{n} no longer applies` (no `[ Apply ]`), never `Clean`/`Likely`.
  Oracle row: staged line 5 / committed line 3 (the review's repro).
- **M2** — no stash-list bar under the preview box; the box owns the bar row as on the graph. Re-take the three captures.
- **m1** index-only stash: name `stash^2`'s paths (`index only: <paths>`), never an empty `working tree:` line.
- **m2** past `PATHS_CAP`: `a, b, c and more`, no wrong `(+n)`.
- **m3** refused box: bar `esc back`, no `←→ choose`.
- **m4** (conductor's ruling): an on-disk blocker that is not an edit of his (untracked/ignored) reads `<path> is in the way`; `your unsaved edits to <paths> are in the way` stays for tracked edits.
- **m5** mixed refusal names both sets (unsaved and staged).
- **m6** `the_stash_preview_honours_stop` loops until `Ok`, asserts every earlier answer `Stopped`, final not `NotChecked`.
- **m7** fence `stash-pop-clean 3` in `docs/design-notes/m6-conflicts.md` `journey-counts`, counted by its walk.

## Acceptance (each tick names its test, at tick time)

- [x] M1 oracle row + graft-ops test: staged part no longer applies → refused, never clean —
  `graft-ops/tests/it/stash_preview.rs` `a_staged_part_that_no_longer_applies_refuses_the_preview` (apply and pop;
  also asserts nothing written: index bytes, refs, stash list, status, git-dir names) and
  `the_stash_forecast_agrees_with_git` row "staged line 5, committed line 3" (git exits non-zero, tree unchanged).
  Red first: before the check both failed with `Clean` / "forecast clean, git failed".
- [x] M2 app/capture test: no stash-list bar while the preview is open; captures re-taken —
  `graft/tests/it/m6_stash_put_back/stash_preview.rs` `no_list_bar` in `stash_look_and_leave_writes_nothing`,
  `the_pop_preview_fits_80_columns`, `unsaved_edits_in_the_way_are_named_in_the_box` (bar words absent, last row
  blank); `graft-ui` `stash_screen::tests::a_box_over_the_list_leaves_the_bar_row_blank`. Three captures under
  `docs/shots/m6/` re-taken (`GRAFT_UPDATE_SHOTS=1`), red before.
- [x] m1–m6 each a named test —
  m1 `an_index_only_stash_names_its_staged_paths` (graft-ops) + `an_index_only_stash_names_its_staged_paths_in_the_box`
  (graft `update/stash/tests.rs`) + oracle row "index only" (clean, git agrees);
  m2 `past_the_cap_the_box_says_and_more` (graft);
  m3 `a_refused_stash_box_offers_only_esc_back` (graft), `a_back_only_box_draws_esc_back` (graft-ui), and the
  in-the-way capture test asserting `╰ esc back ─`, no `choose`;
  m4 `an_untracked_file_in_the_way_refuses_the_preview` (untracked and ignored: `new.txt is in the way`);
  m5 `a_mixed_refusal_names_unsaved_and_staged`;
  m6 `the_stash_preview_honours_stop` (loops until `Ok`, every earlier answer `Stopped`, final is
  `Likely{notes.md}`, not `NotChecked`).
- [x] m7 fence row + walk; `journey_counts_match_the_note` green — row `stash-pop-clean 3` added;
  walked by `m6_stash_put_back/stash_preview.rs` `a_clean_stash_pop_is_three_keys` (`S p y`, counted against
  `noted_keys`), listed in `m6_stash_put_back::JOURNEYS`; `m5_walk::conflict_journeys::tests::journey_counts_match_the_note` green.
- [x] Contact sheet rebuilt; parts ≤ 200 000 B; gates workspace-wide green — `scripts/contact-sheet.sh
  --captions-order m6 docs/shots/m6`: parts 179 772 / 177 281 / 176 526 / 179 713 / 5 189 B. Captions for the three
  captures extended (ticket 87 lines). fmt, clippy `--all-targets -D warnings`, nextest 3529 passed / 1 skipped,
  `cargo test --doc` all green, workspace-wide.

Touches: stash preview (graft-ops `preview/stash.rs`, `preview.rs`, `operation.rs`), `update/preview.rs` stash box,
stash screen bar, 3 captures + sheet, design-note fence, MAP.md rows if shapes change.

## NOTES

- No scout: review 86 named every file:line against f68f0857 and served as the scout.
- Shapes changed: `Refusal::StashInTheWay { unsaved, staged, untracked }` (was `{ paths }`), new
  `Refusal::StashIndexNoLongerApplies { index }`, new `WorktreeEffect::IndexOnly { paths, more }`, new
  `snapshot::index_state_then` (crate-private), `StashView::box_open`, `PreviewBox::back_only`. MAP rows updated
  (graft-ops, graft-ui). Exhaustive matches on `WorktreeEffect` found by the compiler: `update/preview.rs`
  `worktree_line`, `tests/it/m5_walk/claim.rs` (IndexOnly: files on disk unchanged, as `Untouched`).
- **Outside the stated graft-ui scope:** m3 needed `graft-ui/src/preview_box.rs` (`back_only` field, footer
  `esc back`). The footer is built there; no graft-side field could produce it. Scoped to stash put-back closing
  boxes only (`closing_box` sets it from `stash_put_back`).
- Question for the conductor, not fixed: every *other* refused/closing preview box (merge, rebase, …) still shows
  `←→ choose  ⏎ act  esc cancel` over a single `[ Close ]`. Same defect as m3 in other screens; changing it would
  re-take other milestones' captures.
- Question, not checked: the `d` drop confirm over the stash list may also leave the list's bar under it (the bar
  follows `state.preview` only). Not read.
- The `--index` check runs on every git (plumbing old git has); the `merge-tree` forecast still needs 2.40.
- `every_key_the_bar_draws_is_answered` still sweeps only the graph's contexts (review M2's note); not extended.
- Did not compact.
