# ops-conflict-operations: opening a conflict, taking a side per block, resolving the whole-file shapes through git, starting a file over, and the refusals that keep `⏎` honest

Status: done
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

Every tick names the test that verifies it. Two boxes are **not** ticked and
say why.

- [x] Every `(how, side)` resolved through `ResolveWholeFile` leaves `git ls-files -u` empty for that path and the index line and working-tree bytes git's own `checkout --ours|--theirs` + `add`/`rm` would give — `conflict_ops::whole_file_shapes_resolve_like_git` (14 rows over 7 shapes, each run twice: once through the operation, once through raw `git` in a second repository, then compared). Anti-vacuity: `conflict_ops::the_stopped_merge_has_one_path_per_shape` pins that the fixture really holds all seven. The `DeletedByUs` + `Theirs` row the spec marked "confirm" has its own test, `conflict_ops::deleted_by_us_taking_theirs_stages_the_bytes_git_left`. Binary `UU`, against the real corpus: `conflict_ops::a_binary_conflict_resolves_whole`. Pure command lines: `operation::tests::plan_whole_file_resolution_per_shape`.
  *Deviation from the wording:* the table is per `(how, side)`, not per corpus file — the corpus's other `UU` files (`three.txt`, `crlf.txt`, `nofinal.txt`, `md.md`, `zdiff3.txt`) differ only in their marker text, which the whole-file path never reads. The stopped merge is built inside the test because it needs a writable journal and `TempRepo`'s read-back helpers.
