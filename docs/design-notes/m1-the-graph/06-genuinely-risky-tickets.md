# M1 design pass: the graph after the gallery walk (2026-09-13) — Genuinely risky tickets

[See also: m1-the-graph/README.md](README.md) for the full section index.
Previous: [New UI](05-new-ui.md)

---

## Genuinely risky tickets

- **39** (fold model, text tiers): `DrawnRow`'s shape is structural for 40
  and for M2's marker column. Judged by property tests and snapshots → fails
  Fable test 3 → **Opus**.
- **40** (pixel stubs and rail): correctness is by eye only; approach is
  decided here → fails test 1 → **Opus**, user judges the PNGs.
- No ticket passes all three Fable tests. **0 Fable builds** this pass
  (38 is already Fable and re-proves the promise after these land).
