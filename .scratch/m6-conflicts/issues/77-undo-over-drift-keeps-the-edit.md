# undo-over-drift-keeps-the-edit: the drift box names the edit it replaces, and one key brings it back

Status: ready
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
- [ ] Every test in "Failing tests first", red observed, then green, with each red line quoted in NOTES. This includes
      **`a_staged_edit_comes_back_staged`** and `a_staged_and_unstaged_edit_both_come_back`.
- [ ] Step counts asserted in the tests: from the landing to the edit back, **1** key (`U`); through a second `u`,
      **2**; `u` → `y` stays **2**.
- [ ] `the_drift_box_line_is_never_cut` green at 80×24 and 160×48.
- [ ] `replaced_names_exactly_what_the_run_took` green on ≥ 7 scenarios; `a_clean_undo_spawns_no_diff_tree` green.
- [ ] Captures regenerated and the new one added. The user judges them by eye; the builder does not (rule 6). Colour
      is unverified (rule 5).
- [ ] 69's pin no longer asserts a dead end. NOTES cites KNOWN_GAPS M6/q4 as closed, with its tests.

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
