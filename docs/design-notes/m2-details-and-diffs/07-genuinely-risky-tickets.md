# M2 design pass: details and diffs (2026-09-15) — Genuinely risky tickets

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [§8 coverage map](06-8-coverage-map.md) · Next: [Tickets](08-tickets.md)

---

## Genuinely risky tickets

- **`mistake-radar-detection`** — Fable (signature). The foxtrot/backward/accidental rules rest on one heuristic
  (merged-branch name from the subject + "is on the default chain"); a wrong rule is a wrong product, and no test can say the rule is right, only that the fixtures agree.
- **`git-runner`** — Opus. Structural for every M3–M6 write; judged by tests → not Fable.
- **`detail-model-and-jobs`** — Opus. Cancellation and stale-product rules shape six later tickets; testable → not Fable.
- No other ticket passes all three Fable tests. **Planned Fable builds: 1** (+ acceptance).
