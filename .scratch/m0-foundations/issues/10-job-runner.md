# m0-foundations/10 — Job runner

Status: todo
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
- [ ] Tests with fake jobs: completion arrives as an event; N jobs on a pool of 2 all complete; cancellation stops a cooperative job promptly; a panicking job yields a failure event and the pool keeps working.
- [ ] No test sleeps longer than ~100 ms; total added test time seconds, not minutes; at most 4 worker threads in tests (PROMPT.md §7.1).
- [ ] UI thread code path contains no `join()`/blocking wait on a job (review check, noted in code comments).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
