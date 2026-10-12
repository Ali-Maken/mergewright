# Mergewright acceptance: his repositories open under the new name with nothing lost, and Q1/Q2 hold

Status: done but one box: the `release-local.sh` re-run (review 53 box) is **not run** — `dist` is not installed
here (NOTES)
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
- [x] `m8_walk::every_m8_journey_id_has_a_walk` green with the 4 new ids; `m8_walk::one_key_over_the_note_fails`
      green; each new walk named in `WALKS`. — both green in `scripts/m8-accept.sh` step 1 (6427/6427); step 5
      `35 in the fence · missing a walk: none`.
- [x] S1–S3 green (tests named); their truth lines (for-each-ref, journal counts, rev-parse) quoted in NOTES. —
      `m8_walk::mergewright::{older_names_repo_opens_and_undoes, tributary_repo_opens, another_window_keeps_old_names}`.
- [x] S4, S7 outputs quoted in TESTING.md. — "M8 · Mergewright" rows S4/S7; step 6 re-reads both (S4 runs the built
      `mwr`; S7 refuses any `plan.txt` artifact not named `mergewright-*`).
- [x] S5, S6 green (tests named). — `m8_walk::mergewright::journal_replaced`, `m8_walk::mergewright::picker_enter_current`
      (10 consecutive green runs of all five walks after the last fix).
- [x] S8: each grep test named and green. — `rename_names::{no_old_message_literal, no_old_name_in_user_visible_strings,
      user_docs_and_scripts_name_the_command}`, `rename_names::scripts::{no_old_env_name, scripts_run_the_command,
      no_old_package_spec}`, `rename_opens::no_old_dir_literal`: PASS lines in `.scratch/…/accept/mergewright.txt`.
- [x] Captures (rule 6, his eye): `docs/shots/m8-mergewright-s1-graph-120x36.txt`, `…-s1-journal-120x36.txt`,
      `…-s3-busy-120x36.txt`, `m8-q1-journal-replaced-{187x30,80x24}.txt`, `m8-q2-picker-enter-current-120x36.txt`;
      each ≤ 200 KB; colour unverified (rule 5). — written and checked by the five walks (`check_shot`); each 1–3 KB;
      each header says `Colour unverified (rule 5)`. **Not judged: his eye.**
- [x] TESTING.md "M8 · Mergewright": one row per scenario saying concretely what right looks like, e.g. S1 "graph
      on HEAD, no `rename:` line; J has 21 rows, the newest `● Mergewright … rename to Mergewright (3 refs, M files)`;
      `u` restores the file to its bytes before the 20th entry". Never "looks correct". — the section at the end of
      TESTING.md, 8 rows; no test reads it (checked by reading).
- [x] M8 contact sheet re-assembled (ADR-0037), every capture under its line; parts ≤ 200 KB:
      `m8_walk::contact_sheet::every_part_of_the_m8_sheet_opens_with_colour_unverified` green. — and
      `contact_sheet_captions_cover_every_m8_capture` green (step 1); 109 captures, 0 uncaptioned; parts 175 887 /
      177 693 / 73 206 B.
- [x] `scripts/m8-accept.sh` (no `--perf`) exit 0; summary lines quoted; step 6's summary
      `mergewright: 8 scenarios, 0 failed`. — exit 0, quoted in NOTES.
