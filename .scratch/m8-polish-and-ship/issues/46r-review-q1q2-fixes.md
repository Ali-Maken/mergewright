# 46r: review fixes for group 1 (tickets 46, 47)

Status: done
Blocked by: none (46, 47 merged at 4b5a51b2)
Build with: **Opus**. Journal pairing and undo fate (M2 + minor 6) are undo semantics.
Brownfield: the 46/47 scouts and their NOTES serve as the scout; the review names every line.
Needs: none. Before 48 (48 touches every crate's names).

## Read first
- `docs/reviews/m8-mergewright-g1-q1q2.md`, whole (Major 1–2, Minor 1–7, 9, 10)
- Tickets 46 and 47: Decisions and NOTES

## Scope (from the review; conductor's routing)
- **Major 1:** split `journal_list/steps.rs`, `graft-ui journal_screen.rs` and `graft update/journal/keys.rs` back under
  the 400-line limit instead of raising `over_400.txt`; one `returning to` parser (steps.rs:148-153 vs :306-309).
- **Major 2 (question):** back-to-back rebases with no checkout between fold into one run (steps.rs:131-134). First a
  real-repo test that shows the older tip's fate. Wrong → fix the cause, same test holds it. Right → test stays, say so.
- **Minor 6 (question):** after `u` then `U`, does the old tip read `✗ lost`? Same: test first, fix if wrong.
- **Minor 1:** on the current row the bar reads `⏎ close  esc close`, two keys one word for different actions.
  Conductor's call: `⏎`'s clause on that row uses the same words as `⇥`'s clause (it is `⇥`'s action); `esc close` stays.
- **Minor 2:** `?` help (`graft-ui keymap/table.rs:398`) and the man page: `⏎` switches, or goes back to the graph on the
  current branch. Regenerate the man page with its existing writer.
- **Minor 3, 7, 9:** stale doc comment `graft-ui picker.rs:119`; rewrap long doc lines; rename verb constants.
- **Minor 4, 10:** reword ticket 47 box 8; ticket 47 NOTES say whether picker/hints captures drifted (with the grep/diff).
- **Minor 5:** below 120 columns, every lost row uses the widest label's sha column (`pieces.rs:82-83`).
- Not here: minor 8 (gap M8/47-q2-prior-art), J ASCII tier (gap M8/46-j-no-ascii).

## Acceptance (each box: the test named at tick time)
- [x] `over_400.txt` has no new rows from 46; the parse exists once (grep quoted). Test: `graft::it
      file_size` (exact ratchet). Source rows: `steps.rs` 425 → 318 and `update/journal/keys.rs` 426 → 393
      (both rows removed, ≤400), `journal_screen.rs` 490 → 459 (below its old 474). Tests: `graft::it
      file_size::every_file_over_400_lines_is_listed_with_its_count`, `…::the_size_list_has_no_stale_rows`.
      See NOTES for the three test-file rows and two rows this ticket raised.
- [x] Major 2 real-repo test, named; outcome stated. `graft-ops::it
      journal_fate::back_to_back_rebases_each_replace_their_old_tip`: **confirmed defect**, fixed (cause).
- [x] Minor 6 real-repo test, named; outcome stated. `graft-ops::it
      journal_fate::a_redone_rebase_replaces_its_old_tip_again`: **confirmed defect**, fixed (cause).
- [x] hints test: on the current row `⏎`'s clause equals `⇥`'s; no two clauses share a word. Test: graft-ui
      `graph_screen::hints::tests::typed_fields::picker_on_current_enter_says_what_tab_says`.
- [x] help test: `picker.switch` description names both outcomes; man page regenerated (diff quoted). Test:
      graft-ui `keymap::tests::table::picker_switch_names_both_outcomes`; man page held by
      `graft cli::docs_tests::committed_man_page_is_the_generated_one`.
- [x] journal_screen test: at 80 columns `⇢` and `✗` rows' shas start in the same column. Test: graft-ui
      `journal_screen::tests::replaced::lost_rows_shas_line_up_at_every_width` (80, 119, 120, 187).
- [x] captures re-shot where the above changes them (`docs/shots/m8-q1-*`, `m8-q2-*`), for his eye. Tests:
      `journal_screen::tests::replaced::capture_replaced_for_docs_shots`,
      `update::picker::tests::current_row::capture_the_picker_on_the_current_row`,
      `graft::it m7_walk::journal::the_three_lost_work_journeys` (m7 `journal-100x30.txt` drifted too).

## Touches
Callers of the split modules; journal/undo (Major 2, minor 6); hints, keymap §2.16, help, man; captures; MAP.md files.

