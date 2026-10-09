# health — split graft-ops `operation.rs` (1912 lines) by seam, move-only

Status: draft (planner, from `docs/reviews/m8-architecture.md` § KNOWN_GAPS disposition)

Build with: **Sonnet** (conductor, after scout `scouts/34-health-split-operation.md`). The planner expects **Sonnet**. The work is move-only, and `nextest list` plus the line-set check judge it.

Needs:
Blocked by: none (conductor, from Needs)
- 27 and `health-ops-surface-narrowed` merged, because both edit this file.
- `health-file-size-ratchet` merged, if it has landed, since this ticket edits its list.

## Read first
- This file.
- `crates/graft-ops/MAP.md`: grep `operation`.
- `.scratch/m8-polish-and-ship/issues/21b-hygiene-round-2.md` § "Move-only check", steps 1–6. That is the check, with step 5's target below.
- 21b-7 ruling 3 (`21b-7-moves-remainder.md`:26–30). The stop rule copies it.
- Then the scout file.

## Scout (≤1 page)
- `operation.rs`'s top-level items: `fn`/`impl`/`enum`/`struct`/`mod` lines with line numbers.
- The inline `#[cfg(test)]` span.
- 2–5 seams where the items fall into groups that call each other. Candidate groups: the `Operation` enum and `check`; per-kind execution; facts and targets; words.
- Each group's line count.

## Scope
In:
- Make `operation/` children, one per seam. `operation.rs` keeps the enum, its re-exports and the module doc.
- Inline tests move out first, to `operation/tests.rs`.
- Every public path stays the same, through re-exports.
- Update `over_400.txt` rows.
- The MAP gets one line per new child file, saying what it holds.

Out:
- Renaming anything, changing visibility beyond what a move forces (and NOTES lists each forced change), and logic changes.

**Target.** `operation.rs` and each child are at or under 400 lines. If a group can't get under 400 without cutting a fn body, that's a **stop for that group**: give the count and the seam you'd need. Don't cut fns.

## Signatures
None change. Public paths `graft_ops::operation::*` are unchanged.

## Acceptance
- [ ] The move-only check from 21b steps 1–6 passes. The line-set diff is empty apart from `mod`/`use` lines, and NOTES lists those lines.
- [ ] `cargo nextest list --workspace | wc -l` is the same before and after; both numbers go in NOTES.
- [ ] `wc -l` of `operation.rs` and each child is in NOTES, each ≤ 400 or listed as a stop.
- [ ] `file_size::every_file_over_400_lines_is_listed_with_its_count` passes with the rows lowered or removed.

## Touches
- graft-ops `operation*`
- the ops MAP
- `over_400.txt`
- the app's `use graft_ops::operation::…` lines: unchanged, and the gates prove it

No behaviour, keymap or capture changes.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 55k tokens.

## NOTES (builder, 2026-10-09)
Status: **partial — blocked: needs graft-ui** (one allow-list path, see below). Scout used: `scouts/34-health-split-operation.md`. No compaction.

Split (commit 13858647, then `merge main` for ticket 33): `operation.rs` 183, `operation/types.rs` 247, `operation/refusal.rs` **422 (stop: one item, the `Refusal` enum; would need splitting the enum itself)**, `operation/explain.rs` 341, `operation/words.rs` 183, `operation/queries.rs` 317, `operation/check.rs` 277 (`wc -l`). Inline tests were already in `operation/tests.rs`.

- [x] Move-only line-set check (multiset of trimmed lines, orig vs all seven files): only diffs are 6 `mod` lines, `pub use refusal::Refusal; pub use types::*; pub use words::RefusalWords; pub(crate) use words::{opens_in_progress, runs_in_progress};`, private `use types::amends_at_edit_stop; use words::{commits_them, dashed, in_the_way_rungs, in_the_way_words, name_paths, ref_words};`, each child's `//!` line + `use super::{…}` list, the closing/opening `impl Operation {`/`}` around the queries/check halves, and fmt reflowing `and_stash_if_set_aside`'s signature.
- Forced visibility (private → `pub(super)`): `Invocation::new`, `Invocation::with_stdin`, `amends_at_edit_stop`, `in_the_way_rungs`, `in_the_way_words`, `commits_them`, `dashed`, `ref_words`, `name_paths`, `Operation::stash_ref_if_set_aside`, `Operation::and_stash_if_set_aside`.
- [x] `nextest list --workspace`: 6219 before (pre-merge) / 6222 after; sorted diff = only ticket 33's three `file_size::*` tests.
- [x] `over_400.txt`: `operation.rs` row removed, `422 …/operation/refusal.rs` added; `file_size::every_file_over_400_lines_is_listed_with_its_count` passed in the nextest run.
- Gates: fmt ok, clippy -D warnings ok, test --doc ok, nextest 6221/6222 — **fails `graft-ui::no_literal_keys no_user_facing_literal_spells_a_default_key`**: its allow list `crates/graft-ui/tests/no_literal_keys/prose.rs:155,160` keys two `Refusal::explain` strings by `"graft-ops:operation.rs"`; they now live in `operation/explain.rs`. Fix: change both to `"graft-ops:operation/explain.rs"`. Outside my crate set, not edited.
