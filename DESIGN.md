# Graft: Design

This is the design produced by PROMPT.md section 10, step one: view mockups, the
full keymap, the graph layout algorithm, terminal-capability detection, and the
undo journal. No code — this document is reviewed by the user before M0 starts.

It uses the vocabulary fixed in `CONTEXT.md` (default branch, lane, rendering
tier, Ghost Preview, Mistake Radar, marker, ref badge, operation, undo journal,
Command Log, job) and follows the decisions in `docs/adr/0001`–`0010`. Where a
choice here narrows something PROMPT.md left open, it's called out as a design
decision so the checkpoint review can push back on it specifically.

**Split into one file per section** (housekeeping/01, 2026-09-18): read
[`docs/design/README.md`](docs/design/README.md) for the section index. Nothing
was deleted — this file used to hold every section in full; each is now its own
file under `docs/design/`, cross-linked. Cite the file, not a heading in this
document.

---

## Contents

1. [View mockups](docs/design/01-view-mockups.md)
2. [Keymap](docs/design/02-keymap.md)
3. [Graph layout algorithm](docs/design/03-graph-layout-algorithm.md)
4. [Terminal-capability detection](docs/design/04-terminal-capability-detection.md)
5. [Visualization](docs/design/05-visualization.md)
6. [Undo journal design](docs/design/06-undo-journal-design.md)
7. [Open design questions](docs/design/07-open-design-questions.md)
8. [Architecture](docs/design/08-architecture.md)
9. [Proposed changes](docs/design/09-proposed-changes.md)