- [x] `ResolveWholeFile` inside a live operation adds **no** journal line (folded) — `conflict_ops::whole_file_folds_into_live_entry` (a live merge; `facts.in_progress` is what selects the folding path, so a rebase takes the same branch).
- [x] With no live entry it writes one entry with `OperationKind::ResolveConflict` — `conflict_ops::whole_file_is_its_own_entry_when_nothing_is_live` (over a real conflicted `stash pop`, which the test proves leaves `in_progress == None`).
- [ ] **…and `u` restores the unmerged index (`ls-files -u` lists the path again).** **Not true, and not fixed here.** Measured: `u` refuses with `no backup was taken for that operation, so it cannot be undone` — `conflict_ops::undo_after_a_whole_file_resolution_with_nothing_live` records exactly that. Cause, with its line: the index was unmerged when the before-snapshot was taken, `git write-tree` has no tree for one (`crates/graft-ops/src/snapshot.rs:147` records `index_tree: None`), so the entry's `index_before` is `None` and `crates/graft-ops/src/undo.rs:238` turns `touches.index` into `NotInvertible::NoBackup`. Restoring an unmerged index needs the three stages recorded and replayed through `update-index --index-info` — a new `Step` and a new before-field, which is `KNOWN_GAPS` 229's own scope ("the conflict resolver owns unmerged-index state"), not this group's. **This is not new with this operation**: any operation whose before-index is unmerged has the same shape, and because undo is linear the refusal also stands between the user and older entries. Worth a ticket.
- [x] `RestartConflict` brings the markers back and the path back to `ls-files -u` — `conflict_ops::restart_brings_the_conflict_back` (three stages listed again, theirs' lines back in the file).
  *Deviation from the wording:* the acceptance said "taken-then-**staged** `both.txt`". Staging an unmerged path collapses it to stage 0, so the status no longer calls it unmerged and `check` refuses `NotUnmerged` — which the spec's own Scope line asks for, and which matches `open_conflict` (the resolver can only be open on an unmerged path, so `d` cannot be pressed on a staged one). The test therefore takes the side *on disk*, the reachable state, and `conflict_ops::conflict_operations_refuse_a_path_that_is_not_conflicted` pins the refusal on the staged one. Raw git does restore a staged `UU` from its resolve-undo record, but refuses a staged whole-file shape with `does not have all necessary versions` (both verified, git 2.54) — so allowing it would work for one shape and fail for six.
- [x] `open_conflict`: `pic.bin` → `binary: true, file: None`; `theirs-deleted.txt` → `how: DeletedByThem, file: None`; a clean path → `Refused(NotUnmerged)`; a stray marker → `Markers(ParseError { line })` — `conflict::tests::open_names_every_shape`.
- [x] `apply_take` after an external edit → `Changed { now }`, and the edit survives byte for byte — `conflict::tests::take_never_clobbers_an_edit`. `NoSuchBlock`: `conflict::tests::take_names_a_block_that_is_not_there`.
- [x] `Both` on block 1 then `Ours` on block 1 replaces the take rather than appending — `conflict::tests::retake_replaces` (block count unchanged, `open()` down by one, disk bytes equal to the document's rendering).
  *Deviation from the signature:* `apply_take` takes `file: &ConflictFile` instead of `expected: Oid`, and does not re-read the path. **Measured reason:** a taken block renders as its taken lines alone, so re-reading after a write drops every `Resolved` region and re-indexes the rest — `three.txt` goes 3 blocks → 2, and the assertion is in the same test (`on_disk.blocks().len() == before - 1`). With the filed signature a re-take lands on a *different* block and `BlockState::Took` is never observable, so the resolver could not draw it. `write_back` already does the fingerprint comparison the `expected` parameter was for, so the clobber guard is unchanged (and there is one file read instead of two). **G4/G5 must pass the document they are holding.**
- [x] `base_for` on `three.txt` → `Ok(3)`; on `both-added.txt` (`AA`, no stage 1) → `NoBase`; after a hand edit inside a block → `Mismatch` — `conflict::tests::base_for_is_equality_or_nothing`.
- [x] `mark_resolved_refusal` → `Some(ConflictsOpen { open: 2 })` after one of three takes, `None` after three — `conflict::tests::mark_resolved_refusal_counts_open`.
- [x] The three new refusals' words — `operation::tests::conflict_refusals_in_words`.
  *Deviation:* they are arms of `Refusal::explain()`. There is no `words()` on `Refusal` and adding one would be a second way to say the same thing.
- [ ] **`tests/it/seam_rule.rs` extended with the new allow-list.** **Not done: `crates/graft` is not in this group's crate set** (`graft-ops`, `graft-core`), and another builder held it concurrently. The change is one row in `FORBIDDEN` at `crates/graft/tests/it/seam_rule.rs:17`: `("write_back", Allowed::Nowhere)` — the app calls `graft_ops::conflict::apply_take`, never `graft_git::conflict::write_back`. Handed to whoever holds `graft`.
- [x] MAP.md updated (`crates/graft-ops/MAP.md` §operation, new §conflict; `crates/graft-core/MAP.md` §journal) and NOTES records the verified table with its git version.

## Gates

fmt / clippy / nextest / doc, workspace-wide.

## Size

One context; ~500 lines including tests. Reads only the MAP.md sections named and `graft-ops/src/apply.rs`'s fold-into-live-entry region (scout-free: greenfield module, one known seam).

## NOTES

**Scout:** none. The spec called it greenfield in `graft-ops` with one known
seam, and it was: one new module in each of two crates, plus new arms in
`operation.rs` and `apply.rs`'s `kind_name`/`pins_for`, all of which the
compiler names exhaustively. I read `graft-ops/MAP.md` §operation/§apply,
`graft-git/MAP.md` §conflict/§status, `apply.rs`'s fold path, `operation.rs`
and `operation/stage.rs` directly.

**The whole-file table, as git actually behaved.** git 2.54.0 (Apple
Git-157), macOS, 2026-09-22. A throwaway repository stopped on a merge with
one path per `UnmergedHow` (rename/rename is what makes the `DD`, and it
brings the `AU`/`UA` pair with it). Every row exits 0 and empties
`git ls-files -u` for that path:

| how | side | git | index after | working tree after |
|---|---|---|---|---|
| `BothModified` (UU) | ours | `checkout --ours` + `add` | stage 2's blob | stage 2's bytes |
| `BothModified` (UU) | theirs | `checkout --theirs` + `add` | stage 3's blob | stage 3's bytes |
| `BothAdded` (AA) | ours | `checkout --ours` + `add` | stage 2's blob | stage 2's bytes |
| `BothAdded` (AA) | theirs | `checkout --theirs` + `add` | stage 3's blob | stage 3's bytes |
| `DeletedByThem` (UD) | ours | `add` | stage 2's blob | kept |
| `DeletedByThem` (UD) | theirs | `rm --quiet` | gone | removed |
| `DeletedByUs` (DU) | theirs | `add` | stage 3's blob | kept |
| `DeletedByUs` (DU) | ours | `rm --quiet` | gone | removed |
| `AddedByUs` (AU) | ours | `add` | stage 2's blob | kept |
| `AddedByUs` (AU) | theirs | `rm --quiet` | gone | removed |
| `AddedByThem` (UA) | theirs | `add` | stage 3's blob | kept |
| `AddedByThem` (UA) | ours | `rm --quiet` | gone | removed |
| `BothDeleted` (DD) | either | `rm --quiet` | gone | already absent |

Both "confirm" rows came back the way the spec guessed:

- **`DeletedByUs` + `Theirs`.** git *does* leave theirs' bytes in the working
  tree — the file's blob id equals stage 3's before anything of Graft's runs
  — so `add` alone is the whole resolution.
- **`BothDeleted`.** `rm --quiet` exits 0 although no file is on disk: the
  index entry is what it takes away.

Two things nobody asked but that matter: `git rm` needs **no `-f`** on any of
these (an unmerged entry is not "staged content" to its safety check), and
`--literal-pathspecs` is a *global* option — `git --literal-pathspecs rm …`,
not `git rm --literal-pathspecs …`, which is an error. `with_paths` in
`operation/stage.rs` already puts it in the right place and is reused.

**`checkout --merge`, measured.** On a `UU` path whose markers have been
replaced on disk it restores the markers and all three stages. On a path git
has not got all three stages for (a whole-file shape, or one already staged)
it fails with `error: path '<p>' does not have all necessary versions`. That
is why `RestartConflict` refuses `NotUnmerged` rather than trying.

**Signatures that differ from the filed ones**, both argued above in the
acceptance list: `apply_take` takes the document rather than an `Oid` (with a
measured reason), and the refusal words live in `Refusal::explain()` rather
than a new `words()`. The two signatures the conductor corrected —
`diff3_file` returning a `ConflictFile`, and `attach_base` aligning runs —
were used as the merged code has them.

**Open, for the conductor.**

1. The `u` gap above (`KNOWN_GAPS` 229 territory): a whole-file resolution
   with no live entry is journaled but not undoable, and undo being linear,
   the refusal also blocks older entries. Pre-existing in shape, newly
   reachable in practice.
2. `seam_rule.rs` needs its one row; `crates/graft` was not mine.
3. The coordinator's mid-build note about `graft-git` corrections: nothing
   here reads a marker label, and nothing depends on `blocks().first_line`
   surviving `attach_base`, so the label and line-number changes do not reach
   this group. One reading does touch it: `mark_resolved_refusal` treats
   `file.open() == 0` as "nothing left to take a side on". Today a file whose
   `conflict-marker-size` is not 7 parses as clean, so `open()` is 0 and this
   refusal stays silent; `MarkersRemain` in the facts is the second line of
   defence. Once that becomes a typed parse error it will surface from
   `open_conflict` as `OpenError::Markers` instead, which is better and needs
   no change here. Merged main at `f7fec5d` before the final gate run; the
   `graft-git` fixes were not in it yet.

**Gates:** `cargo fmt --check`, `cargo clippy --all-targets -D warnings`,
`cargo nextest run` (2530 passed, 1 skipped), `cargo test --doc` — all
workspace-wide, all green after merging main.
