# refresh-race: `Ctrl+R` before the previous walk's `Started` leaves an old history job running

Status: done
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

- [x] The four tests above green, each named at tick time; none contains `sleep` or a poll loop
  - (a) two refreshes with no wake between them leave one walk, and both refreshes' refs and
    working-tree reads come back on a two-worker pool — `it::refresh_race::two_refreshes_before_started_leave_one_history_job`
    (counts the answers; under the bug it starves and fails at the 120 s bound — measured)
  - (b) the replaced walk is told to stop and the pool reports it stopping, with the status line
    unchanged — `it::refresh_race::refresh_cancels_the_walk_it_replaces`
  - (c) a `Started` from the replaced walk does not reach the screen —
    `update::tests::stale_started_is_dropped` (pure, ordering pinned) and
    `it::refresh_race::a_superseded_walks_started_never_reaches_the_screen` (real pool)
  - (d) `Esc` on a read nothing is standing on neither stops it nor makes anything ask again —
    `update::journal::tests::esc_on_a_pending_journal_read_does_not_rerequest`
  - plus `update::tests::a_refresh_before_the_previous_walk_started_still_cancels_it` (the race
    itself, as a pure `update` sequence — no pool, no scheduler) and
    `it::refresh_race::cancelling_a_parked_walk_gives_its_worker_back`
  - No `sleep`, no poll loop anywhere in the new tests: `recv_timeout(PATIENCE)` is a failure
    bound, and every wait is on an event, never on time passing.
- [x] The M4 walk's `journal-rows` scenario passes with its "wait for Ready before Ctrl+R"
  workaround removed — `it::m4_walk::every_scenario::journal_rows` (the `Step::WaitFor(Wait::Ready)`
  between `Step::Write("the pull")` and `Step::Key(ctrl('r'))` in `m4_walk/hangs_and_undo.rs`
  `fetch_pull_commit_push` is gone; all 25 `m4_walk` tests green)
- [x] Every `Started` handler for a walk asserts `id == awaiting` —
  `update::tests::stale_started_is_dropped`; `history_started` returns early on a mismatch and
  `update::history::walk_progress` already gated `Started`/`Window`/`Progress` the same way
