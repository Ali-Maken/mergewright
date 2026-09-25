# 64 — review fix for 54–58 (Major M1, Major M2; Minor m1, m2, m3, m4, m8)

Status: ready
Crates: graft-ui, graft, graft-ops + docs
Source: `docs/reviews/m6-look-finish-54-58.md` M1, M2, m1, m2, m3, m4, m8.
Build with: builder-opus
Needs: 59 merged (same crates, graft-ui and graft). Runs before 60 (60 adds `c finish` to the done list; m8 must land first).
Scout: none needed. The review names every line; re-check each against main at build time, since 59 lands first.

## Decisions (conductor, 2026-09-25)
- **M1 → option (a): the order ADR-0044 §4 states** (`docs/adr/0044-conflict-colours-and-the-confirm-key.md:96-99`):
  1. Every hint placed at its bare key first.
  2. Then the marked (`keeps_words`) hints raised to `Full`.
  3. Then the rest raised, in the existing fixed sequence.

  Why: the ADR is the approved text, and the user's pick 5 was "drop words from everyday keys first". The code follows the ticket's reading, and at 80 columns it hides `j/k` and `M` outright, which loses visibility (`docs/research/ux-principles.md`). No ADR edit is needed. Monotonicity must still hold.

## Acceptance
- [ ] **M1:** at 80 columns the resolver bar shows every key, bare at least. `j/k` and `M` are present in `docs/shots/m6/m6-13-after-ctrl-t-80x24.txt`, retaken. Assert this in a test, and see the test red first against today's order.
- [ ] **M1:** at 120 columns the bar still keeps `d start over` and `esc close` (56's test stays green). Name it.
- [ ] **M1:** the monotonicity test still holds (name it).
- [ ] **M2:** `crates/graft-ui/src/theme/mod.rs` ≤ ~400 lines: the conflict-token table plus `pinned`/`tint` go to `theme/conflict.rs`. `conflict_resolver/files.rs` ≤ ~400: the words split (`WordsTone`, the cell's two-style draw) goes to `files/words.rs`. Pure moves; the existing tests stay green (name the module tests). MAP.md updated.
- [ ] **m1:** fix the comment at `crates/graft/src/update/banner.rs:376-378` to say what the code does in a shell merge. Behaviour stays unchanged; that question is in KNOWN_GAPS M6/57. Doc edit: no test.
- [ ] **m2:** `NotInvertible::OperationLive` gets one wording function, called by both `graft-ops/src/preview.rs:715` and `undo/execute.rs:231`. `operation_live_words_match_ops` (or a new test) covers both call sites.
- [ ] **m3:** word the doc comments at `crates/graft/src/update/resolver/restart.rs:17` and `:27` by kind (markers vs keep/delete). Doc edit: no test.
- [ ] **m4:** add TESTING.md journey rows for `b-abort-shell-merge` (3) and `b-undo-a-take-in-the-shell` (1), next to `walk-back-a-whole-file-choice` (~:2333). Give each its start, keys, count and "what right looks like". `journey_counts_match_the_note` stays green.
- [ ] **m8:** `graph_screen/hints/lists.rs:346` → `hint("d", "start over").keeps_words()`. Test: the done list's `d` is marked.
- [ ] Captures retaken wherever the bar changed. Rebuild the contact sheet with `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`; each part must be ≤ 200 000 B. Update the captions line for m6-13 80x24 (`m6-contact-sheet-captions.tsv:87`). Colour is **unverified**.

## Touches (check each, report how)
- Every capture showing the resolver bar at 80 or 100: `grep -rl` the old bar words in `docs/shots/m6`.
- The finish and done hint lists: 59 changed them. Re-read them after the merge.
- The walk tests that read the bar.
- MAP.md for graft-ui theme and files.
- KNOWN_GAPS is the conductor's; don't edit it.

## NOTES
