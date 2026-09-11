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
