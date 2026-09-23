# Rulings on ticket 07's spec questions (acceptance-conflicts)

Source: `.scratch/m6-conflicts/issues/07-acceptance-conflicts.md` §Findings. Consulted:
`docs/design-notes/m6-conflicts.md` (Decision 1, Proposed change 2(c), New UI), ADR-0042,
`.scratch/m6-conflicts/issues/08-ops-undo-an-unmerged-index.md` (Decision 3, Scope 5(c), acceptance).
No source read. Ticket 07 is not edited here; the builder applies these.

## F3 — start over after `⏎` in a merge is unreachable

**Ruling (planner, 2026-09-23):** **The scenario is wrong for M6. No code change in M6.** The design wants `d`
after `⏎` (note, Proposed 2(c); 08's title). 08 built only the ops half (`stage_source`, `RestartConflict` on a
stage-0 path; its Scope says "Out: any key or view"). No ticket ever specified how the app reaches the resolver
for a path that is already staged. 08 Decision 3 keeps `u` refused while an operation is live, so the refusal at
`update/undo.rs:102-108` is correct. A staged path has no `CONFLICTS` row by design. So this is a gap in the
spec, not a defect in the build. Adding an entry point (a row that stays in `CONFLICTS` for the rest of the
operation, or a key on a staged row) is a keymap and New UI decision (CLAUDE.md: keys the doc doesn't name aren't
free). That decision belongs to the M7 design pass, not to an acceptance ruling.
- **Rewrite `start-over-brings-the-markers-back`** to start over **before** `⏎`. Walk: `StoppedMerge`, `⏎` on the
  path's `CONFLICTS` row, `o`, `Esc`, `⏎` on the row again (the path is still unmerged, and the taken block now
  shows as plain lines), `d`, confirm. **What right looks like:** `git ls-files -u` lists the path with the
  **same stage oids and modes as at the stop**. The file on disk has as many `<<<<<<< ` lines as it had at the
  stop. The title's open count equals the file's block count.
- **TESTING.md** names the after-`⏎` half as proven **at the ops level only**, and names the tests:
  `graft-ops` `tests/it/chained_backup.rs::restart_after_stage_brings_the_conflict_back` (git's REUC answers it)
  and `::restart_survives_a_relaunch` (answered by the recorded stage tree). The app's entry point is not built.
- **"Times I left"** gets a residual row: *start a file over after `⏎`, inside an operation → `git checkout -m --
  <path>` in the shell*. Expiry M7.
- **Conductor:** file a KNOWN_GAPS row (broken, not unverified): "`d` after `⏎` has no entry point in the app. The
  ops side is built (m6/08). The entry point needs a keymap and New UI decision. Expiry M7."

## F5 — `c` → merge commit on a merge begun in the shell

**Ruling (planner, 2026-09-23):** **The scenario is wrong. No code change in M6.** The promise's merge ends with
"the operation finished" (design note, Promise). The app can only finish an operation it started:
`graft-ops/src/operation.rs:574-575` refuses the rest on purpose, and that rule dates from M5. M6 did not reopen it.
- **Rewrite `merge-resolved-with-the-keys`** so the **app** begins the merge. Setup: the `StoppedMerge` fixture,
  plus `git merge --abort` in the test's setup, before launch. That is fixture preparation, not a walk step.
  Then the m5 walk's own merge steps start the merge in the app and stop on the conflict. The keys and the
  per-block byte checks stay as the ticket wrote them. `c` from the **banner** (F4 is separate and still
  blocks `c` from the pane). **What right looks like:** `git rev-parse HEAD^2` equals the merged branch's tip;
  `git ls-files -u` is empty; the journal has one `Committed` merge entry; after `u`, `git rev-parse HEAD`
  equals the pre-merge tip.
- **Keep the builder's still-the-shell row** in "Times I left": *finish a merge begun in the shell (`git pull`
  conflict) → `git commit` in the shell*. Expiry M7. This is the user's likely daily case (pull in the shell,
  resolve in Graft), so M7's design pass must answer it.
- **Open question, not a finding:** does the changes pane's commit editor finish a shell-begun merge, since git
  itself makes a merge commit while `MERGE_HEAD` exists? **Unverified; no line.** Before the gap row is worded,
  the conductor checks it once in a throwaway repo, with the result, the git version and the file:line of any
  refusal. Don't reason past it.

## `0 of 0 conflicts open` on whole-file cards

**Ruling (planner, 2026-09-23):** **Defect, textual (a number disagreeing with the picture). Code change: yes.**
The file is still unresolved, and the card below the title asks the user to decide. Yet the title reads as if
nothing is open. The design note gives the title's count only for marker files (New UI mockup). It never gave
a title for whole-file shapes, so this is a gap in the note, and the build filled it literally.
- **What code must do:** in `resolver_view` (the one place this screen's title is built, per the design note),
  drop the count clause when `Opened.file` is `None` (whole-file shape or binary). At ≥100 columns the title
  reads `<path> · file 1 of 2`; at <100 it reads `<path> · 1/2`. The card below already names the shape. Marker
  files are unchanged, including `0 of N open` after every block is taken, which is true and is followed by
  `⏎`.
- **Acceptance for the fix:** a view test at both widths over a `DeletedByThem` card and a binary card asserts
  that the title has no `conflicts open`/`open`. The `m6-02`/`m6-04` captures are regenerated, and their
  captions follow.

## Relaunch shows `1 of 1`, the ticket said `1 of 2`

**Ruling (planner, 2026-09-23):** **Scenario error. Code change: no.** The design note's New UI says: "a taken
block keeps its header … until the resolver closes; reopening shows plain lines". A relaunch is a reopen. The
block taken before the kill is plain lines on disk, so one block remains, and `1 of 1 conflicts open` (`1 of 1
open` at 80) is right. The half-take is proven from git by the existing `half_taken_on_disk` check (one
`<<<<<<< ` on disk).
- **Fix the scenario text and the TESTING.md line** to `1 of 1 conflicts open`. Nothing else changes.
