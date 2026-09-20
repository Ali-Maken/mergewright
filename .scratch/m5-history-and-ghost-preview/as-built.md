# m5/07 as built — where the code differs from the design note

Ticket: `.scratch/m5-history-and-ghost-preview/issues/07-ops-history-operations.md`.
Note: `docs/design-notes/m5-history-and-ghost-preview.md`, the G5 block.

Everything the note named exists. This file lists only what **differs**, and
why, so G6/G8–G10 and M6/M7 build against the real thing.

## 1. The journal shape of a conflicted operation (hard choice 2)

Built as the note decided: **one entry, `Pending` from the stop until it
finishes or aborts.** What the note did not spell out, and the code now
fixes:

```rust
pub enum ApplyError {
    …,
    Conflicted   { entry: Box<JournalEntry>, kind: InProgress, paths: Vec<PathBuf> },
    ResumeFailed { entry: Box<JournalEntry>, kind: InProgress, error: RunError },
}
pub enum Resume { Continue, Skip, Abort }
pub fn resume(ctx: &mut Context<'_>, how: Resume) -> Result<Applied, ApplyError>;
pub struct Recovered { pub live: Vec<EntryId>, pub closed: …, pub unreadable: … }
impl Journal { pub fn hold_commands(&mut self, id: &EntryId, commands: Vec<String>); }
```

- **`ResumeFailed` is new.** `--continue` can fail without conflicting (a
  hook rejects the commit; the tree is gone). That leaves the entry
  `Pending` **and** git's state live — neither `Conflicted` (there is
  nothing to resolve) nor `Git { .. }` (which means the entry was closed).
  Swallowing it into either would lie about what the repository holds.
- **`Recovered.live`** is the field behind the note's "SKIPS a Pending entry
  whose kind matches a live `in_progress`". The caller needs the ids: those
  entries are what the banner takes over at launch.
- **`hold_commands`.** Only the *outcome* line records `commands`, and a
  stopped operation has not written one. The lines are kept on the in-memory
  entry so the Command Log shows them while the conflict is open. A relaunch
  loses them — recovery closes the entry with none, and that is honest
  rather than reconstructed.
- `ctx` is `&mut Context` (the note wrote `&Context`); the journal is
  written through it.

**For M6.** The resolver works entirely inside one `Pending` entry: mark a
file resolved with an ordinary `Stage` (which `check` exempts from
`Unmerged`, and refuses as `MarkersRemain` while `<<<<<<< ` is still in the
file), then `resume(ctx, Resume::Continue)`. There is no second entry to
reconcile, and `u` after the finish takes the whole span back — verified by
`tests/it/history_ops.rs::conflict_span_is_one_entry_and_undoes_to_before_the_merge`.

## 2. `PreviewPlan` (the structural piece)

As the note wrote it, with three differences:

```rust
pub struct RefMove { pub name: String, pub from: Option<Oid>, pub to: Option<Target> }
pub fn plan_revert_to(entries: &[JournalEntry], target: &EntryId) -> Result<PreviewPlan, PreviewError>;
pub enum PreviewError { Facts(FactsError), Read(RunError), Refused(Refusal), Stopped }
```

- **`RefMove.to` is `Option<Target>`.** A branch or tag delete is a ref move
  with no destination. `Target` has no "gone" variant because a *parent* can
  never be gone.
- **`plan_revert_to` takes no `now: &RepoStateNow`.** The picture is drawn
  from the very `InversePlan`s `undo::revert_to` will run, so the two cannot
  disagree; drift against the repository as it stands is checked per step by
  `revert_to` itself (§6.7), which is where the user's `force` answer lives.
  Reading a `RepoStateNow` here would be a second, older opinion.
- **`PreviewError::Refused(Refusal)` exists** because some preconditions can
  only be found by a read: a re-push whose commit is not on its branch
  (`merge-base --is-ancestor`), an `--ff-only` merge of diverged branches.
  `check` is pure over `RepoFacts` and cannot spawn, so these refusals are
  the preview's, not `check`'s. A caller that confirms still goes through
  `check`.

Two rules the code holds and the renderer can rely on:

- **`plan.commands` is `op.plan(&facts)` rendered, always.** `preview::plan`
  overwrites whatever a per-kind planner left there, from the same `facts`
  the picture was drawn from. There is no second rendering to drift
  (`preview_commands_come_from_plan` walks eleven kinds).
- **A hole beats both a clean and a likely forecast.** `fold` returns
  `NotChecked` if either side is `NotChecked`. Absence of evidence is not a
  clean bill, and the wording is `may conflict` — `merge-tree` forecasts one
  merge, and a rebase replays commit by commit, so a later one can still
  stop.

**For M7.** A chain rebase draws as `FORECAST_CAP`-bounded ghosts per
branch; `Target::Ghost(usize)` indexes `ghosts` within **one** plan, so a
multi-branch picture needs either one plan with all ghosts or a wrapper that
re-bases the indices — decide before building, do not overload `usize`.

