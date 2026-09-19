# ops-model: the operation shape, the journal entry, and the per-kind inverse

Status: done
Blocked by: 01-git-reads-and-patches
Build with: Fable (signature) — CLAUDE.md names M3's "undo journal's per-kind inverse" a signature ticket; the entry shape (ADR-0031) and the four-property operation (brief point 1) are the same idea, so they live in one group.
Needs: none (pure; no git process, no clock). `git-reads-and-patches` supplies `WorkingTreeStatus`, `StashEntry`, `InProgress`, `GitVersion` — until it lands, define `RepoFacts` over those names and let the conductor order that group first, or stub the three types locally under `#[cfg(test)]` and say so.

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — Promise; Seam rule; Four properties; Journal entry; Hard choices 1–3; Proposed changes 1, 2, 9
- `docs/adr/0031-the-journal-answers-repo-state-at-a-time.md` (whole), `docs/adr/0009-…` (whole), `docs/adr/0010-…` (whole, incl. Correction)
- `docs/design/06-undo-journal-design.md` §6.1, §6.2, §6.4, §6.5, §6.7 (the shape here supersedes §6.1's sketch where the note says so)
- `docs/design/08-architecture.md` lines 92–99 (why `JournalEntry`/`OperationKind` are core types)
- `PROGRESS.md` "M3 design pass brief" point 1 only
- `crates/graft-core/MAP.md` (oid, ref_name, commit); `crates/graft-ops/MAP.md` (crate line); `crates/graft-git/MAP.md` §run (`GitArg`), §status, §version
- `.scratch/m2-details-and-diffs/issues/04-mistake-radar-detection.md` — the shape of a signature ticket that worked

## Scope

In — pure code, in this order (each piece is a commit; report turns at each):

1. `crates/graft-core/src/journal.rs`: the types below, serde `Serialize`/`Deserialize` (add `serde` with `derive` to graft-core; `Oid` gets a hex string serde impl in `oid.rs`). `EntryId::new(unix_ms, random: [u8; 10])` and `parse` — ULID, 26 Crockford base32 chars, `Ord` by string; no `ulid` crate needed (~40 lines).
2. `crates/graft-ops/src/operation.rs`: `Operation`, `Target`, `CommitParams`, `StashParams`, `Invocation`, `RepoFacts`, `Refusal`; `kind`, `describe`, `check`, `plan`, `touches`. One `match` per method, one arm per kind; helpers per kind in `operation/{stage,commit,stash}.rs` if an arm passes 40 lines.
3. `crates/graft-ops/src/journal.rs`: line format (`Line { id, status, ...fields }` — two lines per entry, append-only), `parse` (pure, merges Pending + Committed/Aborted by id; a Pending with no second line stays `Pending`), `render_pending`/`render_outcome` (the bytes to append), `next_undo`, `next_redo`, `in_force`, `state_at`.
4. `crates/graft-ops/src/undo.rs`: `Step`, `PathScope`, `InversePlan`, `NotInvertible`, `inverse`, `plan_undo`, `plan_redo`, `Drift`, `drift`.
5. `crates/graft-ops/MAP.md` + `crates/graft-core/MAP.md` lines; `crates/graft-ops/src/lib.rs` module lines; ARCHITECTURE.md "Where to start reading" gets the seam rule sentence from the note.

Out: any `git` spawn, file I/O (the `Journal` handle that opens/appends the file is `ops-execution`), snapshots, the app.

**The rules, decided here** (a builder changing one says so in the report):

- `check` (preconditions → `Refusal`), per kind:
  - all: `facts.bare` → `Bare`; any `status.entries[..].unmerged` → `Unmerged { paths }`; `facts.in_progress.is_some()` → `InProgress(kind)` for Commit/Amend/Stash*/Discard (Stage/Unstage allowed).
  - Stage/Unstage/Discard `Paths`: every path must be in `status` in a state the op applies to (Stage: unstaged or untracked; Unstage: staged; Discard: staged, unstaged or untracked) else `PathNotInState { path }`. `Patch`: the patch's `+++`/`---` path must equal `path` else `PatchPathMismatch`.
  - Commit: no staged entry and `!allow_empty` → `NothingStaged`; subject (first non-blank line) empty → `EmptySubject`. Amend: `head == Unborn` → `UnbornHead`; subject empty → `EmptySubject` (amend with nothing staged is a reword: allowed).
  - StashPush: nothing staged, unstaged or (when `include_untracked`) untracked → `NothingToStash`; `staged_only` and `facts.git < 2.35` → `NeedsGit { feature: "stash --staged", minimum: 2.35 }`.
  - StashApply/Pop/Drop: `facts.stashes[index].commit != commit` → `StashListChanged`; index out of range → `NoSuchStash`. Apply/Pop: any path the stash touches (`StashEntry::paths`) that is staged, unstaged or untracked now → `TouchesChangedFiles { paths }` (this is what keeps M3 out of conflict territory before M6).
- `plan` (typed `Invocation { args: Vec<GitArg>, stdin: Option<Vec<u8>> }`, exact args):
  - Stage `Paths` → `add -- <paths>`; Stage `Patch` → `apply --cached -` (stdin patch); Unstage `Paths` → `restore --staged -- <paths>`, or `rm --cached --quiet -- <paths>` when `head == Unborn`; Unstage `Patch` → `apply --cached -R -`.
  - Discard `Paths`: untracked → `clean -f -- <p>` (`-fd` when the path is a directory); staged-and/or-unstaged tracked → `restore --staged --worktree --source=HEAD -- <p>` (unstaged-only: `restore --worktree -- <p>`); one invocation per class. Discard `Patch` → `apply -R -`.
  - Commit → `commit -F -` + `--amend` (Amend) + `--signoff`/`--no-verify`/`--allow-empty` as flagged; stdin = message with `\n` line ends and one trailing newline. Never `-m`.
  - StashPush → `stash push` + `-u` (include_untracked) + `--staged` (staged_only) + `-m <message>` when given. StashApply → `stash apply --index stash@{<index>}`; StashPop → `stash pop --index stash@{<index>}`; StashDrop → `stash drop stash@{<index>}`.
- `touches`: Stage/Unstage → `{ refs: [], index: true, worktree: false }`; Discard → `{ [], true, true }`; Commit/Amend → `{ refs: [HEAD's branch full name] or ["HEAD"] when detached, index: true, worktree: false }`; StashPush/Pop/Apply → `{ ["refs/stash"], true, true }`; StashDrop → `{ ["refs/stash"], false, false }`.
- `describe`: `"stage src/a.rs"`, `"stage 3 files"`, `"stage a hunk of src/a.rs"`, `"discard src/a.rs"`, `"commit \"<subject>\""`, `"amend \"<subject>\""`, `"stash \"<msg>\""` / `"stash the working tree"`, `"apply stash@{0}"`, `"pop stash@{0}"`, `"drop stash@{0}"`.
- `in_force(entries, id)`: the entry is `Committed` (or `Aborted` with `worktree_before.is_some()`) and no later in-force entry has `reverses == Some(id)`. `next_undo`: newest in-force entry whose kind ∉ {Undo, Redo}. `next_redo`: newest in-force `Undo` entry with no in-force non-Undo/Redo entry newer than it (a new operation after an undo empties redo, §6.5).
- `inverse(entry, original)` → `InversePlan`:
  - generic steps, in order: for each `(name, before)` in `refs_before`: `UpdateRef { name, expect: refs_after[name], to: before }`; `SetHead(head_before)` when `head_after != Some(head_before)`; `RestoreIndex { tree: index_before, paths: ChangedBetween { from: index_after, to: index_before } }` when `touches.index`; `RestoreWorktree { tree: worktree_before, paths: ChangedBetween { from: worktree_after, to: worktree_before } }` when `touches.worktree`.
  - kind extras: StashPush → `StashDrop { commit: refs_after["refs/stash"] }` after the restores; StashPop/StashDrop → `StashStore { commit: pins[0], message }`; StashApply none; Commit/Amend/Stage/Unstage/Discard none.
  - `Undo` entry (redo of it): generic steps from the Undo entry's own before/after, plus the *forward* stash effect of `original` (StashPush → `StashStore(pins[0])`; StashPop/Drop → `StashDrop(pins[0])`). `Redo` entry (undo of a redo): same as undoing `original`. Any entry `Pending` → `NotInvertible::Pending`; `Aborted` without `worktree_before` → `NoBackup`; a kind with no inverse (none in M3; Push arrives in M4) → `NotUndoable(kind)`.
  - The plan's own `touches` = the union of what its steps touch; `description` = `"undo: <original description>"` / `"redo: …"`.
- `drift(entry, now)`: refs where `now.refs[name] != refs_after[name]`; `index: now.index_tree != index_after`; `worktree: now.worktree_tree != worktree_after`. Pure comparison; the fresh trees come from `ops-execution`.
- `state_at(entries, at_unix_ms)`: for every ref any entry touched, the value from the newest entry with `at_unix_ms <= at` that touched it (`refs_after`), else the `refs_before` of the oldest entry after `at` that touched it; same for head/index/worktree; `None` when no entry is at or before `at`. This is ADR-0031's proof that the shape answers; nothing calls it in M3.

## Signatures

```rust
// crates/graft-core/src/journal.rs
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)] pub struct EntryId(String);
impl EntryId { pub fn new(unix_ms: i64, random: [u8; 10]) -> Self; pub fn parse(s: &str) -> Result<Self, Error>; pub fn as_str(&self) -> &str; }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] pub enum OperationKind { Stage, Unstage, Discard, Commit, Amend, StashPush, StashApply, StashPop, StashDrop, Undo, Redo }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] pub enum EntryStatus { Pending, Committed, Aborted { reason: String } }
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)] pub struct Touches { pub refs: Vec<String>, pub index: bool, pub worktree: bool }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] pub enum HeadRef { Branch(String), Detached(Oid), Unborn(String) }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] pub struct JournalEntry { /* fields exactly as the design note's "Journal entry" block */ }

// crates/graft-ops/src/operation.rs
pub enum Operation { /* note */ }   pub enum Target { /* note */ }
pub struct CommitParams { /* note */ }  pub struct StashParams { /* note */ }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Invocation { pub args: Vec<GitArg>, pub stdin: Option<Vec<u8>> }
impl Invocation { pub fn command_line(&self) -> String; }   // `git <args>` shell-quoted, for the Command Log and tests
pub struct RepoFacts { pub bare: bool, pub head: HeadRef, pub in_progress: Option<InProgress>, pub status: WorkingTreeStatus, pub stashes: Vec<StashEntry>, pub git: GitVersion }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum Refusal { Bare, InProgress(InProgress), Unmerged { paths: Vec<PathBuf> }, PathNotInState { path: PathBuf }, PatchPathMismatch, NothingStaged, EmptySubject, UnbornHead, NothingToStash, NeedsGit { feature: &'static str, minimum: GitVersion }, NoSuchStash, StashListChanged, TouchesChangedFiles { paths: Vec<PathBuf> } }
impl Refusal { pub fn explain(&self) -> String; }   // one plain sentence, names the shell command where M5/M6 owns the fix
impl Operation { pub fn kind(&self) -> OperationKind; pub fn describe(&self) -> String; pub fn check(&self, facts: &RepoFacts) -> Result<(), Refusal>; pub fn plan(&self, facts: &RepoFacts) -> Vec<Invocation>; pub fn touches(&self, facts: &RepoFacts) -> Touches; }

// crates/graft-ops/src/journal.rs
pub struct Line { /* one JSONL record: id, status, and the before- or after-fields it carries */ }
pub fn parse(bytes: &[u8]) -> Result<Vec<JournalEntry>, JournalError>;               // pure; tolerant of a torn last line (reported as JournalError::TornTail { line } only if it is not the last)
pub fn render_pending(entry: &JournalEntry) -> Vec<u8>;                              // one line + '\n'
pub fn render_outcome(entry: &JournalEntry) -> Vec<u8>;                              // the Committed/Aborted line from the entry's after-fields
pub fn in_force(entries: &[JournalEntry], id: &EntryId) -> bool;
pub fn next_undo(entries: &[JournalEntry]) -> Option<&JournalEntry>;
pub fn next_redo(entries: &[JournalEntry]) -> Option<&JournalEntry>;
pub struct PointInTime { pub head: Option<HeadRef>, pub refs: BTreeMap<String, Option<Oid>>, pub index: Option<Oid>, pub worktree: Option<Oid> }
pub fn state_at(entries: &[JournalEntry], at_unix_ms: i64) -> Option<PointInTime>;
pub enum JournalError { Json { line: usize, what: String }, TornTail { line: usize }, DuplicatePending(EntryId), OutcomeWithoutPending(EntryId), Io(std::io::Error) }

// crates/graft-ops/src/undo.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub enum Step { UpdateRef { name: String, expect: Option<Oid>, to: Option<Oid> }, SetHead(HeadRef), RestoreIndex { tree: Oid, paths: PathScope }, RestoreWorktree { tree: Oid, paths: PathScope }, StashStore { commit: Oid, message: String }, StashDrop { commit: Oid } }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum PathScope { ChangedBetween { from: Oid, to: Oid } }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct InversePlan { pub reverses: EntryId, pub kind: OperationKind, pub description: String, pub touches: Touches, pub steps: Vec<Step> }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum NotInvertible { Pending, NoBackup, NotUndoable(OperationKind), NothingToUndo, NothingToRedo }
pub fn inverse(entry: &JournalEntry, original: Option<&JournalEntry>) -> Result<InversePlan, NotInvertible>;
pub fn plan_undo(entries: &[JournalEntry]) -> Result<InversePlan, NotInvertible>;
pub fn plan_redo(entries: &[JournalEntry]) -> Result<InversePlan, NotInvertible>;
pub struct RepoStateNow { pub head: HeadRef, pub refs: BTreeMap<String, Option<Oid>>, pub index_tree: Oid, pub worktree_tree: Oid }
#[derive(Clone, Debug, PartialEq, Eq, Default)] pub struct Drift { pub refs: Vec<(String, Option<Oid>, Option<Oid>)>, pub head: bool, pub index: bool, pub worktree: bool }
impl Drift { pub fn is_clean(&self) -> bool; pub fn explain(&self) -> String; }
pub fn drift(entry: &JournalEntry, now: &RepoStateNow) -> Drift;
```

## Acceptance

- [x] `plan` table test: every kind × the arm-relevant facts → the exact `command_line()` strings listed under "The rules" (e.g. `git commit -F - --signoff`, `git stash push -u -m "wip"`, `git restore --staged --worktree --source=HEAD -- src/a.rs`, `git rm --cached --quiet -- a` on an unborn head).
- [x] `check` table test: each `Refusal` variant reachable by one fact change from a passing baseline; `Stage` is allowed mid-rebase, `Commit` is not; `StashApply` refuses when a touched path is dirty and passes when the dirty path is elsewhere.
- [x] Journal round-trip: three entries rendered as pending+outcome lines, parsed back `==` the originals; a file whose last line is torn parses the rest; an outcome line for an unknown id is `OutcomeWithoutPending`.
- [x] Cursor: `[A, B, Undo(B)]` → `next_undo = A`, `next_redo = Undo(B)`; `[A, B, Undo(B), C]` → `next_undo = C`, `next_redo = None`; `[A, B, Undo(B), Redo(B)]` → `next_undo = B`; `[Aborted(no backup)]` → `NothingToUndo`; `[Aborted(with backup)]` → it.
- [x] `inverse` table test per kind: the exact `steps` vectors (commit: `UpdateRef` CAS + `RestoreIndex`; discard: index + worktree restores; stash push: restores + `StashDrop`; stash drop: `StashStore`; the redo of an undone stash pop is `StashDrop(pins[0])` after the generic steps).
- [x] `state_at`: entries at t=10, 20, 30 touching `refs/heads/main`; `state_at(25)` = entry 20's after-state; `state_at(5)` = `None`; a ref touched only at t=30 reads its `refs_before` at 25.
- [x] `drift`: a moved ref, a changed index tree and a changed worktree tree each set exactly their field; `explain` names them (`"the index changed"`).
- [x] Every `Refusal::explain` is a sentence a person can act on; the mid-operation one names `git rebase --continue` / `--abort`.
- [x] MAP.md updated for core and ops; edge test passes (ops → git, core only; serde is external).

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test.

## Size

Core ~120 lines; ops ~700 (operation 350, journal 200, undo 150) + ~500 test. Pure, no fixtures. Named seams: after piece 2 (operation), after piece 3 (journal). A piece past 60 turns commits and says why.

## NOTES (builder-fable, 2026-09-19)

Built in four commits at the named seams (core → operation → journal → undo), then MAP.md/ARCHITECTURE. No compaction. 47 new tests (4 core, 19 operation, 11 journal, 13 undo); gates green.

**Git behaviour verified in a throwaway repo (git 2.54, scripts under `target/verify-ops-model*.sh`, untracked):** `rm --cached --quiet` unstages on an unborn head while `restore --staged` fails there; `commit -F -` and `--amend -F -` (reword with nothing staged) work; `restore --staged --worktree --source=HEAD` handles a staged modify, a staged **new** file (removes it) and a staged rename (both paths); `restore --worktree` handles unstaged modify/delete; `clean -f -- <dir>` removes a named untracked directory (git: `-d` is irrelevant with a pathspec); `stash push -u/--staged/-m`, `apply --index`, `pop --index`, `drop`, `store -m` all work; a stash commit's subject is byte-identical to its reflog message and `stash store -m <subject>` re-lists it identically; dropping the last stash deletes `refs/stash`; a failed `stash pop` keeps its entry; `update-ref` CAS with the zero id works.

**Rules decided here that differ from the spec text (say so, per the spec):**
1. `parse` returns `Parsed { entries, torn_tail }`, not `Vec<JournalEntry>`: a torn last line is reported, not swallowed. `render_*` return `Result` (`StillPending`, serde). `JournalError` gains `DuplicateOutcome`, `StillPending`.
2. `refs/stash` never gets an `UpdateRef`: the reflog is the stash list, and `update-ref` would add a reflog entry instead of removing one. `StashStore`/`StashDrop` carry only `commit` and are defined idempotent by commit (store = no-op if the reflog has it; drop = no-op if absent), so an `Aborted` pop (git keeps the entry) undoes correctly. `StashStore` has no `message`: the executor uses the commit's subject (verified equal). `pins` live on the pending line (known before the op; the backup ref must point at them).
3. `UpdateRef` emitted only where before ≠ after. `refs_after` missing a key of `refs_before` → `NotInvertible::Incomplete { field }` (new variant) — the executor must record `refs_after` on abort too. Missing after-**tree** → `PathScope::Everything` (new variant) from the before tree, so an op that died before its after-state could be read is still undoable.
4. `inverse(entry, original)`: for `Undo`/`Redo` entries `original` is the **root operation** (`journal::root_operation` follows the chain); `reverses` always names the entry directly reversed (Redo → the Undo).
5. `drift`: a field never recorded cannot drift (the snapshot is the guarantee, ADR-0009).
6. `clean -f` always, never `-fd`: with `--untracked-files=all` status never lists a directory, so `check` refuses a directory path; with a pathspec `-d` is irrelevant anyway.
7. Discard on an unborn head: `rm --cached --quiet -- <staged>` then `clean -f -- <staged ∪ untracked>` (no `HEAD` to restore from). A rename's `original_path` joins every pathspec.
8. `graft_git::run::command_line(&[GitArg])` extracted as a free fn (the method delegates) so `Invocation::command_line` shares the runner's quoting.

**For the execution group (03):** outcome line must carry `refs_after` for every key of `refs_before`, `head_after`, and the trees it could read, on success **and** abort; `Everything` scope means "restore the whole tree from the backup"; `Step` order is refs → `HEAD` → index → worktree → stash reflog.

**KNOWN_GAPS candidates (conductor's file):** `StashStore` re-lists a restored stash at `stash@{0}` even if it was `stash@{2}` when popped/dropped (git has no insert-at-index); `StashList`'s `commit` pin catches the reorder. Multi-line stash messages: reflog keeps one line; restore uses the subject only.
