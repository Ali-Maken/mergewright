# The undo journal is per-worktree; backup refs are shared but namespaced

At the root of a linked worktree, `.git` is a file pointing at the real
git dir, so the journal's original literal path
(`.git/graft/undo-journal.jsonl`) breaks outright — and beyond the path
bug, a single shared journal would let an entry recorded in one worktree
replay its index and working-tree state onto another (review finding C4).
We decide: every path Graft keeps under the git dir resolves via
`git rev-parse --git-path graft/…` (per-worktree by git's own layout
rules — `.git/graft/…` in the main worktree,
`.git/worktrees/<name>/graft/…` in a linked one); the journal is
**per-worktree**, so an entry's working-tree state can only ever be
replayed where it was recorded; and backup refs, which git shares across
all worktrees, are namespaced as
`refs/graft/backup/<worktree>/<timestamp>` (`main` for the main worktree,
the directory name under `.git/worktrees/` otherwise).

## Consequences

An operation is undoable only from the worktree it ran in: a branch
delete performed in worktree A never appears in worktree B's journal,
even though B sees the ref vanish. That asymmetry is the price of never
applying one worktree's snapshot to another's files. M3's journal tickets
need a linked-worktree fixture test: record an operation in a linked
worktree, assert the journal lands under `.git/worktrees/<name>/graft/`,
assert the other worktree's journal doesn't see the entry, and assert the
backup ref carries the right `<worktree>` segment.
