# m1-the-graph/68 — Review M4b: an incoming edge names a column not in the row's lanes (dangling ─)

Status: ready
Build with: **Opus** (high) — the layout engine's invariant.
Blocked by: none

From the M1 review round 1, "M4" dangling-edge bullet. Closes KNOWN_GAPS.md row ~189.

## Read first
- docs/reviews/m1-the-graph.md "M4" only; KNOWN_GAPS.md the dangling-edge row.
- `crates/graft-graph/MAP.md`; scout for `layout` (`stream.rs`, `fold`) where `incoming` and `lanes` are assigned.

## Scope
In: a property test (the crate's existing proptest setup) asserting that every `incoming` column is in the row's `lanes` for every row, across the corpus and fold/first-parent/lane-cap modes. Find a minimal failing case, record it as a regular test, then fix the cause in layout. No renderer-side filtering, which would be a symptom fix. Close the row. If the fix changes rendered output, update the affected snapshots and list them; the user judges them by eye.
Out: renderer changes; file splits.

## Acceptance
- [ ] Property test and minimal regression test red before, green after.
- [ ] Changed snapshots listed by path.
- [ ] KNOWN_GAPS row closed. Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
