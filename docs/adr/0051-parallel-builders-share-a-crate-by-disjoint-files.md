# ADR-0051: Parallel builders may share a crate when their file sets are disjoint

Status: accepted (user, 2026-09-27). Amends ADR-0036.

## Context

ADR-0036 lets builders run concurrently only with disjoint **crate** sets. In M7 almost every remaining ticket touches
`graft`, so the queue ran one ticket at a time: about 20 Opus builds serial behind one crate. Building never clashed:
each builder already has its own worktree and `CARGO_TARGET_DIR`. The crate rule only guarded against merge conflicts.

## Decision

1. **Split by crate first.** Where a ticket's non-`graft` part stands alone, the planner splits it out so it runs beside
   the `graft` queue under the old rule.
2. **Within one crate, disjoint files.** Up to **three** builders may hold the same crate at once when their specs'
   **file lists** share no file. The planner writes each ticket's file list; the conductor pairs tickets by it.
   Shared registries (`mod.rs` lists, MAP.md, the hint catalogue, `docs/design/02-keymap.md`, captures) count as files.
3. A builder that finds it must edit a file outside its list stops and reports `blocked: needs <file>`, as with a crate.
4. The conductor merges one branch at a time and runs the gates after each. A conflict it can't resolve from the two
   specs goes back to the owning builder.
5. Timing runs (perf, benchmarks, diagnosis) still need a quiet machine: the conductor pauses dispatch for them.

## Consequences

- Wall-clock for a serial crate queue drops roughly by the number of builders.
- More disk and CPU at once; worktrees are removed after each merge, as before.
- Merge conflicts become possible where a file list was wrong; the gates after each merge catch semantic clashes.
