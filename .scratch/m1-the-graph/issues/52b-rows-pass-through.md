# m1-the-graph/52b — Rows hydrate on demand: chunks off screen pass through, a rebuild hydrates only what is asked

Status: done — seam (a) 71a7f89, seam (b) measurements + pass-through-adds-zero test (results below)
Build with: **Opus** (high) — the streaming layout's pass-through and rebuild semantics; correctness is judged by determinism tests and the memory curve, so not Fable.
Needs: everything ticket 52 exposes (`RowShape`, `LaneIdentity`, `MergeNames`, `Row::assemble`, `LayoutCursor::extend` on links, `CommitSource::{pull, read}`, `Work.{hydrated, merge_reads}`).
Blocked by: 52

Split out of 52 on 2026-09-14 (planner-fable, spec-only pass): 52 made the layout lay out links with every chunk still materialised; this ticket stops materialising what the screen has not asked for. ADR-0026 item 4 is done when this ticket is.

## Read first
- Ticket 52 "Decision" (why merge identity is deferred; the bound on `merge_reads`) and "Signatures"
- ADR-0026 "Decision" item 4, "Options considered → Churn", "What it costs"
- docs/design/03-graph-layout-algorithm.md §3.6 (streaming, row cache with eviction, checkpoints, cold deep jump)
- Ticket 46 table rows mapped pages, `RepoNodes::node`, allocator retained; ticket 27b "Headline: RSS is not heap"
- `crates/graft-graph/MAP.md` (`layout::stream`, `row`), `crates/graft/MAP.md` (`history` `serve`/`Views`, `History::outstanding`)

## Scope
In:
- `StreamingLayout::advance` **materialises** (reads commits, gathers merge names, assembles rows, caches the chunk) only chunks intersecting `viewport ± buffer_rows`; every other chunk **passes through**: cursor advanced, order noted, checkpoints taken, a fingerprint computed over its shapes, nothing read from the odb.
- One fingerprint kind: `fingerprint(&[RowShape])` (row count + hash of `lane`/`incoming`/`outgoing`/`lanes`; `LaneIdentity` hashes its handles, so it is deterministic across a rebuild). A materialised chunk is fingerprinted from its shapes at materialise time and the value kept with the chunk; eviction keeps the fingerprint, not the rows. `RowCache` stops fingerprinting `&[Row]`.
- `rows(range)` over a passed-through or evicted range rebuilds from the nearest checkpoint (replaying `links`), materialises only the chunks asked for, checks every rebuilt chunk's fingerprint (`RebuildDiffers` on mismatch, as today).
- `Resident` gains `passed_through` (rows laid out never materialised); `Work` gains `links_read`.
- `WalkSource::read` batched if the measurements below want it (one `read` per chunk should be a few ms; if per-handle `read_commit` shows in the `G` number, batch it and say so).
- MAP.md for `graft-graph`; docs/design/03-graph-layout-algorithm.md §3.6 amended in one paragraph (what a pass-through chunk is; what a rebuild reads).

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
- [x] Determinism: on every corpus shape and the 5k synthetic pair, `rows(0..n)` after a cold `advance` to the end (viewport at the top, so all but the first chunks pass through) equals the rows of a layout that materialised everything — same `Row`s, field by field, `lanes` included; every rebuilt chunk's fingerprint matches its pass-through fingerprint.
- [x] `Work.hydrated` after a cold `G` on the 100k fixture with the viewport at the top ≤ `2 × chunk_rows + buffer_rows`; `links_read` ≥ the replay length.
- [x] **Amended 2026-09-14 (planner-fable, spec-only pass; replaces `merge_reads == 0`).** `merge_reads` is bounded by lanes, not history. Two checks:
  1. For the cold-`G` run above: `merge_reads` ≤ Σ over every materialised chunk after the first of the distinct `Merge` identities open in that chunk's first row (52's rule: a merge lane opened above a batch costs one read; frontier chunks are separate batches). `a_merge_lane_open_across_a_chunk_boundary_is_named_with_one_read` stays exactly as it is. Carrying names across contiguous frontier batches is **out**.
  2. Pass-through reads nothing: on the 5k pair with a small `chunk_rows` and the viewport at the top, `Work.hydrated` and `Work.merge_reads` after `advance` to the end **equal** the same counters of a second layout advanced only far enough to materialise the viewport window (the stretch of passed-through chunks after the window adds zero to both). Assert it in a `stream::tests` test.
