# m1-the-graph/04 — Reading history: a repo handle that lives, and three walk strategies

Status: ready
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
- [ ] Each strategy asserted against the fixtures: known commit order out of the linear, criss-cross and octopus fixtures; first-parent view of the foxtrot fixture is exactly the default branch's chain.
- [ ] Topological order is emitted **before** the whole range is walked (assert first-emission happens after a bounded number of explorations, not at the end).
- [ ] A repo with no commit-graph file falls to date order and says so through the API.
- [ ] The handle survives across two jobs without re-opening (assert the open count).
- [ ] `Esc` during a walk cancels it and the partial result is discarded; a stale job's result never lands on screen.
- [ ] Feature-set measurements recorded in the ticket, with the fixture and method named.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
