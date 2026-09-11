# Snapshot unconditionally before anything that rewrites the working tree

DESIGN.md §6.3's snapshot originally fired only for overtly destructive
operations (discard, forced checkout, `reset --hard`, stash drop, clean),
and §6.7's pre-undo guard compared ref OIDs only. That combination loses
work: undoing a merge or rebase restores the pre-op working tree, and an
uncommitted edit made after the operation finished moves no ref — the
guard stays silent and the restore overwrites the edit with no backup
(review finding C3), violating non-negotiable principle 2, "never lose
work". We now take the full combined snapshot — tracked and untracked,
§6.3's mechanism — before **any** operation that touches the working tree
or index, and again before any undo whose inverse rewrites the working
tree, unconditionally rather than only when drift is detected; and the
§6.7 guard compares refs, the index tree and a combined working-tree
hash, never refs alone.

## Consequences

Every merge, rebase, cherry-pick and revert now pays a status walk plus
hashing of changed and untracked files before it starts, and again on
undo. That cost is per user-triggered operation, not per frame, and buys
what the guard alone cannot: detection can miss, a snapshot can't — the
rail is the warning, the snapshot is the guarantee. Backup refs
accumulate faster; §6.8's explicit "clear backups older than…" action is
the pressure valve, still never automatic.
