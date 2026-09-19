# ops-execution: snapshots, the apply protocol, restore, undo and redo — proven byte-for-byte on fixtures

Status: done
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

- [x] **Parity, every kind:** for each `Operation` kind, apply on copy A while the test runs the plan's `command_line()` on copy B; `for-each-ref`, `ls-files -s` and `status --porcelain=v2` are byte-identical between A and B (backup refs excluded).
- [x] **Undo restores exactly, every kind:** snapshot (`for-each-ref` sans `refs/graft`, `ls-files -s`, every worktree file's bytes incl. untracked) before the op equals the snapshot after `apply` + `undo`. Redo then equals the post-op snapshot. Then undo again.
- [x] **The PROMPT §3 test verbatim:** create an untracked file with binary bytes (`[0, 255, 10, 13]…`), `Discard` it, `undo`, bytes come back exactly; the same with an **unstaged edit to a tracked file** (note change 1) and with a **staged + unstaged** file (both index and worktree bytes).
- [x] **Path scoping:** after `Discard(a.rs)` the user creates `new.txt`; `undo` restores `a.rs` and leaves `new.txt` untouched.
- [x] **Failed midway is recoverable:** a `StashPop` whose `git` fails (simulate with a stash whose apply conflicts after the precondition passes — scripted by editing a touched file *between* `read_facts` and the run via a test hook on `Context`, or by a hook-free stash scripted to conflict) leaves an `Aborted` entry with `worktree_before`; `undo` restores the pre-pop tree.
- [x] **Crash between Pending and outcome:** write a Pending line by hand, `Journal::open` reports it in `unresolved()`, `mark_interrupted` appends the Aborted line, and `undo` of it restores the backup.
- [x] **CAS guard:** move the branch with `git update-ref` after a `Commit`; `undo` returns `UndoError::Drift` naming the ref; with `force` it returns `RestoreError::RefMoved` and touches nothing else.
- [x] **Drift guard:** edit a file after a `Commit`; `undo` returns `Drift { worktree: true }`; `force` proceeds and the undo entry's own backup holds the edit.
- [x] **ADR-0010 linked worktree:** `git worktree add`, apply a `Stage` there; the journal lands under `.git/worktrees/<name>/graft/undo-journal.jsonl`, the main worktree's journal has no entry, the backup ref is `refs/graft/backup/<name>/<ulid>`.
- [x] **Stash inverses:** push → undo (stash list back to N, tree back), pop → undo (`stash@{0}` is the popped commit again, tree back), drop → undo (entry back; its commit was reachable via the backup ref throughout — assert `git cat-file -e` after a `git gc --prune=now` between drop and undo).
- [x] **Unborn head:** first commit on a fresh `git init` copy; undo deletes the branch, HEAD is unborn again, index restored.
- [x] **Index bytes:** `apply(Stage)` is the *only* thing that changes `.git/index`; `read_facts` and `take` leave it byte-identical.
- [x] Backup commit: `git ls-tree <commit>` shows exactly `index` and `worktree` trees; a worktree-touching op's ref points at an after-commit whose parent is the before-commit.
- [x] `docs/perf/m3-snapshot.txt` script exists; not run by the builder.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · edge test. Fast tier stays under 3 min (KNOWN_GAPS line 463: it was 24 s; report the new figure).

## Size

~900 lines (journal file 120, snapshot 200, facts 100, apply 220, restore 260) + ~700 test. Seams: after piece 2 (snapshots + journal file, testable alone), after piece 4 (apply without undo), after piece 5. Each piece commits; a piece past 60 turns says why. Scout for `RepoHandle::references` and `head` usage rather than reading `repo.rs`.

## NOTES

Built on git **2.54.0 (Apple Git-157)**, macOS (Darwin 27.0.0). Every "verify
first" in the spec was checked in a throwaway repo on that version. **2.38,
the project's stated minimum, is unproven** — nothing here was run against it.

Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` all green. `graft-ops`'s `it` binary: **49 passed, 0
failed**; whole fast tier **~42 s** wall (KNOWN_GAPS line 463 recorded 24 s;
still far under the 3-minute budget).

### Deviations from the spec text, and why

1. **`Snapshot.index_tree` is `Option<Oid>`, not `Oid`.** An unmerged index has
   no tree — `git write-tree` refuses it — so a conflicting `stash pop` made
   the after-state read, the drift read *and* the undo's own backup all fail:
   undo of a failed pop was impossible, which is exactly the case the design
   note's Proposed change 2 names. Fixed at the cause: `SnapshotError::Unmerged`,
   `take` falls back to a **worktree-only** backup (the `add -A` over the index
   copy resolves the conflict, so the conflicted working tree, markers and all,
   is still captured), `commit_trees`/`mktree` take `Option`, and
   `undo::reverse` reads an unmerged index as `Drift { index: true }` — forcing
   past it works. Verified beforehand in a throwaway repo that `add -A` over
   the copy resolves the stages and that `update-index --index-info` clears them.
2. **Pins force a backup** even when `touches` is refs-only. `StashDrop`'s
   `touches` is `{["refs/stash"]}`, so `scope` says `None` — but then nothing
   keeps the dropped stash commit alive and the acceptance criterion
   (survive `git gc --prune=now`) cannot hold. `begin` takes a backup when
   `scope == Full || !pins.is_empty()`.
3. **`update-ref` "must not exist" uses git's empty old value (`""`), not an
   all-zero id.** Hash-length independent (sha256 repos). Note the asymmetry
   written into `restore.rs`: with `-d`, `""` means *no* expectation, so a
   delete step must always carry `Some(expect)`.
4. **`StashGone` was repurposed.** Contract rule 2 makes `StashDrop` a no-op
   when the reflog lacks the commit, so "gone" can no longer mean that;
   it now means the stash **commit object itself** is unreadable.
5. **`Journal::open` truncates a torn last line** rather than only reporting
   it. Otherwise the next append splices onto it and every later parse refuses
   the whole file. `torn_tail()` still reports the line number.
6. **`mark_interrupted` takes the recovered after-state.** Closing a crashed
   entry with no after-fields would leave it `Incomplete` and in force for
   ever; reading the repository now keeps the restore narrow when it is
   readable (`recovery_that_can_read_the_repository_keeps_the_restore_narrow`)
   and honestly `None` when it is not.
7. **`append_outcome` writes the file first, then swaps the held copy**, so a
   failed write never leaves the two disagreeing.
8. **`take` uses one index copy for both trees** (`write-tree`, then `add -A`,
   then `write-tree`), not two. Keeps git's stat cache: **15 ms against 236 ms**
   for a fresh `read-tree` on 5,000 files, same tree bytes.
9. **`restore_worktree` classifies paths four ways** (tracked → `restore
   --source`; untracked but in the tree → temp index + `checkout-index`;
   untracked and absent from the tree → `clean -f`), because `restore
   --worktree` cannot create a path git does not track.
10. **`RestoreIndex` is `update-index --index-info`**, never `read-tree` on the
    real index — a `read-tree` would blow away the stat cache and the paths
    outside the scope.
11. **`ApplyError::Git` boxes the entry** (`Box<JournalEntry>`); clippy's
    `result_large_err` otherwise.
12. **Parity covers 6 of 9 kinds** — stage, unstage, discard, discard-untracked,
    commit, stash push. `Amend`, `StashApply`, `StashPop`, `StashDrop` are
    covered by round-trip and inverse tests but not by the A/B command-line
    parity harness. Filed below.
13. **Parity compares `%(refname) %(tree)`, not `%(objectname)`.** Two copies
    committing a second apart hold different commit ids; the content is what
    parity is about. (Caught as a flake under `--workspace`.)
14. **`support.rs` and `ops_execution_hard.rs` carry file-scoped
    `#![allow(clippy::expect_used, clippy::unwrap_used)]`** with a written
    reason: `clippy.toml`'s test allowance only reaches `#[test]` functions,
    not plain helpers.
15. **A field that could not be read is recorded as `None`**, with a comment at
    each site saying why that is the honest record and not a swallowed error:
    `drift` reports it as `unknown`, which is *not* clean, so the restore
    widens and the user is asked first.

### `PathScope::Everything` — reached

`an_entry_with_no_after_state_restores_the_whole_tree_after_asking` builds an
entry whose outcome has no after-tree (the crash-mid-write shape), then undoes
it: `drift` reports `unknown`, the undo refuses, forcing it restores the whole
tree from the backup. `scoped_paths` for `Everything` + the index side takes
the **union** of the backup tree's paths and what is tracked now (no "now"
tree, so an unmerged index is fine); the worktree side diffs against a fresh
`worktree_tree`. The KNOWN_GAPS "unverified" line for `Everything` can close.

