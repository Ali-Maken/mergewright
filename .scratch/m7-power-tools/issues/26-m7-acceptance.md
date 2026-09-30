# m7-acceptance — "you can rewrite, trace and recover history without leaving the app": every M7 journey walked and counted

Status: done (2026-10-01) — every box ticked after 26b re-run 2 (`scripts/m7-accept.sh` rc=0); see 26b NOTES "Re-run 2"
Blocked by: every M7 ticket 01–25 (and 02b, 04b)
Blocked by: every other M7 ticket:
- 01-error-rows-resume-restore, 02-rewrite-engine (+02b), 03-land-on-result, 04-slow-key-cause (+04b),
  05-rewrite-preview, 06-absorb-map, 07-rewrite-stops, 08-rewrite-editor, 09-amend-into-and-absorb-keys,
  10-branch-picker, 11-cherry-pick-key, 12-idle-layout-advance, 13-submodule-truth, 14-compare-since-split,
  15-daily-wording, 16-drawn-rows-are-his, 17-restart-after-done;
- trace-readers, blame-place, history-place, lost-commit-cost, journal-list, recover-op, journal-screen,
  m7-docs-and-gates.
Crates: graft (tests only), `scripts/m7-accept.sh`, `TESTING.md`, `docs/shots/m7/`, `docs/perf/m7-accept.txt`.
Source: `docs/design-notes/m7-power-tools.md`: Promise; both `journey-counts` fences (2a: 17 ids, 2b: 9 ids); "Where he
lands" (1b and 2b); every capture's Right line in each ticket. CLAUDE.md "Every milestone ends with an acceptance
ticket"; ADR-0037 (contact sheet).
Build with: **Fable**. It is the acceptance ticket: it judges the whole milestone against the promise, across 25
tickets, and a false tick here is Critical.
Scout: none. It reads each ticket's NOTES and the walk harness's MAP entry.
Needs: every signature above, as built. Each walk calls the app through the existing `m5_walk`/`m7_walk` harness.

## Context guard
- Every run writes to files: `.scratch/m7-power-tools/accept/` (raw) and `docs/perf/m7-accept.txt` (summary).
- The script prints **paths and one summary line per step**, never test output or frames.
- Read only the summary lines, and the failing test's own lines when one fails.

## `scripts/m7-accept.sh` (one command; exits non-zero on any failure)
1. `cargo nextest run` filtered to:
   - `m7_walk`;
   - `journey_counts_match_the_note`;
   - `every_m7_journey_id_has_a_walk` (new, below);
   - the hint-bar tests;
   - the promise scenarios (below).

   Output goes to `accept/nextest.txt`.
2. Every m3–m7 pty walk in **compare** mode (not update). This closes the old gap "captures are compared only when
   someone runs the walks". A mismatch fails, and its diff goes to `accept/pty-diffs/`.
3. Assemble the contact sheet `docs/shots/m7/m7-contact-sheet.txt` from `docs/shots/m7/m7-contact-sheet-captions.tsv`
   (ADR-0037). Each capture is printed under its Right line, in ticket order, with **one exception: the journal's
   captures come first**, under the heading:

   > Unjudged in rev 2: the journal, and every no-colour carrier. Every frame below is colourless: these are what you
   > see with colour off.

   Over 200 KB → split into `-1`, `-2`, and so on.
4. Read the perf summary lines and copy them to `docs/perf/m7-accept.txt`:
   - `docs/perf/m7-key.txt` (04);
   - `docs/perf/lost-commit-cost.txt`;
   - `docs/perf/trace-readers.txt`.

   A missing file fails. A `missed` budget is reported as missed, never as held.
