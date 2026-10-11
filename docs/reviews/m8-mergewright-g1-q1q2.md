# Review: m8 group 1, tickets 46 (Q1, J reads `⇢ replaced`/`deleted`) and 47 (Q2, picker `⏎` on current closes)

Range `c8b08592..4b5a51b2` (commits 5191a559, 2f96de43 and their merges). Reviewer, 2026-10-12. Read-only.

**Not run:** cargo of any kind (the conductor runs the gates; CPU limit). No gate, heavy tier, release perf run,
fuzz or bench result is claimed here. Every "held" below is from reading the test against the box, not from a run.
One `git` experiment was run in the scratchpad (git 2.54.0), quoted under Major 2.

**Counts:** Critical 0 · Major 2 (one is a question) · Minor 10.

## Acceptance spot-check

| Ticket | Boxes | Checked | Held | Notes |
|---|---|---|---|---|
| 46 | 11 | 11 | 11 | Every named test exists (grep) and asserts what its box says. The glyph grep was re-run at 4b5a51b2 and matches NOTES. |
| 47 | 9 | 9 | 9 | The `you're on` grep box overclaims "no hit" in its wording; NOTES quote the real hits and why. Not a false tick (Minor 4). |

No false tick found.

## Pipeline (design note `pass-3-mergewright.md`)

- **Need:** present (:16-19), in the user's words (field notes :97).
- **Prior art:** Q1 cites `docs/research/reflog.md` and marks the choice **I** (no tool found that says whether a
  reflog loss can be recovered). Q2 cites only the app's own switch rule, with no other tool's behaviour (Minor 8).
  The note says no new study is needed because both are word and landing fixes on screens that already have studies.
  That is acceptable, so this is not Major.
- **Journeys:** `journal-replaced` takes 1 step (`J`, then read) and `picker-enter-current` takes 3 (`C`, the name,
  `⏎`). The built path gives the same counts: `⏎` on the current row calls `show_on_graph`
  (`crates/graft/src/update/picker.rs:201-203`), so there is no `Esc` step. **Not walked here:** the pty walks
  belong to ticket 57 (`m8_walk::mergewright::*`), so no step-counting walk exists for this group yet.
- **Lands:** "`⏎` on the current branch: picker closed, graph focused, cursor on HEAD, nothing written" is held by
  `current_row::enter_on_the_current_branch_closes`. "`⏎`/`b` on a `⇢` row as on `✗`" is held by
  `replaced::enter_and_b_on_a_replaced_row_act_as_on_lost`.

## The steps.rs root-cause fix (46, outside its file list)

**Judgement: correct for the case it names. It only ever removes pairings. It does not cover every case its own
comment claims.**

- `fold_one` (`crates/graft-ops/src/journal_list/steps.rs:148-162`) now pairs a `HEAD` rebase run only with the
  finish of the branch in that run's own `returning to` record. Before, it took the finish record nearest by index
  inside the time window. That nearness rule paired the newer of two same-second rebases with the other branch's
  finish. The regression test `journal_list::tests::fate::two_rebases_in_one_second_keep_their_own_finish` builds
  exactly that layout: the other branch's finish sits nearer by index (records 7 and 8).
- The fix is safe. The new filter is stricter than the old one. When a run has no `returning to` record (an aborted
  or detached rebase), `is_none_or` keeps the old behaviour. A detached rebase's `returning to <hex>` now matches no
  branch finish, where before it could take a nearby one. Both changes are improvements.
- It is the cause, not a symptom: it fixes the pairing, not the fate rule. It leaves a neighbouring cause open
  (Major 2).

## Critical

None.

## Major

### 1. Readability ratchet raised for three source files instead of splitting them; the `returning to` parse is duplicated
- **Where:** `crates/graft/tests/it/over_400.txt`. `graft-ops/src/journal_list/steps.rs` 414 → 425,
  `graft-ui/src/journal_screen.rs` 474 → 490 and `graft/src/update/journal/keys.rs` 423 → 426 (plus three test
  files). The duplicate is at `steps.rs:148-153` and `steps.rs:306-309`: the same two-prefix `strip_prefix("rebase
  (finish): returning to ")` / `"rebase -i (finish): returning to "` parse appears in both places.
