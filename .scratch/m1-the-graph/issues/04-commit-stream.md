# m1-the-graph/04 — Reading history: a repo handle that lives, and three walk strategies

Status: done (2026-09-12)
Build with: Opus
Blocked by: 01

Why: the layout engine is pure and takes commits in; something has to produce them, in the right order, off the UI thread, cancellably, without re-opening the repository on every read. Three M0 gaps close here: the handle that dies with its job, the untouched `gix` feature set, and the job nothing ever cancels.

## Read first
- DESIGN.md §3.2 step 2 (walk strategies and the commit-graph split) and ADR-0008
- ADR-0001 (default-branch precedence), ADR-0011 (`gix` primary, `git2` per named gap)
- KNOWN_GAPS.md M0 rows: "The repository handle does not outlive the job", "`gix` is compiled with almost no features", "Nothing cancels a job yet"
- `crates/graft-git/src/repo/` and `crates/graft/src/jobs/` as they stand

## Goal
`graft-git` streams commits out of a real repository in any of the three orders, as a cancellable job, from a handle that lives as long as the app.

## Scope
In:
- A handle owner outside `AppState` (which must stay `Clone` and world-free) that jobs borrow. Decide where it lives and write down why in ARCHITECTURE.md if it is a new seam.
- `gix` features: turn on what a rev-walk actually needs (revision walk, commit-graph, object cache, and `parallel` if it earns its place). **Measure before and after** on a fixture and record the numbers — this is the ticket KNOWN_GAPS.md names as the one that decides it.
- Walk strategies behind one interface: **topological** (generation-number ordered via the commit-graph file, emitting as soon as in-degree is final — the path that makes 150 ms first paint), **commit-date** (the natural streaming order, used as first paint when there is no commit-graph file), and **first-parent only**. Deterministic tie-break: date descending, then commit id.
- Detect whether a commit-graph file exists and expose which path is in force, so the UI can say the order is provisional.
- Chunked streaming (~500 commits) with a resumable position, shaped for `layout_chunk` to consume — the layout engine must not need to know where commits come from.
- Default branch and integration branches resolved per ADR-0001, once per repo, off-thread, cached.
- Run it as a `Job`: `AppState` remembers the id it is waiting for, `Esc` cancels, a result for an id nobody awaits is discarded rather than shown. This closes M0's cancellation gap; prove it with a test that presses the key, not only with the runner's own tests.

Out:
- Offering to run `git commit-graph write --reachable` — KNOWN_GAPS.md R2-4 puts the plumbing in M2. The provisional-order notice stays honest until then.
- Lane assignment, promotion, checkpoints (05, 06).
- `.mailmap`, ref badge data (09).

## Acceptance
- [x] Each strategy asserted against the fixtures: known commit order out of the linear, criss-cross and octopus fixtures; first-parent view of the foxtrot fixture is exactly the default branch's chain. — `graft-git/src/history/tests.rs`, oracle is `git rev-list` run against the same fixture (`--date-order`, `--topo-order`, `--first-parent`).
- [x] Topological order is emitted **before** the whole range is walked. — `the_topological_walk_emits_before_it_has_walked_the_range`: on the criss-cross fixture with a commit-graph file written, the first commit is out after fewer commit reads than the repository holds.
- [x] A repo with no commit-graph file falls to date order and says so through the API. — `Walk::requested_order()` / `order_in_force()` / `is_provisional()` / `has_commit_graph()`; tests `a_repository_with_no_commit_graph_file_falls_to_date_order_and_says_so` and `a_repository_with_a_commit_graph_file_runs_the_topological_path`.
- [x] The handle survives across two jobs without re-opening. — `RepoStore::opens()`; `repos::tests::a_repository_is_opened_once_however_often_it_is_asked_for`, `jobs_on_different_threads_share_one_open_repository`, and `jobs::tests::two_jobs_share_one_open_repository` (an `OpenRepository` job then a `ReadHistory` job, one open).
- [x] `Esc` during a walk cancels it and the partial result is discarded; a stale job's result never lands on screen. — `update::tests::esc_cancels_the_walk_the_screen_is_waiting_for`, `a_cancelled_walk_that_answers_anyway_is_discarded`, `a_superseded_walk_loses_to_the_newer_one`, `a_walk_that_ends_without_an_answer_stops_being_awaited`. All press the key through `update`, not through the runner.
- [x] Feature-set measurements recorded in the ticket, with the fixture and method named. — below, and in `crates/graft-git/Cargo.toml` next to the features they justify.

