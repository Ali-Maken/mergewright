# Conductor's queue — user decisions of 2026-09-18, to apply once the context-diet ticket returns

The context-diet ticket (`housekeeping/01`) is rewriting PROGRESS.md and KNOWN_GAPS.md right now.
These edits wait for it rather than racing it. Apply all three, then delete this file.

1. **KNOWN_GAPS — new row (user, 2026-09-18), goes in the *live* gaps file, not the closed archive:**
   Force-push verdicts now cover only branches seen this session, at most 32 (M2 review fix round 1a,
   commits 32845d3/6437790). **The Mistake Radar is no longer exhaustive for that probe:** a force-push
   on a branch you never visited goes unflagged. This is a behaviour change, not an optimisation note —
   the user asked for it in the gaps list explicitly, not only in a commit message.

2. **PROGRESS.md session log — record the two decisions:**
   - **M11 won't-fix (user, 2026-09-18):** the per-frame recompute of radar lines was a symptom of the
     missing cache; round 1a's cache fixed the cause. Optimising the symptom now would be work for nothing.
   - **M2 is not done over a red gate (user, 2026-09-18):** `housekeeping/02-heap-slope` runs before the
     M2 checkpoint. Measure first, then either get under the 50 B/commit bar or bring the user an ADR
     arguing the bar with the evidence. A bar change needs the user's approval before it lands.

3. **Order of remaining work, decided:** M2 review fix round 1b (rendering Majors) → `housekeeping/02`
   (so the reviewer sees the gate green or an approved ADR) → reviewer round 2 → M2 checkpoint.
