# m1-the-graph/05 — The layout engine: lanes, rows, edges

Status: ready
Build with: **Fable** (the graph layout engine — PROGRESS.md routing; xhigh-class work)
Blocked by: 01

Why: this is the centre of the product. Every invariant in PROMPT.md §1.2 is either true here or nowhere, and "is the history shape obvious at a glance?" is decided by lane choices long before a renderer draws anything.

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
- [ ] A `proptest` per PROMPT.md §1.2 invariant, mapped through DESIGN.md §3.7's table: every parent edge drawn; no edge passes through an unrelated node; output deterministic; a lane never changes column mid-life. 64 cases fast / 1000 heavy.
- [ ] Snapshot tests of the row/lane output for every fixture shape from ticket 01 — these snapshots are the record of what "obvious at a glance" means, so make them readable by a human.
- [ ] Lane reuse never happens on the row that freed the lane (a directed test, not only a property).
- [ ] Two runs over the same commits produce byte-identical output, including tie-broken cases with equal dates.
- [ ] Branch-name recovery covered for all four message shapes plus the no-match fallback.
- [ ] `graph` has no dependency but `core` (the edge test proves it).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
