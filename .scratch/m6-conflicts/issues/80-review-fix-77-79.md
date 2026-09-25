# 80 — review fix for 77, 79 (Major M1, M2; Minor m1, m2, m5, m6, m7, m8, m9, m10, m11)

Status: ready
Blocked by: 70 (graft)
Crates: graft-ops, graft
Source: `docs/reviews/m6-final-fixes-77-79.md`. Every finding names its lines; re-check each against main after 70.
Build with: builder-opus (undo path; mostly moves and tests)
Scout: none. The review names every line.

## Decided (conductor, 2026-09-25)
- m13 (after the first of `U (2 times)` nothing says a second press is owed): **yes, it should say so**, but it needs
  `replaced` carried on the journal entry, a format change. Not here: KNOWN_GAPS row for M7.
- m3, m4, m12: the conductor's (gap rows).

## Acceptance
- [ ] **M1:** `undo_replaced::replaced_names_exactly_what_the_run_took` split into one `#[test]` per scenario over a
      shared `check(scenario)` (or one base repo copied). No test in it over 10 s alone; report before/after times.
- [ ] **M2:** `update/undo.rs` :578-760 (the box's words and fit rule) moved to `update/undo/consequence.rs`, a pure move;
      suite count unchanged. Report non-test line counts of `update/undo.rs`, `consequence.rs`, `jobs/undo.rs` in NOTES.
- [ ] **m1:** D5 and `replaced.rs:16-17` say the true worst case (8 spawns). Measure one drift undo on the 100k fixture
      if the heavy fixture is reachable from the fast tier's helpers, else say "not measured" and why.
- [ ] **m2:** the index comparisons log their commands (pass `commands` through `index_paths_changed`), so the Command
      Log's "every command it ran" holds. Test: the clean-cost test sees the index half.
- [ ] **m5:** the time-machine landing fits 80 columns (the box's left-cut applied to the status path, or drop
      `your edit to <path>` before the key). Add an 80×24 frame of it to `fix-undo-drift-way-back`. Test names it.
- [ ] **m6:** `fix-undo-drift-way-back` frame 1 settles (`settled_frame`, `radar_answered`) after the checkout, before
      `u`; retaken; no `sorting…`.
- [ ] **m7:** the redo landing's sentence moves out of the `keys` slot into `what`. Test.
- [ ] **m8:** a real-repo test drives a whole-tree case through `y` then `U`; the wording for an index-only restore is
      decided in the test's doc comment (say "the whole index" if that is what it is). Report which.
- [ ] **m9:** the unreachable arm in `run_revert_to` removed (or commented why it stays).
- [ ] **m10:** `jobs/operate.rs:174-178`'s refs-for-HEAD test: same hole 79 fixed? Answer with a test, fix it if so.
- [ ] **m11:** 79's tick 3 reworded to name the passing test and the retake.
- [ ] **From 70 (conductor):** retake `docs/shots/m6/m6-18-launch-shell-begun-*` (stale: shows `c continue` on a merge
      since 70's `Banner { merge }`). Then make the design text match what is drawn now, reading the capture, not
      guessing: `docs/design-notes/m6-conflicts.md:258,373,375` and `TESTING.md:2415`. Name the test that writes it.
- [ ] Contact sheets rebuilt for changed captures, each part ≤ 200 000 B. No machine paths.

## Touches (check each, report how)
- Callers of moved items: the build. MAP.md graft and graft-ops.
- Captures: `fix-undo-drift-way-back-*`, `m3-09-*`, m3/m6 sheets and captions.
- TESTING.md rows for walk 13 and :1344.
- KNOWN_GAPS and `docs/design/` are the conductor's.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## NOTES
