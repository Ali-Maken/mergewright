# graft — map
One line per public item. Update when a public item changes.

## lib.rs
- `cli` (mod) — CLI argument parsing.
- `config` (mod) — config file loading and types.
- `crash` (mod) — panic handling and terminal restore on crash.
- `event_loop` (mod) — the run loop, input, signals.
- `history` (mod) — history walk, layout, streaming.
- `jobs` (mod) — background job runner.
- `refs` (mod) — refs on each commit and ahead/behind, for the ref badges.
- `repos` (mod) — repository store, opened once and cached.
- `repository` (mod) — facts about one open repository.
- `session` (mod) — startup and per-run session state.
- `terminal` (mod) — raw mode and terminal session setup/restore.
- `update` (mod) — app state and the update function.
- `view` (mod) — draws state to a frame.

## session
- `Startup` (struct) — what `main` decided before touching the terminal.
- `Session` (struct) — what Graft is drawing with, for as long as this run lasts.
- `Session::icons` (field) — badge glyph set from `ui.icons`.
- `Session::max_lanes` (field) — lane limit from `graph.max_lanes`.
- `Session::settle` (fn) — resolve capabilities, theme, rendering info at startup.

## refs
- `RefBadges` (struct) — refs on every commit that has one, plus the `HEAD` and stash commits.
- `RefBadges::on` (fn) — the refs on one commit, in drawing order.
- `RefBadges::head` (fn) — the commit `HEAD` resolves to; `None` while unborn.
- `RefBadges::stash` (fn) — the newest stash entry's commit.
- `RefLabel` (enum) — one ref before sanitising: detached HEAD, branch, remote, tag, stash.
- `Upstream` (enum) — a branch against its upstream: none, in sync (remote), apart (counts).
- `AheadBehindCache` (struct) — counts keyed by (branch tip, upstream tip), shared by refs jobs.
- `AheadBehindCache::get_or_count` (fn) — cached counts, walking only for a new pair.
- `read` (fn) — read the repository's refs into `RefBadges` (a job's work).
- `badges` (fn) — the pure half of `read`: refs, HEAD, upstreams, a counter → `RefBadges`.

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
- `Task` (enum) — jobs identified so their results can be told apart from stray ones; `ReadRefs` reads the badges; `ReadWorkingTree` reads working-tree status counts; `PromoteHistory` sorts for ADR-0008 promotion.
- `Product` (enum) — what a job brought back; `Refs` carries `RefBadges` or why they failed; `WorkingTree` carries `WorkingTreeCounts` or why they failed; `TopologicalOrder` carries a sort's order.
- `GitCheck` (enum) — what running `git --version` found.
- `AppState` (struct) — the whole app state.
- `AppState::quit` (field) — set when Graft is leaving; terminal restored before reporting.
- `AppState::status` (field) — what the status line has to say.
- `AppState::size` (field) — terminal's last reported size, in character cells.
- `AppState::repo_path` (field) — repo path; starting point, filled in by `event_loop::run`.
- `AppState::repository` (field) — repository facts once the opening job answers.
- `AppState::system_git` (field) — system git check result once that job answers.
- `AppState::awaiting` (field) — job id whose result is still wanted; stale ones dropped.
- `AppState::promoting` (field) — the topological sort still wanted; any other sort's order is dropped.
- `AppState::history` (field) — the history walk shown on screen, if one was asked for.
- `AppState::refs` (field) — refs on each commit, once the refs job answers.
- `AppState::working_tree` (field) — working-tree status counts, once that job answers.
- `AppState::screen` (field) — which screen is on show; graph by default.
- `AppState::lanes_expanded` (field) — `+` pressed: lanes draw unfolded; a view flag, the rows are untouched.
- `AppState::focus` (field) — Focus mode (`F`; `Esc` leaves it before cancelling anything): rows off the selection's thread fade; paint only.
- `AppState::tooltip` (field) — the open tooltip's absolute row number, set identically by hover and `Enter`; `Esc` closes it first (m1-the-graph/17).
- `Screen` (enum) — `Graph` (default) or `Status`.
- `Screen::toggled` (fn) — the other screen; `F2` swaps.
- `Quit` (enum) — why Graft is leaving.
- `Quit::should_quit` (fn) — is the loop finished?
- `Status` (enum) — status-line state; never reads as an error when nothing is wrong.
- `Size` (struct) — terminal size in character cells.
- `Size::columns` (field) — width in character cells.
- `Size::rows` (field) — height in character cells.
- `Command` (enum) — effects `update` asks the loop to carry out; `ShowRows(JobId, RowRequest)` asks a history job for rows; `ReadRefs` reads refs and ahead/behind off the loop's thread; `ReadWorkingTree` reads working-tree status counts off the loop's thread; `PromoteHistory` sorts a provisional walk's range topologically.
- `CHROME_ROWS` (const) — screen rows that are not the graph; `graft_ui::graph_screen::CHROME_ROWS`.
- `AppState::graph_rows` (fn) — history rows the graph pane shows at the current size; never zero.
- `update` (fn) — the pure state transition function.

