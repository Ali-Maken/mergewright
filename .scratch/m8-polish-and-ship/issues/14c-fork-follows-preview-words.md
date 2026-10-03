# G-daily 3b — the preview, the rewrite editor and the banner say each fork's fate

Status: done (partial on one item: `left_behind` kept — see NOTES Q1)
Build with: Opus (conductor, after scout `scouts/14b-fork-follows.md`; chain line is drawn in graft-ui graph_screen/banner.rs:104 — crate set adds graft-ui)
Blocked by: 14b
Needs: `fork-follows-engine`: `PreviewPlan.forks`, `PreviewPlan.forks_more`, `Fork`, `ForkFate`, `WhyStays::words`,
`RewriteProgress.fork`.
File set: graft `update/preview/line_rows.rs`, `view/rewrite_editor/right.rs`, the banner's chain line (scout names
the file), their tests; graft-ops `preview/effects.rs` only to delete `left_behind`/`left_behind_more` and
`LeftBehind` once nothing reads them.

## Read first
- `docs/design-notes/m8-polish-and-ship.md` "Pass 2c" (the words and looks)
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` §2 (fates, reasons)
- `docs/research/ux-principles.md` §3.6 (different things look different)
- `crates/graft/MAP.md` `update::preview` (line rows), rewrite editor, banner; `crates/graft-ops/MAP.md` `## preview`
- Today's drawing: `update/preview/line_rows.rs:134,173`, `view/rewrite_editor/right.rs:216` (ticket 14 NOTES)

## Scout questions (answer before `Build with` is set; ≤1 page, code not intent)
1. Every reader of `left_behind` / `left_behind_more` / `LeftBehind` in the workspace (file:line), docs excluded.
2. The shared line-row builder: its file, and each caller (rebase `r`, `R`, `a`, absorb, rewrite editor).
3. The dependents' row look today (`built on yours`): its marker glyph and theme token; the left-behind row's.
4. The banner's chain line: which file draws `feature-b, branch 2 of 3`, and from which `RewriteProgress` fields.
5. Tests and captures that assert the `left behind:` words (file:line, shot paths), `TESTING.md` rows included.

## Scope
In: one row per fork in the line rows of every rewrite/rebase preview and in the rewrite editor's right pane;
`+N more stay` for `forks_more`; the banner names the fork during a stop in its section; remove `left_behind`.
Out: engine (3a); keys (none claimed; nothing new to press).

