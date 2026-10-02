# Scout 13b: quit waits for a write (HEAD e1c8e8e5)

All read from source. Nothing built or run.

## Quit path today
- `q`/`Ctrl+C` from the graph: `update/keys.rs:319-320` `is_quit` → `quit::asked`. Focused banner: `banner/focus_keys.rs:160` (plain `q` only).
- `quit::asked` `update/quit.rs:26-37`: `Ctrl+C` sets `ask = None` (:28-29) → `Quit::Requested` at once. No box, whatever runs.
- `quit::confirmed` `quit.rs:43-52` takes `stops: Option<JobId>`, calls `remote::stop_write`; a `Stoppable::No` write returns early (:47-49), app stays. Ticket's `QuitThen` replaces this.
- Box `Ctrl+C` passthrough: `confirm.rs:159` (test `ctrl_c_with_a_box_open_quits` :838).
- `Stoppable::No` launch sites as review says: `operate.rs:128`, `undo.rs:507`, `preview/answer.rs:271`, `banner/resume.rs:46`; checked in `remote/in_flight.rs:41`.
- Answer choke point (candidate): every `Product::Operated` goes `update/mod.rs:306` → `operate_answer` :647 → `operate::operated` (`operate/outcome.rs:364`). **Could not confirm** that put-back chains (ADR-0045) are a second job id answered here vs. inside one job; builder must check.
- Banner hints: `graft-ui/src/graph_screen/hints/lists.rs:403` `banner_list(skippable, conflicted, merge)`.

## Ctrl+C during a Stoppable::No write (the Question)
Chain, read only:
1. Raw mode: `terminal.rs:86` → crossterm `enable_raw_mode` → `cfmakeraw` (crossterm-0.28.1/0.29.0 `sys/unix.rs:291`/`:302`), which clears ISIG. So in the running app Ctrl+C is a key byte, **not** a SIGINT; the terminal sends no signal to the git child. `signals.rs:42` says the same (SIGINT only when not raw).
2. Key → `quit.rs:28` → `Quit::Requested` → `event_loop/running.rs:360` `Exit::Quit`.
3. `event_loop/startup.rs:192` (not :205 at HEAD) `jobs.shutdown(SHUTDOWN_GRACE)`; grace 200 ms (`jobs/mod.rs:111`).
4. `jobs/runner.rs:476-478` `live.cancel_all()` → `pool.rs:49-52` sets every token, operate job included (no Stoppable check anywhere in this path).
5. Operate job's token is the git stop: `jobs/operate.rs:339` `Context::open(..., cancel)`; `impl Stop for Cancel` `jobs/detail.rs:42`; rebase runs `git.run(ctx.stop)` `graft-ops/src/apply/engine.rs:119` (also `resume.rs:268`, `run.rs:128`, `set_aside.rs:94,143`).
6. `graft-git/src/run.rs:489-496` `should_stop` → `kill(child)` → `exec.rs:235-259`: **SIGTERM** via `kill(1)`, wait up to 2 s (`TERM_GRACE` :202), then SIGKILL.
So yes: Ctrl+C SIGTERMs the git child mid-rebase. Shutdown waits only 200 ms, worker still in the 2 s loop → `Shutdown::Stalled` (`runner.rs:483-489`), worker detached; Graft then exits, and could not confirm whether the SIGKILL fallback ever fires after exit (it can't: the thread dies with the process).
- Process group: `run.rs:373-397` `spawn` sets no `process_group`/`setsid`; child shares Graft's foreground group. Only matters when ISIG is on (startup, or a child left the tty cooked).
- Not confirmed: what git rebase leaves on SIGTERM, and the journal entry state (`graft-ops/src/restore/outcome.rs:134` comments on a SIGTERMed step). Not run.

## File set
graft: `update/quit.rs`, `update/confirm.rs` (ConfirmThen::Quit), `update/state.rs` (AppState field), `update/mod.rs` (operate_answer / escape chain), `update/operate/outcome.rs`, `update/remote/*` (stop_write, status), `update/keys.rs` (Esc chain, could not confirm exact fn line), `MAP.md`. graft-ui: `graph_screen/hints/lists.rs` (+ its tests), `MAP.md`. Plus docs/shots, captions tsv, TESTING.md.
