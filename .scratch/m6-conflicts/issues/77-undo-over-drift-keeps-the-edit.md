# undo-over-drift-keeps-the-edit: the drift box names the edit it replaces, and one key brings it back

Status: done
From: KNOWN_GAPS **M6/q4** (:812, broken, Major against M3 "you cannot lose work"); `docs/reviews/m6-usability-final.md`
Q4 (:269-270); ticket 69 NOTES (:233-238) and its pin `crates/graft/tests/it/undo_over_drift.rs`
`undo_over_drift_then_undo_again`.
Build with: **Opus (builder-opus-xhigh)**: a new graft-ops read in the undo path and a count across the time machine's
steps, against M3's promise. Correctness is judged by byte-exact tests on real repos, so the work does not need Fable.
Needs: none (69 merged; this extends its pin).
Crates: **graft-ops** (`undo::Drift`, `run_plan`, the forced run's result), **graft** (`jobs::undo`, `update::undo`, the
box's lines). **graft-ui: none.** The 4-line cap is met by graft's words (Decided 6). If it cannot be met:
`blocked: needs graft-ui`.
Scout: `.scratch/m6-conflicts/scouts/undo-over-drift-keeps-the-edit.md` (base `5285a7b6`).

## Read first
- `docs/design/06-undo-journal-design.md` §6.3 (snapshot), §6.4's last two paragraphs, §6.5 (redo), §6.7. ADR-0009.
- `crates/graft-ops/MAP.md` `## undo` (:103-118), `## restore` `execute` (:187), `## undo (execution half)` (:191-196),
  `## journal` `next_undo`/`next_redo` (:95-98), `## snapshot` `index_paths_changed` (:138).
- `crates/graft/MAP.md` `## update::undo` (:277-290), `ConfirmThen` (:263), `Command::Undo/Redo` bool (:194).
- The scout (whole). Ticket 69 NOTES :233-238. `docs/shots/m3-09-undo-drift-box-80x24.txt` (the box today).
- `docs/research/checkout-cherry-pick.md` §1 (the JetBrains and GitHub Desktop rows), §4 ("Dirty tree").

## What is wrong (scout lines at `5285a7b6`)
The box (`m3-09-undo-drift-box-80x24.txt` :10-17) reads **`A backup is taken first, so this is itself undoable.`**
(`graft/src/update/undo.rs:500` time machine, `:527` undo/redo). It names no edit (UX §3.2, H9), and "undoable" reads
as `u`, which answers `nothing to undo` (`undo_over_drift.rs:86-90`). The real way back is `U`. **Scout ran it:** after
`u y`, `U` gave `redo: checkout fix`, `HEAD` = `refs/heads/fix`, and `fix.txt` came back byte-exact as an unstaged edit.
Why: the Redo's `RestoreWorktree` reads the Undo entry's own `worktree_before`, snapshotted at `y` (`graft-ops/src/
undo.rs:316-321`, `restore.rs:222-242`). The staged case was **not run**; it is a failing-test-first below.

## Decided here
1. **The box's promise:** *what the undo replaces is named here, a backup keeps it, and one key brings it back.*
   Today: it names nothing; the backup ✓ (69's pin); the key ✓ for `U` (scout, unstaged case), but the words point at `u`.
2. **Not a design error: a way back missing from the screen.** `y` replacing drift is the design (§6.7: the rail warns,
   the snapshot guarantees; §6.5: undo is undoable). `y` stays. The edit waits in the Undo entry's backup and `U` puts
   it back.
   **Rejected: carrying the edit over the undo.** That is JetBrains' Smart Checkout (study §1). Carrying a change to a
   tree without the file is a modify/delete clash, the lost-work failure users report in GitHub Desktop (study §4).
3. **What counts as replaced:** a path the inverse writes whose content now differs from the entry's recorded
   after-state. Edits to other paths stay in the tree and are not named. If `Drift::unknown` has the working tree, the
   whole tree counts as replaced.
4. **Where it is read, and what it costs.** `Drift` holds tree oids only (`undo.rs:559-580`, `:649-684`), so naming
   paths needs a new read. It goes in **`run_plan`, right after `drift`** (`execute.rs:162`), and **only when the drift is
   not clean**. A clean undo pays nothing.
   The read is plumbing over trees that already exist; nothing new is hashed or written:
   - `git diff-tree -r --name-only -z <entry worktree_after> <now worktree_tree>`;
   - the same for the plan's `PathScope::ChangedBetween { from, to }`;
   - the intersection of the two sets;
   - the index likewise, through `snapshot::index_paths_changed` (`IndexState` shapes).

   That is ≤ 4 `diff-tree` spawns, a few ms each, paid on the drift path only. Before `y` the box needs them (unforced
   run); after `y` the forced run does them again. That is correct if the tree changed between the two, and it costs
   nothing new in kind. `Everything` scope, or no after-state: `whole_tree = true`, no diff.
5. **The time machine.** Drift is checked per step at run time. `force` reaches every step. Unforced drift on step
   k > 0 returns `PartWay` (scout Q4). So the box only ever comes from **step 0**, and it names what step 0 replaces.
   The forced run returns the **union over steps** and `presses = total − (first step that replaced anything)`. The
   landing names the union and the presses.
   Known limit: a later step can replace a path the box did not name. The landing names it and it stays backed up.
   The conductor adds a KNOWN_GAPS row: "time-machine box names step 0's replaced paths only".
6. **Words**:

| Where | Case | Words |
|---|---|---|
| box, consequence line (replaces `…itself undoable.`) | one path | `Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.` |
| | n paths | `Undo replaces your edits to 3 files; a backup keeps them, and U brings them back.` |
| | whole tree | `Undo replaces the whole working tree; a backup keeps it, and U brings it back.` |
| | nothing replaced, tree/index drifted | `Undo leaves your edits since as they are.` |
| | nothing replaced, refs/`HEAD`/merge only | `A backup is taken first, and U brings this state back.` |
| | redo box | `Redo …` and **`u`** |
| | time machine, total > 1 | `… and U (N times) brings it back.` (N = total) |
| status after `y` (`Done` + keys) | undo | `undo: checkout fix ✓ · U brings back your edit to fix.txt` (n: `your edits to 3 files`) |
| | redo | `redo: … ✓ · u brings back your edit to fix.txt` |
| | time machine | today's `Done` words `· U (N times) brings back …` (N = `presses`) |
| `u`, nothing to undo, a redo waiting | any | `nothing to undo · U redoes checkout fix` (the root operation's description) |

   The title, the "what drifted" line and the command slot `undo restores the recorded state` are unchanged. When
   nothing was replaced, the status gets no suffix.
   **Fit (graft's job):** at 80×24 the box has 65 inner columns and `MAX_MESSAGE_LINES = 4` (`graft-ui/src/
   confirm.rs:31`). Today the drift line wraps to 2 (capture :11-12), which leaves the consequence line **2 lines**.
   graft left-truncates the path (`…/fix.txt`) until the sentence wraps to ≤ 2 lines at the box's width, so graft-ui's
   `…` never cuts it. If the drift line itself takes 3 lines (a long title), the consequence line drops `a backup keeps
   it, and ` and keeps the key. If that still does not fit: stop, `blocked: needs graft-ui`.
7. **No ADR.** This fulfils §6.5/§6.7 and ADR-0009 as written, and it is reversible. One sentence for DESIGN §6.7
   (conductor, Proposed): *"the box names the paths the forced run replaces and the key that brings them back."*

## Journeys (UX 1.3; start: graph, `HEAD` on `fix` after a menu checkout, `fix.txt` edited)
| Task | Steps | Before | Best tool |
|---|---|---|---|
| Undo the checkout over the edit | `u` · read (M) · `y` = **2 keys + 1 decision** | same | JetBrains Force Checkout names the loss ("you will lose them"); GitHub Desktop loses it silently (study §1, §3) |
| Get the edit back, from the landing | **`U` = 1** | works, but nothing on screen says so | none studied offers a way back |
| Get it back after a second `u` (Q4's path) | `u` → `nothing to undo · U redoes checkout fix` → `U` = **2** | dead end on screen | — |

Landings: after `y`, `HEAD` on `main`, the status above, and `← U` on the Undo entry in `J` (existing). After `U`, `HEAD` on
`fix` and `fix.txt` holds his bytes, with `redo: checkout fix ✓` (observed by the scout).

## Scope
**In:**
- `Replaced` and its read (Decided 4), carried on `Drift` and on the forced result;
- the consequence line on both paths: `drift_box` (`u`/`U`) and `revert_drift_box` (time machine);
- the `Done` suffix, and the `Nothing` arm's redo hint;
- the tests.

**Out:**
- a way back once a new operation has emptied redo: the backup keeps the edit, but only the shell reaches it;
  KNOWN_GAPS row "`J` restores files from an entry's backup", conductor;
- carrying the edit;
- the time-machine Ghost Preview's `WorktreeEffect` (`graft-ops/src/preview.rs:182`), not traced;
- Decided 5's known limit.

## Signatures
```rust
// graft-ops, undo.rs
/// What running an inverse anyway replaces that the journal never recorded (m6-conflicts/77).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Replaced {
    /// Paths the inverse writes whose content now differs from the entry's recorded after-state; sorted, unique.
    pub paths: Vec<PathBuf>,
    /// No after-state recorded (`Drift::unknown` names the working tree): the whole tree is restored.
    pub whole_tree: bool,
}
impl Replaced {
    #[must_use] pub fn is_empty(&self) -> bool;
    /// The union over the time machine's steps.
    pub fn extend(&mut self, other: Replaced);
}
// `Drift` gains `pub replaced: Replaced`, filled by `run_plan` when `!is_clean()` (Decided 4). `drift` stays pure.
// graft-ops, apply.rs:69: `Applied` gains `pub replaced: Replaced` (default empty; set by a forced undo/redo step).
// `revert_to -> Vec<Applied>`: the caller unions the fields; `presses` is derived by the job (Decided 5).

// graft, jobs::undo (jobs/undo.rs:43, a tuple today; a shape change, every match site updated)
Undone::Nothing { reason: String, redo: Option<String> }  // redo: next_redo's root description, from
                                                         // ctx.journal.entries(), no second read. If the job has no
                                                         // ctx at :220, stop and report.
Undone::Done { .., replaced: graft_ops::undo::Replaced }
RevertedTo::Drift { .., total: usize }
RevertedTo::Done { .., replaced: Replaced, presses: usize }

// graft, update::undo
/// The box's consequence line (Decided 6), fitted to `width` in ≤ `lines` wrapped lines.
pub(crate) fn drift_consequence(drift: &graft_ops::undo::Drift, redo: bool, presses: usize, width: u16, lines: usize) -> String;
/// The status suffix after a forced run; `None` when nothing was replaced.
pub(crate) fn replaced_keys(replaced: &Replaced, redo: bool, presses: usize) -> Option<String>;
```

## Failing tests first (real repos, `crates/graft/tests/it/undo_over_drift.rs` unless named)
- `undo_over_drift_then_redo_brings_the_edit_back`: menu checkout of `fix`, edit `fix.txt` (bytes B), then `u`. The box
  holds `Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.`
  `y`: `HEAD` = `main`, `fix.txt` absent, status `undo: checkout fix ✓ · U brings back your edit to fix.txt`.
  `U`: `HEAD` = `fix`, `fix.txt` == B byte-exact, porcelain v2 and the index tree as before `u`. **Keys from the
  landing: 1.**
- `undo_over_drift_then_undo_again` (69's pin, :86-90 changed): the second `u` reads `nothing to undo · U redoes
  checkout fix`, then `U` restores B. **Keys: 2.**
- **`a_staged_edit_comes_back_staged`** (not run by the scout): `fix.txt` edited **and staged**, then `u`. The box names
  `fix.txt`. `y U`: the index tree **and** the working tree are byte-equal to before `u`. `git diff --cached` shows the
  same blob and `git diff` shows nothing.
- `a_staged_and_unstaged_edit_both_come_back`: staged B1, then B2 on disk. `u y U`: the index holds B1, the disk holds B2.
- `an_edit_outside_the_undos_paths_stays_and_is_not_named`: `fix.txt` plus a tracked file the checkout did not change.
  The box names only `fix.txt`; after `y` the other edit is still on disk.
- `redo_over_drift_names_u`: `u`, an edit, then `U`. The box reads `Redo …` and `u brings it back`. `y`, then `u`
  restores it.
- `the_time_machine_names_the_presses`: two operations, then revert to before both from `J`, with `fix.txt` edited.
  The box and the landing both say `U (2 times)`. `U U` restores B byte-exact.
- `the_commit_drift_is_not_named` (TESTING.md walk 13): `Undo leaves your edits since as they are.`; `sample.txt`
  still holds `DRIFT` after `y`.
- **Fit, `view` tests, 80×24 and 160×48:** `the_drift_box_line_is_never_cut`:
  - a 60-character path and a long commit title;
  - the rendered box holds the key sentence ending `brings it back.` whole;
  - no `…` from graft-ui's cap (`confirm.rs:31`);
  - the path is left-truncated by graft.
- **Invariant, graft-ops `tests/`:** `replaced_names_exactly_what_the_run_took`:
  - scenarios: those above, plus three files and an untracked file inside the restore scope;
  - `replaced.paths` = { p : bytes before `y` ≠ bytes after `y` **and** bytes before `y` ≠ the entry's recorded
    after-state }, read with `git`;
  - for each p, `<undo entry backup>:worktree/p` holds the pre-`y` bytes.
- **Cost:** `a_clean_undo_spawns_no_diff_tree`: a clean `u` runs no `diff-tree`. Assert it on the command log the
  entry records.

## Touches (each entry, and how the builder checks it)
- **Journal/undo:** no new entry kind or journal field. The backup ref, `next_undo`, `next_redo`, `inverse` and
  `generic_steps` are unchanged. Checked by the byte-exact tests and every existing `undo`/`redo`/`revert_to` test green.
- **`Applied` and `Drift` gain a field:** grep every construction and match across crates (graft-ops `apply`, `undo`,
  `restore`; graft `jobs`, the preview). List them in NOTES.
- **Drift box, both paths:** `drift_box` (`update/undo.rs` ~:519, both `u` and `U`) and `revert_drift_box` (~:490).
  Rows in the tests above.
- **`Undone::Nothing` shape change:** every match site (`jobs/undo.rs:220`, `update::undo::undone`, tests), found by grep.
- **Doc comment:** `graft-core/src/journal.rs:28` quotes `itself undoable`. Reword it.
- **Captures** (each regenerated through its own test, named by the builder):
  - `docs/shots/m3-09-undo-drift-box-{160x48,80x24}.txt`;
  - `m3-undo-drift.txt`;
  - `m6/fix-box-owns-the-bar-{120x40,80x24}` frame 3;
  - new: `m6/fix-undo-drift-way-back-{120x40,80x24}.txt`, three frames: the box, the landing, and after `U`.

  Then `git status docs/shots`; list every moved file.
- **Contact sheets:** `docs/shots/m3-contact-sheet.txt` and its `m3-contact-sheet-captions.tsv`, and
  `docs/shots/m6/m6-contact-sheet-4.txt`. All three quote the old line. The builder lists them; the conductor rebuilds
  both sheets and their captions. Each part must stay ≤ 200 KB.
- **TESTING.md:** the :1336 row's words; walk 13 (:1290-1296) adds the box's line; a new row for the new capture.
- **Keymap:** no binding changes. `U` keeps its meaning (`docs/design/02-keymap.md` :45; scout Q8: only `u`/`U`/`y`).
- **KNOWN_GAPS:** M6/q4 closes, with its tests named in NOTES. The two proposed rows (Out; Decided 5) are for the
  conductor.
- **MAP.md:**
  - graft-ops: `Replaced`, `Drift::replaced`, `Applied::replaced`, `run_plan`'s read;
  - graft: `Undone`, `RevertedTo`, `drift_consequence`, `replaced_keys`, `undone`'s `Nothing` arm.

## Acceptance (name the verifying test at tick time)
- [x] Every test in "Failing tests first", red observed, then green, with each red line quoted in NOTES. This includes
      **`a_staged_edit_comes_back_staged`** and `a_staged_and_unstaged_edit_both_come_back`. Tests:
      `undo_over_drift::{undo_over_drift_then_redo_brings_the_edit_back, undo_over_drift_then_undo_again,
      a_staged_edit_comes_back_staged, a_staged_and_unstaged_edit_both_come_back,
      an_edit_outside_the_undos_paths_stays_and_is_not_named, redo_over_drift_names_the_backup_and_no_key (renamed,
      NOTES D1), the_time_machine_names_the_presses, capture_fix_undo_drift_way_back}`,
      `m3_walk::every_scenario::undo_drift_guard` (See `the commit drift is not named`),
      `view::tests::the_drift_box_line_is_never_cut`, graft-ops `undo_replaced::{replaced_names_exactly_what_the_run_took,
      a_clean_undo_spawns_no_diff_tree}`. Reds quoted in NOTES R1–R12.
- [x] Step counts asserted in the tests: from the landing to the edit back, **1** key (`U`); through a second `u`,
      **2**; `u` → `y` stays **2**. Tests: `undo_over_drift_then_redo_brings_the_edit_back` (`u → y stays two keys`,
      `one key from the landing to the edit`), `undo_over_drift_then_undo_again` (`u, then U` = 2),
      `the_time_machine_names_the_presses` (`U U` = the 2 presses the line named).
- [x] `the_drift_box_line_is_never_cut` green at 80×24 and 160×48. Test: `view::tests::the_drift_box_line_is_never_cut`
      (both sizes, long and short form, every built line found on the drawn screen).
- [x] `replaced_names_exactly_what_the_run_took` green on ≥ 7 scenarios; `a_clean_undo_spawns_no_diff_tree` green.
      Tests: `undo_replaced::replaced_names_exactly_what_the_run_took` (11 scenarios in `scenarios()`),
      `undo_replaced::a_clean_undo_spawns_no_diff_tree`.
- [x] Captures regenerated and the new one added. The user judges them by eye; the builder does not (rule 6). Colour
      is unverified (rule 5). Tests: `m3-09-undo-drift-box-{80x24,160x48}` by the pty walk
      (`.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py`), `view::tests::capture_m3_undo_drift`,
      `box_owns_the_bar::capture_fix_box_owns_the_bar` (`m6/fix-box-owns-the-bar-*`, frame 3),
      `undo_over_drift::capture_fix_undo_drift_way_back` (new, `m6/fix-undo-drift-way-back-{120x40,80x24}`).
- [x] 69's pin no longer asserts a dead end. NOTES cites KNOWN_GAPS M6/q4 as closed, with its tests. Test:
      `undo_over_drift::undo_over_drift_then_undo_again` asserts `Done { what: "nothing to undo", keys:
      Some("U redoes checkout fix") }`, then `U` restores the edit byte-exact.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium to large: one read in graft-ops, three answer shapes in graft, one words table with a fit rule, ten tests on
real repos, and four captures. One context. If it runs short, hand off the time machine (`revert_drift_box`,
`presses`) as the second half.

## Optional question for him (non-blocking)
"When you undo a branch switch after editing a file, should the edit come with you to the old branch? Today it waits
in a backup, and `U` takes you back to where you were with it."

## NOTES (builder, 2026-09-25)

**Context compacted mid-ticket, twice.** Nothing remains: every behaviour, test, capture, sheet and doc row below was
finished before handback, and the gates were run after the last edit.

**Scout used:** `.scratch/m6-conflicts/scouts/undo-over-drift-keeps-the-edit.md`. Its byte-exact `U` after `u y` held.
The staged case it did not run is now `a_staged_edit_comes_back_staged` and `a_staged_and_unstaged_edit_both_come_back`,
both byte-exact through `U`: `symbolic-ref HEAD`, `ls-files -s` and porcelain v2 with untracked files, all equal to
before `u`.

**Cause, not symptom.**
- The box and status could not name the edit: `Drift` compared tree ids only. Added a path read, `undo/replaced.rs`,
  run only when the drift is not clean.
- The time machine's drift box was unreachable. `revert_to` wraps every step's error in `PartWay`, including step 0's
  drift (`graft-ops/src/undo/execute.rs`), while `jobs/undo.rs` matched a bare `Drift`. So every revert over an edit
  ended "revert stopped after 0 of N steps". `run_revert_to` now opens the box for `PartWay { done: 0, Drift }`.

### Reds observed (file:line as at red time; all green now)
- R1 `undo_over_drift_then_redo_brings_the_edit_back`, `undo_over_drift.rs:166`: "assertion `left == right` failed:
  Since this checkout fix, the working tree changed. A backup is taken first, so this is itself undoable. left: 0
  right: 1".
- R2 `undo_over_drift_then_undo_again`, `:232`: "the second `u` names the way back, not a dead end. left: Done { what:
  "nothing to undo", keys: None } right: Done { what: "nothing to undo", keys: Some("U redoes checkout fix") }".
- R3 `a_staged_edit_comes_back_staged`, `:264`: "Since this checkout fix, the index changed; the working tree changed.
  A backup is taken first, so this is itself undoable. left: 0 right: 1".
- R4 `a_staged_and_unstaged_edit_both_come_back`, `:293`: the same words as R3.
- R5 `an_edit_outside_the_undos_paths_stays_and_is_not_named`, `:317`: the same words as R1.
- R6 `redo_over_drift_names_the_backup_and_no_key`, `:348`: "Since this undo: checkout fix, the working tree changed. A
  backup is taken first, so this is itself undoable."
- R7 `the_time_machine_names_the_presses`, `:98`: "no box is open; status Sticky { reason: "revert stopped after 0 of 2
  steps: the working tree changed — u again to go ahead", keys: Some("J journal") }". This is the unreachable box above.
- R8 `m3_walk::every_scenario::undo_drift_guard`, `m3_walk.rs:1679`: "FAIL undo-drift-guard: the commit drift is not
  named: the box should leave the edit unnamed: Since this commit "before the drift", the working tree changed. A
  backup is taken first, so this is itself undoable."
- R9 `the_drift_box_line_is_never_cut`, `view/tests.rs:3110`: "80x24, commit "Add OAuth provider": over graft-ui's cap:
  ["Since this commit \"Add OAuth provider\", refs/heads/main", "moved; HEAD moved; the working tree changed.", "Undo
  replaces your edit to", "a/a/a/services/billing/src/handlers/invoice…". That was before the fit rule.
- R10 `replaced_names_exactly_what_the_run_took`, `undo_replaced.rs:273`: "an unstaged edit inside the checkout's paths:
  the box names what the run takes. left: Replaced { paths: [], whole_tree: false } right: Replaced { paths:
  ["a.txt"], whole_tree: false }".
- R11 `a_clean_undo_spawns_no_diff_tree`:
  - First red at `undo_replaced.rs:335`. The run's filter dropped its message line.
  - Re-observed on 2026-09-25 under a mutation: the forced run's reads were not passed into its log. Red at `:400`:
    "the forced run's read is in its log: [... "git diff-tree -r --name-only -z <after> <before>" ...]" (abridged,
    oids elided), with no `diff-tree -r -z --no-renames` among them. Mutation reverted, green again.
  - Also mutated: the read was run on the clean path too. The test stayed **green**, because `in_worktree` and
    `in_index` return early when now == after (`undo/replaced.rs:104`, `:129`), and a clean drift means exactly that.
    So the "no cost on a clean undo" promise holds by two guards, and the test cannot tell the rail's gate from the
    read's own early return.
- R12 `capture_fix_undo_drift_way_back`: "capture docs/shots/m6/fix-undo-drift-way-back-120x40.txt does not match what
  the code draws" (`graft-fixtures/src/capture.rs:147`), before the file existed.
- Not separately recorded: the red of the unit test `update::undo::tests::nothing_to_undo_names_the_redo_waiting`. The
  same behaviour's red is R2.

### Decisions and deviations from the spec
- **D1. A redo names no key back.** The spec's `redo_over_drift_names_u` expected `u brings it back`, with `u` restoring
  the edit. The code does not do that:
  - `next_undo` passes over `Redo` entries (`graft-ops/src/journal.rs:264`).
  - So `u` after a redo inverts the **operation**, whose backup never held the edit. The edit is in the redo's backup
    only.
  - Renamed to `redo_over_drift_names_the_backup_and_no_key`. The box reads "Redo replaces your edit to main-one.txt; a
    backup keeps it." The status after `y` names the backup, not a key.
  - The test pins that `u` then puts back the operation's `one\n`, not the edit.
- **D2.** `Undone::Done(Applied)` stays a tuple. `replaced` rides on `Applied` (`graft-ops/src/apply.rs`), so every
  forced run carries it: undo, redo and each time-machine step.
- **D3.** `CONFIRM_COLUMNS` went from 62 to 60. The box is capped at 64 cells (`graft-ui/src/confirm.rs:156`), and
  graft-ui wraps at 60 inside it (`:435-436`). Pre-wrapping at 62 let graft-ui re-wrap, so the line count graft
  planned was wrong.
- **D4.** `UndoError::Drift` is now `Drift(Box<Drift>)`. With `replaced` added, clippy `result_large_err` (152 > 128 B)
  fired on every `Result` in `undo/execute.rs`.
- **D5.** Worst case is **8** spawns, against the spec's ≤ 4: two `diff-tree` plus three index comparisons, each one
  `diff-tree` for two index states of a shape but two `ls-tree -r` for a merged state against an unmerged one (an undo
  inside a live merge) — 2 + 3 × 2 (`undo/replaced.rs` module doc). A clean undo spawns 0. (Corrected by
  m6-conflicts/80, review m1: this said 5, counting each index comparison as one spawn.)
- **D6.** The nothing-to-undo status renders `nothing to undo ✓ · U redoes checkout fix`. The `✓` is `Status::Done`'s
  own mark. The spec wrote it without one.
- **D7.** The time-machine box's verb is "Undo", like `u`'s. Its key is `U (N times)`, where N = `steps.len() − k`
  and k is the earliest step that replaced anything. `U` redoes the jump's last step first, so reaching step k
  brings back every later step on the way (`jobs/undo.rs:482-500`, `replaced_by_jump`).
- **D8. Fit limit.** When the drift line alone takes all four lines (graft-ui `MAX_MESSAGE_LINES`, `confirm.rs:32`),
  the consequence line has no room and graft-ui's cap cuts it. Only extreme titles hit this: a 3-line drift line
  already falls back to the short form, which keeps the key sentence (R9's second title).
- **D9.** `m3-undo-drift.txt` shows the short form: its drift line takes three lines.

### Pre-existing bug found (not fixed; out of scope)
- **Broken.** After an undo that moves HEAD but touches no ref, the graph keeps the old HEAD badge:
  `graft-ops/src/restore.rs:275-276` picks `Refresh::WorkingTree` when `pending.entry.touches.refs.is_empty()`, and a
  checkout's undo moves only HEAD.
- Visible in `docs/shots/m6/fix-undo-drift-way-back-*.txt` frame 2: the graph still shows `[◆ fix]` after landing on
  `main`. The capture header marks that frame KNOWN WRONG.
- The status and the working tree are right, checked by git in the test.

### Proposed KNOWN_GAPS rows (for the conductor)
- **Broken:** the graph keeps the old HEAD after undoing a checkout. The pre-existing bug above,
  `restore.rs:275-276`.
- **Unverified / design:** a redo over drift has no key back (D1). The edit is kept only in the redo's backup ref.
- **Design:** the time-machine box names step 0's replaced paths only, because drift is read at the first step. The
  landing names the union of every step's.
- **Design:** `J` cannot restore files from an entry's backup. It is the only way to reach D1's edit without git.
- **Limit:** D8, where the consequence line is cut when the drift line takes four lines.

### KNOWN_GAPS M6/q4: closed
Closed by `undo_over_drift_then_undo_again` (the second `u` names `U redoes checkout fix`; `U` restores byte-exact) and
`undo_over_drift_then_redo_brings_the_edit_back` (the box and landing name `U`; one key from the landing).

### Touches, and how each was checked
- **`Drift` gained `replaced`, and `Applied` gained `replaced`.** Every construction site was found by grep
  (`replaced:`) and compiled under clippy:
  - graft-ops: `apply.rs`, `apply/fold.rs`, `apply/resume.rs`, `restore.rs`, `undo.rs`, `undo/execute.rs`. Also
    changed, but not construction sites: `undo/tests.rs` (new `replaced_unions_sorted_and_unique`) and
    `tests/it/main.rs` (`mod undo_replaced;`);
  - graft: `jobs/undo.rs`, `update/operate.rs`, `update/banner/tests.rs`, `update/remote/tests/lease.rs`,
    `update/resolver/tests/words_and_c.rs`, `update/undo.rs`, `update/undo/box_answers.rs`, `view/tests.rs`.
- **`Undone::Nothing` and `RevertedTo` shape changes.** Every match was found by grep: `jobs/undo.rs:249-253`,
  `update/undo.rs:333`, its tests, and `box_answers.rs:103`.
- **Journal/undo.** No new entry kind or journal field. The backup refs, `next_undo`, `next_redo`, `inverse` and
  `generic_steps` are unchanged. The workspace nextest run is green (3300 passed), including every undo, redo and
  revert_to test.
- **Keymap.** No binding changed. The code only names `U` and `u`.
- **Captures.** Every one was regenerated through its own test (see the ticks):
  - `docs/shots/m3-09-undo-drift-box-{80x24,160x48}.txt`
  - `m3-undo-drift.txt`
  - `m6/fix-box-owns-the-bar-{120x40,80x24}.txt`
  - new: `m6/fix-undo-drift-way-back-{120x40,80x24}.txt`
- **Contact sheets.**
  - Captions were updated in `m3-contact-sheet-captions.tsv` and `m6/m6-contact-sheet-captions.tsv` (a frame-3 note on
    the 69 row, plus two rows for this ticket).
  - Both sheets were rebuilt with `scripts/contact-sheet.sh`. A second rebuild gave the same bytes.
  - Sizes: m3 130 209 B; m6 parts 179 678 / 177 038 / 163 295 / 42 937 B, all ≤ 200 000.
- **TESTING.md.** Walk 13 now has the box's line, the Drift box row has the new words, and there is a new row
  "Undo over drift, the way back".
- **Docs.** `graft-core/src/journal.rs:27-29` was reworded: `U` takes an `Undo` back by journaling a `Redo`.
- **MAP.md.**
  - graft-ops: `Replaced`, `Drift::replaced`, `Applied::replaced`, `execute_over_drift`, `run_plan`'s read, and
    `UndoError::Drift(Box<Drift>)`.
  - graft: `undone`, `reverted_to`, `drift_consequence`, `replaced_keys`, `RevertedTo`, `Undone::Nothing`.
- **Not touched.** graft-ui and `docs/design/`, checked with `git status`. KNOWN_GAPS was not read.

### Verified, and not verified
- **Verified on real repos, byte-exact through `U`:** the unstaged edit, the staged edit, both together, the edit
  outside the undo's paths (left on disk), and the time machine's two presses.
- **Verified by drawing at 80×24 and 160×48:** the box's words. Every line graft built was found on the drawn screen.
- **Not verified:** colour (rule 5). How the captures look is the user's call (rule 6).
