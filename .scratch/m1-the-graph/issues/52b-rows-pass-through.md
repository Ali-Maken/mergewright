# m1-the-graph/52b — Rows hydrate on demand: chunks off screen pass through, a rebuild hydrates only what is asked

Status: partial — seam (a) done, seam (b) measurements remain (handoff below)
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

## Handoff (2026-09-14, builder-opus, seam (a) done)

Done (seam a):
- `layout::cache`: fingerprint over `&[RowShape]` (`fingerprint`, running per growing frontier chunk); every chunk laid out keeps `LaidOut { print, materialised }`; eviction drops rows only; `check` / `admit` split; `renote` restores a chunk cut back by `truncate`.
- `StreamingLayout::advance`: materialises a frontier batch only if it starts a chunk meeting viewport ± `buffer_rows`, or continues a held chunk; otherwise passes through.
- `rows` → `rebuild(wanted chunks)`: one replay from the nearest checkpoint, every replayed chunk fingerprint-checked, adjacent wanted chunks materialised as one batch (`admit_run`).
- `Resident.passed_through`, `Work.links_read`; `RowShape: Hash`. MAP.md + DESIGN.md §3.6 paragraph.
- Tests: determinism on the 5k pair vs materialise-everything and `layout()` (odd budget 37, partial chunks pass through); cold jump hydrated ≤ 2×chunk+buffer; deep read merge_reads ≤ merge lanes open at the row. Corpus test asserts pass-through then 0 after the scroll.

Existing tests changed (premise removed by this ticket, not regressions):
- `stream::tests` Forgetful: rewrote commit messages on re-read; a shape fingerprint cannot see a message. Now re-served links drop parents.
- `inclusion::tests::folding_invalidates_only_the_rows_from_the_merge_down`: viewport set over all 42 rows so "nothing kept was rebuilt" still isolates the reinclude.
- `tests/it/streaming.rs` linear: `hydrated == pulled + reread` → `hydrated <` that, `passed_through == 0` after scroll; body moved to `assert_linear_work` (clippy too_many_lines).

Spec conflict to resolve: acceptance wants `merge_reads == 0` on cold G with the viewport at the top, but frontier chunks are materialised one pull at a time, and the existing test `a_merge_lane_open_across_a_chunk_boundary_is_named_with_one_read` (must stay unchanged) expects 1 read across a frontier chunk boundary. So a merge lane crossing chunk 0→1 or 1→2 costs one read. Not changed; measure and decide (carrying names across contiguous frontier batches would make it 0 and break that test).

Next (seam b), exact steps:
1. `crates/graft/tests/it/budget_run.rs` `Numbers` has no `Work`; `serve` never exposes the layout. To report hydrated/merge_reads/links_read for `G`, extend `order_bytes`-style direct drive (it repeats `Views::new` setup) or add a counter at `WalkSource::read` in `crates/graft/src/history/source.rs`.
2. Release, alone, 3 runs each: `cargo run --release -p graft --example budgets -- ~/.cache/graft/git-git.git` (cg and no-cg: check how 46/51 toggled the commit-graph — see `scripts/memory-curve.sh`); 1M cg via `scripts/memory-curve.sh 1000000` (builds a ~350 MiB fixture under target/fixtures, heavy).
3. Record peak RSS (was 219–226 MiB git/git), G time, 1M cg RSS + objects read during G into this ticket. Batch `WalkSource::read` only if per-handle reads show in G.
4. `frames::` rows-on-hand check in release (`crates/graft/tests/it/frames.rs`, heavy tier) and the heavy tier once at the end.
