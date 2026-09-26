# 06d set-aside-split — review 06b's M2 and Minors 2, 4, 5

Status: ready
Blocked by: 30 (same files).
Crates: graft-ops only.
Build with: Sonnet (move code, no behaviour change; small tests).
Scout: none. Source: `docs/reviews/m7-06b-absorb-staged.md` §M2 and §Minor.
Split by: conductor, 2026-09-26. No new decision.

## Scope
- **M2:** get `apply/set_aside.rs` under 400 lines by moving the rebuilt-stash unit to `apply/set_aside/rebuild.rs`. Put the `rev-parse --verify` and `commit-tree` helpers in one crate-private module that both set_aside and absorb use. Re-read after 30: line numbers will have moved.
- **Minor 2:** the dead `Copied` arm. Drop it and fix `StaysWhy::Renamed`'s doc and the MAP line.
- **Minor 4:** `a_fixup_that_empties_its_target_drops_it` asserts that the id of the pick below is unchanged.
- **Minor 5:** a test for a rename with an edit and the tree edited on (probe p6).

## Acceptance
- [ ] `wc -l` for every touched file, all ≤ 400; every set-aside test green (name the filter and count).
- [ ] Minor 2, 4, 5: each change named with its test.
- [ ] Gates workspace-wide.

## Touches
MAP.md (graft-ops). Every user of the moved helpers (grep).
