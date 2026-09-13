# graft — map
One line per public item. Update when a public item changes.

## lib.rs
- `cli` (mod) — CLI argument parsing.
- `config` (mod) — config file loading and types.
- `crash` (mod) — panic handling and terminal restore on crash.
- `event_loop` (mod) — the run loop, input, signals.
- `history` (mod) — history walk, layout, streaming.
- `jobs` (mod) — background job runner.
- `repos` (mod) — repository store, opened once and cached.
- `repository` (mod) — facts about one open repository.
- `session` (mod) — startup and per-run session state.
- `terminal` (mod) — raw mode and terminal session setup/restore.
- `update` (mod) — app state and the update function.
- `view` (mod) — draws state to a frame.

## session
- `Startup` (struct) — what `main` decided before touching the terminal.
- `Session` (struct) — what Graft is drawing with, for as long as this run lasts.
- `Session::settle` (fn) — resolve capabilities, theme, rendering info at startup.

## crash
- `install` (fn) — install a panic hook that restores the terminal before dying.
- `Panicked` (struct) — a panic that was caught rather than allowed to end the process.
- `catching` (fn) — run `work`, catching a panic as `Panicked` instead of aborting.

## terminal
- `SessionOptions` (struct) — what the session turns on at startup.
- `RawMode` (trait) — a `termios` raw-mode switch on the controlling terminal.
- `ProcessRawMode` (struct) — raw mode on the real controlling terminal.
- `TerminalSession` (struct) — RAII guard over terminal modes Graft turns on.
- `TerminalSession::on_stdout` (fn) — set up the real terminal: escapes plus raw mode.
- `TerminalSession::new` (fn) — set up terminal modes, undoing partial setup on failure.
- `TerminalSession::enable_kitty_keyboard` (fn) — turn on the kitty keyboard protocol.
- `TerminalSession::restorer` (fn) — get a handle that can restore from elsewhere (panic hook).
- `TerminalSession::restore` (fn) — put every applied mode back; idempotent.
- `TerminalRestorer` (struct) — restore handle usable from a panic hook.
- `TerminalRestorer::restore` (fn) — put every applied mode back; idempotent.

## update
- `AppEvent` (enum) — events fed into `update`, including job products.
- `Progress` (enum) — what a running job hands over before it ends.
- `Task` (enum) — jobs identified so their results can be told apart from stray ones.
- `Product` (enum) — what a job brought back.
- `GitCheck` (enum) — what running `git --version` found.
- `AppState` (struct) — the whole app state.
- `AppState::quit` (field) — set when Graft is leaving; terminal restored before reporting.
- `AppState::status` (field) — what the status line has to say.
- `AppState::size` (field) — terminal's last reported size, in character cells.
- `AppState::repo_path` (field) — repo path; starting point, filled in by `event_loop::run`.
- `AppState::repository` (field) — repository facts once the opening job answers.
- `AppState::system_git` (field) — system git check result once that job answers.
- `AppState::awaiting` (field) — job id whose result is still wanted; stale ones dropped.
- `AppState::history` (field) — the history walk shown on screen, if one was asked for.
- `Quit` (enum) — why Graft is leaving.
- `Quit::should_quit` (fn) — is the loop finished?
- `Status` (enum) — status-line state; never reads as an error when nothing is wrong.
- `Size` (struct) — terminal size in character cells.
- `Size::columns` (field) — width in character cells.
- `Size::rows` (field) — height in character cells.
- `Command` (enum) — effects `update` asks the loop to carry out.
- `update` (fn) — the pure state transition function.

## repository
- `RepositoryFacts` (struct) — what one open repository told Graft.
- `RepositoryFacts::path` (field) — working tree root, or the repo dir itself if bare.
- `RepositoryFacts::bare` (field) — no working tree.
- `RepositoryFacts::head` (field) — where `HEAD` points.
- `Head` (enum) — where `HEAD` points, one step before the sanitiser.
- `facts` (fn) — read facts about an open repository.

## config
- `Config` (struct) — the config file's settings.
- `Config::ui` (field) — `[ui]`: how Graft looks.
- `Config::graph` (field) — `[graph]`: how history is drawn.
- `Ui` (struct) — `[ui]` section.
- `Ui::theme` (field) — `ui.theme`.
- `Ui::icons` (field) — `ui.icons`.
- `Graph` (struct) — `[graph]` section.
- `Graph::renderer` (field) — `graph.renderer`.
- `Graph::default_branch` (field) — forced default branch full ref name; `None` lets detection decide.
- `ThemeChoice` (enum) — `ui.theme` values.
- `ThemeChoice::overridden` (fn) — the override to hand `ThemeKind::select`.
- `RendererChoice` (enum) — `graph.renderer` values.
- `RendererChoice::overridden` (fn) — the tier the user forced, or `None` for detected.
- `IconSet` (enum) — `ui.icons` values.
- `Loaded` (struct) — what `load` found.
- `Loaded::config` (field) — settings in effect: file values over defaults.
- `Loaded::path` (field) — file the values came from, or `None`.
- `Loaded::warnings` (field) — unrecognised keys, in file key order.
- `ConfigWarning` (enum) — a config problem worth telling the user, non-fatal.
- `ConfigError` (enum) — a config problem that stops startup.
- `load` (fn) — load config from the default location, or defaults if none.
- `load_from` (fn) — load config from an explicit path (tests, future `--config`).
- `default_path` (fn) — where `load` looks: XDG or `~/.config` path.

