# idle-layout-advance — while he isn't typing, the layout runs ahead; `G` is warm; a key always preempts it

Status: ready
Blocked by: 04-slow-key-cause, 04b-harness-input-thread
Crates: graft (`event_loop::pump` idle step, `History`), graft-graph (`StreamingLayout` bounded advance, `truncate`
fingerprint).
Source: note "Pass 2a" R8 step 4. The rows, quoted:
- KNOWN_GAPS :149 "Layout advances only as far as the screen asks (DESIGN.md §3.6 wants background advance between
  frames; on-demand instead, `G` cold until it's run once — first paint 13–113 ms, `G` on git/git 0.52–1.3 s …) … fast
  keys can stall at the last laid-out row."
- :161 "The walk answers nothing while a promotion lays out to the anchor … Bound the advance, or answer from the
  provisional layout."
- :169 "A chunk cut back by `truncate` loses its eviction fingerprint — a later rebuild of that chunk goes unchecked
  once."
- PROGRESS.md :102 (the user).

Build with: Opus. Event-loop scheduling with measurable outcomes (latency under the frame budget during idle work; `G`
time; idle wakes = 0 once caught up).
Scout: `scout` first (ADR-0035). Reuse `slow-key-cause`'s scout file for the event-loop answers, and only add what's
missing.
Needs: slow-key-cause (its perf file names the cause and the gate; this ticket must not regress it).

## Decided
- **Idle step:** when `Wakes::waiting()` is false and no input is pending, `pump` calls `History::idle_step(deadline)`
  with a **2 ms** slice. It advances the layout frontier until it is **10 screens** past the viewport, or history ends.
  Then it returns `IdleWork::Caught`, and no further idle wakes are scheduled until something changes (new refs, a
  scroll past the frontier, a promotion).
- **Preemption:** the slice checks the deadline between batches. A key that arrives is served next frame; its latency
  is inside the frame gate.
- **The order with today's idle rasterise** (m1/30h, `PageDown`/`PageUp` strips): rasterise first (it's what the next
  key shows), then advance.
- **:161:** a promotion's advance is bounded by the same slice; until it catches up, the walk answers from the
  provisional layout.
- **:169:** `truncate` re-fingerprints the prefix it keeps.
- **Memory:** the advance respects the layout cache's existing limits. Heap slope is checked by
  `scripts/heap-slope.sh` before and after.

## Read first
- Note "Pass 2a" R8; `docs/perf/m7-key.txt` (summary lines, from `slow-key-cause`).
- DESIGN.md §3.6 (via `grep -n "3.6" DESIGN.md` → the section in `docs/design/`).
- `crates/graft/MAP.md`: `event_loop` (`pump`, `Wakes`), `history` (`History`, `outstanding`, `RowRequest`, `Views`,
  `stream`/`serve`). `crates/graft-graph/MAP.md`: `StreamingLayout` (`advance`, `rows`, checkpoints, `truncate`).

## Scout brief (≤ 1 page)
1. `StreamingLayout::advance`'s signature and its budget unit (rows? commits?). Can it stop at a deadline between
   batches?
2. Who owns the layout while a history job runs (the worker that "holds" the walk). Can the idle step run on the UI
   thread, or must it be a job?
3. `Wakes`: how an idle wake is scheduled, and how to say "nothing to do".
4. `truncate`'s fingerprint path (:169).
5. The promotion path (:161): where it lays out to the anchor.

## Signatures
```rust
// graft
impl History { pub fn idle_step(&mut self, deadline: Instant) -> IdleWork; }
pub enum IdleWork { Advanced { rows: usize }, Caught }
// graft-graph
// StreamingLayout::advance_until(&mut self, deadline: Instant, /* today's source args, named by the scout */) -> Advance;
```

## Acceptance
- [ ] graft-graph:
  - `advance_until_stops_at_the_deadline` (red first);
  - `truncate_keeps_a_fingerprint_for_the_kept_prefix` (red first, :169);
  - `a_rebuild_after_truncate_is_checked`.
- [ ] graft:
  - `idle_advances_until_ten_screens_ahead_then_stops`;
  - `caught_up_schedules_no_wakes` (idle CPU);
  - `a_key_preempts_the_idle_step`;
  - `a_promotion_answers_from_the_provisional_layout` (:161);
  - `new_refs_restart_the_idle_step`.
- [ ] **Release** (through `scripts/m7-perf-key.sh`, 3 runs → `docs/perf/idle-layout-advance.txt`):
  - p99 ≤ 16 ms per scenario, not worse than `docs/perf/m7-key-after.txt` beyond the recorded spread;
  - a new scenario `g_after_idle`: `G` after 2 s idle on the 50k fixture, recorded and asserted under the frame budget.
    If it can't meet the budget, the number is recorded and the builder stops and asks; no silent threshold.
- [ ] `scripts/heap-slope.sh` before/after into the same perf file; the slope is within the recorded noise.
- [ ] MAP.md (graft, graft-graph). TESTING.md "Performance" rows. The conductor closes the KNOWN_GAPS rows :149, :161 and
      :169 by test name.

## Touches (check each, report how)
- **Every scroll/jump path (`j k PgUp PgDn G g`, search seek, Map seek):** these serve from the same layout. Check: the
  graph navigation tests + m1 captures byte-identical.
- **Idle rasterise (m1/30h):** order kept. Check: its tests.
- **The history walk's worker and jobs:** Check: the history tests; no new thread unless the scout says one is needed
  (then named).
- **Battery / idle CPU:** `caught_up_schedules_no_wakes`.
- **Keymap, journal, captures:** none. Captures must stay byte-identical; the check is `check_shot` over all.

## Rules / Gates
As rewrite-engine. The release runs go through the script only.

## Size
Medium-large (~60k).
