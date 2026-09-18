# ADR-0032: Branch dependency and chain rebase — not "stacked PRs", and not Graphite

**Date:** 2026-09-19
**Status:** accepted (user, 2026-09-19). **Conditional:** built only if real use confirms the pain.

## The correction

The conductor offered "skip stacked work — it's a team-workflow feature and you
work solo." That was wrong on the facts. The user works in a team, and named the
actual pain:

> "I changed a branch that others are built on, now fix the chain."

That is not Graphite's problem (PR stacking, review flow, an external service).
It is a **dependency graph** problem — and Graft is the tool that draws the
dependency graph.

## Decision

Two capabilities, scoped to M7, **built only if real use confirms the user hits
this**:

1. **Show when branches depend on one another.** The graph already computes what
   it needs; this is making an existing relationship visible, not a new model.
2. **"Rebase this branch and everything built on it" as one operation**, with a
   **Ghost Preview of the whole chain's new shape** before confirming — not a
   per-branch preview repeated N times.

## Explicitly out

- **No PR integration.** No GitHub, no forge API, no review state.
- **No external service.** Nothing that needs an account or a daemon.
- **No new mental model.** No "stack" object the user has to learn, name or
  maintain. Branches and their dependencies are what Git already has; Graft
  draws them.

## Why M7, and why conditional

It is history editing, so it belongs with interactive rebase and the other M7
tools — it needs the same machinery and the same preview. And per ADR-0030, M7
is judged on real use: if the user's `docs/field-notes.md` and "Times I left"
(ADR-0033) show they hit the broken-chain problem, it gets built. If the notes
never mention it, it does not.

Recorded now so the decision is deliberate rather than drifting, and so M5's
Ghost Preview is designed knowing a whole-chain preview is a plausible caller.