## NOTES (builder, 2026-10-12)
No compaction. No scout: the review names every line and 46/47 NOTES served (ticket's Brownfield line).

**Major 2 — confirmed, fixed at the cause.** Test first, red before the fix:
`journal_fate::back_to_back_rebases_each_replace_their_old_tip` (Graft rebase of `feature` onto `main`;
`main` moves by `update-ref -m "fetch: fast-forward"`, which writes no `HEAD` record; Graft rebase again).
`HEAD`'s log is six unbroken `rebase` records (the test asserts it). Before: one run, paired with the
newer finish; the first rebase's old tip sat under the **newer** `●` and read `Lost`
(`fates_of(first) == [Lost]`), though `u` twice brings it back. Fix: `steps.rs` `fold_rebases` ends a run at
a `(start)` record and before a `(finish): returning to` record — one rebase per run. After: both
`Replaced`, each under its own `●`.
The reviewer's exact chain (line rebase, `<top>` ≠ checked-out) does **not** produce that layout in the app:
`journal_fate::a_line_rebase_then_another_each_replace_their_old_tips` shows Graft switches back after a
line rebase (`checkout: moving from two to one`), which ends the run; both old tips read `Replaced`. Not
verified that it held before the fix (the module split made a clean revert costly); its first run failed
only on my wrong expectation (`one`'s old tip is inside `two`'s run, not a run of its own).

**Minor 6 — confirmed, fixed at the cause.** `journal_fate::a_redone_rebase_replaces_its_old_tip_again`
(rebase, `u`, `U`): before, the old tip sat under the `Redo` and read `Lost` (`fate_of` refuses a
non-operation). Fix: `fate.rs` `judged_by` — a `Redo`'s run is judged by its root operation
(`journal::root_operation`), in force only when both are; `rows.rs` calls it. An `Undo`'s runs stay `Lost`
(spec: redo's business), held by `an_undone_rebase_leaves_no_replaced_row`.

**Major 1.** Grep, two-prefix `returning to` parse: `crates/graft-ops/src/journal_list/steps.rs:106-107`
only (`fn returning_to`; `words.rs:82` strips a different, already-cut prefix). Splits:
- `journal_list/owners.rs` (new): `match_entries`, `claim`, `moves_ref`, `switch_names`, `head_name`,
  `CHECKOUT_WINDOW_MS`, moved verbatim with their module doc.
- `graft-ui journal_screen/legend.rs` (new): `legend` (re-exported, same path), `legend_at`, `REPLACED_CLAUSE`.
- `graft update/journal/reading.rs` (new): `switched_from`, `not_undone_kind` (re-exported from
  `update::journal`, same paths).
- Test-file rows from 46 left as they were (`journal_screen/tests.rs` 706, `update/journal/tests.rs` 1211,
  `view/journal/tests.rs` 604): +2/+6/+2 lines of `mod replaced;` and a fixture; the ticket named the three
  source rows only. **Raised by this ticket:** `keymap/table.rs` 512 → 513 (the longer `picker.switch`
  label needed a const; rustfmt splits any `b(…)` whose label passes ~24 chars into 5 lines) and
  `hints/tests.rs` 3135 → 3137 (the short-rung table row below).

**Minor 1 — needs the conductor's ruling.** `⏎` on the current row reads `⏎ show it on the graph`. That made
the picker-on-current floor 47 cells and `no_context_hides_every_key` red at 40 columns. Fix chosen: a
prefix short rung `"show it on the graph" => "show"` in `hint.rs` `short_form` (floor 31), same pattern as
the 21b and m8/06 rows ("proposed by the builder, for the conductor's ruling"). The table is the user's —
revert or replace if not ruled. Side effect: `⇥` in the ordinary picker bar may now draw `⇥ show` at
narrow widths; no committed capture changed (full suite green).
Design note `pass-3-mergewright.md:73` amended (dated) to the new words.

**Minor 2.** Label: `Switch to it; on your own branch, back to the graph`. Man diff:
`-Switch to it (picker.switch)` / `+Switch to it; on your own branch, back to the graph (picker.switch)`,
regenerated with `cargo run -q -p graft -- --generate-man`.

**Minor 3, 7, 9:** `picker.rs:119` drops `you're on it`; `view/journal.rs:244` rewrapped (`fate.rs:33`
rewrapped with the doc edit); `PICKER_SWITCH_WORD`/`PICKER_CLOSE` → `PICKER_SWITCH_VERB`/`PICKER_SHOW_VERB`
(the close verb is gone; `PICKER_SHOW_VERB` now feeds `⇥`'s clause too).

**Minor 4, 10:** ticket 47 box 8 reworded, its hints box marked superseded; NOTES say no picker/hints
capture drifted (`git diff --stat 5191a559~1 5191a559 -- docs/shots`: only the new Q2 capture and one
caption row). The Q2 capture's hostile fixture subject (`Commit 3␛[31m`) is unchanged — not routed here.

**Minor 5.** Below 120 every lost row's words start one blank after the widest label (`└ ⇢ replaced`,
`pieces.rs` `WIDEST_LABEL`). Cost he will see: a page with only `✗` rows now has 3 more blanks before
the sha, and at 80 columns a long `✗` line is cut 3 cells sooner (`…no branch holds…`).

**Captures re-shot, for his eye (colour unverified, rule 5):** `m8-q1-journal-replaced-ui-80x24.txt`
(sha column), `m8-q1-…-187x30.txt` (caption only), `m8-q2-picker-current-row-ui-120x36.txt` (bar),
`m7/journal-100x30.txt` (sha column). Captions updated in `m8-contact-sheet-captions.tsv`.
The m7 contact sheet still shows the old `journal-100x30` (46 left it too; an assembled historical file).

**Seen, not fixed (for the conductor):** after a Graft line rebase, Graft's own switch back
(`checkout: moving from two to one`) draws as a `$` row in J — Graft's step shown as the shell's
(`a_line_rebase_then_another_…` list output). Not claimed by its entry; out of scope.

**Touches:** `rename_names.rs` allowlist path moved with the legend (`journal_screen.rs` →
`journal_screen/legend.rs`; ticket 48 renames these). `update::journal` re-exports keep `view/journal.rs`
and the keymap-words tests' paths (`journal::keys::lost_edit_keys`, `branch_needs_row` stay in `keys.rs`).
graft-ops MAP `fate_of` line notes the redo rule; no public item moved (new modules are private).
Gates (workspace-wide): fmt, clippy `-D warnings`, nextest 6388/6388, doc tests pass.
