# graft-fixtures — map
Test infrastructure: the named fixture corpus, generated histories, real remotes and the capture mechanism. It shells out to real `git` and owns nothing a production code path would want.
Depends on no Graft crate; any crate may take it as a dev-dependency only (ARCHITECTURE.md).
One line per public item. History lives in `docs/log/` and the ADRs.

## lib.rs
- `CORPUS_NOW` (const) — `i64` Unix seconds every relative-date corpus commit is dated against.
- `PINNED_NOW` (const) — `i64` Unix seconds, the "now" every capture producer draws ages against.
- `borrow` (fn) — `borrow(Shape) -> PathBuf`; one corpus fixture, read-only; panics if unavailable.
- `copy_into_temp` (fn) — `copy_into_temp(Shape) -> Scratch`; a writable copy deleted on drop; panics if unavailable.

## shape (re-exported)
- `Shape` (enum) — one hand-written corpus repository per history or ref shape.
- `Shape::ALL` / `STOPPED` / `STEPPED` / `TRACED` / `WIDE` (const) — all shapes; and the conflict, two-stop, blame and many-lane subsets.
- `Shape::name` / `from_name` / `every` (fn) — the shape's directory name, its inverse, and an iterator over `ALL`.

## corpus (re-exported)
- `ensure` (fn) — `ensure() -> Result<Report, CorpusError>`; build the corpus cache if missing.
- `build_into` (fn) — `build_into(&Path) -> Result<Report, CorpusError>`; build the corpus into a directory.
- `try_borrow` / `try_copy_into_temp` (fn) — `borrow` / `copy_into_temp` returning why a shape is unavailable.
- `Report` (struct) — what `ensure` / `build_into` did: bytes written and the `unavailable` shapes.
- `Unavailable` (struct) — `shape`, `reason`: a shape this machine cannot hold.
- `CorpusError` (enum) — why the corpus could not be built.

## cache (re-exported)
- `CacheError` (enum) — why the fixture cache could not be used.

## scratch (re-exported)
- `Scratch` (struct) — a temp directory deleted on drop; `path`.
- `FIXED_ROOT_VAR` (const) — `MWR_SCRATCH_ROOT`: when set, scratches get stable names under that root.

## memory (re-exported)
- `measure` (fn) — `measure(work) -> Result<(T, Measured), MemoryError>`; run work and report its peak memory rise.
- `peak_rss` (fn) — `peak_rss() -> Result<u64, MemoryError>`; this process's peak resident set size.
- `Measured` (struct) — one measurement; `rise`.
- `footprint` (fn) — `footprint() -> Result<Footprint, MemoryError>`; physical footprint, macOS only.
- `Footprint` (struct) — `current`, `peak`, in bytes.
- `MemoryError` (enum) — why memory could not be read, `Unsupported` off macOS.

## tier (re-exported)
- `proptest_cases` (fn) — `proptest_cases() -> u32`; property-test case count for the tier in force.

## capture
- `machine` (mod) — what a committed capture may never hold and the placeholders standing in.
- `UPDATE_VAR` (const) — `MWR_UPDATE_SHOTS`: set it to rewrite captures instead of asserting them.
- `updating_shots` / `updating` (fn) — whether a given value / the environment asks for captures to be rewritten.
- `check` / `check_bytes` (fn) — `check(&Path, drawn) -> Result<(), String>`; compare one capture, or write it when updating.
- `capture` / `capture_bytes` (fn) — `check` that panics with the difference.
- `Captures` (struct) — collects several capture checks and fails once at `finish`; `new`, `check`, `finish`.

## capture::machine
- `Machine` (enum) — the machine-specific classes a capture may not carry; `why`.
- `Found` (struct) — one machine-specific span found in a capture.
- `found` (fn) — `found(&[u8]) -> Vec<Found>`; every machine-specific span in the bytes.
- `redacted` / `redacted_bytes` (fn) — the text with each such span replaced by its placeholder.

## fifo
- `Fifo` (struct) — a named pipe with a thread already blocked reading it, for a hook announcing it runs; `create`, `path`, `wait`.

