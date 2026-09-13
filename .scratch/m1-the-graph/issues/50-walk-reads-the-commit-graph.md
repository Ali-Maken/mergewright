# m1-the-graph/50 — The walk reads the commit-graph, and its state is dense

Status: done
Build with: **Opus** (high) — rev-walk correctness against the git oracle, generation-number rules, commits outside the graph.
Needs: `CommitHandle`, `Parents`, `Link`, `HandleSet` (core), `CommitTable`, `Walk::next_links` (commit-handles-and-table).
Blocked by: 49

## Read first
- ADR-0026 "Decision" item 3, "Options considered → Pack pages, `Topological.known`", "The gate at 1M" (first-paint paragraph)
- ADR-0008; DESIGN.md §3.2 step 2 (both paths)
- Ticket 27b "Fix B (landed)" and ticket 46 table rows `Topological::discover`, `Dated::push`, `RepoNodes::node`, mapped pages
- `crates/graft-git/MAP.md` (`history`); a scout summarises `history/topological.rs` and the dated walk (one page: what `discover` reads per commit, when it first emits on the cg path, what `seen` holds).

## Scope
In:
- **Commit-graph path:** `Topological` takes parents, committer time and generation from the commit-graph (`gix::commitgraph::Graph::commit_at(position)`), never from the object database, for every commit whose handle is a graph position. A commit outside the graph (overflow handle) is header-parsed from the odb and given generation `u32::MAX` (git's `GENERATION_NUMBER_INFINITY` rule: explored first).
- **Per-commit state dense by handle:** in-degree `Vec<u32>` indexed by `handle.0`, plus `HandleSet`s for expanded/emitted; `Node.parents` only for the frontier (commits discovered, not yet emitted). No `HashMap<Oid, _>` keyed by commit remains in the walk. Nothing is kept after emission that a later step reads.
- **No-commit-graph promotion sort:** same dense arrays (`parents: Vec<Parents>`, `time: Vec<i64>`, in-degree, flags), keyed by handle; released when the sort finishes.
- **Dated walk:** `seen` becomes a `HandleSet`.
- Confirm or refute, by a scout plus a timing print in the heavy budget test: on the cg path the first emission waits for the in-degree pass down to the oldest tip's generation (git's `compute_indegrees_to_depth`). Write the answer and the 1M first-paint number into the ticket; do not change the algorithm's semantics either way (a provisional paint on the cg path is a later decision, ADR-0026).
- Update `crates/graft-git/MAP.md` (`Walk` docs: what each path reads).

Out: `EmittedOrder`/`Included`/`Views` (order-and-inclusion-on-handles); hydration; madvise or any pack-mapping work; the commit-graph offer (M2).

## Signatures
No new public items beyond the previous ticket's. Internal shape the reviewer checks for (private, `graft-git::history::topological`):
```rust
struct State { indegree: Vec<u32>, expanded: HandleSet, emitted: HandleSet, frontier: HashMap<CommitHandle, Node> /* discovered, not yet emitted, only */ }
fn node_from_graph(graph: &gix::commitgraph::Graph, pos: gix::commitgraph::Position) -> Node;   // parents, time, generation; no odb read
fn node_from_object(repo: &RepoHandle, id: &Oid) -> Result<Node, WalkError>;                // overflow commits only; generation = u32::MAX
```
`Walk::next_links` / `next_chunk` / `explorations` / `has_commit_graph` keep their signatures and their emitted order.

## Acceptance
- [ ] Every existing git-oracle and order test in `graft-git` passes unchanged (same emitted order on both paths, every corpus shape, the 5k synthetic pair with and without a graph).
- [ ] New test: a repository with a commit-graph plus commits made after it was written emits the same order as `git rev-list --date-order` (the oracle) — the overflow path.
- [ ] New test: a commit-graph chain (two layers, `git commit-graph write --split`) walks correctly.
- [ ] Measured and written in the ticket (release, `examples/budgets`, 1M cg fixture, 3 runs): objects read from the odb during `G` is 0 on the cg path (count them: a counter on the odb read path, printed); peak RSS; `G` time (was 4.0 s); first paint (was 292–346 ms).
- [ ] Measured 1M no-cg: peak live heap of the promotion sort ≤ 40 B/commit (was 267 MiB at peak → ≤ 40 MiB).
- [ ] Heavy-tier 100k budgets green, both paths.
- [ ] The first-paint finding (in-degree pass to the oldest tip: yes/no, cost per commit) written for the acceptance ticket.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens. Seam: (a) cg path from the graph + dense state; (b) no-cg sort + `Dated.seen` + measurements. Stop at the seam with a handoff note rather than compacting.

