# ops-execution: snapshots, the apply protocol, restore, undo and redo — proven byte-for-byte on fixtures

Status: ready
Blocked by: 01-git-reads-and-patches, 02-ops-model
Build with: Opus — git writes and the undo path (CLAUDE.md routes both to Opus); correctness is judged by the byte-for-byte tests here, so not Fable.
Needs: `ops-model` (`Operation`, `RepoFacts`, `Refusal`, `JournalEntry`, `parse`/`render_*`, `next_undo`/`next_redo`, `inverse`/`plan_undo`/`plan_redo`, `drift`, `Step`), `git-reads-and-patches` (`status::read`, `stash::list`, `state::in_progress`, `RepoHandle::{git_path, worktree_name}`, `GitInvocation::env`, `version::system_git`).

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — Seam rule; Journal entry block (backup commit shape); Hard choices 1–3; Proposed changes 1, 2; Genuinely risky groups
- `docs/design/06-undo-journal-design.md` §6.2 (recording protocol), §6.3 (snapshot — read with the note's change 1), §6.4 "Undo snapshots before it restores", §6.7, §6.8
- `docs/adr/0009`, `docs/adr/0010` (whole; the linked-worktree test it asks for is here)
- `PROMPT.md` §3 "Undo and redo" (lines 240–252: the untracked-file test), §7.2 item 4
- `KNOWN_GAPS.md` "Unconditional snapshot cost on high-frequency index ops (R2-3)" (line 63)
- `.scratch/m3-working-tree-undo-and-stash/issues/ops-model.md` — "The rules" (inverse steps) and Signatures
- `crates/graft-ops/MAP.md`; `crates/graft-git/MAP.md` §run, §repo, §status, §stash, §state; `crates/graft-fixtures/MAP.md`

## Scope

In — in this order, one commit each, turns reported at each:

1. `crates/graft-ops/src/journal/file.rs` (or `journal.rs` growing a `Journal` handle): `Journal::open(path)` creates `<git-path graft>/` and the file if missing, parses it; `append_pending`/`append_outcome` write one line with `O_APPEND` + `fsync`; `unresolved()`; `mark_interrupted(id)` appends `Aborted { reason: "interrupted" }` for a Pending entry found at open.
2. `snapshot.rs`: `index_tree` — copy the real index file to `<git-path graft>/tmp-index-<ulid>`, `GIT_INDEX_FILE=<tmp> git write-tree` (never `write-tree` on the real index: it rewrites the cache-tree extension). `worktree_tree` — same copy, `GIT_INDEX_FILE=<tmp> git add -A --` (stat cache kept, so only dirty files hash; blobs land in the odb), `write-tree`. `take` — both trees, `mktree` of `040000 tree <index>\tindex\n040000 tree <worktree>\tworktree`, `commit-tree <tree> [-p <parent>]…` with `GIT_AUTHOR_NAME=graft GIT_AUTHOR_EMAIL=graft@backup GIT_COMMITTER_*` env and message `graft <ulid> <kind>: <description>`, `update-ref refs/graft/backup/<worktree>/<ulid> <commit>`. Temp index removed in every path (a `Drop` guard). `scope(&Touches) -> SnapshotScope` is the one function hard choice 3 names: today `Full` whenever `index || worktree`, `None` otherwise.
3. `facts.rs`: `read_facts(repo, stop)` = `status::read` + `RepoHandle::head` + `state::in_progress` + `stash::list` + `system_git` (cached per `Context`) + `bare`. `read_state_now(repo)` for `drift`: head, the refs the entry names, fresh index/worktree trees (a second `take`-less pair of `write-tree`s).
4. `apply.rs`: the protocol — `read_facts` → `op.check` → build the entry (`EntryId::new(now, random)`, before-fields: head, `refs_before` for `touches.refs` read via `RepoHandle::references`, `index_before`/`worktree_before` from the snapshot when `scope != None`) → `append_pending` → run each `Invocation` through `GitInvocation` (`stdin`, `stop`; `commands` = `command_line()`s) → on `Ok`: after-fields (`head_after`, `refs_after`, `index_after` = fresh `index_tree` when `touches.index` else `index_before`, `worktree_after` = fresh `worktree_tree` when `touches.worktree` else `worktree_before`; when `touches.worktree`, a second backup commit with the before-commit as parent, ref moved to it), `pins` (StashPop/Drop: the popped/dropped commit; StashPush: none), `append_outcome(Committed)`; on `Err`: `append_outcome(Aborted { reason: stderr first line })`, return `ApplyError::Git { entry, error, recoverable: entry.worktree_before.is_some() }`. `Refresh::Everything` when `touches.refs` is non-empty, else `Refresh::WorkingTree`.
5. `restore.rs`: `execute(ctx, plan)` — journaled through the same protocol (an `Undo`/`Redo` entry with `reverses`, its own before-snapshot when the plan touches the worktree — §6.4), steps in order:
   - `UpdateRef { name, expect, to }` → `update-ref <name> <to> <expect>` (`-d <name> <expect>` when `to` is `None`; `expect` `None` → the zero id); a CAS failure aborts the plan with `RestoreError::RefMoved { name }` before any worktree step runs (refs first, so nothing is half-done).
   - `SetHead(Branch(b))` → `symbolic-ref HEAD refs/heads/<b>`; `Detached(o)` → `update-ref --no-deref HEAD <o>`; `Unborn(b)` → `symbolic-ref HEAD refs/heads/<b>` (the branch ref is deleted by its own `UpdateRef`).
   - `RestoreIndex { tree, paths: ChangedBetween { from, to } }` → paths = `diff-tree -r --name-only -z <from> <to>`; `ls-tree -r -z <tree> -- <paths>` → `update-index --index-info` (stdin: `<mode> <oid>\t<path>` per present path, `0 0000000000000000000000000000000000000000\t<path>` per absent one). Never `read-tree` on the real index.
   - `RestoreWorktree { tree, paths }` → paths as above; tracked-now paths present in `tree`: `restore --source=<tree> --worktree -- <paths>`; paths present in `tree` but untracked now: `GIT_INDEX_FILE=<tmp> read-tree <tree>` then `checkout-index -f -- <paths>` from the temp index; paths absent in `tree`: tracked → `restore --source=<tree> --worktree -- <p>` (git removes a tracked path missing from the source), untracked → `clean -f -- <p>`. Nothing outside `paths` is touched.
   - `StashStore { commit, message }` → `stash store -m <message> <commit>`; `StashDrop { commit }` → find `n` by `reflog show --format=%H refs/stash`, `stash drop stash@{n}`; not found → `RestoreError::StashGone`.
6. `undo`/`redo` entry points: `plan_undo(entries)` → `read_state_now` → `drift` → `Err(UndoError::Drift(d))` unless `force` → `execute`. Same for redo.
7. `lib.rs` module lines, `crates/graft-ops/MAP.md`, `ARCHITECTURE.md` "ops" row and the seam rule; `scripts/m3-snapshot-cost.sh` writing `docs/perf/m3-snapshot.txt` (heavy fixture, `Scale::HEAVY` with 2,000 untracked files: time of `take`, run at review, never in a helper's context).

Out: any UI, `Command`/`Product` wiring (`changes-pane` and `undo-and-stash-ui`), the notify watcher, push.

**Verify in a throwaway repo before coding** — each is a git behaviour this design leans on; the M2 rounds 1c–1f were lost to trusting docs: (a) `GIT_INDEX_FILE=<copy> git add -A` re-hashes only stat-dirty files (time it on a 5k-file tree: copy vs fresh `read-tree`); (b) `git restore --source=<tree> --worktree -- <p>` deletes a tracked `p` absent from `<tree>`; (c) `restore --source` on an untracked path fails (hence the temp-index `checkout-index` route); (d) `update-ref <name> <new> <old>` fails when the ref moved; `update-ref -d <name> <old>` likewise; (e) `stash store` re-lists the entry at `stash@{0}`; `stash drop stash@{n}` renumbers; (f) `commit-tree` needs author env when `user.name` is unset (tests run with `GIT_CONFIG_GLOBAL=/dev/null`); (g) `git stash pop --index` on a clean touched-path set succeeds and restores staged state.

## Signatures

```rust
// crates/graft-ops/src/journal.rs (handle; the pure fns are ops-model's)
pub struct Journal { /* path, entries */ }
impl Journal {
    pub fn open(path: &Path) -> Result<Journal, JournalError>;
    pub fn entries(&self) -> &[JournalEntry];
    pub fn append_pending(&mut self, entry: &JournalEntry) -> Result<(), JournalError>;
    pub fn append_outcome(&mut self, entry: &JournalEntry) -> Result<(), JournalError>;
    pub fn unresolved(&self) -> Vec<&JournalEntry>;
    pub fn mark_interrupted(&mut self, id: &EntryId) -> Result<(), JournalError>;
}
pub fn journal_path(repo: &RepoHandle) -> Result<PathBuf, RunError>;   // git_path("graft/undo-journal.jsonl")

// crates/graft-ops/src/snapshot.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum SnapshotScope { None, Full }
pub fn scope(touches: &Touches) -> SnapshotScope;
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Snapshot { pub index_tree: Oid, pub worktree_tree: Oid, pub commit: Oid }
pub fn index_tree(repo: &RepoHandle, scratch: &Path, stop: &dyn Stop) -> Result<Oid, SnapshotError>;
pub fn worktree_tree(repo: &RepoHandle, scratch: &Path, stop: &dyn Stop) -> Result<Oid, SnapshotError>;
pub fn take(repo: &RepoHandle, scratch: &Path, id: &EntryId, worktree: &str, label: &str, parents: &[Oid], stop: &dyn Stop) -> Result<Snapshot, SnapshotError>;
pub fn backup_ref_name(worktree: &str, id: &EntryId) -> String;   // refs/graft/backup/<worktree>/<ulid>
pub enum SnapshotError { Run(RunError), Io(std::io::Error), Stopped, NoIndex }

// crates/graft-ops/src/facts.rs
pub fn read_facts(repo: &RepoHandle, git: GitVersion, stop: &dyn Stop) -> Result<RepoFacts, FactsError>;
pub fn read_state_now(repo: &RepoHandle, scratch: &Path, refs: &[String], stop: &dyn Stop) -> Result<RepoStateNow, FactsError>;
pub enum FactsError { Status(StatusError), Head(HeadError), Refs(RefsError), Stash(StashError), State(std::io::Error), Snapshot(SnapshotError), Stopped }

// crates/graft-ops/src/apply.rs
pub struct Context<'a> { pub repo: &'a RepoHandle, pub git: GitVersion, pub worktree: String, pub scratch: PathBuf /* <git-path graft>/ */, pub journal: &'a mut Journal, pub now_unix_ms: i64, pub random: [u8; 10], pub stop: &'a dyn Stop }
impl<'a> Context<'a> { pub fn open(repo: &'a RepoHandle, git: GitVersion, journal: &'a mut Journal, now_unix_ms: i64, random: [u8; 10], stop: &'a dyn Stop) -> Result<Self, ApplyError>; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Refresh { WorkingTree, Everything }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Applied { pub entry: JournalEntry, pub refresh: Refresh }
pub enum ApplyError { Refused(Refusal), Facts(FactsError), Snapshot(SnapshotError), Journal(JournalError), Git { entry: JournalEntry, error: RunError, recoverable: bool }, Stopped }
impl ApplyError { pub fn explain(&self) -> String; }   // one line for the status line; `Git` = "<describe>: <stderr first line>" + " · u restores the backup" when recoverable
pub fn apply(ctx: &mut Context, op: &Operation) -> Result<Applied, ApplyError>;

// crates/graft-ops/src/restore.rs
pub enum RestoreError { RefMoved { name: String }, StashGone { commit: Oid }, Run(RunError), Facts(FactsError), Snapshot(SnapshotError), Journal(JournalError), Stopped }
pub fn execute(ctx: &mut Context, plan: &InversePlan) -> Result<Applied, RestoreError>;

// crates/graft-ops/src/undo.rs (execution half; planning is ops-model's)
pub enum UndoError { Nothing(NotInvertible), Drift(Drift), Restore(RestoreError), Facts(FactsError) }
impl UndoError { pub fn explain(&self) -> String; }
pub fn undo(ctx: &mut Context, force: bool) -> Result<Applied, UndoError>;
pub fn redo(ctx: &mut Context, force: bool) -> Result<Applied, UndoError>;
```

## Acceptance

All on `copy_into_temp` fixtures with `GIT_CONFIG_GLOBAL=/dev/null` (the fixtures runner already does), each asserting **repository state by reading git**, never by trusting the return value:

- [ ] **Parity, every kind:** for each `Operation` kind, apply on copy A while the test runs the plan's `command_line()` on copy B; `for-each-ref`, `ls-files -s` and `status --porcelain=v2` are byte-identical between A and B (backup refs excluded).
- [ ] **Undo restores exactly, every kind:** snapshot (`for-each-ref` sans `refs/graft`, `ls-files -s`, every worktree file's bytes incl. untracked) before the op equals the snapshot after `apply` + `undo`. Redo then equals the post-op snapshot. Then undo again.
- [ ] **The PROMPT §3 test verbatim:** create an untracked file with binary bytes (`[0, 255, 10, 13]…`), `Discard` it, `undo`, bytes come back exactly; the same with an **unstaged edit to a tracked file** (note change 1) and with a **staged + unstaged** file (both index and worktree bytes).
- [ ] **Path scoping:** after `Discard(a.rs)` the user creates `new.txt`; `undo` restores `a.rs` and leaves `new.txt` untouched.
- [ ] **Failed midway is recoverable:** a `StashPop` whose `git` fails (simulate with a stash whose apply conflicts after the precondition passes — scripted by editing a touched file *between* `read_facts` and the run via a test hook on `Context`, or by a hook-free stash scripted to conflict) leaves an `Aborted` entry with `worktree_before`; `undo` restores the pre-pop tree.
- [ ] **Crash between Pending and outcome:** write a Pending line by hand, `Journal::open` reports it in `unresolved()`, `mark_interrupted` appends the Aborted line, and `undo` of it restores the backup.
- [ ] **CAS guard:** move the branch with `git update-ref` after a `Commit`; `undo` returns `UndoError::Drift` naming the ref; with `force` it returns `RestoreError::RefMoved` and touches nothing else.
- [ ] **Drift guard:** edit a file after a `Commit`; `undo` returns `Drift { worktree: true }`; `force` proceeds and the undo entry's own backup holds the edit.
- [ ] **ADR-0010 linked worktree:** `git worktree add`, apply a `Stage` there; the journal lands under `.git/worktrees/<name>/graft/undo-journal.jsonl`, the main worktree's journal has no entry, the backup ref is `refs/graft/backup/<name>/<ulid>`.
- [ ] **Stash inverses:** push → undo (stash list back to N, tree back), pop → undo (`stash@{0}` is the popped commit again, tree back), drop → undo (entry back; its commit was reachable via the backup ref throughout — assert `git cat-file -e` after a `git gc --prune=now` between drop and undo).
- [ ] **Unborn head:** first commit on a fresh `git init` copy; undo deletes the branch, HEAD is unborn again, index restored.
- [ ] **Index bytes:** `apply(Stage)` is the *only* thing that changes `.git/index`; `read_facts` and `take` leave it byte-identical.
- [ ] Backup commit: `git ls-tree <commit>` shows exactly `index` and `worktree` trees; a worktree-touching op's ref points at an after-commit whose parent is the before-commit.
- [ ] `docs/perf/m3-snapshot.txt` script exists; not run by the builder.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test. Fast tier stays under 3 min (KNOWN_GAPS line 463: it was 24 s; report the new figure).

## Size

~900 lines (journal file 120, snapshot 200, facts 100, apply 220, restore 260) + ~700 test. Seams: after piece 2 (snapshots + journal file, testable alone), after piece 4 (apply without undo), after piece 5. Each piece commits; a piece past 60 turns says why. Scout for `RepoHandle::references` and `head` usage rather than reading `repo.rs`.
