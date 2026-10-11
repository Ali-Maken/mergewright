# Usability Q1: in J, a commit an undoable operation left behind reads "replaced", not "✗ lost"

Status: ready
Blocked by: none
Build with: **Opus**. It reads undo semantics (which entries are in force and undoable), and a wrong rule would tell
him real damage is safe. Judged by tests that build the journal and git state, so not Fable.
Brownfield: a **scout first** (ADR-0035), one page: where `build` attaches a lost run to its event, where the row's
words and glyph are made (graft-ops `journal_list::words`? graft `view::journal`? graft-ui `journal_screen`), the
ASCII glyph for `✗`, what `n`/`N` step, and where the legend sheds.
Needs: none (journal API as built). Beside: `usability-q2-picker-enter-on-current-closes` (disjoint files except
`docs/design/02-keymap.md`, different sections; the conductor merges). Both land before the Mergewright set.

## Read first
- `docs/reviews/m8-usability.md` Q1 (:155-158); `docs/field-notes.md:97` ("yes say replaced")
- `docs/design/02-keymap.md` §2.11 (journal keys, preselect rule); `docs/design/01-view-mockups.md` §1.10 (:362-380)
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (Need, Journeys, New UI)
- `crates/graft-ops/MAP.md` `## journal` (`in_force_all`, `undoable`), `## journal_list`;
  `crates/graft-ui/MAP.md` `## journal_screen`; `crates/graft/MAP.md` `## view::journal`
- `docs/research/reflog.md` (J's prior-art study) takeaways only
- The scout's file. Then source, only the files it names.

## Decisions
- **The rule (pure, in graft-ops `build`):** a lost run under a `●` event gets fate
  - `Replaced` when that entry is in force (`in_force_all`), `undoable`, and one of its before-refs pointed at the
    run's tip while its after-state still has that ref at another commit;
  - `Deleted` when the same holds but the after-state has no such ref (a branch delete);
  - otherwise `Lost`. A run under a `$` event is always `Lost`. Commits an undo dropped (redo's case) stay `Lost`:
    outside his decision.
- **Row:** `└ ↺ replaced  9a8b7c6 "Add rate limiter" · was feature's tip · undo brings it back`
  (`deleted` for `Deleted`; `· was HEAD` when HEAD held it). Muted ink, not the warning ink `✗` uses (colour
  unverified, rule 5). ASCII tier: `~` in place of `↺`. **Before using either glyph**, grep `↺` and the ASCII
  candidate across `crates/*/src` and `docs/design/`; any other meaning → stop, report (a glyph is a shared namespace).
- **Keys:** `Enter` and `b` act on a `↺` row exactly as on `✗` (keymap §2.11). `n`/`N` with no find step `✗` rows
  only; `↺` rows are passed over. Preselect unchanged (a `✗` under `●` is never preselected; `↺` never is either).
- **Legend:** gains `↺ undo brings it back`, after the `✗` clause, and shed **first** when the row is short.
- **Design docs (this ticket edits them; the user decided):** keymap §2.11 gains one sentence for `↺` rows (Enter/b as
  `✗`; `n`/`N` pass over) and the n/N row says "`✗` row"; mock-up §1.10 gains one `↺ replaced` row under a `●`
  rebase row and the legend clause. "Amended 2026-10-11 (usability Q1, user: 'yes say replaced')".

## Scope
In: graft-ops `journal_list` fate; graft `view::journal` mapping; graft-ui `journal_screen` row kind, words, glyph,
legend; `n`/`N` stepping in graft `update` (journal); keymap §2.11 and mock-up §1.10 lines; TestBackend captures of J.
Out: `LostEdit` rows; redo's case; J's source column (Mergewright set).

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
`build`'s signature is unchanged; it calls `fate_of`. If the scout shows `Oid`/`JournalEntry` live at other paths, use
those and say so in NOTES.

## Acceptance (each box: the test named at tick time)
- [ ] `journal_list::tests::an_undoable_rebase_replaces_its_old_tip` — a real repo, an in-app rebase entry: the old
      tip's row is `Replaced`; truth for "old tip" from `git rev-parse` before the rebase.
- [ ] `…::an_older_in_force_rebase_still_replaced` — two rebases, the first not the next undo: both `Replaced`.
- [ ] `…::an_undone_rebase_leaves_no_replaced_row` — after undo, no row names the old tip (it is on the branch again).
- [ ] `…::a_shell_reset_stays_lost` — `git reset --hard HEAD~1` outside: `✗ lost` under `$`.
- [ ] `…::a_deleted_branch_reads_deleted` — in-app delete: `Deleted`.
- [ ] `…::fetch_and_push_never_replace` — non-undoable kinds: `Lost`.
- [ ] `journal_screen::tests::replaced_row_words_and_glyph` (Unicode and ASCII), `…::legend_sheds_the_replaced_clause_first`.
- [ ] `update journal tests::n_passes_over_replaced_rows`, `…::enter_and_b_on_a_replaced_row_act_as_on_lost`.
- [ ] pty walk `m8_walk::journal_replaced` (counts asserted: rebase via the app, then `J` = 1 key; the row reads
      `↺ replaced`; cursor on the newest row): captures `docs/shots/m8-q1-journal-replaced-187x30.txt` and
      `…-80x24.txt` (rule 6, his eye; colour unverified, rule 5).
- [ ] Glyph grep output quoted (no other meaning of `↺` or the ASCII glyph).

## Touches
- **Callers:** `build`'s callers (graft view::journal, the journal job); `n`/`N`; `Enter`/`b` row dispatch; preselect.
- **Screens:** J only (the graph draws no lost rows); the tour, if a step reads J's `✗` (scout checks `update::tour`).
- **Keymap:** §2.11 text; no key added or moved. Help (`?`) rows for J if they describe `n`/`N` ("next `✗` row").
- **Journal / undo:** read only; no entry written or changed.
- **Captures:** J captures in m7 and m8 walks (TestBackend regenerate under the gates; drift summary quoted,
  `.scratch/m8-polish-and-ship/q1-shot-drift.txt`).
- **Docs:** keymap §2.11, mock-up §1.10, MAPs (graft-ops journal_list, graft-ui journal_screen).
- **Crates:** graft-ops, graft-ui, graft.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. Capture runs to files; summary lines only.

## Size
One context, ~40k tokens after the scout.
