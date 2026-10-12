# Mergewright acceptance: his repositories open under the new name with nothing lost, and Q1/Q2 hold

Status: ready
Blocked by: 46, 47, 48, 49, 50, 51, 52, 53, 55, 56
Build with: **Fable**. The acceptance ticket (CLAUDE.md "Running work"); not counted against the Fable build budget.
Brownfield: 28's NOTES ("Scout used", "One counter", the `m8_walk` harness) serve as the scout.
Needs: every other spec of this set: `usability-q1-replaced-when-undoable`, `usability-q2-picker-enter-on-current-closes`,
`mergewright-names-table-and-migration`, `mergewright-package-and-command`, `mergewright-app-words`,
`mergewright-env-names`, `mergewright-git-messages-and-journal-column`, `mergewright-release-and-formula`,
`mergewright-docs-sweep`, `mergewright-recaptures`. (`mergewright-protected-lines` is not needed: its edits wait for
the user; this ticket reports whether they landed.)

## Promise
**Your repositories open under Mergewright with nothing lost**: every backup, journal entry and set-aside edit made
under Graft or Tributary is still his, and `u` still takes back his last real operation. Plus the two decided fixes:
J calls an undoable rewrite's old tip "replaced", and `⏎` on his own branch closes the picker.

## Read first
- `docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md` (Promise, Journeys, Lands)
- `docs/design-notes/m8-polish-and-ship.md`: the `journey-counts` fence only (this ticket adds four rows to it)
- ADR-0057 Amendment (2026-10-11), whole
- `issues/28-accept-m8.md`: Decisions 1–5 and NOTES (the harness: `m8_walk::WALKS`, `within_the_note_taking`,
  `noted_keys`, the contact sheet assembler, `scripts/m8-accept.sh` steps)
- Each spec of this set: its Acceptance section only (to find the test each scenario already has)
- `crates/graft/MAP.md` lib.rs entries for `tests/it/m8_walk`; `crates/graft-ops/MAP.md` test support (`init_on`)

## Decisions
1. **One command:** `scripts/m8-accept.sh` (existing) gains step 6 `mergewright`. Steps 1–5 unchanged; step 3's sheet
   now includes this set's captures; step 4 quotes `brew style` from `docs/release/build.txt`.
