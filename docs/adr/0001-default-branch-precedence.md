# Resolving a repo's default branch by ordered precedence

The graph reserves lane 0 for the **default branch** (CONTEXT.md), so we must
decide which branch that is before we can draw anything, and hard-coding `main`
is wrong often enough to matter: the two repos on this machine disagree with
each other (`main` and `master`), and plenty of real repos use neither. We
resolve it in order — `origin/HEAD` if it resolves, then `init.defaultBranch`
from git config, then the first existing of `main`/`master`, then the branch
that is the target of the most first-parent merges — and make it overridable
per repo in config.

## Consequences

The last rule is a heuristic over history, so it can be slow on a large repo and
must be computed off-thread and cached. Branches like `develop` and `release/*`
are **integration branches**: they get a lane adjacent to lane 0 and never
compete for it, which stops the graph from flipping its spine when a release
branch is busier than the default branch.
