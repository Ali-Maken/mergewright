# M2 design pass: details and diffs (2026-09-15) — Hard-to-reverse choices

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [Interfaces and data shapes](02-interfaces-and-data-shapes.md) · Next: [Where I'd differ from DESIGN.md (Proposed changes)](04-where-id-differ-from-designmd-proposed-changes.md)

---

## Hard-to-reverse choices

1. **Diffs from system git through a typed runner, per file on demand.**
   Options: (a) gix tree diff + own unified rendering; (b) `git diff-tree` parsed
   (PROMPT Technology). **(b)**: follows the user's diff config, renames and
   copies for free; the runner (`GitInvocation`: typed args, `-z` byte paths,
   kill-on-stop, output cap) is what M3–M6 writes build on, so its shape is the
   real decision. File list first (`--raw -z --numstat -z`), hunks only for the
   selected file — a 2,000-file merge on git/git costs one small spawn.
2. **Mistake Radar is a paint-time lens over facts, not a layout input.**
   Options: (a) DESIGN §8.2: `graph::layout` takes a `Map<CommitId, Kind>` from
   a whole-history scan (O(history) subject reads before any marker shows;
   re-layout when it lands); (b) `classify_commit` per hydrated row, O(1), with
   one job building the global context (default-branch first-parent chain; ref
   facts from `RefBadges`; previous default tips from its reflog), plus lazy
   probes (evil per selected merge, force-push per tracked remote, duplicates
   over a 500-commit window). **(b)**: markers appear with the rows, cost nothing
   off-screen, and the fixture test still classifies every commit of every shape.
3. **"Contained in" via `git for-each-ref --contains <oid>`.** Options: (a) own
   reachability index over `CommitTable` (a ticket of its own, O(history)
   memory); (b) gix `revision` feature (drags gix-index, rejected in M1); (c) one
   git spawn per selection, cached 64 deep, cancelled on move. **(c)**; result
   bounded to "main, release/2.1 +47 more". PROMPT Technology says gix for reads:
   this is a query gix lacks cheaply (ADR-0011 names such gaps). Flagged below.
