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
file), then `resume(ctx, Resume::Continue)`.

This paragraph said "there is no second entry to reconcile" before the step
that would have written one was built. It was false as shipped and the fix
round (m5/07 review, C1) made it true. What now holds, exactly:

- **A step taken while `in_progress` is `Some` writes no journal line.**
  `apply` dispatches every such operation to
  `apply::resume::inside_live_operation`, which runs the plan, folds its
  command line onto the live entry via `hold_commands`, and returns an
  `Applied` whose `entry` is **not in the journal**. The rule is "git has an
  operation in progress", not "the index cannot be snapshotted" — the last
  file a user resolves leaves a clean index, and that step must fold like
  every one before it. Verified by
  `tests/it/history_ops.rs::resolving_each_file_in_turn_still_leaves_the_span_one_entry`
  (two conflicted files, the second leaving the index clean; one journal
  line, one entry).
- **Read `description` and `refresh` off that `Applied`; never look its `id`
  up.** It is a record of what ran, for the status line. `Journal::entries`
  will not have it.
- **`u` after the finish takes the whole span back**, including the resolve
  steps, because they are the same entry — verified by
  `tests/it/history_ops.rs::conflict_span_is_one_entry_and_undoes_to_before_the_merge`,
  which stages the resolution through `Operation::Stage` (as the resolver
  will) and then undoes to before the merge.
- **A span begun outside Graft has no entry to fold onto**, and the step
  still runs: `fold_commands` records nothing and says why. Graft never
  adopts a shell-started merge as its own entry (`live_entry` compares the
  entry's `head_before` against git's `rebase-merge/head-name` or the
  current symbolic `HEAD`, and requires the branch to still be at the
  recorded tip) — verified by
  `tests/it/history_ops.rs::a_shell_merge_is_not_adopted_as_grafts_own_entry`.
- **The gap M6 inherits (not fixed here).** `commands` held on a `Pending`
  entry live in memory only. The app opens a fresh `Journal` per job, so the
  Command Log shows the span's lines only within one job — the fold is
  correct, its visibility is not. `tests/it/history_ops.rs::a_resolve_step_joins_the_live_entrys_commands`
  proves the fold by running the merge and the stage through one `Journal`,
  and its doc records the limitation. Fixing it means persisting a
  `Pending` entry's commands, which is app-side (m5/07 review, M9).

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
| `PushParams { … }` | `+ source_commit: Option<Oid>` | As the note said. `check` refuses `RepushNeedsLease` when it is set without a lease; the refspec's source is the commit. The `NotOnBranch` ancestry guard is refused **twice** — `preview::plan` draws it, and `apply::guard_repush` asks `merge-base --is-ancestor` again before the first write, because a rule only the picture enforces is not enforced (m5/07 review, M5). Both share `preview::is_ancestor`, and a read that fails is `FactsError::Ancestry`, never a guess. |

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

## 8. The plans pin the reader's config out (fix round)

Added by the m5/07 fix round (review M1, M2). Not in the note, and it
belongs here because every later milestone's plan lines inherit it: **a plan
says what it means, so a config the reader happens to have set cannot make
the preview a lie.**

- `merge` carries `--ff` for `FastForward::Default`, not the absence of a
  flag. `merge.ff=false` turns a plain `merge` into a merge commit, and the
  picture had drawn a fast-forward. Verified against real git 2.54: with
  `merge.ff=false` set, `merge --ff` fast-forwards (one parent), no flag
  makes a merge commit (two). Test:
  `tests/it/history_ops.rs::a_merge_follows_the_picture_not_the_readers_merge_ff`.
- `rebase` carries `--no-update-refs --no-autostash`, gated on
  `GitVersion::MINIMUM` (2.38, which is where `--update-refs` arrived).
  `rebase.updateRefs=true` moves every branch pointing into the replayed
  range, and the entry records one. Verified against real git: with the
  config set, an unflagged rebase moves `f1x`; `--no-update-refs` leaves it.
  Test: `tests/it/history_ops.rs::a_rebase_moves_only_the_branch_the_entry_records`.
