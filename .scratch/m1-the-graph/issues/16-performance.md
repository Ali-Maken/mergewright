# m1-the-graph/16 — Meeting the performance budgets

Status: ready
Build with: Opus
Blocked by: 02, 11, 14, 15

Why: PROMPT.md §1.4's numbers are the product, not a nice-to-have — "feels instant" is the whole premise. This ticket turns them from prose into measurements that fail the build when they regress.

## Read first
- PROMPT.md §1.4 (all four budgets) and §7.1 (heavy tier: 300 MB, 100k fixture, timing)
- DESIGN.md §3.6 (checkpoints), §3.8 (strip cache)
- ADR-0008 (first paint depends on the commit-graph file)
- KNOWN_GAPS.md rows "Resize frame busts the frame budget" and N4 (50k+ refs)

## Goal
Every M1 budget is measured on a real corpus, with numbers written down, and the ones that are not met are stated plainly rather than rounded away.

## Scope
In:
- criterion benches for the hot paths: `layout_chunk`, row rendering per tier, strip rasterisation, strip cache hit/miss. Short runs — the fast tier must stay under 3 minutes, so benches are not part of it.
- Heavy-tier assertions on the 100k fixture (ticket 02's harness):
  - first paint ≤150 ms, measured **both** with a commit-graph file present and with it absent (ADR-0008 says the strategy differs; both paths need a number).
  - scroll sustains 60 fps; input-to-frame <16 ms at the Unicode tier and at the pixel tier.
  - peak RSS <300 MB via getrusage, including a jump to the bottom of history and back — the case that would leak the whole walk if ticket 06's R2-1 fix were wrong.
- The 50k-refs case (gap N4): generate the refs, measure badge lookup and the row model, and either meet the budget or record what breaks.
- Honesty about scale: PROMPT.md claims 1M+ commits; the fixture is 100k. Either measure a real 1M repo once by hand and record it, or state in the report and in KNOWN_GAPS.md that the 1M claim is extrapolated from 100k. **Do not report a 1M number you did not observe.**
- Where a budget is missed, fix it if the fix is in M1's scope; otherwise record it with the measurement, not as a vague "needs optimisation".

Out:
- New features. If a budget can only be met by changing behaviour, stop and ask through the conductor.
- CI wiring (no remote repo yet).

## Acceptance
- [ ] A results table in the ticket: budget, measured value, machine, fixture, with/without commit-graph.
- [ ] Heavy-tier tests fail when a budget is exceeded — verified by temporarily tightening a threshold, not by assuming.
- [ ] The fast tier is still under 3 minutes and ~2 GB after this ticket lands.
- [ ] One heavy job at a time; no fixture generated outside the heavy tier.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