### Stash reflog facts, now in-tree

The four M5 facts that were only ever proved by untracked scripts under
`target/` are tests in `ops_execution_hard.rs`:
`a_stash_commits_subject_is_its_reflog_message`,
`storing_a_commit_the_reflog_already_holds_below_the_top_duplicates_it`,
`dropping_the_last_stash_removes_the_ref_entirely`, `a_failed_pop_keeps_its_entry`.

### For KNOWN_GAPS (conductor's file — not touched)

- **Unverified:** git 2.38–2.53. Every behaviour here was proved on 2.54 only.
- **Unverified:** `Amend`, `StashApply`, `StashPop`, `StashDrop` have no
  command-line parity test (deviation 12).
- **Unverified:** `docs/perf/m3-snapshot.txt` — `scripts/m3-snapshot-cost.sh`
  is written and committed but **not run** (measurement never runs in a
  builder's context). The 15 ms/236 ms figure above is from a throwaway
  5,000-file repo, not the heavy fixture.
- **Close:** `PathScope::Everything` unreached-by-any-fixture.
- Nothing is **broken**.

---

## Review fix round (2026-09-19, `builder-opus`)

One round against `docs/reviews/m3-03-ops-execution.md`. Critical and every
Major fixed at the cause; Minors 1–7 and 9–10 taken while the file was open.

### C1 — unanchored after-state (cause)

`finish` sealed an after-commit only for worktree operations with both trees
present, so an index-only op's `index_after`, a conflicted pop's
`worktree_after` and an amend's pre-amend commit hung off nothing.

- `begin` now parents the **before**-commit on the pins *and* every commit the
  entry's before-fields name (`anchors`): that is the pre-amend commit, and the
  branch tip a commit moves off.
- `finish` seals **whenever there is a backup ref**, over whichever trees were
  readable, parented on the before-commit and on every commit the after-fields
  name (so a redo after an undo still has its commit). `snapshot::commit_trees`
  takes `worktree_tree: Option<Oid>` for the same reason `index_tree` is one.
- A seal that fails drops only the freshly-read fields, never the before-ones:
  an entry must not name a tree nothing reaches.
- `recover_interrupted` seals the recovered after-state **before** it is
  recorded, and records nothing when the seal fails — the same bug one level up,
  which the review did not name.

Proof: four tests in `ops_execution_hard.rs` run
`reflog expire --expire-unreachable=now --all && gc --prune=now` between the
operation and the undo (stage, amend, commit-then-undo-then-redo, conflicted
pop). All four fail against the old sealing rule (verified by restoring it);
all four pass now.

### Majors

- **M1** No `unwrap_or_default`/`.ok()` anywhere in `finish`. Every after-read
  failure comes back as `ApplyError::AfterState { applied, unread }` on a
  success, and is appended to the `Aborted` reason on a failure. `applied` is
  inside the error so the caller still refreshes: the write did happen.
  `recover_interrupted` returns `Recovered { closed, unreadable }`. The
  after-reads take `&()`, never `ctx.stop` — a cancel mid-plan no longer makes
  a weaker entry, and a test asserts it.
- **M2** `StashApply`/`StashPop`/`StashDrop` round-trip (apply → undo → redo →
  undo) and all four missing kinds are in the parity harness, which now takes
  `fn(&TempRepo) -> Operation` so a stash op can name the commit it found.
- **M3** Two multi-invocation tests: one stopped between invocations, one whose
  first invocation fails. `StopAfter` is **deleted**: counting `should_stop`
  polls is a different stop point on every machine. `StopWhenGone` replaces it —
  it fires on the plan's own side effect, so the second invocation is the one
  killed, every time. `support.rs` no longer carries `#![allow(dead_code)]`.
- **M4** A malformed `ls-tree` record is `RestoreError::Run` with the record,
  as `stash_reflog` already did. It cannot silently delete a staged entry.
- **M5** `read_state_now` returns `index_tree: Option<Oid>`; the `Unmerged`
  short-circuit in `reverse` is gone, so `HEAD`, refs and the working tree are
  compared and reported even on a conflicted index.
- **M6** `RepoHandle::reference(name)` added; `facts::read_refs` resolves each
  name directly instead of reading the whole ref store three times per op.

### Minors taken

1 (`Journal { error, after }` carries git's failure), 2 (`explain` calls
`reason_for`), 3 (`Context::scratch()` everywhere, no clone), 4 (`eprintln!`
not `debug_assert!` in `Drop`), 5 (a non-UTF-8 symbolic target errors), 6
(`StashGone` only on `Exited`), 7 (`RefMoved` on git's `but expected` /
`reference already exists`), 9 and 10 (said in MAP.md).

**Not taken: Minor 8** — `begin` still re-reads `HEAD`. Passing `facts.head` in
would mean `restore::execute`, which has no `RepoFacts`, carrying an `Option`
through `begin` for one saved read. Left as it is, deliberately.

### Deviations 12 and 15, contested

- **12 — conceded.** Parity now covers all nine kinds, and `StashApply` has
  round-trip and redo coverage. The original note's claim that the four kinds
  were "covered by round-trip and inverse tests" was wrong for `StashApply`.
- **15 — conceded, and wider than the review asked.** `None` for a field the
  repository would not give up is still the honest *record* — `drift` reports it
  as unknown and the user is asked. What was wrong is that the *reason* was
  dropped at the same line. No field is now set to `None` without the failure
  travelling back to the caller, refs included.

### Heavy-tier guard

`many_refs::reading_tips_from_50k_real_refs_is_within_budget` records rather
than asserts under `cfg!(debug_assertions)`, like `frames.rs` and `heavy.rs`
(ADR-0021). The 200 ms constant is unchanged and the measured number is still
printed; `--release` still fails over it.

### For KNOWN_GAPS (conductor's file — not touched)

- **Close:** C1, both halves. **Close:** the 50k-refs debug guard row (138–140 ms
  isolated debug against 200 ms; now record-not-assert in debug). **Close:**
  `StashApply` execution, redo of an undone pop/drop, multi-invocation plans,
  parity for the four kinds — all now tested.
- **Limitation, not broken (M5a):** redo of an undone *conflicted* pop restores
  the conflict markers with a clean index, a state git itself never produces.
  Recording stage-1/2/3 entries needs a different backup shape; not M3's.
- **Unverified:** git 2.38–2.53 — everything here is still proved on 2.54 only,
  including the two new `stderr` matches in `RefMoved`.
- **Unverified:** a second Graft process on the same worktree. `Journal::open`
  truncates a torn tail, so it is a writing call; two in-memory copies would
  diverge and `append_outcome`'s duplicate check is per process.
- **Unverified:** `docs/perf/m3-snapshot.txt` is still unrun. An undo pays three
  snapshots and an apply of a worktree op two, so on a 20k-file repository `u`
  is roughly 600 ms warm, off-thread.
