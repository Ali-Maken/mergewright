# m1-the-graph/51c — EmittedOrder bytes at 100k, printed and checked (51's last box)

Status: ready
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
- [ ] Numbers at 100k and 1M no-cg recorded here; 51's box ticked (or cause named and stopped).
- [ ] Gates clean, both clippy invocations.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Small. Heavy/release runs one at a time, blocked on in the foreground.
