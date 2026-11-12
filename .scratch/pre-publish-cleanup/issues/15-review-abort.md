# review screen's route to Abort, tested

Status: done
Build with: conductor-run builder (worktree `ppc-abort`), no scout (one screen, known test fixtures)
Source: KNOWN_GAPS `pp/13-review-abort-untested`.

## Acceptance
- [x] A1. The review has a route to Abort: `Esc` (review to clashes), `Esc` (clashes to banner), `a` opens the abort box. No regression from M4; no source change. Verified by `resolver::tests::review_at_y::abort_is_reachable_from_the_review_by_esc_esc_a`, which drives the real review screen (`review_over_the_resolver`) by those keys.
- [x] A2. Gap row closed to `docs/gaps/closed.md`.
- [x] A3. Gates: see report.

## NOTES
- Unverified: whether the review's key bar visibly names `Esc`; the test proves the path, not its discoverability by eye.
- Colour: no visual change.
