# M2 design pass: details and diffs (2026-09-15) — Sizing from M1

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Next: [Interfaces and data shapes](02-interfaces-and-data-shapes.md)

---

## Sizing from M1

20 of ~100 M1 builds compacted (AUTHORSHIP.md). Causes, from the log lines:
- **Whole big files read** — 41 (~100k), 70/71 (splitting a 1.5k-line `update.rs`), 48 (131k).
- **Measurement or pty runs in the builder's context** — 07 ("after pty runs"), 26 (pty+pyte at 100k), 27 (memory), 47/51b (heavy runs; 51b "ended turn 3× waiting").
- **Several modules in one ticket** — 12 (git status + app wiring + chrome), 08 (164k), 09 (160k), 51 (168k: graph + app + fixes).
- **Visual iteration** — 17 (132k: mouse + tooltip + sanitising), 41.
- **Spec question mid-build** — 52 blocked, then compacted; 36 stopped for a design pass.
- Fable and Sonnet compacted as readily as Opus: it is the ticket, not the model.

Rules applied to every M2 spec: (1) at most two crates per ticket and new modules
named up front, never "edit `update/`" wholesale; (2) captures and measurements
go to files by script (`docs/shots/`, `docs/perf/<ticket>.txt`), the builder reads
`wc -c`/grep/summary lines only; (3) at most six snapshots per ticket; (4)
consumers read the producer's MAP.md, never its source (interface tickets ship
real code, so that is enough); (5) a spec gap is a `partial` report at once, not
a guess; (6) no ticket waits on a heavy run — nothing in M2 needs one. Budget:
~600 new lines, ≤ 30 tool calls. Each spec names its seam for a `partial`.