## 3. Params that grew a field

| Note | As built | Why |
|---|---|---|
| `PickParams { commits, head }` | `+ subject: String` | `describe` is pure — it cannot spawn to read the picked commit's subject. |
| `RevertParams { commit, mainline, head }` | `+ subject: String, parents: usize, parent: Option<Oid>` | `subject` as above; `parents` is what `check` validates `mainline` against; `parent` is the commit's first parent, which the forecast diffs against — a revert's commit is already on `HEAD`, so `commits_between` cannot reach it the way a cherry-pick's can. |
| `PushParams { … }` | `+ source_commit: Option<Oid>` | As the note said. `check` refuses `RepushNeedsLease` when it is set without a lease; `preview::plan` refuses `NotOnBranch` unless it is an ancestor of the branch; the refspec's source is the commit. |

## 4. `touches` for a checkout: **no** ref, not `[HEAD]`

The note's touches line says `checkout {[HEAD], index, worktree}`. Built as
`{[], index, worktree}`, and the same rule now applies to the undo
executor's own entries (`undo::touches_of` contributes no ref for a
`SetHead` step).

A touched ref is snapshotted into `refs_before`/`refs_after`, and inverting
that snapshot emits `update-ref HEAD <old> <new>` — which moves the
**branch `HEAD` points at**, not `HEAD`. Redoing a checkout therefore moved
`main` onto `feature`'s commit. `HEAD`'s position is recorded by the entry's
`head_before`/`head_after` and put back by `SetHead`, which is
symbolic-aware. Found by `history_ops.rs::checkout_applies_undoes_redoes`.

## 5. Other signatures worth knowing

- `undo::plan_revert_to(entries, target: &EntryId) -> Result<Vec<InversePlan>, NotInvertible>`
  and `undo::revert_to(ctx: &mut Context, target: &EntryId, force: bool)` —
  by reference, and `&mut`.
- `UndoError::PartWay { done: Vec<Applied>, error: Box<UndoError> }` — a
  revert that stopped part way. What ran stays; it is journaled and
  undoable. Rolling it back would be a second write nobody asked for.
- `Context::advance(&mut self)` — a `Context` names one entry id. The time
  machine journals one per step, and two steps share a millisecond, so it
  increments the random half as an 80-bit counter (ULID's own rule). Without
  it the second step was refused as a duplicate.
- `Operation::tips() -> Vec<Tip>`, `is_history()`, `starts_in_progress()`,
  and `FactsScope { remotes, refs }` — the scope is what `check` **and**
  `preview::plan` need; a branch-from-stash names `HEAD`, a push names its
  branch.
- `graft_core::oid::Oid::from_sha1([u8; 20])` — infallible, for the ghost
  ids. `from_bytes` is for lengths known only at run time.
- `preview::ghost_oid(description, n)` — `sha1("graft-ghost\0<description>\0<n>")`.
  Stable for one plan, distinct per row, **never written to the object
  database**.

## 6. Named in the note, not built here

- **`OperationKind::RevertTo` is unused, and stays unused.** The time
  machine journals one `Undo` entry per step, which the note also asked for,
  so no entry ever carries this kind — a `RevertTo` entry would be the
  summary §6.6 rejected. The reasoning now lives on the variant itself in
  `graft-core`. Ticket 08's `PreviewSubject::RevertTo` is a different type
  with the same name and is not a user of it. **Delete or claim: the
  conductor's call.**
- `Refusal::NotInProgress`, `BadRefName`, `RefExists` exist and `explain`
  covers them; nothing in `graft-ops` returns the first one yet — `resume`
  is reached only from a live conflict, and the UI is where a stale banner
  key would be refused.

## 7. The seam this ticket left open — **closed**

`graft` (the app crate) does not compile against this. Five sites, all
mechanical:

- `crates/graft/src/jobs/operate.rs:117` — the match over `&ApplyError`
  needs `Conflicted { .. }` and `ResumeFailed { .. }` (both → a full
  refresh: git's state changed).
- `crates/graft/src/jobs/undo.rs:95` — the match over `&UndoError` needs
  `PartWay { .. }`.
- `crates/graft/src/update/remote.rs:299` and `:317`, and
  `crates/graft/src/view/tests.rs:2998` — `PushParams` literals need
  `source_commit: None`.

**Closed in the m5/07 follow-up session.** `main` was merged in first (clean
merge, four seam errors on the first compile and nothing else). The two new
`ApplyError` variants do **not** become `Operated::Failed`: `Operated` grew
`Conflicted { description, kind, paths, stalled }`, which both land in,
worded as a state and refreshing `Everything` — §1's "one entry, `Pending`
from the stop" reaches the user as "you are in this merge", not as an error
line. `UndoError::PartWay` is handled in `jobs/undo.rs`'s
`refresh_after_failure` and `undone_from`. All three `PushParams` literals
take `source_commit: None`, since the guided re-push (ADR-0038 §4) is not
built. Details and the tests that hold it: the ticket's second NOTES section.