5. Print: the sheet's paths, `docs/perf/m7-accept.txt`, and `accept: <n> journeys walked, <n> at or under count, <n>
   over`.

## Decided
- **Every journey id in both fences has a walk.** `every_m7_journey_id_has_a_walk` parses both fences in the M7 note
  (26 ids) and fails on any id that has no walk test named for it (the harness's id → test mapping).
  - A missing walk is written here, following its ticket's journey and the note's start state.
  - Each walk asserts `len(keys) <= noted_keys(id)` **and** the landing the note gives ("Where he lands", both
    tables).
- **Three promise scenarios**, end to end, one fixture each. The step count of each is the sum of its journeys'.
  - **Rewrite:** his branch with a teammate's built on it.
    1. `R s s y`: squash 3; the teammate's branch follows. Assert both branches' trees and messages, and the squashed
       message leads with the kept row.
    2. `u`: both branches back.
    3. Then an `edit` stop: `space a c` lands on the graph with the stack gone.
  - **Trace:** the "session" fixture.
    1. From the diff pane on line 22: `B`. The trailer names Lina.
    2. `^`: before Lina's commit, cursor on the old line.
    3. `Esc`, then `L`: the function's history; Lina's row selected; the file pane marks her added lines `+` (in the
       fixture: 18, 21, 22). (Amended 2026-10-01, planner's Q-trace ruling in 26b: was "line 19"; the fixture stays.)
    4. `⏎`: the graph on that commit.
    5. `Esc Esc Esc`: back to the diff pane, same line, same graph cursor.
  - **Recover:** the "lost work" fixture.
    1. `J ⏎ y`: main has D again.
    2. `u`: back to the reset state.
    3. `J n ⏎ y` (amended 2026-09-26, was `J ⏎ y`; see the ruling below), then `J n ⏎ y`: spike is back.
    4. `J n ⏎ y`: the edit to `src/a.rs` is back.

    No shell step anywhere after the fixture is built.
- **"Times I left"** (`TESTING.md`): the row M7 claims to close (:109, start over after `⏎`) has its walk named, and
  the row is marked closed with that test. No new row may be added by this ticket; one found goes to NOTES as a
  failure.

## Read first
- The note: Promise; "Journeys" (both tables); both `journey-counts` fences; "Where he lands" (both); "Pass 2b"
  (whole section, for the start states).
- Each M7 ticket's Acceptance and NOTES: the walks, captures and Right lines, and any deviation.
- `crates/graft/MAP.md`: the walk harness (`m5_walk`, `m7_walk`, `journeys::parse_counts`, `noted_keys`,
  `check_shot`).
- `TESTING.md`: the M6 acceptance section (as the format model) and "Times I left".

## Acceptance (name the verifying test at tick time; spot-check your own ticks)
- [x] `scripts/m7-accept.sh` exists; it exits 0 on the finished milestone and non-zero when one walk's count is lowered
      in a scratch copy (show it, then revert; say so in NOTES).
      **Half:** the red side shown (NOTES "Red demonstration": rc=1, `1 over`, `pull::pull_current_branch` red). It
      does **not** exit 0: F-recover, F-guard and 9 pty drifts (NOTES). Not ticked.
      **26b re-run (2026-10-01):** still rc=1 — `accept: 29 journeys walked, 29 at or under count, 0 over`, but step 1
      has 1 FAIL (recover, now at its last step) and step 2 1 drift (m4-03 80x24, a pulse race). Not ticked; 26b NOTES.
      **26b re-run 2 (main `a3901dee`): rc=0** (`docs/perf/m7-accept.txt`: steps 1–4 green, `29 at or under count,
      0 over`). Ticked.
- [x] `every_m7_journey_id_has_a_walk` green (`m7_walk::every_m7_journey_id_has_a_walk`, PASS in `accept/nextest.txt`);
      all **29** ids (the note grew from 26) listed in NOTES with their test names.
- [x] Promise scenarios green: `promise_rewrite_without_leaving` ✓ and `promise_trace_without_leaving` ✓ (both PASS);
      `promise_recover_without_leaving` **red** (F-recover). Not ticked.
      **26b re-run:** the `u` byte assert (`recover.rs:103`) now holds; red at `recover.rs:123` (step 4's put-back of the
      lost edit writes CRLF). Not ticked; 26b NOTES.
      **26b re-run 2:** all three PASS in `accept/nextest.txt` (`m7_walk::promise::{rewrite,trace,recover}::…`); byte
      asserts `recover.rs:103` and `:123` hold. Ticked.
- [x] Every m3–m7 pty walk passes in compare mode (step 2): **red**, 9 of 13 runs drifted (NOTES "pty compare"). Not
      ticked. Captures re-shot by tickets: `scouts/26-gap-rows.md` §3, as named in each ticket.
      **26b re-run:** 13 runs, 1 drifted (`m4-03-credential-masked-80x24.txt` :14, the field's pulse `◦`); the same walk
      re-run 3× clean. Not ticked; 26b NOTES.
      **26b re-run 2:** `2 pty: 13 runs (m3–m6 × sizes), 0 drifted or failed`; `accept/pty-diffs/` empty. Ticked.
- [x] **`TESTING.md` "M7 acceptance"** checklist: 29 journey entries + Rewrite, Trace, Recover and the 80-column guard;
      each names its test (the command), keys vs best tool, a Right line taken from its test's asserts (a scout checked
      each claim against file:line). Colour: one statement covers every entry, `colour: unverified (rule 5)`.
- [x] **Contact sheet** `docs/shots/m7/m7-contact-sheet-{1,2}.txt` (179 992 B and 33 330 B; step 3 prints `every file ≤
      200 KB: yes`), the heading and the four `journal-*` captures first; 74 captions, none `NOT RECORDED` (the only
      `NOT RECORDED` is the header's promise, set in `scripts/contact-sheet.sh`, outside 26). Its look is the user's.
- [x] `docs/perf/m7-accept.txt` holds the three perf summary lines verbatim (step 4: `3 files, 0 missing, 0 lines say
      missed`).
- [x] NOTES:
  - the KNOWN_GAPS rows the walks prove closed, each with its test: M6/68, M6/77 (both rows), :217, :847, :848, :109,
    the push-journey gates, and each 2a ticket's closed rows as its NOTES claim them;
  - the rows that stay open, with why.

## Touches (check each, report how)
- The walk harness: new walks and the id-coverage test. Check: `cargo nextest run` workspace-wide.
- `TESTING.md`: a new section; "Times I left" row :109 marked. Check: `git diff --stat TESTING.md` shows only those.
- Captures: no capture re-shot here. A compare failure is a failing ticket, not a re-shoot. Check: `git diff --stat
  docs/shots/**/*.txt` shows only the contact sheet.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (the rows are restated
above and in each ticket). No capture's "Right" is ticked by you: the sheet goes to the user (rule 6).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; then `scripts/m7-accept.sh`.

## Size
~70k. If it compacts, split:
- **m7-acceptance-a:** the script, the id-coverage test, the missing walks.
- **m7-acceptance-b:** the promise scenarios, `TESTING.md`, the contact sheet.

## Ruling (planner, 2026-09-26) — F1's preselect in the Recover scenario

The rule is 24's "Ruling (planner, 2026-09-26)": preselect the newest `✗` under a `$` row, when that `$` is newer
than every `●` except a fetch. The lost-work fixture changes order first (24's ruling; graft-fixtures).
- Step 1 `J ⏎ y` = 3: the journey `lost-to-shell-reset` as the note counts it (:1250).
- Step 3 is `J n ⏎ y` = 4, not `J ⏎ y`. After step 2's `u`, the undo is a `●` row newer than the reset (06 §6.5), so
  nothing is preselected and the cursor opens on the newest row. `n` moves to D's `✗`. That is not journey 13's start
  state (he just worked in Graft, and `U` would redo it in 1), so it is asserted in the scenario, not against
  `noted_keys("lost-to-shell-reset")`.
- Recover scenario total: 3 + 1 + 4 + 4 + 4 = **16** keys. `promise_recover_without_leaving` asserts it, and the
  `TESTING.md` entry says 16.

## Carry (conductor, 2026-09-26): review 28 Q2
Add a scenario: a guard refusal at 80 columns with a long branch name and a deep path. **Right:** the file name and the next step are both visible on the status line.

## Carry (conductor, 2026-09-30)
- Every ticket after 25 is merged too (24 a/b, 30a–30r, rebase-keeps-drops, 12e/12g). Journeys and "Where he lands" rows
  were amended since 2026-09-26 (e.g. put-back of a lost edit, u/U in J, keymap §2.11): count against the note **as it
  is now**; where a scenario's count differs from the note, the note wins unless the note is wrong — then say so as a
  question with the line, don't change the note.
- Re-take the m6-25 pty captures and rebuild the m5 and m6 contact sheets (review 10..07c M3); resolve TESTING.md's pty
  SHA (`401cfbe` vs `fbb2394`) by reading which commit the captures were taken at — cite it.
- Byte-exact `u` (30q/30r) is in the promise "you cannot lose work": the Recover scenario's `u` steps must compare bytes,
  including one LF file under `core.autocrlf=true`.
- Release perf this milestone is already run by the conductor (`docs/perf/m7-review-release.txt`, 17/17); don't re-run it.

## NOTES (builder, 2026-10-01)

**Compacted once** mid-ticket (after the promise scenarios and the script were written, before TESTING.md and these
NOTES). Nothing was dropped; what remains is listed under "Open" below. Scouts used: `scouts/26-journey-ids.md`,
`26-pty-and-sheets.md`, `26-promise-helpers.md`, `26-findings-lines.md`, `26-gap-rows.md`; a scout also checked every
TESTING.md Right line against its test's asserts (file:line list kept out of the repo; each entry's claims are asserts).

### Findings (product; not fixed here — the conductor files them)
- **F-recover, byte-exact `u` under `core.autocrlf=true` — broken.** Observed: `promise_recover_without_leaving`
  fails at `crates/graft/tests/it/m7_walk/promise/recover.rs:103` (`assert_bytes`, :145-158): `src/a.rs`
  is LF before the put-back (`J ⏎ y`), CRLF after it, and still CRLF after `u`. Cause **not confirmed** (question, with
  the scout's lines, `scouts/26-findings-lines.md` §1): the put-back's `git stash apply --index` re-smudges the set-aside
  edit (`graft-ops/src/apply/set_aside/put_back.rs:66,99-109`); the undo's `RestoreWorktree` scopes to paths whose
  *cleaned* trees differ (`graft-ops/src/undo.rs:339-347`), `scoped_paths` returns empty when they are equal
  (`restore/steps.rs:604-606`) and `write_raw` never runs (`steps.rs:280-283`), although the raw bytes were kept
  (`snapshot/raw.rs:80-107`, `apply/record.rs:126`).
- **F-guard, the refusal at 80 columns loses the file and the next step — broken** (review 28 Q2's carry).
  `guard_refusal_at_80_names_the_file_and_the_next_step` fails at `promise/guard.rs:85-92`: the bottom row reads
  `src/services/authentication/providers/oauth2/callbacks/session-c… · esc dismiss`. Lines: the words are one
  reason string (`graft-ops/src/operation.rs:1104`, `"{first} (ignored) is in the way of {doing} · move it aside
  first"`); `esc dismiss` is appended (`graft/src/view/status.rs:76,96-99`); `fit_status_line`
  (`graft-ui/src/graph_screen/hints/fitting.rs:356-371`) cuts the reason with `fit` (`graft-ui/src/fit.rs:29`), a
  left-to-right cut that always loses the tail. No middle or path elision exists (grep in the scout).
