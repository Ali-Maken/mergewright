# m1-the-graph/30e — Eviction floors to row 0 when no checkpoint precedes the viewport

Status: done
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
- [x] Original bound restored and green.
- [x] graft-graph eviction test red → green.
- [x] Gates pass: fmt, clippy (both feature sets), test.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Done (2026-09-13, builder-opus)

- **Cause fixed** (graft-graph `StreamingLayout::evict`): the protected window reached back to the nearest *kept* checkpoint, or row 0 if none. Kept checkpoints are capped (LRU), so a viewport far behind the frontier had none before it → every laid-out row protected. Now it reaches back to where a checkpoint is *due* (`top − top % checkpoint_every`): never further back than before, bounded by one interval. Rebuilds still replay from any earlier kept checkpoint and re-record the due one. `Checkpoints::floor` removed (no caller; crate-private, MAP unchanged).
- Cost to user before the fix: memory grew with every row scrolled past (no eviction); after: bounded again. Rows scrolled back to far behind may replay from further back (CPU, not memory).
- Red → green: `layout::stream::tests::a_viewport_before_every_kept_checkpoint_still_evicts_the_rows_before_it` (53 chunks resident, bound 11 → passes).
- `scrolling_top_to_bottom_keeps_resident_rows_bounded`: WIP's loosened terms (a second two-screen window, chunk rounding) removed; original rule restored — asked window + checkpoint interval + buffers, in chunks, + resident budget. The window term is **7 screens** (what 30c now asks for), not the literal 3: with 7-screen windows the 3-screen number is unreachable by any eviction (a 70-row viewport alone exceeds it). Conductor/user: confirm this reading.
- Clippy: `mut` in update.rs test; unused `Handed::asked_before` in frames.rs; heavy-tests `too_many_lines` in `frames::check` → printing moved to `print_details`. No release runs touched.
- Not verified: production config at 100k/1M (heavy tier, 30f/46).
