# acceptance-stash-put-back: walk picks 15–17, count every journey against the note, captures, TESTING.md

Status: ready
Blocked by: 67, 71, 72, 73, 74 (and 68–70 for the bottom row and tab words)
From: the acceptance update for picks 15–17 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24-26). Promises: M6
"a real conflict is resolvable without leaving the app" (now a stash put back by hand or by Graft); M3 "you cannot lose
work" (edits set aside are in the tree or the stash list at every instant).
Build with: **Fable** — the acceptance ticket (judges the pass against the note; does not count against the Fable build
expectation).
Needs: 67, `checkout-and-cherry-pick-lead-the-menu`, `stash-clash-lands-on-the-resolver`,
`set-aside-around-rebase-and-switch`, `set-aside-shows-in-the-preview-and-lands`, all merged; **and** the conductor's
doc rows (design note "Where I'd differ"): the ten journey rows in `docs/design-notes/m6-conflicts.md`'s
`journey-counts` fence, keymap §2.1/§2.6/§2.9, the `DirtyTree` doc line, KNOWN_GAPS rows (staged-after-clash,
fast-forward pull over overlapping edits, row 50).
Crates: graft (tests only), scripts, docs (TESTING.md, captions TSV). A behaviour defect found here is filed with its
line (rule 7), not fixed.
Scout: none; ticket 43's NOTES and 63's NOTES describe the harness and the sheet script.

## Read first
- `docs/design-notes/m6-stash-put-back.md` "Journeys", "Where each event lands him", "New UI".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §5 (the survival table), §6.
- Ticket 63 "Scope" and NOTES (the harness: `m5_walk::every_scenario`, `journey_counts_match_the_note`, the pty walk,
  `scripts/contact-sheet.sh`).
- NOTES of every ticket in Needs (captures and TESTING.md rows each listed for this ticket).
- TESTING.md "Times I left" (:87-117) and "Conflict journeys", "Captures" (:2284-2386, lines as they stand).

## Scope
**In:**
1. **Every journey as a scenario** in `m5_walk::every_scenario::journey`, keyed by the note's ids:
   `stash-pop-clash-to-done` 4, `stash-apply-clash-to-done` 4, `stash-clash-put-back` 2, `pull-rebase-dirty` 3,
   `rejected-push-dirty-to-pushed` 5, `switch-dirty` 3, `switch-dirty-clash` 5, `pull-rebase-stop-then-clash` 8,
   `menu-checkout` 3, `menu-cherry-pick` 4. Each asserts git's truth at the end (stash list, `git status`, branch,
   `git log origin/main` where it pushes), not the screen alone. The group walks in `tests/it/m6_stash_put_back.rs` stay;
   these are the counted ones.
2. **The survival check, end to end:** `m6_stash_put_back::edits_survive_a_kill_at_every_step` — the app driven through
   `. ⏎ y` on a dirty tree, killed (the harness's stop) before each of ADR §4's steps 1–6, relaunched: the edits are in
   the tree or in `git stash list`, and the relaunch line names `stash@{n}` when they are in the list.
3. **One command:** `cargo nextest run -E 'test(m5_walk::every_scenario::journey) | test(journey_counts_match_the_note) |
   test(m6_stash_put_back::)'` green, every journey at or under its row. The route table to
   `.scratch/m6-conflicts/acceptance-stash-put-back-routes.txt`; read its summary line only (context guard).
4. **Captures** through the pty walk (`.scratch/m6-conflicts/pty-walk-m6.py`), 120x40, 187x50 and 80x24:
   `m6-20-stash-clash-landing` (`S p`), `m6-21-stash-clash-done` (`S p o ⏎`), `m6-22-set-aside-preview` (`p r` on a
   dirty tree), `m6-23-set-aside-clash-landing` (`. ⏎ y` into a clash), `m6-24-set-aside-done` (`p r r`),
   `m6-25-menu-fixed-slots` (`.` on a branch row). Matrix and captions TSV updated
   (`captures_are_named_keyed_and_listed_in_the_matrix`).
5. **Contact sheet:** `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`, parts ≤ 200 000 B, this pass's
   captures in spec order (67's `fix-rebase-refused-before-preview` is gone: F2 deleted it).
6. **TESTING.md**, every line concrete:
   - "Conflict journeys": one row per journey above — start, keys, count, landing words, git truth. Landing words are the
     note's, e.g. `stash-pop-clash-to-done`: resolver on `notes.md`, notice `stash@{0} clashed in 1 file · esc, u puts
     it all back`; after `o ⏎`, bottom row `every file done · stash@{0} dropped ✓ · u brings it back`, `git stash list`
     empty.
   - "Captures": what right looks like for m6-20 … m6-25 (the words each spec's capture line gives).
   - "Times I left": the stash-pop-clash row (N3, 11 keys) → closed, naming `stash-pop-clash-to-done`; the dirty-pull row
     (N4, 16 keys) → closed, naming `rejected-push-dirty-to-pushed`.
   - The colour paragraph: dim menu slots and resolver roles are asserted in buffer tests (named); a capture carries no
     colour; **unverified** until he looks.
   - Every row the Needs tickets' NOTES listed (`every file done` for a pop, `later milestone`, menu positions,
     `preview failed:` for a refusal) rewritten.
7. **Perf:** the release perf run from its script to `docs/perf/acceptance-stash-put-back.txt`; summary lines only. The
   preview gains a row and `check`: the preview frame and the menu open stay inside budget (ADR-0021).

**Out:** any behaviour fix; M7's checkout/cherry-pick keys and picker; the whole-app usability walk (the reviewer's,
after this).

## Acceptance (each tick names its test, at tick time)
- [ ] `m5_walk::every_scenario::journey` (the ten new scenarios among all) and `journey_counts_match_the_note` green;
      route table path; none over.
- [ ] `m6_stash_put_back::edits_survive_a_kill_at_every_step` green; every step 1–6 covered (listed).
- [ ] Captures m6-20 … m6-25 at three sizes, each ≤ 200 KB, listed in the matrix.
- [ ] Contact sheet part paths and sizes, each ≤ 200 000 B.
- [ ] TESTING.md rows as above; the build's counts equal the note's (the parser holds them).
- [ ] Perf summary lines, inside budget, with the file path.
- [ ] NOTES: defects found, each with its line or as a question (rule 7); what was not verified (colour, rule 5).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. Context guard: captures, route tables and perf output go to files; the report names paths only.

## Size
Medium (~60k): ten scenarios on the existing harness, one kill walk, six captures at three sizes, TESTING.md.
