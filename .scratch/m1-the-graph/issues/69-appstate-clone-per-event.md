# m1-the-graph/69 — Review M2: every event clones AppState, 50k-ref badges by value

Status: ready
Build with: **Opus** (high) — profiling attribution plus a state-sharing change on the event path.
Blocked by: none

From the M1 review round 1, "M2". The m1/64 `fifty_thousand_refs_…_unicode` flake is real: max 14.5 ms against 16.

## Read first
- docs/reviews/m1-the-graph.md "M2" only; KNOWN_GAPS.md "30h" row (idle prefetch and the clone suspicion).
- `crates/graft/MAP.md`; `crates/graft/src/update.rs` around 160–180 and 495–520; `crates/graft/src/refs/mod.rs` ~25–45; `crates/graft/tests/it/frames.rs` `check`.

## Scope
In:
1. Profile first: the release 50k-refs Unicode frames run, attributing per-frame time to the `AppState` clone vs badge lookup/render. Numbers into the ticket.
2. Fix what the profile names: likely `Arc<RefBadges>`, and `Arc` for other large immutable sub-states, so the per-event clone is O(1). Everything stays safe code with no `unsafe`.
3. Re-run the release perf run (four `frames::` tests) 3×, alone. Report p99/max per test before and after.
Out: raising any bar, asserting p99 instead of max, retries (all symptom fixes); file splits (70–74).

## Acceptance
- [ ] Profile attribution in the ticket.
- [ ] 50k-refs Unicode max well under 16 ms in all 3 runs; other frames tests not worse.
- [ ] Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
