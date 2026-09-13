# m1-the-graph/30c — Unicode chunk-edge frame spikes; frame asserts pass and are proven; benches run

Status: ready
Build with: **Opus** (high) — event loop timing, split from 30
Blocked by: 30b
Leaves runnable: yes — heavy `frames::` tests pass and each is proven to fail when tightened.

Why: 30 measured Unicode input-to-frame spikes of 32–41 ms (debug) at layout-chunk edges; release 2–12 of 420 keys over 16 ms, max 17–23 ms.

## Read first
- Ticket 30 "Handoff" items 2–4 and NOTES (the debug vs release question); results table
- DESIGN.md §3.6 (background advance paragraph); ADR-0008
- `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
1. Decide with the benches (`StreamingLayout::advance/one chunk`): background advance ahead of the viewport vs smaller `chunk_rows`. Numbers first; fix; re-measure.
2. **Debug vs release for frame asserts:** the conductor's decision goes in here before this ticket starts (see "Decision" below). Implement it (TESTING.md "The heavy tier" command updated to match).
3. Threshold-tightening proof for each `frames::` assertion, reverted.
4. Run the benches once (`cargo bench -p graft-graph -p graft-ui -- --warm-up-time 1 --measurement-time 3`), numbers into 30's table.

Out: pixel byte cost (30b), 50k refs real read (30d), 1M claim (extrapolated, in KNOWN_GAPS).

## Decision (debug vs release)
Pending the user (asked 2026-09-13). Recommendation: assert frame budgets only under `--release`; debug heavy tier still runs them as measurements without asserting.

Also covers 30d's remainder: first paint with 50k refs passes in release (57 ms) but is 494 ms in debug (three passes over 50k refs: resolve 195, references 85, tips 113 ms). If the user keeps debug asserts, that needs a shared ref read (new graft-git signatures) or an optimized-deps dev profile.

**Until the user answers:** do scope items 1 and 4 (spike fix, benches), stop at seam (a) with a handoff note. Don't implement item 2 or the proofs that depend on it.

## Acceptance
- [ ] Spike cause named with numbers; fix measured before/after.
- [ ] All four `frames::` tests pass in the chosen build mode; each proven by tightening.
- [ ] Bench numbers recorded.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy and benches once, alone.

## Size
Under ~80k tokens. Seam: (a) spike fix + decision, (b) proofs + benches.
