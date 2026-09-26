# recover-op — the put-back as one journaled operation: move a ref back, recreate a branch, or restore an edit; `u` undoes it

Status: blocked: needs graft (five test literals, `files: None`; see NOTES) — graft-ops done
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
- [x] **Targets** (real repos, `tests/it/recover.rs`) — each test below, by name, in `recover.rs`:
  - `move_ref_not_checked_out_moves_only_the_ref`.
  - `move_ref_checked_out_resets_and_puts_unsaved_edits_back`.
  - `move_ref_refuses_when_moved_on_since`.
  - `recreate_makes_the_branch`.
  - `recreate_refuses_an_existing_name`.
  - `files_restores_the_edit_and_leaves_the_index`.
  - `files_refuses_over_unsaved_edits`.
  - `held_again_is_refused`.
- [x] **Undo:** `u_undoes_each_target`, one case per target (MoveRef off `HEAD`, MoveRef checked out with an edit set
      aside, Recreate, Files); the repo equals the pre-`y` snapshot (refs, index, every file's bytes, `HEAD`,
      `status --porcelain=v2 --untracked-files=all`, stash list), after asserting `y` changed it. Plus
      `u_after_gc_puts_the_branch_back_on_a_tip_only_the_entry_holds` (the `expected` pin).
- [x] **Lost edits:**
  - `lost_edits_finds_the_edit_an_undo_over_drift_replaced` (**not red first**: written after the code; its teeth
    shown by mutation instead — dropping the drift condition and the on-disk filter made it and the two below fail).
  - `lost_edits_finds_the_same_after_a_redo_over_drift`.
  - `an_edit_already_back_on_disk_is_not_lost`.
- [x] **Preview:**
  - `recover_preview_lists_your_files`.
  - `recover_preview_says_unchanged_when_not_checked_out`.
- [x] **Journal:**
  - `a_recover_entry_describes_itself_in_words` (the `describe` strings above).
  - `touches_match_what_changed`.
- [x] MAP.md (graft-ops): `Operation::Recover`, `FilesEffect`, `lost_edits` — MAP lines added (checked by reading).
      NOTES: below.

## NOTES (m7-power-tools-23 builder)
**Blocked on graft (tests only).** `PreviewPlan` gained `files`; graft's lib builds, but five test literals in graft
spell every field and now miss it (`cargo clippy --all-targets` workspace-wide fails there). Each needs `files: None,`:
`crates/graft/src/update/preview/slice/tests.rs:47`, `crates/graft/src/update/preview/tests.rs:99` and `:129`,
`crates/graft/src/update/stash/tests.rs:161`, `crates/graft/src/view/tests.rs:3507`. No graft `match` on `Operation`
is exhaustive (its sites use `_`), so `Operation::Recover` broke nothing there. Gates run: fmt workspace-wide ok;
clippy `-p graft-ops --all-targets` and `-p graft` (lib) ok; nextest `--workspace --exclude graft` 1934/1934; doc
tests workspace ok. graft's own tests not run (they do not compile until the five lines land).

No scout: the conductor's call (22's NOTES describe the seam); I read the sites myself.

**`Operation` match sites touched** (graft-ops): `operation.rs` `kind`, `sets_aside` (checked-out `MoveRef` wraps),
`facts_scope`, `describe`, `check`, `plan`, `touches`; `apply/record.rs` `pins_for` (exhaustive; pins
`MoveRef.expected`, so `u` survives `gc`); `preview.rs` `plan` (new `preview/recover.rs`); `preview/put_back.rs`
`forecast` (checked-out `MoveRef`: from `expected`, lands at `to`). Not touched, by choice: `is_history` (refresh is
`Everything` anyway, since the entry touches a ref; a `Files` put-back refreshes the working tree only).

**Deviations from the spec, and why:**
- `lost_edits(repo: &RepoHandle, entries, stop)` — no `git: &Git` (no such type in graft-ops; 22 dropped it the same
  way). It lives in `journal_list` (beside `LostEdit`, `find_lost`), not `operation`: `operation` is pure by rule and
  this reads the repository.
- `lost_edits`' rule has a third condition the spec lacks: the backup must also differ from the **reversed** entry's
  after-state (his drift). The spec's two conditions alone list every file the undo itself wrote — undoing a checkout
  would list the checkout's own files, all held by commits. Test: `lost_edits_finds_the_edit_an_undo_over_drift_replaced`
  asserts `o.txt` (the checkout's file) is not listed. Same rule as `undo::replaced`. An unrecorded after-state narrows
  nothing (unknown ≠ unchanged); both unrecorded → the entry is skipped.
- `FilesEffect` lives in `operation` (as the spec says), re-exported beside `RecoverTarget`.
- New refusal `Refusal::UnsavedEditsTo { paths }` for the Files words; it counts staged and untracked paths too (the
  restore writes over the file whatever state it is in).
- `HeldAgain` is decided from the facts only: `MoveRef` when the branch is already at `to`, `Recreate` when the name
  is at `at`. "Reachable from some other ref" is not checked at `y` (it needs a walk; `recover_for` already refuses
  that case from the list).
- `MoveRef` whose branch is gone → `NoSuchRef` (the spec named no refusal for it).
- `FilesEffect::Changes` for a checked-out `MoveRef`: `come_back` = paths present in `to` that differ from `expected`
  (added or changed), `go` = paths absent from `to`. For `Files`: present in the backup / absent.

**Round 2 (conductor rulings):**
- Open point 1, **refuse**: a checked-out `MoveRef` over an untracked file at a path `to` holds is refused
  `UnsavedEditsTo { paths }` in `preview` (`preview/recover.rs`) and again in `apply` (`guard_untracked`, before the
  `Pending` line), both through one read, `preview::untracked_in_the_way` (`ls-tree <to> -- <untracked paths>`; no
  spawn when nothing is untracked). Words reused: `c.txt has unsaved edits · c commits or S stashes them first` fits;
  `S` stashes untracked files with its `-u` toggle. Red first:
  `move_ref_checked_out_refuses_over_an_untracked_file_it_would_overwrite` failed with "his file was overwritten"
  (c.txt became `c\n`), now passes: refused at `y` and in the preview, bytes unchanged, nothing journaled.
- Open point 2, **tested, acceptable**: `files_over_a_path_the_backup_lacks_changes_nothing` — git refuses the
  pathspec, the entry closes `Aborted`, repo state (refs, index, files, status, stashes) unchanged. Passed as-is.

**Touches:** every `Operation` match — graft-ops build + clippy, the sites above; graft lib builds (`cargo clippy -p
graft`), graft tests blocked as listed. `PreviewPlan` users — graft-ops preview tests green; graft's five literals
above. Set-aside — `set_aside.rs` tests green, plus `move_ref_checked_out_resets_and_puts_unsaved_edits_back`.
Undo/redo — no new inverse code; `undo_*` tests green, `u_undoes_each_target`. `undo::replaced::raw_diff` now calls
a crate-shared `tree_diff(repo, …, stop)`; `undo_replaced*` tests green. Journal format — `a_recover_entry_round_trips`
green. UI, keymap, captures: none.

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
