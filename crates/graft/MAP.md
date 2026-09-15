# graft — map
One line per public item. Update when a public item changes.

## lib.rs
- `cli` (mod) — CLI argument parsing.
- `config` (mod) — config file loading and types.
- `crash` (mod) — panic handling and terminal restore on crash.
- `event_loop` (mod) — the run loop, input, signals.
- `history` (mod) — history walk, layout, streaming.
- `search` (mod) — `/` search over commit subject, author, sha prefix and body (m1-the-graph/26).
- `jobs` (mod) — background job runner.
- `refs` (mod) — refs on each commit and ahead/behind, for the ref badges.
- `repos` (mod) — repository store, opened once and cached.
- `repository` (mod) — facts about one open repository.
- `session` (mod) — startup and per-run session state.
- `terminal` (mod) — raw mode and terminal session setup/restore.
- `update` (mod) — app state and the update function.
- `view` (mod) — draws state to a frame.
- `examples/gallery` (dev-only example, not in the binary) — walks every test repo with `]`/`[`, grouped as a progression; a three-line banner under the header names the stop, group, keys and tier, then `shows:` and `differs:` (m1-the-graph/38); `--list` prints the same lines, `--matrix` writes `docs/shots/35-coverage-matrix.txt` (m1-the-graph/35). Its pure half is `tests/it/gallery.rs`, included by `#[path]`; `Entry::clock()` (m1-the-graph/53) says which clock each stop views its dates at — `Fixed(graft_fixtures::CORPUS_NOW)` for a corpus shape, `Wall` for a real repository — and `show` passes it into the `Harness`.
- `examples/budgets` (dev-only example) — `budgets <repo>` times first paint, G, g, RSS through the real jobs (`tests/it/budget_run.rs`); `budgets --phases <repo>` (m1-the-graph/55, `examples/budgets/phases.rs`) times first paint phase by phase on one thread with no jobs, plus discriminator laps. Each phase prints its peak-RSS and footprint rise; `refs.ref_snapshot` also prints `TipStats` (m1-the-graph/58b). Without a commit-graph `--phases` continues through `examples/budgets/no_cg.rs` (m1-the-graph/59): open, first paint, date walk to end, `topological_order`, promote swap, `g`, each with footprint, interner, order and resident bytes.

