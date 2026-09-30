# 30p: the put-back's `u` for every path kind, a staged path's words, J find's cost, four minors (review m7-30m-30o M1, M2, Minor 2, 3, 6, 7, Q1, Q3)

Status: ready
Build with (four parts; **a, b, c concurrent**: disjoint file sets, three builders in graft at most (ADR-0051); **d only if the
conductor's release run of b reads OVER**):
- **30p-a: Opus.** It holds the M3 promise over every path kind the put-back now writes over. It is real-git tests with a
  stop condition, plus one graft-ops field and one landing. Nothing structural; the tests judge it, so not Fable.
- **30p-b: Opus.** A new `frames` scenario and its fixture sizing. The harness (`KeyFeed`, `Gate`, the shared `Wake`
  channel) is subtle, and the scenario must not break `scripts/m7-perf-key.sh`'s parse.
- **30p-c: Sonnet.** Four local fixes, each pinned by a named test or grep.
- **30p-d (conditional): Opus.** J's find computes its hits once per find and once per list rebuild. It is dispatched
  only on an OVER verdict in `docs/perf/30p-journal-find.txt`.
Blocked by: nothing; 30o merged (01636c0f).
Scout: none for a, c (the review names every site; re-grep by function, the review's lines are 01636c0f's). b: the
builder reads `tests/it/frames/*.rs` and `tests/it/frame_trace.rs` itself (graft MAP "Frame trace").
Needs: no signature from another ticket.

Source: `docs/reviews/m7-30m-30o.md` M1 (:72), M2 (:113), Minor 2, 3, 6, 7 (:161-189), Q1 (:193), Q3 (:202); ticket 30o R1
and NOTES 30o-a; graft-ops MAP `snapshot::worktree_tree`, `journal_list::lost_edits`; graft MAP "Frame trace",
`update::journal::find`; ADR-0021.

## Rulings (planner, 2026-09-30; not open)

**R1 (M1): `u` after a put-back is proved byte-exact for every path kind the put-back writes over or deletes.**
- The mechanism is unchanged. The entry takes a before-snapshot (`add -A` on an index copy; MAP
  `snapshot::worktree_tree`), and `u` is `RestoreWorktree` over `ChangedBetween(after, before)`. The review's
  "by reading" is not an observed run (rule 1), so eight real-git tests observe it (Acceptance, 30p-a).
- **Any test red → stop that path kind.** Do not refuse the kind. Do not change the snapshot. Add no set-aside, copy,
  retry or `#[ignore]`. Leave the red test **uncommitted**; its body and failing lines go in NOTES. Report
  `blocked: the before-snapshot does not hold <kind>`. The planner then decides between refusing that kind in `check`
  (as the ignored path is refused, `InTheWay`) and fixing the snapshot. R2 depends on test 1: if test 1 is red, R2 stops
  too. R3 and 30p-b/c/d go on.
- Planner's expectation, **unverified, not to act on:** test 6 (an LF file under `core.autocrlf=true`) may fail. git's
  own clean-then-smudge writes CRLF there. Whether that is lost work is the planner's call after the run.

**R2 (Q3): a staged path stays staged, and the box says so; the landing is on the row that shows the lost edit.**
- **Decision:** the put-back keeps git `restore`'s default and writes the working tree only (`Touches { index: false }`,
  unchanged). His staged copy stays in the index.
  - The box adds one clause after `replaced`: `src/a.rs stays staged as you had it`. With several paths:
    `src/a.rs +N more stay staged as you had them`. The paths are those in `replaced` whose index differs from `HEAD`.
  - After `y`, the changes pane lands on that path's **`UNSTAGED`** row. That row's diff (the working tree against the
    index) is the lost edit. The `STAGED` row's diff is his staged copy.
- **Why:**
  - H5, show the result before confirming: `what's in src/a.rs now is replaced` alone reads as if his staged work goes
    too. It does not, so the box says so.
  - H3, user control: his staging is a separate, deliberate change, and the put-back does not undo it.
  - §3.1, land where the result is: the `UNSTAGED` row shows the edit he asked for, and his next key (stage it) works on
    that row.
  - §3.6, different things look different: the two versions sit in two sections, as `assemble_working_tree` already
    draws a file that is both staged and unstaged.
- **Rejected:**
  - "Write the index too" (`--staged --worktree`). The entry would then touch the index, which widens its inverse. It
    would also discard his staging decision (H3). git's `restore` defaults to the worktree for this reason.
  - "Refuse a staged path". He would have to unstage first: +2 steps on journey 15 (fence `J n ⏎ y` = 4), for a backup
    the snapshot already takes (§3.5).
