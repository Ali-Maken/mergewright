# m1-the-graph/05 — The layout engine: lanes, rows, edges

Status: done
Build with: **Fable** (the graph layout engine — PROGRESS.md routing)
Blocked by: 01

Why: this is the centre of the product. Every invariant in PROMPT.md §1.2 is either true here or nowhere, and "is the history shape obvious at a glance?" is decided by lane choices long before a renderer draws anything.

## Progress (2026-09-12) — done

Interrupted mid-flight once; resumed by a second builder. Nothing landed was
rewritten.

Landed on `main`:

- `3c30a91` — `MistakeRadarKind` in `core`; `BranchIdentity` (FNV-1a, stable
  across runs, `of_name` / `of_short_sha`); `recover_branch_name` with unit
  tests for all four message shapes, the no-match fallback and malformed
  variants (acceptance item 5).
- `8a2aa1b` — `Lane`, `Row`, `Seeds`/`BranchSeed`, `LayoutCursor`, `layout()`.
  Lane 0 reserved and wins ties; integration lanes reserved adjacent to 0 and
  never returned to the free pool on any closing path; first parent continues
  the row's lane; extra parents take the lowest free column or open a new one;
  pending-free list at **row** granularity (documented in `layout.rs` — chunk
  granularity would make the layout depend on chunk size); merge-lane identity
  from the message, short-SHA fallback; octopus handled generically. Directed
  tests: freed column reused only on the next row (acceptance item 3), two
  runs byte-identical, chunk-boundary invariance, reserved-lane guards,
  octopus, short-SHA fallback.
- `bbb2fd0` — the mid-flight handoff note.
- `7b07d2a` — property tests for the four §1.2 invariants
  (`src/layout/tests/properties.rs`): a generated DAG of up to 32 commits in
  walk order, replayed row by row from the outside (which columns are live,
  which are retired, which reserved) so every edge, convergence and column
  reuse is checked against what a renderer would see; fast 64 / heavy 1000
  via `graft_fixtures::proptest_cases()`; chunking randomised for the
  determinism property. Dev-deps `proptest`, `graft-fixtures`, `graft-git`,
  `insta` and the `heavy-tests` forwarding feature. Also the equal-date tie
  test (acceptance item 4) and a test for an edge to a parent already drawn
  (commit-date order / clock skew): recorded, lane closes at once.
- `b07a588` — `tests/it/corpus.rs`: one human-readable snapshot per fixture
  shape (18 written under `tests/it/snapshots/`; `NonUtf8RefName` skipped
  with its reason on APFS). Each snapshot: lane grid, short SHA, lane label
  (ref name / recovered merge name / short SHA), subject; header records the
  walk order in force. `KNOWN_GAPS.md` updated ("Layout engine" section).

Verified beyond green gates:

- The property checker catches bugs: two deliberate mutations of `layout.rs`
  (pooling reserved lanes; choosing the highest waiting lane instead of the
  lowest) each failed `rows_replay_cleanly_from_the_outside`. Both reverted.
- All 18 snapshots read row by row against the fixture's intended shape
  (criss-cross, octopus, foxtrot, stash, ref-states traced in full).
- Edge test (`crates/graft/tests/it/dependency_edges.rs`) lists exactly
  `graft-graph -> graft-core`; the new `graft-git`/`graft-fixtures` deps are
  dev-only and not ADR-0006 edges.

Not verified: the heavy tier (1000 cases) — milestone review runs it.
Snapshots are in commit-date order because the corpus has no commit-graph;
they will change when the fixture builder writes one (KNOWN_GAPS.md).

## Read first
- PROMPT.md §1.2 (all of it, including the invariant list) and §1.3's merge-commit rules
- DESIGN.md §3.1, §3.2 steps 1/3/4, §3.3 (pseudocode), §3.4, §3.7
- CONTEXT.md (lane, default branch, integration branch, ref badge)
- ARCHITECTURE.md: `graph` may depend on `core` and nothing else. No terminal code, no I/O, no clock, no threads.

## Goal
`graph`: commits in, `Row`s with lane assignments and edges out — pure, deterministic, and provably obeying every §1.2 invariant.

## Scope
In:
- `Lane`, `Row`, `LayoutCursor` and the branch identity hash per DESIGN.md §3.1. Lane colour comes from branch identity, never from column index.
- Lane 0 is the default branch, straight for its whole life, and wins every tie. Integration branches get reserved lanes adjacent to lane 0 and never compete for lane 0.
- Assignment per §3.2 step 3: incoming lanes converge, the first parent continues in the commit's own lane, each extra parent takes the lowest free column or a new one, never a reserved lane.
- §3.2 step 4's pending-free list: a lane freed on row N is only reusable from row N+1.
- Merge handling per §3.4: the second-parent lane carries the merged branch's identity (that is what makes a merge read as "this colour flowed into that one"), and the merged branch's **name is recovered from the merge message** — `Merge branch 'x'`, `... into y`, `Merge pull request #n from y`, `Merge remote-tracking branch 'x'` — falling back to the merged parent's short SHA without treating that as a failure.
- Octopus merges (three or more parents) handled, not special-cased into a panic.
- `Row.marker` exists as plain input data only; `graph` never calls into `ops` (M2 fills it).

Out:
- Streaming, checkpoints, the row cache, promotion (06).
- Compaction and folding (07).
- Any renderer (08 onward).

## Acceptance
- [x] A `proptest` per PROMPT.md §1.2 invariant, mapped through DESIGN.md §3.7's table: every parent edge drawn; no edge passes through an unrelated node; output deterministic; a lane never changes column mid-life. 64 cases fast / 1000 heavy.
- [x] Snapshot tests of the row/lane output for every fixture shape from ticket 01 — these snapshots are the record of what "obvious at a glance" means, so make them readable by a human.
- [x] Lane reuse never happens on the row that freed the lane (a directed test, not only a property).
- [x] Two runs over the same commits produce byte-identical output, including tie-broken cases with equal dates.
- [x] Branch-name recovery covered for all four message shapes plus the no-match fallback.
- [x] `graph` has no dependency but `core` (the edge test proves it).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
