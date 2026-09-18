# Graft: Design — Open design questions

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Undo journal design](06-undo-journal-design.md) · Next: [Architecture](08-architecture.md)

---

## 7. Open design questions

Parameters this document fixed with a reasonable default but that the M0/M1
prototype (PROGRESS.md step 3) or early milestones should validate against a
real terminal and real fixtures, adjusting here (and in an ADR, if the answer
changes something PROMPT.md fixed) rather than in code comments:

- ~~**`graph.max_lanes` default of 8** before compaction kicks in (§3.5)~~ —
  resolved against `git/git` and a many-lanes fixture: unset means 8 lanes
  under 100 columns, 10 at 100–159, 12 at 160+ (ADR-0022); the cap is a
  budget admitted by priority, not a set of vetoes (ADR-0025).
- **Checkpoint interval of 2,000 rows** (§3.6) trades memory against jump
  latency; tune it against the heavy tier's 100k-commit fixture once that
  exists.
- **Strip height of 32 rows** (§3.8) trades cache granularity against the
  cost of rendering one new strip per scroll frame; tune it in the same
  performance pass.
- **Capability-probe budgets (200 ms local / 500 ms over SSH)** (§4.1, §4.4)
  are starting points, and `tmux -V`'s deadline is the local one by
  construction rather than a number of its own; the manual terminal checklist
  (PROMPT.md §7.1) is what actually validates them across
  Ghostty/Kitty/WezTerm/iTerm2/tmux.
- **8×16 px fallback cell size** (§4.1) when no geometry query answers at
  all — a placeholder until the graph-drawing prototype shows whether that
  assumption produces a usable picture or needs to be per-platform.
- **`Ctrl+A` for "stage all"** (§2.4) is this document's choice to keep the
  bulk variant off a bare letter; worth confirming it doesn't collide with a
  terminal or OS-level binding on the user's actual setup before it ships.
- **First-paint order vs. the commit-graph file (§3.2, ADR-0008)** is the
  biggest performance risk in this document, not a tunable: validate
  against the `git/git` heavy-tier fixture, before M1 ships, that the
  commit-graph path really paints inside 150 ms, and that the
  no-commit-graph path's date-order→topo promotion re-anchors selection
  and scroll without visible jumping.

None of these are stubbed features — they're documented defaults and decided
designs awaiting validation, not gaps in scope, so they aren't duplicated in
`KNOWN_GAPS.md`.

---
