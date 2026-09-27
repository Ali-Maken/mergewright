# 30h: split six files that grew past ~400 lines (PROMPT.md 6.2)

Status: ready
Build with: Sonnet. Pure moves; no behaviour change.
Needs: nothing new.
Blocked by: 16b, 10b, 21, 09 (all merged), 21c (holds graft-ui `history.rs`) (they hold or may touch these files). Conductor dispatches after they merge.
Scout: none. The review names the seams (`docs/reviews/m7-01a-to-20o.md` Major 3). Read `ARCHITECTURE.md` and use
`codebase-design` (allowed: moving modules).

Files: the six files below, their new children, the `mod` lines, each crate's MAP.md.

| File | Lines | Move |
|---|---|---|
| `crates/graft-ops/src/preview.rs` | 950 | `plan_as_run`, `checked`, `fold` → `preview/as_run.rs` |
| `crates/graft-ops/src/restore.rs` | 608 | stop shapes + `outcome_of` → `restore/outcome.rs` |
| `crates/graft-ops/src/preview/kinds.rs` | 639 | builder picks the seam; say it in NOTES |
| `crates/graft-ui/src/detail.rs` | 791 | compare types → `detail/compare.rs` |
| `crates/graft-git/src/status.rs` | 589 | `submodule_status`, `submodules_not_set_up` → `status/submodule.rs` |
| `crates/graft-git/src/history_reads.rs` | 586 | builder picks the seam; say it in NOTES |
| `crates/graft/src/update/place/history.rs` | 715 | builder picks the seam (review m7-16b-to-21b Major 3) |
| `crates/graft-ui/src/history.rs` | 661 | builder picks the seam (review m7-16b-to-21b Major 3) |

Line counts are at 112a5c47; re-measure first — later merges may have moved them.

## Acceptance

- [ ] Each file ≤ ~400 lines after the move, or a gap row naming it and why not. List the before/after counts.
- [ ] Public paths unchanged (re-exports where needed): `cargo doc` and every caller compile with no edit outside
      the six files' crates, or NOTES lists each edit.
- [ ] No test changed except `use` paths. Test count before = after (say both).
- [ ] MAP.md for each crate. Gates workspace-wide.

## Touches (check each, say how)

Every caller of a moved item (grep). Doc links in MAP.md/ARCHITECTURE.md naming the old file. Stale `file:line`
citations in KNOWN_GAPS rows naming these files — list them for the conductor, don't edit KNOWN_GAPS.

## NOTES (builder)