## history
- `Wanted::Find { commit, rows, fallback }` — lay out until `commit` has a row, answer around it (search's far hit); `fallback` when the walk ends without it.
- `History::seeking` (field) — the commit a pending `Find` is to select.
- `History::seek` (fn) — select a commit: at once if on hand, else a `Find` request (a jump, so Esc withdraws it).
- `History::restore` (fn) — put cursor/top back (search cancelled) and ask for that window.
- `History::strip_rows` (field) — pixel tier's strip height; requests widen to whole strips plus the row above. `None` (text tiers) asks exactly as before.

## view::strips
- `strips` (mod) — pixel-tier rows: aligned strips covering the viewport, painted like the text tiers.
- `pixel_rows` (fn) — this frame's strips from `AppState`: one fold across strips, uncommitted row on `HEAD`, thread/focus/cursor emphasis; strips missing rows or the row above are left out; all wait while the cursor is in the viewport and its row is missing (no thread yet, 30b); a cursor the wheel left off screen draws unlit (37).
- `PixelRows` (struct) — `strips`, shared `lane_columns`, `marker`, `uncommitted_on_top` (history rows sit one screen row lower). Each painted row carries the fold's stubs into `RasterRow::stubs` (39).
- `StripRows` (struct) — one strip: `first_row`; `row_count`, `uncommitted_at`, `with_strip` (hands a `Strip` to e.g. `StripCache::get`).
- `StripPaint` (struct) — `cell_width`, `cell_height`, `palette`, `muted`; `StripPaint::of(session)`.

## session
- `Startup` (struct) — what `main` decided before touching the terminal; `harness` is what a wrapper around `run` asks for (`update::Harness`, default in the binary).
- `Session` (struct) — what Graft is drawing with, for as long as this run lasts.
- `Session::icons` (field) — badge glyph set from `ui.icons`.
- `Session::max_lanes` (field) — `graph.max_lanes` as configured, or `None` to let the width breakpoint decide (`graft_ui::graph_screen::max_lanes_for` resolves it per frame, m1-the-graph/44).
- `Session::settle` (fn) — resolve capabilities, theme, rendering info at startup.
- `Session::apply` (fn) — take in a re-detected cell size or theme; `true` when anything changed. Never touches the tier.
- `TerminalChange` (enum) — a mid-session answer that changed: `CellSize(CellSize)`, `Theme(ThemeHint)`.
- `Clock` (enum) — `Wall` (default) or `Fixed(i64)`; `now_seconds()` is the one `SystemTime::now()` read on the date path (m1-the-graph/53). `update::Harness::clock` carries it; `view::graph_view` takes the result.

## event_loop
- `Wake::Terminal` (variant) — a re-detected answer changed; `pump` applies it to the session and redraws.
- `Wake::TerminalFailed` (variant) — a re-query could not be written or read; ends the loop with the error.
- `pump` (fn) — idle, while nothing waits, rasterises the strips `PageDown`/`PageUp` would show (30h). Now takes `&mut Session`, so re-detection can update it.
- `input::spawn_reader` (fn) — forwards input; on `FocusGained` also asks the redetector for the theme; returns the `Forwarder`.
- `input::Forwarder` (struct) — the running forwarder; `finish` waits for it to end (after `InputMux::hold`, a bounded drain, never a job wait).
- `signals::watch_resizes` (fn) — resize wakes; each also asks the redetector for the cell size.
- `redetect::Redetector` (struct) — handle to the re-query worker; `spawn(host, cell_size, theme, budget, wakes)`, `ask(Query)`.
- `redetect::Query` (enum) — `CellSize` or `Theme`.

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
- `Task` (enum) — jobs identified so their results can be told apart from stray ones; `ReadRefs` reads the badges; `ReadWorkingTree` reads working-tree status counts; `PromoteHistory` sorts for ADR-0008 promotion; `Search` scans for `/`; `WriteCommitGraph`/`DeclineCommitGraph` the offer's jobs. `Progress::Search` carries one chunk's hits.
- `Product` (enum) — what a job brought back; `Refs` carries `RefBadges` or why they failed; `WorkingTree` carries `WorkingTreeCounts` or why they failed; `TopologicalOrder` carries a sort's order; `CommitGraph(Result<Written, String>)` and `CommitGraphDeclined(Result<(), String>)` the offer's answers.
- `GitCheck` (enum) — what running `git --version` found.
- `AppState` (struct) — the whole app state.
- `AppState::quit` (field) — set when Graft is leaving; terminal restored before reporting.
- `AppState::harness` (field) — `Harness`: a `banner` pinned under the header (its rows come off `graph_rows` and the mouse row math) and `hand_back` keys that end the run with `Quit::HandedBack(key)`; never while the search prompt is open (m1-the-graph/35, 38).
- `Harness` (struct) — `banner: Option<Banner>`, `hand_back: Vec<KeyCode>`, `clock: Clock`, `offer_commit_graph: bool`; `Default` asks for nothing (`Clock::Wall`, offer on; the gallery turns the offer off). `banner_rows()` is the rows the banner takes, 0 without one.
- `Banner` (struct) — `title: String` (the view appends ` · <tier>` for the tier in force), `lines: Vec<String>`; `rows()` is `1 + lines.len()` (m1-the-graph/38).
- `AppState::status` (field) — what the status line has to say.
- `AppState::size` (field) — terminal's last reported size, in character cells.
- `AppState::repo_path` (field) — repo path; starting point, filled in by `event_loop::run`.
- `AppState::repository` (field) — repository facts once the opening job answers.
- `AppState::system_git` (field) — system git check result once that job answers.
- `AppState::commit_graph` (field) — `CommitGraphOffer`, decided when facts arrive (only from `NotNeeded`/`Offered`).
- `AppState::commit_graph_line` (fn) — the offer's footer line, if shown (waits for the git check; no usable git → `NoGitToBuild`); `graph_rows` subtracts its row.
- `CommitGraphOffer` (enum) — `NotNeeded`, `Offered` (`B` → `Command::WriteCommitGraph`, `X` → `Command::DeclineCommitGraph`), `Writing(Option<JobId>)`, `Written { commits }` (cleared by the next key), `Declining(Option<JobId>)`, `Declined`, `Failed` (problem line; no retry this run), `Suppressed`. Ids are `None` until `Started` (m1-the-graph/54).
- `AppState::awaiting` (field) — job id whose result is still wanted; stale ones dropped.
- `AppState::promoting` (field) — the topological sort still wanted; any other sort's order is dropped.
- `AppState::search_prompt` (field) — the `/` prompt's typed text while open; every key but Ctrl-C is text.
- `AppState::search` (field) — the running or finished search, its footer line; Esc cancels a running one (restoring the selection), then closes it.
- `AppState::history` (field) — the history walk shown on screen, if one was asked for.
- `AppState::refs` (field) — `Option<Arc<RefBadges>>`: refs on each commit, once the refs job answers; shared so the per-event state clone never copies them (m1-the-graph/69).
- `AppState::working_tree` (field) — working-tree status counts, once that job answers.
- `AppState::screen` (field) — which screen is on show; graph by default.
- `AppState::lanes_expanded` (field) — `+` pressed: lanes draw unfolded; a view flag, the rows are untouched.
- `AppState::focus` (field) — Focus mode (`F`; `Esc` leaves it before cancelling anything): rows off the selection's thread fade; paint only.
- `AppState::hover` (field) — the absolute row under the pointer, set by mouse movement; drives the footer reveal when set and the cursor row isn't peeking (m1-the-graph/41, ADR-0023).
- `AppState::peek` (field) — `Enter` toggles a one-shot reveal of the cursor row's subject in the footer; any other key clears it, `Esc` clears it before anything else (m1-the-graph/41, ADR-0023).
- `AppState::renderer_override` (field) — the tier-drop key (`F3`) forced this tier for the rest of the run, or `None` for whatever `Session::settle` decided (m1-the-graph/19).
- `AppState::settled_tier` (field) — `Session::settle`'s tier, copied in at loop start; `None` without a session (m1-the-graph/24d).
- `AppState::tier` (fn) — tier in force: F3's override, else settled, else Unicode. F3 (`cycle_tier`) cycles Pixel→Unicode→Ascii→settled tier, never above it; back at the top the override clears. History pads to strips only at Pixel (m1-the-graph/33).
- `Screen` (enum) — `Graph` (default) or `Status`.
- `Screen::toggled` (fn) — the other screen; `F2` swaps.
- `Quit` (enum) — why Graft is leaving: `Requested`, `HandedBack(KeyCode)`, `Fatal(String)`.
- `Quit::should_quit` (fn) — is the loop finished?
- `Status` (enum) — status-line state; never reads as an error when nothing is wrong. `Copied { short }`: `y` sent the SHA (OSC 52); footer notice until the next key; worded as what was sent (m1-the-graph/45).
- `Size` (struct) — terminal size in character cells.
- `Size::columns` (field) — width in character cells.
- `Size::rows` (field) — height in character cells.
- `Command` (enum) — effects `update` asks the loop to carry out; `ShowRows(JobId, RowRequest)` asks a history job for rows; `ReadRefs` reads refs and ahead/behind off the loop's thread; `ReadWorkingTree` reads working-tree status counts off the loop's thread; `PromoteHistory` sorts a provisional walk's range topologically; `Search(Query)` starts a search job. `CopyToClipboard(String)` is terminal output the loop writes itself (through the session's tmux transport; a write error ends the loop), never dispatched to jobs (m1-the-graph/45).
- `CHROME_ROWS` (const) — screen rows that are not the graph without a harness banner; `graft_ui::graph_screen::CHROME_ROWS`. `AppState::graph_rows` adds `Harness::banner_rows`.
- `AppState::graph_rows` (fn) — history rows the graph pane shows at the current size; never zero.
- `update` (fn) — the pure state transition function.

