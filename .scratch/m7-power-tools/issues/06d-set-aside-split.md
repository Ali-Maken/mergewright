# 06d set-aside-split — review 06b's M2 and Minors 2, 4, 5

Status: done
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
- [x] `wc -l`: set_aside.rs 360, set_aside/{answer 161, put_back 268, rebuild 63, returned 186}.rs, objects.rs 45, absorb/build.rs 187, absorb/mod.rs 238. Exception: tests/it/absorb_staged.rs was 553 before, ~590 now (test file, not split — see NOTES). Filter `nextest -p graft-ops -E 'test(/absorb|set_aside|put_back|rename|fixup/)'`: 133/133 pass.
- [x] Minor 2: `Copied` arm dropped (absorb/mod.rs), `StaysWhy::Renamed` doc + MAP line fixed — covered by `a_staged_rename_stays_whole` (still green). Minor 4: `a_fixup_that_empties_its_target_drops_it` asserts f3's id unchanged. Minor 5: `a_staged_rename_with_an_edit_and_the_tree_edited_on_stays_whole` (RM, index byte-equal, tree as left).
- [x] Gates workspace-wide: fmt, clippy -D warnings, nextest 4052/4052, test --doc ok.

## Touches
MAP.md (graft-ops). Every user of the moved helpers (grep).

## NOTES (builder, 2026-09-27)
- Split differs from the ticket's single `rebuild.rs`: after 30, `from_index_base` lived in put_back.rs, so set_aside.rs needed ~270 lines out. Moved: `rebuild.rs` (from_index_base), `returned.rs` (step 6 `put_back`, `Returned`, `Kept`, drop), `answer.rs` (`Outcome`, `status_for`, `answer`, `back`). Code moved verbatim; visibility only.
- Shared helpers: `crate::objects` (`resolve`, `commit_tree`, `printed_oid`), used by set_aside/rebuild.rs and absorb/build.rs. Absorb's commit now `commit-tree -m <subject>` instead of `-F -` with `<subject>\n` on stdin: same message bytes; absorb records no command line. `rewrite/reads.rs:272` commit-tree and `rewrite/progress.rs:148` rev-parse not moved (outside the ticket's "set_aside and absorb").
- absorb_staged.rs over 400 before and after; a test file — split left for the conductor.
- No compaction.
