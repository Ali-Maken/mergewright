# m1-the-graph/51d — EmittedOrder growth slack: under 10 B/commit at 100k

Status: ready
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
- [ ] 51c's heavy test passes unchanged: ≤ 10 B/commit at 100k. Number recorded here.
- [ ] 1M no-cg release `examples/budgets` by hand: B/commit and peak RSS recorded here (51c's run: 10.70, 482.4 MiB).
- [ ] graft-graph property tests and corpus snapshots byte-for-byte unchanged.
- [ ] 51's last box ticked; 51c ticked.
- [ ] Gates clean, both clippy invocations.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Small. Heavy/release runs one at a time, in one foreground Bash call with a long timeout; don't end the turn while one runs.
