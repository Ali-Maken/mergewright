# 30m: split the app files that grew again (review 10..07c Major 4)

Status: done
Build with: Sonnet. Pure moves; no behaviour change (as 30h).
Blocked by: 07d, 08-b, 24-b, 30l, 30n (all edit these files). Dispatch right before 26.
Scout: none. Seams from the review (`docs/reviews/m7-10-to-07c.md` Major 4). Read ARCHITECTURE.md; use `codebase-design`.
Files: graft `src/update/banner.rs` (→ `banner/rewrite.rs`, `banner/continue_in_git.rs`), `update/banner/stop.rs`,
`update/preview.rs` (→ `preview/take.rs` …), `update/commit_editor.rs` (edit-stop amend → child), `update/operate.rs`;
graft-ops `src/preview.rs` (430 after 30l); their new children, `mod` lines, MAP.md. Re-measure each at dispatch; add any other file over ~400 that grew in M7.

## Acceptance
- [x] Each file ≤ ~400 after; before/after counts in NOTES (`wc -l`). Largest now: `update/preview.rs` 423, `operate/outcome.rs` 420, `banner/resume.rs` 405.
- [x] Public paths unchanged — every path a caller uses re-exported from the old module (`cargo check --all-targets` clean, no caller in src edited). Five unused crate-internal re-exports dropped and two test imports re-pointed: listed in NOTES.
- [x] `#[test]` count before = after: 4787 = 4787 (`grep -r '#\[test\]' crates | wc -l`).
- [x] MAP.md (graft, graft-ui, graft-ops "File layout (30m)" sections); ARCHITECTURE.md:150 re-pointed to `preview/take.rs`; gates workspace-wide: fmt ok, clippy -D warnings ok, nextest 4879/4879 (5 skipped), test --doc ok.

## Conductor note 2026-09-29 (review m7-08-to-rebase-keeps-drops, M2 + M3)
Added to scope, same rules (split along seams, public paths unchanged, tests moved not rewritten):
- M2 (review :123-137): graft-ui `rewrite_editor.rs` (939), graft `update/rewrite_editor.rs` (716), `view/rewrite_editor.rs` (420), `update/rewrite_editor/keys.rs` (413); graft-ops `rewrite/plan.rs` inline tests (:336) → `rewrite/plan/tests.rs`.
- M3 (review :139-160): the six-field tuple `Stop` in `update/operate.rs` (~:1017) → a named struct with a distinct name (review suggests `ConflictedAnswer`); orphan doc (~:842) back onto `incomplete`; rename `by_name` (`update/rewrite_editor.rs` ~:104) per the review. Re-grep lines; they moved.
- Acceptance adds: [x] every file above ≤ ~400 non-test lines (`wc -l`, NOTES; plan.rs 571 → 337, tests to `rewrite/plan/tests.rs`); [x] M3's three items:
  - [x] `Stop` tuple → `ConflictedAnswer { description, kind, paths, stalled, entry, rewrite }` (`operate/outcome.rs`; `grep -n 'struct Stop(' crates/graft/src/update/operate*` → none).
  - [x] `incomplete`'s doc moved back onto `fn incomplete` (`operate/outcome.rs`; `incomplete_answered` has one paragraph).
  - [x] `by_name` → `marked_by`, `keys.rs` comment reworded (`grep -rn '\bby_name' crates/graft/src` → none).
- [x] Scope addition (review m7-24-and-30n M2): `update/journal.rs` 814 → 329 (+ `answers.rs` 164, `keys.rs` 348); graft-ui `journal_screen.rs` 1087 (727 non-test) → 398 (+ `view.rs` 145, `pieces.rs` 210, tests → `tests.rs`); `view/journal.rs` 775 (456 non-test) → 323 (+ `summary.rs` 141, tests → `tests.rs`). Same `#[test]` count above.

## NOTES (builder)

No scout: pure moves along seams the review named; each seam read from the file's item list (`grep -n '^fn\|^pub'`).

Before → after (lines, root file + children):
- `update/operate.rs` 2999 (1505 non-test) → 396 + `diff_cursor` 348, `stage_discard` 224, `outcome` 420, `failure` 221; tests → `operate/tests.rs`.
- `update/banner.rs` 1150 → 367 + `focus_keys` 180, `pressed` 241, `resume` 405.
- `update/banner/stop.rs` 630 → 336 + `stop/wording` 218, `stop/tab` 88.
- `update/preview.rs` 1784 (1620 non-test) → 423 + `keys` 223, `answer` 361, `take` 118, `boxes` 165, `plan_lines` 341, `rect` 78, `history` 173.
- `update/commit_editor.rs` 900 → 401 + `refusal` 165, `edit_stop` 55, `keys` 302.
- graft-ops `preview.rs` 430 → 197 + `effects` 246.
- graft-ui `rewrite_editor.rs` 939 → 91 + `view` 157, `layout` 219, `draw` 285, `right` 224.
- `update/rewrite_editor.rs` 747 → 358 + `open` 169, `routing` 161; facts types into `words.rs` (256). `rewrite_editor/keys.rs` 413 → 293 + `keys/field` 131.
- `view/rewrite_editor.rs` 420 → 178 + `right` 252.
- graft-ops `rewrite/plan.rs` 571 → 337 + `plan/tests.rs`.

Not pure moves (kept to what a gate forced):
- `operated` crossed clippy `too_many_lines` (61/60) once `ConflictedAnswer` got named fields. Its `Done` and `StagingNotKept` arms moved to `done_answered` / `staging_not_kept_answered`, and the clash-drop take to `clash_drop_of` — same statements, same order; the take still runs before the match for every outcome. Verified by the existing operate tests only (nextest green), not by new ones.
- Children use explicit `use super::{…}` lists (clippy `wildcard_imports`, applied by `clippy --fix`).

Re-exports dropped (no caller used them; the lint rejects dead re-exports; items unchanged at their child path): `operate::NotStaged`, `operate::staging_not_kept_rungs` (test `operate/tests/staging_not_kept.rs` import re-pointed), `preview::keeps_drops`, `rewrite_editor::MessageFacts`. Kept under `#[cfg(test)]` for tests: `banner::{kept_open, resumed_words}`, `preview::confirm_key`, `journal::{journal_enter, switched_from}`. `operate/tests.rs` imports `stage_discard::{NotStaged, untracked_size}`.

Pin re-pointed: `crates/graft/tests/it/stop_ladder_readers.rs` lists `banner/stop/wording.rs` (the ladder's definition moved there). Failed once in nextest before re-pointing; that is the only test that changed.

Not done (outside the listed set): review m7-08 table's `update/keys.rs` 438 and graft-ui `graph_screen/hints/lists.rs` 485 — not in this ticket's or the conductor note's file list.