## search
- `Query` (struct) — a case-insensitive literal needle; `parse` (None when blank), `text`, `matches(id, subject, author, body)` (sha prefix only when all hex).
- `SearchState` (enum) — `Running{hits}` / `Done{hits}` / `Cancelled{hits}` / `Failed(String)`; `describe` words each distinctly ("N matches so far", "no matches", "search cancelled, N matches found", "search failed: …").
- `Scanned` (enum) — how `scan` ended: `Finished` or `Stopped`.
- `scan` (fn) — walks every ref chunk by chunk, handing each chunk's hits to `found`; stops on `stop` or when `found` returns false.
- `Search` (struct) — the search on screen: query, owning job, hit batches (`Arc` so state clones stay cheap), current hit, prior cursor/top for Esc, `seeking`/`missed`; `hits`, `hit`, `step(forward)` (wraps), `line` (footer text).

## repository
- `RepositoryFacts` (struct) — what one open repository told Graft.
- `RepositoryFacts::path` (field) — working tree root, or the repo dir itself if bare.
- `RepositoryFacts::bare` (field) — no working tree.
- `RepositoryFacts::head` (field) — where `HEAD` points.
- `RepositoryFacts::mailmap` (field) — the repository's resolved `.mailmap`, for the author chip.
- `RepositoryFacts::commit_graph` (field) — `CommitGraphFact` (m1-the-graph/54).
- `CommitGraphFact` (enum) — `Present { commits }`, `Absent { declined }`, `Unreadable(msg)` (shown as a problem, no offer).
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
- `Graph::max_lanes` (field) — `graph.max_lanes`: lanes drawn before folding behind `+N`. `None` (unset, the default) follows the terminal's width; a set value below 2 is a parse error (m1-the-graph/44).
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
- `RepoStore::refs` (fn) — the refs and their tips (`RefSnapshot`), read once, dropped by `refresh`; first paint's branches, seeds, walk tips and `refs::read`'s badges share it.
- `RepoStore::branches` (fn) — the repository's branches, resolved from `refs`.
- `RepoStore::table` (fn) — the repository's one `Arc<CommitTable>`, opened once, kept across `refresh`.
- `RepoStore::opens` (fn) — how many times the repository has been opened from scratch.
- `RepoStore::refresh` (fn) — mark cached refs and conclusions (e.g. default branch) stale.
- `StoreError` (enum) — why the store could not answer (open, refs, branches, id table).

