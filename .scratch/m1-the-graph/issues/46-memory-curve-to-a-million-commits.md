# m1-the-graph/46 — Memory curve to a million commits

Status: ready
Build with: **Opus** (high) — memory under background history loading; heavy tier only
Blocked by: 43
Leaves runnable: yes — one heavy-tier command reports peak RSS against history size.

Why (user, 2026-09-13, for 27/30): "git/git peaks at 219–226 MiB with 85k commits. If memory tracked the viewport that number would be roughly the same at 1M. Report the curve — peak RSS at 10k, 100k and 1M commits — and say what holds memory as history grows: rows, commit order, gix caches or refs. Then say plainly whether the 300 MB budget survives a million-commit branch. If it doesn't, that's a design finding for planner-fable, not a number to adjust."

## Read first
- PROMPT.md §1.2 (budgets), §7.1 (heavy tier)
- Tickets 27, 27b, 27c, 27d result notes (fixes B, C, D; what the git/git numbers measured); ADR-0008
- TESTING.md "The heavy tier"; KNOWN_GAPS git/git RSS headroom row
- `crates/graft-fixtures/MAP.md`, `crates/graft/MAP.md`, `crates/graft-git/MAP.md`; scouts for source

## Scope
In:
- Heavy tier only (`--features heavy-tests`, release build, one heavy job at a time). Generate linear-ish and merge-heavy synthetic histories at **10k, 100k, 1M** commits (reuse graft-fixtures generators; if 1M generation is too slow or large for the laptop, stop and say so with the number — don't shrink silently).
- Measure peak RSS for: open → first paint → full background load → scroll to bottom, per size. Same method as 27d (ru_maxrss; ADR records RSS as the gate).
- Attribute the memory: rows / laid-out lanes, commit order (topological index), gix object and pack caches, refs — by measurement (counters, allocator stats, or A/B with each piece disabled), not by guess. Say how each scales (flat, per-commit bytes).
- Report the curve as a table in the ticket and in TESTING.md next to the heavy-tier budgets.
- Verdict, plainly: does the 300 MB budget survive a 1M-commit branch? If not → stop there: write the finding (what grows, per-commit cost, projected 1M peak) for planner-fable. **Don't change the budget, don't tune a cap to squeeze under it.**
- A heavy-tier guard for the curve's slope if it's cheap; otherwise a KNOWN_GAPS row.

Out: memory fixes (they follow from the verdict, via planner-fable if the design is wrong).

## Acceptance
- [ ] Table: size × history shape → peak RSS, with the phase where it peaks.
- [ ] Attribution by measurement, per component, with scaling.
- [ ] Plain verdict on 300 MB at 1M.
- [ ] If it fails: design finding written for planner-fable, no budget change.
- [ ] Gates pass (fast tier + heavy clippy).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
