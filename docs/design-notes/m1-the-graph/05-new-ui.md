# M1 design pass: the graph after the gallery walk (2026-09-13) — New UI

[See also: m1-the-graph/README.md](README.md) for the full section index.
Previous: [Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)](04-where-id-differ-from-designmd-promptmd-proposed-changes.md) · Next: [Genuinely risky tickets](06-genuinely-risky-tickets.md)

---

## New UI

**Fold vocabulary** (graph column, all tiers). Eye lands on nodes and lane 0;
the rail is the quietest thing in the column.

| Element | Unicode | ASCII | Pixel |
|---|---|---|---|
| folded outgoing edge | `▾` gap cell right of node, branch colour | `v` | tail curving right-down, one lane long, alpha → 0 |
| folded incoming edge | `▴` same cell | `^` | tail curving right-up, same |
| several on one row | one glyph, first edge's colour | same | one tail |
| rail (last drawn column) | `⋮` text-muted, every row | `:` | 1 px dotted vertical, muted |
| count | header `+54 lanes` after filters; absent when nothing folds | same | same |
| expanded (`+`) | no rail, no count, `+ collapse` hint | same | same |

**Footer reveal** (all breakpoints): while the pointer is on a row whose
subject is cut with `…`, or after `Enter` on such a cursor row, the hint line
is replaced by the full subject (sanitised, one line, cut at width). Hints
return when the pointer leaves the row / any key. Subject not truncated:
nothing happens — no empty line, no label.

**Copied notice**: `y` → footer notice `848ec2e… → clipboard (osc 52)` until
the next key. Graft cannot see whether the terminal honoured OSC 52; the
wording says what Graft did, not what happened. No selection: nothing.
