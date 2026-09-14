# m1-the-graph/52b — Rows hydrate on demand: chunks off screen pass through, a rebuild hydrates only what is asked

Status: ready
Build with: **Opus** (high) — the streaming layout's pass-through and rebuild semantics; correctness is judged by determinism tests and the memory curve, so not Fable.
Needs: everything ticket 52 exposes (`RowShape`, `LaneIdentity`, `MergeNames`, `Row::assemble`, `LayoutCursor::extend` on links, `CommitSource::{pull, read}`, `Work.{hydrated, merge_reads}`).
Blocked by: 52

Split out of 52 on 2026-09-14 (planner-fable, spec-only pass): 52 made the layout lay out links with every chunk still materialised; this ticket stops materialising what the screen has not asked for. ADR-0026 item 4 is done when this ticket is.

## Read first
- Ticket 52 "Decision" (why merge identity is deferred; the bound on `merge_reads`) and "Signatures"
- ADR-0026 "Decision" item 4, "Options considered → Churn", "What it costs"
- DESIGN.md §3.6 (streaming, row cache with eviction, checkpoints, cold deep jump)
- Ticket 46 table rows mapped pages, `RepoNodes::node`, allocator retained; ticket 27b "Headline: RSS is not heap"
- `crates/graft-graph/MAP.md` (`layout::stream`, `row`), `crates/graft/MAP.md` (`history` `serve`/`Views`, `History::outstanding`)

## Scope
In:
- `StreamingLayout::advance` **materialises** (reads commits, gathers merge names, assembles rows, caches the chunk) only chunks intersecting `viewport ± buffer_rows`; every other chunk **passes through**: cursor advanced, order noted, checkpoints taken, a fingerprint computed over its shapes, nothing read from the odb.
- One fingerprint kind: `fingerprint(&[RowShape])` (row count + hash of `lane`/`incoming`/`outgoing`/`lanes`; `LaneIdentity` hashes its handles, so it is deterministic across a rebuild). A materialised chunk is fingerprinted from its shapes at materialise time and the value kept with the chunk; eviction keeps the fingerprint, not the rows. `RowCache` stops fingerprinting `&[Row]`.
- `rows(range)` over a passed-through or evicted range rebuilds from the nearest checkpoint (replaying `links`), materialises only the chunks asked for, checks every rebuilt chunk's fingerprint (`RebuildDiffers` on mismatch, as today).
- `Resident` gains `passed_through` (rows laid out never materialised); `Work` gains `links_read`.
- `WalkSource::read` batched if the measurements below want it (one `read` per chunk should be a few ms; if per-handle `read_commit` shows in the `G` number, batch it and say so).
- MAP.md for `graft-graph`; DESIGN.md §3.6 amended in one paragraph (what a pass-through chunk is; what a rebuild reads).

Out: the walk's internals; the table; the strip cache; search (still reads whole commits, ADR-0026 Consequences); anything in 52.

## Signatures
```rust
// graft-graph::layout::stream
impl<S: CommitSource> StreamingLayout<S> {
    pub fn advance(&mut self, budget: usize, stop: &dyn Fn() -> bool) -> Result<Advance, StreamError>;   // unchanged; materialises only near the viewport (set_viewport)
    pub fn rows(&mut self, range: Range<usize>) -> Result<Vec<Row>, StreamError>;                         // unchanged; rebuild path hydrates the range only
}
pub struct Resident { /* existing */ pub passed_through: usize }
pub struct Work { /* existing, incl. 52's hydrated, merge_reads */ pub links_read: usize }
pub fn fingerprint(shapes: &[RowShape]) -> Fingerprint;   // pub(crate) is fine; named so the test can call it
```

## Acceptance
- [ ] Determinism: on every corpus shape and the 5k synthetic pair, `rows(0..n)` after a cold `advance` to the end (viewport at the top, so all but the first chunks pass through) equals the rows of a layout that materialised everything — same `Row`s, field by field, `lanes` included; every rebuilt chunk's fingerprint matches its pass-through fingerprint.
- [ ] `Work.hydrated` after a cold `G` on the 100k fixture with the viewport at the top ≤ `2 × chunk_rows + buffer_rows`; `Work.merge_reads == 0` for that run (every merge lane open in the top chunks opened inside them); `links_read` ≥ the replay length.
- [ ] After `rows(r)` deep in a passed-through stretch: `merge_reads` for that call ≤ the count of distinct `Merge` identities in row `r.start`'s lanes (bounded by lanes, not history); assert it in a test on the 5k pair with a small `chunk_rows`.
- [ ] Every existing `graft-graph` test and every corpus snapshot green, unchanged.
- [ ] `graft::history` tests green; `frames::` rows-on-hand check still passes in release (a `PageDown` at the frontier must not wait on a rebuild: the materialisation window covers the viewport plus a screen either side, matching `History::outstanding`'s ask policy).
- [ ] Measured and written in the ticket (release, `examples/budgets`, 3 runs): git/git cg and no-cg peak RSS (was 219–226 MiB), `G` time; 1M cg peak RSS and odb objects read during `G` (expected: only the chunks shown plus `merge_reads`).
- [ ] MAP.md and DESIGN.md §3.6 updated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens. Seam: (a) pass-through + fingerprint + rebuild with tests; (b) measurements + DESIGN.md. Stop at the seam with a handoff note rather than compacting.
