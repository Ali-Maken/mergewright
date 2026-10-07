# Scout 13b review fixes: quit, shutdown, signals, loop clock

Paths are relative to `crates/graft/src/` unless noted.

## 1. Exit sequence (tty path)
- `main.rs:51` `start()`, then `main.rs:97-98`: `Terminal::open()` → `event_loop::drive` (`event_loop/places.rs:101`). `startup.rs:189 run()` is used only for non-tty (`main.rs:85`) → `run_headless`.
- `places.rs:126` `run_repo(...)` (`startup.rs:258`). Inside it, in order:
  - `JobRunner` is created at `startup.rs:279` (a local `jobs` owned by `run_repo`).
  - Loop: `pump_with` at `startup.rs:310`.
  - `drop(fences)` at :324. Then `hand_back_input` at :325. Then `drop(terminal)`, which shows the cursor, at :328.
  - **`jobs.shutdown(jobs::SHUTDOWN_GRACE)` at :333**. Then the signal and resize watchers `close()` at :338-340. Returns `Ran{outcome,shutdown}`.
- Graphics/Kitty frees: in `pump_with` after the loop, `release(terminal, graphics)` at `running.rs:144`. This runs **before** shutdown.
- `places.rs:138` / `:151-152`: `term.close(ran)` (`startup.rs:159`) → `wind_down` (:409). It calls `session.restore()` at :416, then `report_stalled_jobs` at :417 (stderr "Stalled" line, :503), then `finish` (:452-490).
- Fatal message: `main.rs:66-68` `report(&reason)` to stderr after restore. `Quit::Fatal` → `Exit::Fatal` at `running/turn.rs:338`.
- Signal: `main.rs:57-63` `signals::reraise(signal)` (`signals.rs:128`) = `signal_hook::low_level::emulate_default_handler`. That restores the default disposition and raises.

## 2. Signals (`event_loop/signals.rs`)
- signal-hook `Signals::new(WATCHED)` self-pipe (:56) on thread `graft-signals` (:62). It sends `Wake::Shutdown(signal)` per signal (:64). `WATCHED=[SIGINT,SIGTERM,SIGHUP]` (:47).
- The loop acts on it at once: `turn.rs:205` (`held` at_once) and `turn.rs:231` → `Exit::Signal`. Same in `places.rs:296` and `headless.rs:57`.
- After the loop returns, nothing reads `wakes` (the `ChannelWakes` receiver is consumed by `hand_back_input`, `startup.rs:325`).
- `watcher.close()` comes only after `jobs.shutdown` (:338). A second signal during shutdown goes to the self-pipe, and the thread's `send` fails (:64-67 → return). **It is not observed by anyone.** Handlers stay signal-hook's until `reraise`.

## 3. Wake/deadline machinery
- `Wakes` trait: `event_loop/mod.rs:115`; `next_wake_by(deadline)->Waited` :125; `now()` :151 (default `Instant::now`). Real impl `ChannelWakes::next_wake_by` :237 (`recv_timeout`).
- The loop body is `Turn` in `running/turn.rs`. `run` :105 loops `turn()` :118.
  - Deadlines are computed in `turn` :119 (`rest_deadline` :293 → `Rest::deadline`, `rest.rs:121`), `idle` :150 (blocks `next_wake()` :166 when no rest deadline; `next_wake_by(at)` :160), `paced` :179 (min of `pacer.deadline()` `pace.rs:113` and rest).
  - Fence wait is at :355. `Pacer` struct is at `pace.rs:35`.
- Adding a deadline: there are three blocking sites (`turn.rs:160`, `:166`, `:182`, plus :356). The idle `next_wake()` at :166 has no deadline today.
- `self.state: &mut AppState` is on `Turn` (`turn.rs:39`), so `self.state.quit_when_done` is readable in `turn()`. The quit match is at `turn.rs:334`.
- Test timeline: `event_loop/tests/settle.rs:35 Timed`. `next_wake_by` (:91) advances `now` to the deadline when no step is due. `now()` :122.

## 4. Jobs
- `SHUTDOWN_GRACE = STOP_WITHIN + 500ms` (`jobs/mod.rs:117`). `STOP_WITHIN` is at `graft-git/src/run.rs:59`.
- `JobRunner::shutdown(self, grace)->Shutdown` is at `runner.rs:493`. Order: clear row_requests, `live.cancel_all()`, `drop(queue)`, `live.wait_for_unstoppable()` (**unbounded**), `alive.wait_for_all(grace)`, join.
- `Shutdown{Clean, Stalled{workers}}` is at `pool.rs:157`.
- `Live` (`pool.rs:37`):
  - `register` :54
  - `finish` :58 (notifies `ended`)
  - `unstoppable(id)` :66
  - `cancel(id)` :72 (skips unstoppable)
  - `cancel_all()` :83
  - `wait_for_unstoppable()` :96 (Condvar, no timeout)
- `JobRunner::cancel` :454 and `unstoppable` :473. Dispatch `Command::Unstoppable` is at :1287.
- Token → git: `Cancel(Arc<AtomicBool>)` (`jobs/types.rs:40`). `impl Stop for Cancel` (`jobs/detail.rs:42`). The `Stop` trait is at `graft-git/src/run.rs:93` and is polled by `GitInvocation::run`.

## 5. Exhaustive matches
- `Quit` enum `update/state.rs:831` (Requested, HandedBack, Fatal).
  - Exhaustive match: `running/turn.rs:334-340`.
  - `should_quit` is `is_some()` (`state.rs:846`).
  - `headless.rs:70` uses `if let`.
- `Command` enum `update/events/command.rs:17`. Exhaustive-looking matches:
  - `jobs/runner.rs:1244` (`dispatch`)
  - `update/place/write.rs:~30-45`
  - `runner.rs:269` and `:335` are sub-dispatches; whether they are exhaustive or wildcard was *could not confirm*.

## 6. Tests
- `jobs/tests.rs`:
  - `PATIENCE=5s` :20
  - `watching_job` :756
  - Tests: `shutdown_gives_up_on_a_job_that_ignores_its_token` :298, `shutting_down_an_idle_pool_is_clean` :339, `an_unstoppable_job_ignores_a_cancel_by_id` :780, `shutdown_waits_for_an_unstoppable_job_and_never_cancels_it` :805, `the_shutdown_grace_covers_a_git_childs_stop` :858, `no_git_child_outlives_shutdown` :868.
- `crates/graft/tests/it/quit_waits.rs`:
  - Tests: `quit_mid_merge_waits_for_git_then_quits` :87, `ctrl_c_mid_merge_boxes_and_never_cancels` :111, `capture_quit_waits` :148, `shutdown_mid_merge_lets_it_land` :196.
  - Helpers: `merging` :23, `press` :46 (dispatches commands to `app.jobs`).
- Harness `tests/it/app_harness.rs:104 App{pub jobs}`. Methods: `open`, `key`, `press`, `pump(state, what, done)` :243, `close()` :273 = `jobs.shutdown(10s)`, which panics on Stalled.
- The harness drives update + the real `JobRunner`, **not the event loop or signals**. No test found drives `run_repo`'s exit sequence with a real signal.
