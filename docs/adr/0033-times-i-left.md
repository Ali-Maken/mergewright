# ADR-0033: "Times I left" — the promise is measured, not asserted

**Date:** 2026-09-19
**Status:** accepted (user, 2026-09-19). Standing practice, every checkpoint from M3.

## Decision

The user keeps a note of **every time they leave Graft for the shell**. The list
lives in `TESTING.md` as **"Times I left"**, and every entry is a finding
reviewed at the next checkpoint.

In the user's words:

> "'I never need another tool' is only a real goal if it's measured."

## Why this is a gate and not a nice-to-have

Graft's whole claim is that a day's Git work happens inside it. Nothing in the
test suite can measure that: the fast tier proves logic on fixtures we invented,
the heavy tier proves scale, the acceptance walk proves the promise along a path
someone scripted. **Every one of them tests a path we already thought of.** A
reach for the shell is the one signal that comes from outside our imagination —
it marks precisely the thing nobody specified.

It also converts an unfalsifiable claim into a number that can go the wrong way.
"Graft covers my day" cannot be argued with. "I left eleven times this
milestone, four of them for the same thing" can.

## How it runs

- **Where:** `TESTING.md`, section "Times I left". One line: date, what the user
  wanted, what they ran instead. Half a line beats a forgotten one.
- **Who writes it:** the user, or the conductor when told. No ceremony, no triage
  at write time.
- **At each checkpoint:** the conductor reads the list, groups repeats, and
  reports them as findings. A reason that appears three times is a design gap,
  not a missing feature request.
- **Into design:** the list is Read-first material for the next milestone's
  design pass, alongside `docs/field-notes.md` (ADR-0029).
- **Entries are not deleted when fixed.** They get the commit that closed them
  and stay, so the trend is visible and the same gap is not rediscovered.

## What it does not mean

A reach for the shell is not automatically a bug or a promise to build the
thing. Some will be deliberate deferrals (`docs/agents/` scope, KNOWN_GAPS
"not planned"). The point is that the decision gets made in daylight, against a
count, instead of never surfacing.