- [x] After `rows(r)` deep in a passed-through stretch: `merge_reads` for that call ≤ the count of distinct `Merge` identities in row `r.start`'s lanes (bounded by lanes, not history); assert it in a test on the 5k pair with a small `chunk_rows`.
- [x] Every existing `graft-graph` test and every corpus snapshot green, unchanged.
- [x] `graft::history` tests green; `frames::` rows-on-hand check still passes in release (a `PageDown` at the frontier must not wait on a rebuild: the materialisation window covers the viewport plus a screen either side, matching `History::outstanding`'s ask policy).
- [x] Measured and written in the ticket (release, `examples/budgets`, 3 runs): git/git cg and no-cg peak RSS (was 219–226 MiB), `G` time; 1M cg peak RSS and odb objects read during `G` (expected: only the chunks shown plus `merge_reads`, which must not grow with the fixture: compare git/git's and 1M's `merge_reads` for the same viewport and say so).
- [x] MAP.md and docs/design/03-graph-layout-algorithm.md §3.6 updated.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier once, alone, at the end.

## Size
One context, under ~80k tokens. Seam: (a) pass-through + fingerprint + rebuild with tests; (b) measurements + DESIGN.md. Stop at the seam with a handoff note rather than compacting.

## Handoff (2026-09-14, builder-opus, seam (a) done)

Done (seam a):
- `layout::cache`: fingerprint over `&[RowShape]` (`fingerprint`, running per growing frontier chunk); every chunk laid out keeps `LaidOut { print, materialised }`; eviction drops rows only; `check` / `admit` split; `renote` restores a chunk cut back by `truncate`.
- `StreamingLayout::advance`: materialises a frontier batch only if it starts a chunk meeting viewport ± `buffer_rows`, or continues a held chunk; otherwise passes through.
- `rows` → `rebuild(wanted chunks)`: one replay from the nearest checkpoint, every replayed chunk fingerprint-checked, adjacent wanted chunks materialised as one batch (`admit_run`).
- `Resident.passed_through`, `Work.links_read`; `RowShape: Hash`. MAP.md + docs/design/03-graph-layout-algorithm.md §3.6 paragraph.
- Tests: determinism on the 5k pair vs materialise-everything and `layout()` (odd budget 37, partial chunks pass through); cold jump hydrated ≤ 2×chunk+buffer; deep read merge_reads ≤ merge lanes open at the row. Corpus test asserts pass-through then 0 after the scroll.

Existing tests changed (premise removed by this ticket, not regressions):
- `stream::tests` Forgetful: rewrote commit messages on re-read; a shape fingerprint cannot see a message. Now re-served links drop parents.
- `inclusion::tests::folding_invalidates_only_the_rows_from_the_merge_down`: viewport set over all 42 rows so "nothing kept was rebuilt" still isolates the reinclude.
- `tests/it/streaming.rs` linear: `hydrated == pulled + reread` → `hydrated <` that, `passed_through == 0` after scroll; body moved to `assert_linear_work` (clippy too_many_lines).

Spec conflict to resolve: acceptance wants `merge_reads == 0` on cold G with the viewport at the top, but frontier chunks are materialised one pull at a time, and the existing test `a_merge_lane_open_across_a_chunk_boundary_is_named_with_one_read` (must stay unchanged) expects 1 read across a frontier chunk boundary. So a merge lane crossing chunk 0→1 or 1→2 costs one read. Not changed; measure and decide (carrying names across contiguous frontier batches would make it 0 and break that test).
**Resolved 2026-09-14 (planner-fable): the test was right, the `== 0` bound was wrong. See the amended Acceptance bullet: per-boundary lane bound plus "pass-through adds zero" equality check; the test stays unchanged.**

Next (seam b), exact steps:
1. `crates/graft/tests/it/budget_run.rs` `Numbers` has no `Work`; `serve` never exposes the layout. To report hydrated/merge_reads/links_read for `G`, extend `order_bytes`-style direct drive (it repeats `Views::new` setup) or add a counter at `WalkSource::read` in `crates/graft/src/history/source.rs`.
2. Release, alone, 3 runs each: `cargo run --release -p graft --example budgets -- ~/.cache/graft/git-git.git` (cg and no-cg: check how 46/51 toggled the commit-graph — see `scripts/memory-curve.sh`); 1M cg via `scripts/memory-curve.sh 1000000` (builds a ~350 MiB fixture under target/fixtures, heavy).
3. Record peak RSS (was 219–226 MiB git/git), G time, 1M cg RSS + objects read during G into this ticket. Batch `WalkSource::read` only if per-handle reads show in G.
4. `frames::` rows-on-hand check in release (`crates/graft/tests/it/frames.rs`, heavy tier) and the heavy tier once at the end.

## Result, seam (b) (2026-09-14, builder-opus)

Tests added:
- `stream::tests::passing_through_the_rest_of_history_reads_nothing` (5k pair, `small()`): layout advanced to the end vs one advanced only through the window (chunks 0-2): `hydrated` and `merge_reads` equal; `merge_reads` <= sum of merge lanes open at the first row of chunks 1 and 2 (3 <= 3, 9 <= 9). Chunk-boundary test untouched.
- `heavy::a_cold_jump_on_the_100k_fixture_reads_only_the_top` (100k cg): `hydrated <= 2 x chunk_rows + buffer_rows`, `links_read >= rows`, `merge_reads <=` per-chunk lane bound. Green in release.
- `budget_run::cold_jump` (drives the layout directly, shared `direct_layout` setup with `order_bytes`); `examples/budgets` prints its line.

Measured: release, `examples/budgets`, each run alone, 3 runs. RSS = getrusage peak of the open -> paint -> G -> g run. Default `StreamConfig` (chunk 500, buffer 1000), viewport 0..40.

| Repo | peak RSS | was | G | was | hydrated | merge_reads (bound) | links_read |
|---|---|---|---|---|---|---|---|
| git/git cg (85,649) | 45.6-46.6 MiB | 223.6-225.3 | 138-144 ms | 0.53-0.54 s | 1500 | 288 (288) | 85,649 |
| git/git no cg (`target/27b/nocg.git`, regen: `docs/perf/27b.txt` § `target/27b/nocg.git`) | 97.2-98.3 MiB | 219.1-221.5 | 792-815 ms | 0.85-0.86 s | 1500 | 288 (288) | 85,649 |
| 1M cg synthetic | 238.2-240.9 MiB | — | 507-514 ms | — | 1500 | 1 (1) | 1,000,000 |

- Commits read from the odb during a cold G = `hydrated + merge_reads`: 1,788 on git/git, 1,501 at 1M. Neither grows with history: `hydrated` is the window (identical at 85k and 1M), `merge_reads` equals the merge lanes open at the window's chunk starts (git/git's top is merge-heavy, 288; the synthetic fixture's top has one). It follows the topology at the top, not the fixture size.
- `WalkSource::read` not batched: 1,788 reads inside a 138 ms G on git/git; nothing points at per-handle reads. Not profiled.
- 1M cg peak RSS 241 MiB, under the 300 MB gate. First paint at 1M is 263 ms with 219 MiB already held at paint: that is before G, not this ticket's path; ticket 46's open item.
- Heavy tier (`cargo test --release -p graft --features heavy-tests`): 29/29, `frames::` rows-on-hand checks included.