- **Why:** the ratchet file's own header says "lower a row when a file shrinks". Raising a row turns the gate into
  a record of growth (PROMPT.md 6.2). Ticket 47 moved code out to stay under its rows, so ticket 46 could have done
  the same. The duplicated parse means a third subject form (git changing the wording) would be fixed in one place
  and missed in the other.
- **Fix:**
  - Move `legend`, `legend_at` and `REPLACED_CLAUSE` to `journal_screen/legend.rs`.
  - In `steps.rs`, add `fn returning_to(subject) -> Option<&str>` next to `rebase_finish_branch`, and call it at
    :148 and :306. The file drops back under 414.
  - Put the three source rows back to their old values.

### 2. Question: adjacent rebase runs still fold into one, so the "chain of branches" case the fix names is only partly fixed
- **Where:**
  - `crates/graft-ops/src/journal_list/steps.rs:131-134` groups every consecutive `HEAD` record whose subject passes
    `is_rebase` into one run.
  - `steps.rs:148` then takes the first (newest) `returning to` in that run.
  - Graft's own rebase passes `<top>` when branches follow (`crates/graft-ops/src/operation/history.rs:800-802`).
- **Observed:** git 2.54.0 in the scratchpad, `git rebase -q main one && git rebase -q main two` run from `main`.
  `HEAD`'s reflog, newest first:
  `rebase (finish): returning to refs/heads/two`, `rebase (pick): two`, `rebase (start): checkout main`,
  `rebase (finish): returning to refs/heads/one`, `rebase (pick): one`, `rebase (start): checkout main`, `commit: M`.
  No `checkout:` record separates the two rebases. By :131-134 they make **one** run, which pairs only with `two`'s
  finish. `one`'s finish is left over as a step of its own.
- **Not verified (rule 7, hence a question):**
  - Whether an in-app rebase with `<top>` ≠ the checked-out branch, run after another in-app rebase, produces this
    layout.
  - Where `one`'s old tip then lands: which event, and whether it reads `✗ lost` while `u` would bring it back.
  - The comment at `steps.rs:145-147` claims this case ("a chain of branches rebased in turn"). It is his daily
    workflow (dependent branches).
  - Any error is in the safe direction: it says "lost" when the commit can be recovered, never the reverse.
- **Fix (if confirmed):** end a run at each `rebase (finish): returning to` record, so each rebase is its own run.
  Add a real-repo test in `tests/it/journal_fate.rs`: two `rebase_onto_main`-style operations, the second with
  `top` set to the other branch and no switch between them, and assert both old tips are `Replaced`.

## Minor

1. **The picker bar says `⏎ close  esc close`.** Two keys use the same word but do different things: `⏎` on the
   current row runs `show_on_graph` (`crates/graft/src/update/picker.rs:202`), which is `⇥`'s action and puts the
   cursor on HEAD, while `esc` only closes. Seen in `docs/shots/m8-q2-picker-current-row-ui-120x36.txt` (last line).
   The test pins it: `hints/tests/typed_fields.rs` asserts `["to find", "close", "close", …]`. The spec chose
   "close" (the user said "close the picker"), so this is a question for the usability walk: should the clause read
   what it does (`⏎ back to the graph`), or should `esc close` drop out on this row?
2. **Help and man text not updated, though the spec's Touches asked for it.** `crates/graft-ui/src/keymap/table.rs:398`
   (`"Switch to it"`) and `docs/man/trib.1:385`. `?` describes `⏎` as switch only, while §2.16 now says it closes on
   the current branch. NOTES disclose this; no KNOWN_GAPS row exists. Fix: one clause in the description, then
   regenerate the man page; or file a row.
3. **Stale doc comment** at `crates/graft-ui/src/picker.rs:119`: it lists `you're on` among the reasons `note` is
   set, and that message no longer exists. NOTES flagged it but did not fix it. Fix: drop the clause.