2. **Fence rows** (added to the main note's `journey-counts` fence by this ticket, each with its walk in `WALKS`):
   `open-older-names-repo 1`, `undo-after-rename 1`, `journal-replaced 1`, `picker-enter-current 3`, comments as in
   the pass-3 Journeys table. The comments of `open-recent-outside` / `open-second-recent` read `mwr` (docs sweep did
   it; checked here).
3. **Older-generation repositories** are laid out as ticket 1's `init_on(Generation)` does. If that helper lives in
   graft-ops' tests and graft's `it` can't reach it, the walk builds the same layout with plain `git` commands in its
   own helper (refs, dirs, marker), and NOTES says so. Never by running an older build.
4. **Scenarios** (each a pty walk in `tests/it/m8_walk/mergewright.rs` unless named; Unicode tier; 120×36 unless
   stated; the binary is `CARGO_BIN_EXE_mwr`; truth from git and the journal reader, never from the app's words):
   - **S1 Graft-era repo.** 3 backups, a 20-entry journal, a set-aside stash tagged `graft:`, all under Graft names.
     Launch `mwr` there (1 step). Lands: graph, `▸` on HEAD, **no** `rename:` status line. `git for-each-ref
     refs/graft refs/tributary` empty; `refs/mergewright/` holds the 3. `J`: the 20 rows plus one `● Mergewright …
     rename to Mergewright (N refs, M files)`, N and M from the fixture. `Esc`, `u` (1 step): the newest real
     operation is undone byte-exactly (`git rev-parse`, file bytes). The set-aside is offered.
   - **S2 Tributary-era repo** (marker `clean`): as S1; J shows both `rename to Tributary` and `rename to Mergewright`.
   - **S3 Another window open.** S1's repo with a helper holding `<common>/tributary/lock` shared: launch lands on the
     graph with `rename: still using the old names · another Mergewright window is open on this repository`; nothing
     moved (`for-each-ref`). Helper drops the lock; relaunch moves.
   - **S4 Command and manual.** `mwr --version`, `mwr --help` line 1, `man docs/man/mwr.1 | head -3` (or the roff
     NAME line if `man` is absent) quoted in TESTING.md.
   - **S5 Q1.** An in-app rebase of `feature` onto `main`, then `J` (1 step): the old tip's row reads `⇢ replaced … was
     feature's tip · undo brings it back`; a shell `git reset --hard HEAD~1` in the same repo still reads `✗ lost`
     under `$`. 187×30 and 80×24.
   - **S6 Q2.** On `main`: `C`, `main`, `⏎` (3 steps). The picker is gone, `▸` on HEAD, no status line, `git reflog
     -1` unchanged.
   - **S7 What ships.** `docs/release/plan.txt`'s artifact lines and `build.txt`'s `brew style` line quoted; no
     artifact named `graft`/`trib`.
   - **S8 No old name anywhere live.** The grep tests of the set, run and named: `no_old_name_in_user_visible_strings`,
     `no_old_env_name`, `no_old_package_spec`, `no_old_message_literal`, `scripts_run_the_command`,
     `user_docs_and_scripts_name_the_command`, `no_old_dir_literal`.
5. **Context guard:** every run writes to files under `.scratch/m8-polish-and-ship/accept/`; the builder reads summary
   lines and reports paths only, never a capture whole.

## Scope
In: `tests/it/m8_walk/mergewright.rs`, `WALKS` entries, the fence rows, `scripts/m8-accept.sh` step 6, TESTING.md
"M8 · Mergewright" section, `docs/shots/m8-mergewright-*.txt`, the M8 contact sheet re-assembled. Out: product code
(a walk that finds a defect stops and reports it, rule 2); the perf box of 28 (the user's terminal run); the whole-app
usability walk (reviewer, after this).

## Signatures
None in product code. Test-side: `m8_walk::mergewright::{older_names_repo_opens_and_undoes, tributary_repo_opens,
another_window_keeps_old_names, journal_replaced, picker_enter_current}`, each through `within_the_note_taking`.

## Acceptance (each box: the test named at tick time)
- [ ] `m8_walk::every_m8_journey_id_has_a_walk` green with the 4 new ids; `m8_walk::one_key_over_the_note_fails`
      green; each new walk named in `WALKS`.
- [ ] S1–S3 green (tests named); their truth lines (for-each-ref, journal counts, rev-parse) quoted in NOTES.
- [ ] S4, S7 outputs quoted in TESTING.md.
- [ ] S5, S6 green (tests named).
- [ ] S8: each grep test named and green.
- [ ] Captures (rule 6, his eye): `docs/shots/m8-mergewright-s1-graph-120x36.txt`, `…-s1-journal-120x36.txt`,
      `…-s3-busy-120x36.txt`, `m8-q1-journal-replaced-{187x30,80x24}.txt`, `m8-q2-picker-enter-current-120x36.txt`;
      each ≤ 200 KB; colour unverified (rule 5).
- [ ] TESTING.md "M8 · Mergewright": one row per scenario saying concretely what right looks like, e.g. S1 "graph
      on HEAD, no `rename:` line; J has 21 rows, the newest `● Mergewright … rename to Mergewright (3 refs, M files)`;
      `u` restores the file to its bytes before the 20th entry". Never "looks correct".
- [ ] M8 contact sheet re-assembled (ADR-0037), every capture under its line; parts ≤ 200 KB:
      `m8_walk::contact_sheet::every_part_of_the_m8_sheet_opens_with_colour_unverified` green.
- [ ] `scripts/m8-accept.sh` (no `--perf`) exit 0; summary lines quoted; step 6's summary
      `mergewright: 8 scenarios, 0 failed`.
- [ ] NOTES: whether `mergewright-protected-lines` was applied (and the user's words), else "waiting for the user".

## Touches
- **Callers:** `m8_walk` harness, `m5_walk::parse_counts`, `scripts/m8-accept.sh`, the contact-sheet assembler.
- **Screens:** none changed; walked: graph, J, picker, status row.
- **Keymap / journal / undo:** none changed; `u` and J exercised.
- **Captures:** the new files above; the M8 sheet.
- **Docs:** the main note's fence, TESTING.md.
- **Crates:** graft tests only.
- Each is checked by the named test or file in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide; then `scripts/m8-accept.sh`. All output to files; summary lines only.

## Size
One context, ~60k tokens.

## Added by the conductor (review 53, 2026-10-12)
- Re-run `scripts/release-local.sh` after 55; the archive's README/CHANGELOG name Mergewright (quote `tar xOf … | grep`).
  Local only: no tag, push or release.