## What was built

| Where | What |
|---|---|
| `graft-git/src/history/node.rs` | The cheap half of a commit (`id`, parents, time, generation), read from the commit-graph file when there is one and from the object database when there is not. `Key` is the date-descending-then-smaller-id tie-break. |
| `graft-git/src/history/simple.rs` | Commit-date and first-parent walks. One loop, one read per commit. |
| `graft-git/src/history/topological.rs` | Git's incremental generation-number sort: emits as soon as a commit's in-degree is final, rather than after the range. |
| `graft-git/src/history/mod.rs` | `Walk` — one interface over the three, `next_chunk(limit, stop)` returning `Chunk { commits, done, stopped }`, `CHUNK = 500`, ADR-0008's fallback from topological to commit date when no commit-graph file exists. |
| `graft-git/src/branches.rs` | ADR-0001's five rules, with `Chosen` saying which one answered. |
| `graft/src/repos.rs` | `RepoStore`: the one open repository for the run, plus the cached branch layout. |
| `graft/src/history.rs` | The job-side glue: borrow the repository, drain the walk, hand back one `History`. |
| `graft/src/update.rs` | `Task`, `AppEvent::Started`, `AppState::awaiting`, `Esc` → `Command::CancelJob`. |

## Feature-set measurements

**Fixture.** `graft-fixtures`' synthetic 20,000-commit repository with no
commit-graph file (`Scale::of(20_000)`), and a `git clone --no-local` copy of
it repacked `-adf --depth=250 --window=250`, to give the delta chains what a
`git gc --aggressive` would.

**Method.** Release build; a throwaway `graft-git/examples/walk_timing.rs`
(deleted afterwards) running one full commit-date `Walk` over all refs in
500-commit chunks; best of three rounds. Times in milliseconds, with and
without the 16 MiB per-handle object cache (`SharedRepo::OBJECT_CACHE_BYTES`).

| feature set | plain fixture (cache on / off) | deep deltas (on / off) |
|---|---|---|
| `sha1` + `sha256` | 56.8 / 82.4 | - |
| + `parallel` | 56.3 / 81.4 | 55.0 / 79.9 |
| + `pack-cache-lru-static`, `-dynamic` | 57.2 / 81.9 | 55.8 / 80.2 |

**Decisions.**
- The pack delta caches are **off**: no measurable difference even against
  depth-250 chains. A feature that buys nothing is a dependency with no payer.
- `parallel` is **on**, for correctness rather than speed. Verified with a
  compile probe: without it `gix-features` swaps `Arc`/`RwLock` for
  `Rc`/`RefCell` and `gix::ThreadSafeRepository` is not `Send + Sync`, so
  `SharedRepo` — one repository shared by every worker — does not compile.
  The alternative is re-opening the repository per job, which is the M0 gap.
- The 16 MiB object cache is the only thing that pays: 82.4 -> 56.8 ms, about
  a third. It is code, not a feature.
- Still off: `revision` (drags in `gix-index`; the walk needs neither
  `describe` nor `merge_base`), `status`, `blame`, `mailmap`. The commit-graph
  reader is not a feature at all — `gix-commitgraph` is a hard dependency.

## What was verified, and how

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`: all pass. 364 fast-tier tests, 0 failures.
- The three orders are checked against `git rev-list` on the corpus, so the oracle is git itself, not a hand-written expectation.
- The topological path is exercised by writing a commit-graph file into a *copy* of a fixture (`git commit-graph write --reachable` in a `Scratch`), because no corpus fixture ships one.
- **Not verified:** nothing has been seen on screen. No widget reads `AppState::history`, so the walk's output has only ever been looked at by tests. No large repository has been walked as part of the product (only the synthetic fixture, through the throwaway timing example). Both are logged in KNOWN_GAPS.md.

## Gaps logged (KNOWN_GAPS.md, "Reading history")

History arrives in one lump rather than streaming to the screen; ADR-0008's
promotion half is not built; `graph.default_branch` is not a config key; the
corpus cannot tell date order and topological order apart; the walk has never
run against `git/git`; nothing reads `AppState::history` yet.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
