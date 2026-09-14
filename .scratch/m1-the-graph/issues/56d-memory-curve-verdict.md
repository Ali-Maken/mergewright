# m1-the-graph/56d — Memory curve acceptance: the verdict and the review section

Status: done (2026-09-14, builder-fable; no compaction)
Build with: **Fable** (high) — the acceptance judgment: what the numbers say about "memory tracks the viewport", against the bars. User decision 2026-09-14: Fable judges from 56c's numbers, no runs.
Blocked by: 56c

Split from 56 (conductor, 2026-09-14). **No cargo, no runs, no source.** If a number is missing or doesn't add up, say which and stop — don't measure it yourself.

## Read first (only these)
- `56c-memory-curve-runs.md` `## Numbers` (all).
- `56-memory-curve-acceptance.md`: "Acceptance", "For the M1 review" (the pass-1 table), "Spec-only pass 2" findings table and "For the user", "Spec-only pass 3".
- ADR-0026: "Decision (design)", "The gate at 1M" including the amendments (61; no-cg heap), "User decisions on approval". ADR-0027 (first paint warm).
- DESIGN.md §3.1 and §3.6 (what the design says holds memory) — to judge whether the numbers show the design holding, not only each number under its bar.
- 59b's result section (the no-cg floor, if it stopped).

## Scope
In:
1. A verdict per budget: pass/fail, the number, the bar, and one line on whether it shows the design holding (which holder the design names, whether the numbers agree). A miss names its holder from 56c/59b's tables and goes to the conductor for the user — never re-derived here. The no-cg heap verdict is on the **live** number (≤ 100 B/commit, gated) — the footprint number is reported beside its named floor (~140) in the same line, not judged pass/fail.
2. `## For the M1 review` in 56 rewritten from 56c's numbers (same columns, plus `checkpoint stride` and `heap B/commit`; pass-1 table kept as "before the findings tickets"). The M1 review file must reproduce it: say so in the report.
3. KNOWN_GAPS rows marked `verdict: 56d`: write each verdict/expiry sentence (closed, or open with expiry and who decides).
4. ADR-0026 status line: "Landed: m1/49–61", the slope guard's heap and RSS numbers as the user decided, the no-cg heap number or the floor finding.
5. Tick 56's acceptance boxes honestly (and 56c's numbers are the evidence, cite paths). Mark 56 `done` only if every box is ticked; else `partial` with what is open and whose decision it is.

Out: runs, fixes, bars, TESTING.md/DESIGN.md number edits (56c's).

## Acceptance
- [x] Verdict per budget written in 56 with number, bar, design line (`56-memory-curve-acceptance.md` "## Verdict").
- [x] `## For the M1 review` rewritten; KNOWN_GAPS verdicts; ADR-0026 status line.
- [x] 56's boxes ticked honestly; status set (`done`; the open items are decisions, not misses — see 56 "Open, whose call").
- [x] Report: one line per budget (pass/fail + number), open items for the user, and "review file must carry the reported-cell table".

## Result (2026-09-14, builder-fable)

Judged from 56c's `## Numbers` only; no runs, no source, no raw logs. Every number needed was present and added up (footprint slope 138–140, live 76, RSS − footprint ≈ pack size, far-jump rows = row mod stride + one chunk). Beyond the two rows 56c marked `verdict: 56d`, three stale KNOWN_GAPS rows that 56's item 3 named (2× `EmittedOrder`, no slope guard, order ~90 B/commit) were closed with 56c's numbers, and the three rows 56 owed but no ticket had written (cold first paint / ADR-0027, the missing 1M `frames::` numbers, the counters harness) were added as one section. One fidelity note: DESIGN.md §3.1/§3.6 cite "9–12 B/commit (m1-the-graph/56c)" for the order, but 56c's `## Numbers` carries no order-bytes column — the figure is pass 1's counters run; 59b's profile (order 8 B/commit live) corroborates it, and no bar sits on it.

## Size
One context, target under ~30k tokens.
