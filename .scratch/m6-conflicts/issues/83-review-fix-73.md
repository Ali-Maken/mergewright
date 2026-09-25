# 83 — review fix for 73 (Critical C1; Major M1, M2; Minor m1, m2, m3, m8, m9)

Status: ready
Blocked by: 73 (merged)
Crates: graft-ops only (src and tests). Runs beside 74 (graft-ui, graft): disjoint.
Source: `docs/reviews/m6-final-fixes-73.md`. Every finding names its lines; re-check each against main.
Build with: builder-opus (git-write tests and one pure count; no new design)
Scout: none. The review names every line, and the 73 scout covers the module.

## Decided (conductor, 2026-09-26)
- m4 (early `?` exits after step 4 don't name `S`), m5 (put back after a step-5 stop?), m6 (`Aborted` for an op that
  took effect): gap rows, question/unverified, M7. Not here.
- m7 (README says a dirty rebase is refused): 74, which owns the words.
- **M1 changes what `SetAside::staged` means.** 74 words the preview row from it. 74 is told; if 74 merged first
  with a test that counts a staged new file as "comes back unstaged", fix that test here only if it is in graft-ops,
  otherwise report `blocked: needs graft` naming it.

## Acceptance
- [ ] **C1:** `set_aside_round_trips` asserts, after the put-back (before any undo), per tree: every edited file's
      bytes are back, and the index shape git actually gives (`git status --porcelain` per row: `MM` → ` M`, staged
      deletion → unstaged, staged rename → `A b` + ` D a`, staged new file stays `A`). 73's tick 3 and NOTES
      (`73-…md:144-145`, `:227-228`) corrected to say what is proven: the edits come back, and exactly this much
      staging is lost. Name the test.
- [ ] **M1:** `SetAside::staged` = staged entries whose staging the put-back loses (modifications, deletions; a
      rename counts its old-path deletion; `Added` does not). Doc at `operation.rs:274-276` says so.
      `sets_aside_table` moves `b.rs` to "stays staged". A real-repo test compares `sets_aside(facts).staged` with
      what `git status` shows after the put-back, per C1's trees. Red first.
- [ ] **M2:** `a_stop_before_each_step_loses_nothing` asserts per step what that step leaves: entries written and
      status, stash list length, the entry's last command, tree dirty or clean. `|| step == 6` removed. 73's NOTES
      row 5 corrected (the op never ran at 5; tree clean, edits in `S`). Red by moving a stop one step (say how).
- [ ] **m1:** a journal failure after step 6 carries the put-back's outcome (kept `S` / clash) in its words. Test.
- [ ] **m2:** an unreadable status after the put-back names `stash@{n}` in the error. Test.
- [ ] **m3:** `Dropped::Unlisted` gets its own `Kept` arm: "your edits are back; the set-aside copy could not be
      found to drop". Test.
- [ ] **m8:** `u` after a kept put-back (the hook test, `set_aside.rs:376`) and after a stop at 4 and at 5: tree,
      index and stash list back to before. Tests.
- [ ] **m9:** `set_aside_round_trips` split per kind (one `#[test]` each over a shared check); none over 5 s alone.
      Report before/after.
- [ ] graft-ops MAP.md updated where the `staged` meaning and `Kept` arms changed.

## Touches (check each, report how)
- `sets_aside` callers: `preview.rs:363`, `apply` (grep). 74's preview row reads `staged` (graft; not yours).
- Existing set-aside tests, the stash undo suite, `live_clash` table (82) — green.
- ADR-0045 Consequences ("A put-back loses the staged/unstaged split"): unchanged; the tests now prove it.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit. A known failure from the base: none (a1c86055 fixed `published_repo`).

## NOTES
