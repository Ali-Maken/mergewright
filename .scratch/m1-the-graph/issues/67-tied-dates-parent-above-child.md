# m1-the-graph/67 — Review M4a: with tied commit dates and no commit-graph, a parent draws above its child

Status: ready
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
- [ ] Test red before, green after; existing order tests untouched and green.
- [ ] 1M no-cg first paint and G time reported before/after.
- [ ] KNOWN_GAPS row closed. Report: cause or symptom; what it costs the user.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Near it, stop at a clean seam with a handoff note.
