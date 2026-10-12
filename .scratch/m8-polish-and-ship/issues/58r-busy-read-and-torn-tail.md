# 58r — a busy journal read must not claim "nothing live"; torn tail cut only under the lock

Status: done
Build with: Opus xhigh (undo area)
Blocked by: none
From: docs/reviews/m8-mergewright-58.md, Majors 1 and 2 (lines verified by the conductor)

## Major 1 — busy read reports `live: []`
`crates/graft-ops/src/apply/recover.rs:47-48` returns `Recovered::default()` when a writer holds the journal;
`crates/graft/src/update/journal/answers.rs:32` then overwrites `live_entries`, which its own comment (:28-31) forbids.
A standing conflict, or the running operation (push/fetch included), is drawn as interrupted
(`update/undo/notice.rs:56-63`, `view/journal.rs:191-198`, `update/banner/stop/wording.rs:206`,
`update/commit_editor/refusal.rs:32`). Regression from 58 for standing conflicts.
Fix the cause (review's sketch, yours to decide): a busy read says so and still works out `live` from git's state (a
read, no lock needed); the app keeps prior `live_entries` on a busy read; a writer-held `Pending` entry is drawn as
running, not interrupted. Wording from existing CONTEXT.md terms; no new key.

## Major 2 — `Journal::open` truncates a torn tail with no lock
`crates/graft-ops/src/journal/file.rs:97-104`. Cut the tail only while holding the journal alone; otherwise ignore it
as `reread` does (`file.rs:164-168`). Whether a reader can see a partial `O_APPEND` write is unverified — say so.

## Touches (check each, name how)
Every `read_journal`/`recover_interrupted` caller; launch read; J screen + footer notice + banner + commit-editor
refusal; undo/redo; captures showing a stopped merge/rebase or J after one (re-shoot any whose text changes).

## Acceptance
- [x] Test, red first: while a writer holds the lock, a read keeps a standing conflict live (name it, quote red)
  — `graft-ops` `it::journal_two_writers::a_busy_read_keeps_a_standing_conflict_live`; red quoted in NOTES.
- [x] Test, red first: while a writer holds the lock, its running entry is not drawn/noticed as interrupted
  — ops: `it::journal_two_writers::a_journal_read_mid_rebase_leaves_the_running_entry_one_outcome` (asserts `held`);
  app: `update::undo::tests::notice::a_running_operations_entry_is_not_announced_as_interrupted` (footer),
  `view::journal::tests::an_entry_a_busy_read_held_is_drawn_running_not_interrupted` (J row). Reds in NOTES.
- [x] Test: torn tail is not truncated without the lock (red first if reachable; else say why)
  — `journal::file::tests::open_leaves_a_torn_tail_alone_while_a_writer_holds_the_journal` (red: reachable in-process);
  plus `journal::file::tests::recovery_alone_cuts_the_torn_tail_a_busy_open_left` (the tail left must still be cut).
- [x] Gates workspace-wide green after last edit; report says cause/symptom and cost to the user
  — fmt --check, clippy --all-targets -D warnings, nextest --workspace 6437/6437 (16 skipped), test --doc ok.

### Added by the conductor (review 58 Minors 1, 2, 8)
- [x] Minor 1: `alone()` (`file.rs:153-154`) does not silently drop this `Journal`'s own claim — error or debug_assert
  — `journal::file::tests::a_writer_asking_for_the_journal_alone_is_refused_and_keeps_its_lock`.
- [x] Minor 2: red test for the resume path (`resume.rs:131`): a journal read mid-resume, after git finishes the continue
  — `it::journal_two_writers::a_journal_read_mid_resume_leaves_the_resumed_entry_one_outcome`.
- [x] Minor 8: one shared helper for `ApplyError::Journal { error, after: None, kept: None }` (`resume.rs:131-135`, `recover.rs:121`)
  — `ApplyError::journal`; no behaviour change, covered by the existing journal-error tests in the green workspace run.
- [x] NOTES carry a `Touches:` line covering the J screen and captures — see **Touches:** below.

## NOTES (builder)

Scout used: `scout` on the app side (`live_entries` readers, J row words, notice, `JournalRead`); ops files read
directly (named by the ticket).

**Major 1 — fix (cause).** The read answered "nothing live" because a busy read skipped git's state read
(`recover.rs`, pre-fix `:47-48`). Now `recover_interrupted` always works out `live` from git (a read, no lock), and
a busy read lists every other `Pending` entry as `Recovered::held` instead of saying nothing (`recover.rs`
`find_live_entry`, `hold_the_rest`).
The app carries it as `JournalRead::held`; the footer notice skips held entries (`update/undo/notice.rs`); a held
J row reads `running`, has no `interrupted` mark, no "a previous run stopped" line and no `⏎` words
(`view/journal.rs` `read_progress`, `view/journal/summary.rs` `Progress::Running`).
*Deviation from the review's sketch, on purpose:* `answers.rs` does **not** keep the prior `live_entries` on a busy
read. The read no longer lies — its `live` is git's answer now, fresher than the prior one — so it is trusted like
any other (comment at `answers.rs`). Keeping the prior would keep a stale `live` after a merge finished in the
shell during a push.

**Major 2 — fix (cause).** `Journal::open` cuts a torn tail only under `FileLock::try_exclusive`, re-reading the
file under the lock (the line may have been finished meanwhile); while any writer holds the journal the tail is
left, as `reread` leaves it, and `torn_tail()` is `None` (`journal/file.rs` `cut_torn_tail`).
A tail so left is cut by the next `alone()` (recovery), which holds the journal exclusively anyway
(`read_cutting_torn_tail`, shared with `open`). Found by this ticket's own spec review: without it, recovery's
`mark_interrupted` spliced its outcome onto the torn bytes and every later read refused the file.
**Unverified:** whether a reader can see a partial `O_APPEND` write at all on APFS or ext4. Not measured; the fix
holds either way.

**Reds (each run before its fix):**
- `a_busy_read_keeps_a_standing_conflict_live`: `left: [] right: [EntryId("01HF7YAT000000000000000000")]`.
- `a_journal_read_mid_rebase_leaves_the_running_entry_one_outcome` (new `held` assert): `left: [] right: [EntryId("01HF7YAT…")]`.
- `a_running_operations_entry_is_not_announced_as_interrupted`:
  `left: Some("an operation was interrupted: push main to origin · J to inspect") right: None`.
- `an_entry_a_busy_read_held_is_drawn_running_not_interrupted`: `left: Interrupted right: None`.
- `open_leaves_a_torn_tail_alone_while_a_writer_holds_the_journal`: the unfinished line `{"id":"01","stat` was cut
  ("a writer holds the journal: its unfinished line is not cut").
- `recovery_alone_cuts_the_torn_tail_a_busy_open_left`: `the outcome is a line of its own: Json { line: 2, what:
  "expected `:` at line 1 column 19" }`.
- `a_writer_asking_for_the_journal_alone_is_refused_and_keeps_its_lock`: `refused, not done: Ok(true)`.
- `a_journal_read_mid_resume_leaves_the_resumed_entry_one_outcome` — red by removing the `claim()` at `resume.rs`
  (the line the test guards): `the read closed an entry its resume is still finishing: [EntryId("01HF7YAT…")]` at
  `journal_two_writers.rs:173`. Claim restored; green. With the claim in place the test was green from the start
  (58 already fixed it); this proves the test guards it.

**Minor 1.** `alone()` no longer drops this `Journal`'s own claim: it refuses with `JournalError::StillWriting`
and keeps the lock. No production caller hits it (recovery runs on the read job's own `Journal`,
`jobs/undo.rs` `read_journal` → `with_context`).
**Minor 8.** `ApplyError::journal(error)` (`apply/error.rs`) replaces the five identical
`Journal { error, after: None, kept: None }` spellings (`resume.rs`, `recover.rs`, `error.rs` PrepareError arm,
`restart.rs`, `apply.rs` `Context::open`). No behaviour change.

**Self-review** (`code-review`, Standards + Spec). Fixed: `alone()` torn tail (Spec, above); a test doc comment
split from its test; verb names (`hold_the_rest`, `find_live_entry`, `read_progress`); app enum `Open` → `Progress`
(clashed with `recover.rs`'s private `Open`; `Open::No` → `Progress::Settled`); `RUNNING` private. Left: the footer
notice (`update/undo/notice.rs`) and `read_progress` both test "live or held" — one in update, one in view; a
shared helper would cross that line for two lines of logic (judgement call).

**Cost to the user.**
- J opened (or a refresh) during a push, fetch or any journaled write now shows that entry `running`, a word J did
  not have before; nothing on the footer.
- A dead run's entry seen by a busy read is drawn `running` too, not `interrupted`, until the first read after the
  writers finish closes it. Nothing tells the two apart under one file lock; `running` is the side that does not
  invite `u` on a live operation. Rare: dead entries are normally closed by the launch read.
- A held entry stays `running` in J until the next journal read. Nothing watches the file: `JournalState::read`
  has four callers — launch (`update/journal.rs:249`), opening J (`update/journal/keys.rs:62`), refresh
  (`update/keys/refresh.rs:30`), an operation's failure (`update/operate/failure.rs:301`); none is a file watch
  (grep). So an entry held because *another window* was writing stays `running` after that window finishes,
  until this app reads again. Before 58r it read `interrupted` in the same case, wrongly.
- Residual (question, not reproduced): a writer that appends while a crash's torn tail is still on disk — every
  `open` since the crash found the journal busy, and no recovery has run alone — splices its line onto the
  wreckage. Needs a crash mid-`write`, then writers running across every read since. Recovery alone now cuts it.
- `running` is a new on-screen word in J. CONTEXT.md has no status term (its entries are internal,
  `CONTEXT.md:9-19`); `running` passes its test ("would a Git user know it the first time"). For the user's eye.
- File-size ratchet (`over_400.txt`) rows raised (production: `apply/error.rs` 554→560, `journal.rs` 404→408,
  `jobs/undo.rs` 813→818 — doc for the new helper, variant and field; the rest test files gaining the `held` field);
  `restart.rs` lowered 407→403. `journal/file.rs` 384→399, under the line.

**Touches:**
- `recover_interrupted` callers: app `jobs/undo.rs` `read_journal` (only one, via `jobs/runner/operations.rs:232`,
  scout + grep) and 13 ops test files — all green in the workspace run.
- `Journal::alone` (now also cuts a torn tail): one production caller, `recover.rs:50` (grep `\.alone()`).
- Launch read (`update/relaunch.rs:120,165`, `LiveOrigin`): reads `live_entries`, now right on a busy first read
  too; covered by the ops test, no new app test.
- J screen: rows, marks, summary, failure line, `⏎` words — `an_entry_a_busy_read_held_is_drawn_running_not_interrupted`.
  `⏎` itself (`update/journal/keys.rs` `enter_graft`) unchanged: on a `Pending` row it opens the revert preview,
  which ops refuses, as on a live row.
- Footer notice — `a_running_operations_entry_is_not_announced_as_interrupted`.
- Banner (`banner/stop/wording.rs:206`), commit-editor refusal (`commit_editor/refusal.rs:32`),
  `shell_merge_known` (`update/undo.rs:234`): code unchanged; they read `live_entries`, which a busy read now
  fills from git — `a_busy_read_keeps_a_standing_conflict_live`.
- Undo/redo: unchanged; `u` on a `Pending` entry is refused in ops as before.
- Captures: every `docs/shots/` text capture is checked by its test (`captures::check_shot` → `graft_fixtures::capture`
  fails on drift); the workspace run is green, so no capture's text changed. None re-shot.
- Keymap: no key. Docs: both MAP.md files. Not done: `docs/design/06-undo-journal-design.md` §6.2 (review Minor 4,
  not in this ticket).