- **Stale pty captures** — see "pty compare" below. Textual (rule 7 exempt): each names file and first differing line.

### Questions (rule 7: no line proves a defect)
- **Q-trace:** the Decided Trace step 3 says "the file pane marks line 19 `+`". Lina's commit, as drawn, adds 18, 21
  and 22 (`+3 −2`); 19 (`loop {`) is context. The walk asserts the drawn truth (`promise/trace.rs:136-151`). Is "19" a
  typo for 18, or should the fixture change? Not changed.
- **Q-count:** the ticket says 26 ids (2a: 17); the note now has 29 (2a fence :823, 20 ids; 2b :1262, 9). Counted
  against the note as it is (carry 2026-09-30). No note count looked wrong.
- **Q-push gates:** Acceptance lists "the push-journey gates" among rows proven closed, but 25's NOTES (25:89) say `P y`
  has no id in any `journey-counts` fence, so there is nothing to gate; `p r r P y` was already gated
  (`rejected-push-dirty-to-pushed`, `every_scenario.rs:139`). Not claimed closed here.

### Journey ids → tests (29; all in `crates/graft/tests/it/m7_walk.rs` `WALKS`, checked by `every_m7_journey_id_has_a_walk`)
2a: squash-last-three `rewrite_editor::squash_last_three` · swap-adjacent `rewrite_editor::swap_adjacent` ·
fix-into-older `amend_into::fix_into_older` · absorb-staged `amend_into::absorb_staged` · amend-with-dependents
`amend_into::amend_with_dependents` · rebase-line-onto-main `rebase_line::r_y_moves_the_line` · squash-merged-bottom
`rebase_line::r_y_drops_the_squash_merged_branch` · keep-a-suggested-drop `rebase_line::keep_a_suggested_drop` ·
rewrite-stop-to-first-file `rewrite_stops::rewrite_stop_to_first_file` · edit-stop-lands `rewrite_stops::edit_stop_lands`
· edit-stop-amend-continue `rewrite_stops::edit_stop_amend_continue` · abort-from-a-chain-stop
`rewrite_stops::abort_from_a_chain_stop` · restart-after-done `restart_after_done` · switch-named-branch
`picker::switch_named_branch` · switch-back `picker::switch_back` · switch-with-edits `picker::switch_with_edits` ·
cherry-pick-in-view `picker::cherry_pick_in_view` · cherry-pick-other-branch `picker::cherry_pick_other_branch` ·
pr-contents `pr_contents::pr_contents` · pull-current-branch `pull::pull_current_branch` (**new here**: it had no walk;
`pull.rs` had hard-coded its count).
2b: blame-line-to-commit `blame::blame_line_to_commit` · commit-back-to-blame `blame::commit_back_to_blame` ·
blame-who-at-line `blame::blame_who_at_line` · blame-line-before `blame::blame_line_before` · function-history
`history::function_history` · file-history `history::file_history` · lost-to-shell-reset, deleted-branch-back,
edit-in-undo-backup `journal::the_three_lost_work_journeys`.

