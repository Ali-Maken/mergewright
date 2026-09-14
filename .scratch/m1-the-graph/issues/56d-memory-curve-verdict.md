# m1-the-graph/56d — Memory curve acceptance: the verdict and the review section

Status: ready
Build with: **Fable** (high) — the acceptance judgment: what the numbers say about "memory tracks the viewport", against the bars. User decision 2026-09-14: Fable judges from 56c's numbers, no runs.
Blocked by: 56c

Split from 56 (conductor, 2026-09-14). **No cargo, no runs, no source.** If a number is missing or doesn't add up, say which and stop — don't measure it yourself.

## Read first (only these)
- `56c-memory-curve-runs.md` `## Numbers` (all).
- `56-memory-curve-acceptance.md`: "Acceptance", "For the M1 review" (the pass-1 table), "Spec-only pass 2" findings table and "For the user", "Spec-only pass 3".
- ADR-0026: "Decision (design)", "The gate at 1M" including the amendment (61), "User decisions on approval". ADR-0027 (first paint warm).
- DESIGN.md §3.1 and §3.6 (what the design says holds memory) — to judge whether the numbers show the design holding, not only each number under its bar.
- 59b's result section (the no-cg floor, if it stopped).

## Scope
In:
1. A verdict per budget: pass/fail, the number, the bar, and one line on whether it shows the design holding (which holder the design names, whether the numbers agree). A miss names its holder from 56c/59b's tables and goes to the conductor for the user — never re-derived here.
2. `## For the M1 review` in 56 rewritten from 56c's numbers (same columns, plus `checkpoint stride` and `heap B/commit`; pass-1 table kept as "before the findings tickets"). The M1 review file must reproduce it: say so in the report.
3. KNOWN_GAPS rows marked `verdict: 56d`: write each verdict/expiry sentence (closed, or open with expiry and who decides).
4. ADR-0026 status line: "Landed: m1/49–61", the slope guard's heap and RSS numbers as the user decided, the no-cg heap number or the floor finding.
5. Tick 56's acceptance boxes honestly (and 56c's numbers are the evidence, cite paths). Mark 56 `done` only if every box is ticked; else `partial` with what is open and whose decision it is.

Out: runs, fixes, bars, TESTING.md/DESIGN.md number edits (56c's).

## Acceptance
- [ ] Verdict per budget written in 56 with number, bar, design line.
- [ ] `## For the M1 review` rewritten; KNOWN_GAPS verdicts; ADR-0026 status line.
- [ ] 56's boxes ticked honestly; status set.
- [ ] Report: one line per budget (pass/fail + number), open items for the user, and "review file must carry the reported-cell table".

## Size
One context, target under ~30k tokens.