4. **Box 8 of ticket 47 overclaims.** It says "`you're on` in `crates/*/src`: no hit", but grep hits
   `graft-ui/src/picker.rs:83,119,284` (`you're on it`, the `· here` row's subject) and a test comment. NOTES explain
   this. Fix: reword the box to "the removed `you're on <b>` message: no hit".
5. **At 80 columns the sha column shifts between `⇢` and `✗` rows.** In `docs/shots/m8-q1-journal-replaced-ui-80x24.txt`
   the `⇢` row's sha starts at column 18 and the `✗` row's at column 15. The cause is `pieces.rs:82-83`
   (`column.max(label.width() + 1)`, applied per row). This is a textual finding. Fix: below 120 columns, use the
   widest label's column for every lost row, so all shas line up.
6. **Question: after `u` then `U`, does the rebase's old tip read `✗ lost` again?**
   - `place` puts a run under the newest event that had the tip as an old value (`journal_list/rows.rs:234-242`).
   - A Redo entry whose `refs_before` holds the old tip would therefore own it.
   - `fate_of` returns `Lost` for any non-operation (`fate.rs:44`).
   - Yet `next_undo` would undo the re-applied rebase and bring the tip back.
   - Not run. The spec's "redo's case stays Lost" covers what an **undo** dropped, not this.
   - Fix if confirmed: judge the fate by the in-force operation whose before-ref held the tip, not by the event
     the run sits under.
7. **Long doc lines that rustfmt does not wrap.** `crates/graft-ops/src/journal_list/fate.rs:33` (128 columns) and
   `crates/graft/src/view/journal.rs:243-244`. Fix: rewrap.
8. **Q2's Prior art row (`pass-3-mergewright.md:28`) names no other tool's `⏎`-on-current behaviour.** VS Code's
   checkout picker, lazygit and Sublime Merge were not checked. The choice is marked **I**, but without a look at
   what they do it is not clear anything needed inventing. Fix: one line per tool, added to `palette-find.md`.
9. **Constant names don't match.** `graft-ui/src/graph_screen/hints/lists/editors.rs` has `PICKER_SWITCH_WORD` and
   `PICKER_CLOSE`, both verb words, sitting beside `PICKER_SWITCH` and `PICKER_SHOW`, which are the keys. So
   `PICKER_CLOSE` reads like a key constant. Fix: `PICKER_SWITCH_VERB` / `PICKER_CLOSE_VERB`.
10. **Loose ends from the two tickets:**
    - The J-has-no-ASCII-tier gap was reported to the conductor but has no KNOWN_GAPS row (grep). Conductor's to
      file.
    - Ticket 47's Touches asked for `.scratch/m8-polish-and-ship/q2-shot-drift.txt`. The file does not exist, and
      NOTES do not say "no drift". The diff changes no existing picker or hints capture, so it is probably nil;
      NOTES should say so.
    - The Q2 capture's graph rows show the shared fixture's hostile subject (`Commit 3␛[31m`), which is noise in a
      capture he judges by eye.

## Other checks

- **Rule 3:** no `let _`, `unwrap`, `expect`, `unwrap_or_default` or `Err(_)` added in non-test source (grep of the
  `+` lines).
- **Terminal escapes:** `⇢` words still pass through `sanitize_line` (`view/journal.rs:140`).
- **Git on the UI thread:** none added. `fate_of` is pure; `hint_context` reads state.
- **Data loss:** the risky direction is calling a commit "replaced" when `u` cannot bring it back. `fate_of`
  (`fate.rs:41-58`) requires all four:
  - the entry is in force;
  - it is an operation;
  - it is `undoable`;
  - one of its before-refs was on the tip, and its recorded after-state has that ref moved or absent.

  A ref that was never recorded stays `Lost`. `undo::drift` (`undo.rs:794`) asks before a drifted undo but does not
  refuse it, so "undo brings it back" holds even after a later shell move. I found no path to a false "replaced".
- **Keymap:** §2.11 and §2.16 were amended with dates and the user's words. No key or action id changed.
  `n`/`N` skip `⇢` rows (`keys.rs:134-140`). `Enter` and `b` are unchanged.
- **Architecture:** `Fate` lives in graft-ops (pure), its mapping in graft, its drawing in graft-ui. No crate rule is
  crossed. The MAP lines match the code. The graft-ops MAP correction about `undoable_kind` is right
  (`journal.rs:217-219` and `next_undo` at :279 step over Undo/Redo separately).
- **Touches (46):** the m7 J captures drifted, with the summary in `.scratch/m8-polish-and-ship/q1-shot-drift.txt`.
  One m7 row now reads `⇢ replaced` (a squash is undoable), which is the intended change. The palette test literal
  was updated (`palette_live.rs:116`).

SKILLS: code-review