- **Pull was checked and is honest under `pull.rebase=true`** (the user has
  it set globally). `plan_pull` is `fetch` + `merge --ff-only <tracking>` —
  it never runs `git pull`, so `pull.rebase` cannot reach it, and `--ff-only`
  overrides `merge.ff=false`. Observed directly, not reasoned: with both
  configs set, the `merge --ff-only` fast-forwarded (exit 0, single parent).
- **`WorktreeEffect::NotShown { why }`** is new (review M3). A revert with
  no recorded parent used to draw `Untouched` — "nothing happens to your
  files", which is the one thing it does not mean. It now names the hole, as
  `ConflictForecast::NotChecked` does. Test:
  `tests/it/history_ops.rs::a_revert_with_no_recorded_parent_says_so_rather_than_drawing_nothing`.

## graft (app), group 08 continuation — pieces 7–10

Deviations 1–8 from the first half are in the ticket's NOTES. These are new,
from the second half (pieces 7, 8, 9, 10 and group 06's scope item 5).

9. **`Running.opened_by: Option<&'static str>`** and a matching parameter on
   `update::remote::launched` and `update::operate::launch_unstoppable`. The
   ticket asks a `Moved` refusal to say `m again` / `r again` / `. again`,
   but `PreviewSubject` — the only thing that knows which key drew the
   picture — is consumed at confirm time. The key now rides along on the
   write in flight, which is the one record that outlives the preview and
   dies with the operation.
10. **`Operated::Failed` gains `refused: Option<graft_ops::operation::Refusal>`**,
    mirroring its existing `remote: Option<RemoteFailure>`. The refusal is
    carried as a value rather than as `error.to_string()` because
    `Status::Sticky` needs the reason and the **keys** apart, and a key hint
    cannot be recovered from a sentence.
11. **`Undone::Cancelled { clean, why, refresh }`** is new (group 06 scope
    item 5). `RestoreError::{Cancelled, CancelledPartway}` used to fall into
    `Undone::Failed` and read as a failure; a cancel the user asked for is
    not one. `clean` chooses `Status::Done` over `Status::Sticky`, and the
    words are `restore`'s own (`undo cancelled · nothing changed`), not the
    ticket's shorter `undo cancelled` — same meaning, one more fact.
12. **`RemoteState.diverged_upstream` is *not* cleared by a refresh**, though
    the ticket lists refresh among its three clears. It cannot be: a pull
    that fails non-fast-forward asks for `Refresh::Everything` itself, so
    clearing on refresh would wipe the field microseconds after it was set
    and the two keys it offers would never fire. It is cleared by dismissing
    the line (`Esc`) and by any write launching. Safety does not rest on the
    clear: the `Operation` is pinned to the recorded tip, and `check`
    refuses `Moved` if the upstream moved in between — which is hard choice
    3 doing exactly its job, and now has words (deviation 10).
13. **`Operated::Conflicted` still carries `stalled: Option<String>`, not
    `entry: EntryId`.** The Signatures block asks for `entry`; nothing in
    this group's scope reads it (the banner is `app-conflict-interim`'s), so
    adding it would have been an unused field. The conflicted line is
    unchanged and already honest.
14. **The verb in the `DirtyTree` line is the first word of
    `Operation::describe`** (`rebase feature onto main` → `rebase`). Every
    `describe_*` in `graft_ops::operation::history` begins with its verb.
    Written this way rather than as a table in `app` so the refusal and the
    Command Log cannot describe one operation two ways (KNOWN_GAPS row 229).
15. **`m4_walk::fetch_pull::pull_diverged_refused`'s expectation changed.**
    It asserted M4's line (`pull stopped: … merge or rebase (M5); shell: git
    pull --rebase`). M5 has both verbs inside Graft, so the app now words it
    `<branch> and <upstream> have diverged · m merge it  r rebase onto it`.
    `graft_ops::apply::remote::explain`'s own string still says "(M5); shell:
    …" and is now only reached when the tracking tip is unknown — a
    `graft-ops` tidy-up, outside this builder's fence.