## git
- `GitError` (enum) — why a system `git` run failed.
- `run` / `run_with_stdin_file` / `run_at` (fn) — run git with fixed identity and clock, machine config off; `run_at` sets the date.
- `run_isolated` (fn) — `run_isolated(cwd, args, stdin) -> Result<Vec<u8>, GitError>`; git with no system or global config.
- `isolated` (fn) — `isolated(cwd, args) -> Command`; the isolated command, unrun.
- `IDENTITY_NAME` / `IDENTITY_EMAIL` / `FIXED_DATE` (const) — the fixture identity and clock.
- `init_repo` (fn) — `init_repo(&Path) -> Result<(), GitError>`; an empty repository with fixture config.
- `version` (fn) — `version() -> Result<String, GitError>`; the system git's version.

## lost_work
- `START` / `END` (const) — Unix seconds bracketing the fixture's story.
- `LOST_ON_MAIN` / `LOST_ON_SPIKE` (const) — subjects of the two lost commits.
- `EDITED` / `DISCARDED_EDIT` / `LOST_EDIT` (const) — the edited file, the discarded edit, the lost edit.
- `Graft` (trait) — Graft's own operations the fixture calls: `fetch`, `discard`, `undo_forced`.
- `Commits` (struct) — the fixture's commit ids: `a`, `b`, `c`, `d`, `e`, `s`, `fetched`.
- `LostWorkError` (enum) — why the fixture could not be built.
- `LostWork` (struct) — the "lost work" repository built fresh per test; `build`, `build_with`, `work`, `origin`, `commits`.

## remote
- `LocalRemote` (struct) — bare clone of a shape plus a working clone over `file://`; `new`, `bare`, `work`, `url`.
- `LocalRemote::default_branch` / `commit_on_remote` / `commit_locally` / `git` (fn) — the branch and helpers moving either side.
- `HttpRemote` (struct) — a bare repo over smart HTTP on loopback with basic auth; `serve`, `url`, `port`, `requests`.
- `HttpRemote::url_anonymous` (fn) — the URL without credentials.
- `HungRemote` (struct) — a loopback port that accepts and never answers; `listen`, `url`, `connections`.
- `HungRemote::wait_for_connection` (fn) — `wait_for_connection(within) -> bool`; block until a client connects.

## synthetic (feature `heavy-tests`)
- `Scale` (struct) — size and options of a generated history; `HEAVY`, `of`, `commits`, `name`.
- `Scale::with_commit_graph` / `with_realistic_commits` / `with_refs` (fn) — variants of a scale.
- `CommitGraph` (enum) — whether the generated repo carries a commit-graph file.
- `CommitSize` (enum) — tiny or realistic commit contents.
- `Report` (struct) — what `ensure` built for a scale.
- `SyntheticError` (enum) — why a generated history could not be built.
- `ensure` / `borrow` (fn) — build a scale's history if missing; its read-only cached path.
- `RECORDS` / `LOST` (const) — reflog entries written and how many name otherwise-lost commits.
- `Reflogged` (struct) — a generated history with a long reflog.
- `with_reflog` / `with_reflog_of` (fn) — build such a history for a scale.

Corpus, synthetic and team fixtures are all built with the app's rename marker (`.git/mergewright/migrated` = `clean`, `Generation::CURRENT`'s, private `settled.rs`, in all three cache keys).

## team
- `TeamSpec` (struct) — a team-monorepo history's shape; `MONOREPO` (heavy tier), `SMALL` (fast tier).
- `RefMix` (struct) — `local`, `remote`, `tags`, `annotated` ref counts.
- `TeamStats` (struct) — `commits`, `merges`, `refs`, `most_open`, read back from a built repository.
- `TeamError` (enum) — `Io`, `Git`, `Cache`, `Spec`.
- `build` (fn) — `build(&TeamSpec, into) -> Result<TeamStats, TeamError>`; build into an empty directory.
- `borrow` (fn) — `borrow(&TeamSpec) -> Result<PathBuf, TeamError>`; a cached read-only copy.
- `stats` (fn) — `stats(&Path) -> Result<TeamStats, TeamError>`; counts read back from a repository.
