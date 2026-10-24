# 24 — The hint bar re-measures on every step of its ladder

Status: ready · Build with: Sonnet · Blocked by: 23 (merged)
Source: 23 NOTES (`.scratch/ease-pass/issues/23-fast-tier-speed.md:221-250`, `docs/perf/23-fast-tier.txt`).

## Need
The fast tier runs 387–391 s against 180 s. The biggest single cost is product code, not test code: 8 hints tests
(`graph_screen/hints/tests.rs:90`, `:1282`, ~212 test-seconds) and `update/tests/bar_keys.rs:56` sweep `hints_for`
(`graph_screen/hints.rs:119`), which clones and re-measures the bar on every ladder step (`hints/fitting.rs:129-216`).
The same path runs on every frame the bar draws, so it is also frame cost in the app.

## Scope
1. Measure first: `hints_for` cost per call (unit bench or timed test, script → `docs/perf/24-hints-fitting.txt`) and the
   8 tests' seconds.
2. Fix at cause: measure each entry once and fit the ladder from the cached widths (no clone per step). Output must be
   byte-identical: every existing hints/bar test and capture unchanged (none re-accepted).
3. Then probe the workspace profile: `[profile.dev.package.mergewright-ui] opt-level = 1` (and graph, ops) — record cold build
   seconds and fast-tier run seconds, before and after, two runs each. Adopt only if build + run is a net win on a cold and a
   warm build; else record and reject.
4. Fast tier measured twice with `scripts/23-fast-tier.sh` at the end.

## Acceptance
- [ ] `hints_for` per-call cost before/after, perf-file line.
- [ ] Every hints/bar test unchanged and green (name the test modules); no snapshot accepted.
- [ ] opt-level probe numbers recorded; adopted or rejected with the numbers.
- [ ] Fast tier two runs, seconds and slow_over_10s, perf-file lines.
- [ ] Five gates + `scripts/walks.sh`; Touches:.
