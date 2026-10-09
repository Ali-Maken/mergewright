# 20e — fixes for review m8 words-focus (M1, M2, M4, m1–m9)

Status: done (m9 open, a question)
Spec: `docs/reviews/m8-words-focus.md` — each finding's own **Fix** text. M3 done by the conductor; not here.
Scout: none dispatched — the review's file:line citations served as the scout; each was read at HEAD before its edit.

## Boxes

- [x] **M1** — `Below`/`Tab` pick the radar's rung against the cells after `subject · `; when even the narrow
      rung does not fit, the subject is cut with `…` so `N not checked` stays (`header.rs` `subject_radar`).
      Test: `graft-ui detail_pane::tests::the_radar_beside_a_long_subject_keeps_not_checked_at_80_columns`
      (seen red: `… · 2 han…`, then `… · 2 not…` after the first half-fix; green after the subject cut).
      Capture: `docs/shots/m8/radar-narrow-80x24.txt`, test `capture_radar_beside_a_long_subject_80x24`.
- [x] **M2** — on the focused banner `u`/`U` name no way clause; the reason alone (`undo.rs` `banner_keys`).
      Test: `keymap_words::undo_journal::u_refused_on_the_focused_banner_names_no_way_to_it` (seen red:
      `Some("⇥ banner")`). Old test `banner::tests::a_focused_banner_answers_u_instead_of_swallowing_it` asserted
      focused == unfocused words; now the same reason, keys `None`. Likewise `it::conflict_interim::u_still_refuses_after_relaunch_into_a_merge_begun_in_graft` pinned `⇥ banner` on the banner; now `None`.
- [x] **M4** — `02-keymap.md` §2.6a: the `u`/`U` line names the shorter way, ties forward, and the focused-banner
      case; `Tab` / `Shift+Tab` / `Esc` row (Shift+Tab lands on the diff, verified `focus_keys.rs:124`); `edit`
      stop "`⇥` focuses it with the pane closed, one `⇧⇥` with it open". Check: by reading against
      `stop/tab.rs` `way_keys` and the M2 test above.
- [x] **m1** — caption for `m6/fix-c-live-frames-187x50.txt` frame 2 now "leads with `⇧⇥ banner` (one press back
      reaches it), no forward `⇥ banner`": test header in `resolver/shell_begun.rs`, captions TSV, sheet 3
      re-assembled (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`, 168 727 B). Test:
      `resolver::shell_begun::capture_fix_c_live_frames` green after re-shoot.
- [x] **m2** — TESTING.md scenario 10 and the Rejected-sticky row quote `has commits you don't · p pull, then m or r`.
      The row's "68 columns" claim replaced with what `m4-06-rejected-sticky-80x24.txt:26` shows. Check:
      `grep "p pull first" TESTING.md` → none.
- [x] **m3** — stale `⌃N presses reach it` dropped (`keymap_words/resolve.rs`). Check: by reading.
- [x] **m4** — one private `spell_way(action, n, keys)` shared by `tab_keys` and `way_keys`. Tests:
      `keymap_words::resolve` and `undo_journal::u_refused_from_the_changes_pane_names_two_tabs` green.
- [x] **m5** — `WAY_BACK` → `WAY_TO_BANNER_FORWARD`, `WAY_BACK_PREV` → `WAY_TO_BANNER_BACK`. Test: graft-ui
      `hints` suite green (compile-checked rename).
- [x] **m6** — `MARK_RESOLVED => MARK_RESOLVED_SHORT` as a match arm. Test: `hints::tests` row asserting
      `MARK_RESOLVED_SHORT == "mark"` and the graft-ui suite green.
- [x] **m7** — `session.rs` `Clock` doc states both halves: drawn clock `Wall` or `Fixed` under `GRAFT_NOW`;
      journal always `Wall`. Check: by reading against `launch_clock`.
- [x] **m8** — renamed to `clock_pin::u_undoes_the_newest_under_a_pinned_drawn_clock`; green.
- [ ] **m9** — open, a question (below).

## NOTES

- **m9 question (not decided):** `docs/shots/m8-tour-80x24.txt` caption promises "`◇ work` with a gap before the
  path"; line 3 draws `◇ work· …`. Which is right — the caption (then the header lacks a space: a code defect in
  the tour header) or the picture (then reword the caption)? A product/layout call; left for the conductor / ticket 28.
- M1 goes past the review's minimum: fitting the radar to `width − subject` alone still cut `not checked` at 80
  columns with a 47-cell subject (`… · 2 not…`). So the subject yields cells. A design call the review implies
  ("the one fact R7 exists to say") but does not state — flag if wrong.
- Colour unverified (rule 5). The new capture is a TestBackend grid, not a pty.
- Gates: fmt, clippy -D warnings, test --doc clean; nextest workspace 6207/6208, the one red was the pinned test above, fixed and re-run alone green (not a second full run).
- No compaction.
