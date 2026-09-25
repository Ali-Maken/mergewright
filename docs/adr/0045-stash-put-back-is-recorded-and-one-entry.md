# ADR-0045 — A stash put back is recorded by Graft; setting edits aside around a rebase or switch is one entry

Date: 2026-09-25
Status: accepted. Direction by the user: picks 15 and 16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24-25,
field notes :67, "all three took the recommended option"). Mechanism by the design pass:
`docs/design-notes/m6-stash-put-back.md`. Study: `docs/research/commit-stash.md` addendum A1–A4.
Supersedes: `docs/design-notes/m5-history-and-ghost-preview.md` hard choice 4 ("rebase on a dirty tree: refused, never
`--autostash`") for **rebase and checkout**. Other kinds keep their refusals.

## Context
- A pop that clashes keeps the stash, leaves markers and **no operation state** (A1.1, A1.3). Nothing in git says "a
  stash is half put back", so neither the resolver's close nor a relaunch can name the stash or finish the job.
- M5 refused a dirty rebase because git's `--autostash` put-back "can conflict outside any journal entry" (hard choice
  4). His pick 16 asks for the put-back anyway. The reason stands; the answer is to put it **inside** the entry.
- git exits 0 when an autostash put-back clashes (A1.5). A tool that trusts the exit code hides the clash (VS Code
  #262360, Sublime Merge #1199).

## Decision

### 1. Graft records every put-back it starts, on the journal's Pending line
`PutBack { stash: Oid, why: Pop | Apply | SetAside }`, on `PendingLine` and `JournalEntry` as
`#[serde(default, skip_serializing_if = "Option::is_none")]` — the same additive rule as `merge_before`: every line
written before reads as `None` and re-renders byte-identically. For a pop or apply, `stash` is the entry's commit. For
a set-aside it is the commit `git stash create` printed **before** the Pending line was written (§4).

### 2. A clash is live by one pure rule over the journal (`graft_ops::live_clash`)
- No clash while git has an operation in progress, or while nothing is unmerged.
- Walk the in-force entries newest first. Skip an entry whose `index_before` is unmerged or not recorded (a step taken
  inside the clash: stage, a whole-file choice, start over; or an entry that never touched the index), and an Undo or
  Redo whose `index_after` is unmerged (walking back into the clash). The first entry left must have `index_before`
  merged, `index_after` unmerged and `put_back: Some` — that is the clash. Anything else: no clash.
- The stash must still be in the list, found **by commit id**; its position is looked up, never assumed.
- A pop made in the shell has no record: the resolver works as today and names no stash. Residual, words only: a shell
  pop that clashes straight after a Graft **apply**-clash was finished, with no Graft write between, is named as that
  apply's stash. Apply never drops, so nothing is lost.

### 3. Finishing drops the stash by commit id, without a box
When the last clashed file is marked done, a **pop** or **set-aside** clash ends in
`Operation::DropPutBack { stash }`: `stash drop stash@{n}`, `n` looked up in the facts `apply` reads, refused
`NoSuchStash` when it is gone, journal kind `StashDrop` with `pins: [stash]`, so `u` puts it back (the existing
`StashStore` inverse). An **apply** clash never drops. Undo is the guard, not a confirm (GitUp; lazygit #4491).

### 4. Setting aside is one entry, in this order
For `Rebase` and `Checkout` when tracked files are changed (`Operation::sets_aside`); untracked files are left where
they are and covered by the entry's backup, as every write is (ADR-0009).
1. `git stash create "<message>"` → `S`. Writes objects only: no ref, index or file changes.
2. The before-snapshot and the Pending line, with `put_back = SetAside(S)`, `pins ∋ S`, `touches ∋ refs/stash`.
3. `git stash store -m "graft: your edits, set aside for <description>" S` — from here `S` is in the stash list.
4. `git reset --hard` (tracked files only; untracked untouched).
5. The operation's own plan.
6. Put back: `git stash apply <S>` (no `--index`), then **read the index whatever the exit code**. Nothing unmerged →
   `stash drop` of `S` by id, entry `Committed`. Unmerged → `S` kept, entry `Committed` with an unmerged
   `index_after`, outcome `ApplyError::Clashed` (§2 finds it). Another failure → `S` kept, words name it.
- The operation **stops on a conflict** at 5: the entry stays `Pending` with `put_back` on it; `S` waits in the list.
  `resume` runs step 6 after `--continue` finishes and after `--abort` (onto the original tip, where `S` was made).
- The operation **fails** at 5 (git refused, e.g. an untracked file in the way): step 6 onto the unchanged tip, then
  the failure's own words.

### 5. The edits survive every failure
| Stops at | Working tree | `S` | What he is told |
|---|---|---|---|
| 1 or 2 | untouched | not stored | the error; nothing changed |
| 3 | untouched | objects only, pinned by the backup | the error; nothing changed |
| 4 or later | reset or part-way | in the stash list | the error names `stash@{n}`; `u` restores the backup |
| a kill anywhere after 3 | as git left it | in the stash list | relaunch: the interrupted line names `stash@{n}` |
| 6, clash | markers | in the list | the resolver (§2, §3) |
| 6, other failure | the operation's result | in the list | `your edits are in stash@{n} · S p puts them back` |

### 6. Undo and redo
- **Undo** of a set-aside entry: the generic inverse (branch back by compare-and-swap, `HEAD`, index and working tree
  from the backup — the edits come back exactly, staged split included), plus `StashDrop(S)` **only when the entry's
  `refs/stash` moved to `S`** (a clash left it there): the same rule as a push's undo. A clean put-back already
  dropped `S`, so undo stores nothing. `S` stays pinned by the undo entry's backup.
- **Redo**: the after-state, plus `StashStore(S)` only when the undo's own `refs/stash` moved (the existing pop/drop
  rule, m6-conflicts/07 F2).
- **Undo during a clash** (`esc u`) is the "abort" A4 asked for: pop or apply → the tree before the pop, stash still
  listed (git kept it); set-aside → the tree and branch before the rebase or switch, edits back, `S` dropped as a
  duplicate of what came back. With a take made since, the drift box asks first, as for any undo.
- A set-aside entry that is still `Pending` (a rebase stopped) is not undoable, as today; `a` is its way back.

### 7. What must be proven before it ships
- Round trip on a table of trees (staged + unstaged + untracked, a staged new file, a deletion, a rename): apply →
  undo leaves the index tree, the working-tree tree and the stash list equal to before; redo equal to after.
- A stop injected before each step 1–6 (the existing `Stop`) leaves the edits in the tree or in the stash list, never
  in neither — asserted by reading both.
- The put-back's outcome comes from reading the index, never the exit code: a failed `stash apply` with nothing
  unmerged (an untracked file in the way) is "kept", one with unmerged paths is a clash. `stash apply` itself exits 1
  on a clash, so git's exit-0 case (A1.5, `--autostash`) is excluded by not using `--autostash`, not by a test.
- `live_clash`'s rule as a table test, including the undo-back-into-a-clash row and the shell-pop row.

## Consequences
- `check` no longer refuses `DirtyTree` for rebase or checkout. Ticket 67's gate keeps its job for every other refusal;
  its dirty-tree tests flip in the set-aside group.
- After a clash is resolved, every file is **staged**, as git leaves it (A1.2). Not unstaged by Graft: the files he
  had staged before are indistinguishable. Named in KNOWN_GAPS.
- A put-back loses the staged/unstaged split on the clean path too (no `--index`); the preview says so before `y`.
- A fast-forward pull over overlapping edits still fails with git's words. Out of pick 16's scope; a KNOWN_GAPS row.
