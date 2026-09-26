# 06c absorb-config-and-oracle — review 06's remaining Majors

Status: ready
Blocked by: 06b (same files, graft-ops). Question 2's ruling from the planner (below) before the Minor 1 box.
Crates: graft-ops only (`absorb.rs` and its split, tests). Another crate needed → commit at the seam, report `blocked:
needs <crate>` with the signature.
Build with: Opus (config-sensitive git reads; an oracle that must not share the code under test).
Scout: none. The review cites every site; re-read each before editing.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-06-absorb.md`. No new decision.

## Scope
Each fix is the one the review's section states, at the cause, red first.
- **M1** (§M1): `blame.ignoreRevsFile` makes absorb fail (missing file) or misplace (present). Absorb's blame must not
  read his ignore-revs config.
- **M4** (§M4): a staged submodule bump fails the whole absorb.
- **M5** (§M5): `apply.whitespace` changes what is absorbed or fails the map.
- **M7** (§M7): split `absorb.rs` (571 lines) and its test file (659) under 400 each. No behaviour change.
- **M8** (§M8): an oracle that computes targets independently of the map under test, and tests for the three
  placement rules without one.
- **Minor 1 / Question 2** (what "his branch" is on `main` with no upstream): build only the planner's ruling, appended
  below when it lands. If it has not landed when you reach it, leave that box open and say so.

## Acceptance
- [ ] M1: both cases (file missing, file present) as tests, red first.
- [ ] M4, M5: each as a test, red first.
- [ ] M7: `wc -l` of every file after the split, all ≤ 400.
- [ ] M8: the independent oracle named; each of the three rules' tests named, each seen red under a mutation.
- [ ] Minor 1: per the planner's ruling (below), test named.
- [ ] Gates workspace-wide.

## Touches
MAP.md (graft-ops) for moved code; every caller of `absorb` (grep: graft's `A` key path). Name how each was checked.
