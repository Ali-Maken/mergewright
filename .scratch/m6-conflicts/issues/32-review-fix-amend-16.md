# review-fix-amend-16: stale footer after a queued bulk take; the combined `⌃O/⌃T all` rung; bar words lost at 120; two over-long files

Status: ready
Blocked by: 17-resolver-file-keys.md (building; same crates, ADR-0036). Runs before 19.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-16.md` Major 1, 2, 3 and the glyph Minor. The capture finding (bar words lost at 120) is folded in because the note's mockups draw them in full at 120 (`docs/design-notes/m6-conflicts.md:164`, `:692`).
Build with: **Opus** (low).
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-16.md`: Major 1–3, the glyph Minor, "Captures".
- `docs/design/02-keymap.md` §2.6 (`Ctrl+O`/`Ctrl+T`), P3 in `docs/field-notes.md` / the note (the combined short form).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md` (`graph_screen::hints`); `codebase-design` for the splits.

## Scope
In:
1. **Major 1.** After a queued `o` then `Ctrl+T`, the footer keeps `2 of 3 open ✓` until a key (`update/resolver/answers.rs:309`, cleared only at `update/keys.rs:83`), while notice and header say 0 open. Failing test first. Fix at the cause: the footer reads the same count as the header when the bulk take lands.
2. **Major 2.** Ticket 16's box at `:127` is ticked but the short rung built is `^O all  ^T all`, not the combined `⌃O/⌃T all` P3 approved; six `all` rows were added to the user's short-word table test (`graft-ui/.../hints/tests.rs:465`) outside P3. Build the combined rung; remove the rows P3 did not approve; correct 16's box to name the test that checks it.
3. **Glyph.** On-screen convention is `⌃` (`hints.rs:75`, `keys.rs:725`, `lists.rs:241`). The resolver's take-all hints use `⌃`.
4. **Bar at 120.** With 1–3 in, the 120-col resolver bars keep `d start over` and `esc close` in words, as the note draws them (`:164`, `:692`). If they still cannot fit, say so with the column count and stop — do not reorder the ladder on your own. Regenerate the affected captures; list them in NOTES. Look is the user's.
5. **Major 3.** `update/keys.rs` (856) and `update/events.rs` (684): split, pure moves, ~400 per file where a seam exists; MAP.md updated; no seam → say so with the count.

Out: review-16's other Minors (→ KNOWN_GAPS).

## Acceptance (each tick names its test, at tick time)
- [ ] M1: failing test first, red on unchanged code — test:
- [ ] M1: footer, header and notice agree after a queued take then `Ctrl+T` — test:
- [ ] M2 + glyph: combined `⌃O/⌃T all` rung; user table test holds only P3's rows; 16's box corrected — test:
- [ ] Bar at 120: `d start over` and `esc close` in words on the merge and rebase resolver bars — test (capture pin):
- [ ] M3: splits, line counts in NOTES, MAP.md — gates green
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES
