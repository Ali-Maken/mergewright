# acceptance-look-and-finish: walk the look-and-finish changes, re-take the M6 captures, rebuild the sheet, TESTING.md

Status: done
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
- [x] `m5_walk::every_scenario::journey` (every journey) and `journey_counts_match_the_note`, green; route table path.
      — `m5_walk::every_scenario` 102/102 and `journey_counts_match_the_note` 1/1;
      `.scratch/m6-conflicts/acceptance-look-and-finish-routes.txt` (28 routes, none over).
- [x] `every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`, green with the rename. — green in the
      102/102 run above and in the workspace nextest.
- [x] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with every new `fix-*`. — same runs.
- [x] Every capture listed by path; each "what right looks like" line's words found in its file by the pty run (words,
      not looks). Colour **unverified** (rule 5); look is his (rule 6). — `pty-walk-m6.py` compare run at 80x24,
      120x40, 160x48, 187x50, 200x50: 68/68 `docs/shots/m6/m6-*` files match with their on-screen words; TESTING.md
      phrases checked against the files by script (one miss, an absence). Colour unverified; look his.
- [x] Contact sheet part paths and sizes (each ≤ 200 000 B, from the script's output). — m6 parts 179 678 / 177 038 /
      140 530 B (`docs/shots/m6/m6-contact-sheet-{1,2,3}.txt`); m3, m4, m5 sheets listed in NOTES.
- [x] TESTING.md rows written as above; the build's counts equal the note's (the parser holds them). —
      `journey_counts_match_the_note` green; rows listed in NOTES.
- [x] Perf summary line from `docs/perf/acceptance-look-and-finish.txt`, with the frame budget named. —
      `resolver_frames: 2 sizes, max 494 us, 0 at/over the 16000 us budget` (test
      `resolver::frame_budget::three_panes_at_187_and_200_columns_keep_input_to_frame_under_16_ms`, release).
- [x] Gates workspace-wide. — `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok (and
      with `--features graft/heavy-tests`); `cargo nextest run --workspace --no-fail-fast` 3232/3232 (1 skipped);
      `cargo test --workspace --doc` 3/3.

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

**Compacted once, mid-ticket** (after the m6 retakes and most TESTING.md rows). Nothing of the scope remains; every
item below was done or re-checked after the compaction.

**Scout:** one (`scout`, Opus low) on the journeys' landings and assertions before the TESTING.md rows; its report
stayed in the session scratchpad, not the repo.

**Walk (Scope 1).** `cargo nextest run -p graft --test it m5_walk::every_scenario`: 102/102 (every `journey_*`, the capture and caption
checks); `conflict_journeys::…::journey_counts_match_the_note` run on its own with `--no-capture`: 1/1. Route table:
`.scratch/m6-conflicts/acceptance-look-and-finish-routes.txt`, 28 routes, every one equal to the ticket's counts
(`last-block-to-finished` 3, `b-abort-shell-merge` 2, `b-undo-a-take-in-the-shell` 1, `walk-back-a-whole-file-choice`
3, `a-whole-two-files` 5, …). None over its row (the test fails if one is).

**Captures (Scope 2).** Keys as the ticket says: m6-16's arm now presses `⏎` (`conflict_journeys.rs`, the `captured()`
table); m6-14/15/17 already had theirs. `m6-15-finish-summary` → `m6-15-finish-row-after-esc` (files, `coverage.rs`
`M6_STEMS`, the captions TSV, TESTING.md; matrices regenerated by `m5_walk --matrix`:
`.scratch/m6-conflicts/matrix.txt`, `docs/shots/35-coverage-matrix.txt`). Drift found by `pty-walk-m6.py` at all five
sizes and retaken: m6-07 at 120/80 (ticket 60's finish row; its on_screen in `resolved_operations.rs` was the old
`every file done ✓`, now `✓ every file done`), m6-16 ×5 (the `⏎` key), m6-19 at 160/187/200. Re-check after the
retakes: **all 68 m6 pty captures match at every size they exist at** (the 27 "DRIFTED" lines in that run are
stem × size pairs that have no file, m6-01…09 above 120x40). m6-14/16 at 187/200 are 9.9–10.6 KB, over the script's
8 KB soft note, under the 200 KB cap.

**Every pty capture re-walked (M6/64).** m3, m4, m5 and m6 walks run in compare mode:
- **m3** (`pty-walk-m3.py`): the script wrote frames itself, unredacted (a real run put the machine's temp path into
  m3-08/m3-09). Fixed the same way as m4: frames go through `graft-fixtures capture` (redacts, compares, rewrites only
  under `GRAFT_UPDATE_SHOTS=1`, exits red on drift), commit dates pinned. Drift retaken: m3-01/02 (bottom-row status,
  commit ids), m3-03 (the confirm box is now framed, `▸[ Cancel ]   [ Discard ]`, bar `y discard …`), m3-04-160 (id),
  m3-09 (framed drift box, `▸[ Stop ]   [ Undo ]`, `undo · esc cancels`). TESTING.md m3 "Per breakpoint" Confirm and
  Drift rows and the m3 captions rewritten. Re-check green at 80x24 and 160x48.
- **m4** (`pty-walk-m4.py`, **M6/50**): redaction fixed first (same writer), commit dates pinned, then every m4
  capture retaken at 80x24 and 160x48 — m4-01 now shows `y push  ←/→ move  ⏎ choose  esc cancel`; m4-02 (lease) shows
  no `y`. No machine path in any m4 file (grep). **Flaky, not fixed:** m4-03-160x48 sometimes catches the field's
  pulse glyph `◦` (ADR-0038, by design); the committed frame is the settled one.
- **m5** (`pty-walk-m5.py`): m5-01/02/03 at 80/160 (the box footer now leads with `y confirm`, ticket 50) and m5-06 at
  80/120/160 (the resolver's key bar) retaken, captions noted; m5-05 at 38x10 matches. The script also walked the M6
  stems against `docs/shots/<stem>` (no such file) and reported them as drift; it now skips `m6-` stems, which
  `pty-walk-m6.py` owns.

**Contact sheets (Scope 3)**, from the script's output, each ≤ 200 000 B:
- m6: `docs/shots/m6/m6-contact-sheet-1.txt` 179 678, `-2.txt` 177 038, `-3.txt` 140 530 (138 captures, 0 without
  a caption).
- m3: `docs/shots/m3-contact-sheet.txt` 129 487. m4: `docs/shots/m4-contact-sheet.txt` 114 701. m5:
  `docs/shots/m5-contact-sheet-1.txt` 172 347, `-2.txt` 111 541.
- This pass's `fix-*` in spec order at the end of the m6 captions: band ×2, shell-merge-abort-and-undo,
  start-over-whole-file, summary-preview ×2, last-file-opens-summary, finish-row, c-in-pane-finishes, c-live-frames,
  box-takes-y ×2, subject-enter-commits ×2. **Three names in the ticket have no file, by earlier decisions:**
  `fix-bar-keeps-the-way-out-*` (ticket 56 retook m6-13-after-ctrl-t instead, conductor-approved),
  `fix-stop-line-short-origin-*` and `fix-no-more-this-way-*` (ticket 61 retook m6-18, m6-08 and m6-06 instead).

**TESTING.md (Scope 4).** Conflict journeys table (every changed row, plus the four new ones), Captures rows m6-14…18,
the colour paragraph (names the buffer tests; Zed/dark/187 **unverified**), the m6 "Per breakpoint" rows m6-01…08
rewritten to the drawn words (they still said `took ours`, `file 11 of 13`, `all conflicts resolved · c continue ✓`),
Times I left :108 (`c y` / `y`) and :113 **closed** (`a_on_the_banner_aborts_a_shell_merge`,
`u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`). Every backticked phrase of every m6 capture row was
checked against its files by script; the one miss is an absence (`no MERGING` in m6-17).

**Gap closures, for the conductor:**
- M6/52: `fix-c-mid-merge-answers-in-words-*` → `fix-a-mid-merge-answers-in-words-*` (80x24, 187x50); the test
  writing them is `resolver::shell_begun::capture_fix_a_mid_merge_answers_in_words`.
- M6/50: redaction in `pty-walk-m4.py` (and the same fix in `pty-walk-m3.py`); `m4-01-push-box-plain-{80x24,160x48}`
  retaken and asserted by the walk.
- M6/64: every pty walk re-run in compare mode, drift listed above, retaken, re-checked green.
- Times I left :113: closed with the two tests above.
- Ticket 50's lines (`or y`): TESTING.md 1259–1260 and 1262–1263 (discard, the drop), 1304, 1492 (push), 1506; ticket
  51's commit line 1275–1276 (`⏎` in the subject). Tests: `y_and_enter_emit_the_same_commands`,
  `the_y_word_is_the_acting_label`, `enter_in_the_subject_commits`.

**Perf (Scope 5).** New heavy test `resolver::frame_budget::three_panes_at_187_and_200_columns_keep_input_to_frame_under_16_ms`
(a 600-line merge, 120 one-line blocks, 12 taken so the band shows, 80 `n`/`N` keys per size, each timed from the key
to the frame written into a sink). Script `.scratch/m6-conflicts/perf-m6-63.sh` →
`docs/perf/acceptance-look-and-finish.txt`, release: **187x50 p99 465 µs, max 494 µs; 200x50 p99 410 µs, max 410 µs;
budget 16 000 µs, 0 over.** heavy::budgets alongside: first paint max 110 935 µs (budget 150 000; 39 252 µs on the
run before, so it varies by run), peak RSS 106 MiB (budget 300).
What it leaves out: the terminal's own parsing and painting (as `frames.rs` does). Gotcha found writing it: git joins
conflicts with ≤ 3 unchanged lines between them into one block, so the fixture spaces them every 5th line.

**Verified, how:** journeys and counts by the tests named; capture words by the pty walks (words, not looks);
frame time by the release run. **Not verified:** colour (rule 5, buffer tests only); the look of every capture (rule
6, his eye); legibility in Zed, dark, at 187 columns.

**Questions (no line, so not defects):**
- Wording: the finish row says `c continue the rebase` (m6-07) while the status line under it says `c continues the
  rebase`. Same key, two grammatical forms on one screen. Capture: `docs/shots/m6/m6-07-all-resolved-120x40.txt`.

**Touches:** the harness (`conflict_journeys.rs`, `coverage.rs`, `resolved_operations.rs`, `shell_begun.rs`,
`resolver.rs` + new `frame_budget.rs`), checked by the every_scenario run and the full gates; both coverage matrices
(regenerated, `captures_are_named_keyed_and_listed_in_the_matrix` green); captions TSVs m3/m4/m5/m6 and their sheets
(`contact_sheet_captions_cover_every_m6_capture` green, script reports 0 uncaptioned); TESTING.md; the m3/m4/m5 pty
scripts (re-run green after the change); `docs/perf/`. No `crates/*/src` touched (git status). Machine-path grep over
every changed file: none.
