# m1-the-graph/68 — Review M4b: an incoming edge names a column not in the row's lanes (dangling ─)

Status: done
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
- [x] Property test and minimal regression test red before, green after.
- [x] Changed snapshots listed by path. (none changed: no corpus shape hit the case)
- [x] KNOWN_GAPS row closed. Report: cause or symptom; what it costs the user.

## Notes (builder)
- Cause: `assign_row` hid an `unreached` reserved lane from `lanes` but still listed it in `incoming` when its tip landed in a lower lane (e.g. `develop` tip == or on `main`). Fix: such a lane closes with no edge and leaves `unreached`.
- The existing test `reserved_integration_lane_never_reused_via_convergence` asserted `incoming == [1]` — the bug itself; now `[]`. The property Replay model tracks unreached reserved columns the same way.
- Invariant checked: `layout/tests/properties.rs` (plain + every inclusion), `fold/tests.rs` (drawn row under lane cap), `tests/it/corpus.rs`.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
