# M1 design pass: the graph after the gallery walk (2026-09-13) — What the screenshot says (facts the decisions rest on)

[See also: m1-the-graph/README.md](README.md) for the full section index.
Next: [Interfaces and data shapes](02-interfaces-and-data-shapes.md)

---

## What the screenshot says (facts the decisions rest on)

- `+54` is the same on every row **by construction**: `Fold::plan` folds
  *columns* for the whole window, so N is a per-window number printed per row.
- Every folded lane is nodeless in the window (a column holding a node is
  never folded). The curves into the marker therefore carry no per-row
  information: they say "second parent is somewhere behind the fold", 40×.
- The cursor starts on the top row; its ancestry is all of lane 0; the
  ancestry thread draws heavier (ticket 16/22). So the heavy stroke is the
  **default state**, not a highlight.
- The Unicode capture shows stale cells (`many/00`, `+17a`, `│r│`, `[seen]0`):
  the previous stop's frame bleeding through where the graph column got
  narrower. Not a decision; 37/38 territory (see report).
