# m1-the-graph/06 — Streaming layout: chunks, checkpoints, eviction, promotion

Status: ready
Build with: **Fable** (the graph layout engine)
Blocked by: 04, 05

Why: two hard budgets live here — first paint in 150 ms and memory that grows with the viewport, not with history. **This ticket is also the one KNOWN_GAPS.md marks REQUIRED (R2-1):** post-promotion checkpoint state on the no-commit-graph path is undefined, and the naive reading costs 100+ MB of checkpoints on the 100k fixture, busting the 300 MB gate.

## Read first
- DESIGN.md §3.6 (all of it), §3.2 step 2's promotion paragraph, §3.1's `LayoutCursor`
- KNOWN_GAPS.md, "REQUIRED M1 TICKET — R2-1" (read it twice; the fix is named there)
- ADR-0008 (first-paint order depends on the commit graph)

## Goal
Layout runs in chunks, resumes from checkpoints, evicts what is off screen, and promotes from date order to topological order without the user losing their place.

## Scope
In:
- `layout_chunk` as the unit of incremental work (~500 commits), with the first chunk sized for the 150 ms first paint.
- A chunked row cache keyed by row range, LRU-evicted outside `[viewport_top − buffer, viewport_bottom + buffer]`. Never an append-only buffer. A rebuilt chunk must be identical to the evicted one — layout is deterministic, so assert that.
- Checkpoints: snapshot `LayoutCursor` every N rows (start at 2,000), cache-only, bounded, LRU, always rebuildable. A deep jump resumes from the nearest earlier checkpoint.
- A cold deep jump is a cancellable background job with progress, never a UI-thread stall.
- **Promotion (R2-1).** After the background topo walk completes, retain the emitted order as a compact commit-id list; post-promotion `walker_state` is an **index into that list**, so checkpoints stay small on both paths. Write the one or two sentences into DESIGN.md §3.2/§3.6 that say so, and implement it. Selection and scroll anchor are re-found by commit id, never by row index.
- Until promotion, the order is provisional and the app can say so (the notice itself is ticket 10's).

Out:
- Compaction and folding predicates (07).
- The budget assertions themselves (16) — but leave the seams a benchmark can grab.
- Durable/on-disk checkpoints: they are cache-only by design.

## Acceptance
- [ ] Scroll 100k rows' worth of a fixture through the cache and show resident chunks stay bounded; report the measured resident set.
- [ ] Evict a chunk, scroll back, assert the rebuilt rows are byte-identical.
- [ ] Jump to a far row from cold: resumes from a checkpoint, not from row 0 (assert the number of commits walked).
- [ ] Promotion test: a fixture with no commit-graph file paints in date order, promotes to topo order, and the selected commit is still selected and still on screen afterwards.
- [ ] **R2-1 closed:** checkpoint size after promotion is O(1) in remaining history — measure checkpoint bytes on the reduced-size large fixture and state the number. DESIGN.md updated in the same commit.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
