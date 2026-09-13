# m1-the-graph/49 — Commit handles and the shared id table

Status: partial — seam (a) done, seam (b) remains (handoff below)
Build with: **Opus** (high) — gix commit-graph internals; the table's shape is structural for three crates, but the approach is decided in ADR-0026 and correctness is round-trip tests, so not Fable.
Needs: none (first ticket of the group; defines the types and trait additions the others fill in).
Blocked by: 53

## Read first
- ADR-0026 (all; "Decision" items 1–2 are this ticket)
- DESIGN.md §3.1 (`EmittedOrder`, `walker_state`), §8.2 (`core` holds plain data; `git`/`graph` are siblings)
- `crates/graft-core/MAP.md` (`oid`), `crates/graft-git/MAP.md` (`history`, `repo`), `crates/graft-graph/MAP.md` (`layout::source`, `layout::order`), `crates/graft/MAP.md` (`history::source`)
- Ticket 46 "What holds memory at 1M" table (the sizes this replaces). Scouts for source.

## Scope
In:
- `graft-core`: new module `commit_handle` with `CommitHandle`, `Parents`, `Link`, `HandleSet` (below). Plain data plus one bitset container; say in the module doc why the container is within ADR-0007.
- `graft-git::history::CommitTable`: one table per open repository. With a commit-graph (`gix` `Repository::commit_graph()`): a handle below `graph_count` is the commit's position in the commit-graph (chains included: use gix's global positions); `id_of` reads the graph, `handle_of` looks the id up in the graph. Above `graph_count`: the overflow interner — raw id bytes at the repository's hash length in one `Vec<u8>`, a `HashMap<u64, u32>` keyed by the id's first 8 bytes, every hit verified against the stored bytes, and a `HashMap<Oid, u32>` side map for a genuine prefix collision. Interior mutability (`RwLock`), `Send + Sync`, shared as `Arc<CommitTable>`.
- `Walk::all_refs`, `Walk::from_tips`, `Walk::full_topological` take `Arc<CommitTable>`; `Walk::next_links` added beside `next_chunk`; `Walk::next_ids` and `IdChunk` (27c) removed once `graft::history::topological_order` uses `next_links` (in this ticket: it only keeps `link.commit`, so the switch is one call site).
- `graft-graph::layout::source::CommitSource` gains `links`, `handle_of`, `id_of` (below). `pull` and `read` keep today's shapes in this ticket — the hydrate-on-demand ticket changes them. Every implementor (`graft::history::WalkSource`, `graft-graph::Included`, test sources) implements the new methods; `WalkSource` and `Included` do so through the table.
- `graft::history::WalkSource::new` / `in_order` take the `Arc<CommitTable>`; `RepoStore` (or `SharedRepo`) opens the table once and hands it out.
- MAP.md of every crate touched; ARCHITECTURE.md if a module is added (use `codebase-design`).

Out: changing what `EmittedOrder`, `Included`, `Topological`, `Dated`, `Views.promoted` store (next tickets); hydration; any memory measurement beyond a unit-level `approximate_bytes`.

