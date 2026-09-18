# M2 design pass: details and diffs (2026-09-15)

Read: CLAUDE.md standing rules; PROMPT.md §1.3–1.4, §2, §3, §4 Comparing, §5 Diffs,
§6.1, §6.3, §7.2, §8 M2; DESIGN.md §1.1–1.4, §1.8, §2.2–2.5, §2.15, §3.1, §5.1–5.3,
§8; ADR-0007, 0011, 0023; every `crates/*/MAP.md`; KNOWN_GAPS → M2 rows;
PROGRESS.md compaction lines; AUTHORSHIP.md; M1 review round 1. No source.

**Promise:** *what a commit changed is readable without leaving the graph, and a
bad merge is flagged and explained.* The acceptance ticket proves it on the
fixtures (foxtrot, backward, accidental-pull, evil, cherry-pick-duplicates,
unicode-names, non-utf8-path) and on git/git.

**Split into one file per section** (housekeeping/01, 2026-09-18): read
[`m2-details-and-diffs/README.md`](m2-details-and-diffs/README.md) for the
section index. Nothing deleted — cite the file, not a heading in this document.

## Sections

- [Sizing from M1](m2-details-and-diffs/01-sizing-from-m1.md)
- [Interfaces and data shapes](m2-details-and-diffs/02-interfaces-and-data-shapes.md)
- [Hard-to-reverse choices](m2-details-and-diffs/03-hard-to-reverse-choices.md)
- [Where I'd differ from DESIGN.md (Proposed changes)](m2-details-and-diffs/04-where-id-differ-from-designmd-proposed-changes.md)
- [New UI](m2-details-and-diffs/05-new-ui.md)
- [§8 coverage map](m2-details-and-diffs/06-8-coverage-map.md)
- [Genuinely risky tickets](m2-details-and-diffs/07-genuinely-risky-tickets.md)
- [Tickets](m2-details-and-diffs/08-tickets.md)