## Result (builder, 2026-09-14)

Code: dc47d28. `Nodes` is handle-based (`handle`/`id`/`node`/`rereadable`); `node_from_graph` reads by position, `node_from_object` header-parses overflow commits (generation `u32::MAX`). `Topological`: `State { indegree: Vec<u32>, discovered/expanded/emitted: HandleSet, frontier: HashMap<CommitHandle, Node> }`. `Sorted` (no-cg `full_topological`): paged dense arrays by handle, freed at the end. `Dated.seen`: `HandleSet`. `divergence` (ahead/behind) ported to the same `Nodes` trait. `Parents` gained `Default` (core).

Deviations from the spec's private shapes (reviewer, please check):
- `frontier` holds only nodes that are **not** rereadable (overflow commits). A graph commit is re-read from the mapped file instead of kept: stricter than "only the frontier", and matches ADR-0026 item 3 ("no entry is ever hashed"). An extra `discovered` bitset makes that possible.
- `node_from_graph` returns `Result<Node, NodeError>`: a corrupt parent edge is surfaced, not a silent odb fallback.
- `node_from_object(repo: &gix::Repository, id, table) -> Result<Node, NodeError>` (not `&RepoHandle`/`WalkError`): parents are interned there.
- No-cg arrays are `Paged<T>` (2^16-entry pages), not `Vec`: `Vec` doubling put the sort over 40 B/commit.

### Measurements (release, 1M synthetic fixtures, 3 runs each)
Walk odb reads counted with a temporary counter in `node_from_object`, printed by `examples/budgets` and a throwaway probe (both reverted).

| | before | after |
|---|---|---|
| cg: walk objects read from odb during `G` | 1M (via nodes) | **0** (all 3 runs) |
| cg: `G` | 4.0 s | 3.56–3.58 s |
| cg: peak RSS (getrusage) | 933–935 MiB | 646–650 MiB |
| cg: footprint (`time -l`) | 590 MiB | 304–307 MiB |
| cg: first paint | 292–346 ms | 261–327 ms |
| no-cg: `G` / peak RSS | 5.8–6.0 s / 881 MiB | 5.56–5.57 s / 786–787 MiB |
| no-cg: promotion sort peak live heap (counting allocator, table pre-filled) | 267 MiB | **38.3 B/commit** (36.5 MiB) |
| no-cg: same, table filled during the sort | — | 107.7 B/commit (table 56.6 final + its growth transients) |

Remaining `G` RSS on the cg path is hydration (`next_chunk` reads every commit): the hydrate-on-demand ticket.

### First-paint finding (for the acceptance ticket)
**Refuted for Graft.** The incremental sort expands only down to the generation of the candidate it is confirming (the newest tip), not down to the oldest tip's generation as git's `compute_indegrees_to_depth` does. 1M cg fixture, 6,497 tips: first emission after **6,498 node reads, 3.0–3.2 ms** (≈0.5 µs per read from the graph); the whole 1M walk takes 264 ms, 7.8 B/commit peak heap. No-cg `full_topological`: first emission after 1M reads, 2.1 s. So the 261–327 ms app first paint at 1M is **not** the walk; something else linear runs before the first window (RSS at paint still ~215 MiB). Logged in KNOWN_GAPS.

### Heavy tier (debug, once, alone)
- `cargo test -p graft-git --features heavy-tests --lib`: 97/97 (5k synthetic pair with and without a graph included).
- `cargo test -p graft --features heavy-tests --test it heavy::budgets`: 5/5 — 100k with and without a commit-graph, realistic both ways, 50k refs.
