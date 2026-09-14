# m1-the-graph/60 — Checkpoints thin to an even grid instead of LRU eviction

Status: ready
Build with: **Sonnet** (medium) — a cache policy with one exact invariant and tests that state it; the approach is decided below; no interface change.
Needs: none (independent of the memory tickets; `budgets`' existing `far_jump` reports the 1M number).
Blocked by: none

## The finding

`StreamingLayout` keeps at most `max_checkpoints` (64) `LayoutCursor` clones, one every `checkpoint_every` (2,000) rows, evicting least-recently-used. At 1M rows only the last 128k laid out keep checkpoints; a `Find` to row 500,000 after a cold `G` replays **500,500 rows, 1.39–1.46 s** (56, both cg and no-cg); row 900,000 replays 500 rows, 2 ms. Below 128k rows (git/git, the 100k fixture, every daily repo) nothing is ever evicted, so nothing changes there.

## Decision

**Thin to a grid; never LRU.** A `stride` starts at `checkpoint_every`. A checkpoint is recorded when the frontier or a rebuild passes row `r` with `r % stride == 0` (and not at the very end of history, as today). When recording would exceed `max_checkpoints`, `stride` doubles and every checkpoint whose row is not a multiple of the new stride is dropped; then record `r` if it is on the new grid. Stride never shrinks within a layout's life (`truncate` may leave it sparser than needed; acceptable, documented). A rebuild from the checkpoint at or before the target re-records grid rows it passes that are missing.

At 1M rows: 2k → 4k → 8k → 16k (64 × 16k ≥ 1M), so any jump replays ≤ 16k links ≈ 45 ms at 56's 2.9 µs/row, on the worker, not the UI thread. Memory unchanged (≤ 64 × ~1.6 KiB).

Considered and not taken: a two-tier policy (a thinned global grid plus a dense LRU band near the viewport). It would make re-visits near the last jump replay ≤ 2k rows instead of ≤ 16k, but scrolling upward away from the resident set replays from the grid either way, and 45 ms once per 16k rows scrolled at 1M is under what the user sees today by two orders of magnitude. Revisit only if the 1M scroll numbers in the M1 perf run say so.

**Invariant (test it):** at every moment, `checkpoint_rows()` is exactly the multiples of `checkpoint_stride()` in `(0, frontier)` minus the end-of-history exclusion, and `len ≤ max_checkpoints`.

## Read first
- DESIGN.md §3.6 lines 849–864 (checkpoints: what they are, LRU today) and 865–870 (a cold deep jump is a background job)
- `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md` line 76 (the measurement and the seam) and "Spec-only pass 2" (finding 3)
- KNOWN_GAPS.md row "Least-recently-used is the wrong policy once checkpoints pass `max_checkpoints`" (line ~225)
- `crates/graft-graph/MAP.md` `layout::stream` (`StreamConfig`, `Resident`, `Work`, `StreamingLayout::{rows, checkpoint_rows, truncate, work}`) and `layout::cache`
- `crates/graft/MAP.md` `examples/budgets` line (`far_jump`)

## Scope
In:
- The policy above in `layout::cache` / `StreamingLayout` (graft-graph). `StreamConfig` unchanged: `max_checkpoints` is the grid's cap, `checkpoint_every` the starting stride.
- `StreamingLayout::checkpoint_stride()` for tests and `budgets`.
- Fast-tier tests with a small config (e.g. `checkpoint_every = 10`, `max_checkpoints = 8`, a linear source of 1,000 links): the invariant after every advance; stride ends at 160; `rows(500..510)` with everything evicted replays ≤ stride + one chunk (from `Work`); `truncate` keeps the invariant; the existing rebuild/fingerprint tests untouched.
- `budgets` prints `checkpoint stride` with its far-jump lines.
- DESIGN.md §3.6 lines 861–863: replace "least recently used evicted; a rebuild that needed an evicted one replays from the one before and re-records it on the way" with one sentence on the grid (stride doubles when full, spacing stays even across all of history; at 1M rows 16k apart). Nothing else in DESIGN.md.
- MAP.md lines (`layout::cache`, `checkpoint_stride`).

Out: `resident_chunks`/`buffer_rows` policy, the cg/no-cg question, background advance (KNOWN_GAPS row stays), any two-tier band.

## Signatures
```rust
// graft-graph layout::stream
impl<S: CommitSource> StreamingLayout<S> {
    /// Rows between kept checkpoints: `checkpoint_every`, doubled each time the cap was reached.
    pub fn checkpoint_stride(&self) -> usize;
}
```
Existing, unchanged: `StreamConfig`, `checkpoint_rows()`, `rows()`, `truncate()`, `work()`. Calls from `examples/budgets`: `checkpoint_stride`, `work().replayed`.

## Acceptance
- [x] Invariant tests green in the fast tier; `cargo test -p graft-graph` under 5 s extra.
- [x] Release, 1M cg fixture, `budgets` far jumps after a cold `G`: rows/2 replays ≤ 20,000 rows and ≤ 100 ms; 9·rows/10 ≤ 20,000 rows; `g` after `G` ≤ 5 ms; stride printed = 16,384 (2k × 2³) or the value the arithmetic gives for the fixture's row count. Same on 1M no-cg. Numbers in the ticket; raw under `target/<ticket>/`.
- [x] 100k fixture and git/git: `checkpoint_stride() == 2,000`, jump numbers unchanged from 56/27 (report one each).
- [x] DESIGN.md sentence replaced; KNOWN_GAPS row "Least-recently-used…" closed with the before/after numbers (56 carries the rest of the docs).
- [x] Report: cause (the policy), cost to the user (≤ ~45 ms per far jump at 1M, on the worker).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; the 1M `budgets` runs once each, release.

## Size
One context, well under ~80k: one policy, one accessor, four tests, two `budgets` runs.

## Result

Grid-thinning policy replaces LRU in `crates/graft-graph/src/layout/checkpoint.rs` (full
rewrite of `Checkpoints`). `StreamingLayout::checkpoint_stride()` added and wired into
`examples/budgets`' far-jump lines. Four new invariant tests in `layout/stream/tests.rs`.

Release measurements (synthetic fixtures, all pre-cached under `target/fixtures/`):

| Fixture | Jump | Rows replayed | Time | Stride |
|---|---|---|---|---|
| 1M cg | rows/2 (500,000) | 4,500 | 13–15 ms | 16,000 |
| 1M cg | 9·rows/10 (900,000) | 4,500 | 13–15 ms | 16,000 |
| 1M no-cg | rows/2 | 4,500 | 13–15 ms | 16,000 |
| 1M no-cg | 9·rows/10 | 4,500 | 13–15 ms | 16,000 |
| 100k cg | both | 500 | ~1.8 ms | 2,000 |

`g` after `G` on the 1M fixtures: 615–649 µs (bar ≤5 ms). All bars met with large margin
(bound was ≤20,000 rows / ≤100 ms; actual 4,500 rows / 13–15 ms).

Stride value: ticket text says "16,384 (2k × 2³)" but 2,000 × 2³ = 16,000, not 16,384
(16,384 = 2¹⁴ only applies from a base of 2,048). The ticket hedges with "or the value the
arithmetic gives for the fixture's row count" — 16,000 is that value and meets the bar.

git/git: not available in this environment (`scripts/clone-git-git.sh` is deliberately
manual/network-dependent, "no test calls it, and none ever should" — no local clone
present, none fetched). The 100k synthetic fixture's exact match (`checkpoint_stride() ==
2,000`) plus ticket 56/27's prior git/git measurements (85,649 commits, never reaches the
128k-row threshold where either policy would ever thin, `g` after `G` ≈ 0.1 ms) stand as
the evidence for this line; nothing about the policy change can affect a repo below the
threshold.

Before/after (KNOWN_GAPS, 1M cg, jump to row 500,000): before (m1/56) 1.39–1.46 s / 500,500
rows replayed; after (m1/60) 13–15 ms / 4,500 rows replayed.

**Cause fix**, not symptom: replaces the eviction policy itself (LRU → even grid). Cost to
the user: worst case ≤16,000 rows replayed per far jump at 1M-row scale (~45 ms estimated
in the ticket's own arithmetic; measured 13–15 ms), on the worker thread, not the UI thread.
No behavior change below 128k rows (100k fixture, git/git, every daily repo unaffected).

Two incidental test-code fixes while writing the invariant helper (both cause fixes, no
cost to the user — test code only): the frontier-row boundary needed to include the row
just reached by `advance()` (not exclude it) since a checkpoint is legitimately recorded
there before end-of-history is known; and a `(1..).map().take_while().filter()` chain
triggered clippy's `maybe_infinite_iter` lint, resolved by computing the expected checkpoint
list with a direct bounded range instead.
