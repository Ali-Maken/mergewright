# Usability Q1: in J, a commit an undoable operation left behind reads "replaced", not "✗ lost"

Status: done
Blocked by: none
Build with: **Opus**. It reads undo semantics (which entries are in force and undoable); a wrong rule tells him real
damage is safe. Judged by tests that build the journal and git state; one new pure fn, no shape others build on, so
not Fable.
Brownfield: scout done, `.scratch/m8-polish-and-ship/scouts/46-q1-replaced.md` (code at 8c7b46df). Revised against it
2026-10-11 (spec-only pass): glyph `↺` → `⇢` (`↺` is the graph's "put back"); no ASCII glyph (J has none); words in
graft `view::journal::lost_words`; legend is one string cut by width; pty walk and its captures moved to the
acceptance ticket (57 owns `m8_walk::mergewright::journal_replaced`).
Needs: none (journal API as built). Beside: `usability-q2-picker-enter-on-current-closes`, runnable side by side
(file sets below are disjoint; shared only `docs/design/02-keymap.md` §2.11 vs §2.16 and MAP.md files, different
sections; the conductor merges those hunks). Both land before the Mergewright set.

## Read first
- `docs/reviews/m8-usability.md` Q1 (:155-158); `docs/field-notes.md:97` ("yes say replaced")
- `docs/design/02-keymap.md` §2.11 (:501-516); `docs/design/01-view-mockups.md` §1.10 (:362-380)
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (Need, Journeys, New UI; its `↺` reads `⇢` per this spec)
- `crates/graft-ops/MAP.md` `## journal` (`in_force_all`, `undoable`), `## journal_list`;
  `crates/graft-ui/MAP.md` `## journal_screen`; `crates/graft/MAP.md` `## view::journal`, `## update::journal`
- The scout's file. Then source, only the files it names.

## Decisions
- **The rule (pure, graft-ops):** `fate_of`, called from `rows::assemble` where a run is attached
  (`journal_list/rows.rs:116-131`; `Event::Graft { entry, .. }` indexes `entries`). A lost run under a `●` event gets
  - `Replaced` when that entry is in force (`in_force_all`), `undoable`, and one of its before-refs pointed at the
    run's tip while its after-state has that ref at another commit;
  - `Deleted` when the same holds but the after-state has no such ref (a branch delete);
  - otherwise `Lost`. A run under a `$` event is always `Lost`. Commits an undo dropped (redo's case) stay `Lost`.
  If `JournalEntry` does not carry both before- and after-refs, stop and report (do not infer from git).
- **Glyph `⇢` (U+21E2), one for both fates; the word says which.** Chosen 2026-10-11 against the namespace:
  `✗` = lost (warning), `↺` = "put back" on the graph (`graph_screen/mod.rs:444,470`, `update/recover.rs:64`,
  `update/undo/lands_on.rs:39`), `→` = the drag drop target on the graph, `◌` = uncommitted, `◆`/`◇` = header.
  `⇢` appears nowhere in `crates/` or `docs/design/` (grep 2026-10-11; one hit in the throwaway
  `prototypes/m8-polish-and-ship/polish.py:1437`, a drag ghost label, not shipped). A dashed arrow, not a cross:
  "moved on, not gone" (memory: different things must look different).
- **No ASCII variant.** J draws `✗` identically in every tier today (scout; `pieces.rs:66`); `⇢` follows suit. J's
  missing ASCII tier is out of scope (reported to the conductor as a gap, not built here).
- **Row (graft-ui `pieces.rs:60-66`, label per `RowKind`):** `└ ⇢ replaced` / `└ ⇢ deleted`, muted ink (an existing
  `Ink` variant; none fits → stop, report), not `Ink::Warning`. Colour unverified (rule 5). The label column keeps its
  width (`└ ⇢ replaced` is 12 cells; today's column is 15).
- **Words (graft `view/journal.rs:240` `lost_words`, mapped at :139-141):** for `Replaced`/`Deleted` the tail is
  `· was <b>'s tip · undo brings it back` (`· was HEAD · undo brings it back` when HEAD held it), in place of
  `· no branch holds it` / `· <b> is gone`; `+N under it` kept where it is today. `Lost` words unchanged.
  Full row: `└ ⇢ replaced  9a8b7c6 "Add rate limiter" · was feature's tip · undo brings it back`.
- **Keys (graft `update/journal/keys.rs`):** `n`/`N` with no find (`next_lost` :127-143) step `ListRow::Lost` rows
  whose `commit.fate == Fate::Lost`, and `LostEdit` as today; `Replaced`/`Deleted` rows are passed over. `Enter` and
  `b` are unchanged: a `⇢` row is still `ListRow::Lost`, so they act as on `✗`. Preselect (`update/journal.rs:102`)
  unchanged: `⇢` rows sit under `●` only, never preselected.
- **Bar:** a `⇢` row's bar is the `✗` row's. Expected with no edit (same `ListRow`). If it differs, the fix is in the
  hint chooser (`graft/src/view/rows.rs`, `graft-ui/src/graph_screen/hints/`), which ticket 47 holds: commit at the
  seam, report `blocked: needs hints`, stop.
- **Legend (graft-ui `journal_screen.rs:58-70`, cut at :412):** clause ` · ⇢ undo brings it back` appended last, and
  only when the whole legend with it fits the width; otherwise today's legend, cut as today. So it goes first and never
  half-shows. Shape private to the builder.
- **Design docs (this ticket edits them; the user decided):** keymap §2.11: the `n`/`N` row reads "next / previous `✗`
  row (`⇢` rows passed over)"; one sentence after the table: "A `⇢ replaced` / `⇢ deleted` row is a commit an undoable
  step left behind; `Enter` and `b` act on it as on `✗`." Mock-up §1.10: one `└ ⇢ replaced` row under a new `●` rebase
  row, and the legend clause. Both: "Amended 2026-10-11 (usability Q1, user: 'yes say replaced')".

## Scope
In: graft-ops `journal_list` (`Fate`, `fate_of`, `LostCommit.fate`, `rows.rs`); graft `view/journal.rs`
(`lost_words`, `RowKind` mapping), `update/journal/keys.rs` (`next_lost`); graft-ui `journal_screen.rs` (legend),
`journal_screen/view.rs` (`RowKind`), `journal_screen/pieces.rs` (label, ink); keymap §2.11; mock-up §1.10;
TestBackend captures of J; MAP lines.
Out: `LostEdit` rows; redo's case; J's ASCII tier; J's source column (`mergewright-git-messages-and-journal-column`);
the pty walk (`mergewright-accept`, S5); the hint chooser files (ticket 47's).

## Signatures
```rust
// graft-ops::journal_list
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate { Lost, Replaced, Deleted }
pub struct LostCommit { pub run: LostRun, pub subject: Option<String>, pub held_by: HeldBy, pub fate: Fate } // + fate
/// Pure. `event`: the `●` entry the run sits under and whether it is in force; `None` for a `$` event.
pub fn fate_of(event: Option<(&graft_core::journal::JournalEntry, bool)>, tip: graft_core::Oid) -> Fate;
// graft-ui::journal_screen
pub enum RowKind { Graft, Outside, Lost, LostEdit, Replaced, Deleted } // + Replaced, Deleted
```
`build`'s signature unchanged. Paths per the scout (`Oid` graft-core/src/oid.rs:22, `JournalEntry`
graft-core/src/journal.rs:262).

## Acceptance (each box: the test named at tick time)
- [x] `journal_list::tests::an_undoable_rebase_replaces_its_old_tip` — built as graft-ops `tests/it/journal_fate.rs::an_undoable_rebase_replaces_its_old_tip` (real repos need `TempRepo`, which lives in `tests/it`): real repo, in-app rebase entry; the old tip's
      row is `Replaced`; "old tip" from `git rev-parse` before the rebase.
- [x] `journal_fate::an_older_in_force_rebase_still_replaced` (+ pure `journal_list::tests::fate::two_rebases_in_one_second_keep_their_own_finish`, see NOTES): two rebases, the first not the next undo: both `Replaced`.
- [x] `journal_fate::an_undone_rebase_leaves_no_replaced_row`: after undo no row names the old tip.
- [x] `journal_fate::a_shell_reset_stays_lost`: `git reset --hard HEAD~1` outside: `Lost` under `$`.
- [x] `journal_fate::a_deleted_branch_reads_deleted`: in-app delete: `Deleted`.
- [x] `journal_list::tests::fate::fetch_and_push_never_replace` (pure, over `fate_of`: Fetch, Push, Migrate, Undo, Redo; a real fetch/push never moves a `refs/heads` ref, so a real repo would pass vacuously). Rule edges: `journal_list::tests::fate::*` (11 tests).
- [x] graft `view::journal::tests::replaced::` tests `replaced_words_say_undo_brings_it_back` (branch, HEAD, `+N` cases) and
      `lost_words_unchanged` (today's three tails).
- [x] `journal_screen::tests::replaced::replaced_row_label_and_ink` (`└ ⇢ replaced`, `└ ⇢ deleted`, not Warning ink; `⇢` is one
      cell by the width fn `cut_at_word` uses), `…::legend_shows_the_replaced_clause_only_when_whole`
      (fits: clause last; one cell short: today's legend exactly).
- [x] graft `update::journal::tests::replaced::` tests `n_passes_over_replaced_rows`, `enter_and_b_on_a_replaced_row_act_as_on_lost`,
      `replaced_row_bar_matches_lost_row_bar`, `preselect_never_lands_on_a_replaced_row`.
- [x] TestBackend captures (`journal_screen::tests::replaced::capture_replaced_for_docs_shots`) (rule 6, his eye; colour unverified, rule 5): `docs/shots/m8-q1-journal-replaced-ui-187x30.txt`
      and `…-ui-80x24.txt`: a `●` rebase row, `└ ⇢ replaced` under it, a `$` reset with `└ ✗ lost` under it; legend
      with the `⇢` clause at 187, without at 80.
- [x] Glyph grep quoted in NOTES: `⇢` in `crates/*/src` and `docs/design/` only at this ticket's sites.

## Touches
- **Callers:** `build`'s callers (graft `view::journal`, the journal job); every `LostCommit {` literal (grep
  workspace-wide, tests included); `next_lost`; `Enter`/`b` dispatch; preselect; `u`/`U` ("never a `$` or `✗` row":
  `⇢` rows are `ListRow::Lost`, unchanged; test above).
- **Screens:** J only (the graph draws no lost rows). Tour: scout grep of `update/tour.rs` found no `✗`/lost; re-run
  and quote.
- **Keymap / help:** §2.11 text; no key added or moved; no action id changed. `?`/palette strings that say "`✗` row"
  for `n`/`N`: grep `"✗` in graft and graft-ui src, update any to match §2.11.
- **Journal / undo:** read only.
- **Captures:** J TestBackend captures in m7 and m8 regenerate under the gates; drift summary quoted,
  `.scratch/m8-polish-and-ship/q1-shot-drift.txt`.
- **Later tickets:** `mergewright-git-messages-and-journal-column` maps `RowKind` in `source_word` (the new variants
  draw as `Lost` does) and edits the legend after this lands; `mergewright-accept` S5 walks `⇢ replaced`.
- **Docs:** keymap §2.11, mock-up §1.10, MAPs (graft-ops `journal_list`, graft-ui `journal_screen` `RowKind`/`legend`,
  graft `view::journal`).
- **Crates:** graft-ops, graft-ui, graft. Files disjoint from ticket 47's (its: `update/picker*`, `view/picker.rs`,
  `view/rows.rs`, `graft-ui` `picker.rs`, `graph_screen/hints/**`).
- Each checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. Capture runs to files; summary lines only.

## Size
One context, ~40k tokens.

## NOTES (builder, 2026-10-11)
- Scout used: `.scratch/m8-polish-and-ship/scouts/46-q1-replaced.md`; no new scout.
- `fate_of` adds `is_operation` to `undoable`: `undoable` alone passes `Undo`/`Redo` (only Fetch/Push/Migrate are
  excluded, `graft-ops/src/journal.rs:231`), so redo's case would have read "replaced". graft-ops MAP said
  `undoable_kind` excluded Undo/Redo; it does not — MAP line corrected.
- A ref absent from `refs_after` is "never recorded", so `Lost`, not `Deleted` (journal.rs doc: absent = unknown).
- **Cause fix outside the listed files (journal_list/steps.rs `fold_one`):** two in-app rebases in one second (his
  chain-rebase case) paired each `HEAD` rebase run with the nearest branch `(finish)` record by index, not by the
  branch the run returned to; the first rebase's old tip was placed under the second rebase and read `✗ lost` (and the
  real-repo test failed 2 of 3 runs). Now the finish must name the run's `returning to` branch. Regression:
  `journal_list::tests::fate::two_rebases_in_one_second_keep_their_own_finish` (red before, green after). Placement of
  shell rebases benefits the same way; all 32 journal integration tests and the oracle still pass.
- At 80 cols the age column (10) is narrower than `└ ⇢ replaced` (12): the words start one blank after the label
  (`pieces.rs` `lost_pieces`), else they overwrote it. `✗` rows unchanged.
- Bar: same `ListRow`, no hint-chooser edit (test `replaced_row_bar_matches_lost_row_bar`).
- Tour: `grep -n "✗\|lost" crates/graft/src/update/tour.rs` → no hits. Help strings: no `"✗ row"` text in graft /
  graft-ui src (grep `✗ row\|next lost`), only the `next_lost` doc comment, updated.
- Captures: drift in `.scratch/m8-polish-and-ship/q1-shot-drift.txt` (9 files: legend clause; one row in
  `m7/journal-after-squash-187x44.txt` now `⇢ replaced` — Graft's squash is undoable). Captions added to
  `docs/shots/m8-contact-sheet-captions.tsv`; the sheet files are the acceptance ticket's to assemble.
- Glyph grep `grep -rn "⇢" crates/*/src docs/design`: graft-ops `journal_list.rs:104` (doc); graft-ui
  `journal_screen.rs` (legend clause + docs), `journal_screen/pieces.rs` (labels), `journal_screen/view.rs`
  (`RowKind` docs), `journal_screen/tests/replaced.rs`; graft `update/journal/keys.rs` (doc), `view/journal.rs` (doc),
  `update/journal/tests/replaced.rs`, `view/journal/tests/replaced.rs`; docs `02-keymap.md` §2.11, `01-view-mockups.md`
  §1.10. No other site.
- File-size ratchet raised (`crates/graft/tests/it/over_400.txt`): journal_screen.rs 490, journal_screen/tests.rs 706,
  update/journal/keys.rs 426, update/journal/tests.rs 1211, view/journal/tests.rs 604, journal_list/steps.rs 425. New
  tests went to new `replaced.rs` files to keep growth small.
- Colour unverified (rule 5): muted vs warning ink is asserted on the `Piece`, not seen.
- Gap for the conductor: J has no ASCII tier (`✗`/`⇢` drawn in every tier).
