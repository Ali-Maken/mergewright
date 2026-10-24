# ADR-0066: Acceptance walks and pty tests are a walk tier, run after every merge

Date: 2026-10-24 · Status: accepted

## Context
The fast tier (`cargo nextest run`) took 478–711 s against 180 s (PROMPT.md 7.1; review ease-pass-07-14 Major 5). Ticket
ease-pass/23 measured it (`docs/perf/23-fast-tier.txt`): 516.8 s, 7235 tests. No slow test had a test-code cause; the
largest product-code cost is the hint bar's fitting (ticket 24).

## Decision
- Acceptance walks (`*_walk` modules) and pty tests form the **walk tier**, split off by nextest's `default-filter` in
  `.config/nextest.toml` — not a cargo feature (plain clippy must still compile them, ADR-0018's lesson) and not `#[ignore]`.
- `scripts/walks.sh` runs them, after first checking every test is in exactly one tier. It is a merge gate (conductor) and a
  builder's pre-hand-in check when a ticket changes drawing, keys or a walk.
- What moves is chosen by kind, never by time alone. A fast test over 10 s is a bug.

## Consequences
- Fast tier 387–391 s after the split (6772 tests); walks 463 tests, 120–124 s solo. 180 s still unmet → ticket 24.
- A builder can break a walk and not see it until the merge gate, unless its ticket triggers the walk run.
- Acceptance commands naming walk modules need `--ignore-default-filter` or `--profile walks` (scripts updated in 23;
  older TESTING.md/ticket lines are not).
