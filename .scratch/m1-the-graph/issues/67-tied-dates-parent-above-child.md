# m1-the-graph/67 — Review M4a: with tied commit dates and no commit-graph, a parent draws above its child

Status: done

## Notes (builder, 2026-09-15)
- **Cause fix, and tie-breaking alone would not have been enough:** the streaming date walk emitted a parent before an unseen child at the same date. `Dated::next` now takes the whole equal-date class (reading down through same-dated parents) and emits it in Kahn order, smaller id first. On distinct dates the class is one commit, so the walk behaves as before.
- Promotion (`full_topological`, Kahn over the range) was already right; the test guards it.
- 1M no-cg, release, fixture `target/fixtures/synthetic-1000000-no-commit-graph-8ac5be635bb9d262`: before, warm first paint 102.8 ms and G 5.27–5.50 s (2 runs); after, warm first paint 104–125 ms and G 5.31–5.47 s (5 runs). The cold first run is ~1 s both times. Raw output: `target/m1-67/before.txt`, `after.txt`.
- Cost: a repo whose commits all share one date reads the whole tie class before first paint. A backwards clock can still misorder the provisional view.
Build with: **Opus** (high) — the walk's topological order; the order tests are the guard.
Blocked by: none

From the M1 review round 1, "M4" tied-dates bullet. Closes KNOWN_GAPS.md row ~190.

## Read first
- docs/reviews/m1-the-graph.md "M4" only; KNOWN_GAPS.md the tied-dates row.
- `crates/graft-git/MAP.md`; send a scout for the no-commit-graph date walk and the topological sort in `crates/graft-git/src/history/`.
- ADR-0026 "Decision (design)" (the walk must stay one table, bounded by the viewport).

## Scope
In: a failing test on a fixture where every commit shares one timestamp, with no commit-graph and branches plus merges. It asserts every child is emitted before each of its parents in the first chunk *and* after promotion. Then a cause fix inside ADR-0026: tie-breaking alone isn't enough if the streaming date walk can emit a parent before an unseen child, so say which the fix is. Keep 1M no-cg first paint and `G` time within 10% of 56c's numbers (`docs/shots/m1-memory-curve.txt`), measured once in release. Close the row.
Out: the commit-graph path (generation numbers already order it); any bar.

## Acceptance
- [x] Test red before, green after; existing order tests untouched and green.
- [x] 1M no-cg first paint and G time reported before/after.
- [x] KNOWN_GAPS row closed. Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
