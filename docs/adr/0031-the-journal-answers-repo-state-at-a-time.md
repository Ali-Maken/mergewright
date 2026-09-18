# ADR-0031: The undo journal answers "what did the repo look like at 14:20" from day one; the time machine appears in M5

**Date:** 2026-09-19
**Status:** accepted (user, 2026-09-19)

## Decision

**M3's undo journal is designed so it can reconstruct whole-repo state at a
point in time**, not only invert the last operation. The user's words:

> "The journal must be able to answer 'what did the whole repo look like at
> 14:20' from the start — that's a data-shape decision, free now and expensive
> to retrofit."

**No key exposes it in M3.** The time machine — "put everything back to how it
was N minutes ago" — ships in M5, behind Ghost Preview:

> "'put everything back' without showing what it will do is how someone loses
> work."

## What this binds on M3's design pass

The journal's entry shape must carry enough to answer a point-in-time query:
what refs pointed where, what the index and working tree held, per entry, with
a timestamp. Whether that is stored per operation or reconstructed by replay is
an implementation choice; being *able* to answer is not.

The per-kind inverse (the M3 signature ticket, Fable) stays as specified. This
ADR adds a constraint on the data shape, not a second mechanism.

## What this binds on M5

The time machine is an M5 operation like any other: preconditions, typed git
invocation, what it changes, and its inverse (the M3 brief's point 1 shape). It
is planned by Ghost Preview and shown before it is confirmed. It is not a
special case that bypasses preview.

## Why the split

Building the data shape late means migrating a journal that already has the
user's history in it. Building the key early means shipping a "put everything
back" button with nothing to show what it will do — in the one milestone whose
promise is "you cannot lose work".
