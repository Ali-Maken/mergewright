# m1-the-graph/30e — Eviction floors to row 0 when no checkpoint precedes the viewport

Status: ready
Build with: **Opus** (high) — streaming layout memory under background history (split from 30c part b seam 2)
Blocked by: none (main is red at 5c4594e: this ticket turns it green)
Leaves runnable: yes — gates green; keys never wait on the worker and resident rows stay bounded.

Why: 30c part b seam 2 (commit 5c4594e, WIP) made keys draw from rows on hand and ask for 3 screens only when 2 screens either side aren't held (`History::outstanding` stops re-asks). That fixed the cause of keys waiting (user decision in 30c). The wider ranges exposed a real bug in graft-graph: `StreamingLayout::evict` floors to row 0 when no kept checkpoint precedes the viewport, so nothing is evicted. **A memory regression; must not ship.**

State at 5c4594e: lib 272/273; red test `scrolling_top_to_bottom_keeps_resident_rows_bounded`; its bound was **loosened** in the WIP commit and still fails — **restore the original bound**, don't keep the loosened one. 2 clippy warnings.

## Read first
- Ticket 30c: section "Decision (key 37 / release assert flake)" and the part b handoff notes
- ADR-0008; ticket 27c/27d notes (promotion, checkpoints, memory guard)
- `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source (`StreamingLayout::evict`, checkpoints; `graft/src/history/scroll.rs`)

## Scope
In:
- Restore the test's original bound (red). Test at the graft-graph layer too: evict with no kept checkpoint before the viewport → rows before the viewport are released (red before, green after).
- Fix the cause in `evict` (e.g. a checkpoint policy that always keeps one at or before any viewport that can be evicted to); no raised bound, no cap tweak.
- Fix the 2 clippy warnings.
- Gates green. MAP.md if a public item changes.
- Report: cause or symptom, and what it costs the user.

Out: frames.rs release runs, idle-wake report, proofs, docs (→ 30f).

## Acceptance
- [ ] Original bound restored and green.
- [ ] graft-graph eviction test red → green.
- [ ] Gates pass: fmt, clippy (both feature sets), test.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
