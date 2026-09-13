# m1-the-graph/52 — Rows hydrate on demand: the layout lays out links, reads commits only near the viewport

Status: ready
Build with: **Opus** (high) — the streaming layout's core semantics (pass-through, materialisation, rebuild fingerprints); correctness is judged by determinism tests and the memory curve, so not Fable.
Needs: `Link`, `Parents`, `CommitHandle` (core); `CommitSource::{links, handle_of, id_of}`, `Walk::next_links` (commit-handles-and-table); `EmittedOrder` on handles, `Lane.waiting_for: CommitHandle` (order-and-inclusion-on-handles); the walk yielding links from the graph (walk-reads-the-commit-graph) for the cg-path numbers.
Blocked by: 49, 50, 51

## Read first
- ADR-0026 "Decision" item 4, "Options considered → Churn", "What it costs"
- DESIGN.md §3.6 (streaming, row cache with eviction, checkpoints, cold deep jump)
- Ticket 46 table rows mapped pages, `RepoNodes::node`, allocator retained; ticket 27b "Headline: RSS is not heap"
- `crates/graft-graph/MAP.md` (`layout`, `layout::stream`, `layout::source`, `row`), `crates/graft/MAP.md` (`history::source`, `history` `serve`/`Views`)

## Scope
In:
- `CommitSource::pull` yields `Link`s (handles, parents as the view shows them). `read` takes handles and returns whole `Commit`s in the same order. `Included` trims parents in `pull` and `links`.
- `LayoutCursor::extend` takes links and returns `RowShape`s (no `Commit`). `Row::assemble` builds a `Row` from a shape and its commit; `commit.parents` is set to the shape's outgoing parent ids in order (the view's trimmed parents, exactly what rows hold today), so snapshots do not move.
- `StreamingLayout::advance` **materialises** (reads commits, assembles rows, caches the chunk) only chunks intersecting `viewport ± buffer_rows`; every other chunk **passes through**: cursor advanced, order noted, checkpoints taken, a fingerprint computed over the shapes (row count + hash of lane/incoming/outgoing/lanes), nothing read from the odb. `rows(range)` over a passed-through range rebuilds from the nearest checkpoint (replaying `links`), materialises only the chunks asked for, and checks the fingerprint (`RebuildDiffers` on mismatch, as today).
- `Resident` gains `passed_through` (rows laid out never materialised); `Work` gains `hydrated` (commits read) and `links_read`.
- `graft::history::WalkSource` implements the new `pull`/`read` (`next_links`; `read` through `read_commit` or a batched equivalent); `serve`'s `Views` unchanged in shape.
- MAP.md for `graft-graph` and `graft`; DESIGN.md §3.6 amended in one paragraph (what a pass-through chunk is; what a rebuild reads).

Out: the walk's internals; the table; the strip cache; search (still reads whole commits, ADR-0026 Consequences).

## Signatures
```rust
// graft-graph::layout::source
pub struct Pull { pub links: Vec<Link>, pub done: bool, pub stopped: bool }
pub trait CommitSource {
    fn pull(&mut self, stop: &dyn Fn() -> bool) -> Result<Pull, /* existing error type */>;
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, /* existing */>;
    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, /* existing */>;   // whole commits, same order
    fn handle_of(&self, id: &Oid) -> Option<CommitHandle>;
    fn id_of(&self, handle: CommitHandle) -> Oid;
}
// graft-graph::row
pub struct RowShape { pub commit: CommitHandle, pub lane: usize, pub incoming: Vec<usize>, pub outgoing: Vec<(usize, CommitHandle)>, pub lanes: Vec<Option<BranchIdentity>> }
impl Row {
    pub fn assemble(shape: RowShape, commit: Commit, id_of: &dyn Fn(CommitHandle) -> Oid) -> Row;   // commit.parents := outgoing parents' ids, in order
}
// graft-graph::layout
impl LayoutCursor {
    pub fn extend(&mut self, links: &[Link], order: &mut EmittedOrder) -> Result<Vec<RowShape>, LayoutError>;
}
// graft-graph::layout::stream
impl<S: CommitSource> StreamingLayout<S> {
    pub fn advance(&mut self, budget: usize, stop: &dyn Fn() -> bool) -> Result<Advance, StreamError>;   // signature unchanged; materialises only near the viewport (set_viewport)
    pub fn rows(&mut self, range: Range<usize>) -> Result<Vec<Row>, StreamError>;                         // unchanged; rebuild path hydrates the range
}
pub struct Resident { /* existing */ pub passed_through: usize }
pub struct Work { /* existing */ pub hydrated: usize, pub links_read: usize }
pub fn fingerprint(shapes: &[RowShape]) -> Fingerprint;   // pub(crate) is fine; named so the test can call it
```

**Amended 2026-09-14 (planner-fable, spec-only pass on 49's seam-(b) question):** `Walk::next_links(limit, stop) -> LinkChunk { links, done, stopped }` (ticket 49, amended Signatures). `WalkSource::pull` calls `next_links` with the same `limit` it passes to `next_chunk` today and maps `LinkChunk` to `Pull` field for field (`links`, `done`, `stopped`); no translation, no `more`.

## Acceptance
- [ ] Determinism: on every corpus shape and the 5k synthetic pair, `rows(0..n)` after a cold `advance` to the end (viewport at the top, so all but the first chunks pass through) equals the rows of a layout that materialised everything — same `Row`s, field by field; every rebuilt chunk's fingerprint matches its pass-through fingerprint.
- [ ] `Work.hydrated` after a cold `G` on the 100k fixture with the viewport at the top ≤ `2 × chunk_rows + buffer_rows` (only what the screen asked for); `links_read` ≥ the replay length.
- [ ] Every existing `graft-graph` test and every corpus snapshot green, unchanged.
- [ ] `graft::history` tests green; `frames::` rows-on-hand check still passes in release (a `PageDown` at the frontier must not wait on a rebuild: the materialisation window covers the viewport plus a screen either side, matching `History::outstanding`'s ask policy).
- [ ] Measured and written in the ticket (release, `examples/budgets`, 3 runs): git/git cg and no-cg peak RSS (was 219–226 MiB), `G` time; 1M cg peak RSS and odb objects read during `G` (expected: only the chunks shown).
- [ ] MAP.md and DESIGN.md §3.6 updated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens. Seam: (a) `graft-graph` (`Pull`/`RowShape`/`extend`/pass-through/`rows` rebuild) with tests; (b) `graft::history::WalkSource` + measurements. Stop at the seam with a handoff note rather than compacting.
