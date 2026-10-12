# 58r — a busy journal read must not claim "nothing live"; torn tail cut only under the lock

Status: ready
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
- [ ] Test, red first: while a writer holds the lock, a read keeps a standing conflict live (name it, quote red)
- [ ] Test, red first: while a writer holds the lock, its running entry is not drawn/noticed as interrupted
- [ ] Test: torn tail is not truncated without the lock (red first if reachable; else say why)
- [ ] Gates workspace-wide green after last edit; report says cause/symptom and cost to the user