### pty compare (step 2): 13 runs, 9 drifted; 26 capture files differ (m4 4, m5 5, m6 17), none re-shot here
Logs: `.scratch/m7-power-tools/accept/pty-diffs/<m>-<size>.txt` (uncommitted, raw). m3 matches at both sizes. First
differing line per kind:
- **m4-03, m4-04** credential captures (80x24, 160x48): line 1 — the committed header carries a hand-added
  `· PREDATES m6-conflicts…` note the walk does not draw. Only the first difference is reported; the rest of each frame not compared by eye.
- **m5-01-preview-rebase** (80x24 :23, 160x48 :47): committed `git rebase --no-update-refs --no-autostash main`, drawn
  `git rebase -i --no-update-refs --no-autostash --empty=drop main` — M7's rewrite engine changed the command; stale.
- **m5-07-branch-row-hints-160x48** :50: the hint bar differs past column 150 (the log's cut; not read further).
- **m5-11-diverged-pull-sticky** (both sizes): line 4 is drawn blank, and the app was `not gone 10 s after SIGTERM —
  killed`. **Question**, no line: a slow read (`reading history…` where the capture had `184 rows so far`, 160x48 :3)
  or a real hang on SIGTERM — not diagnosed.
- **m6-06, m6-07** (80x24, 120x40): `REBASING step 1 of 2` → `REBASING  step 1 of 2 · pick <sha7> feature writes` — M7's
  banner wording; stale.
- **m6-18-launch-shell-begun** (every size): `diff vs HEAD 1e78110` → `a301516`. **Question:** a changed fixture or an
  unpinned date — not diagnosed.
- **m6-19-after-esc-esc** (every size): the banner bar gained `c finish` and `C`, lost `c` and the `merge`/`rebase` labels
  — M7's keymap; stale.
- **m6-21-stash-clash-done** (80x24, 120x40, 187x50): a `▎` drawn after `+1 −1` — **question**: a new marker (which
  ticket?) or a stray.
Re-shooting is not 26's (Touches: "a compare failure is a failing ticket, not a re-shoot"); the conductor decides whose.
Captures other M7 tickets name as re-shot: `scouts/26-gap-rows.md` §3 (names found in each ticket, not each proven).

### Red demonstration (done, reverted)
`pull-current-branch` lowered 1 → 0 at `docs/design-notes/m7-power-tools.md:844`; a scratch copy of the script with
steps 2–3 cut ran: `rc=1`, `accept: 29 journeys walked, 28 at or under count, 1 over`, the test saying
`pull-current-branch took 1 keys ["p"], the note allows 0`. The note restored (`git diff --stat` on it empty).

### Deviations from the spec
- No m7 pty walk exists: M7's captures are in-process `check_shot`s, compared by step 1's tests. Step 2 walks m3–m6.
- `accept/pty-diffs/` holds each failing run's log (file, first differing line, how to re-take), not a unified diff.
- Step 4 reads `docs/perf/m7-04-slow-key.txt` (`^SUMMARY `): the ticket's `m7-key.txt` does not exist.
- The sheet is over 180 KB, so it is `m7-contact-sheet-1.txt`, `-2.txt` (the script's split); the heading is at the top
  of `-1`. `scripts/contact-sheet.sh` prints `Promise: NOT RECORDED` for m7 — it has no m7 promise; that file is outside
  26's set. 31 captions were missing from the tsv; added from each ticket's Right lines, next to their feature.
- Rewrite scenario counts `R e y` (3) that makes the edit stop: 4 + 1 + 3 + 0 + 3 = 11.

### Carry 2026-09-30
- m6-25 re-taken: `docs/shots/m6/m6-25-menu-fixed-slots-{80x24,120x40,187x50}.txt` (`Cherry-pick 401cfbe` →
  `Cherry-pick 401cfbe (o)`; the bar row gains `R`, `C`). m6 sheet rebuilt (`m6-contact-sheet-4.txt` changed); m5
  sheet rebuilt, unchanged.
- SHA: `401cfbe` is right for the pty capture. The m6 pty walk pins the commit date (`.scratch/m6-conflicts/pty-walk-m6.py:60`,
  used at :111); the re-taken `m6-25-menu-fixed-slots-120x40.txt:6` reads `Cherry-pick 401cfbe (o)`, as `TESTING.md:2622`
  says. `fbb2394` is the same "fix the bug" commit in the in-process captures, dated by the harness clock
  (`docs/shots/m6/fix-undo-drift-way-back-80x24.txt`, `fix-box-owns-the-bar-80x24.txt`); it is not in TESTING.md. No
  TESTING change needed.

### KNOWN_GAPS rows (from ticket text only; KNOWN_GAPS not read — `scouts/26-gap-rows.md`)
Proven closed, each test green in this run unless said:
- M6/68 (aborted entry wording): `graft/src/view/journal.rs::tests::an_aborted_entry_reads_aborted_by_you` (24).
- M6/77 row 1 (the edit an undo over drift replaced): `recover.rs` `lost_edits_finds_the_edit_an_undo_over_drift_replaced`
  and `…_after_a_redo_over_drift` (23). Row 2 (nothing in the app brings it back): walk
  `journal::the_three_lost_work_journeys` journey 15, shell-free since 30o, **and** the promise's step 4 — but the
  promise scenario is red on F-recover's `u` step before it, so row 2 rests on the journal walk alone.
- :217 (no status line in J): `journal_screen::a_refusal_in_j_shows_on_its_status_row` (24; never seen red — 24:435).
- :847 (the set-aside drop's two names): `graft-ops … operation::tests::a_stash_drop_has_one_name` (24).
- :848 (a cut row runs words together): `graft-ui … journal_screen::tests::a_cut_row_cuts_at_a_word` (24).
- :109 "Times I left" (start over after `⏎`): `m7_walk::restart_after_done`; TESTING row marked closed.
- Each 2a ticket's closed rows: as the table in `scouts/26-gap-rows.md` §2 lists them with their tickets' lines; 26
  re-ran only the walks, so those rows stand on their own tickets' tests (the workspace gate below runs them all).
Stay open:
- The push-journey gates (Q-push above).
- M6/77 as a *promise* ("you cannot lose work" through `u`): F-recover.
- The 80-column refusal wording: F-guard.
- 30i's M7/13b-120 tick is unticked (30i:90); 24:3's Status still says "journey 15 question open" (30o:218 answers it).
- Unverified, not broken: colour everywhere (rule 5); every capture's look (rule 6, the user's).

### Gates and Touches (2026-10-01)
- `cargo fmt --check` ✓, `cargo clippy --workspace --all-targets -- -D warnings` ✓, `cargo test --doc` ✓.
  `cargo nextest run --workspace`: 4985 run, 4982 passed, **3 failed**: guard and recover (the findings), and
  `published_repo::nothing_a_commit_would_carry_names_this_machine` — caused by this ticket's raw logs in
  `.scratch/m7-power-tools/accept/` (untracked, home paths). Cause fixed: `accept/.gitignore` ignores the raw dir (it is
  never committed, CLAUDE.md "raw streams"); the test re-run alone passes.
- Walk harness: new `m7_walk/promise/*`, `pull_current_branch`, `WALKS` + the coverage test; helpers made
  `pub(super)` in `rewrite_editor.rs`, `journal.rs`; `Route::keys` in `blame.rs`; `TEN` `pub(crate)` in
  `pull_over_edits.rs`. Checked by the workspace nextest above. `crates/graft/MAP.md` gains one line.
- `TESTING.md`: `git diff --numstat` = 91 added, 1 removed (row :109's status cell); nothing else touched.
- Captures: `git diff --stat docs/shots/` = the m6-25 re-take (3 files, the carry's exception), `m6-contact-sheet-4.txt`,
  the m7 sheet (the old single file replaced by `-1`/`-2`) and its captions tsv. Nothing else re-shot.
- `scripts/m7-accept.sh` is new. `docs/perf/m7-accept.txt` is the last full run's summary.

### Open (for the conductor to split)
- F-recover, F-guard (product, other crates: graft-ops, graft-ui/graft).
- 26 stale pty captures (m4 4, m5 5, m6 17) and the two m5-11 / m6-18 / m6-21 questions.
- The script exits 0 only when those close; re-run `scripts/m7-accept.sh` then and tick the first, third and fourth
  boxes.
- `scripts/contact-sheet.sh` has no m7 promise (its header says `NOT RECORDED`).

### 26b (2026-10-01) — continuation, see `26b-m7-acceptance-finish.md` NOTES
- Merged into main's base with 30s, 30t, 30t-b, 30u, 30v (no conflicts). Q-trace: Decided step 3 amended per the
  planner's ruling; Q-push: gates exist, tests PASS (26b NOTES). The m7 promise is in `scripts/contact-sheet.sh`.
- Still open: F-recover moved — `u` now byte-exact; the lost edit's put-back (`J n ⏎ y`) writes CRLF
  (`M7/30q-putback-fwd`). F-guard closed. pty: one race (m4-03 80x24 pulse). Boxes 1, 3, 4 stay unticked.
