# m1-the-graph/51 — Order, inclusion and the promoted order on handles

Status: ready
Build with: **Sonnet** (medium) — a key-type substitution across `graft-graph` and `graft::history`, held by property tests, the corpus snapshots and the promotion tests. Escalates to Opus on a second failure.
Needs: `CommitHandle`, `HandleSet` (core), `CommitSource::{handle_of, id_of}` (commit-handles-and-table).
Blocked by: 49

## Read first
- ADR-0026 "Decision" item 2, "The 2× no-commit-graph `EmittedOrder`"
- DESIGN.md §3.1 (`EmittedOrder`, `LayoutCursor`), §3.6 (checkpoints, rebuild, promotion paragraphs)
- Ticket 46 table rows `EmittedOrder::note`, `Included`, `Views.promoted`
- `crates/graft-graph/MAP.md` (`layout`, `layout::order`, `layout::stream`, `inclusion`), `crates/graft/MAP.md` (`history`: `Views`, `Promote`, `topological_order`, `Open`)

## Scope
In:
- `EmittedOrder`: `Vec<CommitHandle>` row → handle, plus a dense `Vec<u32>` handle → row (`u32::MAX` = not emitted), grown to the largest handle seen. Public API keeps its names; ids in and out go through the source (`row_of` by `Oid` stays for callers that have one).
- `Included.walked` and `shown`: `HandleSet`. `reinclude` re-reads through `links`, not whole commits.
- `LayoutCursor.waiting`: keyed by `CommitHandle`; `Lane.waiting_for` becomes a handle. `Row` is unchanged (`outgoing` still carries the parent `Oid`; this ticket converts with `id_of` at row build; the hydration ticket moves that into `Row::assemble`).
- `Views.promoted`: `Arc<[CommitHandle]>`; `Promote.order`, `Product::TopologicalOrder`, `topological_order`'s return, `StreamingLayout::from_order`, `WalkSource::in_order` all carry handles.
- `RepeatedCommit` keeps `Oid`s for its message; converted on the error path only.
- **The 2× check:** a heavy test on the 1M no-cg fixture is too heavy for the tier; instead, on the 100k no-cg fixture, after `G` and after promotion assert `CommitTable::overflow_len() == 100_000` and `EmittedOrder::len() == 100_000` for the promoted view. Then run the same check by hand on the 1M no-cg fixture (`examples/budgets` with a print) and write the two numbers in the ticket. If either exceeds the commit count: stop, report where the extra handles are interned (which caller), and do not paper over it.
- MAP.md for `graft-graph` and `graft`.

Out: `pull`/`read` shapes and hydration (rows-hydrate-on-demand); the walk (walk-reads-the-commit-graph); any change to `Row`'s fields.

## Signatures
```rust
// graft-graph::layout::order
pub struct EmittedOrder { /* rows: Vec<CommitHandle>, row_by_handle: Vec<u32> */ }
impl EmittedOrder {
    pub fn new() -> Self;
    pub fn from_handles(order: Vec<CommitHandle>) -> Self;                  // replaces from_ids
    pub fn note(&mut self, commit: CommitHandle) -> Result<usize, RepeatedCommit>;   // the row it took
    pub fn row_of(&self, commit: CommitHandle) -> Option<usize>;
    pub fn handle_at(&self, row: usize) -> Option<CommitHandle>;           // replaces id_at
    pub fn handles(&self) -> &[CommitHandle];                              // replaces ids
    pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool;
    pub fn truncate(&mut self, rows: usize);
    pub fn approximate_bytes(&self) -> usize;
}
pub struct RepeatedCommit { pub commit: CommitHandle, pub first: usize, pub again: usize }

// graft-graph::layout::stream — id-facing API kept for the app
impl<S: CommitSource> StreamingLayout<S> {
    pub fn from_order(source: S, order: Vec<CommitHandle>, config: StreamConfig) -> Self;   // was ids
    pub fn row_of(&self, id: &Oid) -> Option<usize>;     // unchanged: source.handle_of then order
    pub fn id_at(&self, row: usize) -> Option<Oid>;      // unchanged: order then source.id_of
}
// graft-graph::lane
pub struct Lane { pub index: usize, pub identity: BranchIdentity, pub waiting_for: CommitHandle, pub reserved: bool }
// graft-graph::inclusion
impl<S: CommitSource> Included<S> { pub fn walked(&self) -> &HandleSet; pub fn shown(&self) -> &HandleSet; }

// graft::history
pub fn topological_order(/* existing */) -> Result<Option<Vec<CommitHandle>>, WalkError>;
pub struct Promote { pub order: Arc<[CommitHandle]>, pub anchor: Option<Anchor>, pub height: usize }
impl WalkSource { pub fn in_order(handle: RepoHandle, table: Arc<CommitTable>, order: Arc<[CommitHandle]>) -> Self; }
```

## Acceptance
- [ ] `graft-graph` property tests (invariants, determinism, rebuild-equals-evicted fingerprint) and every corpus snapshot pass byte-for-byte unchanged.
- [ ] `graft::history` promotion tests (27c's five, m1/18's) green; `page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place` included.
- [ ] `EmittedOrder::approximate_bytes` at 100k ≤ 10 B/commit (printed in the heavy budget test; was ~90).
- [ ] The 2× check numbers written in the ticket for 100k and 1M no-cg (`overflow_len`, `len` after `G` and after promotion), both equal to the commit count — or the cause named and the ticket stopped.
- [ ] MAP.md updated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Seam: (a) `graft-graph` (order, lane, inclusion, stream); (b) `graft::history` (promotion path) + the 2× check. Stop at the seam with a handoff note rather than compacting.
