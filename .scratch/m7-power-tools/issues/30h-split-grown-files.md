# 30h: split six files that grew past ~400 lines (PROMPT.md 6.2)

Status: done
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

- [x] Each file ≤ ~400 lines after the move, or a gap row naming it and why not. List the before/after counts.
      Verified by `wc -l` (see NOTES).
- [x] Public paths unchanged (re-exports where needed): `cargo doc` and every caller compile with no edit outside
      the six files' crates, or NOTES lists each edit. Verified by `cargo clippy --workspace --all-targets -D warnings`
      and `cargo doc --workspace --no-deps`, both clean of new errors; no caller file edited.
- [x] No test changed except `use` paths. Test count before = after (say both). Verified by `git grep -c '#[test]'`
      4485 = 4485; nextest 4579 passed / 5 skipped.
- [x] MAP.md for each crate. Gates workspace-wide. fmt, clippy, nextest, `cargo test --doc` (3/3) all pass.

## Touches (check each, say how)

Every caller of a moved item (grep). Doc links in MAP.md/ARCHITECTURE.md naming the old file. Stale `file:line`
citations in KNOWN_GAPS rows naming these files — list them for the conductor, don't edit KNOWN_GAPS.

## NOTES (builder)

Status: done.

- Sizes (before at 7c4794cc → after): graft-ops `preview.rs` 955→425, `restore.rs` 619→371, `preview/kinds.rs`
  659→271; graft-ui `detail.rs` 791→407, `history.rs` 681→377; graft-git `status.rs` 589→360, `history_reads.rs`
  586→385; graft `update/place/history.rs` 756→336. `preview.rs` 425 / `detail.rs` 407 = "~400": what remains is the
  plan / view types themselves. No gap row.
- Tests unchanged: `restore` and `detail` inline test modules moved verbatim to `restore/tests.rs`, `detail/tests.rs`
  (dedent only). Private items the preview tests reach come back into `preview.rs` via `#[cfg(test)] use` (`sha1`,
  `parse_ident`, `parse_offset`, `not_invertible`), so no test `use` line changed either.
- Pure-move check: multiset diff of removed vs added lines (visibility stripped) shows only `mod`/`use` lines, new
  module docs, rustfmt rewraps, `Replay` fields → `pub(super)`, three `super::` → `super::super::` in
  `place/history/open.rs`, and dropped `// ──` banners.
- Seams: `preview.rs` → `as_run.rs`, `ghost.rs`, `identity.rs`, `landing.rs`, `revert_to.rs`. `preview/kinds.rs` →
  `kinds/rebase.rs` (rebase, line ghosts, followers) + `kinds/refs.rs` (ref-only kinds + push). `restore.rs` →
  `restore/outcome.rs` (`Ran`, `run_plan`, `outcome_of`, failure words), `restore/ref_names.rs`. `status.rs` →
  `status/parse.rs`, `status/submodule.rs`. `history_reads.rs` → `history_reads/forecast.rs` (merge-tree forecasts).
  `detail.rs` → `detail/compare.rs`, `detail/diff.rs`, `detail/submodule.rs`. graft-ui `history.rs` →
  `history/list.rs`, `history/file_pane.rs`. graft `place/history.rs` → `history/open.rs` (open_history, `F`) and
  `history/view.rs` (second `impl HistoryState`: view/title/header + row words).
- Children use `#[allow(clippy::wildcard_imports)] use super::*;` (lint not enabled; allow inert).
- No scout: the review named the seams; files read by item index only.

For the conductor (not edited):
- Stale KNOWN_GAPS citations: line 559 `history_reads.rs:452` → `history_reads/forecast.rs`; line 857
  `preview/kinds.rs:151-155` → `preview/kinds/rebase.rs`; line 979 `graft-ops/src/preview.rs:498` →
  `preview/as_run.rs`; line 990 `preview/kinds.rs:316` → moved within `kinds.rs` (re-find).
- `ARCHITECTURE.md:42` says `restore.rs` holds the plan loop (now `restore/outcome.rs`); `ARCHITECTURE.md:95` says
  `place/history.rs` holds `open_history` (now `place/history/open.rs`). Outside my file set; not edited.