## Signatures
```rust
// graft-core::commit_handle
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct CommitHandle(pub u32);           // dense per open repository; stable for the process's life
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Parents { None, One(CommitHandle), Two(CommitHandle, CommitHandle), Many(Vec<CommitHandle>) }
impl Parents { pub fn iter(&self) -> impl Iterator<Item = CommitHandle> + '_; pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool; pub fn first(&self) -> Option<CommitHandle>; }
impl FromIterator<CommitHandle> for Parents {}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Link { pub commit: CommitHandle, pub parents: Parents }   // no allocation for ≤ 2 parents
#[derive(Clone, Default, Debug)]
pub struct HandleSet { /* Vec<u64> */ }
impl HandleSet {
    pub fn new() -> Self;
    pub fn insert(&mut self, h: CommitHandle) -> bool;   // grows as needed; true if newly inserted
    pub fn contains(&self, h: CommitHandle) -> bool;
    pub fn remove(&mut self, h: CommitHandle) -> bool;
    pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool;
    pub fn approximate_bytes(&self) -> usize;
}

// graft-git::history (table.rs)
pub struct CommitTable { /* graph: Option<gix::commitgraph::Graph>, graph_count: u32, overflow: RwLock<Overflow> */ }
impl CommitTable {
    pub fn open(repo: &RepoHandle) -> Result<Self, TableError>;      // binds the commit-graph if the repository has one
    pub fn has_commit_graph(&self) -> bool;
    pub fn graph_count(&self) -> u32;                                 // commits in the graph; 0 without one
    pub fn intern(&self, id: &Oid) -> CommitHandle;                   // graph position, else overflow (appends once)
    pub fn handle_of(&self, id: &Oid) -> Option<CommitHandle>;        // never appends
    pub fn id_of(&self, handle: CommitHandle) -> Oid;                 // handles come only from this table: infallible; panics on a foreign handle, documented
    pub fn len(&self) -> usize;                                       // graph_count + overflow entries
    pub fn overflow_len(&self) -> usize;                              // entries not in the graph (the 2× check reads this)
    pub fn approximate_bytes(&self) -> usize;                         // heap only; the mapped graph is not counted
}
pub enum TableError { /* opening the commit-graph failed: name the file and the gix error */ }

// graft-git::history
pub struct LinkChunk { pub links: Vec<Link>, pub more: bool }
impl Walk {
    pub fn all_refs(repo: &RepoHandle, order: Order, table: Arc<CommitTable>) -> Result<Self, WalkError>;   // existing params kept, table added
    pub fn from_tips(/* existing */, table: Arc<CommitTable>) -> Result<Self, WalkError>;
    pub fn full_topological(/* existing */, table: Arc<CommitTable>) -> Result<Self, WalkError>;
    pub fn next_links(&mut self, stop: &dyn Fn() -> bool) -> Result<LinkChunk, WalkError>;   // parents as walked (untrimmed); same order as next_chunk
    pub fn table(&self) -> &Arc<CommitTable>;
}
// removed: Walk::next_ids, IdChunk

// graft-graph::layout::source
pub trait CommitSource {
    fn pull(&mut self, stop: &dyn Fn() -> bool) -> Result<Pull, /* existing error type */>;   // unchanged this ticket
    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, /* existing */>;                   // unchanged this ticket
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, /* existing */>;      // NEW: parents as this view shows them (Included trims), in the given order
    fn handle_of(&self, id: &Oid) -> Option<CommitHandle>;                                   // NEW
    fn id_of(&self, handle: CommitHandle) -> Oid;                                            // NEW
}
```
Callers: `graft::history::WalkSource` (implements), `graft_graph::Included` (delegates, trims in `links`), `graft::history::topological_order` (uses `next_links`), `RepoStore` (opens the table once).

