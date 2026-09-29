# 30m: split the app files that grew again (review 10..07c Major 4)

Status: ready
Build with: Sonnet. Pure moves; no behaviour change (as 30h).
Blocked by: 07d, 08-b, 24-b, 30l, 30n (all edit these files). Dispatch right before 26.
Scout: none. Seams from the review (`docs/reviews/m7-10-to-07c.md` Major 4). Read ARCHITECTURE.md; use `codebase-design`.
Files: graft `src/update/banner.rs` (→ `banner/rewrite.rs`, `banner/continue_in_git.rs`), `update/banner/stop.rs`,
`update/preview.rs` (→ `preview/take.rs` …), `update/commit_editor.rs` (edit-stop amend → child), `update/operate.rs`;
graft-ops `src/preview.rs` (430 after 30l); their new children, `mod` lines, MAP.md. Re-measure each at dispatch; add any other file over ~400 that grew in M7.

## Acceptance
- [ ] Each file ≤ ~400 after, or a gap row naming why not; before/after counts.
- [ ] Public paths unchanged; no caller edited, or NOTES lists each.
- [ ] `#[test]` count before = after.
- [ ] MAP.md each crate; ARCHITECTURE.md lines re-pointed; gates workspace-wide.

## Conductor note 2026-09-29 (review m7-08-to-rebase-keeps-drops, M2 + M3)
Added to scope, same rules (split along seams, public paths unchanged, tests moved not rewritten):
- M2 (review :123-137): graft-ui `rewrite_editor.rs` (939), graft `update/rewrite_editor.rs` (716), `view/rewrite_editor.rs` (420), `update/rewrite_editor/keys.rs` (413); graft-ops `rewrite/plan.rs` inline tests (:336) → `rewrite/plan/tests.rs`.
- M3 (review :139-160): the six-field tuple `Stop` in `update/operate.rs` (~:1017) → a named struct with a distinct name (review suggests `ConflictedAnswer`); orphan doc (~:842) back onto `incomplete`; rename `by_name` (`update/rewrite_editor.rs` ~:104) per the review. Re-grep lines; they moved.
- Acceptance adds: [ ] every file above ≤ ~400 non-test lines; [ ] M3's three items, each naming the test or grep that shows it.

## NOTES (builder)
