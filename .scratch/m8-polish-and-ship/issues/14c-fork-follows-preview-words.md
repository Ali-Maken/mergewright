# G-daily 3b — the preview, the rewrite editor and the banner say each fork's fate

Status: done — B1–B5 built 2026-10-04 (remainder compacted once; see NOTES)
Build with: Opus (conductor, after scout `scouts/14b-fork-follows.md`; chain line is drawn in graft-ui graph_screen/banner.rs:104 — crate set adds graft-ui)
Remainder build with: Sonnet — words and one style path in files the NOTES already name; no decision left open.
Blocked by: 14b
Needs: `fork-follows-engine`: `PreviewPlan.forks`, `PreviewPlan.forks_more`, `Fork`, `ForkFate`, `WhyStays::words`,
`RewriteProgress.fork`.
File set: graft `update/preview/line_rows.rs`, `view/rewrite_editor/right.rs`, the banner's chain line (scout names
the file), their tests; graft-ops `preview/effects.rs` only to delete `left_behind`/`left_behind_more` and
`LeftBehind` once nothing reads them. Remainder adds graft-ui `rewrite_editor/right.rs` (the muted path, R4).

## Read first
- `docs/design-notes/m8-polish-and-ship.md` "Pass 2c" (the words and looks)
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` §2 (fates, reasons)
- `docs/research/ux-principles.md` §3.6 (different things look different)
- `crates/graft/MAP.md` `update::preview` (line rows), rewrite editor, banner; `crates/graft-ops/MAP.md` `## preview`
- Today's drawing: `update/preview/line_rows.rs:134,173`, `view/rewrite_editor/right.rs:216` (ticket 14 NOTES)
- Remainder: this ticket's NOTES and Rulings, then `graft-ui rewrite_editor/right.rs:70` (the Warning path).

## Scout questions (answer before `Build with` is set; ≤1 page, code not intent)
1. Every reader of `left_behind` / `left_behind_more` / `LeftBehind` in the workspace (file:line), docs excluded.
2. The shared line-row builder: its file, and each caller (rebase `r`, `R`, `a`, absorb, rewrite editor).
3. The dependents' row look today (`built on yours`): its marker glyph and theme token; the left-behind row's.
4. The banner's chain line: which file draws `feature-b, branch 2 of 3`, and from which `RewriteProgress` fields.
5. Tests and captures that assert the `left behind:` words (file:line, shot paths), `TESTING.md` rows included.

## Scope
In: one row per fork in the line rows of every rewrite/rebase preview and in the rewrite editor's right pane;
`+N more stay` for `forks_more`; the banner names the fork during a stop in its section; remove `left_behind`
(amended by R1: kept until `r` reports forks).
Out: engine (3a); keys (none claimed; nothing new to press); per-line colour in the preview box (R3).

