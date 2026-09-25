# Scout: resume-words-say-what-happened (68) — at dbf4fcf2

**Exists.** `resume` (`crates/graft/src/update/banner.rs:576-607`) builds `"{verb} {kind}"` (:588-593), sets `operating = Some(id)` (:595), `Busy` (:596), `remote::launched(.., description ..)` (:597-604), pushes `Command::Resume(id, what)` (:605). `resumed` (:686-) checks `operating == Some(id)` (:692), clears `operating`/`remote.running` (:695-696), finish-summary gate (:699-702), `Done` → `resumed_done` (:704-706). `resumed_done` (:762-776): `operation_ended`, `Status::done(applied.entry.description)` (:769), refresh, land on HEAD, finish-summary `Merged(words)` overrides (:772-774).

**Q1 — the sent `Resume` at the answer: none.** `AppState::operating` is `Option<JobId>` only (`update/state.rs:86`); `resumed` gets `(JobId, Operated)` (:686-691). The only trace is the `"abort rebase"` string in `remote.running` (:597-601), cleared at :696 before `resumed_done`. Spec's "field needed" branch applies.

**Q2 — entry.** Graft-begun: `close_resume` returns the op's own entry; status `(true,None)` → `Aborted{"aborted"}`, `(false,None)` → `Committed`, **`(_, Some(error))` → `Aborted{reason_for(error)}`** (`graft-ops/src/apply/resume.rs:344-353`). Shell-begun: `Target::ShellMerge` → `apply(FinishMerge)`, `ShellAbort` → `apply(AbortMerge)` (:191-192), own entries. Could not confirm which `Operated` arm an `Aborted`-with-failure reaches (if `Done`, the table's "Abort → aborted:" would also catch a failed continue).

**Q3 — final skip closes `Committed`:** code comment says so (:351); **no ops test found** (only `finish_merge.rs:259`, a refused skip on a shell merge).

**Q4 — other wordings.** Resolver notice via `ReturnTo::Resolver` (`banner.rs:436,443,463`), not read for abort wording. Journal browser words `Aborted` as `failed: <reason>` (`view/journal.rs:190,218`) — an abort reads `failed: aborted`, no `✓`. `Incomplete` arm (`banner.rs:709-712`) also words `description` without abort. `undo::land_after_undo` not read.

**Q5 — captures.** No capture shows a Graft-begun abort line. `rebase feature onto main ✓` (a continue) in `fix-landing-after-continue-rebase-{120x40:24,187x50:50}`, contact sheet 1 :1165,:1237 — unchanged. Shell abort: `fix-shell-merge-abort-and-undo-120x40.txt:83,124`, TESTING.md:2336 — must stay.

**Keys:** none. **Mismatch:** spec assumes abort ⇔ `Aborted`; failure also yields `Aborted` (:345).

**Model:** Sonnet (small, one function + state field) — group with 69 argues Opus, see 69.