## repos
- `RepoStore` (struct) — the open repository, and what has been worked out once.
- `RepoStore::new` (fn) — build a store for a path, with an optional forced branch.
- `RepoStore::path` (fn) — the path Graft was pointed at.
- `RepoStore::shared` (fn) — the shared open repository handle.
- `RepoStore::branches` (fn) — the repository's branches.
- `RepoStore::opens` (fn) — how many times the repository has been opened from scratch.
- `RepoStore::refresh` (fn) — mark cached conclusions (e.g. default branch) stale.
- `StoreError` (enum) — why the store could not answer.

## cli
- `Invocation` (enum) — what the user asked for.
- `CliError` (enum) — what the arguments could not be made sense of.
- `USAGE` (const) — what `--help` prints.
- `from_process` (fn) — parse the real process's arguments.
- `parse` (fn) — parse an arbitrary argument list.

## view
- `view` (fn) — draw the current state to a frame.
- `status_view` (fn) — build a `StatusView` from session and app state.

## event_loop
- `input` (mod) — input reader thread and event translation.
- `signals` (mod) — signal watcher thread.
- `Exit` (enum) — why the loop stopped.
- `Wake` (enum) — something that wakes the loop.
- `Wakes` (trait) — a seam so `pump` can be driven by a scripted list in tests.
- `ChannelWakes` (struct) — the real wake source: one channel, two feeder threads.
- `ChannelWakes::new` (fn) — build the channel and the sender its producers clone.
- `run` (fn) — run the event loop for a session, start to `Exit`.
- `pump` (fn) — drive one loop iteration; testable independent of a real terminal.

## event_loop::signals
- `WATCHED` (const) — signals watched: SIGINT, SIGTERM, SIGHUP.
- `watch` (fn) — start the signal-watching thread; returns a `Handle` to stop it.
- `reraise` (fn) — restore default disposition and re-raise a signal.

## event_loop::input
- `spawn_reader` (fn) — start the input-reading thread.
- `translate` (fn) — turn a terminal event into an `Event`, or drop it.

## jobs
- `MAX_WORKERS` (const) — worker thread cap (PROMPT.md §7.1).
- `SHUTDOWN_GRACE` (const) — how long shutdown waits before abandoning workers.
- `Outcome` (enum) — how a job ended, as the job itself sees it.
- `Work` (type) — a boxed, cancellable unit of job work.
- `Cancel` (struct) — cooperative cancellation flag checked by a job's own code.
- `Cancel::is_cancelled` (fn) — has cancellation been requested?
- `JobEvents` (trait) — how a worker reports job results back.
- `Delivery` (enum) — what became of a delivered event.
- `Dispatch` (trait) — hands a command off with nothing a caller waits on.
- `Shutdown` (enum) — what happened when the runner was asked to stop.
- `JobRunner` (struct) — owns the worker pool and hands work to it.
- `JobRunner::start` (fn) — start a pool with a given worker count.
- `JobRunner::with_default_size` (fn) — start a pool sized for this machine.
- `JobRunner::worker_count` (fn) — how many worker threads this pool is running.
- `JobRunner::submit` (fn) — queue work; returns immediately with a `JobId`.
- `JobRunner::cancel` (fn) — request cancellation of a submitted job.
- `JobRunner::shutdown` (fn) — stop the pool, waiting up to `grace` before abandoning.

## history
- `seeds_for` (re-export from seeds) — see history::seeds.
- `WalkSource` (re-export from source) — see history::source.
- `WalkFacts` (struct) — facts about a history walk: order, branches its lanes anchor on.
- `WalkFacts::requested` (field) — the order that was asked for.
- `WalkFacts::order` (field) — the order actually used; may fall back per ADR-0008.
- `WalkFacts::default_branch` (field) — branch owning lane 0, or `None` with no branches.
- `WalkFacts::integration` (field) — branches beside lane 0 that never compete for it.
- `WalkFacts::is_provisional` (fn) — is this order one the user should be told isn't final?
- `HistoryUpdate` (enum) — one thing a running walk tells the screen.
- `Phase` (enum) — where a walk is.
- `History` (struct) — the history on screen: what has arrived of the walk so far.
- `History::walk` (field) — the walk's facts, once it has said.
- `History::chunks` (field) — rows laid out so far, chunk by chunk.
- `History::rows` (field) — how many rows `chunks` holds between them.
- `History::phase` (field) — where the walk is.
- `History::started` (fn) — a walk asked for but that has sent nothing yet.
- `History::apply` (fn) — take in one update from the walk.
- `Ended` (enum) — how a `stream` or `lay_out` call ended, when it did not fail.
- `stream` (fn) — stream a walk's commits, forming errors that cross the job's thread.
- `lay_out` (fn) — lay out commits from a `CommitSource`, reporting failure/repeat/stall.

## history::source
- `WalkSource` (struct) — a walk paired with the repo handle it re-reads from, one worker thread.
- `WalkSource::new` (fn) — build a source that streams `walk` and re-reads from `handle`.

## history::seeds
- `seeds_for` (fn) — compute walk seeds from refs and branches; symbolic refs excluded.
