# 65 — review fix for wave 2 (Major M1, M2, M3; Minor m1, m4, m6, m8, m9)

Status: ready
Crates: graft-ui, graft (src and tests) + docs
Source: `docs/reviews/m6-look-finish-wave2.md` M1, M2, M3, m1, m4, m6, m8, m9.
Build with: builder-opus
Needs: 63 merged. 63 edits graft tests and captures in the same files.
Scout: none needed. The review names every line; re-check each against main after 63 lands.

## Decisions (conductor, 2026-09-25)
- **M2: the built landing stands.** After `space` on the last `CONFLICTS` row, the pane keeps the keyboard. Since 52, `c y` finishes from the pane, so `⇥` to the banner would be a wasted key. The note is already corrected (`docs/design-notes/m6-conflicts.md`, "The last file done outside the resolver"). Only the test name is left to fix.
- **M3: fix the cause, not the tolerance.** The settle waits for the radar's answer. The check does not learn to accept a missing `ⓘ`.

## Acceptance
- [ ] **M3:** `capture_fix_c_live_frames` (`crates/graft/tests/it/resolver/shell_begun.rs:1327-1359`): `bisect_frame`'s settle also waits for the radar's answer to land. Pin which state field, and name it in NOTES.
  - Seen red first: run alone, today it fails 5 of 5.
  - Then run it alone 5 times: 5 of 5 pass. Report the counts.
  - Retake `fix-c-live-frames-187x50` if it changes.
  - The heavy tier passes: `cargo test --features heavy-tests`, summary line only.
- [ ] **M1:** four files back under ~400 lines, pure moves, existing tests unedited and green (name the module tests); MAP.md updated.
  - `finish_summary.rs`: the frame and border to `finish_summary/frame.rs`, the list rows and `tints` to `finish_summary/list.rs`.
  - `conflict_resolver.rs`: `FinishRow` and `finish_row_at` to `conflict_resolver/finish_row.rs`.
  - `hints/lists.rs`: the finish and done lists to `lists/resolver.rs`.
  - `update/resolver.rs`: `cancel_owed_finish` goes next to `finish_if_owed` in `resolved.rs`.
- [ ] **M2:** rename `opens_summary.rs:142` to what it pins, e.g. `space_on_the_last_conflicts_row_opens_no_summary_and_keeps_the_pane`. Assertions unchanged.
- [ ] **m1:** pane or graph `c` mid-rebase while a write runs gets one refusal in the summary's words: "a write is already running". Rule 3: never silent. Test red first.
- [ ] **m4:** note `:916` `editor-to-finish` matches the journey-counts fence value. Doc edit: `journey_counts_match_the_note` stays green.
- [ ] **m6:** one word for `⌃E` in both subject fields (ADR-0044 §3.4). If `esc` stays different (cancel vs back), say why in the note. Test: the words pinned in both footers.
- [ ] **m8:** keymap §2.3c: "the hint bar leads with `y <verb>`" (the box has no footer). Doc edit: no test.
- [ ] **m9:** the summary's footer builds its spans from `footer_hints(..)`: the first `Hint`'s key strong, the rest muted. No splitting of rendered text. Test on the styled spans; colour **unverified**.
- [ ] Captures retaken where the output changed. Rebuild the m6 sheet; each part ≤ 200 000 B. No machine paths anywhere, NOTES included.

## Touches (check each, report how)
- Every caller of the moved items: the build proves them.
- Captures of the summary footer and both subject editors: `grep -rl` them in `docs/shots`.
- The walk tests that read those footers.
- MAP.md for graft-ui and graft.
- KNOWN_GAPS is the conductor's; don't edit it.

## NOTES