## cli
- `Invocation` (enum) — what the user asked for.
- `CliError` (enum) — what the arguments could not be made sense of.
- `USAGE` (const) — what `--help` prints.
- `from_process` (fn) — parse the real process's arguments.
- `parse` (fn) — parse an arbitrary argument list.

## view
- `view` (fn) — draw the current state to a frame: the graph screen, or the status screen.
- `view_with_graphics` (fn) — `view`, plus the gfx status in the header at the pixel tier; returns the graph rect the images go in, `None` when no pixels this frame.
- `tier_in_force` (fn) — the graph's tier from `Session` + `AppState`: F3's override, else the session's (m1-the-graph/33).
- `draws_pixels` (fn) — graph screen, pixel tier (override else session), Kitty or iTerm2 protocol — or no acked protocol at all (forced `pixel` attempts Kitty, m1-the-graph/24d).
- `graph_view(state, now_seconds)` (fn) — build a `GraphView` from the history window, cursor and status; the selection's `Ancestry` over the window lights lanes, fades rows in Focus mode, and weighs its columns as a fold priority — not a veto (ADR-0025). `now_seconds` (m1-the-graph/53) is the instant every row's date is fitted against; callers pass `state.harness.clock.now_seconds()`.
- `status_view` (fn) — build a `StatusView` from session and app state.

