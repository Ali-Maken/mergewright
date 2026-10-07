# Scout 13c — crates/graft (stop / exit line / forced job flow)

Paths relative to `crates/graft/`.

## 1. Exit line
- Built: `event_loop::exit_wait::stopped_line(what, after)` src/event_loop/exit_wait.rs:101-105 (`after.as_secs()`); re-exported src/event_loop/mod.rs:50.
- Printed, two paths:
  - Stop box `y`: `Exit::StoppedHalfWay{what,after}` → src/main.rs:72-79 (stderr, `unwrap_or(())`, SUCCESS).
  - Shutdown forced (60 s bound / 2nd signal): `Pool::stop_to` loops `shutdown.forced`, src/event_loop/exit_wait.rs:81-93, `after = began.elapsed()` (began :60).
- Asserting the words: tests/it/quit_waits/signal.rs:142-147 (`a_signal_restores_then_waits_and_a_second_stops_git`, "stopped merge main into feature half-way"), :171-180 (`stop_now_after_the_real_offer_…`, "… half-way after " + "here shows where it stopped"). src/event_loop/tests/quit_offer.rs:137-145 asserts `Exit::StoppedHalfWay` variant only. Box words "quit now stops it half-way · the next launch shows where it stopped": src/update/quit.rs:279, asserted src/update/quit/tests/stuck.rs:105. **No unit test of `stopped_line` text found** (grep "half-way after").

## 2. JobRunner
- `force_stop(&self, id)` src/jobs/runner.rs:484 → `Live::force_stop` src/jobs/pool.rs:83 (removes mark, `token.cancel()`). Dispatched from `Command::ForceStop` runner.rs:1319; pushed only at src/update/quit.rs:182.
- `shutdown(self, grace: Duration, writes: Duration, force: &dyn graft_git::run::Stop) -> Shutdown` runner.rs:517-550: cancel_all, drop queue, `wait_for_unstoppable` (pool.rs:159) else `force_all_unstoppable` (pool.rs:95), then `alive.wait_for_all(grace)`, join workers.
- `Shutdown { stalled: usize, forced: Vec<String> }` pool.rs:235-247; `CLEAN` :251.
- `SHUTDOWN_GRACE = STOP_WITHIN + 500 ms` src/jobs/mod.rs:118; `EXIT_WRITE_WAIT = 60 s` mod.rs:126.
- Forced job's answer: pool waits only for the **worker thread** (`alive`), not the answer. `run_job` pool.rs:326: `Ok(_) if cancel.is_cancelled() => JobCancelled(id)` — the `Operated` (incl. a `Conflicted`) is **discarded** for any cancelled job. After the loop exits nobody reads events anyway (Delivery::NobodyListening pool.rs:280).
- Merge/op apply job: runner.rs:948 `Product::Operated(run_operation(..))`; `run_operation` src/jobs/operate.rs:271; ApplyError→`Operated` in `answer` operate.rs:361-425 (`Conflicted` :384, `ResumeFailed` :392, `Clashed` :405, rest `Failed`).

## 3. exit_wait / wind_down
- `Pool::stop(self, signalled)` exit_wait.rs:53 → `stop_to` :59 (waiting_line per write if signalled :61-71; `again = signals.again()` :73 is the 2nd-signal `force`).
- `wind_down` src/event_loop/startup.rs:424-439: restore terminal, then `pool.stop(signalled)`; signalled = `Exit::Signal`.
- `Quit::StoppedHalfWay{what}` (src/update/state.rs:845) → `Exit::StoppedHalfWay` at src/event_loop/running/turn.rs:368-381; `after = quit_clock.after(wakes.now()).unwrap_or(ZERO)` :373-376 (seconds since the wait began, loop clock).
- On the `y` path the mark was lifted by ForceStop, so `shutdown.forced` is empty and only main.rs prints.

## 4. Conflicted landing
- `Operated::Conflicted` → `update::operate::outcome::stopped` src/update/operate/outcome.rs:154 (arm :375); also finish/answers.rs:167, banner/resume.rs:181,397.
- Exhaustive match on ApplyError: only `refresh_after_failure` src/jobs/operate.rs:206-250 (`ApplyError::Stopped => Some(Refresh::WorkingTree)` :248). `answer` has a catch-all `Err(error)`.

## 5. Banner words
- `"c continue  a abort"` (two spaces): src/update/banner/pressed.rs:112,147,169; focus_keys.rs:190; commit_editor/refusal.rs:148. Relaunch banner text tested via frame: tests/it/quit_waits/stuck.rs:313-322 checks "from a previous run", "edits aside", "c continue", "a abort". Could not confirm the single draw site of the relaunch banner (in graft-ui, not scouted).

## 6. Tests
- src/jobs/tests.rs:1017 `the_shutdown_grace_covers_a_git_childs_stop` (`SHUTDOWN_GRACE > STOP_WITHIN`); :1027 `no_git_child_outlives_shutdown`; forced: :911 `shutdown_stops_a_write_still_running_at_its_bound_and_names_it`, :938 `a_forced_shutdown_stops_the_write_without_waiting_out_the_bound`, :298, :858.
- tests/it/quit_waits.rs + dir `quit_waits/{signal,stuck}.rs` (`mod signal; mod stuck;` :260-261).
- stuck.rs:208 `stop_now_mid_merge_stops_git_and_leaves_no_lock`: hook `hang(repo,"pre-merge-commit")` (:55-65, writes git.pid/hook.pid, `exec sleep 600`); offer injected by `graft::update::quit::waited_long` inside `walk_to_the_stop` :129-160; `close_within` :163 calls `jobs.shutdown(SHUTDOWN_GRACE, EXIT_WRITE_WAIT, &())`; asserts `Shutdown::CLEAN`, no locks, 1 merge entry. **Contradiction with ticket expectation noted in-code** :244-256: no MERGE_HEAD (hook runs before it), entry closes `Aborted{"git was stopped"}`; only eprintln'd, not asserted.
- `within_the_note(id, keys)` stuck.rs:35-50, reads `docs/design-notes/m8-polish-and-ship.md` journey-counts fence via `m5_walk::parse_counts`.
- stuck.rs:280 `stop_now_mid_rebase_with_edits_set_aside`: `post-checkout` hang, asserts rebase dir kept, entry `Pending`, `put_back` set, banner words.
- quit_waits.rs:205 `capture_quit_stop_now`: `waited_long` injected, `captures::frame_at` 80x24 fixed clock, `captures::check_shot("m8-quit-stop-now-80x24.txt", …)` (regen `GRAFT_UPDATE_SHOTS=1`).

## 7. update::quit
- src/update/quit.rs (`STOP_OFFER_AFTER` :46, `waited_long` :90, `stop_box` :273); tests src/update/quit/tests.rs with `mod stuck;` :890 → quit/tests/stuck.rs; also quit/tests/modals.rs.
