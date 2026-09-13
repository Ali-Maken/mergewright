# m1-the-graph/51 — Order, inclusion and the promoted order on handles

Status: in progress — seam (a) done, seam (b) pending (see Handoff below)
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

## Handoff (session 2, stopped at the seam)

**Seam (a) — done, green.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
(workspace), `cargo clippy --all-targets --features heavy-tests -- -D warnings` (graft-graph),
`cargo test -p graft-graph` all pass. Changed: `layout/order.rs` (`EmittedOrder` on handles),
`lane.rs` (`Lane.waiting_for: CommitHandle`), `layout.rs` (`LayoutCursor` on handles, new
`HandleLookup` trait + private `LocalHandles` backing the unchanged `layout()` signature),
`layout/stream.rs` (`StreamingLayout` internals on handles; `row_of`/`id_at` still Oid-facing),
`inclusion.rs` (`Included.walked`/`shown` → `HandleSet`, plus a private `sequence: Vec<CommitHandle>`
kept for walk order — see below — `reinclude` replays via `links`, not `read`). Test files across
graft-graph updated to match (unit, property, corpus and streaming integration tests).

**Design note on `Included`:** `walked`/`shown` are `HandleSet`s as specified, but a private
`sequence: Vec<CommitHandle>` also exists, because the decision algorithm is inherently
walk-order-dependent (`Frontier::decide` must run in the order commits were walked) and
`CommitHandle` numeric order does **not** track walk order in general (a parent can be interned
by `links()` before it is itself walked). A pure bitset can't answer "what came after position
`decided`". `sequence` costs 4 bytes/commit — a real cut from the old `Vec<Oid>`'s 33, just not
the 0.125 B/commit a bitset alone would give. Flagging this as a considered deviation, not a
silent one — worth a look if the reviewer wants the extra 4 B/commit back.

**`StreamingLayout::from_order` kept its `seeds: Seeds` parameter** and `Result` return type
(only `ids: Vec<Oid>` → `order: Vec<CommitHandle>` changed). The signature block above elides it
to `(source, order, config) -> Self`, matching the block's style of eliding unrelated params
elsewhere (e.g. `topological_order(/* existing */)`); removing `seeds`/`Result` would be a real
behavioural change (branch-tip lane seeding, error handling) I didn't think was intended. Not
verified against Fable's original intent — flag if wrong.

**Bug found and fixed along the way:** two tests built a *fresh* `WalkSource`/table and tried to
resolve a topological order's ids through it via `CommitSource::handle_of` before ever walking —
always `None`, since a fresh table starts empty. Fixed by interning
(`table.intern`)/reusing the source's own table instead of just looking up
(`crates/graft-graph/tests/it/streaming.rs`, `promotion_on_a_real_repository_finds_the_selected_commit_again`).
Cause fix, not symptom: the bug was in the test's setup, not in production code, and it's now
fixed instead of worked around.

**Seam (b) — not started, and it has a real blocker already visible.** `cargo test --workspace`
(run once, seam (a) work in place) shows 6 pre-existing failures, all new, all in
`crates/graft/src/history` (`a_search_jump_in_flight_when_promotion_lands_reaches_its_match`,
`g_pressed_while_promotion_swaps_reaches_the_bottom`,
`page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place`,
`promotion_keeps_the_selected_commit_where_it_is_on_screen`,
`promotion_asked_after_g_still_reaches_the_bottom`,
`promotion_asked_after_a_search_jump_still_reaches_its_match`) — confirmed via `git stash` that
these passed before this session's changes. Panic: `<oid> has no handle in this source's table`,
from `layout.rs`'s `HandleLookup` blanket impl. Root cause: `LayoutCursor::new` now resolves each
seed's tip to a handle *before* any commit is pulled; `crates/graft/src/history/source.rs`'s
`WalkSource::in_order(handle, ids: Arc<[Oid]>, table)` (the promoted-order source) is handed a
`table` that doesn't yet have the seed tip's handle interned at that point — this used to be fine
because the old, Oid-keyed `LayoutCursor` never needed a handle lookup at all. These are exactly
the tests this ticket's acceptance criteria name (27c's five, plus
`page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place`) — the seam (b) handle
conversion (`Promote.order`, `topological_order`, `WalkSource::in_order`, `Views.promoted` all on
`Arc<[CommitHandle]>`, sharing one `Arc<CommitTable>` throughout) should fix this as a side effect
of threading a shared, already-populated table through; if it doesn't, this is the first thing to
chase.

**Still to do (seam b):** convert `crates/graft/src/history/mod.rs` (`Promote`, `Views`,
`topological_order`) and `crates/graft/src/history/source.rs` (`WalkSource`, `Feed::Order`) to
carry `CommitHandle`/`Arc<[CommitHandle]>` per the Signatures block above; fix the 6 failing
tests; the 2× check (100k assertion in a heavy test, plus the by-hand 1M `examples/budgets` run,
numbers written here); `crates/graft/MAP.md` update; full gate suite again at the end
(`cargo fmt --check`, both clippy invocations, `cargo test`).

**Not yet run this session:** the 100k/1M 2× check, `examples/budgets`, and `graft`'s own
`cargo clippy --features heavy-tests`.
