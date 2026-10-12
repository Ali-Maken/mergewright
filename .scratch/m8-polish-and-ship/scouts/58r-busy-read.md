# Scout 58r: busy journal read (crate graft). Paths relative to crates/graft/src

1. **read_journal**: `jobs/undo.rs:564`. It returns `Result<Arc<JournalRead>, String>`. `JournalRead` is at `jobs/undo.rs:180`, with fields `entries`, `unreadable: Vec<String>`, `live: Vec<EntryId>` and `interrupted_edits`. How the `Recovered` fields map (`:575-597`):
   - `unreadable` is formatted as "desc: why".
   - `live` is cloned.
   - `set_aside` becomes `interrupted_edits`.
   - `closed` and `plans_left` are dropped.
   - `entries` is read from `ctx.journal` after recovery.

   No busy flag exists anywhere. `Recovered` is at `graft-ops/src/apply/recover.rs:131`. The busy path is at `recover.rs:46-48` (`alone()` gives None, so it returns `Recovered::default()`).

   Callers:
   - app: only `jobs/runner/operations.rs:232`, which is reached from `dispatch.rs:60` (`Command::ReadJournal`).
   - `recover_interrupted`, ops tests: rewrite_run:603,683; history_ops:1405,1425,1501; journal_two_writers:78; set_aside:1298; ops_execution_hard:676; stop_hooks:639; ops_execution:101; set_aside_generations:75; set_aside_kill:147; fork_follows:643.
2. **journal_event**: `update/journal/answers.rs:10`.
   - It drops the answer if `journal.reading != Some(id)` (:20).
   - It sets `journal.reading=None` (:26).
   - On Ok:
     - It **replaces** `live_entries` (:32).
     - It merges `journal.unreadable` (:34-38) and `interrupted_edits` (:41-45).
   - In every case it sets `journal.entries = Ready|Failed` (:47).
   - Then it calls `rebuild(state)` (:52), then `relaunch::journal_read_landed` on the first read only (:54), then `clash::refresh_and_land` (:58).
3. **live_entries readers**: `state.rs:290` is the field. Every reader only tests membership or emptiness. None can tell a writer-held entry from one git has live.
   - `undo/notice.rs:62`: the footer notice leaves out entries in the live set.
   - `view/journal.rs:191`: a live entry gets `RowMark::None` with summary "in progress — <continue/abort keys>"; otherwise `is_interrupted()` gives `RowMark::Interrupted` (:199).
   - `view/journal.rs:305`: no ⏎ words on a live row.
   - `relaunch.rs:120,165`: non-empty means `LiveOrigin::PreviousRun`, empty means `OutsideGraft`.
   - `commit_editor/refusal.rs:32`: empty means `Begun::InShell`, otherwise `InGraft`. `undo.rs:234` `shell_merge_known` builds on that (u / abort box).
   - `banner/stop/wording.rs:206`: `has_edits_aside` (live + Pending + set-aside entry).
   - Writers: `operate/outcome.rs:176`, `banner/resume/answers.rs:70,293`.
   - Every reader except the notice and the J row is about a git-live stop. A running entry would land there only if it were put in `live_entries`.
4. **J row words**:
   - Live gives "in progress — {keys}" or "in progress" (`view/journal/summary.rs:17-22`). The keys are the banner's continue/abort, so they make sense only for a git stop.
   - Interrupted gives "interrupted" (`summary.rs:78,154`).
   - Others: "aborted by you · nothing changed", "failed: …", Undone, NothingWritten.
   - No "running"/"under way" row wording exists. In `CONTEXT.md` the only hit is `:130` (Job: "A unit of Git work running off the UI thread…"). It has no glossary term for interrupted/running/stopped.
5. **Notice** (`undo/notice.rs:35`):
   - It returns None if `notice_seen` is set or entries are not Ready.
   - It lists entries that are `is_interrupted() && !undone && !live`.
   - Words:
     - full: "an operation was interrupted: {description}{tail}" (:178)
     - short: "interrupted: " (:166)
     - shortest: "interrupted{tail}" (:201)
     - more than one adds "(and n more)"
6. **Running marker**: `state.rs:101` `operating: Option<JobId>` (the write the screen waits on, set by `operate::claim_write`), plus `:1230` (working-tree write status). Neither holds an `EntryId`. Could not confirm any way to map `operating` to the Pending entry it wrote. Another window's writer: the app has no signal.
7. **Tests**:
   - `update/relaunch/tests.rs:65-80` (`journal_read`) builds `AppEvent::Produced(id, Product::Journal(Ok(Arc::new(JournalRead{..}))))` after setting `journal.reading=Some(id)`.
   - `update/clash/tests.rs:182`.
   - `update/undo/tests.rs:191` `merging()` sets `Loading::Ready` + `live_entries` directly.
   - View: `view/journal/tests.rs:94,241` (Interrupted mark).
8. **Shots containing "interrupted"**:
   - m3-journal-80x24.txt, m3-journal-expanded.txt, m3-journal-160x48.txt, m3-contact-sheet.txt
   - m5-ui-banner-conflict-{80x24,120x40,160x48}.txt
   - m8-mergewright-s1-graph-120x36.txt, m8-contact-sheet-3.txt, m8-contact-sheet-captions.tsv

**Where the ticket's assumption meets the code**:
- A busy read returns `live: []` with `entries` holding the writer's Pending entry.
- `answers.rs:32` then clears `live_entries`, so `notice.rs:58-63` shows it as "interrupted" and `view/journal.rs:197-199` marks it Interrupted.
- On a first read it also sets `LiveOrigin::OutsideGraft` (`relaunch.rs:165`).
