# 23b review-fixes — review 22-23's Critical and Majors

Status: ready
Blocked by: 03-land-on-result (holds graft-ops)
Crates: graft-ops only (`preview/recover.rs`, `apply.rs` `guard_untracked`, `journal_list/steps.rs`, `rows.rs`,
`recover.rs`, `edits.rs`, and their tests). If a fix needs another crate, commit at the seam and report
`blocked: needs <crate>` with the signature.
Build with: Opus (unchanged from 23: git writes on the recovery path).
Scout: none. The review cites every site with file:line against `b09274ea`; re-read each site before you edit it.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-22-journal-recover.md`. No new decision.

## Scope
Each fix is the one the review states, at the cause. Read the review's section for each.
- **C1** (§Critical C1): decide "in the way" from the disk, not from status. For each path `tree_diff(expected, to)`
  adds or changes that the index does not track, a file on disk (ignored included) refuses with `UnsavedEditsTo`, in the
  preview and in `guard_untracked` alike. Precedent: `preview/stash.rs` `occupied`. The review's probe is the red-first
  test.
- **C1 open question:** one probe of whether Graft's `Checkout` overwrites an ignored file the target tracks. Report
  it in NOTES with the test or probe output. **Don't fix it here** if it is exposed; the conductor files it.
- **M1** (§Major M1): `after_head_record` takes a `to` as a branch only when the logs prove it; `record_hint` reads a
  hex `from` as detached (`Hint::Head`); `recover_for` never `Recreate`s a name that exists under `refs/remotes/` or
  `refs/tags/`, offering `restored` instead. Oracle tests: detached at `origin/main`, detached at a tag.
- **M2** (§Major M2): `an_edit_to_a_file_the_undo_never_wrote_is_not_lost`, plus a test for the one-side-unknown arm
  (`edits.rs:102`). Each seen red under the review's mutation.
- **M3** (§Major M3): the three-way match in `recover_scope`; `let left = left_at(…)` in `lost_commit`. No behaviour
  change; existing tests green.

Not in scope: M4 (a spec question, with the planner) and the Minors (they go to KNOWN_GAPS).

## Acceptance
- [ ] C1: the review's probe as a test, red first then green; the preview refuses and `y` refuses. Name the test(s).
- [ ] C1 open question answered in NOTES, with the evidence.
- [ ] M1: both oracle tests red first, then green; the list and `recover_for` output named in the test.
- [ ] M2: both tests named, each red under the stated mutation.
- [ ] M3: done; name the tests that cover the rewritten code.
- [ ] Gates workspace-wide.

## Touches
Every caller of `untracked_in_the_way` and `guard_untracked` (grep), the journal-list oracle tests, 22b's lost_work
fixture tests, the recover tests. 24 draws these rows: the row words change for the M1 case only. Name how each
was checked.
