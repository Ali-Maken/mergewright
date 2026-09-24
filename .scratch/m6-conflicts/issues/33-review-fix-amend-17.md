# review-fix-amend-17: a false tick on the after-`T` capture; resolver.rs and answers.rs over size

Status: ready
Blocked by: 32-review-fix-amend-16.md (building; same crates, ADR-0036). Runs before 19.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-17.md` Critical 1, Major 1.
Build with: **Opus** (low).
Needs: none.
Crates: graft.

## Read first
- `docs/reviews/m6-amend-17.md`: Critical 1, Major 1 — each names file and line.
- `.scratch/m6-conflicts/issues/17-resolver-file-keys.md`: the Captures box (b) and NOTES.
- `crates/graft/MAP.md` (`update::resolver`); `codebase-design` for the split.

## Scope
In:
1. **Critical 1 — false tick.** Ticket 17's Captures box (b) claims the card bar has no `O/T`/`j/k`, but neither after-`T` frame draws a bar: the bottom row is ` stage notes.txt ✓` (`docs/shots/m6/fix-file-keys-120x40.txt:40`, `-200x50.txt:50`), `file_keys_capture` asserts nothing about the bar (`tests/it/resolver/file_keys.rs:161-179`), and the 200x50 after-frame has no assertions. Fix: after landing, a key that clears the status line and changes nothing else (as 17 used `PgUp`), then assert the card bar holds no `O/T` and no `j/k` at both widths, plus the strip / identity row / notice bullets at 200x50. Regenerate the two after-frames. Then correct 17's box (b) to name the test that checks each claim.
2. **Major 1 — sizes.** `update/resolver.rs` 394 → 428, `update/resolver/answers.rs` 373 → 406, `walk_blocks` 60 → 73 lines. Split, pure moves, to ~400 per file and a plain function under ~50 lines where a seam exists; MAP.md updated; no seam → say so with the count. Note: ticket 32 lands before you and may move lines in these files — measure after merging main.

Out: review-17's Minors (→ KNOWN_GAPS).

## Acceptance (each tick names its test, at tick time)
- [ ] C1: after-`T` frames at 120 and 200 draw the card bar; the test asserts no `O/T`, no `j/k` on it, and the strip / identity / notice bullets at both widths — test:
- [ ] C1: ticket 17 box (b) names the tests that check each of its claims — (textual; line)
- [ ] M1: splits, line counts in NOTES, MAP.md — gates green
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
