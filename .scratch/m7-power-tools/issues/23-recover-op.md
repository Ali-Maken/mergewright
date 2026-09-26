# recover-op — the put-back as one journaled operation: move a ref back, recreate a branch, or restore an edit; `u` undoes it

Status: ready
Blocked by: 22-journal-list
Blocked by: journal-list (or journal-list-a for the types)
Crates: graft-ops only. If graft fails to build on `Operation::Recover` (an exhaustive `match` on `Operation`), stop
and report `blocked: needs graft`, naming each site.
Source: `docs/design-notes/m7-power-tools.md`: "Pass 2b" F5, J1 (the preview, the landings), hard choice 5,
"Interfaces (2b)" graft-ops::operation; ADR-0045 §4 (set-aside); ADR-0046.
Build with: **Opus**. It is one operation over the existing apply/record/undo protocol, with an oracle per target. The
types and their meaning were decided in journal-list.
Scout: `scout` first (ADR-0035, brownfield: how a new `Operation` is wired through apply, record, touches, preview and
the undo inverse). `Build with` is set by the conductor after the scout.
Needs: `RecoverTarget`, `RecoverParams`, `OperationKind::Recover`, `LostEdit` (journal-list).

## Background (from KNOWN_GAPS; builders don't read it)
M6/77, two rows:
- An undo over drift, or a redo over drift, can replace an edit of his that then survives **only** in that undo's or
  redo's backup ref.
- Nothing in the app shows it or brings it back: getting it back takes the shell.
- Journey 15 closes that gap: `lost_edits` finds such edits, and `Files` puts one back.

## Decided
- **`Operation::Recover(RecoverParams)`**, with kind `OperationKind::Recover`.
- **`MoveRef{name, to, expected}`:**
  - Not checked out: `git update-ref refs/heads/<name> <to> <expected>` (CAS). A mismatch → `Refusal::MovedOnSince{name,
    now}`.
  - Checked out: `git reset --hard <to>`, wrapped in the ADR-0045 set-aside (unsaved edits set aside and put back after;
    a clash lands on the resolver as today). CAS on the branch tip before it.
- **`Recreate{name, at}`:** `git branch <name> <at>`. The name existing → the existing `RefExists`-style refusal (the
  scout names it).
- **`Files{entry, backup_tree, paths}`:** `git restore --source=<backup_tree> --worktree -- <paths>`. The index is
  untouched. Unsaved edits to those paths → refused in words: `src/a.rs has unsaved edits · c commits or S stashes
  them first`. No set-aside here: the edit **is** the file.
- **Held again** (the commit is reachable again by the time `y` is pressed) → `Refusal::HeldAgain{commit, by}`: `<sha7>
  is on <by> again · nothing to put back`.
- **`describe`:**
  - `put <name> back to <sha7> (<because>)`;
  - `made <name> again at <sha7> (<because>)`;
  - `put back your edit to <path>` (`+N more` when there are several paths).
- **`touches`:**
  - MoveRef: the ref; worktree and index when checked out; the stash when set aside.
  - Recreate: the ref.
  - Files: the worktree paths.
- **Undo inverse:** the generic one from `refs_before`/`worktree_before`.
  - MoveRef → back to `expected`.
  - Recreate → delete the ref (CAS on `at`).
  - Files → the paths back from the entry's own backup.
  - `u` after a put-back leaves the repo byte-for-byte as before `y`.
- **Preview data** (the app draws it; journal-screen): the `PreviewPlan` for Recover carries `now` / `after` ref lines
  and a files line.

  ```rust
  pub enum FilesEffect { Unchanged { not_checked_out: String },            // "main isn't checked out"
                         Changes { come_back: Vec<PathBuf>, go: Vec<PathBuf>, set_aside: usize } }
  ```

  The drawn words: `your files: src/api/limit.rs comes back · 1 unsaved file set aside, put back after` / `your files:
  unchanged (main isn't checked out)`.
- **`lost_edits`:** for each in-force undo or redo entry that set a backup (`worktree_before`) over drift, the paths
  where:
  - the backup's copy differs from the entry's after-state; **and**
  - the file on disk differs from the backup's copy (still lost now).

  Capped at the newest 50 entries. Stops on `stop`.

