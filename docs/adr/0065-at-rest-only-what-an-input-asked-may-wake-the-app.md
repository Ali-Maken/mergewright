# ADR-0065: At rest, only what an input asked may wake the app; under the fold, rows are shown commits

Date: 2026-10-22 · Status: accepted

## Context
- Ticket 18, cause 2: after `Rested`, fold counts chained one wake per merge, each answer asking the next
  (`docs/perf/18-lag.txt:28-35`; `fold_count.rs:28-32`).
- Ticket 16: heavy-tier tests assumed one graph row per commit, but the fold is the decided default
  (graph MAP.md:29; design note :226-243), so rows ≠ commits under it (`heavy.rs:585`, `after_idle.rs:58`).
- Checked 2026-10-22: no earlier ADR decides idle wakes. ADR-0054 ("waiting is not idling") governs frames, not
  which reads may run at rest; m7-power-tools/12's `idle_gap_wakes` is a test measure, not a decision.

## Decision
- **D1.** An input's reads are asked by its own update, its `Rested`, or its first window, and each is answered
  once. No read asked by another read's answer; no timers, polls or background reads.
- **D2.** Fold counts are asked at rest in one batched read per rest (every uncounted fold on screen), not one per fold.
- **D3.** Under the fold, rows + hidden == walked == every commit. Time budgets run in both modes (default fold and
  every branch shown).

## Consequences
- Every future background read must be one of D1's three sources. `frames.rs:393` enforces it through `unasked_wakes`;
  the harness's notion of "asked" (`readings.rs:257`) matches D1.
- A batch shows its first number later than a chain did, but all at once.
- Tests assert shown commits under the fold, not total commits.
Source: `.scratch/ease-pass/issues/19-fold-rules.md`.
