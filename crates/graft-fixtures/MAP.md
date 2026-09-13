# graft-fixtures MAP

Public items, one line each.

- `Shape` (enum) — one hand-written corpus repository per history shape; `Shape::ALL`, `name()`. `SkewedDates` has a commit a day ahead of its children, so date order is not topological.
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
- `synthetic` (mod, `heavy-tests`) — large generated histories: `Scale`, `CommitGraph`, `ensure`, `borrow`.
