# restart-after-done — `d` on a file already marked done starts it over, inside the operation

Status: done
Blocked by: none
Crates: graft (the resolver's `d` on a done row); graft-ui only if the done row can't show `d` on the bar.
Source: note "Pass 2a" R4 "Start over after `⏎`"; `TESTING.md` "Times I left" :109 ("still the shell — `git checkout
-m -- <path>` … the ops half is built (ruling F3)"); journey `restart-after-done`.
Build with: Sonnet. It wires the existing `Operation::RestartConflict` to a key on one more row kind. The box and the
words already exist (before-`⏎` start-over, TESTING.md :2215).
Scout: `scout` first (ADR-0035).
Needs: none. It runs in parallel with graft-ops tickets; it can't run alongside another graft ticket.

## Decided
- In the resolver's file list, a file marked done (`✓`) answers `d` with the same box as before `⏎`, titled
  `Start <file> over`, plus one line: `your choices in <file> are dropped`. `y` confirms (§2.3c).
- It runs `RestartConflict` (`git checkout -m -- <path>`). The row goes back among the conflicts. The index has the
  **same stage oids and modes as at the stop**, and the markers are back on disk.
- `u` stays refused while the operation is live (§2.6a). The box's line is the warning.
- If git can't recreate the conflict (the resolve-undo info is gone, e.g. after a shell `git add` + commit of the
  index), the refusal comes in git's reason. The words are `git can't bring back the conflict in <file>: <reason>`,
  and nothing is written.

## Read first
- Note "Pass 2a" R4 (the start-over bullet), "Journey counts" `restart-after-done`.
- `TESTING.md` :109 and :2215-2225 (the before-`⏎` start-over walk).
- `docs/design/02-keymap.md` §2.6 (`d`), §2.3c.
- `crates/graft/MAP.md` resolver entries (the file list, `d`, `resolved.rs`); `crates/graft-ops/MAP.md`
  `RestartConflict`.

## Scout brief (≤ 1 page)
1. Does the resolver's file list keep a row for a file marked done after `⏎`? Where is it drawn, and which keys does it
   answer?
2. The before-`⏎` `d` path: the function, the box, the op launched.
3. What `RestartConflict` needs as input (the path only? the stop's stage oids?), and whether it works on a staged path
   (the ruling F3 note).

## Signatures
```rust
// no new pub API; name the resolver function that routes `d` on a done row in NOTES.
```

## Acceptance
- [ ] **App:**
  - `d_on_a_done_file_opens_the_start_over_box` (red first);
  - `y_restores_the_stages_as_at_the_stop`;
  - `esc_in_the_box_changes_nothing`;
  - `a_restart_git_cannot_do_is_refused_in_words`.
- [ ] **Walk** `restart_after_done` (`m7_walk` or `m6_walk`; the scout picks the harness): after `⏎` on `three.txt`,
      motion `d y` ≤ `noted_keys("restart-after-done")`. `git ls-files -u` lists `three.txt` with the stop's oids and
      modes; two `<<<<<<< ` lines are on disk.
- [ ] **Capture** `docs/shots/m7/restart-after-done-box-120x40.txt`. **Right:** title `Start three.txt over`; `your
      choices in three.txt are dropped`; bar `y`. Colour unverified.
- [ ] Hint-bar tests green. TESTING.md: the "Times I left" :109 row gets the walk's name, for the conductor to close.
      MAP.md (graft).

## Touches (check each, report how)
- **The before-`⏎` start-over:** unchanged. Check: TESTING.md :2215's walk (`start-over-brings-the-markers-back`) green.
- **`⏎` (mark done), `c`, `j/k` in the resolver:** unchanged. Check: the m6 resolver tests.
- **The finish row / summary:** a restarted file un-does "every file done". Check:
  `restart_after_the_last_file_hides_the_finish_row` (add).
- **Journal:** `RestartConflict` folds into the operation's entry as before. Check: the existing restart journal test.
- **Keymap:** §2.6 `d` row per the diff (the conductor).

## Rules / Gates
Worktree only; `CARGO_TARGET_DIR` in the worktree; the four gates workspace-wide.

## Size
Small (~30k).

## NOTES (builder, 2026-09-26)
Scout used (`.scratch/m7-power-tools/scouts/17.md`). `d` on a done file already existed (m6-conflicts/28): routed by `update/resolver.rs` `act` → `Char('d') => restart::restart_box`. This ticket closed the gap only. No compaction.

Ticks:
- [x] `d_on_a_done_file_opens_the_start_over_box` (`tests/it/resolver/file_list/start_over.rs`; red first on the words).
- [x] `y_restores_the_stages_as_at_the_stop` — `d y`; `ls-files -u` mode+oid+stage equal the stop's; one `<<<<<<< ` on disk (the fixture has one block). **Characterises existing behaviour: never red.** `y` already confirmed (label `Start over` → `ConfirmYes::StartOver`) and the stages already matched; not test-first.
- [x] `esc_in_the_box_changes_nothing`.
- [x] `a_restart_git_cannot_do_is_refused_in_words`.
- [x] Capture `docs/shots/m7/restart-after-done-box-120x40.txt` (test `d_on_a_done_file_opens_the_start_over_box`) — the file is `shared.rs`, not `three.txt` (fixture `app_merge_with_a_whole_file_conflict`). Colour unverified.
- [x] Walk `crates/graft/tests/it/m7_walk.rs::restart_after_done` — route click·`d`·`y` = 3 ≤ `restart-after-done` read from the M7 note's `journey-counts` fence (`m5_walk::parse_counts`, now re-exported); stop's stages and one `<<<<<<< ` checked. File `shared.rs`, not `three.txt`. Characterisation: passed on its first run.
- [x] `resolver::file_list::start_over::restart_after_the_last_file_hides_the_finish_row` — `every file done` shown after `Esc` from the summary, gone after `d y`. Characterisation: passed on its first run.
- Test helpers made `pub(crate)` for the walk: `resolver::file_list::{rows, click_file, on_gone_card, shared_done}`, `resolver::restart::app_merge_with_a_whole_file_conflict`.

Word changes (done file only; before-`⏎` box unchanged):
- Title `Start <file> over?` → `Start <file> over`.
- Line `takes and edits in this file are lost` / `edits since the choice are lost` → `your choices in <file> are dropped` (suffix `· not undoable while the <op> is open` kept).
- Refusal `can't start over · <why>` → `git can't bring back the conflict in <file>: <reason>`; a leading `<file>: ` in ops' reason is stripped. The reason is ops' (`NoStagesKept`), not git's: ops doesn't use resolve-undo, kept as asked.

Captures regenerated: `m6/fix-done-start-over-cannot-120x40.txt`, `m6/fix-start-over-whole-file-120x40.txt`. m6 contact sheet rebuilt (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`): parts 1 and 3 changed, all parts ≤ 179 832 B; captions rows for the two shots updated to the new words.