- **For the conductor** (the note's "Where he lands" row from 30o R1): append `; a staged path stays staged, and the
  pane lands on its UNSTAGED row`.

**R3 (Q1): the content a put-back replaces is never a J `✗` row. This is intended.**
- **Decision:** `lost_edits` keeps scanning `Undo`/`Redo` entries only.
- **Why:**
  - `✗` means work an undo overwrote **without asking**: his drift under a forced undo. A put-back's replaced copy was
    named in the box before `y`, and he chose it. That is the same class as a discard, whose copy is not a `✗` row
    either (H4, consistency).
  - §3.6: if chosen and unchosen losses shared `✗`, the glyph would stop telling them apart.
  - H8: every put-back would add a `✗` row for what it replaced, and putting that back would add another. J would fill
    with his own choices.
  - The copy stays reachable: `u` while the put-back is next, and after that the put-back's own `●` row.
  - No field note asks for the recursive case. If he hits it, it goes to `docs/field-notes.md` and the next design pass.
- **Pinned** by a graft-ops test (30p-a). The test also closes 30o-a's unverified "the lost-edit row is gone after the
  put-back" (NOTES 30o-a).

**R4 (M2): measure first; fix only on an OVER verdict.**
- 30p-b writes a `frames` scenario: J over at least 2 000 rows with a broad find kept. It also writes a script. The
  release run is the **conductor's**, from `scripts/m7-30p-perf.sh` into `docs/perf/30p-journal-find.txt`. It never
  runs in a builder's context (CLAUDE.md, Gates).
- `Gate` (p99 ≤ 16 ms, ADR-0021's 16.7 ms frame) is the verdict.
  - `ok`: M2 closes with the perf file as evidence, and 30p-d is not dispatched.
  - `OVER`: 30p-d is dispatched, with this scenario as its red test.

**R5 (Minor 7): the m3 J captures show a settled J.**
- The fixture answers stages 2 and 3 **empty** through J's real answer functions (`outside_landed`, `lost_landed`), not
  by setting fields. The commands it returns are asserted.
- The title then reads exactly what the capture's own "Judge by eye" line says: `JOURNAL · what happened in this repo,
  newest first`.
- Why: a capture the user judges must show the steady state. A transient tail that the header only half explains is
  noise (H8). The m3 captures are about the journal's own marks, not the stages.

## Part 30p-a: `u` after every kind of put-back (R1), a staged path (R2), no row for the replaced copy (R3)

### Files (exact; outside this list → stop, `blocked: needs <file>`)
- **graft-ops:**
  - `tests/it/recover.rs` (the premise tests' file; reuse its `GraftOps`).
  - The file that builds `FilesEffect::Changes` for a `Recover` preview (`preview/recover.rs`, `changed_now`), and the
    file defining `FilesEffect` (grep `enum FilesEffect`).
  - `tests/it/lost_work.rs` or `tests/it/recover.rs` for R3's test (whichever holds `lost_edits` tests; grep).
  - `MAP.md`: `FilesEffect`, `journal_list::lost_edits` (R3's line).
- **graft:**
  - `update/recover.rs` (`files_words`) and its tests.
  - The function that lands a `Recover` `Files` put-back on the changes pane (grep the `your edit to` done words or
    `Recover` in `update/operate*`/`update/recover.rs`). Only its path-row choice.
  - `tests/it/journal_put_back.rs` (R2's walk and captures).
  - `MAP.md` (`update::recover`, and the landing function's entry).
- **graft-fixtures: none.** Build each case inside the test, on `lost_work` as built.
- **Docs:**
  - New `docs/shots/m7/put-back-staged-preview-187x50.txt` and `docs/shots/m7/put-back-staged-landed-187x50.txt`.
  - `docs/shots/m7/m7-contact-sheet-captions.tsv`: two rows after `put-back-edit-landed-187x50.txt`.
  - `docs/shots/m7/m7-contact-sheet.txt`, rebuilt with `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`
    (relative paths; 30o-b NOTES).
  - `TESTING.md` "M7: the journal, J": two rows after "`y` on the edit's box (journey 15 lands)".

### Signatures
```rust
// graft-ops — FilesEffect::Changes gains one field; the others are unchanged.
FilesEffect::Changes { /* come_back, go, set_aside, replaced as today */ still_staged: Vec<PathBuf> }
//   Recover Files: the paths in `replaced` whose index entry differs from HEAD (staged), from the same facts
//   `changed_now` reads. It is a subset of `replaced`. Every other builder of Changes: `still_staged: Vec::new()`.
// graft::update::recover — signature unchanged; one clause (R2), order: come back · go · replaced · still staged · set aside.
pub(crate) fn files_words(effect: &FilesEffect) -> String;
```

### Acceptance (name the verifying test at tick time; **(red)** = write it, see it fail on today's code, then build)
- [ ] **R1, the eight premise tests** — **7 green, test 6 red (R1's stop; not ticked)**: see NOTES 30p-a. Green: `recover::u_after_a_put_back_over_a_staged_file_brings_back_both`, `u_brings_back_a_file_a_put_back_deleted`, `u_brings_back_the_exec_bit_a_put_back_replaced`, `u_brings_back_a_symlink_a_put_back_replaced`, `u_brings_back_a_crlf_file_under_autocrlf`, `u_brings_back_a_file_under_a_clean_smudge_filter`, `u_after_a_put_back_that_recreates_a_deleted_file_removes_it`. (graft-ops `tests/it/recover.rs`, real git). Each one runs a put-back, then `u`,
  through the same path as `u_after_a_file_put_back_brings_back_what_it_replaced`, and asserts with **byte-exact**
  reads (`std::fs::read`, `symlink_metadata`, `read_link`, `git ls-files -s`, `git show :<path>`). Not red-first: each
  tests an existing mechanism. A red one is R1's stop, not a build step.
  1. `u_after_a_put_back_over_a_staged_file_brings_back_both`: index `S` ≠ `HEAD`, worktree `W` ≠ `S`. After the
     put-back, the worktree is `LOST_EDIT` and the index is still `S`. After `u`, the worktree is `W` and the index `S`.
  2. `u_brings_back_a_file_a_put_back_deleted`: the `go` direction. The lost edit's backup lacks the path (his drift was
     a deletion, which the forced undo re-created). He then writes `W` there. The preview's `go` holds the path. After
     the put-back the path is absent; after `u` it is `W`.
  3. `u_brings_back_the_exec_bit_a_put_back_replaced`: his file is `W` with mode `0o755`, and the backup's is `0o644`.
     After `u`: `W`, with the exec bit set.
  4. `u_brings_back_a_symlink_a_put_back_replaced`: his path is a symlink to `b.rs`. After `u` it is a symlink, and
     `read_link` gives `b.rs`.
  5. `u_brings_back_a_crlf_file_under_autocrlf`: `core.autocrlf=true`, and his file is `W` with CRLF endings. After `u`:
     the same bytes.
  6. `u_brings_back_an_lf_file_under_autocrlf`: `core.autocrlf=true`, and his file is `W` with LF endings only. After
     `u`: the same bytes.
  7. `u_brings_back_a_file_under_a_clean_smudge_filter`: in `.git/info/attributes`, `src/a.rs filter=rot`, with
     `filter.rot.clean` and `filter.rot.smudge` both `tr 'A-Za-z' 'N-ZA-Mn-za-m'`. After `u`: `W` byte-exact.
  8. `u_after_a_put_back_that_recreates_a_deleted_file_removes_it`: a tracked path deleted on disk, which the put-back
     re-creates. After `u` it is absent again, and the index is unchanged throughout.
  - NOTES: one line per test, `green` or `red: <first failing line>`.
- [x] **R2, the field:** `still_staged_names_the_staged_paths` (graft-ops `tests/it/recover.rs`, green; red by compile first) (red, graft-ops): for test 1's state,
  `replaced == [src/a.rs]` and `still_staged == [src/a.rs]`. For the unstaged `lost_work` state, `still_staged` is
  empty.
- [x] **R2, the words** (graft `update/recover.rs` tests): `files_words_says_a_staged_path_stays_staged` seen red (tests.rs:301), then green; `files_words_names_what_is_replaced`, `files_words_joins_the_clauses_that_apply` green.
  - `files_words_says_a_staged_path_stays_staged` (red). One path:
    `… · what's in src/a.rs now is replaced · src/a.rs stays staged as you had it`. Three paths:
    `src/a.rs +2 more stay staged as you had them`.
  - Green: `files_words_names_what_is_replaced`, `files_words_joins_the_clauses_that_apply`.
- [x] **R2, the walk** `a_staged_path_put_back_lands_on_its_unstaged_row` (green; landing green on today's code, NOTES) (graft `tests/it/journal_put_back.rs`, real
  git). Setup: `lost_work` as built, then test 1's state. `J n ⏎ y`, with `Route::within_count` ≤ 4. Red or green on
  today's code for the landing; say which in NOTES. Asserted **from the frame**:
  - After `⏎`: the box holds `src/a.rs stays staged as you had it`.
  - After `y`:
    - J is closed (no `JOURNAL ·` row).
    - The pane holds `STAGED (1)` and `UNSTAGED (1)`.
    - The diff on screen holds the lost edit's line (`2`), and not a line that exists only in `S`'s diff against
      `HEAD`. Name both lines in NOTES.
  - From state as well: `selected_entry()`'s section is `Unstaged`.
  - After `u` (uncounted): `src/a.rs` is `W` and `git show :src/a.rs` is `S`.
- [x] **R2, the captures** (written by `journal_put_back::a_staged_path_put_back_lands_on_its_unstaged_row`; 3.7 KB, 2.3 KB; TSV + TESTING rows; sheet rebuilt, 131 KB) (written by the walk, `check_shot`, each ≤ 200 KB):
  - `put-back-staged-preview-187x50.txt`. **Right:** the box titled `Preview: put back your edit to src/a.rs`, with the
    line `your files: src/a.rs comes back · what's in src/a.rs now is replaced · src/a.rs stays staged as you had it`,
    then `u undoes it`. The footer starts `y put it back`.
  - `put-back-staged-landed-187x50.txt`. **Right:** the changes pane shows `STAGED (1)` and `UNSTAGED (1)`, each with
    `M a.rs`. The diff shown is the lost edit's (`2`). The bottom row is `your edit to src/a.rs is back ✓ · u undoes`.
    The cursor row is colour only (30o-a NOTES).
  - The TSV rows and TESTING rows say exactly those Right lines, plus the test name and "Colour unverified (rule 5)".
    The sheet is rebuilt.
- [x] **R3** `a_put_back_leaves_no_lost_edit_row_and_u_brings_it_back` (graft-ops `recover.rs`, green; mutation checked, NOTES) (graft-ops, real git). After the put-back over
  the changed file, `lost_edits` is empty: the lost edit's row is gone, and no row names the replaced copy. After `u`,
  `lost_edits` holds the one lost edit again. Not red-first: it pins R3. By mutation, a `lost_edits` that also scans
  `Recover` entries fails it; say so.
- [x] **Unchanged:** (`m7_walk::journal::the_three_lost_work_journeys`, `the_put_back_box_draws_over_j` green in the full nextest run, no `GRAFT_UPDATE_SHOTS`; `git status docs/shots` shows only the two new files and the sheet) `put-back-edit-preview-187x50.txt` and `put-back-preview-187x50.txt` are byte-identical
  (`the_three_lost_work_journeys`, `the_put_back_box_draws_over_j` green; walk 15's path is unstaged).
- [x] MAP.md (graft-ops: `FilesEffect`, `lost_edits`; graft: `update::recover`, `lands_on_changes`). Gates workspace-wide (NOTES).

### Touches (check each; say how on the **Touches:** line)
- **`FilesEffect::Changes`:** every constructor and pattern. Per review Minor 5 these are `preview/recover.rs`
  `changed_now` and the `Vec::new()` site, and graft `update/recover.rs`. Grep `FilesEffect::Changes` in both crates.
- **The landing function:** its other callers. A put-back of a commit lands on the graph, so walk 14 and
  `the_three_lost_work_journeys` must stay green. So must an unstaged path's landing (walk 15, unchanged capture).
- **Journal/undo:** no new operation. `Touches { index: false, worktree: true }` is unchanged (grep it in
  `operation/recover.rs`). `u` is the generic inverse.
- **`lost_edits`:** unchanged code (R3). The only change is its MAP line.
- **Keymap:** none. **Design note:** R2's clause is the conductor's.

## Part 30p-b: J's find, measured (R4, M2)

### Files (exact)
- **graft-fixtures:** `synthetic` (the `with_reflog` module). Add `with_reflog_of(scale, records, lost)`; `with_reflog`
  becomes `with_reflog_of(scale, RECORDS, LOST)`, with its output unchanged. Plus `MAP.md`.
- **graft:**
  - New `tests/it/frames/journal_find.rs`, and the one line registering it (grep where `frames`' children are
    declared).
  - `tests/it/frame_trace.rs` only if `Gate` needs a label it lacks (say why in NOTES).
  - `MAP.md` ("Frame trace": the new scenario).
- **scripts:** new `scripts/m7-30p-perf.sh`.
- **Docs: none.** The builder does not write `docs/perf/`; the conductor's run does.

### Signatures
```rust
// graft-fixtures::synthetic (heavy-tests)
/// `records` scripted reflog records (on HEAD and three branches, as with_reflog's), `lost` of whose olds nothing
/// holds. with_reflog(scale) == with_reflog_of(scale, RECORDS, LOST).
pub fn with_reflog_of(scale: &Scale, records: usize, lost: usize) -> Result<Reflogged, /* with_reflog's error */>;
// graft tests/it/frames/journal_find.rs (heavy-tests, as the other frames scenarios)
#[test] fn j_with_a_broad_find_holds_the_frame_budget();
```

### The scenario
- **Fixture:** `with_reflog_of(<small scale>, 2_000, 500)` (`RECORDS_CAP`, `LOST_CAP`). J's cost does not depend on
  the graph's size, so pick the smallest `Scale` whose cold build stays under ~30 s, and name it.
- **Terminal:** 187×50, Unicode tier (his daily view).
- **Before measuring**, wait until `finding_lost()` is false, using the harness's `pump_until_from`. Then check the
  premises. **A premise failure fails the test**; it is never skipped:
  - rows ≥ 2 000, of which `✗` rows ≥ 400;
  - the query (chosen by the builder from the fixture's scripted subjects, and named) has ≥ 1 500 hits;
  - after `/<query>⏎`, the frame's title ends `· find "<query>" 1 of <n>`, with n ≥ 1 500.
- **Keys** (`KeyFeed`, paced as the existing scenarios pace; all of them measured): `J`, then the wait above, then `/`,
  the query, `⏎`, `n`×40, `N`×40, `j`×40.
- **Verdict:** `Gate`, as the other scenarios use it. Its `frames:` line carries `tier=`, `refs=<rows>` and
  `pane=JournalFind` (or the harness's existing equivalents), so `scripts/m7-perf-key.sh`'s regex
  (`tier=\S+ refs=\S+ pane=\S+`, `p50_us`/`p99_us`/`max_us`) parses it. `FrameTrace` lines carry every field that
  script's Python reads (`kind`, `latency_us`, `pane`, `key`, …).
- **The script** `scripts/m7-30p-perf.sh`, modelled on `m7-perf-key.sh`:
  - A release build with `--features heavy-tests`, then 3 runs of `frames::journal_find` only, with
    `--test-threads=1`.
  - It writes `docs/perf/30p-journal-find.txt` (or `$1`): a header (date, HEAD, machine, git), then per run a
    `SUMMARY run<k> rows=<n> hits=<h> p50_us=… p99_us=… max_us=… exit=<s>`, then
    `SUMMARY journal-find gate(p99<=16ms)=ok|OVER|FAIL(<run> failed)`.
  - It exits non-zero unless every run passed and the gate is `ok`. `$HOME` is replaced with `~`.

### Acceptance
- [x] `with_reflog` unchanged: its existing callers' tests stay green under `--features heavy-tests` compile (grep
  `with_reflog(`), and `with_reflog_of_counts_what_it_is_given` (heavy tier; the builder does **not** run it, the
  conductor's review does) asserts the record and lost counts. — callers (grep: `graft-git/examples/lost_cost.rs:69`,
  `stash_only_cost.rs:61`; no test calls it) compile under `cargo clippy --all-targets --features heavy-tests`;
  unchanged output pinned by `with_reflog_splits_its_records_as_before` (`split(RECORDS) == LOGS`, run: ok) and
  `with_reflog_of_refuses_more_lost_than_head_records` (gap stays `LOST_EVERY` for `RECORDS, LOST`, run: ok);
  `with_reflog_of_counts_what_it_is_given` written in `synthetic/reflog.rs` tests, **not run** (conductor's).
- [x] The scenario and the script exist as specified. `cargo clippy --all-targets --features heavy-tests -- -D warnings`
  is green (this compiles the scenario). — `frames::journal_find::j_with_a_broad_find_holds_the_frame_budget`,
  `scripts/m7-30p-perf.sh`; clippy heavy green (workspace). Two deviations, NOTES 30p-b.
- [x] **Smoke, not a measurement:** one **debug** run of `frames::journal_find` only. Its output goes to a file in the
  worktree's scratch; read only the `test result` line and any panic line.
  - A panic from `Gate` in debug is not a finding (ADR-0021: budgets fail only under `--release`). Report no numbers.
  - A premise panic is a finding: fix the fixture or the query.
  - Say in NOTES which one it was. — `j_with_a_broad_find_holds_the_frame_budget` debug: first run a premise panic
    (fixed, NOTES), then `test result: ok. 1 passed`.
- [x] `scripts/m7-perf-key-selftest.sh` is green, and it is unchanged. In NOTES, quote the new scenario's `frames:`
  line format against `m7-perf-key.sh`'s regex. — selftest: "every check held"; script not in the diff. Format in NOTES.
- [x] MAP.md (graft-fixtures, graft). Gates workspace-wide (fast tier). — both MAPs; fmt, clippy (fast and heavy),
  doctests green; nextest 4916/4917, the one red `published_repo::nothing_a_commit_would_carry_names_this_machine`
  is `docs/reviews/m7-30m-30o.md:16` (committed c14ce973, not this part's file), NOTES 30p-b.

### Touches
- **`scripts/m7-perf-key.sh`** runs every `frames::` test, so it will run this one from now on. Its parse must hold
  (the check above), and its runtime grows by one fixture build. Say by roughly how much, from the smoke's wall time.
- **`with_reflog`'s callers** (m7/19, `lost_cost` example, `m7-perf-lost.sh`): their output is unchanged, because the
  counts are the same constants.
- **Nothing in `src/`.** The scenario reads state through the harness only.

## Part 30p-c: four minors (review Minor 2, 3, 6, 7)

### Files (exact)
- **graft:**
  - `tests/it/m7_walk/journal.rs` (walk 15's asserts only).
  - `tests/it/journal_screen.rs` (one rename).
  - `src/view/tests.rs` (`journal_state`, `capture_m3_journal`).
  - `MAP.md` if it names the renamed test (grep).
- **graft-ui:** `src/graph_screen/hints/lists.rs` (the comment above `HintContext::Journal`'s entry only).
- **Docs:** `docs/shots/m3-journal-{80x24,160x48,expanded}.txt` and `docs/shots/m3-contact-sheet.txt`, rebuilt with
  `scripts/contact-sheet.sh m3 docs/shots` (relative paths).

### Acceptance
- [x] **Minor 2.** (`m7_walk::journal::the_three_lost_work_journeys` via `j_closed_and_pane_shown`; mutation: J forced open on journey 15's landed state → red at journal.rs:396 "J is closed in the frame"; restored) In `journey_15`, beside the state asserts, add asserts from the frame after `y`:
  - no row holds `JOURNAL ·`;
  - the frame holds the pane's header `UNCOMMITTED · 1 unstaged` and `UNSTAGED (1)` (as in
    `put-back-edit-landed-187x50.txt`).
  - Not red-first. Check by mutation: make `confirm` skip J's close, see the new assert fail, then restore. Say so.
- [x] **Minor 3.** (grep: 0 old name in `crates/`, `TESTING.md`; MAP.md never named it) `esc_while_typing_puts_the_cursor_back` is renamed `esc_while_typing_closes_the_prompt_only`, with
  the body unchanged. Grep: 0 of the old name in `crates/` and `TESTING.md`. Ticket NOTES are history; leave them.
- [x] **Minor 6.** (`the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` green in full nextest) The comment reads, in substance: this entry is the hint catalogue's `Journal` slot; J draws its own
  bar (`journal_screen`), which also answers `b`, `/`, `n`, `N`, `u`, `U`, `space`.
  `the_bar_behind_an_overlay_draws_no_key_the_overlay_swallows` is green.
- [x] **Minor 7 (R5).** (`view::tests::capture_m3_journal`; red seen first at tests.rs:2900 before the stages were answered; grep 0 `finding lost commits` in `docs/shots/m3-*`; sheet 131445 B)
  - `capture_m3_journal` answers stage 2 with an empty `Ready` through `outside_landed`, asserts the `FindLost` command
    it returns, and answers that with an empty `LostFound` through `lost_landed`.
  - (red on today's fixture) the frame's title row holds no `finding lost commits`.
  - The three m3 J captures are re-shot. Their header drops "the shell's steps are still being read (the title ends
    finding lost commits…)" and says the stages were answered empty, so the title is settled.
  - Grep: 0 `finding lost commits` in `docs/shots/m3-*`. The m3 sheet is rebuilt, ≤ 200 KB. Diff the title lines in
    NOTES.
- [x] Gates workspace-wide. (fmt, clippy, doc clean; nextest 4916/4917 — the 1 red is `published_repo` on `docs/reviews/m7-30m-30o.md:16`, pre-existing on 7c1ee6e7, outside 30p-c)

### Touches
- **Other J captures:** m3-08 and m4-07 are pty walks over a real repo, not this fixture, so they are unchanged. Check
  with a full nextest run and `git status` over `docs/shots`.
- **`journal_state`'s other users** (grep): every one stays green.

### NOTES 30p-c (builder, 2026-09-30)
- No scout (ticket: none for c). No compaction.
- Minor 7 answered in `journal_state` (its only user is `capture_m3_journal`, grep: 2 calls). Title diff, all three captures:
  `- JOURNAL · what happened in this repo, newest first · finding lost commits…` → `+ JOURNAL · what happened in this repo, newest first`.
- Touches: m3-08/m4-07 unchanged (`git status docs/shots`: only the four m3 J files); hint comment only, no key change.
- Pre-existing red, not mine: `published_repo::nothing_a_commit_would_carry_names_this_machine` → `docs/reviews/m7-30m-30o.md:16` home path. Fix: `cargo run -q -p graft-fixtures -- redact docs/reviews/m7-30m-30o.md`.
- Colour unverified (captures record none).

## Part 30p-d (conditional, only on OVER): J's hits computed once per find and once per rebuild

### Files (exact)
- **graft:** `update/journal/find.rs` + tests; `JournalState`'s file (the `find` field, `rebuild`); `view/journal.rs`
  (the `count` call); `MAP.md` (`update::journal::find`).

### Signatures
```rust
// graft::update::journal::find
pub(crate) struct JournalFind {
    /* typing, kept as today */
    /// Row indices into `journal.list` that hold `kept`, newest first. Rebuilt by `refresh_hits` only.
    pub(crate) hits: Vec<usize>,
}
/// Rows whose naming words hold `query`, as list indices. The `●` descriptions are looked up through one
/// HashMap<&EntryId, &str> built per call, never a scan per row.
pub(crate) fn hits(journal: &JournalState, query: &str) -> Vec<usize>;
/// Recompute `find.hits` from `kept`. Called by `find_key` on ⏎ and by `rebuild` when a find is kept; nowhere else.
pub(crate) fn refresh_hits(journal: &mut JournalState);
// count (at, of) reads `hits`: `of = hits.len()`, `at` by `partition_point` on the selected index. No list scan per frame.
```

### Acceptance
- [x] **Red:** the conductor's OVER line in `docs/perf/30p-journal-find.txt`, cited: `:33 SUMMARY journal-find
  gate(p99<=16ms)=OVER` (runs `:30-32`: p50 2.3–2.4 s, p99 3.7–3.8 s, 2425 rows, 2000 hits; 121 of 127 keys over,
  from key 6 `Enter` on). Verifying test: `frames::journal_find::j_with_a_broad_find_holds_the_frame_budget`.
- [x] Green, named (nextest, 4961/4961): `update::journal::find::tests::{hits_find_a_lost_subject_a_branch_and_a_path,
  hits_skip_verdict_words, a_blank_find_has_no_hits}` (now assert list indices), `journal_screen::{a_find_survives_rows_arriving,
  enter_keeps_the_find_and_n_steps_the_hits, u_typed_into_the_find_is_text}` (unchanged).
- [x] `count_reads_the_cached_hits` (red seen on today's `count`, at its `"the cached hits, not the list"` assert: left `Some((1, 0))`, right
  `Some((1, 2))`; green after).
- [x] Grep: `grep -n "index_of\|kept_hits" crates/graft/src/update/journal/find.rs` → no match (exit 1); `kept_hits` is
  gone.
- [ ] The conductor re-runs `scripts/m7-30p-perf.sh`: `gate … =ok`. It is the conductor's run, not the builder's.

### Touches
- **The view:** `view/journal.rs` reads `count` only. `journal-find-187x50.txt` is byte-identical.
- **Stages 2 and 3:** `rebuild` refreshes the hits. `a_find_survives_rows_arriving` is green.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` and `cargo test --doc`, all
workspace-wide. Part b adds `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
- **30p-a** ~55k. If it compacts: R1 first, then R3, then R2.
- **30p-b** ~50k.
- **30p-c** ~30k.
- **30p-d** ~35k.

If any part compacts, say so; the conductor splits the rest.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. No perf or release run in
a builder's context. The design note row (R2) and any KNOWN_GAPS row are the conductor's:
- R1 all green: no gap row; the review's filter/CRLF row is not needed.
- R1 any red: a **broken** row naming the kind.

## NOTES (builder)

### 30p-b (builder, Opus, 2026-09-30)
- **No scout**: the ticket names the harness files for the builder to read; one scout for J's rows/find (pub API).
- **Smoke:** first debug run was a **premise** panic, not the Gate: the title read `find "step" 2 of 2000`. Cause, by
  design: once stage 3 is in, J's cursor starts on the newest `✗` row under a `$` row (`update/journal.rs:95-110`
  `preselect`), and `⏎` finds the first hit at or below it (`update/journal/find.rs:214`) → `2`, not `1`. The premise
  now accepts `<at> of <n>` with `1 <= at <= n` and n ≥ 1 500 (`hits_in`, comment cites both lines). Second run:
  `test result: ok`. No numbers reported (debug).
- **Premises observed:** 2 425 rows, 425 `✗`, 2 000 hits (`/step`). Only 425 of 500 lost show: `find_lost` checks the
  newest `LOST_CAP` candidates, and HEAD's slots 400-499 tie in time with the three branches' records.
- **Rows are counted by the functions J's jobs run** (`read_outside`/`find_lost`/`build` with `RECORDS_CAP`/`LOST_CAP`,
  on the same repo), not from J's state: `JournalState.list` is `pub(crate)`. Hits come from the drawn title of the
  loop's own state (replayed).
- **Harness:** two pumps (J; then `/step⏎ n×40 N×40 j×40`), the stage 2-3 wait between via `pump_until_from`, all 127
  keys in one `Gate`. `run::scroll`/`ready_to_scroll`/`capabilities` are private to `run.rs` (not in the file list), so
  the scenario opens via `budget_run::open_into` and carries its own Unicode `Capabilities` copy. The rows-on-hand
  (`waited`) check is not applied (graph rows, not J's). Resize to 187×50 is an `update` before the ready pump.
- **Deviations:** `with_reflog_of(scale: Scale, …)` by value, as `with_reflog` (ticket wrote `&Scale`); new
  `SyntheticError::TooManyLost` in `synthetic.rs` (lost > HEAD's records; no existing variant fits).
- **frames: line** (format): `frames: tier=Unicode refs=<rows> pane=JournalFind control=<c> keys=127 p50_us=… p99_us=…
  max_us=… over_budget=… hits=<h> lost_rows=<x>`; `m7-perf-key.sh`'s `fields()` gives tier/refs/pane and
  p50/p99/max (checked by running its regex on the smoke line). Trace lines (`GRAFT_FRAME_TRACE`, traced smoke run)
  carry every field its Python reads, plus `phase=open|find`; all `kind=scroll`.
- **Script:** in release the test panics on an over-budget p99 after its line; `m7-30p-perf.sh` counts that exit as
  `OVER` (line > 16 ms and the gate's own panic text), any other failure `FAIL`. Summariser checked on canned
  outputs (ok / OVER / OVER-by-panic / FAIL / no line). The release run itself is the conductor's.
- **Touches:** `m7-perf-key.sh` now runs this test in each of its 6 runs: +1 fixture open + ~1 min per run in debug
  (smoke test time 52-65 s, first including the 10k fixture's cold build); release unmeasured. Its OVER handling
  (a panicked plain run → FAIL) is unchanged. `with_reflog` callers: output unchanged (split test). `src/`: untouched.
- **Pre-existing red, not this part:** `published_repo` flags `docs/reviews/m7-30m-30o.md:16` (a home path).

### 30p-a (2026-09-30, builder Opus; no scout: the ticket names every site, re-grepped by function)

**R1 — 7 green, 1 red. `blocked: the before-snapshot does not hold an LF file under core.autocrlf=true`.**
- 1 `u_after_a_put_back_over_a_staged_file_brings_back_both`: green.
- 2 `u_brings_back_a_file_a_put_back_deleted`: green (preview `go == [a.txt]`, `replaced == [a.txt]`).
- 3 `u_brings_back_the_exec_bit_a_put_back_replaced`: green.
- 4 `u_brings_back_a_symlink_a_put_back_replaced`: green.
- 5 `u_brings_back_a_crlf_file_under_autocrlf`: green.
- 6 `u_brings_back_an_lf_file_under_autocrlf`: **red: `crates/graft-ops/tests/it/recover.rs:781:5`** (the final
  `assert_eq!(disk(..), bytes)`). The put-back ran, `u` ran **unforced with no drift**, and the index was the same
  before/between/after (`put_back_then_u`'s asserts passed). Only the bytes differ: after `u`, `src/a.rs` is
  `pub fn a() -> u32 {\r\n    4\r\n}\r\n`; he had `…{\n    4\n}\n`. As the planner expected: the snapshot's `add -A`
  cleans LF to the same blob CRLF would give, and `u`'s restore smudges it to CRLF. Left out of the committed file
  (a comment marks its place); per R1 no fix, no refusal, no `#[ignore]`. Body, to restore it verbatim:
  ```rust
  /// 6. `W` with LF endings only under `core.autocrlf=true`.
  #[test]
  fn u_brings_back_an_lf_file_under_autocrlf() {
      u_under_autocrlf_brings_back(WORKING);
  }
  ```
  (`u_under_autocrlf_brings_back` and `WORKING` are committed; test 5 uses the helper.)
- 7 `u_brings_back_a_file_under_a_clean_smudge_filter`: green.
- 8 `u_after_a_put_back_that_recreates_a_deleted_file_removes_it`: green (index equal before/between/after).
- Tests 3–8 share `put_back_then_u`, which also asserts `git ls-files -s` unchanged across the put-back and `u`.
- The op is read from `lost_edits` right after the fixture builds, before each test changes `src/a.rs`, so the kind
  under test never decides whether the row is found.

**R2.** Test 1 green, so R2 went on. `still_staged` = `changed_now(paths, facts, StatusEntry::is_staged)` — the same
status facts `replaced` reads (`preview/recover.rs`). Every other `Changes` builder gives `Vec::new()`.
- The words' clause is `listed(still_staged, "stays staged as you had it", "stay staged as you had them")`, between
  replaced and set aside.
- **The landing was already right on today's code**: `banner::lands_on_changes` sets `KeptCursor.section = Unstaged`
  (`update/banner/stop.rs:222`). No landing change was made. Mutation: set it to `Staged` → the walk fails at
  `journal_put_back.rs:446` ("the diff is the lost edit's"); restored.
- Walk's two diff lines: shown `2 +     2` (the lost edit, asserted `+     2`); absent `+     3` (S's line against
  HEAD; the unstaged diff draws it as `-     3`).
- **Setup deviation, say so:** on the fixture as built, J preselects D's `✗` row and `n` goes to E, not the edit
  (first run failed at `journal_put_back.rs:376`, frame showed `▸` on E). Walk 15 has 13 and 14 done first. The walk
  therefore stands in for them by the shell (`git merge --ff-only D`, `git branch spike E`, stamped END+30 s), so its
  J shows two `$ … now` rows on top. Their put-backs through the app need the pinned-clock child
  (`the_three_lost_work_journeys`), which is outside this file set. After that, the lost edit is the only `✗` row and
  preselected; `n` stays on it. Route `J n ⏎ y` = 4 (`within_count`).

**R3.** Mutation: `lost_edits`' filter made `Undo | Redo | Recover` (`journal_list/edits.rs:56`) → the test fails at
`recover.rs:884` (the "no row" assert after the put-back); restored (`git checkout` on that file).

**Touches:** `FilesEffect::Changes` — grep in both crates: `preview/recover.rs` (2), graft `update/recover.rs`
(destructure), `update/recover/tests.rs` (3), graft-ops `tests/it/recover.rs`; all compile with the field. The
landing function is unchanged (one caller, `operate/outcome.rs:187`). `Touches { index: false }` unchanged
(`operation/recover.rs:267`). `lost_edits` code unchanged. No key bound.

**For the conductor:** R1 test 6 red → the planner's call (refuse in `check` vs fix the snapshot) and a KNOWN_GAPS
**broken** row. R2's design-note row is yours.


### 30p-d (2026-09-30, builder Opus; no scout agent: a short read of the three named files, as the dispatch asked)
- **Where the per-key cost was (today's code, df0596af):** `count` (`find.rs:112-118`) ran `kept_hits` every frame
  (`view/journal.rs:73`); `kept_hits` (`find.rs:99-107`) ran `hits` (a full list scan, `to_lowercase` per name, and for
  each `●` row a scan of every entry, `find.rs:64-68`) and then `index_of` per hit (`journal.rs:81-83`, O(rows) with a
  `RowKey` clone — a refname `String` — per compare): ~2000 × ~1200 clones per frame. `⏎` (`find.rs:213`) and `n`/`N`
  (`find.rs:225`) ran `kept_hits` again. Typing a char does no hit work (the 5 typing keys are the 6 under budget),
  and the view's only find call is `count`. So the spec's signatures fix it; not `blocked: spec`.
- **Why the latency climbs per key** (73 ms → 700 ms by key 21, perf file): keys are sent a tick apart and each frame
  took ~40 ms, so keys queue behind each other; the constant per-frame cost above is the cause, the climb its backlog.
  This is reasoning from the file's numbers, **not observed** in a trace.
- **Built:** `JournalFind.hits: Vec<usize>`; `hits` returns list indices and looks `●` descriptions up in one
  `HashMap<&EntryId, &str>` per call; `refresh_hits` called by `⏎`, `Esc` over a kept find (clears) and both `rebuild`
  paths (`list = None` too); `count` = `partition_point` on the cached hits; `n`/`N` read the cache. Left per frame:
  `selected()`'s one `index_of` (O(rows), one key) — unchanged, outside this part.
- **Not verified here:** the release gate. Conductor re-runs `scripts/m7-30p-perf.sh`.
- `view/journal.rs` needed no edit (reads `count` only). `journal-find-187x50.txt` unchanged in `git status` after
  nextest (`m7_walk::journal::journal_find_capture` green).