## Acceptance
- [ ] Round-trip on every corpus shape: for each commit `id_of(intern(id)) == id`, `handle_of(id) == Some(intern(id))`, `intern` twice gives one handle, `len()` equals the commit count; with and without a commit-graph (the 5k synthetic pair in the heavy tier covers the graph path at scale; a corpus shape gets a commit-graph written in-test for the fast tier).
- [ ] A commit absent from the graph (write the graph, then add a commit) interns above `graph_count`, `overflow_len() == 1`, round-trips.
- [ ] A constructed 8-byte prefix collision (two ids sharing their first 8 bytes) resolves to two handles, both round-trip.
- [ ] SHA-256 hash length handled (a unit test on the overflow with 32-byte ids; a repo fixture is not required).
- [ ] `HandleSet`: insert/contains/remove/len property test against `HashSet<u32>` at 64 cases.
- [ ] `Walk::next_links` yields the same commit sequence as `next_chunk` on every corpus shape and the 5k synthetic pair; `stop` honoured mid-walk (27c's `next_ids` tests moved over, `next_ids` gone).
- [ ] `topological_order` uses `next_links`; git/git no-cg promotion still lands (`history/tests.rs` promotion tests green).
- [ ] MAP.md updated in core, git, graph, graft; ARCHITECTURE.md if the crate map text changes.
- [ ] Report: `approximate_bytes` of the overflow at 100k no-cg (heavy tier, printed), expected ≤ 60 B/commit.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Seam if it grows: (a) core types + `CommitTable` + tests; (b) `Walk` and `CommitSource` additions + implementors. Stop at the seam with a handoff note rather than compacting.

## Handoff (2026-09-14, builder-opus; stopped at seam (a), no compaction)

Context passed the ~80k target once seam (a) was green, so stopped per Size.

**Done (seam a):**
- `graft-core::commit_handle` — `CommitHandle` (+ `index()`), `Parents`, `Link`, `HandleSet` as specified; module doc says why the bitset is within ADR-0007. Tests incl. 64-case proptest vs `HashSet<u32>`. `proptest` added as a core dev-dependency.
- `graft-git::history::table` — `CommitTable` + `TableError`, re-exported from `history`. Opens via `commit_graph_if_enabled` (same rule the walk uses, so handles agree with `Walk::has_commit_graph`). Tests: every corpus shape round-trips with and without a commit-graph (non-UTF-8 fixture skipped on APFS, said); late commit → overflow above `graph_count`, `overflow_len()==1`; 8-byte prefix collision → two handles; SHA-256 overflow; `Send + Sync`. Heavy-tier tests written, not run (builders run fast tier only): 5k pair round-trip; 100k no-cg overflow `approximate_bytes` printed, asserted ≤ 60 B/commit.
- MAP.md: core and git updated for the new items.

**Remaining (seam b), exact next steps:**
1. `Walk::{all_refs, from_tips, full_topological}` gain `table: Arc<CommitTable>` (last param); store it; `Walk::table()`.
2. `Walk::next_links(limit, stop) -> LinkChunk { links, more }`: reuse `Walk::advance`; parents come from `self.nodes.node(id)?.parents` (node.rs) — check whether `Topological`/`Dated` already hold the node at emit time to avoid a second lookup. Intern commit + parents. Note: the spec signature omits `limit`, but `next_chunk`/`next_ids` both take one and `topological_order` passes `CHUNK`; keep `limit` unless planner says otherwise. `LinkChunk` spec has `more` only, not `done`/`stopped` — decide whether `stopped` is needed for `stop` semantics (next_ids tests check it).
3. Move `next_ids` tests (`history/tests.rs` ~L545–580) to `next_links`; delete `next_ids`, `IdChunk`; update MAP.
4. `graft/src/history/mod.rs` `topological_order` (~L362–380) → `next_links`, keep `link.commit` mapped through `table.id_of`.
5. `CommitSource` (graft-graph/src/layout/source.rs) + `links`, `handle_of`, `id_of`. Implementors: `graft::history::WalkSource`, `graft_graph::Included` (inclusion.rs L280, trim parents in `links`), test sources `layout/stream/tests.rs` (VecSource, Forgetful, Stalling), `inclusion/tests.rs` VecSource, `graft-graph/tests/it/streaming.rs` WalkSource, `benches/hot_paths.rs` InMemory. Test sources need their own id↔handle map (graph may not depend on git).
6. `WalkSource::new/in_order` take the `Arc<CommitTable>`; `RepoStore` (graft/src/repos.rs) opens it once. Call sites of `Walk::all_refs`: graft search.rs (2), history/mod.rs (3), history/tests.rs (~15), graft-ui/tests/support/corpus.rs, graft-graph/tests/it/{corpus,streaming}.rs, graft-git/tests/it/corpus.rs, graft-git history/tests.rs (~20).
7. MAP.md for graph and graft; check ARCHITECTURE.md crate-map text.

Gotchas: workspace denies `expect_used`/`unwrap_used` in lib code — use `match` + `panic!` with a reason. Shape fixtures: use `try_borrow` + the "not valid UTF-8" skip pattern.
