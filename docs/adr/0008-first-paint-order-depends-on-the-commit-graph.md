# First paint order depends on the commit-graph file

DESIGN.md §3.2 originally specified one walk — the in-degree topological
sort git uses for `--topo-order` — which cannot emit anything until it has
walked the whole visible range, so the 150 ms first-paint budget was
asserted, not achievable (review finding C1). We resolve it with the
repo's commit-graph file:

- **Commit-graph present:** stream strict topo order incrementally using
  generation numbers — git's own incremental topo-sort. A child's
  generation always exceeds its parent's, so a commit's in-degree is final
  once the exploration frontier drops to its generation; emission starts
  after a small frontier, not a full walk. gix exposes the file via
  `gix-commitgraph`, and any commit-graph written by git ≥2.38 (Graft's
  minimum) has generation numbers.
- **Commit-graph absent:** first paint is **commit-date order** (the
  rev-walk's natural streaming order); a full topo walk runs as a
  background job, and the view promotes to strict topo order in one step
  when it finishes, re-anchoring selection and scroll by commit id. Graft
  also offers — once per repo, declining remembered — to run
  `git commit-graph write --reachable` in the background so later opens
  take the fast path.

The 150 ms budget stands, with its meaning made precise: without a
commit-graph it buys a date-order first paint, and strict topo order
arrives at promotion, not at first paint.

## Consequences

`git gc` and `fetch.writeCommitGraph` already write commit-graphs, so the
fast path is the common case and the one-time offer converts most of the
rest. The heavy tier must exercise both paths against the 100k fixture
(commit-graph deleted for the slow one), and promotion needs its own
test: selection and scroll land on the same commits before and after.

## Landed (m1-the-graph/54)

- **System git builds it:** `git --git-dir <common_dir> commit-graph write --reachable --no-progress` (`graft_git::commit_graph::write`); gix has no writer.
- **As a job,** offered on a footer line: `B` builds, `X` declines. Never automatic.
- **Never cancelled or killed** once git starts: git writes through a `.lock` and renames; `SIGKILL` would strand the lock. On quit, the child finishes on its own.
- **The decline lives in the common dir,** `<common_dir>/graft/commit-graph-offer`, not `--git-path`: the commit-graph belongs to the shared object store, so one decline covers every worktree. The one deviation from DESIGN.md §6.1's rule.
- **Outside the undo journal, by name:** a commit-graph is a cache, not history; nothing to undo. The running session keeps its walk; the next open takes the fast path.

## Amendment (m1-the-graph/67, user 2026-09-17): the no-commit-graph date walk emits a whole tie class at once

The commit-date first paint (line 16 above) is not commit-by-commit when dates tie: `history::simple::Dated::next` must take the whole class of commits sharing the frontier's newest date, reading down through same-dated parents, before it can emit any of them children-first (fixed by m1-the-graph/67, which also closed a parent-above-child bug for tied dates). On distinct dates the class is one commit and the walk streams as described above; nothing changes there.

**Cost:** a repo where every commit shares one date (fast-import with a fixed timestamp, some mirrors) makes the whole visible range one class, so first paint becomes the full walk instead of a small frontier — measured ≈5.3 s on the 1M no-commit-graph fixture (`docs/reviews/m1-the-graph-round-2.md` "m2", `target/m1-67/after.txt`). **Accepted, not capped** (user, 2026-09-17): a cap would emit a provisional order for an unseen tie class, which hides the cost behind a wrong picture rather than showing it. The 150 ms budget's meaning (line 24) now further depends on tie density, not only on graph presence.
