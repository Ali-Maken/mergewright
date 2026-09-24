# acceptance-look-and-finish: walk the look-and-finish changes, re-take the M6 captures, rebuild the sheet, TESTING.md

Status: ready
From: the milestone's acceptance update for the 2026-09-24 picks (P11-P16, batch 2, 3, 6, 7, 8, 9, 10). Promise M6: "a
real conflict is resolvable without leaving the app".
Build with: **Fable** — the acceptance ticket (judges the whole wave against the note; does not count against the Fable
build expectation).
Needs: every other spec of this pass merged: `conflict-colour-tokens`, `conflict-band-and-state-colours`,
`hint-words-keep-the-way-out`, `undo-and-abort-a-shell-begun-merge`, `start-over-a-whole-file-choice`,
`finish-summary-is-a-preview`, `last-file-opens-the-summary`, 52, 50, 51, `resolver-words-and-marks`
(`short-words-from-the-draft` only if unblocked and built).
Crates: graft (tests only), scripts, docs. A behaviour defect found here is filed with its line (rule 7), not fixed.
Scout: none; ticket 43's NOTES and `.scratch/m6-conflicts/scout-21-harness.md` describe the harness.

## Read first
- Ticket 43 in full (the harness, the sheet script, its gotchas).
- `docs/design-notes/m6-conflicts.md`: the `journey-counts` fence as it stands after every spec's "For the conductor"
  rows; Amendment 2026-09-24 J13-J17 (E6 of `.scratch/m6-conflicts/spec-pass-edits-2026-09-24.md`) and "Where each
  event lands him" (:916-934).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` (what the buffer tests prove; what is left to his eye).
- TESTING.md :87-117 ("Times I left") and :2284-2386 ("Conflict journeys", "Captures").

## Scope
**In:**
1. **The walk, one command:** `cargo nextest run -p graft --test it m5_walk::every_scenario::journey` and
   `journey_counts_match_the_note` green, every journey at or under its row. Report the route table it prints to a file
   (`docs/perf/` is for perf; the table goes to `.scratch/m6-conflicts/acceptance-look-and-finish-routes.txt`) and read
   its summary line only (context guard).
2. **Re-take the M6 captures** `m6-10` … `m6-19` at the five sizes (120x40, 160x48, 187x50, 200x50, 80x24) through
   `.scratch/m6-conflicts/pty-walk-m6.py`, re-keyed:
   - `m6-14-last-file-done` → keys `m m o T`: the summary, opened by itself.
   - `m6-15-finish-summary` → **renamed** `m6-15-finish-row-after-esc`, keys `m m o T Esc`: the resolver with the finish
     row.
   - `m6-16-summary-expanded` → keys `m m o T ⏎`: `README.md`'s diff under its row.
   - `m6-17-after-commit` → keys `m m o T y`.
   - Every other stem re-taken as is (the band glyphs and `•` change them).
   - Update `.scratch/m6-conflicts/matrix.txt` and the captions TSV (`docs/shots/m6/m6-contact-sheet-captions.tsv`) for
     the rename; no stale stem left (`captures_are_named_keyed_and_listed_in_the_matrix`).
3. **The contact sheet:** `scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`, parts ≤ 200 000 B, every
   `fix-*` capture of this pass in spec order under its caption:
   `fix-band-*`, `fix-bar-keeps-the-way-out-*`, `fix-shell-merge-abort-and-undo-*`, `fix-start-over-whole-file-*`,
   `fix-summary-preview-*`, `fix-last-file-opens-summary-*`, `fix-finish-row-*`, `fix-c-in-pane-finishes-*`,
   `fix-box-takes-y-*`, `fix-subject-enter-commits-*`, `fix-stop-line-short-origin-*`, `fix-no-more-this-way-*`.
4. **TESTING.md**, every line concrete:
   - "Conflict journeys" table: every row whose keys or landing changed — `one-file-one-conflict`,
     `three-conflicts-all-theirs`, `two-files-in-order`, `two-files-out-of-order` land on the summary (top border
     `╭ Preview: finish merging feature into main`, status `<file> done ✓ · every file done`); `a-finish` `y` 1;
     `b-finish` `y` 1; `b-finish-again-after-undo` `c y` 2; `walk-back-a-done-file` `j d y` 3; `a-whole-one-file`
     `m m O y` 4; `a-whole-two-files` `m m o T y` 5; `b-whole-two-files` `⏎ o T y` 4; `editor-to-finish` `c y` 2.
     New rows: `last-block-to-finished` `o ⏎ y` 3; `b-abort-shell-merge` `a y` 2; `b-undo-a-take-in-the-shell` `u` 1;
     `walk-back-a-whole-file-choice` `k d y` 3 — each with its start, its landing words and its git truth check, as the
     existing rows do.
   - "Captures" table: the new keys and "what right looks like" for m6-14 … m6-17 (text only).
   - The colour paragraph (:2367-2370): "the band, tints and state colours are proven by buffer tests (named); a capture
     carries no colour; legibility in Zed, dark, 187 columns is **unverified** until he looks."
   - "Times I left" :108 → `c y` (from the banner) / `y` (the summary the last file opened); :113 (undo/abort in a shell
     merge) **closed**, naming `a_on_the_banner_aborts_a_shell_merge` and
     `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`.
   - Push and discard scenarios that say `→ ⏎` gain "or `y`" (the lines ticket 50's NOTES listed); the commit scenario
     gains `⏎` in the subject (ticket 51's list).
5. **Perf:** the release perf run the review needs, from its script, writing `docs/perf/acceptance-look-and-finish.txt`;
   read the summary lines only. The band adds per-cell style work on the resolver's hot draw path: the three-pane frame
   at 187x50 and 200x50 must stay inside its budget (ADR-0021).

**Out:** any behaviour fix (file with the line, or as a question); the `STALE FRAME` ticket-07 captures; batch 5.

## Acceptance (each tick names its test, at tick time)
- [ ] `m5_walk::every_scenario::journey` (every journey) and `journey_counts_match_the_note`, green; route table path.
- [ ] `every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`, green with the rename.
- [ ] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with every new `fix-*`.
- [ ] Every capture listed by path; each "what right looks like" line's words found in its file by the pty run (words,
      not looks). Colour **unverified** (rule 5); look is his (rule 6).
- [ ] Contact sheet part paths and sizes (each ≤ 200 000 B, from the script's output).
- [ ] TESTING.md rows written as above; the build's counts equal the note's (the parser holds them).
- [ ] Perf summary line from `docs/perf/acceptance-look-and-finish.txt`, with the frame budget named.
- [ ] Gates workspace-wide.

## What right looks like (the user's checklist, for TESTING.md)
- Resolver at 187, `notes.txt` with one block taken theirs and one open: a purple band runs from theirs through `◀┃`
  into the result; the open block has amber `?` at both separators and a sky-blue left, purple right, amber middle.
- The last `⏎` of a merge lands on a framed `Preview: finish merging feature into main`; the first two lines say
  nothing is committed until `y` and `u` takes it back; `⏎` on `README.md` opens its diff; `y` commits.
- `Esc` from the summary shows `✓ every file done — c review and finish the merge` under the title, and `c finish`
  first in the bar.
- On a shell `git pull` that stopped: `a y` aborts and `u` brings the merge back; `u` on the banner undoes a whole-file
  take.
- `P y` pushes; the force box shows no `y`. In the commit editor, `⏎` in the subject commits.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide. Context guard: captures, route tables and perf output go to files; the report names paths only.

## Size
Large but mechanical; the seam if it compacts: (1)-(3) first, (4)-(5) second (ticket 21/43's precedent).

## NOTES
