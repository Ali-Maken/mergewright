# 30m: split the app files that grew again (review 10..07c Major 4)

Status: ready
Build with: Sonnet. Pure moves; no behaviour change (as 30h).
Blocked by: 07d, 08-b, 24-b, 30l (all edit these files). Dispatch right before 26.
Scout: none. Seams from the review (`docs/reviews/m7-10-to-07c.md` Major 4). Read ARCHITECTURE.md; use `codebase-design`.
Files: graft `src/update/banner.rs` (→ `banner/rewrite.rs`, `banner/continue_in_git.rs`), `update/banner/stop.rs`,
`update/preview.rs` (→ `preview/take.rs` …), `update/commit_editor.rs` (edit-stop amend → child), `update/operate.rs`;
their new children, `mod` lines, MAP.md. Re-measure each at dispatch; add any other file over ~400 that grew in M7.

## Acceptance
- [ ] Each file ≤ ~400 after, or a gap row naming why not; before/after counts.
- [ ] Public paths unchanged; no caller edited, or NOTES lists each.
- [ ] `#[test]` count before = after.
- [ ] MAP.md each crate; ARCHITECTURE.md lines re-pointed; gates workspace-wide.

## NOTES (builder)
