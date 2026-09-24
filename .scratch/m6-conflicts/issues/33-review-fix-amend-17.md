# review-fix-amend-17: a false tick on the after-`T` capture; resolver.rs and answers.rs over size

Status: done
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
- [x] C1: after-`T` frames at 120 and 200 draw the card bar; the test asserts no `O/T`, no `j/k` on it, and the strip / identity / notice bullets at both widths — test: `it::resolver::file_keys::file_keys_capture` ((b) loop over 120x40 and 200x50; both shots regenerated, TSV captions fixed, contact sheet rebuilt)
- [x] C1: ticket 17 box (b) names the tests that check each of its claims — (textual; `17-resolver-file-keys.md` Captures (b) bullet; the stale NOTES "Captures" bullet rewritten)
- [x] M1: splits, line counts in NOTES, MAP.md — gates green (pure moves; nextest 2906 passed, same suite)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2906 passed, 1 skipped), doctests — all green

## NOTES
- No scout: the review names every line.
- Sizes after merging main (32 in): `update/resolver.rs` 428 → 406 (`take_keys` → `take.rs`); `resolver/answers.rs` 412 → 387 (`settle_landed` → new `resolver/settle.rs`, 93 lines, with `settle_key` from `take.rs`); `take.rs` 296 → 264; `walk_blocks` 70 → 62 (`cross_from_done`, `no_more_conflicts` pulled out). The remaining 62 lines are one flow (resolve document or card, then rules 1/2/3 in order); no further seam without splitting that sequence. `resolver.rs` stays 6 over ~400: the rest is `ResolverState` + `act`.
- The after-`T` capture now presses `PgUp` (as `done_open_capture` does) so the card bar replaces the Stage's status line.
- Question (no line, rule 7): at 200x50 the side list draws `notes.txt` as `…tes.txt  done ✓` in a 27-column list. Test asserts the drawn text as is.
