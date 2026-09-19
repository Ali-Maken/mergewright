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
- `footprint` (fn) — this process's physical footprint, current and lifetime peak (`ri_phys_footprint`/`ri_lifetime_max_phys_footprint`); macOS only, `Err(MemoryError::Unsupported)` elsewhere (m1-the-graph/57).
- `Footprint` (struct) — `current`, `peak`, in bytes.
- `MemoryError` (enum) — why memory could not be read.
- `proptest_cases` (fn) — property-test case count for the tier in force.
- `git` (mod) — running system git with the machine's config switched off; `GitError`. `run` / `run_with_stdin_file` (fixed identity and clock too, for reproducible objects), `run_isolated` (config off, clock and identity the caller's — for a test whose commits must be dated now), `isolated` (the unspawned `Command`, for a caller that needs its own pipes), `IDENTITY_NAME` / `IDENTITY_EMAIL` / `FIXED_DATE`.
- `synthetic` (mod, `heavy-tests`) — large generated histories: `Scale` (`of`, `HEAVY`, `with_commit_graph`, `with_realistic_commits`, `with_refs` (50k-ref variant: packed branches + upstreams + `[branch]` sections), `commits`, `name`), `CommitGraph`, `CommitSize` (`Small` default, `Realistic` sized after git/git), `ensure`, `borrow`.
- `remote` (mod) — real remotes for M4's tests; no TLS, no SSH. Every `git` hermetic: fixed identity/clock, no system/global config, `HOME` in the scratch, `GIT_TERMINAL_PROMPT=0`.
- `remote::LocalRemote` (struct) — bare clone of a shape + a working clone with real `origin`/upstream, `file` transport. `new(Shape) -> Result<_, CorpusError>`, `bare`, `work`, `url` (file://), `default_branch`, `commit_on_remote` (teammate clone pushes; work falls behind), `commit_locally` (work goes ahead), `git(dir, args)` (hermetic, stdout trimmed). Oids are 40-hex `String`s.
- `remote::HttpRemote` (struct) — a bare repo over smart HTTP (`tiny_http` → `git http-backend` CGI) on `127.0.0.1:<ephemeral>`, Basic auth (`401` + `WWW-Authenticate` otherwise). `serve(bare, user, password)`, `url` (user in it), `url_anonymous`, `port`, `requests() -> (authorised, rejected)`. Each `http-backend` runs in its own process group; Drop kills the running group (hooks included), joins the listener within 2 s and frees the port — or panics (fails the test) if the thread still runs; never leaves it behind silently.
- `remote::HungRemote` (struct) — loopback port that accepts and never answers. `listen`, `url`, `connections`. Non-blocking listener: Drop unparks and joins it (no wake-up connect), then closes held streams.
