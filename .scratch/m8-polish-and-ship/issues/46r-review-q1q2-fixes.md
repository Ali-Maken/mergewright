# 46r: review fixes for group 1 (tickets 46, 47)

Status: ready
Blocked by: none (46, 47 merged at 4b5a51b2)
Build with: **Opus**. Journal pairing and undo fate (M2 + minor 6) are undo semantics.
Brownfield: the 46/47 scouts and their NOTES serve as the scout; the review names every line.
Needs: none. Before 48 (48 touches every crate's names).

## Read first
- `docs/reviews/m8-mergewright-g1-q1q2.md`, whole (Major 1–2, Minor 1–7, 9, 10)
- Tickets 46 and 47: Decisions and NOTES

## Scope (from the review; conductor's routing)
- **Major 1:** split `journal_list/steps.rs`, `graft-ui journal_screen.rs` and `graft update/journal/keys.rs` back under
  the 400-line limit instead of raising `over_400.txt`; one `returning to` parser (steps.rs:148-153 vs :306-309).
- **Major 2 (question):** back-to-back rebases with no checkout between fold into one run (steps.rs:131-134). First a
  real-repo test that shows the older tip's fate. Wrong → fix the cause, same test holds it. Right → test stays, say so.
- **Minor 6 (question):** after `u` then `U`, does the old tip read `✗ lost`? Same: test first, fix if wrong.
- **Minor 1:** on the current row the bar reads `⏎ close  esc close`, two keys one word for different actions.
  Conductor's call: `⏎`'s clause on that row uses the same words as `⇥`'s clause (it is `⇥`'s action); `esc close` stays.
- **Minor 2:** `?` help (`graft-ui keymap/table.rs:398`) and the man page: `⏎` switches, or goes back to the graph on the
  current branch. Regenerate the man page with its existing writer.
- **Minor 3, 7, 9:** stale doc comment `graft-ui picker.rs:119`; rewrap long doc lines; rename verb constants.
- **Minor 4, 10:** reword ticket 47 box 8; ticket 47 NOTES say whether picker/hints captures drifted (with the grep/diff).
- **Minor 5:** below 120 columns, every lost row uses the widest label's sha column (`pieces.rs:82-83`).
- Not here: minor 8 (gap M8/47-q2-prior-art), J ASCII tier (gap M8/46-j-no-ascii).

## Acceptance (each box: the test named at tick time)
- [ ] `over_400.txt` has no new rows from 46; the parse exists once (grep quoted).
- [ ] Major 2 real-repo test, named; outcome stated.
- [ ] Minor 6 real-repo test, named; outcome stated.
- [ ] hints test: on the current row `⏎`'s clause equals `⇥`'s; no two clauses share a word.
- [ ] help test: `picker.switch` description names both outcomes; man page regenerated (diff quoted).
- [ ] journal_screen test: at 80 columns `⇢` and `✗` rows' shas start in the same column.
- [ ] captures re-shot where the above changes them (`docs/shots/m8-q1-*`, `m8-q2-*`), for his eye.

## Touches
Callers of the split modules; journal/undo (Major 2, minor 6); hints, keymap §2.16, help, man; captures; MAP.md files.
