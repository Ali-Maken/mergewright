# m7-acceptance — "you can rewrite, trace and recover history without leaving the app": every M7 journey walked and counted

Status: ready
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
    3. `Esc`, then `L`: the function's history; Lina's row selected; the file pane marks line 19 `+`.
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
- [ ] `scripts/m7-accept.sh` exists; it exits 0 on the finished milestone and non-zero when one walk's count is lowered
      in a scratch copy (show it, then revert; say so in NOTES).
- [ ] `every_m7_journey_id_has_a_walk` green; all 26 ids listed in NOTES with their test names.
- [ ] Promise scenarios green: `promise_rewrite_without_leaving`, `promise_trace_without_leaving`,
      `promise_recover_without_leaving`.
- [ ] Every m3–m7 pty walk passes in compare mode (step 2). Any capture a ticket re-shot is named in that ticket's
      NOTES, and in this ticket's NOTES with the ticket.
- [ ] **`TESTING.md` "M7 acceptance"** checklist: one entry per journey and per promise scenario.
  - Each entry has the command, the step count against the best tool's, and a **Right:** line saying concretely what
    right looks like (the words on screen, the row the cursor is on, which glyph where), never "looks correct".
  - Colour entries say `colour: unverified (rule 5)`.
- [ ] **Contact sheet** `docs/shots/m7/m7-contact-sheet.txt` (≤ 200 KB per file), journal first, every M7 capture under
      its Right line.
- [ ] `docs/perf/m7-accept.txt` holds the three perf summary lines verbatim.
- [ ] NOTES:
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
