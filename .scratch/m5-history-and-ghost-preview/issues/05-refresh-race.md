# refresh-race: `Ctrl+R` before the previous walk's `Started` leaves an old history job running

Status: ready
Blocked by: none
Build with: (blank — brownfield in the app's job routing; recommendation: **Opus** — a concurrency bug whose fix pattern (ids minted by `update`) is the one KNOWN_GAPS row 216 wants everywhere)
**Conductor set after the scouts: Opus.**
Crates: graft
Needs: none

## Scout first (Sonnet `Explore`, ≤ 1 page)

- `crates/graft/src/update/keys.rs` `refresh_key`; `update/mod.rs` where `AppState::awaiting` is set (`HistoryUpdate::Started`?) and read; `history.rs` `Command::OpenRepository`/`ShowRows` and how the history job's id is chosen (runner-issued via `submit`, or `queue_as`?).
- `crates/graft/src/jobs/runner.rs`: `submit` vs `queue_as`; `MAX_WORKERS`; what `cancel` does to a job not yet started (queued) — is it dequeued or run-then-dropped?
- `crates/graft/src/update/settle.rs` (or wherever `settle` lives): what re-requests after a cancel (KNOWN_GAPS row 217, the journal read churn — same shape).
- Report: every site that mints a job id for a history job, and every consumer of `awaiting`.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces (G3 block).
- `docs/adr/0037-m4-checkpoint-decisions.md` §6 (second bullet).
- `.scratch/m4-remotes/issues/07-acceptance-remotes.md` NOTES "Findings" 1 (the observed failure: two resident history jobs fill a two-worker pool; `ReadRefs`/`ReadRemotes`/`ReadWorkingTree` never run).
- `crates/graft/MAP.md` §update (`AppState::awaiting`, `Command`, `LocalJobs`, `LOCAL_JOB_BIT`), §jobs (`JobRunner::submit`/`cancel`), §history (`RowRequest`, `stream`).
- `KNOWN_GAPS.md` rows 216 (id-rewind pattern), 217 (Esc on a journal read re-requests), 473.

## Scope

In:

1. **Cause, not symptom.** `awaiting` is set only when `Started` arrives, so a `Ctrl+R` in the window between dispatch and `Started` cannot name the job to cancel. Fix: `update` mints the history job's id (`LocalJobs::next_id`, `LOCAL_JOB_BIT`) and passes it in `Command::OpenRepository(JobId, …)`; the runner queues it with `queue_as` (the detail pane's pattern). `awaiting = Some(id)` at dispatch. `refresh_key` (and `Refresh::Everything`, and `OpenRepository` on any path) sends `CancelJob(awaiting)` before dispatching the next. A `Started`/`Window`/`Progress` under any other id is dropped (already the rule).
2. **Queued-but-not-started cancel**: if the scout finds `cancel` on a queued job lets it run to completion, make the pool drop a cancelled queued job before it starts (the `Cancel` flag checked at dequeue). Otherwise two `Ctrl+R` still cost a full walk.
3. **Journal read churn (row 217)**: `Esc` cancels a pending journal read and `settle` re-requests at once. Same cause family: `settle` re-requests whenever `entries` is `Idle`; after an `Esc` cancel it should stay `Idle` until the browser opens or a write lands. Make `JournalState::read` cancellation set a `wanted: bool` guard that `settle` respects. Small; same files as the fix's tests if the scout confirms; else leave and say so.
4. Tests (headless app harness, real pool, `tests/it/`): (a) `two_refreshes_before_started_leave_one_history_job`: pool of 2 workers, `Ctrl+R` twice with no wake in between, then `ReadRefs` + `ReadWorkingTree` complete within the walk's normal bound (assert by event order, no sleep) and exactly one `Started` is accepted; (b) `refresh_cancels_the_walk_it_replaces`: the first job's cancel flag is set (observe through the runner's cancel record or a `JobCancelled` event); (c) a `Started` from the cancelled id is dropped and does not overwrite `history` — `stale_started_is_dropped`; (d) row 217: `esc_on_a_pending_journal_read_does_not_rerequest`.
5. MAP.md (`update` `Command::OpenRepository`, `AppState::awaiting`, `jobs`).

Out: refactoring the other 12 id-minting sites (row 216) — name in NOTES which ones share this exact defect, for a M7 housekeeping row; any change to the history walk itself.

## Signatures

```rust
// graft/src/update/mod.rs
pub enum Command { …, OpenRepository(JobId, /* existing payload */), CancelJob(JobId) /* existing */ }
// AppState::awaiting: Option<JobId> — set when the command is built, never first on `Started`
// graft/src/jobs/runner.rs — `queue_as(id, work)` used for the history job (exists for the detail pane)
```

## Acceptance

- [ ] The four tests above green, each named at tick time; none contains `sleep` or a poll loop
- [ ] The M4 walk's `journal-rows` scenario passes with its "wait for Ready before Ctrl+R" workaround **removed** (it was added to dodge this bug: `.scratch/m4-remotes/issues/07` NOTES finding 1) — `m4_walk::every_scenario::journal_rows`
- [ ] Every `Started` handler asserts `id == awaiting` — `stale_started_is_dropped`
- [ ] NOTES: cause or symptom (must be cause), scout used or why not, the row-216 site list

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft` · `cargo test --doc -p graft`.

## Size

One context. graft only; files `update/keys.rs`, `update/mod.rs`, `update/journal.rs`, `history.rs`, `jobs/{runner,pool}.rs` — disjoint from `undo-cancel-rollback`'s `update/remote.rs`/`update/undo.rs` but the same crate: separate worktrees, never one.