## repository
- `RepositoryFacts` (struct) — what one open repository told Graft.
- `RepositoryFacts::path` (field) — working tree root, or the repo dir itself if bare.
- `RepositoryFacts::bare` (field) — no working tree.
- `RepositoryFacts::head` (field) — where `HEAD` points.
- `RepositoryFacts::mailmap` (field) — the repository's resolved `.mailmap`, for the author chip.
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
- `Graph::max_lanes` (field) — `graph.max_lanes`: lanes drawn before folding behind `+N`; default 8, values below 2 are a parse error.
- `DEFAULT_MAX_LANES` (const) — 8.
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
- `view` (fn) — draw the current state to a frame: the graph screen, or the status screen.
- `graph_view` (fn) — build a `GraphView` from the history window, cursor and status; the selection's `Ancestry` over the window lights lanes, fades rows in Focus mode, and pins its columns against folding.
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
- `Wanted` (enum) — rows a request names: `Rows(range)` or `Last(n)` (walks to the end first).
- `View` (struct) — `follow` (every parent / first-parent) and `inclusion` (folded merges, hidden stashes).
- `RowRequest` (struct) — `id` (increasing; only the newest answer is wanted), `wanted`, the `view` they are wanted in, and `promote` while a promotion swaps in.
- `Promote` (struct) — a topological `order` to swap to, the selection's `anchor` and the viewport `height`; the worker lays the new layout out to the anchor before swapping (ADR-0008).
- `Promotion` (enum) — where promoting to topological order is: `Sorting`, `Swapping { order, anchor }`, `Kept` (stopped or failed; order stays provisional).
- `topological_order` (fn) — the whole range in strict topological order (a job's work); `Ok(None)` when stopped.
- `Views` (struct) — one `StreamingLayout<Included<_>>` per `Follow`, opened on first ask; `Views::new`, `Views::layout` (opens or re-includes), `Views::promote` (lay a promoted order out to the anchor, then swap; idempotent).
- `Open` (type) — boxed closure opening a fresh source over the same walk, or over a promoted order when given one.
- `Window` (struct) — rows answering one request: `request`, `first`, `rows`, `laid_out`, `done`, `promoted` (the first answer in a promoted order); `end()`.
- `HistoryUpdate` (enum) — what a walk tells the screen: `Started(facts)`, `Progress { request, laid_out }`, `Window(window)`.
- `Phase` (enum) — where a walk is.
- `History` (struct) — history on screen: one bounded window, never all rows.
- `History::walk` (field) — the walk's facts, once it has said.
- `History::window` (field) — rows last answered for the newest request.
- `History::rows` (field) — rows laid out, as far as the screen has heard.
- `History::phase` (field) — where the walk is.
- `History::top` (field) — row at the top of the viewport.
- `History::cursor` (field) — the selection: a row number in the current view, so the same commit across scrolls, arriving chunks and eviction.
- `History::reanchor` (field) — commit a view change looks for in the next window, to put the cursor back on it.
- `History::asked` (field) — id of the newest request.
- `History::jump` (field) — the `G` request still walking, if any.
- `History::promotion` (field) — where promoting a provisional order is, if it has begun.
- `History::promote` (fn) — hand the walk a topological order, returning the request carrying it (`None` if the walk has gone).
- `History::view` (field) — the view every request asks for; kept across refresh.
- `History::change_view` (fn) — switch view, returning the request that shows it (a `Follow` switch goes to the top; the selected commit is re-found in the rows that come back).
- `History::selected` (fn) — the row under the cursor, if it has arrived.
- `History::row_at` (fn) — the row at an absolute row number, if it has arrived; what `selected` and a mouse hover both ask for.
- `History::select_at` (fn) — move the cursor straight to a row (a click), clamped to what has laid out.
- `History::scroll` (fn) — move the viewport by N lines without moving the selection (the wheel).
- `History::started` (fn) — a walk asked for but that has sent nothing yet.
- `History::apply` (fn) — take in one update; answers to any but the newest request are dropped.
- `History::show` (fn) — request filling the viewport where it is (start, resize), if needed.
- `History::move_cursor` (fn) — move by a `Motion`, returning the request for rows now needed.
- `History::withdraw_jump` (fn) — withdraw a pending `G`, returning the replacement request.
- `History::visible` (fn) — rows on screen for a viewport height.
- `Motion` (enum, re-export from scroll) — `Down(n)`, `Up(n)`, `PageDown`, `PageUp`, `Top`, `Bottom`.
- `Ended` (enum) — how `stream`/`serve` ended when not failing: `Cancelled`, `NobodyListening`.
- `stream` (fn) — open the walk and serve row requests from a channel until it closes.
- `serve` (fn) — answer `RowRequest`s through `Views`, each in the view it names, until the channel has no sender.

## history::source
- `WalkSource` (struct) — a walk paired with the repo handle it re-reads from, one worker thread.
- `WalkSource::new` (fn) — build a source that streams `walk` and re-reads from `handle`.
- `WalkSource::in_order` (fn) — build a source that streams a known order of ids, reading each commit.

## history::seeds
- `seeds_for` (fn) — compute walk seeds from refs and branches; symbolic refs excluded.

## working_tree
- `read` (fn) — read working-tree status counts off the repository store (a job's work).
