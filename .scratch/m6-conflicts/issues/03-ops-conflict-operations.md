# ops-conflict-operations: opening a conflict, taking a side per block, resolving the whole-file shapes through git, starting a file over, and the refusals that keep `⏎` honest

Status: ready
Blocked by: **01**.
Build with: Opus — greenfield in `graft-ops` on a fixed signature set from G1; the decisions (which git command resolves which `UnmergedHow`, how a whole-file resolution folds into the live journal entry) are checkable against a throwaway repo, so not Fable.
Crates: graft-ops, graft-core
Needs:
- `git-conflict-model` — `conflict::{ConflictFile, Take, read, write_back, stage_bytes, diff3_blocks, attach_base, ReadError, WriteError, BaseMismatch}`, `status::UnmergedHow`, `Shape::{StoppedRebase, StoppedMerge}`.

## Read first

- `docs/design-notes/m6-conflicts.md` — Interfaces (the `graft-core`/`graft-ops` block), Hard-to-reverse choices 2 and 3, Proposed change 2(c) (`d` start over — build the operation; the key is bound in G5 only once the keymap names it).
- `docs/design/06-undo-journal-design.md` §6.1–§6.4 (a conflicted operation is one `Pending` entry; a step inside a live span folds into it).
- `crates/graft-ops/MAP.md` §operation (`Operation`, `Target`, `Refusal`), §apply (`apply`, `ApplyError::Conflicted`, `resume`, `Recovered`, the fold-into-live-entry rule from m5), §journal, §seam rule (`ops` decides, `app` wires; `tests/it/seam_rule.rs`); `crates/graft-core/MAP.md` §journal (`OperationKind`, entry shape).
- `crates/graft-git/MAP.md` §conflict, §status (after G1).
- `KNOWN_GAPS.md` rows 229 (redo of a conflicted stash pop — the resolver owns unmerged-index state; this group's `open_conflict` must work with no `in_progress`), 235.

## Scope

In:

1. **`graft_core::journal::OperationKind::{ResolveConflict, RestartConflict}`** with their words for the journal row (`resolved <path> (ours)`, `restarted <path>`); JSONL round trip like every other kind.
2. **`Operation::ResolveWholeFile(ResolveWholeParams)`** — the whole-file shapes, mapped to git (verify each in a throwaway `StoppedMerge` repo before writing the table into MAP.md):
   - `BothModified`/`BothAdded` (called with `side`) → `git checkout --ours|--theirs -- <p>` then `git add -- <p>` (used by the app only for a **binary** UU/AA; text ones go through block takes);
   - `DeletedByThem` + `Ours` → `git add -- <p>`; + `Theirs` → `git rm --quiet -- <p>`;
   - `DeletedByUs` + `Theirs` → `git add -- <p>` (git leaves theirs' bytes in the tree — confirm); + `Ours` → `git rm --quiet -- <p>`;
   - `AddedByUs` + `Ours` → `add`; + `Theirs` → `rm`; `AddedByThem` mirrored;
   - `BothDeleted` → `git rm --quiet -- <p>` (either side);
   every path `--literal-pathspecs`. Refuses `NotUnmerged { path }` (status says so), `NotMarkerShape` is **not** raised here (it is the block-take path's refusal, see 4). Folds into the live entry when one is live (same rule as `Stage` on an unmerged path); with no live entry (a conflicted stash pop, row 229) it is its own entry, undoable through the normal backup.
3. **`Operation::RestartConflict(RestartParams)`** → `git checkout --merge -- <p>`: the conflict comes back from the stages; the app's confirm box says it is not undoable while the operation is open (folded into the live entry; the entry's backup still covers the pre-operation tree). Refuses `NotUnmerged`.
4. **`graft_ops::conflict`** (new module) — the job-side functions the app calls, no `Operation`, no journal line (an fs write inside a live span, like a hand edit — Hard choice 2):
   - `open_conflict(repo, status, path)` → `Opened { file, how, binary }`: refuses `NotUnmerged`; `how.has_markers()` and not binary → `file = Some(read(path)?)`; binary or a whole-file shape → `file = None`. `ReadError::Parse` surfaces as `OpenError::Markers(ParseError)` — the app shows the line; never a silent "no conflicts".
   - `apply_take(repo, path, expected, block, take)` → re-`read`, `fingerprint == expected` else `TakeError::Changed { now }` (never clobbers a file `$EDITOR` touched), `take`, `write_back`, return the new `ConflictFile`. `NoSuchBlock` → `TakeError::NoSuchBlock`.
   - `base_for(repo, path, file)` → `diff3_blocks` + `attach_base`; `BaseError::{NoBase /* a stage missing */, Mismatch(BaseMismatch), Diff3(Diff3Error)}`.
   - `mark_resolved_refusal(file: &ConflictFile) -> Option<Refusal>` → `ConflictsOpen { path, open }` when `open() > 0`; the app calls it before `Stage` so the refusal is in words *before* a git process runs (`MarkersRemain` stays as the second line of defence).
5. `Refusal::{NotUnmerged { path }, ConflictsOpen { path, open }, NotMarkerShape { path, how }}` with `words()` for each: `<path> is not conflicted`, `<n> conflicts still open in <path>`, `<path>: <how> — no markers to take from`.
6. `tests/it/seam_rule.rs` extended: `conflict::apply_take` and `write_back` are the only non-`GitInvocation` writers `ops` exposes; the app never calls `graft_git::conflict::write_back` directly (test greps `crates/graft/src` for the symbol).
7. MAP.md entries for `graft-ops` (`conflict`, the whole-file table) and `graft-core`.

Out: any key or view; the `Stage` operation (exists, m5); `merge-tree` forecasts (row 513, M7); rerere; `diff --cc`.

## Signatures

```rust
// graft-core/src/journal.rs
pub enum OperationKind { /* existing */ ResolveConflict, RestartConflict }

// graft-ops/src/operation.rs
pub enum Operation { /* existing */ ResolveWholeFile(ResolveWholeParams), RestartConflict(RestartParams) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Side { Ours, Theirs }
pub struct ResolveWholeParams { pub path: PathBuf, pub side: Side, pub how: UnmergedHow }
pub struct RestartParams { pub path: PathBuf }
pub enum Refusal { /* existing: MarkersRemain, Unmerged, NotInProgress, InProgress, … */ NotUnmerged { path: PathBuf }, ConflictsOpen { path: PathBuf, open: usize }, NotMarkerShape { path: PathBuf, how: UnmergedHow } }

// graft-ops/src/conflict.rs
pub struct Opened { pub file: Option<ConflictFile>, pub how: UnmergedHow, pub binary: bool }
pub enum OpenError { Refused(Refusal), Read(std::io::Error), TooLarge { bytes: u64 }, Markers(ParseError) }
pub fn open_conflict(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<Opened, OpenError>;
pub enum TakeError { Changed { now: Oid }, NoSuchBlock(usize), Read(ReadError), Write(WriteError) }
pub fn apply_take(repo: &RepoHandle, path: &Path, expected: Oid, block: usize, take: Take) -> Result<ConflictFile, TakeError>;
pub enum BaseError { NoBase, Mismatch(BaseMismatch), Diff3(Diff3Error) }
pub fn base_for(repo: &RepoHandle, path: &Path, file: &mut ConflictFile) -> Result<usize, BaseError>;
pub fn mark_resolved_refusal(path: &Path, file: &ConflictFile) -> Option<Refusal>;
```

## Acceptance

- [ ] Each of the eleven `StoppedMerge` files resolved through `ResolveWholeFile` (both sides where a side exists) leaves `git ls-files -u` empty for that path and the index/tree bytes git's own `checkout --ours/--theirs` + `add`/`rm` would give — `apply::tests::whole_file_shapes_resolve_like_git` (a table test, one row per `(how, side)`).
- [ ] `ResolveWholeFile` during a live rebase adds **no** journal line (folded); with no live entry it writes one entry with `OperationKind::ResolveConflict` and `u` restores the unmerged index (`ls-files -u` lists the path again) — `apply::tests::whole_file_folds_into_live_entry`, `whole_file_is_its_own_entry_when_nothing_is_live`.
- [ ] `RestartConflict` on a taken-then-staged `both.txt` brings the markers back and the path back to `ls-files -u` — `apply::tests::restart_brings_the_conflict_back`.
- [ ] `open_conflict` on `pic.bin` → `binary: true, file: None`; on `theirs-deleted.txt` → `how: DeletedByThem, file: None`; on a clean path → `Refused(NotUnmerged)`; on a marker file with a stray marker → `Markers(ParseError { line, .. })` — `conflict::tests::open_names_every_shape`.
- [ ] `apply_take` after an external edit of the file → `Changed { now }` and the edit survives byte-for-byte — `conflict::tests::take_never_clobbers_an_edit`.
- [ ] `apply_take(Both)` on `three.txt` block 1 then `Ours` on block 1 again replaces the take (not appends) — `conflict::tests::retake_replaces`.
- [ ] `base_for` on `three.txt` → `Ok(3)`; on `both-added.txt` → `NoBase`; after a hand edit → `Mismatch` — `conflict::tests::base_for_is_equality_or_nothing`.
- [ ] `mark_resolved_refusal` → `Some(ConflictsOpen { open: 2 })` after one of three takes; `None` after three — `conflict::tests::mark_resolved_refusal_counts_open`.
- [ ] `Refusal::words()` for the three new variants match the design note's strings — `operation::tests::conflict_refusals_in_words`.
- [ ] `seam_rule` passes with the new allow-list — `tests/it/seam_rule.rs`.
- [ ] MAP.md updated; NOTES records the verified whole-file table with the git version it was checked on.

## Gates

fmt / clippy / nextest / doc, workspace-wide.

## Size

One context; ~500 lines including tests. Reads only the MAP.md sections named and `graft-ops/src/apply.rs`'s fold-into-live-entry region (scout-free: greenfield module, one known seam).
