# undo-over-drift-keeps-the-edit: the drift box names the edit it replaces, and one key brings it back

Status: draft
From: KNOWN_GAPS **M6/q4** (:812, broken, Major against M3 "you cannot lose work"); `docs/reviews/m6-usability-final.md`
Q4 (:269-270); ticket 69 NOTES (:233-238) and its pin `tests/it/undo_over_drift.rs` `undo_over_drift_then_undo_again`.
Build with: *(blank: the conductor sets it after the scout. Undo is the xhigh area.)*
Needs: none (69 merged; this extends its pin).
Crates: **graft-ops** (`undo::Drift`, the forced run's result), **graft** (`jobs::undo`, `update::undo`, the box's lines).
**graft-ui only if** scout Q5 finds the confirm box cannot take a wrapped consequence line: then `blocked: needs graft-ui`.
Scout first: `.scratch/m6-conflicts/scouts/undo-over-drift-keeps-the-edit.md` (brief below).

## Read first
- `docs/design/06-undo-journal-design.md` §6.3 (snapshot), §6.4 last two paragraphs ("Undo snapshots before it
  restores"), §6.5 (redo), §6.7 (safety rails). ADR-0009.
- `crates/graft-ops/MAP.md` `## undo` (:103-118: `inverse`, `Drift`, `drift`, `plan_revert_to`), `## restore`
  `execute` (:187), `## undo (execution half)` (:191-196), `## journal` `next_undo`/`next_redo` (:95-98).
- `crates/graft/MAP.md` `## update::undo` (:277-290), `ConfirmThen` (:263), `Command::Undo/Redo` bool (:194).
- Ticket 69 NOTES :233-238. Capture `docs/shots/m3-09-undo-drift-box-80x24.txt` (the box today).
- `docs/research/checkout-cherry-pick.md` §1 JetBrains/GitHub Desktop rows, §4 "Dirty tree".
- This ticket's scout (whole, ≤1 page).

## What is wrong (from the observed pin; code lines are the scout's to name)
The box today (`m3-09-undo-drift-box-80x24.txt` :10-17): `Undo "…"?` · `Since this commit "…", the working tree changed.` ·
**`A backup is taken first, so this is itself undoable.`** · `undo restores the recorded state` · `▸[ Stop ] [ Undo ]`.
- It does not say **which** of his edits the undo replaces (UX §3.2: a box hiding what would be lost; H9).
- "itself undoable" reads as `u`. The pin shows the second `u` answers `nothing to undo`. The real way back is `U`
  (redo): the Undo entry's inverse is a Redo built from that entry's own before-state, which is the snapshot taken at
  `y` (graft-ops MAP :111, design §6.4/§6.5). **Nobody has seen `U` work** (69 NOTES :237-238).
- The landing, `undo: checkout fix ✓`, says nothing about where the edit went.

## Decided here
1. **The box's promise:** *what the undo replaces is named here, a backup keeps it, and one key brings it back.*
   Today: named ✗; backup ✓ (pinned, `worktree/fix.txt`); one key: ✗ as worded (`u`), unproven as built (`U`).
2. **Not a design error: a way back that is missing from the screen and was never proven.** `y` replacing drift is the
   design (§6.7: "the rail is the warning, the snapshot is the guarantee"; §6.5: undo is undoable, like an editor's).
   `y` stays. The edit ends up **in the Undo entry's backup; `U` puts it back**, byte-exact, with `HEAD` where it was
   before `u`, because redo restores the state he had at `y`.
   **Rejected: carry the edit over the undo** (JetBrains' Smart Checkout, study §1). Carrying a change onto a tree
   without that file is a modify/delete clash: GitHub Desktop's reported lost-work failure (study §4). A third button
   ("undo and keep my edit") could come later with no change here. That is an optional question for him, and nothing
   here waits on it (below).
3. **What counts as "replaced"** is computed by graft-ops, from the same reads as `drift`: every path the inverse writes
   (its `RestoreWorktree` or `RestoreIndex` paths) whose content now differs from the entry's recorded after-state.
   Edits outside those paths stay in the tree and are not named. `Drift::unknown` containing the working tree means
   the whole tree is replaced.
4. **Words** (repo-relative paths; the box wraps as today):

| Where | Case | Words |
|---|---|---|
| box, consequence line (replaces `A backup is taken first, so this is itself undoable.`) | one path | `Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.` |
| | n paths | `Undo replaces your edits to 3 files; a backup keeps them, and U brings them back.` |
| | whole tree | `Undo replaces the whole working tree; a backup keeps it, and U brings it back.` |
| | nothing replaced, tree or index drifted | `Undo leaves your edits since as they are.` |
| | nothing replaced, refs/`HEAD`/merge only | `A backup is taken first, and U brings this state back.` |
| | redo box (`U` over drift) | same, `Redo …` and **`u`** |
| | time machine, N steps > 1 | `… and U (N times) brings it back.` |
| status after `y` (`Done` + keys) | undo | `undo: checkout fix ✓ · U brings back your edit to fix.txt` (n: `your edits to 3 files`) |
| | redo | `redo: … ✓ · u brings back your edit to fix.txt` |
| | time machine | today's `Done` words `· U (N times) brings back your edit to fix.txt` |
| `u` with nothing to undo and a redo waiting | any | `nothing to undo · U redoes checkout fix` (the root operation's description) |

   The command slot `undo restores the recorded state` is unchanged. The title and "what drifted" line are unchanged.
   Nothing replaced → no ` · U brings back …` suffix on the status.
5. **No ADR.** This fulfils §6.5/§6.7 and ADR-0009 as written: words plus one field computed from values `drift`
   already holds. It is reversible. One sentence for DESIGN §6.7 (conductor, Proposed): *"the box names the paths the
   forced run replaces and the key that brings them back."*

## Journeys (UX 1.3; start: graph, `HEAD` on `fix` after a menu checkout, `fix.txt` edited)
| Task | Steps | Before | Best tool |
|---|---|---|---|
| Undo the checkout over the edit | `u` · read box (M) · `y` = **2 keys + 1 decision** (unchanged) | same | JetBrains Force Checkout warns "you will lose them" and names the loss; GitHub Desktop loses silently (study §1, §3) |
| Get the edit back, from the landing | **`U` = 1 key** | impossible in-app: shell and backup ref (a Times I left) | no studied tool offers a way back |
| Get it back after reaching for `u` again (Q4's path) | `u` → `nothing to undo · U redoes checkout fix` → `U` = **2 keys** | dead end | — |

Landings: after `y`, the graph with `HEAD` on `main`, the status line above, and the Undo entry's `← U` in `J` (the
existing mark). After `U`, `HEAD` on `fix`, `fix.txt` back with his bytes, and `redo: checkout fix ✓`.

## Scope
**In:** `Replaced` in graft-ops and the forced run's result carrying it; the box's consequence line on both paths
(`undone` for `u`/`U`, `reverted_to` for the time machine); the `Done` suffix; the `Nothing` arm's redo hint; the tests.
**Out:**
- a way back once a new operation has emptied redo. The backup ref still holds the edit (§6.8 keeps it), but only
  the shell can reach it. The conductor adds a KNOWN_GAPS row: `J` "restore files from this entry's backup".
- carrying the edit (Decided 2).
- the Ghost Preview of a time-machine revert showing the replaced edits (scout Q7 reports; not built here).

## Signatures (names fixed; where they attach is the scout's)
```rust
// graft-ops, undo.rs
/// What running an inverse anyway replaces that the journal never recorded (m6-conflicts/77).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Replaced {
    /// Paths the inverse writes whose content now differs from the entry's recorded after-state, sorted.
    pub paths: Vec<PathBuf>,
    /// The after-state was never recorded (`Drift::unknown` has the working tree): the whole tree is restored.
    pub whole_tree: bool,
}
impl Replaced { #[must_use] pub fn is_empty(&self) -> bool; }

impl Drift {
    /// Computed by `drift` from the reads it already made, and the plan's own restore paths.
    #[must_use] pub fn replaced(&self) -> &Replaced;
}
// The forced `undo`/`redo` success value and `revert_to`'s gain `replaced: Replaced` (from the drift the run
// computed and ignored). `revert_to` also gains `presses: usize`, the `U` count that restores the earliest replaced
// content (total − index of the first step that replaced anything + 1).

// graft, jobs::undo
Undone::Done { .., replaced: graft_ops::undo::Replaced }
Undone::Nothing { .., redo: Option<String> }   // next_redo's root description, from the journal the job already read
RevertedTo::Drift { .., total: usize }
RevertedTo::Done { .., replaced: Replaced, presses: usize }

// graft, update::undo
/// The box's consequence line (Decided 4). `back` is `'U'` for an undo, `'u'` for a redo; `presses` > 1 adds "(N times)".
pub(crate) fn drift_consequence(drift: &graft_ops::undo::Drift, redo: bool, presses: usize) -> String;
/// The status suffix after a forced run; `None` when nothing was replaced.
pub(crate) fn replaced_keys(replaced: &Replaced, redo: bool, presses: usize) -> Option<String>;
```

## Scout brief (≤1 page, code not intent; lines at the base hash)
1. `graft_ops::undo::{undo, redo, revert_to}`: their success types, and whether `force = true` still computes `Drift`
   (so the forced run can return `replaced` from the same read).
2. `Drift::worktree`/`index` field types. Is there a path-level diff of `worktree_after` against
   `RepoStateNow::worktree_tree` (`index_paths_changed` or similar)? Are the inverse's `RestoreWorktree.paths` and
   `RestoreIndex.paths` resolvable to a path list before running?
3. **The inverse, traced:** for the Q4 Undo entry, which tree does its Redo's `RestoreWorktree` read (the entry's
   backup `worktree/` subtree?), and do its paths include `fix.txt`? Does redo's drift check pass untouched? A pin, not
   a verdict: the builder's first red test decides.
4. The time machine: is drift checked per step at run time or for all steps before any runs? Does `force` reach every
   step? Can `RevertedTo::Drift` arrive after some steps ran?
5. Where the box's lines are built, and whether graft-ui's confirm box grows a row for a wrapped consequence line at
   80×24 (today :11-12 wrap and the box grows).
6. `Undone::Nothing`: does the job hold the journal entries so that `journal::next_redo` needs no second read?
7. Does the time-machine Ghost Preview show working-tree effects (`PreviewPlan::worktree`)? Report only.
8. Every test or capture asserting `A backup is taken first, so this is itself undoable.`, `nothing to undo`, or the
   drift box's body.

## Failing tests first (real repos, `tests/it/`)
- `undo_over_drift_then_redo_brings_the_edit_back`: menu checkout of `fix`, edit `fix.txt` (bytes B), `u`: the box
  holds `Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.`. `y`: `HEAD` = `main`, `fix.txt`
  absent, status `undo: checkout fix ✓ · U brings back your edit to fix.txt`. `U`: `HEAD` = `fix`, `fix.txt` == B
  byte-exact, `git status --porcelain=v2` and the index tree equal to before `u`. **Keys from the landing: 1.**
- `undo_over_drift_then_undo_again` (69's pin, extended): the second `u` reads `nothing to undo · U redoes checkout
  fix`, then `U` restores B. **Keys: 2.**
- `a_staged_edit_comes_back_staged`: `fix.txt` edited **and staged**; `u y U`: index tree and working tree byte-equal
  to before `u`.
- `an_edit_outside_the_undos_paths_stays_and_is_not_named`: edit `fix.txt` and a tracked file the checkout did not
  change. The box names only `fix.txt`; after `y` the other edit is still on disk.
- `redo_over_drift_names_u`: `u`, then an edit, then `U`: the box reads `Redo …` and `u brings it back`; `y`, `u` restores it.
- `the_time_machine_names_the_presses`: two operations, revert to before both from `J`, with `fix.txt` edited. The box
  says `U (2 times)` and the landing says the same. `U U` restores B byte-exact. The count equals `presses`.
- `the_commit_drift_is_not_named` (TESTING.md walk 13's case): the box reads `Undo leaves your edits since as they are.`
  and `sample.txt` still holds `DRIFT` after `y`.
- **The invariant, graft-ops:** `replaced_names_exactly_what_the_run_took` covers the scenarios above plus three files
  and an untracked file inside the restore scope. `replaced.paths` must equal { p : bytes before `y` ≠ bytes after `y`
  **and** bytes before `y` ≠ the entry's recorded after-state }, read with `git` on the repo. For each such p,
  `<undo entry backup>:worktree/p` must hold the pre-`y` bytes.

## Touches (each entry, and how the builder checks it)
- **Journal/undo:** no new entry kind or journal field; the backup ref, `next_undo`, `next_redo` and the inverse are
  unchanged. Checked by the byte-exact tests above and `cargo nextest run` of every `undo`/`redo`/`revert_to` test.
- **Drift box, both paths:** `undone` (`u` and `U`) and `reverted_to`. Rows for each in the tests above. `ConfirmThen::
  Undo`/`RevertTo` keep their shape unless scout Q4 says the box needs `total`.
- **graft-ops callers of `Drift`:** grep `Drift` across crates; each caller is listed in NOTES (preview included).
- **Captures:** regenerate `docs/shots/m3-09-undo-drift-box-{160x48,80x24}.txt`, `m3-undo-drift.txt`, and
  `m6/fix-box-owns-the-bar-{120x40,80x24}` frame 3, each through its own test (builder names it). New capture:
  `docs/shots/m6/fix-undo-drift-way-back-{120x40,80x24}.txt` with three frames: the box, the landing, and after `U`.
  Afterwards run `git status docs/shots` and list every moved file. The conductor rebuilds the m3 and m6 contact sheets
  and their captions.
- **TESTING.md:** the :1336 row's words; walk 13 (:1290-1296) adds the box's line; a new row for this capture says what
  right looks like.
- **Keymap:** no binding changes. `U` keeps its meaning (`docs/design/02-keymap.md` :45). Grep the doc for the box's
  words and edit only a quoted line.
- **KNOWN_GAPS:** M6/q4 closed by the tests above, stated in NOTES; the Out row is proposed. The conductor edits.
- **MAP.md:** graft-ops `Replaced`, `Drift::replaced`, the result fields; graft `Undone`, `RevertedTo`,
  `drift_consequence`, `replaced_keys`, and `undone`'s `Nothing` arm.

## Acceptance (name the verifying test at tick time)
- [ ] Every test in "Failing tests first", red observed, then green; each red line quoted in NOTES.
- [ ] Step counts asserted in the tests: landing → edit back **1** (`U`); via a second `u` **2**; `u` → `y` unchanged at **2**.
- [ ] `replaced_names_exactly_what_the_run_took` green on ≥ 6 scenarios.
- [ ] The captures above were regenerated and the new one added. Judged by eye by the user, not by the builder (rule 6).
      Colour unverified (rule 5).
- [ ] 69's pin no longer asserts a dead end; KNOWN_GAPS M6/q4 is cited as closed, with the test named.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium. One derived field computed in graft-ops, three answer shapes in graft, one table of words, eight real-repo
tests, and three captures. It fits one context. If it runs short, the time machine (`reverted_to`, `presses`) is the
half to hand off.

## Optional question for him (non-blocking; nothing here depends on it)
"When you undo a branch switch after editing a file, should the edit come with you to the old branch? Today it waits
in a backup, and `U` takes you back to where you were with it."
