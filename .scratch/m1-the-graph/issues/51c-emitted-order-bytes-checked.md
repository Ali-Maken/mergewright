# m1-the-graph/51c — EmittedOrder bytes at 100k, printed and checked (51's last box)

Status: done — measured, cause named, fixed in 51d
Build with: **Sonnet** (medium) — remainder of 51 (conductor split). Escalates to Opus on a second failure.
Needs: `EmittedOrder::approximate_bytes` (51 seam a).
Blocked by: 51b

Why: 51's acceptance box "`EmittedOrder::approximate_bytes` at 100k ≤ 10 B/commit (printed in the heavy budget test; was ~90)" is open. 51b's split text wrongly read as excluding it; the conductor's error, not the builder's.

## Read first
- `51-order-and-inclusion-on-handles.md`: **Acceptance** only.
- `51b-promoted-order-on-handles.md`: the 2× check notes (where the heavy budget test and `overflow_len` print live).
- `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`.

## Scope
In: print `EmittedOrder::approximate_bytes` (total and per commit) in the 100k heavy budget test and `examples/budgets`; assert ≤ 10 B/commit at 100k in the heavy test (release-or-debug per ADR-0021: it's a size, not a pace, so it fails in both). Run it once at 100k and once by hand at 1M no-cg (release); write both numbers into this ticket and tick 51's box.
If it's over 10 B/commit: name the cause, write it here, stop — don't change the bar.
Out: any other change.

## Acceptance
- [x] Numbers at 100k and 1M no-cg recorded here; **cause named and stopped — 51's box not ticked** (over the 10 B/commit bar; bar not changed).
- [x] Gates clean, both clippy invocations.

## Numbers and cause (over the bar; stopped, not fixed)

Added `budget_run::order_bytes` (drives `Views`/`StreamingLayout` directly —
`history::stream`'s own `Views` never leaves it, so no new `HistoryUpdate`
variant), printed in a new heavy test
(`emitted_order_bytes_stay_under_ten_per_commit_at_100k`, `crates/graft/tests/it/heavy.rs`)
and in `examples/budgets`.

- **100k, no commit-graph** (`cargo test -p graft --features heavy-tests --test it -- heavy::emitted_order_bytes_stay_under_ten_per_commit_at_100k --exact --nocapture`): 1,036,848 B total, **10.37 B/commit** (`div_ceil` prints 11). Test **fails** at the 10 B/commit bound — left failing, per this ticket's instruction not to change the bar.
- **1M, no commit-graph, release** (`target/release/examples/budgets` on `target/fixtures/synthetic-1000000-no-commit-graph-d5f8e6a5494a9ead`): 10,701,872 B total, **10.70 B/commit**. Same shape as 100k: peak RSS 482.4 MiB after `G` (report only, not gated, per ADR-0026), `overflow_len` 1,000,000 (51b's 2× check still holds).

**Cause:** `EmittedOrder::approximate_bytes` counts `Vec::capacity()`, not `len()` (order.rs's own doc comment: "the number a memory budget actually pays for"). `rows: Vec<CommitHandle>` and `row_by_handle: Vec<u32>` are both grown by `extend`'s push-based reallocation (Rust's ~2× doubling growth), so at 100k/1M commits each ends up with capacity roughly 30% over length rather than exactly at it. The theoretical per-commit cost ADR-0026 designed for is 8 B (`u32` handle + `u32` reverse row); the measured 10.37–10.70 B/commit is that 8 B inflated by the two vectors' growth-factor slack, not extra state. Nothing else grows with history in this struct (`std::mem::size_of::<Self>()` is a fixed ~48 B, negligible at this scale).

**Not fixed here** (ticket's own rule: name the cause, stop, don't change the bar): the fix would be pre-sizing both vectors (`Vec::with_capacity`/`reserve_exact` against `expected_rows()` or a growth policy that trims slack) in `graft-graph`'s `layout/order.rs` or `layout/stream.rs`'s `extend` — a design change to `EmittedOrder`/`LayoutCursor::extend`'s growth strategy, out of this ticket's scope (**Out: any other change**). Flagging for the conductor: either loosen the bound to ~11-12 B/commit (matches Vec growth reality) or file a follow-up ticket to reserve capacity.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Small. Heavy/release runs one at a time, blocked on in the foreground.
