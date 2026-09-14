# m1-the-graph/56c — Memory curve acceptance: the runs, the numbers, the mechanical doc edits

Status: ready
Build with: **Sonnet** (medium) — runs existing commands and writes their numbers down; no verdicts. User decision 2026-09-14: 56's remainder split so Fable only judges (56d).
Blocked by: 59b, 60, 63 (58b, 61 done)

Split from 56 (conductor, 2026-09-14) along "Spec-only pass 2 → Remaining here". Scope and commands are 56's, unchanged; only who does which part moved. **This ticket records numbers; it never says pass/fail on a budget** — 56d does.

## Read first (only these)
- `56-memory-curve-acceptance.md`: "Scope" (the command, the columns, the offer, the jumps, context guard), "Signatures", "Gates", "Spec-only pass 2 → Remaining here" items 1, 3, 4, 5.
- Result sections of 59b, 60 and 61 (their numbers are what the re-run confirms).
- TESTING.md "The heavy tier" and "Memory curve to a million commits (m1/46)"; DESIGN.md §3.1 `EmittedOrder` and `walker_state` lines, §3.6 the "grows with history" paragraph; ADR-0026 status line only.
- `scripts/memory-curve.sh`; `crates/graft-fixtures/MAP.md`.

## Scope
In:
1. Remaining item 1: full curve re-run (`scripts/memory-curve.sh 10000 100000 1000000`, release, 3 runs, 12 cells) → regenerate `docs/shots/m1-memory-curve.txt`; far jumps (rows/2, 9·rows/10) on both 1M fixtures; offer pair only if the offer line or its numbers changed.
2. Remaining item 5: heavy tier full run once, alone (`cargo test --release --features heavy-tests -- --test-threads=2`), then the release perf run (ADR-0021) with the 1M `frames::` scroll numbers. Report a red test with its output path; don't touch it.
3. **`## Numbers` section in this ticket** — the one input 56d reads. One table per group, every number with its unit and the path of the raw file it came from:
   - curve cells: peak RSS (3 runs), footprint peak and after `G`, heap B/commit, first paint (run 1 and warm), `G` time, odb objects during `G`, `overflow_len`, pack size, checkpoint stride;
   - far jumps: time and rows replayed at 500k and ~900k, both 1M fixtures;
   - slope guard: heap and RSS slopes;
   - heavy tier: pass/fail count, names of any red tests;
   - perf run: each `frames::` budget number next to its bar.
   Bars go in a column **copied from TESTING.md/ADR-0026 as written**, never computed or changed.
4. Remaining items 3–4, mechanical parts only: KNOWN_GAPS rows listed in 56's item 3 get their **numbers** updated (leave each row's verdict/expiry sentence to 56d; mark it `verdict: 56d`); TESTING.md curve table replaced with the new numbers; DESIGN.md §3.1/§3.6 one sentence each with the after numbers.
5. Context guard: all run output to `target/56c/` and `docs/shots/`; never paste curves or logs into your context beyond the numbers you copy.

Out: pass/fail verdicts, the `## For the M1 review` section, ADR-0026's status line, acceptance boxes in 56, any fix, any bar.

## Acceptance
- [ ] `docs/shots/m1-memory-curve.txt` regenerated, 12 cells × 3 runs.
- [ ] `## Numbers` complete as scoped, every row with a raw path; bars copied verbatim.
- [ ] Heavy tier run once alone and release perf run done; results in `## Numbers`.
- [ ] KNOWN_GAPS numbers, TESTING.md table, DESIGN.md sentences updated; verdict sentences left for 56d.
- [ ] Gates (56's list) green. A 1M fixture too slow or large: stop, say so with the number.
- [ ] Report: paths only; no verdicts.

## Size
One context, under ~60k. Runs ~15–25 min wall, all to files.