## Words and looks (decided)
- Follows: `feature-x follows · 2 commits from a1b2c3d` — the same marker and colour as a dependent branch
  (`built on yours`); the ghost graph already shows its rows moved (3a's ghosts).
- Stays: `feature-x stays on a1b2c3d · <WhyStays::words>` — the m7/37 left-behind look (muted, its glyph).
- Order: follows first, then stays, each by fork point oldest first. A stays row never reads as an error unless it
  is one: `would conflict` is the warning colour, the other reasons muted.
- Banner, stop inside a section: `feature-x, forked at a1b2c3d` where the line form reads `feature-b, branch 2 of 3`.

## Acceptance
- [x] Preview with one follower and one conflicting fork: both rows, words as above (test on the rendered rows).
      — `update::preview::tests::the_box_draws_one_row_per_fork`, `fork_rows_table`; real repo:
      `tests/it m7_walk::fork_rows::the_put_in_box_names_each_fork`.
- [x] Rewrite editor right pane: same two rows (test). — `view::rewrite_editor::right::tests::the_after_side_names_each_fork`,
      `a_fork_that_stays_for_a_plain_reason_is_no_warning`; real repo: `m7_walk::fork_rows::the_rewrite_editor_names_each_fork`.
- [x] Banner at a stop inside a section names the fork (test on `RewriteProgress { fork: Some(..) }`).
      — `update::banner::rewrite_stop_tests::a_stop_inside_a_fork_names_the_fork` (line of 3 and single branch),
      `graft-ui graph_screen::banner::tests::a_stop_inside_a_fork_says_the_fork`, `at` read from git:
      `graft-ops it fork_follows::progress_inside_a_section_names_the_fork`; real repo:
      `m7_walk::fork_rows::a_stop_inside_a_fork_names_it_on_the_banner`.
- [ ] `left_behind` gone workspace-wide — **not done**, NOTES Q1: `r` and backups still read it.
- [x] Captures `docs/shots/m8/fork-follows-preview-187x44.txt`, `…-rewrite-editor-187x44.txt`,
      `…-banner-187x44.txt`; retake `docs/shots/m7/amend-into-left-behind-187x44.txt` and
      `rewrite-editor-left-behind-187x44.txt` (or mark them superseded in the contact sheet). Colour unverified.
      — written by the three `fork_rows` tests above; m7 two retaken by `the_box_says_who_stays_behind`,
      `the_rewrite_editor_after_says_who_stays_behind`. Colour **unverified**.

## NOTES (builder, 2026-10-04)
No compaction. Scout used: `scouts/14b-fork-follows.md` (answers 1, 2, 4); 3 and 5 checked by grep here.

- **Q1 — `left_behind` kept.** Two readers the ticket did not count: `r` (`preview/kinds/rebase.rs:81`,
  `forks` empty for `r` — MAP graft-ops) and a rewrite's backup at a line tip (not a fork, ADR-0059 §1;
  `preview/forks.rs` `candidates_of` skips it, `Fates::left_behind` keeps it). Deleting the field drops
  both rows (m5 shot `left behind: main, redesign` is an `r`). Built instead: forks drawn from `forks`, and
  `left behind:` only for `not_forks` (left_behind minus forks). Removing it needs `r` forks (KNOWN_GAPS)
  or backups given words — a spec question: what does a backup row say?
- **Q2 — words not in the spec:** a following fork's AFTER ghost note reads `follows` (it was `built on
  yours · follows`, false for a fork); `+N more stay` drawn as `+1 more stay` at one; `0 commits` for a
  0-commit follower (unreached in tests).
- **Q3 — looks:** the preview box draws every plan line in `Text` (`graft-ui preview_box.rs:268`), so
  muted/warning per row is not possible there without a graft-ui change; the fork rows look as the
  dependents' and `may conflict:` rows do today. Editor: `would conflict` rows lead with `!` (Warning,
  `graft-ui rewrite_editor/right.rs:70`); other stays rows are `Text`, not muted (same file, no muted path).
- **Banner `at` needed a graft-ops change** outside the file set: `RewriteProgress.fork` is now
  `Option<Box<ForkStop { name, at }>>` (`at` = parent of the section's first commit row, one `rev-parse`).
  Boxed because 40 more inline bytes in `AppState` tripped `clippy::large_stack_arrays` in 5 graft tests
  (verified: errors gone once boxed). Cause, not symptom, for the banner; the box is a layout choice.
- **Banner capture's stop is made with git** (`rebase -i` with an `edit` inside a `graft-fork-1` section),
  then the app opened on it: a follower is forecast clean, so the app never stops there by itself.
- Branches *on* a follower (moved with it, not forks) still draw as `built on yours · follows` — unchanged.
- Gates: fmt, clippy, doc tests pass. Full nextest 5345/5347: `graft-fixtures shapes::conflicts::stopped_shapes_are_as_documented`
  and `graft it resolver::shell_begun::c_y_finishes_a_shell_begun_merge` failed once under load (969 s run,
  another builder's cargo alongside), both pass re-run alone; neither touches this ticket's code. Cause not looked into.
- m5 shots (`left behind: main, redesign`) unchanged: that is `r`. TESTING.md: no `left behind` rows (grep).

## Touches
- Every preview kind that draws line rows (rebase `r`, `R`, `a`, absorb): one shared row builder (grep its callers).
- Captures above; m5 contact-sheet `left behind: main, redesign …` lines (main now reads `stays · the default
  branch`): retake or mark superseded. `TESTING.md` M7 rows naming `left behind` (grep). MAP.md graft, graft-ops.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~40k.
