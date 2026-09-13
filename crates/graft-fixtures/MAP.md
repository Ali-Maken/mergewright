# graft-fixtures MAP

Public items, one line each.

- `Shape` (enum) — one hand-written corpus repository per history shape; `Shape::ALL`, `name()`. `SkewedDates` has a commit a day ahead of its children, so date order is not topological. `RelativeDates` is timed off `CORPUS_NOW`, the corpus's fixed clock, not the wall clock (m1-the-graph/53) — like every other shape, its object ids are byte-identical run to run, and `corpus::identity` (tests) no longer exempts it.
- `CORPUS_NOW` (const) — the instant every `RelativeDates` commit is dated against (m1-the-graph/53); past the corpus's latest fixed-clock commit, so ages never go negative.
- `borrow` (fn) — the directory of one corpus fixture, read-only; panics if unavailable.
- `try_borrow` (fn) — `borrow` that returns why a shape is unavailable.
- `copy_into_temp` (fn) — a writable copy of one fixture, deleted on drop; panics if unavailable.
- `try_copy_into_temp` (fn) — `copy_into_temp` that returns the error.
- `ensure` (fn) — build the corpus cache if missing, reporting what was built.
- `build_into` (fn) — build the corpus into a given directory.
- `Report` (struct) — what `ensure`/`build_into` did.
- `CorpusError` (enum) — why the corpus could not be built.
- `Unavailable` (enum) — why one shape cannot be had on this machine.
- `CacheError` (enum) — why the fixture cache could not be used.
- `Scratch` (struct) — a temp copy deleted on drop; `path()`.
- `measure` (fn) — run work and report its peak memory rise.
- `peak_rss` (fn) — this process's peak resident set size.
- `Measured` (struct) — a measurement; `rise()`.
- `MemoryError` (enum) — why memory could not be read.
- `proptest_cases` (fn) — property-test case count for the tier in force.
- `git` (mod) — running system git for fixture building; `GitError`.
- `synthetic` (mod, `heavy-tests`) — large generated histories: `Scale` (`of`, `HEAVY`, `with_commit_graph`, `with_realistic_commits`, `with_refs` (50k-ref variant: packed branches + upstreams + `[branch]` sections), `commits`, `name`), `CommitGraph`, `CommitSize` (`Small` default, `Realistic` sized after git/git), `ensure`, `borrow`.
