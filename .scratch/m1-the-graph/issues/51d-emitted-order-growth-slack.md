# m1-the-graph/51d — EmittedOrder growth slack: under 10 B/commit at 100k

Status: done
Build with: **Sonnet** (medium) — remainder of 51 (conductor split); no signature change. Escalates to Opus on a second failure.
Needs: `EmittedOrder` (51 seam a), 51c's heavy test and `examples/budgets` print.
Blocked by: 51c

Why: 51c measured `EmittedOrder::approximate_bytes` at 10.37 B/commit (100k) and 10.70 (1M no-cg) against a bar of ≤ 10. Cause named there: the vectors grow by push doubling, so `capacity()` carries ~30% slack over the ~8 B/commit layout. The bar stays; the slack goes. The heavy test `emitted_order_bytes_stay_under_ten_per_commit_at_100k` is red until this lands.

## Read first
- `51c-emitted-order-bytes-checked.md` (numbers and cause).
- `crates/graft-graph/MAP.md`; ticket 50's notes on paged arrays (`50-walk-reads-the-commit-graph.md`, its measurements/deviation notes only) — the same technique already in graft-git.

## Scope
In: bound the growth slack in `EmittedOrder`'s storage (graft-graph `layout/order.rs`, and `layout/stream.rs` if it owns the vectors). Pick the approach that bounds slack while the order is still streaming (a load can stop part way, so a one-off `shrink_to_fit` at the end alone isn't enough): fixed-size pages as ticket 50 did, or reserving from a known commit count where one exists, plus shrink at done. Keep public signatures; `approximate_bytes` keeps counting capacity (that's the honest number).
Out: changing the bar, changing what `approximate_bytes` measures, any other structure.

## Acceptance
- [x] 51c's heavy test passes unchanged: ≤ 10 B/commit at 100k. **9 B/commit** (800,072 B total, was 10.37).
- [x] 1M no-cg release `examples/budgets` by hand: **9 B/commit** (8,000,072 B total), peak RSS **481.2 MiB** (was 10.70 B/commit, 482.4 MiB — RSS effectively unchanged, bytes fixed).
- [x] graft-graph property tests and corpus snapshots byte-for-byte unchanged (`cargo test`: 0 failed across the workspace).
- [x] 51's last box ticked; 51c ticked.
- [x] Gates clean, both clippy invocations.

## Fix (cause, not symptom)

Cause named in 51c: `rows: Vec<CommitHandle>` and `row_by_handle: Vec<u32>` grew one
`push`/`resize` at a time, so `Vec`'s amortized doubling left ~30% capacity slack over
`len()` by 100k–1M commits — `approximate_bytes` honestly counts that slack, so it read
high even though nothing extra was stored.

Fix: `EmittedOrder::reserve(additional_rows)` (`layout/order.rs`, `pub(crate)`) reserves
exact capacity for a batch about to be appended. `LayoutCursor::extend` (`layout.rs`)
calls it once per batch (`order.reserve(commits.len())`) before laying the batch out, so
streaming growth pays for exactly the rows about to land instead of `Vec::push`'s
doubling — bounded to at most one batch (~500 rows by default) of slack at any time,
not a fraction of the whole history. `from_handles` (the full-order/promoted path) gets
the equivalent one-shot `row_by_handle.reserve_exact(rows.len())` up front, since it
already knows the final row count.

No pre-sizing from `expected_rows()` and no page-based storage: `rows`/`row_by_handle`
stay plain, contiguous `Vec`s (keeping `handles()`'s `&[CommitHandle]` signature
untouched), and no capacity is reserved further ahead than the load has actually
committed to — a load stopped partway never over-reserves. No structural change: public
signatures unchanged, `approximate_bytes` still counts capacity.

Costs the user: nothing observable — same API, same memory-safety, RSS at 1M within
noise of before (481.2 vs 482.4 MiB); the fix only removes counted-but-unused capacity.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Small. Heavy/release runs one at a time, in one foreground Bash call with a long timeout; don't end the turn while one runs.
