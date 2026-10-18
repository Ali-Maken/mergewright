# Mergewright: Design — index

`DESIGN.md` was split into one file per top-level section (housekeeping/01,
2026-09-18) so a helper reads one section instead of the whole document.
Nothing was deleted or reworded; content only moved. See also: the preamble
below, and each file's own "see also" header linking its neighbours.

## Preamble (scope, vocabulary, decisions this document follows)

This is the design produced by PROMPT.md section 10, step one: view mockups, the
full keymap, the graph layout algorithm, terminal-capability detection, and the
undo journal. No code — this document is reviewed by the user before M0 starts.

It uses the vocabulary fixed in `CONTEXT.md` (default branch, lane, rendering
tier, Ghost Preview, Mistake Radar, marker, ref badge, operation, undo journal,
Command Log, job) and follows the decisions in `docs/adr/0001`–`0010`. Where a
choice here narrows something PROMPT.md left open, it's called out as a design
decision so the checkpoint review can push back on it specifically.

---

## Contents

1. [View mockups](#1-view-mockups)
2. [Keymap](#2-keymap)
3. [Graph layout algorithm](#3-graph-layout-algorithm)
4. [Terminal-capability detection](#4-terminal-capability-detection)
5. [Visualization](#5-visualization)
6. [Undo journal design](#6-undo-journal-design)
7. [Open design questions](#7-open-design-questions)
8. [Architecture](#8-architecture)
9. [Proposed changes](#9-proposed-changes)
10. [Repetition](#10-repetition)

---

## Sections

1. [View mockups](01-view-mockups.md)
2. [Keymap](02-keymap.md)
3. [Graph layout algorithm](03-graph-layout-algorithm.md)
4. [Terminal-capability detection](04-terminal-capability-detection.md)
5. [Visualization](05-visualization.md)
6. [Undo journal design](06-undo-journal-design.md)
7. [Open design questions](07-open-design-questions.md)
8. [Architecture](08-architecture.md)
9. [Proposed changes](09-proposed-changes.md)
10. [Repetition](10-repetition.md)