## Read first
- The note: "Pass 2b" F5, J1 (the preview box and the landings), hard choice 5, "Interfaces (2b)".
- `docs/adr/0045-…` §4; `docs/adr/0046-…`; `docs/design/06-undo-journal-design.md` §6.1–6.4 and the inverse table.
- journal-list's NOTES (the decided open points).
- `crates/graft-ops/MAP.md`: `operation` (`check`/`plan`/`touches`/`describe`, `sets_aside`, `Refusal`), `apply`,
  `record`, `undo`/`restore`, `preview` (`PreviewPlan`).

## Scout brief (≤ 1 page; `.scratch/m7-power-tools/scouts/recover-op.md`)
1. Every site in `operation.rs` (and its submodules) that matches `Operation`, with lines: what a new variant must add
   at each.
2. Whether any crate outside graft-ops matches `Operation` exhaustively. The sites, with lines.
3. How `Reset` and `BranchCreate` build their invocations and their CAS today (the closest precedents).
4. How the set-aside wraps an operation (`sets_aside`), and where its put-back clash lands.
5. What an undo/redo entry records as its drift backup, and how a test can tell "an edit that only the backup holds"
   (the M6/77 tests, if any exist).
6. `PreviewPlan`'s fields today and who builds it for a ref move.

## Signatures (exact)
```rust
// graft-ops::operation
// Operation::Recover(RecoverParams); check / plan / touches / describe / kind / sets_aside arms
pub enum FilesEffect { Unchanged { not_checked_out: String },
                       Changes { come_back: Vec<PathBuf>, go: Vec<PathBuf>, set_aside: usize } }
pub fn lost_edits(repo: &Repo, git: &Git, entries: &[JournalEntry], stop: &dyn Stop)
    -> Result<Vec<LostEdit>, FactsError>;
// PreviewPlan gains `files: Option<FilesEffect>` (None for every other operation)
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Targets** (real repos, `tests/it/recover.rs`):
  - `move_ref_not_checked_out_moves_only_the_ref`.
  - `move_ref_checked_out_resets_and_puts_unsaved_edits_back`.
  - `move_ref_refuses_when_moved_on_since`.
  - `recreate_makes_the_branch`.
  - `recreate_refuses_an_existing_name`.
  - `files_restores_the_edit_and_leaves_the_index`.
  - `files_refuses_over_unsaved_edits`.
  - `held_again_is_refused`.
- [ ] **Undo:** `u_undoes_each_target`, one case per target; the repo equals the pre-`y` snapshot (refs, index,
      worktree, `status --porcelain=v2`).
- [ ] **Lost edits:**
  - `lost_edits_finds_the_edit_an_undo_over_drift_replaced` (red first).
  - `lost_edits_finds_the_same_after_a_redo_over_drift`.
  - `an_edit_already_back_on_disk_is_not_lost`.
- [ ] **Preview:**
  - `recover_preview_lists_your_files`.
  - `recover_preview_says_unchanged_when_not_checked_out`.
- [ ] **Journal:**
  - `a_recover_entry_describes_itself_in_words` (the `describe` strings above).
  - `touches_match_what_changed`.
- [ ] MAP.md (graft-ops): `Operation::Recover`, `FilesEffect`, `lost_edits`. NOTES: the scout answers used; each
      `Operation` match site touched.

## Touches (check each, report how)
- **Every `Operation` match** (scout Q1–Q2). Check: the build, plus each site listed in NOTES.
- **`PreviewPlan` users:** a new optional field. Check: graft builds, and its preview tests stay green.
- **Set-aside (ADR-0045):** now wraps a checked-out `MoveRef`. Check: the set-aside tests, plus the new one above.
- **Undo/redo:** the generic inverse; no new inverse code expected. If one is needed, name it. Check: the undo tests.
- **Journal format:** the kind tag is journal-list's. Check: its round trip stays green.
- **UI, keymap, captures:** none (journal-screen).

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Experiments go in a
script file run with `sh`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~60k. If it compacts, split:
- **recover-op-a:** the operation, its targets, and undo.
- **recover-op-b:** `lost_edits` and the preview.
