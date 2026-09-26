# 06e absorb-pushed-and-whitespace — review 06c's M1, M3 and Minors 3, 5, 6

Status: ready
Blocked by: 30 (holds graft-ops; M3 is in the put-back 30 rewrites).
Crates: graft-ops only.
Build with: Opus (the team-safety rule, and a put-back).
Scout: none. Source: `docs/reviews/m7-06c-absorb-config.md`.
Split by: conductor, 2026-09-26. No new decision. The review's M2, Minor 2 and both Questions are the planner's and are not here.

## Scope (each at the cause, red first)
- **M1:** the pushed stop is unpinned. A mutant that counts only remote *tips* as pushed passes all graft-ops tests. A mutant that counts only `*/main` passes the 12 `absorb_unpushed` tests, and a probe showed it rewrites a teammate's commit. Add tests that kill both mutants: a pushed commit below a remote tip, and a commit on a non-`main` remote branch. Show each mutant red.
- **M3:** with `apply.whitespace`, `fix` strips a stay's staged spaces and `error` fails the run (`apply/set_aside.rs:508-514`; re-read after 30, since the lines will have moved). Fix at the cause, as 06c did for absorb's own apply.
- **Minor 3:** `AlreadyPushed.by` only ever holds `AmendInto`, and `by = Open` would render an untrue sentence. Drop the field, or document it.
- **Minor 5:** the oracle hard-codes `main..feature` (`absorb_printed.rs:72`). Take the range as a parameter, and add one oracle row on the fallback range.
- **Minor 6:** split `absorb_and_a_into_agree_on_the_range` into one test per fixture.

## Acceptance
- [ ] M1: both mutants shown red, with the tests named.
- [ ] M3: `fix` and `error` cases as tests, red first.
- [ ] Minors 3, 5, 6: each change named with its test.
- [ ] Gates workspace-wide.

## Touches
Every `unpushed_range` caller (absorb, `a`), the put-back callers, MAP.md (graft-ops). Name how each was checked.