## Words and looks (decided)
- Follows: `feature-x follows · 2 commits from a1b2c3d` — the same marker and colour as a dependent branch
  (`built on yours`); the ghost graph already shows its rows moved (3a's ghosts).
- Stays: `feature-x stays on a1b2c3d · <WhyStays::words>` — the m7/37 left-behind look (muted, its glyph).
- Order: follows first, then stays, each by fork point oldest first. A stays row never reads as an error unless it
  is one: `would conflict` is the warning colour, the other reasons muted.
- Banner, stop inside a section: `feature-x, forked at a1b2c3d` where the line form reads `feature-b, branch 2 of 3`.

## Acceptance
- [x] Preview with one follower and one conflicting fork: both rows, words as above (test on the rendered rows).
      — `update::preview::tests::a_rewrite_box_draws_one_row_per_fork` (renamed in the remainder), `fork_rows_table`; real repo:
      `tests/it m7_walk::fork_rows::the_put_in_box_names_each_fork`.
- [x] Rewrite editor right pane: same two rows (test). — `view::rewrite_editor::right::tests::the_after_side_names_each_fork`,
      `a_fork_that_stays_for_a_plain_reason_is_muted` (renamed); real repo: `m7_walk::fork_rows::the_rewrite_editor_names_each_fork`.
- [x] Banner at a stop inside a section names the fork (test on `RewriteProgress { fork: Some(..) }`).
      — `update::banner::rewrite_stop_tests::a_stop_inside_a_fork_names_the_fork` (line of 3 and single branch),
      `graft-ui graph_screen::banner::tests::a_stop_inside_a_fork_says_the_fork`, `at` read from git:
      `graft-ops it fork_follows::progress_inside_a_section_names_the_fork`; real repo:
      `m7_walk::fork_rows::a_stop_inside_a_fork_names_it_on_the_banner`.
- [ ] ~~`left_behind` gone workspace-wide~~ — **replaced by B1** (ruling R1).
- [x] Captures `docs/shots/m8/fork-follows-preview-187x44.txt`, `…-rewrite-editor-187x44.txt`,
      `…-banner-187x44.txt`; retake `docs/shots/m7/amend-into-left-behind-187x44.txt` and
      `rewrite-editor-left-behind-187x44.txt` (or mark them superseded in the contact sheet). Colour unverified.
      — written by the three `fork_rows` tests above; m7 two retaken by `the_box_says_who_stays_behind`,
      `the_rewrite_editor_after_says_who_stays_behind`. Colour **unverified**.

### Remainder (from the Rulings; each tick names its test)
- [x] B1 (R1): no rewrite kind (`a`, `R`, absorb, reword-head, rewrite editor) draws `left behind:`; a rewrite's
      backup row reads `<name> stays on <sha7> · on the old tip`, muted, last in the stays block; `r` keeps
      `left behind:` (row tests: one rewrite kind with a backup, one `r`).
      — `update::preview::tests::a_rewrite_box_draws_one_row_per_fork` (any `Operation::Rewrite`; all five
      kinds run one), `a_rebase_box_keeps_left_behind`; editor: `view::rewrite_editor::right::tests::the_after_side_names_each_fork`
      (backup row `FactTone::Muted`, last of the stays).
- [x] B2 (R2): `+1 more stays` / `+N more stay` (N ≥ 2); follower `1 commit`, `N commits`, and 0 → `feature-x
      follows · from a1b2c3d` (unit tests on the row builder, all four). — `update::preview::tests::fork_rows_table`.
- [x] B3 (R3): in the preview box a `would conflict` stays row leads with `!`, as the editor's does (row test).
      — `a_rewrite_box_draws_one_row_per_fork`, `a_conflicting_fork_row_warns`; real repo:
      `m7_walk::fork_rows::the_put_in_box_names_each_fork` (shot shows `! feature-c …`).
- [x] B4 (R4): editor stays rows other than `would conflict`, and the backup row, draw muted (graft-ui test on the
      row's style; colour **unverified** in captures). — graft-ui `rewrite_editor::tests::each_fact_draws_in_its_tone`
      (buffer fg per `FactTone`); graft `a_fork_that_stays_for_a_plain_reason_is_muted`. Colour **unverified**.
- [x] B5: retake the three m8 fork-follows captures and any capture whose rewrite box shows a backup's
      `left behind:` line (grep `docs/shots` for `left behind:`; `r` shots, m5 `left behind: main, redesign`, stay).
      — `m7_walk::fork_rows::*` (three tests). Diffs: preview `! feature-c stays …` (B3) and the 07 cursor `▸`
      in preview + banner; rewrite-editor shot unchanged (its only stays row was already `!`). grep `left behind:`
      in `docs/shots`: only m5 `r` shots — no rewrite shot to retake.

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

## NOTES (remainder, builder, 2026-10-04)
**Compacted once** mid-remainder (context ran out after the B1–B4 code edits, before tests); nothing left undone.
- graft-ui: `AfterView::Ready.facts` is now `Vec<AfterFact { text, tone: FactTone::{Plain, Warning, Muted} }>`
  (was `Vec<SanitizedText>` with a `!` prefix read as Warning). graft's editor builds them; no other caller (grep
  `AfterView::Ready` workspace).
- graft `fork_rows(plan, rewrite) -> Vec<ForkRow{text, tone}>`; `left_behind_row(plan, rewrite, columns)` is
  `None` for a `Rewrite`; `not_forks` private. Editor `after_facts` split into helpers (clippy too_many_lines).
- Merged main twice (07's cursor `▸`; then `ddde033c` came-back-landed re-shoot) — no conflicts.
- Box tone per line: none (R3); the box's fork rows are all `Text` — gap `M8/preview-box-line-tone` (conductor's).

## Rulings (planner, 2026-10-04)

**R1 — Q1: keep `left_behind`, don't rename, remove later.**
- Kept: `r` still leaves forks behind (gap M8/fork-follows-r), so `left behind:` is true there. Renaming now churns
  `r` code for a field that goes when that gap closes.
- Removal moves to the `fork-follows-r` spec: once `r` fills `forks`, the backup row gets its own source (a
  `PreviewPlan` list of backups, or `Fork` with a `Backup` fate; that spec decides), then the field and
  `LeftBehind` go. Conductor: add this to the KNOWN_GAPS M8/fork-follows-r row.
- One box, one vocabulary (UX consistency): a rewrite box must not hold `stays on …` rows beside a `left behind:`
  row. A rewrite's backup row (from `not_forks`) reads `<name> stays on <sha7> · on the old tip`, muted, last in
  the stays block. Not "backup" in the words: that word already means the journal entry's backup (`EditsAt::Backup`).
- `r` keeps `left behind: …` unchanged until fork-follows-r.

**R2 — Q2: words approved, with number agreement.**
- AFTER ghost note `follows`: approved (ADR-0059's word; `built on yours` was false for a fork). Branches on a
  follower keep `built on yours · follows`: approved.
- `+N more stay` was already in Scope. At one: `+1 more stays`.
- Follower count: `1 commit`, `N commits`; 0 own commits (ADR-0059 §1, git moves it) → `feature-x follows · from
  a1b2c3d` — no `0 commits`.

**R3 — Q3, preview box: no per-line colour in this ticket.**
- The box draws every line in `Text`, `may conflict:` lines included; colouring only the fork rows would make the
  box disagree with itself. Per-line tone in the box (all kinds) is a graft-ui scope change → conductor files gap
  `M8/preview-box-line-tone` (broken: UX §3.6 not met in the box; the words carry it meanwhile).
- Shape instead of colour, in the file set: a `would conflict` stays row in the box leads with `!`, the editor's
  marker. Same thing, same mark, both surfaces.

**R4 — Q3, editor: add the muted path.** graft-ui `rewrite_editor/right.rs` already has the Warning path (`:70`);
add muted for the other stays rows and the backup row, as "Words and looks" decided. graft-ui is in the crate set.

**R5 — banner `ForkStop { name, at }` in graft-ops: accepted.** The decided banner words need `at`; one `rev-parse`
in `progress` is the smallest source; the box is a size fix with its cause measured. graft-ops MAP already
describes it (`## rewrite`, `progress`). Next time: report `blocked: needs graft-ops` first unless the crate is
free and the change is the ticket's own need — here both held.

## Touches
- Every preview kind that draws line rows (rebase `r`, `R`, `a`, absorb): one shared row builder (grep its callers).
- Captures above; m5 contact-sheet `left behind: main, redesign …` lines (main now reads `stays · the default
  branch`): retake or mark superseded. `TESTING.md` M7 rows naming `left behind` (grep). MAP.md graft, graft-ops.
- Remainder: graft-ui `rewrite_editor/right.rs` (muted path) and its MAP entry; graft `update::preview` MAP entry
  (backup row words); the m8 contact sheet (retaken captures); KNOWN_GAPS rows M8/fork-follows-r (removal owed),
  M8/14c-fork-wording (closed by the `follows` note), new M8/preview-box-line-tone — conductor's edits.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~40k. Remainder ~15k.