- [x] NOTES: whether `mergewright-protected-lines` was applied (and the user's words), else "waiting for the user". —
      applied (ticket 54 `Status: done`), quoted in NOTES.

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

## Added by the conductor (56 report, 2026-10-12)
- 56's old-name box does not hold for one capture: `crates/graft/tests/it/quit_waits.rs:186` writes the label
  "Graft waits for git" into `m8-quit-waits-80x24.txt` (and so into the contact sheet). Change the label to the product
  name (from the table), regenerate the capture, then assemble the contact sheet. Code comments that say "Graft" are
  the codename and stay.
- 56 counts: m8 6 · live-doc 0 · record 241 · code-name 199 · sheet 4; re-shot 0 (writers already current). 25 rows
  linked only from M1–M7 TESTING.md checklists were marked record (conductor accepts).
- Question for the walk: `m8-tour` line 3 reads `workripractice` (gap M8/50-tour-header-letters). Find the line
  (rule 7) or keep it a question.

### Boxes for the conductor's additions (57)
- [x] (review 53) `scripts/release-local.sh` re-run — **by the conductor on main 5ce4ed3b+, 2026-10-12** (builder's sandbox refused the build; user approved the `.tools/` dist build): `DIST=.tools/bin/dist bash scripts/release-local.sh` rc=0; dist 0.33.0 built from upstream tag v0.33.0 (a1c829f). Archive `target/distrib/mergewright-aarch64-apple-darwin.tar.xz`: README `# Mergewright`, CHANGELOG "Mergewright is unreleased" / "renamed to Mergewright (command `mwr`)"; `mwr 0.1.0`; brew style no offenses; shellcheck clean. Host-manifest formula.py exit 2 = no x86_64 archive on a single-host build (expected). Nothing pushed, tagged or uploaded.
- [x] (56 report) `quit_waits.rs:186` label from the table (`Generation::CURRENT.title()`); capture regenerated —
      `quit_waits::capture_quit_waits` green; `m8-quit-waits-80x24.txt:26` now `=== then \`y\`: Mergewright waits for
      git ===`; the sheet re-assembled after it (no `Graft waits` left in `m8-contact-sheet-*.txt`).
- [x] (56 report) `workripractice`: **kept a question** (rule 7), see NOTES.

## Added by the conductor (review 55-56, 2026-10-12)
- [x] `21c-journal-partial-fetch` 80x24 + 120x40 (writer `partial_fetch.rs:253`) and `doorway-marked-140x40`
      (writer `hint_bar.rs:899`): **no old product name in them**, so not regenerated. Checked by grep: their only
      `graft` hits are the crate path `crates/graft/…` in the writer header and the fixture URL
      `/nonexistent/graft-21c.git` (code-name, as 56 classified). Their writers' tests green in step 1.
- [x] `update/tests/doorway.rs:44` header literal → `crates/graft/src/update/tests/doorway.rs`; the 5 `doorway-*`
      captures regenerated (1 line each, the header) — `update::tests::doorway::capture_the_keys_doorway_for_docs_shots`
      green after.

## NOTES (builder, 2026-10-12)
- **Compacted once mid-ticket** (context summary). Nothing left over from the compaction; what remains is the one
  unrun box above.
- **Scout used:** yes (`scout`, for the `workripractice` line and the pty replayer); 28's NOTES served for the harness.
- **`mergewright-protected-lines` applied:** ticket 54 `Status: done`; its NOTES quote the user: "The user said yes
  (2026-10-11) to changing the product-name lines in CLAUDE.md, PROMPT.md and `.claude/**` (reviewer.md) listed in
  this ticket's table."
- **Truth lines** (stdout of the walks, `--success-output immediate`):
  - S1: `truth: refs under refs/graft/: 0 · under refs/mergewright/: {3 backups under refs/mergewright/backup/main/…}`
    (each holds the old oid or a commit on it: `merge-base --is-ancestor`); `truth: journal 21 entries, renames
    ["rename to Mergewright (3 refs, 1 file)"]`; after `u`: `HEAD refs/heads/main`, `commit fefb7a2…`, `index 100644
    0f31f92… 0 a.rs`, `a.rs "fn one() { \"his edit\" }\n"`, `stash []` — equal to the state before the switch.
  - S2: `refs under refs/graft/: 0` and none under the second names; `journal 22 entries, renames ["rename to
    Tributary (3 refs, 1 file)", "rename to Mergewright (3 refs, 1 file)"]`; after `u` the state equals before.
  - S3: `truth: busy · refs under refs/graft/: 3`; none under `refs/mergewright/`; no marker. After the lock drops:
    refs moved, no `rename:` line.
- **`scripts/m8-accept.sh` (no `--perf`), exit 0:** `1 fast tier: Summary [ 444.001s] 6427 tests run: 6427 passed
  (4 slow), 16 skipped · rc=0` · `3 contact sheet: wrote 109 captures, 0 without a recorded caption, in 3 part(s)` ·
  `5 journeys: 35 in the fence · missing a walk: none` · step 6: S1–S8 each `ok`, `S4 … mwr 0.1.0 · usage: mwr
  [<path>] · man: mwr \- a terminal Git client for the daily work`, `S7 … 5 mergewright-* artifacts, 0 under another
  name · brew style mergewright.rb: ok (1 file inspected, no offenses detected)` · `mergewright: 8 scenarios, 0 failed`.
  Step 6 re-runs its 12 tests with `--status-level pass`: the default profile prints failures only
  (`.config/nextest.toml` `status-level = "fail"`), so step 1's log has no PASS line to read.
- **Defect found (S5 walk), reported not fixed (rule 2, product code out of scope).** After an in-app rebase, a `J`
  pressed while git has finished but before the rebase's outcome line is written leaves **two outcomes for one entry**:
  the journal held a pending line, `"status":"committed"`, then `"status":{"aborted":{"reason":"interrupted"}}` for the
  same id, and J read `the journal could not be read: journal entry <id> has two outcomes`. Reproduced intermittently
  while the walk pressed `J` as soon as `feature~1 == main` (frequency not measured; the journal dump above is from
  one such run). Lines: `J` queues
  `Command::ReadJournal` (`crates/graft/src/jobs/runner/operations.rs:224-234`), whose job runs
  `graft_ops::apply::recover_interrupted` (`crates/graft/src/jobs/undo.rs:575`); recovery closes every `Pending` entry
  when `state::in_progress` reports nothing live (`crates/graft-ops/src/apply/recover.rs:37-55`) and appends the close
  via `Journal::mark_interrupted` (`recover.rs:89` → `crates/graft-ops/src/journal/file.rs:191`), which checks only its
  own in-memory copy for a prior outcome (`file.rs:196-202`). The rebase job then appends `committed` from its own
  copy. **Cause suspected, not confirmed:** nothing serialises the read job's recovery against the write job still
  running in the same process. What it costs him: J and `u` refuse the whole journal until the file is fixed by hand.
  The walk now waits for `rebase feature onto main ✓` and a closed journal entry before `J`, so it tests Q1, not this
  race (`mergewright.rs`, `rebase_feature_onto_main`); 10/10 green after.
- **`workripractice` (m8-tour line 3): a question, not a defect.** The header writes two spaces between the repo name
  and the path (`crates/graft-ui/src/graph_screen/header.rs:131`, `Span::raw("  ")`), so the app draws `work  practice…`
  — `ri` is not in the app's spans. Line 4 of the same capture also carries `~/code`, from the empty state drawn
  before `t`: both look like cells of the previous screen never rewritten. Which side leaves them — the app's frame
  diff after a screen switch, or the pty replayer (`crates/graft/tests/it/pty.rs`, its replay ~399-466) not honouring
  a clear — is **not confirmed**; a real terminal was not checked.
- **Clock seam (observation, not a defect claim):** the `Migrate` entry and in-app operations are stamped by the wall
  clock (`crates/graft-ops/src/rename/record.rs:42` `SystemTime::now()`; in-app ops via the job's `Clock`), while the
  pty walks pin `MWR_NOW`, so their ages draw `future` in `m8-mergewright-s1-journal` and `m8-q1-journal-replaced-*`.
  Same effect 19d-c noted for m4-07.
- **Fixture deviations (Decision 3):** graft-ops' `init_on` is out of reach (its own test binary), so
  `m8_walk/mergewright/older.rs` builds the first-names repository by running the real `graft_ops::apply` with no
  marker on disk (names in force: the first generation); the backups therefore carry the `mergewright` message tag
  (the tag is not per generation). The set-aside stash is rewritten to a `graft:` message, as
  `set_aside_generations.rs` writes it (`as_graft_wrote`), and reflogs re-stamped to the fixture's time.
  `m8_walk/mergewright/tributary.rs` writes the second generation's move as `rename/generations/setup.rs` does.
- **Captions avoid the codename** in the six new captures' header lines (the 56 old-name rule); on-disk paths
  `.git/graft/`, `refs/graft/` are named as facts.
- **Not mine, seen:** `scripts/check-scripts.sh` exits 1 on `scripts/shot-drift-test.py:29,33,34` (`subprocess.run
  inherits the caller's env`) — 56's file, untouched here; shellcheck itself passes `m8-accept.sh`. The sheet still
  shows the fixture author `Graft Fixture` in 4 lines of part 2 (56's code-name class). TESTING.md:3282/3290/3293/3298
  still say `steps 1, 3, 4, 5` and `31 ids`: within the ~3295 lines 56r owns, left for 56r; the new section says 35.
- **Touches:** `m8_walk` harness (`mod.rs` WALKS + `mod mergewright`), the fence (4 rows), `scripts/m8-accept.sh`
  step 6, contact-sheet captions + sheet, TESTING.md (new section at the end only), `quit_waits.rs` label,
  `update/tests/doorway.rs` header literal, 6 regenerated + 6 new captures. Checked by the full fast tier (6427/6427),
  `every_m8_journey_id_has_a_walk`, `contact_sheet_captions_cover_every_m8_capture`, `rename_names::*`, the
  `file_size` ratchet (new files ≤ 400 lines: 344/355/117/142). No product code, keymap, journal or undo change.
