# Scout 21c B+1: what a cancelled fetch puts on the status line

Paths are relative to `crates/graft/` unless they name another crate.

## 1. Job
- `src/jobs/runner.rs:938-945`: an Operate job returns `Outcome::Cancelled` if the job was cancelled before git started. Otherwise it returns `Outcome::Produced(Product::Operated(run_operation(..)))`.
- `src/jobs/pool.rs:297-325` `run_job`, **confirmed**:
  - cancelled while queued → `JobCancelled` (:300-302);
  - `Outcome::Answered` → `Produced`, even after a cancel (:316);
  - **any other outcome once `cancel.is_cancelled()` → `Event::JobCancelled`** (:319), checked before `Produced` (:321).
- `src/jobs/operate.rs:445` `answer`: `ApplyError::Remote{..}` falls into the catch-all arm (:498-516), which builds `Operated::Failed`:
  - `error = failure_words` → `error.explain(&OpKeys)`. This is the explained string, the stop words included (:525-531).
  - `refresh = Some(Refresh::Everything)` (:309-312).
  - `remote = Some(failure.clone())` (:500-503).
  - `refused: None`, and `edits` from `error.edits_at()`.
  - The `entry` is **not carried**. `Failed` has no entry field (:140-158).
- A Cancelled failure is built as `Operated::Failed`, but pool.rs:319 drops it. **`RemoteFailure::Cancelled` is unreachable as `Operated` through the pool.**
- ops side: `graft-ops/src/apply/remote.rs:317` `explain`, `graft-ops/src/apply/fetched.rs:134` `stopped_words`.

## 2. Precedent: `Outcome::Answered`
- `src/jobs/clone.rs:51-80`: the clone job **always** answers with `Outcome::Answered(Product::Cloned(..))`.
  - Success → `Done`.
  - Any `Err`, stop included → `Ended{words: sanitize_line(error.explain())}`.
  - It never checks `cancel` itself.
- Update: `src/empty_state.rs:250-270` matches `Produced(Cloned(Ended{words}))` for the running id and writes `CloneRun::Ended{words}`. `JobCancelled` stays only for the "stopped before it ran" case (:262-263).

## 3. Update side
**Cancel route.** `src/update/mod.rs:1248` → `job_cancelled` (:1309-1345). For any id, before the write is checked, it does: walk/promotion (:1312-1319), `status_read` (:1321), `refs_read.answered(id,false)` (:1326), `amend_into` (:1328), `rewrite_editor` "cancelled" (:1330), `remote::leaves_job_ended` (:1333), `tour` (:1334). Then:
- `remote::cancelled` (`src/update/remote/in_flight.rs:86-106`) acts only if `operating == id` **and** `cancelling == id`:
  - clears both;
  - sets `Status::done("<running_words> cancelled")`;
  - refreshes `Everything`.
- Otherwise `operate::write_job_ended` (`src/update/operate.rs:52-74`): clears `operating`, `lands` and `resuming`; asks a status read and a detail refresh.
- **Neither branch calls `quit::write_ended`.**

**Answer route.** `src/update/mod.rs:735` `operate_answer` → `src/update/operate/outcome.rs:327` `operated`:
- guard: `operating == id`;
- `operated_answer` clears `operating` (:355) and `remote.cancelling` (:359);
- `Failed` → `operation_failed` (:411-412). In `src/update/operate/failure.rs:40-45`, `edits` set → `edits_status`; else `failure_line` → `remote_failed` (:117-120) when the op is remote. Then refresh at the carried scope (`Everything`).
- `remote_failed` (`src/update/remote/in_flight.rs:166`), arm `(Cancelled, _)` (:183): `Status::done("<op.describe()> cancelled")`. **It ignores `explained`, so the stop words would be dropped.** The comment at :177-182 calls this arm unreachable.
- `quit::write_ended(state, id, done=false)` (outcome.rs:340): clears `quit_when_done` and does not quit.

**If a Cancelled `Operated` arrived:**
- `operating` and `remote.cancelling`: cleared.
- Refresh: Everything.
- Status: Done, not Sticky.
- Wording: the bare "cancelled". The stop words are not used (in_flight.rs:183).
- Exception: if `edits_at()` is `Some`, `edits_status` wins (failure.rs:40-41). Could not confirm whether `edits_at` returns `None` for Remote.

## 4. Quit / forced stop
- The forced answer is taken in `Live::finish` from `token.stopped_at()` (`src/jobs/pool.rs:86-93`). It does not depend on which event is sent.
- `wait_for_forced` waits on `Answer::Awaited` (`src/jobs/forced.rs:99-125`), not on `JobCancelled`.
- `quit::write_ended` is reached only on `Produced` (outcome.rs:340), not on `JobCancelled`.
- `src/update/quit.rs` has no `JobCancelled` match (grep).
- `drop_wait` (`src/update/quit.rs:215`) does not read events.
- Not checked: whether `wait_for_forced` and `quit_when_done` hold different answers for the same job.

## 5. Tests
- Unit: `src/update/remote/tests.rs:391` `esc_cancels_the_running_write_once` (`JobCancelled` → `done("fetch all remotes cancelled")`, :421-423). It tests `remote_failed` at :478 and :493, but no test hits the `Cancelled` arm (grep found no other place).
- It: `tests/it/remote_by_key.rs:765` `esc_cancels_hung_fetch` (:789).
- It: `tests/it/m4_walk/hangs_and_undo.rs:31` `hung_remote_esc_cancels` (:46, `done_containing`).
- Other files mention "cancelled" (`quit_waits.rs`, `credential_by_key.rs`, `m5_walk/remotes.rs`, `m4_walk/credentials.rs`). Not inspected.

## Contradiction
`in_flight.rs:177-182` says the `Cancelled` arm is only for other routes. Its body (:183) never shows `explained`, so the stop words cannot reach the line even through `Produced`.

## Smallest seam
Answer a stopped remote write through `Outcome::Answered` (pool.rs:316), the way clone does. Make the `(Cancelled, _)` arm (in_flight.rs:183) use `explained`.
