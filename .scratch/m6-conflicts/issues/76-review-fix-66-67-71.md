# 76 — review fix for 66, 67, 71 (Major M1; Minor 1, 2, 4, 5, 6, 7, 8, 9)

Status: ready
Blocked by: 68+69 merged (both touch graft; 69 edits `update/preview.rs`)
Crates: graft, graft-ops, graft-ui (tests only: `context_menu.rs` test) + TESTING.md, `docs/shots/`
Source: `docs/reviews/m6-final-fixes-66-67-71.md` M1, m1, m2, m4, m5, m6, m7, m8, m9. Each names its lines.
Build with: builder-sonnet
Scout: none needed. The review names every line; re-check each against main after 68+69 land.

## Decided (conductor, 2026-09-25)
- **Stash rows keep the pre-71 order** (review Question A). A stash commit's Checkout/Cherry-pick is not his daily task;
  `is_stash_row` stays, moved with the slots. Say so in the moved function's doc comment.
- m3 (keymap and note words) and m10 (log wording) are the conductor's; not here.

## Acceptance
- [ ] **M1:** `Slots`, `history_slots` and `is_stash_row` move to `update/context_menu/slots.rs`; one function builds the
      slots directly (no re-match of `verbs[0].op`, no second current-branch lookup, one stash-row test). `verbs.rs`
      under 400 non-test lines, or say why not. Existing menu tests unedited and green. Report new line counts of
      `verbs.rs`, `slots.rs`, `update/preview.rs`, `update/operate.rs` in NOTES (the conductor extends the size rows).
- [ ] **m1:** the duplicate walk ends its revisions with `GitArg::Separator`; `HEAD` is not a `Flag`. Test: a probe on a
      repo with a tracked root file named `HEAD` finds the duplicate (seen red first).
- [ ] **m2:** `Ctrl+R` dropped from the 66 walk, its `HEADER` and the m6 caption row; `fix-radar-bookkeeping-120x40.txt`
      retaken. The walk still fails with `DUPLICATE_WALK = ["--all"]` (say how you checked).
- [ ] **m4:** TESTING.md M2 gallery stop 7 rewritten to today's menu order. `docs/shots/m2-context-menu.txt` and
      `m2-07-foxtrot-merge-dot-*.txt`: retake through the gallery if a test writes them, else mark them historical in
      their header line.
- [ ] **m5:** `every_preview_refuses_exactly_what_check_refuses` builds each kind's repo once (clean case, then dirty).
      Report the time before and after.
- [ ] **m6:** `menu_contents_do_not_depend_on_width` either dropped (the tick points at `for_row`'s signature) or made
      able to fail (drawn through `view::frame` at 80 and 120, labels compared). Say which.
- [ ] **m7:** `enter_on_each_enabled_slot_runs_its_own_verb` also asserts the label at slot *n* is `verbs[index]`'s.
- [ ] **m8:** the `failed` closure in `jobs/preview.rs:47` replaced by `map_err(PreviewFailure::Failed)` at the sites.
- [ ] **m9:** `capture_menu_fixed_slots` caption: say the `ⓘ` is m9's known false no-upstream mark, or push `fix` upstream
      in the fixture. Retake if the capture changes.
- [ ] Contact sheets rebuilt for any milestone whose captures changed; each part ≤ 200 000 B. No machine paths.

## Touches (check each, report how)
- Every caller of the moved items: the build.
- Captures naming `Ctrl+R` or the menu order: grep `docs/shots`.
- TESTING.md rows for the 66 walk and the menu.
- MAP.md for graft (new `slots.rs`).
- KNOWN_GAPS and `docs/design/` are the conductor's; don't edit them.

## NOTES
