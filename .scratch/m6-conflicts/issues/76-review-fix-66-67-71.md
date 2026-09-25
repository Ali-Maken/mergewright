# 76 — review fix for 66, 67, 71 (Major M1; Minor 1, 2, 4, 5, 6, 7, 8, 9)

Status: done
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
- [x] **M1** (`update::context_menu::verbs::tests::*` incl. the slots table, and `update::context_menu::tests::*`; unedited except m7; green): `Slots`, `history_slots` and `is_stash_row` move to `update/context_menu/slots.rs`; one function builds the
      slots directly (no re-match of `verbs[0].op`, no second current-branch lookup, one stash-row test). `verbs.rs`
      under 400 non-test lines, or say why not. Existing menu tests unedited and green. Report new line counts of
      `verbs.rs`, `slots.rs`, `update/preview.rs`, `update/operate.rs` in NOTES (the conductor extends the size rows).
- [x] **m1** (`graft-ops mistake_radar::probes::tests::duplicates_are_found_beside_a_root_file_named_head`, red first: exit 128 "ambiguous argument 'HEAD'"): the duplicate walk ends its revisions with `GitArg::Separator`; `HEAD` is not a `Flag`. Test: a probe on a
      repo with a tracked root file named `HEAD` finds the duplicate (seen red first).
- [x] **m2** (`graft it radar_bookkeeping::a_pull_rebase_marks_no_duplicates`; with `duplicate_walk()` swapped to `--all --` it failed at :123 with 4 duplicate findings, then restored): `Ctrl+R` dropped from the 66 walk, its `HEADER` and the m6 caption row; `fix-radar-bookkeeping-120x40.txt`
      retaken. The walk still fails with `DUPLICATE_WALK = ["--all"]` (say how you checked).
- [x] **m4** (docs only; no test writes the m2 files — grep of `crates/` finds only the pty-driven gallery example — so they are marked HISTORICAL): TESTING.md M2 gallery stop 7 rewritten to today's menu order. `docs/shots/m2-context-menu.txt` and
      `m2-07-foxtrot-merge-dot-*.txt`: retake through the gallery if a test writes them, else mark them historical in
      their header line.
- [x] **m5** (`graft-ops it history_ops::every_preview_refuses_exactly_what_check_refuses`: 14.4 s before, 5.6 s after, nextest single test): `every_preview_refuses_exactly_what_check_refuses` builds each kind's repo once (clean case, then dirty).
      Report the time before and after.
- [x] **m6** (dropped; the tick is `graft_ui::context_menu::ContextMenu::for_row`'s signature, which takes no width, and its doc comment says so): `menu_contents_do_not_depend_on_width` either dropped (the tick points at `for_row`'s signature) or made
      able to fail (drawn through `view::frame` at 80 and 120, labels compared). Say which.
- [x] **m7** (`update::context_menu::tests::enter_on_each_enabled_slot_runs_its_own_verb`, green; not seen red): `enter_on_each_enabled_slot_runs_its_own_verb` also asserts the label at slot *n* is `verbs[index]`'s.
- [x] **m8** (build and the `graft` preview tests green; no behaviour change): the `failed` closure in `jobs/preview.rs:47` replaced by `map_err(PreviewFailure::Failed)` at the sites.
- [x] **m9** (`graft it m6_stash_put_back::capture_menu_fixed_slots` now asserts that fix's only finding is `Risk::NoUpstream`; caption and header reworded; capture retaken): `capture_menu_fixed_slots` caption: say the `ⓘ` is m9's known false no-upstream mark, or push `fix` upstream
      in the fixture. Retake if the capture changes.
- [x] (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`; parts 179 678 / 177 038 / 163 295 / 31 857 B) Contact sheets rebuilt for any milestone whose captures changed; each part ≤ 200 000 B. No machine paths.

## Touches (check each, report how)
- Every caller of the moved items: the build.
- Captures naming `Ctrl+R` or the menu order: grep `docs/shots`.
- TESTING.md rows for the 66 walk and the menu.
- MAP.md for graft (new `slots.rs`).
- KNOWN_GAPS and `docs/design/` are the conductor's; don't edit them.

## NOTES
- Line counts (non-test / total): `verbs.rs` 378 / 380 (was 469); `slots.rs` 119 / 119; `update/preview.rs` 1123 / 1292; `update/operate.rs` 1030 / 2501. 76 did not change preview.rs or operate.rs.
- `history_slots` builds the list and the slots in one pass. `checkout_verbs` returns the current branch it skipped (the dim reason), so there is no `verbs[0].op` match and no second branch lookup. `history_verbs` is now `#[cfg(test)]`, because only tests called it. `pick_verb` was extracted.
- `duplicate_walk()` replaces `DUPLICATE_WALK`. `HEAD` is now `GitArg::Ref`, and the walk ends in `--` and comes after `-n`/`--format`/`-p`.
- m2: in the retaken capture, Frame 1's bottom row now shows `pushed main → origin/main ✓`, because no `Ctrl+R` cleared it.
- m4: TESTING stop 7's menu order was read from `for_row` + `history_slots`, not from a gallery run (the gallery needs a pty; not run). The gallery banner text for stop 7 was updated too (`tests/it/gallery.rs`).
- Question: the m2-07 stills have a partly masked checkout path in their title row. This was already there before 76. Not scrubbed, because scrubbing it changes the truncation.
- Incident: an early edit landed in the main checkout's `crates/graft/src/update/context_menu/verbs.rs` (a heredoc run from the main path). It was restored byte-for-byte from `f7942bf6`'s blob and checked with `diff`. No other file in the main tree was mine (mtime scan of `crates/`).
- Colour unverified. No scout: the review named every line.
