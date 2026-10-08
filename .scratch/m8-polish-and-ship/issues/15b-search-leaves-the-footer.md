# 15b — Search leaves the footer after ⏎ (split from 15, A2)

Status: ready (conductor 2026-10-08; 15 merged) — split from `15-bar-content-pass.md` along its own seam; no new decision.
Build with: **Opus** (as 15).
Blocked by: 15 (merged first: `Searching` slots and fit order build on its A1/A4/A5).

## Scope

Exactly `15-bar-content-pass.md` **Amendment A2** (from "**A2 — Search**", line 59) and its acceptance boxes:
the three `A2` boxes plus the capture `docs/shots/m8-bar-after-search-120x36.txt`. Read 15's **NOTES (builder,
2026-10-08)** first: it is the handoff (what 15 left and where A2 starts). Scout:
`.scratch/m8-polish-and-ship/scouts/15-bar-content.md` (search path: `rows.rs:325-335`, `footer.rs:86-90`,
`escape.rs:30`, the test at `footer.rs:334` A2 replaces).

Out: everything else in 15 (built).

## Acceptance

Tick the A2 boxes **in 15's ticket file**, each naming its test, and this line:

- [ ] All three A2 boxes in `15-bar-content-pass.md` ticked with tests; capture `m8-bar-after-search-120x36.txt`
      written and asserted by a test, ≤200 KB. **Colour unverified.**

## NOTES