## event_loop
Internal split (m1-the-graph/71, public paths unchanged): `startup` (terminal
setup, `run`), `headless` (no-tty path, `run_headless`), `running` (the frame
loop, `pump`/`pump_with`), plus the pre-existing `graphics`, `redetect`,
`row_clear` — all private, re-exported from `mod.rs` where still public.
- `input` (mod) — input reader thread and event translation.
- `signals` (mod) — signal watcher thread.
- `Exit` (enum) — why the loop stopped: `Quit`, `HandedBack(KeyCode)` (a `Harness::hand_back` key), `Signal(i32)`, `Fatal(String)`.
- `Wake` (enum) — something that wakes the loop.
- `Wakes` (trait) — a seam so `pump` can be driven by a scripted list in tests.
- `Wakes::waiting` (fn) — is a wake already here, never blocking; the loop asks before each idle-prefetched strip (m1-the-graph/30h). `ChannelWakes` keeps a wake it saw for `next_wake` and `unhandled_input`.
- `ChannelWakes` (struct) — the real wake source: one channel, two feeder threads.
- `ChannelWakes::new` (fn) — build the channel and the sender its producers clone.
- `ChannelWakes::unhandled_input` (fn) — the terminal input queued after the wake that ended the loop, in order; the rest is dropped with the loop.
- `run` (fn) — run the event loop for a session, start to `Exit`; may be called again by a wrapper (the gallery): stdin has one reader per process, and the keys this run never handled go to the next.
- `pump` (fn) — drive the loop; testable independent of a real terminal. Draws Kitty strips at the pixel tier, drains image replies each wake, frees every upload on exit (m1-the-graph/24c). Theme/cell-size changes flush the strip cache; a resize flushes nothing and retransmits nothing — strips re-place, only changed keys rasterise (25). iTerm2 protocol: bands re-sent per frame, encoded once (25).

## event_loop::signals
- `WATCHED` (const) — signals watched: SIGINT, SIGTERM, SIGHUP.
- `watch` (fn) — start the signal-watching thread; returns a `Handle` to stop it.
- `watch_resizes` (fn) — watch `SIGWINCH`, waking the loop with `Event::Resize` at the new size.
- `reraise` (fn) — restore default disposition and re-raise a signal.

## event_loop::input
- `spawn_reader` (fn) — forward the input mux's `Inputs` into the loop's wakes, on a thread of its own.

## jobs
Internal split (m1-the-graph/71, public paths unchanged): `types` (`Outcome`,
`Work`, `Cancel`, `JobEvents`, `Delivery`, `Dispatch`), `pool` (`Shutdown`,
plus the private queue/worker bookkeeping), `kinds` (the work behind each
command), `runner` (`JobRunner`) — re-exported from `mod.rs`.
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
- `Promote` (struct) — a topological `order` (`Arc<[CommitHandle]>`) to swap to, the selection's `anchor` and the viewport `height`; the worker lays the new layout out to the anchor before swapping (ADR-0008).
- `Promotion` (enum) — where promoting to topological order is: `Sorting`, `Swapping { order, anchor }`, `Kept` (stopped or failed; order stays provisional).
- `topological_order` (fn) — the whole range in strict topological order, as handles sharing the store's `CommitTable` (a job's work); `Ok(None)` when stopped.
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
- `History::outstanding` (field) — rows the newest request asked for, until its window lands; a key finding them asked does not ask again. Keys ask only when the window stops holding the viewport plus a screen either side, then for two screens either side, so no key waits on an answer (m1-the-graph/30c).
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
- `WalkSource` (struct) — a walk paired with the repo handle it re-reads from, one worker thread; `pull` yields links (`Walk::next_links`), `read` hydrates by handle through `read_commit`.
- `WalkSource::new` (fn) — build a source that streams `walk` and re-reads from `handle`; handles from the walk's table.
- `WalkSource::in_order` (fn) — build a source that streams a known order of handles, reading each commit; takes the `Arc<CommitTable>` the order's handles were interned against.

## history::seeds
- `seeds_for` (fn) — compute walk seeds from refs and branches; symbolic refs excluded.

## working_tree
- `read` (fn) — read working-tree status counts off the repository store (a job's work).
