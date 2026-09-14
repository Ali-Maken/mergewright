# m1-the-graph/59b — No-commit-graph heap at 1M: Phase B (the fix)

Status: ready
Build with: **Opus** (high) — same as 59.
Blocked by: 59 (Phase A done)

Split from 59 at its Phase A seam (conductor, 2026-09-14). Scope, signatures, acceptance, gates and the stop rule are 59's, unchanged.

## Read first
- `59-no-cg-heap-named-cause.md`: "Phase B — fix within ADR-0026, re-measure", "Signatures", "Acceptance" (Phase B lines), "Gates", "Phase A result", and "Handoff → Phase B" (fix list in order of yield, estimate, gotchas). Only those sections.

## Scope
In: 59's Phase B, following its handoff's order: (1) sort parents as CSR, (2) no `Vec`→`Arc` copy of the order, (3) paged interner — item 3's `by_prefix` part only if it is still "one table" under ADR-0026, else record. Measure after each step. 3× runs as 59 says; numbers into 59b for 56's re-run.
Out: everything 59 marks out; any bar; the cg path; `unsafe`/`madvise`/pressure relief.

## Acceptance
59's Phase B acceptance lines. If all three fixes land and the slope is still over 100 B/commit: stop, record the numbers and the named floor. That is a finding for the user, not a bar change.

## Size
One context, under ~80k. Near 45k with fixes left, stop at a clean seam with a handoff.
