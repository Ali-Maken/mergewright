# m1-the-graph/14 — Compaction: `+N`

Status: ready
Build with: **Fable** (high) — algorithm: ranking and folding lanes under invariants
Blocked by: 09
Leaves runnable: yes — busy histories fit 80 columns.

Why: a graph twelve lanes wide on an 80-column terminal fails the checkpoint question however correct the lanes are.

## Read first
- PROMPT.md §1.2 ("Compact output" only)
- DESIGN.md §3.5, §3.7, and §1.3 (the 80×24 width it has to survive)
- ADR-0017 (proptest case counts)
- Code: public API of `crates/graft-graph/src/layout.rs`; `layout/tests/properties.rs` (the proptest pattern); 08's widget

## Scope
In:
- Triggered when open non-reserved lanes exceed `graph.max_lanes` (default 8; config key under M0's no-silent-default rules).
- Fold least-recently-touched first. Never fold lane 0, a reserved integration lane, or any lane in a caller-supplied protected set (16 supplies the selected commit's ancestry; take it as a parameter now).
- A render-time decision over `Row` data that keeps every real column: expanding never re-runs layout.
- A rendering-agnostic description of the fold; the Unicode widget draws the `+N` column from it. One key expands/collapses.

Out:
- Folding merged branches and first-parent view (15). `+N` in ASCII (19) and pixels (22).

## Acceptance
- [ ] A fixture with more lanes than `max_lanes` renders within 80 columns, and the folded set is the least-recently-touched one.
- [ ] Property test: lane 0, reserved lanes and the protected set are never folded.
- [ ] Property test: expanding restores exactly the pre-fold columns of affected rows and changes no other row.
- [ ] Toggling fold makes zero layout calls (asserted).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
