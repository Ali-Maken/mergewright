# m1-the-graph/30 — Frame budgets, benches, 50k refs, and the 1M claim

Status: ready
Build with: **Opus** (high) — timing the event loop and the pixel path
Blocked by: 11, 25, 27
Leaves runnable: yes — benches run at review; input-to-frame asserted in the heavy tier.

Why: the rest of the retired performance ticket (16, text in 35bec3e). Split from 27 so each fits one context.

## Read first
- PROMPT.md §1.2 (the 60 fps bullet), §6.3 "Performance budgets" and the 50k-refs bullet, §7.1 "Benchmarks" bullet
- DESIGN.md §3.8 (the 16 ms frame budget paragraphs only)
- KNOWN_GAPS.md rows "50k+ refs in the sidebar and ref badges (review N4)", "Resize frame busts the frame budget"
- 25's measured resize numbers; 27's results table
- Code: `MAP.md` of `graft-graph`, `graft-ui`, `graft`

## Scope
In:
- `criterion` benches: `StreamingLayout::advance` per chunk, Unicode row render, strip rasterise, strip cache hit/miss. Review-only runs (`--warm-up-time 1 --measurement-time 3`).
- Heavy tier on the 100k fixture: input-to-frame < 16 ms at the Unicode and pixel tiers; sustained scroll at 60 fps.
- 50k refs: generate them in the heavy tier, measure badge lookup and the row model, meet the budget or record what breaks.
- The 1M+ claim: fixture is 100k. Measure a real 1M repo once by hand, or state in KNOWN_GAPS.md that 1M is extrapolated. **Never report a 1M number not observed.**

Out:
- Memory and first paint (27). Search timing (26). CI wiring.

## Signatures
Expose: none public. Benches in `benches/` of the crate that owns each hot path.
Call: `StreamingLayout::advance(budget, stop)`, `rows(Range<usize>)`; 22's rasteriser entry point; 23's cache lookup; 11's badge lookup; `graft_fixtures::synthetic::borrow(Scale::HEAVY)`.

## Acceptance
- [ ] Results table in this ticket: budget · measured · tier · fixture · machine.
- [ ] Each heavy assertion proven to fail by tightening its threshold, then reverted.
- [ ] 50k-refs result recorded; N4 row closed or re-owned with a reason.
- [ ] 1M claim measured or explicitly marked extrapolated.
- [ ] Fast tier still under 3 minutes; benches not in it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; heavy tier and benches once, alone, at the end

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
