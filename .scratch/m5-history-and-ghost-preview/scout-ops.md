# Scout: graft-ops and graft-graph layout

Sonnet `Explore`, 2026-09-20, for M5's pass 2 specs. Reports what the code **is**.
Written to this file by the conductor: `Explore` is read-only and could not write it.

## 1. graft-ops

### `Operation` (`crates/graft-ops/src/operation.rs`)

12 variants: `Stage(Target)`, `Unstage(Target)`, `Discard(Target)` — `Target` is
`Paths(Vec<PathBuf>)` or `Patch { path, patch, side: PatchSide }`; `Commit(CommitParams)`,
`Amend(CommitParams)` — `{ message, sign_off, no_verify, allow_empty }`;
`StashPush(StashParams)` `{ message, scope }`, `StashApply/StashPop/StashDrop { index, commit }`;
`Fetch(FetchParams)` `{ remote, prune }`; `Pull(PullParams)` `{ branch, upstream }`;
`Push(PushParams)` `{ branch, remote, remote_branch, set_upstream, lease }`, `Lease { expected: Oid }`.

**No `Merge`/`Rebase`/`Reset`/`CherryPick`/`Revert` variants exist.**

### `preview` (`operation.rs:527-535`)

Returns `Some(Vec<String>)` — the command lines a remote plan would run, computed
before facts are read — for `Fetch`/`Pull`/`Push` only. **`None` for every local kind.**
No preview exists for Stage/Unstage/Discard/Commit/Amend/stash.

### `apply` (`crates/graft-ops/src/apply.rs`)

`read_facts` → `check` → before-state + backup + `Pending` journal line → run the plan
→ after-state + outcome line. `ApplyError`: `Refused(Refusal)`, `Facts`, `Snapshot`,
`Journal{error, after}`, `AfterState{applied, unread}`, `Git{entry, error, recoverable}`,
`Stopped`, `Remote{entry, failure, stderr_tail, names}`.

### Journal shape and states

`JournalEntry` (`graft-core/src/journal.rs:199`): `id`, `at_unix_ms`, `kind`,
`description`, `status`, `touches`, `head_before`, `head_after`, `refs_before/refs_after`,
`index_before/after`, `worktree_before/after`, `backup_ref`, `pins: Vec<Oid>`,
`commands: Vec<String>`, `reverses: Option<EntryId>`.

`EntryStatus` (`journal.rs:151`): `Pending` (before-state recorded, may have started),
`Committed`, `Aborted { reason: String }` (still undoable if backed up). On disk,
`Line` is one JSONL record — `Pending(PendingLine)` or `Outcome(OutcomeLine)`, two lines
per entry, append-only.

### The stop token in `restore::execute`

`crates/graft-ops/src/restore.rs:116-119` — checked as the **first statement**, before
`begin`, before any journal write or step. `undo.rs` has no stop check of its own; both
`undo` and `redo` delegate here. **This is the only stop point in the undo path.**

### `facts` / `NotInvertible`

`read_facts(repo, git, stop, with_remotes) -> RepoFacts { bare, head, in_progress,
status, stashes, git, remotes, upstream }`, one pass, read before `check`/`plan`/`touches`.
`NotInvertible`: `Pending`, `NoBackup`, `NotUndoable(kind)`, `NothingToUndo`,
`NothingToRedo`, `Incomplete { field }`.

## 2. graft-graph layout

- **`layout(commits: &[Commit], seeds: Seeds) -> Vec<Row>`** (`layout.rs:144`) takes a
  whole in-memory slice, builds its own private id table, returns an owned `Vec<Row>`.
  Every merge commit's parents must be present in the slice (`MergeNames::gather`'s
  reader is `unreachable!()` here). **Confirmed usable on a bounded slice independently
  of the app's walk:** `crates/graft/src/view/strip.rs:113-124` lays out a synthetic
  7-commit slice that never reads the repository.
- **`StreamingLayout<S: CommitSource>`** (`layout/stream/mod.rs`) is the app's real
  chunked/resumable layout: `new`, `from_order`, `advance`, `rows(range)`, `set_viewport`,
  `truncate`.

The app draws rows via `layout.rows(range)` in `crates/graft/src/history/views.rs:298`;
`graft-ui`'s `graph_screen` consumes `&[Row]`/`GraphEntry` and applies a `Fold`.
Styling: terminal tiers in `graft-ui/src/graph_column/{mod,style.rs}` (glyph tables,
`off_thread`/`on_thread`, `dashes_odd_columns`); pixel tier in `graft-graph/src/raster.rs`
(`Ring`, `Emphasis`, `OFF_THREAD_ALPHA`/`FADED_ALPHA`) via `raster/cache.rs::StripCache`.

## 3. Plain statements

**Which ops write refs, and how backups are made.** Per `Operation::touches`
(`operation.rs:541-565`): `Commit`/`Amend` (HEAD's branch ref), stash push/apply/pop/drop
(`refs/stash`), `Pull` (the branch ref via `merge --ff-only`, never the tracking ref),
`Push` (records the upstream tracking ref for the browser only). `Fetch` touches no refs.
`undo`/`redo` write refs themselves through `Step::UpdateRef`, a compare-and-swap
`git update-ref <name> <to> <expect>` (`undo.rs:32-44`, run at `restore.rs:197`/`213`).
Backups: `snapshot::take` builds `{index/, worktree/}` trees, a `commit-tree` under a
fixed `graft` identity, and points `refs/graft/backup/<worktree>/<ulid>` at it
(`snapshot.rs:100`, `:166`). `apply` takes one whenever scope is `Full` or the op has pins.

**Is a conflicted or pending operation state modelled?** `EntryStatus::Pending` and
`Aborted { reason: String }` exist — `reason` is a plain string, not structured. **There
is no `Conflicted` variant anywhere.** The only related thing is a `Refusal::Unmerged
{ paths }`, refused *before* any write. No "pending operation the user must resolve"
state exists.

**Is there `Tip`/`Oid` pinning for staleness?** **No generic `Tip` type and no reusable
pin-and-refuse mechanism.** Two narrow instances: `Lease { expected: Oid }` on push
(refuses `LeaseStale` unless it equals the fetched `TrackedUpstream.tracking_tip`,
`operation/remote.rs:222`), and `PullParams.upstream` (refuses `UpstreamChanged` when
facts disagree). Both are single-purpose gates on specific fields.
