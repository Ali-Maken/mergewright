# ADR-0030: Build order is M3, M4, M5, M6 — and daily use starts the day M3 lands

**Date:** 2026-09-19
**Status:** accepted (user, 2026-09-19)

## Decision

1. **Build through M6 in order: M3 (working tree, undo, stash), M4 (remotes),
   M5 (merge, rebase, reset, Ghost Preview), M6 (conflict resolver).**
2. **M7 (power tools) and M8 (polish, final name, ship) are judged on real use**,
   not on the plan. Neither is designed until M6 is done and there is evidence.
3. **The user starts using Graft as their daily tool the day M3 lands**, not
   after M6. Findings from that use shape M5 and M6 *while they are still being
   designed*.

## Why remotes are not "later"

The conductor proposed deferring M4 on the grounds that remotes matter mainly
when other people use the tool. The user rejected that, and the reason is the
part worth recording:

> "Without fetch, pull and push I'd be in the shell several times a day, which
> makes the evidence worthless."

Daily use is a test tier (ADR-0029). A tier that forces the user out to the
shell several times a day does not measure what it claims to. Remotes are not a
feature for other people — they are load-bearing for the measurement everything
after M3 depends on.

## What this binds

- **M3 must be daily-usable the day it lands**, not merely complete. Its design
  pass is written against that, not against a feature list.
- **M5 and M6 are not designed early.** Their design passes run after real use
  has had a chance to produce findings, so those findings can change them.
- `docs/field-notes.md` and TESTING.md's "Times I left" (ADR-0033) start
  collecting from the day M3 lands. They are inputs to M5's and M6's design
  passes, named as Read-first material.

## Cost

M7 and M8 may never be built, and that is the point — the decision moves from
the plan to the evidence. If real use says interactive rebase and blame are
daily, they get built; if the tool is already enough, they do not.
