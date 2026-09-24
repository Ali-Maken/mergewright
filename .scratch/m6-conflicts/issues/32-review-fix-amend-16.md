# review-fix-amend-16: stale footer after a queued bulk take; the combined `⌃O/⌃T all` rung; bar words lost at 120; two over-long files

Status: done
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
- [x] M1: failing test first, red on unchanged code — test: `update::resolver::tests::take_all::footer_count_follows_a_queued_bulk_take` (observed red before the fix: status stayed `Done("… 2 of 3 open")`)
- [x] M1: footer, header and notice agree after a queued take then `Ctrl+T` — test: `…::footer_count_follows_a_queued_bulk_take` (header `open() == 0`, status `Ready`, notice `in 2 blocks`) and `…::ctrl_t_waits_behind_a_queued_take` (status `Ready` asserted)
- [x] M2 + glyph: combined `⌃O/⌃T all` rung; user table test holds only P3's rows; 16's box corrected — test: `graft-ui graph_screen::hints::tests::resolver_offers_take_all`; six `all` rows removed from `every_short_rung_is_the_users_own_word`; bar key pressed by `update::tests::every_key_the_bar_draws_is_answered` (`⌃O/⌃T` → Ctrl+O, Ctrl+T)
- [ ] Bar at 120: `d start over` and `esc close` in words on the merge and rebase resolver bars — **not met, stopped per scope 4** (column counts in NOTES). Partial: without `B`/`d`, all words fit at 120 — `hints::tests::resolver_offers_file_keys` (`bar(120)`)
- [x] M3: splits, line counts in NOTES, MAP.md — gates green (pure moves; nextest 2906/2906)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest (2906 passed, 1 skipped), doctests — all green

## NOTES

- No scout: the review names every line.
- M1 fix (cause): an `Every` landing now resets a standing `Status::Done` to `Ready` (`update/resolver/answers.rs`, `take_landed`). Footer goes back to the hints; the notice carries `took <side> in n blocks`. Chose reset over a second `k of m open` line to avoid saying the same thing twice.
- M2: one `hint("⌃O/⌃T", "all")` (`hints/lists.rs`); `all_of` and the six `all …` short-form rows gone; `BOUND_ACTIONS` has one row. Role words dropped from the bulk hint — `o`/`t` just before name them; matches mockups `01-view-mockups.md:173`, note `:661`. Note `:692` draws `^T all theirs` instead — the two mockups disagree; conductor may want to reconcile.
- Glyph: design docs still spell `^O/^T` (`01-view-mockups.md:173`, note `:661`, `:692`); not edited (docs outside ticket scope).
- Scope 4 column counts (full words): merge bar with `B base` + `d start over` = 135 cells; rebase = 137; without `B`/`d` = 113 (fits). At 120 with `B`+`d`, the ladder draws `… ⏎ done  j/k file  M mergetool  d  esc`. Cannot fit without reordering or cutting; stopped, ladder untouched.
- At 80 the bar now reads `… ⌃O/⌃T all  O/T file  e edit  n  N  ⏎  j/k  M  esc` (gained `O/T file`, `e edit`).
- Captures regenerated (look is the user's): `docs/shots/m6/` `fix-card-names-numbers-120x40`, `-80x24`, `fix-file-keys-before-120x40`, `-200x50`, `fix-file-list-many-120x40`, `fix-file-list-rebase-step2-120x40`, `fix-stop-lands-in-resolver-120x40`, `-187x50`, `-80x24`, `fix-take-all-120x40`.
- M3 line counts: `update/keys.rs` 859 → 446 (+ `keys/bound.rs` 136, `keys/predicates.rs` 153, `keys/actions.rs` 79, `keys/view_change.rs` 76); `update/events.rs` 684 → 421 (+ `events/command.rs` 276). `Product` enum alone is ~180 lines, so events.rs stays just over 400 without splitting an enum.