- [x] NOTES: cause or symptom, scout used, the row-216 site list — below

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft` · `cargo test --doc -p graft`.

## Size

One context. graft only; files `update/keys.rs`, `update/mod.rs`, `update/journal.rs`, `history.rs`, `jobs/{runner,pool}.rs` — disjoint from `undo-cancel-rollback`'s `update/remote.rs`/`update/undo.rs` but the same crate: separate worktrees, never one.

## NOTES

**Cause, not symptom.** `AppState::awaiting` — the name of the history walk the screen is waiting
for — was only learned from the pool's `Started` event. Between `Command::ReadHistory` going out
and that event being read, the walk had no name, so a second `Ctrl+R` in that window cancelled
nothing and the walk it meant to replace stayed parked on a worker; two of those fill the pool and
every read queued behind them starves. Fixed at the cause: `update` mints the walk's id
(`LocalJobs::next_id`, `LOCAL_JOB_BIT`), passes it in `Command::ReadHistory(JobId, Order)`, the
runner queues it with `queue_as`, and `awaiting` is set at dispatch. Nothing sleeps, polls or
retries; the only durations added are test failure bounds. Cost to the user: none — one refresh
still replaces one walk.

**Scout: yes.** Sonnet `Explore` over the job launch/cancel path, per the ticket (ADR-0035).

**Deviation from the design note (G3 block) and the ticket's Signatures block.** Both name
`Command::OpenRepository(JobId, RepoOpen)`. `OpenRepository` is not the history walk: it is a
separate repository-facts job that refreshes the `RepoStore` on the dispatching thread and that
nothing ever cancels. The walk is `Command::ReadHistory(Order)`, so that is the variant that took
the id. `OpenRepository` is unchanged (still a unit variant), and `keys::refresh_key`'s signature
is unchanged, so **group 08 is unaffected either way** — it reaches this through `refresh_key`,
not through the `Command` enum.

**Scope item 2 (queued-but-not-started cancel) needed no change.** `jobs::pool::run_job` already
checks the token before running the work and reports `JobCancelled` instead, so a cancelled queued
job never costs a walk. Verified by reading it, not assumed.

**A second half of the same cause, found while building, already half-present.** A walk that is
idle parks in `Receiver::recv` and cannot see its cancel token at all; what wakes it is the runner
dropping its row-request sender. That drop existed, but only in `Command::CancelJob`'s dispatch
arm, and it ran *before* the token was set. Both halves now live in `JobRunner::cancel` (token
first), so any caller of the public `cancel` gets it. `it::refresh_race::cancelling_a_parked_walk_gives_its_worker_back`
locks it in; that test would have passed on `main` too — it guards the relocation, it is not
evidence of a new fix.

**Scope item 3 (row 217) done, but not the way the ticket specified, and the difference matters.**
The ticket asked for a `wanted: bool` guard that `settle` respects. Written that way it turns a
churn bug into a loss bug: `update::undo::interrupted_notice` ("an operation did not finish, `u`
can still take it back") only draws from `Loading::Ready` entries, so a `J` then `Esc` during the
launch read would have left that notice absent for the rest of the session. The cause is one step
earlier — `Esc` was cancelling a read the application still wanted, and the blanking to `Idle` is
exactly what `journal::request` re-reads on. `journal::close` now cancels only a read that is
refreshing a list already on hand, and leaves a read with nothing behind it to finish. No guard
flag, no loss, no re-request.

**Row 216 — sites that share this exact defect** (state holds a job id, something cancels by it,
and the id is learned from `Started`), for an M7 housekeeping row:

1. `Task::PromoteHistory` → `AppState::promoting`, set in `update::history::sort_started`; runner
   mints via `announce`. Cancelled by `refresh_key`, `escape_key` and `give_up_promotion` — the
   same window, the same miss. Partly self-healing (`sort_started` cancels the older id when the
   newer `Started` lands), but between the two the pool holds two sorts. The closest sibling of
   the bug fixed here.
2. `CommitGraphOffer::Writing(Option<JobId>)` / `Declining(Option<JobId>)` — ids are `None` until
   `Started` (m1-the-graph/54), by the same pattern.
3. `Task::Search` → `Search::job`, set in `update::search::search_started`. Guarded rather than
   fixed: an `unstarted` counter cancels a superseded start as it arrives.

Runner-minted but **not** sharing the defect, because no state holds their ids and nothing cancels
them: `OpenRepository`, `CheckSystemGit`, `ReadRefs`, `ReadWorkingTree`.

**Also noticed, not fixed (out of scope):** `JobRunner::row_requests` keeps a walk's sender until
a `ShowRows` send fails, a cancel removes it, or shutdown clears the map — so a walk that ends on
its own (`Ended::NobodyListening`, or a failure) leaves an entry behind. Bounded by the number of
walks in a run and harmless, but it is a map that only ever grows on that path.

**How the race was proved closed, and what I could not prove.** Every one of the seven tests was
run red before green by reverting only the behaviour and keeping the signature:
- pure `update`, ordering pinned by construction, not by a scheduler:
  `a_refresh_before_the_previous_walk_started_still_cancels_it` (red: `awaiting` `None` after the
  first refresh), `stale_started_is_dropped` (red: the stale `Started` replaced a 40-row
  `History` with a fresh empty one), `esc_on_a_pending_journal_read_does_not_rerequest`
  (red: `[CancelJob(JobId(1))]`).
- real two-worker pool, ordering pinned because `App::press` dispatches without reading a single
  job event: `two_refreshes_before_started_leave_one_history_job` **starved for the full 120 s
  bound** with the fix reverted, which is the M4 `journal-rows` failure reproduced, and passes in
  0.12 s with it. `cancelling_a_parked_walk_gives_its_worker_back` likewise starved for 120 s with
  the sender-drop removed.
- I did **not** run the heavy tier, fuzzing, benchmarks or a release build; no capture was taken
  (nothing here is visual). Nothing about colour is claimed. The claim I am making is about
  ordering, and it rests on the pure tests, where the ordering is written down — not on the
  integration tests having passed some number of times.

**Gates, workspace-wide, as printed:** `cargo fmt --check` clean;
`cargo clippy --all-targets --message-format=short -- -D warnings` clean;
`cargo nextest run` — `1999 tests run: 1999 passed, 31 skipped`;
`cargo test --doc` — `0 passed; 0 failed` (there are no doctests in `graft`; the gate credits
nothing).

**Other test files touched, and why.** `update/tests.rs` gained a `walk_started(state, id)` helper:
67 tests fed `AppEvent::Started(id, Task::ReadHistory)` for a walk the screen had never asked for,
which the new contract drops. The helper names the walk first, the way `refresh_key` and
`update::opening` do. `app_harness.rs`, `changes_pane_writes.rs`, `commit_editor_writes.rs` and
`budget_run.rs` each had their own copy of the opening command list; all four now call
`update::opening`, so no harness can drift back into the race.
