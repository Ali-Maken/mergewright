# Graft: Design — Undo journal design

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Visualization](05-visualization.md) · Next: [Open design questions](07-open-design-questions.md)

---

## 6. Undo journal design

The undo journal is the mechanism behind `Ctrl+Z`/`u` and `Ctrl+Shift+Z`/`U`
(ADR-0002) and lives in the `ops` module (PROMPT.md §6), which is also the
only place allowed to run a write operation.

### 6.1 Storage and data model

- **Append-only log**, one JSON record per line, at the path
  `git rev-parse --git-path graft/undo-journal.jsonl` resolves to — never a
  literal `.git/` join, because at the root of a linked worktree `.git` is
  a *file* pointing at the real git dir, not a directory. Every other path
  Graft keeps under the git dir resolves the same way. `--git-path` puts
  the journal in the **per-worktree** git dir: `.git/graft/…` for the main
  worktree, `.git/worktrees/<name>/graft/…` for a linked one. The journal
  is deliberately per-worktree (ADR-0010): every entry's index and
  working-tree state belongs to exactly the worktree the operation ran in,
  so an entry can never be replayed against the wrong working tree — and
  an operation is undoable only from the worktree it ran in. Repo-local
  (travels with the repo, not with Graft's global config), never committed
  to the repo's own history, human-readable enough to inspect by hand if
  something goes wrong.
- **Backup content:** real git objects under
  `refs/graft/backup/<worktree>/<ulid>`, created before any operation
  that touches the working tree (§6.3). Refs, unlike the journal, are
  shared across every worktree of a repo, so the `<worktree>` segment —
  `main` for the main worktree, the directory name under
  `.git/worktrees/` for a linked one — keeps two worktrees' backups from
  colliding and says where each came from (ADR-0010). The `<ulid>`
  segment is the owning journal entry's `id` — unique by construction, so
  two snapshots taken in the same second can never write the same ref
  name (a timestamp there could, and the second write would silently
  orphan the first snapshot's objects) — and the name alone links a
  backup back to its journal entry. A backup ref keeps
  its objects reachable for ordinary `git gc`, so no extra pinning
  mechanism is needed while the ref exists.

```
JournalEntry {
    id: Ulid,                       // sortable, unique
    timestamp: DateTime,
    kind: OperationKind,            // Commit, Amend, Merge, Rebase, Reset,
                                     // BranchDelete, Discard, StashDrop, Push, …
    description: String,            // human text shown in the browser (§1.10)
    status: Pending | Committed | Aborted,
    refs_before: Map<RefName, Option<Oid>>,   // captured before the op runs
    refs_after: Map<RefName, Option<Oid>>,    // filled in once the op finishes
    index_before: Option<Oid>,       // pre-op index tree
    index_after: Option<Oid>,        // post-op index tree, recorded with the Committed record
    worktree_after: Option<Oid>,     // post-op combined tree (§6.3 hashing), for §6.7's comparison
    backup_ref: Option<RefName>,     // refs/graft/backup/<worktree>/<ulid>, set for any op
                                     //  that touched the working tree (§6.3)
    reverses: Option<Ulid>,          // set on an undo/redo entry: which entry it reverses
}
```

### 6.2 Recording protocol

1. **Before** running the operation: read the current OIDs of every ref it
   might touch, and — for any operation that touches the working tree or
   index at all, not only the obviously destructive ones — take the
   combined snapshot (§6.3), unconditionally. Append a `Pending` record.
2. **Run** the operation through `ops` (the same typed-argument git
   invocation that populates the Command Log).
3. **On success:** capture the resulting OIDs, the post-op index tree and
   the post-op combined working-tree hash (`index_after` /
   `worktree_after` — what §6.7 later compares against), and append a
   `Committed` record referencing the same `id`. The log is append-only —
   this is a second line, not an edit to the first — so a crash between steps 1 and 3 leaves a
   `Pending` record with no matching `Committed` one. On next launch, Graft
   treats an unmatched `Pending` entry the same way it treats a stray
   `rebase-merge/` directory (PROMPT.md §6): surfaced explicitly, offered for
   inspection, never silently dropped or silently assumed to have succeeded.
4. **On failure/abort:** append an `Aborted` record. Nothing needs undoing,
   but it stays in the log for the Command Log / troubleshooting view.

### 6.3 Working-tree snapshot (the untracked-file fix)

Per PROMPT.md §3, `git stash create` silently omits untracked files and must
not be used for this. Instead, before **any operation that touches the
working tree or index** — merge, rebase, cherry-pick, revert and stash
apply/pop as much as discard, forced checkout, `reset --hard`, stash drop
and clean — and before any undo whose inverse rewrites the working tree
(§6.4), unconditionally, not only when drift is detected (ADR-0009):

1. Start from the pre-op index tree.
2. For every untracked file in scope, `git hash-object -w` its contents into
   a blob and add it to that tree at its path (ignored files too, when the
   operation in question would also remove them).
3. Commit that combined tree under `refs/graft/backup/<worktree>/<ulid>`
   (the `<ulid>` being the owning journal entry's `id`, §6.1) — one
   object graph that represents tracked *and* untracked content as it stood
   immediately before the operation.

Undoing such an operation reads that tree back and restores working-tree
files from it (tracked content via the ordinary index/checkout path,
untracked content by writing the recovered blobs back to their recorded
paths). This is exactly what closes the gap PROMPT.md calls out, and it's
covered by the test PROMPT.md names explicitly: create an untracked file,
discard it, undo, assert the bytes come back exactly.

### 6.4 Per-kind inverse

Undo is not one generic "run the reverse command" — each kind has its own
specific inverse, computed from the journal entry:

| Kind | Inverse |
|---|---|
| Commit / Amend | Move ref(s) back to `refs_before`, restore the index from `index_before`. Working tree untouched (neither commit nor amend touches it) |
| Merge / Rebase / Cherry-pick / Revert | Move the branch ref back to `refs_before` using a compare-and-swap `update-ref` (fails loudly rather than clobbering if something else moved it since), restore index and working tree to match |
| Reset | Move ref back to `refs_before` (working tree was already covered by the pre-reset snapshot if it was `--hard`) |
| Branch delete | Recreate the ref at its recorded pre-delete OID |
| Discard | Restore files from the combined snapshot tree (§6.3) |
| Stash drop | Recreate the `refs/stash` reflog entry pointing at the (still-reachable, thanks to the backup ref) stash commit |
| Push | **Not silently undoable** — it changed shared state. Journaled for transparency and shown in the browser, but "undo" here opens a guided, freshly-confirmed re-push of the pre-push ref state rather than a one-key reversal, since it affects other people |

Undoing a push moves the remote branch backward — a non-fast-forward
update — so the re-push goes through the exact same rail as every force
push (PROMPT.md §4): `--force-with-lease --force-if-includes`, with
explicit confirmation, never a plain `--force`. Undo is not a separate,
less-guarded path into pushing.

**Undo snapshots before it restores.** Any inverse that rewrites the
working tree (the Merge / Rebase / Cherry-pick / Revert row, Reset,
Discard) starts by taking a fresh combined snapshot (§6.3) of the working
tree and index *as they stand at undo time* — unconditionally, not only
when §6.7's comparison notices drift. An edit made after the operation
finished and before `Ctrl+Z` never moved a ref, but it's still someone's
work; the fresh snapshot means the restore is always backed by a recovery
point, so "never lose work" holds even for changes no ref ever saw
(ADR-0009). Undo is itself journaled (§6.5), so this snapshot is simply
the undo entry's own `backup_ref`.

A mid-flight operation (a rebase currently stopped on a conflict) is not a
journal entry yet — its `Pending` record only resolves to `Committed` once
the operation finishes or is cleanly aborted through the operation banner
(§1.13). The banner's Abort is the mid-flight escape hatch; the undo journal
picks up once there's a finished operation to reverse.

### 6.5 Redo

Undo is itself journaled as an ordinary entry (`kind: Undo`, `reverses: Some(id)`),
computed and recorded the same way as any other operation, including its own
`refs_before`/`refs_after`. Redo is therefore not a separate data structure —
`Ctrl+Shift+Z`/`U` looks at the most recent entry; if it's an `Undo`, redoing
re-applies the entry it reversed from its recorded `refs_after`. This keeps
undo and redo symmetric: undo is always undoable too, the same way a text
editor's undo stack works.

Performing a new operation after an undo doesn't erase anything from the
log (it's append-only) — it just means the next `Ctrl+Shift+Z` has nothing to
redo, because the active cursor has moved past that branch of history. The
undo journal browser (§1.10) can still show the abandoned entries; they're
just not reachable through plain redo any more, matching how the browser is
described as fully browsable even for entries off the current undo/redo path.

### 6.6 Browsing and Ghost Preview

Selecting an entry in the browser (§1.10) and confirming reverts state to
just *before* that entry, by applying each entry's specific inverse from the
most recent one back to the target, in order. Before confirming, Graft draws
the predicted resulting ref state as a Ghost Preview (§1.7) — undo gets the
same "see it before you do it" treatment as every other history-changing
operation, per PROMPT.md §1.5.

### 6.7 Safety rails

Before applying an undo, Graft compares the state the journal entry says
the operation left behind against what's actually there now — three
comparisons, not one:

- **Refs:** every ref in `refs_after` against its current OID.
- **Index:** the entry's `index_after` tree against the current index
  tree.
- **Working tree:** the entry's `worktree_after` hash against a fresh
  combined-tree hash (§6.3's mechanism, untracked files included) of the
  working tree as it is now.

If any of the three differ — a bare `git` command run outside Graft, or
just an edited file — undo doesn't silently overwrite the unexpected
state: it stops, explains exactly which of the three changed, and asks
before proceeding. A plain working-tree edit never moves a ref, which is
why comparing refs alone was never enough. And because §6.4's inverse
snapshots unconditionally before touching the working tree, even a drift
this comparison could somehow miss is still recoverable afterwards: the
rail is the warning, the snapshot is the guarantee (ADR-0009).

### 6.8 Retention

Backup refs and journal entries are **kept indefinitely by default.** Git
objects are content-addressed and deduplicated, and a ref list entry is
cheap, so "never lose work" wins over tidiness by default. A "Clear backups
older than…" maintenance action is reachable from the command palette, always
explicit and always confirmed — never run automatically, and never as a side
effect of any other operation.

---
