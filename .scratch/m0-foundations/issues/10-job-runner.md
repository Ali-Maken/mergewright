# m0-foundations/10 — Job runner

Status: done
Build with: Sonnet
Blocked by: 09

Why: "the UI thread never waits on a Job" (CONTEXT.md) is a structural promise; M0 builds the machinery so M1's rev-walks and M4's fetches drop into it.

## Read first
- CONTEXT.md ("Job") and ARCHITECTURE.md ("The shape")
- PROMPT.md Technology ("Concurrency" line)
- DESIGN.md §8.3

## Goal
`graft` (app): a worker pool executing `Command`s as cancellable `Job`s off the UI thread, reporting results back as `Event`s.

## Scope
In:
- Fixed-size worker pool (small; cap at 4 to respect PROMPT.md §7.1's laptop limits), fed by a channel of (JobId, work).
- Cancellation token per job, checked cooperatively; cancelling an unstarted job drops it.
- Results (success, failure, cancelled) sent into the event channel as the `core::Event` job-result variant from ticket 09; the loop feeds them through `update` like any other event.
- A panicking job is caught (`catch_unwind` at the worker boundary), reported as a failed-job event — never kills the app.
- One real use to prove the pipeline: an "open repo" command variant stub that ticket 11 fills in (here it can carry a closure/test work).

Out:
- Real git jobs, progress reporting, job priorities/queues beyond FIFO (later milestones as needed).
- async runtimes — plain threads + channels are enough at M0; adding tokio would be a spec change (Technology section says worker pool; propose via ADR if you disagree).

## Acceptance
- [x] Tests with fake jobs: completion arrives as an event; N jobs on a pool of 2 all complete; cancellation stops a cooperative job promptly; a panicking job yields a failure event and the pool keeps working.
- [x] No test sleeps longer than ~100 ms; total added test time seconds, not minutes; at most 4 worker threads in tests (PROMPT.md §7.1).
- [x] UI thread code path contains no `join()`/blocking wait on a job (review check, noted in code comments).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## What was built
- `crates/graft/src/jobs/mod.rs`: `JobRunner` (pool, FIFO channel queue, `MAX_WORKERS = 4`), `Cancel` token, `Outcome`, the `JobEvents` and `Dispatch` seams, and `shutdown(grace) -> Shutdown::{Clean, Stalled{workers}}` — a bounded `Condvar` wait, never a `join` that can hang. Every job ends in exactly one event; the five outcomes are tabulated in the module doc.
- `crates/graft/src/crash.rs`: `catching()` — `catch_unwind` at the worker boundary with Graft's crash hook stood down for the duration, so a job's panic still writes a crash report but does **not** restore the terminal out from under a live screen. The report's path rides back in `JobFailure::Panicked { report }`.
- `crates/graft-core`: `JobFailure` (`Failed(String)` / `Panicked { message, report }`) and a new `Event::JobCancelled`. Ticket 09's `JobFailed(JobId, core::error::Error)` could not carry a panic or a git error, and `core` may not name `git`/`ops` error types, so a failure crosses as rendered text.
- `crates/graft/src/update.rs`: `AppState.repo_path`; `Ctrl+R` (DESIGN.md §2.1 manual refresh) emits `Command::OpenRepository`; job events reach the status line.
- `crates/graft/src/event_loop/mod.rs`: dispatches every command from `update` to the runner and returns immediately, opens the cwd's repository at startup, shuts the pool down after the terminal is restored and reports a stalled worker on stderr.

## Verified
- 226 tests pass (32 new: 19 in `jobs`, 2 in `crash`, 3 in `event_loop`, 6 in `update`, 3 in `core::job`). Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all clean.
- No test sleeps at all: order is forced with channels; the only durations are failure bounds (a 5 s `recv_timeout`) and the 20 ms grace in the shutdown test. Pools of 1-4 threads; the whole `graft` lib suite runs in ~0.06 s.
- Real run in a pty (80x24, `q` to quit): inside this repository the status line stays `ready`; in a plain directory it reads `job 1 failed: <path> … is not a git repository`. Exit 0, terminal restored, nothing on stderr.
- Not verified: the panic hook's suppression branch under a real installed hook (see KNOWN_GAPS.md).
