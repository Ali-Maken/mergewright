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
- `layout` (mod) — the detail pane's saved split, `<common_dir>/graft/layout` (m2-details-and-diffs/10b, ADR-0028).
- `map` (mod) — the Map view's per-branch bucket counts: pure data, no drawing (m2-details-and-diffs/18).
- `repos` (mod) — repository store, opened once and cached.
- `repository` (mod) — facts about one open repository.
- `session` (mod) — startup and per-run session state.
- `terminal` (mod) — raw mode and terminal session setup/restore.
- `update` (mod) — app state and the update function.
- `view` (mod) — draws state to a frame.
- `examples/gallery` (dev-only example, not in the binary) — walks every test repo with `]`/`[`, grouped as a progression; a three-line banner under the header names the stop, group, keys and tier, then `shows:` and `differs:` (m1-the-graph/38); `--list` prints the same lines, `--matrix` writes `docs/shots/35-coverage-matrix.txt` (m1-the-graph/35). Its pure half is `tests/it/gallery.rs`, included by `#[path]`; `Entry::clock()` (m1-the-graph/53) says which clock each stop views its dates at — `Fixed(graft_fixtures::CORPUS_NOW)` for a corpus shape, `Wall` for a real repository — and `show` passes it into the `Harness`. `--group graph|details` picks the progression (m2/23; default graph); `--list` prints both. `tests/it/gallery.rs`: `Group`, `stops(group)`, `keys_label`, `capture_stem`, `GIT_GIT_RELEASE_MERGE`. `--group details` walks M2's 19 key-prelude stops; `--stems` prints their capture stems (`tests/it/gallery.rs::capture_stem`, the coverage matrix's details rows); `--matrix` writes the whole matrix through `tests/it/coverage.rs::write_matrix` — the gallery's two tables (`tests/it/gallery.rs::gallery_matrix`) plus the M3 and M4 walks' tables from the stem constants `coverage::{M3_STEMS, M4_STEMS}`, so no walk is compiled here and `gallery`, `m3_walk` and `m4_walk --matrix` write the same file (m2-details-and-diffs/23, m4-remotes/07 fix).
- `examples/m4_walk` (dev-only example) — M4's acceptance walk (m4-remotes/07): 21 scenarios, each a `graft_fixtures::remote::LocalRemote` (a bare over `file://` and its work clone) or the same bare over `HttpRemote` (Basic auth) or a `HungRemote`, the app driven headless by `Step`s over the real job pool with an askpass helper, then **truth read by running git on both the work clone and the bare**. Same CLI as `m3_walk` (`--only`, `--list`, `--stems`, `--capture`, `--matrix`); the binary also answers **askpass mode** itself (`cli::askpass` → `run::askpass::respond`), so a captured push can ask for a password. Pure half `tests/it/m4_walk/` (split by seam, m4-remotes/07 fix): `mod.rs` (`scenarios()`, `stems_line`, `capture_prelude`, re-exports `Scenario`, `run_scenario`, `prepare_capture`), `fixture.rs` (`RemoteKind`, `Fixture` — `build`, `change_on_remote`, `many_objects_on_remote` — `USER`/`PASSWORD`/`WRONG`/`SECRETS`), `truth.rs` (`Truth`: work `RepoTruth` + bare refs + journal + every status line + the remote's counters; `truth`, `diff_truth`, `Marks`), `steps.rs` (`Step` — `TypeSecret` types a password the walk never prints — `Wait`, `Scenario`, `Capture` with `secret` and `When`), `run.rs` (`run_scenario`, `prepare_capture`, `askpass_helper` — `CARGO_BIN_EXE_graft` under test, `current_exe` in the example — `no_secret_leaked`, run after every scenario: `PASSWORD`/`WRONG` in no event, status line or journal byte), `vocabulary.rs` (the step and assertion helpers), the scenarios by promise (`fetch_pull.rs`, `push.rs`, `credentials.rs`, `hangs_and_undo.rs`) and `every_scenario.rs` (one test per scenario under `cargo test`, `#[cfg(test)]`). The remotes table is `tests/it/coverage.rs::remotes_matrix` over `M4_STEMS`; `--matrix` writes all four tables through `coverage::write_matrix`, so the three `--matrix` commands agree.
- `examples/m3_walk` (dev-only example) — M3's acceptance walk (m3-working-tree-undo-and-stash/07): 22 scenarios, each a `Scratch` copy of a corpus shape, a scripted dirty state, the app driven headless by `Step`s over the real job pool, then repository truth read by running git. `ok <name>` / `FAIL <name>: <≤20 lines>`, exit 1 on failure; `--only <name>`, `--list`, `--stems` (capture stem, scenario, expected text, key label), `--capture <name>` (setup + headless prefix, then the interactive loop with the capture's keys as the prelude — what `pty-walk-m3.py` records), `--matrix` (all four tables, through `tests/it/coverage.rs::write_matrix`). Pure half `tests/it/m3_walk.rs`: `Step`, `Capture`, `Scenario`, `Marks`, `scenarios()`, `run_scenario`, `prepare_capture`; under `cargo test`, one test per scenario (`every_scenario::*`). Git as evidence is shared with the M4 walk in `tests/it/repo_truth.rs` (`RepoTruth`, `git_truth`, `diff_truth`, `git_path`, `run_git`, `neutralise_global_config`, `fsck_clean`, `keys_label`); its table is `tests/it/coverage.rs::working_tree_matrix` over `M3_STEMS`.
- `examples/budgets` (dev-only example) — `budgets <repo>` times first paint, G, g, RSS through the real jobs (`tests/it/budget_run.rs`); `budgets --phases <repo>` (m1-the-graph/55, `examples/budgets/phases.rs`) times first paint phase by phase on one thread with no jobs, plus discriminator laps. Each phase prints its peak-RSS and footprint rise; `refs.ref_snapshot` also prints `TipStats` (m1-the-graph/58b). Without a commit-graph `--phases` continues through `examples/budgets/no_cg.rs` (m1-the-graph/59): open, first paint, date walk to end, `topological_order`, promote swap, `g`, each with footprint, interner, order and resident bytes.

## map
The Map view's per-branch bucket counts (m2-details-and-diffs/18): one bounded walk per local branch into fixed time buckets, built fresh on toggle-on and discarded on toggle-off. No drawing — that is `view::map`, a later ticket.
- `MAX_ROWS` (const, `usize = 200`) — rows drawn before the rest collapse into `MapData::more`.
- `MAX_COMMITS_PER_ROW` (const, `usize = 50_000`) — a row's walk cap; past it the row and `MapData` are `complete = false`.
- `MapRequest { buckets, clock, policy, previous_default_tips }` — what `build_map` is asked to build.
- `MapMark` (enum) — `Tag(Vec<u8>)` or `Radar(Severity)`, one mark per bucket per row (Radar: loudest wins).
- `MapBranch { name, identity, tip, tip_date, counts, is_current, is_default, merged_into, marks, complete }` — one row.
- `MapData { branches, more, from, to, buckets, complete }` — every row plus the shared time span.
- `MapData::bucket_of(date) -> usize`, `MapData::max_count() -> u32`.
- `MapError` (enum) — `Store`/`Walk`/`Read`/`Stopped`; one-line `Display`.
- `build_map(store, req, progress, stop) -> Result<MapData, MapError>` — local branches only (M2); default-branch merges only.
- Split into files, public items unchanged (m2 review M8): `walk.rs` (refs, span, default chain, `walk_branch`/`RowWalk`, one bounded walk per branch), `rows.rs` (`RowContext`, `build_row`/`build_rows`, one row per local branch); this file keeps the public data shapes and `build_map`, the entry point wiring the two together.

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
- `Clock` (enum) — `Wall` (default) or `Fixed(i64)`; `now_seconds()` is the one `SystemTime::now()` read on the date path (m1-the-graph/53). `now_millis()` is the same instant at millisecond resolution, read for the focus-refresh debounce only (m3-working-tree-undo-and-stash/06). `update::Harness::clock` carries it; `view::graph_view` takes the result.

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
- `RefBadges::remotes` (fn) — `(Oid, &[u8])` per remote-tracking branch, short name (`origin/main`): what `history::preview_seeds` colours ghost lanes from (m5/08).
- `RefBadges::stashes` (fn) — `&[(Oid, usize)]`: every stash entry's commit and index, newest first, so each one gets its own badge rather than only `stash@{0}` (m3-working-tree-undo-and-stash/06).
- `RefLabel` (enum) — one ref before sanitising: detached HEAD, branch, remote, tag, `Stash(usize)` (the entry's index, drawn `stash@{n}`; m3-working-tree-undo-and-stash/06).
- `Upstream` (enum) — a branch against its upstream: none, in sync (remote), apart (counts).
- `AheadBehindCache` (struct) — counts keyed by (branch tip, upstream tip), shared by refs jobs.
- `AheadBehindCache::get_or_count` (fn) — cached counts, walking only for a new pair.
- `read` (fn) — read the repository's refs into `RefBadges` (a job's work); reads the stash entries through `graft_git::stash::commits` (reflog only, no `diff-tree`).
- `badges` (fn) — the pure half of `read`: refs, HEAD, upstreams, stashes, a counter → `RefBadges` (the `stashes` argument added m3-working-tree-undo-and-stash/06).

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
- `TerminalSession::suspend` (fn) — restore every applied mode and hand back a `Suspended` naming them, so `$EDITOR` (or a `SIGTSTP`) gets the terminal as Graft found it; a failure mid-way puts back what it took off before returning the error (m3-working-tree-undo-and-stash/05).
- `TerminalSession::resume` (fn) — re-apply exactly the modes a `Suspended` names, nothing else; a mode `suspend` did not find is not turned on.
- `Suspended` (struct) — the modes a `suspend` took off, the only way to `resume` them.
- `TerminalRestorer` (struct) — restore handle usable from a panic hook.
- `TerminalRestorer::restore` (fn) — put every applied mode back; idempotent.

## update
Split into files, public items unchanged (m2 review M8): `mod.rs` keeps only the dispatcher (`update`, `answer`, `core_event`); `geometry.rs` is the pure screen geometry a mouse event needs (`Bounds`, the detail pane's hit-testing, `context_menu_area`); `mouse.rs` is what a mouse event does once `geometry` says where it landed.
- `AppEvent` (enum) — events fed into `update`, including job products.
- `Progress` (enum) — what a running job hands over before it ends. `CredentialNeeded { prompt, pulse_seed }` is git asking for a password or a username through the askpass helper: it opens `AppState::credential` and the job blocks until `AnswerCredential`/`CancelCredential` comes back (m4-remotes/06). `pulse_seed` is where the masked field's keystroke pulse starts, drawn at random per prompt by `jobs::askpass::pulse_seed` and carried on the event so `update` stays pure (m5-history-and-ghost-preview/04, ADR-0038 §2).
- `Task` (enum) — jobs identified so their results can be told apart from stray ones; `ReadRefs` reads the badges; `ReadWorkingTree` reads full working-tree status; `PromoteHistory` sorts for ADR-0008 promotion; `Search` scans for `/`; `WriteCommitGraph`/`DeclineCommitGraph` the offer's jobs; `Operate` one working-tree write (m3-working-tree-undo-and-stash/04). `Progress::Search` carries one chunk's hits; `Progress::Operating { description }` a running write's description; `ListFiles { spec }`, `FileDiff { key }`, `ContainedIn { oid }` the detail pane's reads; `EvilProbe { merge }` the open merge's evil-merge check; `Map` the Map view's build (m2-details-and-diffs/18), announced under `update`'s own id like the detail pane's own jobs. `ReadStashes` the stash list behind the `S` overlay (m3-working-tree-undo-and-stash/06). `Undo` one undo or redo and `ReadJournal` the journal read behind it (m3-working-tree-undo-and-stash/06); both announce a `Progress::Operating { description }` like `Operate`. `ReadRemotes` the remote facts behind `f`/`p`/`P`; `Progress::RemoteProgress { brief }` a remote operation's git progress, already shortened by `RemoteProgress::brief` (m4-remotes/05).
- `Product` (enum) — what a job brought back; `Refs` carries `RefBadges` or why they failed; `WorkingTree` carries `Result<Arc<graft_git::status::WorkingTreeStatus>, String>` — full status, not just counts, shared for the changes pane and the uncommitted-row alike (m3-working-tree-undo-and-stash/04); `TopologicalOrder` carries a sort's order; `CommitGraph(Result<Written, String>)` and `CommitGraphDeclined(Result<(), String>)` the offer's answers; `Files(Result<(Arc<FileListView>, Arc<FileList>), String>)`, `FileDiff(DiffKey, Result<Arc<DiffView>, String>)`, `ContainedIn(Oid, Result<Contained, String>)` the detail pane's answers; `EvilProbe(Oid, EvilVerdict)` one merge's evil-merge verdict (never a `Result` — `NotCheckable` is one of the verdict's own answers); `Map(Result<Arc<MapData>, String>)` the Map view's build, or why not; `Operated(Box<crate::jobs::operate::Operated>)` one working-tree write's answer — boxed (large-variant lint), not a plain `Result`, so `ApplyError::AfterState`'s "it ran, only part of the after-state could not be read" case cannot be collapsed away. `Undone(Box<crate::jobs::undo::Undone>)` one `u`/`U`'s answer — boxed and not a `Result`, for the same reason as `Operated`; `Journal(Result<Arc<crate::jobs::undo::JournalRead>, String>)` the journal as read from disk, entries plus the unreadable ones, and `Stashes(Result<Arc<Vec<graft_git::stash::StashEntry>>, String>)` the stash list behind `S` (m3-working-tree-undo-and-stash/06). `ExternalEditor(Result<String, String>)` carries what `$EDITOR` saved (comments already stripped) or why the handover failed, and `HandedOver(Result<(), String>)` reports a handover with nothing to bring back (`OpenExternalEditor`, `Suspend`) — both produced by the loop, not the pool (m3-working-tree-undo-and-stash/05). `Remotes(Result<Arc<crate::jobs::remote::RemoteFacts>, String>)` the remotes and HEAD's upstream, or why not (m4-remotes/05).
- `GitCheck` (enum) — what running `git --version` found.
- `AppState` (struct) — the whole app state.
- `AppState::quit` (field) — set when Graft is leaving; terminal restored before reporting.
- `AppState::remotes` / `AppState::remote` (fields) — the remote facts `f`/`p`/`P` decide from (`Loading<Arc<jobs::remote::RemoteFacts>>`, read once the repository opens and again on `Ctrl+R`), and `update::remote::RemoteState` (m4-remotes/05).
- `AppState::harness` (field) — `Harness`: a `banner` pinned under the header (its rows come off `graph_rows` and the mouse row math) and `hand_back` keys that end the run with `Quit::HandedBack(key)`; never while the search prompt is open (m1-the-graph/35, 38).
- `Harness` (struct) — `banner: Option<Banner>`, `hand_back: Vec<KeyCode>`, `clock: Clock`, `offer_commit_graph: bool`; `Default` asks for nothing (`Clock::Wall`, offer on; the gallery turns the offer off). `banner_rows()` is the rows the banner takes, 0 without one. `prelude: Vec<KeyPress>` (m2/23): keys pressed for the user before the first frame, fed one at a time by `update::prelude` once the first window, the refs answer and the working-tree status are on hand and nothing an earlier key started is in flight — the radar context, a search, the pane's file list or diff, and since m3/07 an operation (`operating`), a journal or stash read and the commit editor's prefill; empty in the binary.
- `Banner` (struct) — `title: String` (the view appends ` · <tier>` for the tier in force), `lines: Vec<String>`; `rows()` is `1 + lines.len()` (m1-the-graph/38).
- `AppState::stash` (field) — `update::stash::StashState`: the `S` overlay's list, cursor, file preview and save prompt (m3-working-tree-undo-and-stash/06).
- `AppState::status` (field) — what the status line has to say.
- `AppState::size` (field) — terminal's last reported size, in character cells.
- `AppState::repo_path` (field) — repo path; starting point, filled in by `event_loop::run`.
- `AppState::repository` (field) — repository facts once the opening job answers.
- `AppState::system_git` (field) — system git check result once that job answers.
- `AppState::commit_graph` (field) — `CommitGraphOffer`, decided when facts arrive (only from `NotNeeded`/`Offered`).
- `AppState::commit_graph_line` (fn) — the offer's footer line, if shown (waits for the git check; no usable git → `NoGitToBuild`); `graph_rows` subtracts its row.
- `CommitGraphOffer` (enum) — `NotNeeded`, `Offered` (`B` → `Command::WriteCommitGraph`, `X` → `Command::DeclineCommitGraph`), `Writing(Option<JobId>)`, `Written { commits }` (cleared by the next key), `Declining(Option<JobId>)`, `Declined`, `Failed` (problem line; no retry this run), `Suppressed`. Ids are `None` until `Started` (m1-the-graph/54).
- `AppState::awaiting` (field) — the history walk the screen is waiting for; stale answers dropped. **Set when `update` asks for the walk (`Command::ReadHistory`), never first on its `Started` event** (m5-history-and-ghost-preview/05): learning the id late left a window in which a second `Ctrl+R` had no name to cancel, so the walk it replaced stayed parked on a worker and two of those filled the pool. Every `Started` handler for a walk checks `id == awaiting` and drops anything else.
- `AppState::promoting` (field) — the topological sort still wanted; any other sort's order is dropped.
- `AppState::search_prompt` (field) — the `/` prompt's typed text while open; every key but Ctrl-C is text.
- `AppState::search` (field) — the running or finished search, its footer line; Esc cancels a running one (restoring the selection), then closes it.
- `AppState::history` (field) — the history walk shown on screen, if one was asked for.
- `AppState::refs` (field) — `Option<Arc<RefBadges>>`: refs on each commit, once the refs job answers; shared so the per-event state clone never copies them (m1-the-graph/69).
- `AppState::working_tree` (field) — `Option<Arc<graft_git::status::WorkingTreeStatus>>`, full working-tree status once that job answers (m3-working-tree-undo-and-stash/04): shared so the changes pane, the header counts and the uncommitted row read the same `Arc` rather than three copies.
- `AppState::operating` (field) — `Option<JobId>`, the working-tree write the screen is waiting for (`Space`/`s`/`Ctrl+A`/`d`); every other write key is a no-op while it is `Some` (m3-working-tree-undo-and-stash/04).
- `AppState::commit_editor` (field) — `Option<CommitEditor>`, the open commit/amend editor; modal, so nothing behind it takes a key (m3-working-tree-undo-and-stash/05).
- `AppState::commit_draft` (field) — `Option<CommitEditor>`, what `Esc` put aside, so reopening with `c` brings the typed message back; cleared once a commit lands.
- `AppState::external_editor` (field) — `Option<JobId>`, the `$EDITOR` handover in flight; an answer under any other id is dropped, so `Esc` in between cannot let the editor overwrite a retyped draft.
- `AppState::suspend_on_ctrl_z` (field) — `[keys] suspend_on_ctrl_z` (ADR-0002): `Ctrl+Z` stops the process instead of undoing. Off by default.
- `AppState::on_changes_pane` (fn) — `(&AppState) -> bool`; the pane is shown and its spec is `DetailSpec::WorkingTree` — the only place the write keys mean anything, and so the only place the footer hints them (m3-working-tree-undo-and-stash/04).
- `AppState::confirm` (field) — `Option<update::confirm::PendingConfirm>`, the discard confirm box; modal like the context menu (m3-working-tree-undo-and-stash/04).
- `AppState::credential` (field) — `Option<update::credential::CredentialPrompt>`, git's credential question as an in-app prompt; modal over everything, including the confirm box (m4-remotes/06).
- `AppState::screen` (field) — which screen is on show; graph by default.
- `AppState::lanes_expanded` (field) — `+` pressed: lanes draw unfolded; a view flag, the rows are untouched.
- `AppState::focus` (field) — Focus mode (`F`; `Esc` leaves it before cancelling anything): rows off the selection's thread fade; paint only.
- `AppState::hover` (field) — the absolute row under the pointer, set by mouse movement; drives the footer reveal when set (m1-the-graph/41). The `Enter` peek is gone: `Enter` opens the detail pane (ADR-0023).
- `AppState::detail` (field) — `DetailState`, the detail pane; hidden by default (m2-details-and-diffs/06).
- `AppState::local_jobs` (field) — `LocalJobs`, ids for the jobs `update` names itself (the detail pane's reads).
- `AppState::on_working_tree` (field) — the uncommitted-changes row's one-bit selection, beside `History`'s cursor rather than a position of it (m2-details-and-diffs/06b); rules in `update::selection`.
- `AppState::compare_base` (field) — `Option<update::detail::CompareBase>`, the pinned base of a live compare (`x`, m2-details-and-diffs/11); set/cleared by `update::detail::toggle_compare`, followed by `update::selection::follow_selection`.
- `AppState::context_menu` (field) — `Option<update::context_menu::OpenMenu>`: the `.`/right-click popup, if one is open — the row it opened on (`update::context_menu::WORKING_TREE_ROW` for the uncommitted-changes row), the `ContextMenu` the widget draws, and the `HistoryVerb`s its `History(i)` items index into (`context-actions-menu`, m2-details-and-diffs/12; the verbs, m5-history-and-ghost-preview/08 piece 6).
- `AppState::radar` (field) — `update::radar::RadarState`: the Mistake Radar's default-branch context and `[radar]` policy (m2-details-and-diffs/13, 13b).
- `AppState::radar_column_seen` (field) — `Cell<bool>`, sticky once any row's marker has been `Some` this session; keeps the marker column showing after the row scrolls off-screen (m2-details-and-diffs/13b).
- `AppState::map` (field) — `update::map::MapState`: the Map view's visibility and its build, if any (m2-details-and-diffs/18).
- `AppState::last_focus_refresh` (field) — `Option<i64>`, Unix milliseconds: when the last focus-gained working-tree read ran. Terminals report focus in pairs and tmux reports it per pane switch, so reports inside 500 ms are ignored (m3-working-tree-undo-and-stash/06).
- `AppState::journal` (field) — `update::journal::JournalState`: the undo journal as this run has read it, and the browser's own cursor (m3-working-tree-undo-and-stash/06).
- `AppState::strip_shown` (field) — `bool`, off until `O` asked (no config default, M8 polish); the overview strip draws only while this holds and `map.shown` does not (m2-details-and-diffs/20).
- `AppState::uncommitted_row_shown` (fn) — does the working-tree row draw this frame: a dirty tree, and nothing else. Pinned at row 0 from m3-working-tree-undo-and-stash/04; it used to need `HEAD` on top too, which hid the user's own uncommitted work as soon as they scrolled (closes `KNOWN_GAPS.md` m2/06b). `view::entries::uncommitted_shown` and `view::strips` both delegate to it.
- `AppState::uncommitted_row_attached` (fn) — is `HEAD` (or an unborn branch) the topmost visible row this frame. It no longer decides whether the row draws, only how: attached it hangs off `HEAD`'s lane, detached it is a lone `◌` with no lane under it (m3-working-tree-undo-and-stash/04).
- `AppState::selected_spec` (fn) — the `DetailSpec` the cursor now names: `WorkingTree` on the flag, else the history cursor's commit.
- `AppState::renderer_override` (field) — the tier-drop key (`F3`) forced this tier for the rest of the run, or `None` for whatever `Session::settle` decided (m1-the-graph/19).
- `AppState::settled_tier` (field) — `Session::settle`'s tier, copied in at loop start; `None` without a session (m1-the-graph/24d).
- `AppState::tier` (fn) — tier in force: F3's override, else settled, else Unicode. F3 (`cycle_tier`) cycles Pixel→Unicode→Ascii→settled tier, never above it; back at the top the override clears. History pads to strips only at Pixel (m1-the-graph/33).
- `Screen` (enum) — `Graph` (default) or `Status`.
- `Screen::toggled` (fn) — the other screen; `F2` swaps.
- `Quit` (enum) — why Graft is leaving: `Requested`, `HandedBack(KeyCode)`, `Fatal(String)`.
- `Quit::should_quit` (fn) — is the loop finished?
- `AppState::text_field_open` (fn) — `(&self) -> bool`: is a text field taking the keyboard (the search prompt, the commit editor, the stash save prompt, the credential prompt)? Every global binding — `u`, `U`, `Ctrl+Z`, `Esc` on a failure — asks this first, so a letter typed into a field cannot fire a keybinding. **A new text field belongs in this list** (m3/06 review, Critical 1).
- `Status` (enum) — status-line state; never reads as an error when nothing is wrong. `Copied { short }`: `y` sent the SHA (OSC 52); footer notice until the next key; worded as what was sent (m1-the-graph/45). `Busy(description)`: a write is running — drawn as `<description>…` in the footer, set the moment the key is pressed (`update::operate::launch`), and not wiped by a `JobFinished`. `Done(description)`: it finished — drawn as `<description> ✓`, cleared by the next key like `Copied`. `Failed(message)`: a read failed on its own — the graph screen's `problem` line, not the footer; cleared by the next job that works, or by `Esc`. `Sticky { reason, keys: Option<String> }`: **a failure that must outlive the refresh it triggers** — a hook that rejected a commit, a patch git would not apply, an amend whose `HEAD` message could not be read, an `$EDITOR` that would not start. Drawn the same as `Failed`, but a `JobFinished` never clears it: the refresh the failure itself set off would otherwise wipe the reason a moment after it appeared (m3/04 review). Only `Esc` dismisses it. Named `WriteFailed` until the m3/05 review, which is what three of its uses are not. Reshaped from a single `Sticky(String)` in `hint-bar-redesign` (ADR-0037 §4): `keys`, when present, is the line's key hints (`"p pull first · P force with lease"`), kept apart from `reason` so `graph_screen::hints::fit_status_line` can cut the reason with `…` and keep the keys whole — a blind right-cut once lost `P force` at 80 columns. `Status::sticky(reason)` is the plain constructor (`keys: None`); `update::remote::remote_failed` still builds its combined `"reason · keys"` string and splits it once, downstream, since the convention is `ops`'s (`split_reason_and_keys`) — not worth a shape change in `graft-ops`. `view::status::problem` still joins the two back for the status screen/stash overlay (single-line contract); `view::status::graph_problem` (returns `graft_ui::graph_screen::ProblemLine`) is the graph footer's own reading, kept apart.
- `Size` (struct) — terminal size in character cells.
- `Size::columns` (field) — width in character cells.
- `Size::rows` (field) — height in character cells.
- `Command` (enum) — effects `update` asks the loop to carry out; `ReadHistory(JobId, Order)` starts the history walk under an id `update` chose (`queue_as`), so `awaiting` can name it before the pool reports it (m5-history-and-ghost-preview/05); `ShowRows(JobId, RowRequest)` asks a history job for rows; `ReadRefs` reads refs and ahead/behind off the loop's thread; `ReadWorkingTree` reads full working-tree status off the loop's thread; `PromoteHistory` sorts a provisional walk's range topologically; `Search(Query)` starts a search job. `ListFiles(JobId, DiffSpec, DiffOptions)`, `ReadFileDiff(JobId, DiffKey, FileEntry)`, `ContainedIn(JobId, Oid)`, `EvilProbe(JobId, Oid)` run the detail pane's reads under ids `update` chose (`CancelJob` stops them). `CopyToClipboard(String)` is terminal output the loop writes itself (through the session's tmux transport; a write error ends the loop), never dispatched to jobs (m1-the-graph/45). `SaveLayout(PaneSize)` is written to disk by the loop itself (`crate::layout::save_split`); a failure is expected and non-fatal (m2-details-and-diffs/10b). `BuildMap(JobId, MapRequest)` runs the Map view's build under an id `update` chose itself, like the detail pane's own jobs (m2-details-and-diffs/18). `Operate(JobId, Box<graft_ops::operation::Operation>)` runs one working-tree write under an id `update` chose — `jobs::operate::run_operation` opens the journal and calls `graft_ops::apply::apply`; the seam rule (`tests/it/seam_rule.rs`) keeps every other write detail out of `app` (m3-working-tree-undo-and-stash/04). Boxed like `Product::Operated` for the same enum-size lint. `Undo(JobId, bool)` and `Redo(JobId, bool)` run one undo or redo under an id `update` chose — the `bool` is "the drift guard was answered, go anyway"; `ReadJournal(JobId)` reads the journal, and `ReadStashes(JobId)` reads the stash list behind the `S` overlay (m3-working-tree-undo-and-stash/06). `OpenExternalEditor { path, line }`, `EditCommitMessage { id, message }` and `Suspend` are carried out by the frame loop itself, not the job pool: each hands the terminal over and takes it back, which only the thread owning the session and the input mux can do (m3-working-tree-undo-and-stash/05). `ReadRemotes(JobId)` reads the remote facts under an id `update` chose (m4-remotes/05). `AnswerCredential(JobId, update::credential::Answer)` hands one typed secret to the waiting job and `CancelCredential(JobId)` tells it nobody will answer; both are delivered by `JobRunner`, never by a worker, and the `Answer` is the only value in `Command` carrying a secret (m4-remotes/06).
- `CHROME_ROWS` (const) — screen rows that are not the graph without a harness banner; `graft_ui::graph_screen::CHROME_ROWS`. `AppState::graph_rows` adds `Harness::banner_rows`.
- `AppState::graph_rows` (fn) — history rows the graph pane shows at the current size; never zero.
- `update` (fn) — the pure state transition function.
- `opening` (fn) — `(PathBuf, Size) -> (AppState, Vec<Command>)`: the state Graft starts in and the jobs that fill it (open the repository, check git, walk history, read refs, read the working tree). The real loop and every integration harness go through it, because the history walk's id is minted here and `awaiting` set from it (m5-history-and-ghost-preview/05).

## update::selection
The uncommitted-changes row's one-bit selection: pure rules for setting, clearing and following `AppState::on_working_tree` (m2-details-and-diffs/06b).
- `move_from_graph` (fn) — a graph motion key, before `History::move_cursor` sees it: sets or clears the flag, returns the motion still left for `History` (or `None`).
- `reconcile` (fn) — called once at the end of every event with the history cursor's value from the start of it; clears the flag when the row stopped showing or the cursor moved under it; returns whether it cleared.
- `follow_selection` (fn) — a shown, non-`Compare` pane follows the selection onto or off the working-tree row; while `AppState::compare_base` is pinned it instead follows the cursor as a live compare against that base (fresh `head` each move, base kept), a no-op back on the base's own row (m2-details-and-diffs/11).

## update::context_menu
The `.`/right-click popup: opening it, its own keys, and carrying out whatever it dispatches (`context-actions-menu`, m2-details-and-diffs/12).
- `open_for_history_row`/`open_for_working_tree`/`open_selected` (fns) — open the menu for a given row, the working-tree row, or whatever the cursor now names. `marked` asks `view::radar::findings_for` for the row under the cursor — the same source of truth the marker column reads (`explain-reachable-from-the-menu`, m2-details-and-diffs/24) — not a hand-maintained flag.
- `context_menu_key` (fn) — the menu's own keys while open (`j`/`k` move, `Enter` dispatches and closes, anything else including `Esc` closes without acting); modal, so `key_event` returns as soon as this reports `true`.
- `apply_context_action` (fn) — `(state, action, row, oid: Option<Oid>, verbs: &[HistoryVerb], next_id) -> Vec<Command>`; `oid` is `None` only for the working-tree row, where `CopySha`/`CompareWithThis`/`Explain` are unreachable (`debug_assert!`, no command). `Fetch`/`Pull`/`Push`/`Undo` call the very functions `f`/`p`/`P`/`u` call (`remote::fetch_key`, `remote::pull_key`, `remote::push_key`, `undo::undo_key`), proved equivalent by `update::context_menu::tests::the_menu_runs_what_the_dropped_keys_run`. `History(i)` opens the Ghost Preview of `verbs[i].op` (`PreviewSubject::Operation { opened_by: "." }`) and runs nothing — `.` ends in a picture (PROMPT.md §1.7).
- `OpenMenu` (struct) — `{ row, menu, verbs }`: one answer to one `.`, kept in one field so the labels on screen and the operations behind them cannot drift apart.
- `HistoryVerb` (struct, re-exported from `verbs`) — `{ label: SanitizedText, op: graft_ops::operation::Operation }`.

## update::context_menu::verbs
The history verbs behind `.` on a commit row, and the operation each one means (m5-history-and-ghost-preview/08 piece 6). Pure: no `AppState`, no preview, no write.
- `history_verbs` (fn) — `(id, parents, subject, &RefBadges, &[Finding]) -> Vec<HistoryVerb>`, in menu order: `Checkout <b>` per local badge `HEAD` is not on (`Checkout <sha7> (detached)` on a row with no local badge), `Merge <b> into <cur> (m)` and `Rebase <cur> onto <b> (r)` — the only two verbs whose labels name a key, because they are the only two that have one (the very operations `m`/`r` build — same `preview::source_at`/`current_branch`, absent on a detached `HEAD` and on the current branch's own tip), `Cherry-pick <sha7>`, `Revert <sha7>` (`(-m 1)` on a merge), `Reset <cur> to here (mixed)`/`(hard)`, `Delete branch <b>` (never the current one), `Delete tag <t>`, then the three Fix… items a Foxtrot finding earns, all three stated against the finding's `real_tip`; `Fix: rebase instead` only while `HEAD` is on the merge. Empty on an unborn `HEAD`; the Fix items are absent when `real_tip` is not among the merge's parents. **This is the boundary the `CheckoutTarget`/`ResetMode` mapping lives at** — `graft-ui` sees only an index and a `SanitizedText` label (ARCHITECTURE.md, `DESIGN.md` §8.2).

## update::operate
Working-tree writes from the changes pane (m3-working-tree-undo-and-stash/04). Which region has focus decides the unit: in the file list `Space`/`s`/`d` mean the whole file (or every file of a section heading), in the diff region the hunk or line under the line cursor. **Section membership decides direction** — a row under STAGED unstages, anything else stages — because a path that is staged *and* unstaged has a row in both groups with identical status flags. `Ctrl+A` stages every path. Seam rule: builds an `Operation` and a `Command::Operate` from it, nothing else — `graft_ops` decides what the write does.
- `operation_in_flight` (fn) — `AppState::operating.is_some()`; every write key is a no-op while true.
- `stage_selected` (fn) — `(&mut AppState, &mut Vec<Command>, next_id) -> bool`; `Space`/`s` in the file list: stages or unstages the cursor's file — or every file under the section heading the cursor is on, stopping at the next heading. Direction comes from the row's `WorkingTreeSection`, never from the file's status flags; `false` (no-op) lets the caller fall back to `Space`'s other meaning (folding a directory row). A `WorkingTreeSection::Unmerged` row answers the key with `conflict_refusal`'s words instead of staging: staging a conflicted path is how git marks it resolved, a different act with its own confirmation (m5-history-and-ghost-preview/04 seam).
- `stage_at_diff_cursor` (fn) — `(&mut AppState, &mut Vec<Command>, next_id) -> bool`; in `DiffMode::SideBySide` it refuses in words (`press v for the unified view…`) rather than write a hunk under a cursor that view does not draw (m3/04 review M5). `Space`/`s` in the diff region: `Target::Patch` built by `graft_git::diff::hunk_patch` from `DetailState::diff_raw` — the whole hunk on a `@@` heading or a context line, that one line on a `+`/`-` line. `PatchDirection::Forward`/`PatchSide::Worktree` when staging, `Reverse`/`Index` when unstaging. An untracked file stages whole (ADR-0034). A `PatchError` answers the key and sets `Status::Failed` with the reason and the way out — never a key that silently does nothing. Same for a conflicted file: `patch_direction` returns `None` (a file with conflict markers has no clean side to cut a hunk against) and the key is answered with `conflict_refusal`'s words.
- `request_discard_at_diff_cursor` (fn) — `(&mut AppState) -> bool`; refuses in `DiffMode::SideBySide` like `stage_at_diff_cursor`. `d` in the diff region: the confirm box for discarding that hunk or line — buttons `Cancel`/`Discard`, so the question and the button name the same act (m3/04 review M6), the patch always built `Reverse`, the consequence counting the patch's own `+`/`-` lines. Untracked falls back to `request_discard`; a conflicted file is refused in words, like `stage_at_diff_cursor`.
- `stage_all` (fn) — `(&mut AppState, &mut Vec<Command>, next_id) -> bool`; stages every **unstaged or untracked** path `AppState::working_tree` lists. A staged deletion, a staged rename's vanished old name and an unmerged path are skipped: `git add` on the first two is `fatal: pathspec ... did not match any files` (exit 128, nothing staged at all), and on an unmerged path it marks the conflict resolved with the markers still in the file (m3/04 review M2).
- `conflicted_line` (private fn) — `(&str, graft_git::state::InProgress, &[PathBuf], Option<&str>) -> String`; the status line for an operation git stopped part-way through. Worded as a state, never a failure: what was being done, that the operation is still open, and what is in front of the user. Names no key — `resume` has none yet. One path is named; several point at CONFLICTS; none says so rather than claiming a conflict that is not there (a rebase can stop with nothing unmerged) (m5-history-and-ghost-preview/07).
- `request_discard` (fn) — `(&mut AppState) -> bool`; opens `AppState::confirm` with the exact `git` command a whole-file discard of the cursor's file class runs (`clean -f`, `restore --staged --worktree`, or `restore --worktree`), `Cancel` under the cursor. A `WorkingTreeSection::Unmerged` row is refused in words instead (`conflict_refusal`): a conflicted path has no "before" to discard to, and its `UU` entry would otherwise reach the "both go back to HEAD" box and throw away the resolution in progress (m5-04 review Major 3). The consequence is **per class**, with line counts from the pane's own file list: untracked says the file is deleted; a file both staged and unstaged says both changes go (`restore --source=HEAD` throws away the staged one too, whichever row the cursor was on); otherwise which side the change comes back from (m3/04 review M6).
- `operated` (fn) — `(&mut AppState, JobId, jobs::operate::Operated, next_id) -> Vec<Command>`; drops a stale answer; `Done`/`Incomplete` both refresh — `Incomplete` still refreshes because the write happened even though part of its after-state could not be read (`ApplyError::AfterState`'s contract, `as-built.md`). `Failed` (via `operation_failed`/`failure_line`) sets the status line **and** refreshes on the scope the job reports (`Operated::Failed::refresh`): `ApplyError::Git` means git ran part-way, so the file list and `diff_raw` on screen are stale (m3/04 review M1). `Conflicted` sets a **sticky** line worded as a state rather than an error (`conflicted_line`) and always refreshes `Everything` — the wide re-read is what puts the unmerged paths under CONFLICTS and fills `AppState::in_progress`; the commit editor and its draft are left alone, since no commit was made (m5-history-and-ghost-preview/07 seam). `Refresh::WorkingTree` emits `ReadWorkingTree` + the pane's reload; `Refresh::Everything` runs `update::keys::refresh_key` — `Ctrl+R`'s own path, walk cancellation, `OpenRepository` and radar reset included, so the two cannot drift (m3/04 review M8).

## update::commit_editor
The commit/amend editor (m3-working-tree-undo-and-stash/05). Modal: while
`AppState::commit_editor` is `Some`, no key reaches the screen behind it.
Seam rule: builds a `graft_ops::operation::Operation` and a
`Command::Operate`, never a `GitInvocation`.
- `CommitEditor` (struct) — `subject: TextField`, `body: TextArea`, `focus: Field`, `amend`, `sign_off`, `no_verify`, `allow_empty`, `head_short`, `template_applied`, `from_draft`.
- `CommitEditor::new` / `CommitEditor::amend` (fn) — a fresh editor, or one prefilled from `HEAD`'s short id and message.
- `CommitEditor::load_message` (fn) — replace both fields from one message (first line the subject, the rest the body); used by the template, `HEAD`'s message and `$EDITOR`'s answer alike, so the three cannot split the message differently.
- `CommitEditor::message` (fn) — the fields back as one message: subject, a blank line, body; no trailing blank.
- `CommitEditor::params` / `CommitEditor::operation` (fn) — the `graft_ops::operation::CommitParams` the ticks describe, and the `Operation` (`Commit` or `Amend`) built from it.
- `CommitEditor::refusal` (fn) — `(Option<&WorkingTreeStatus>, Option<InProgress>) -> Option<SanitizedText>`: the preview of why `[ Commit ]` is dimmed — a merge/rebase/cherry-pick in progress (naming the `git` command to finish or abort it, ADR-0034), nothing staged, or an empty subject. A preview only: `Operation::check` is what actually refuses.
- `CommitEditor::is_empty` / `CommitEditor::options` (fn) — whether anything was typed (so `Esc` can drop rather than stash a draft), and the four checkboxes with their ticks.
- `open` (fn) — `c`/`a` from the graph or the changes pane: takes back `AppState::commit_draft` when there is one, else a fresh editor plus the jobs for `commit.template` and, on an amend, `HEAD`'s message.
- `head_message_arrived` / `template_arrived` (fn) — fill an editor still waiting, dropping an answer for an editor that has since closed; the template never overwrites text the user typed first.
- `editor_returned` (fn) — `$EDITOR`'s answer, accepted only while `AppState::external_editor` still names that job.
- `paste` (fn) — `(&mut AppState, &str) -> bool`; a bracketed paste into whichever text field has focus (the subject drops newlines, the body keeps them). `true` whenever the editor is open, because it is modal. Before this the editor dropped every paste, though Graft turns bracketed paste on (m3/05 review M3).
- `editor_key` (fn) — the editor's whole key map: text into whichever field has focus, `Tab`/`Shift+Tab` and `←`/`→` for focus, `Space` on an option, `Enter` on `[ Commit ]`, `Ctrl+Enter` anywhere (ADR-0034: the button is the way that always works, `Ctrl+Enter` the shortcut where the terminal reports it), `Ctrl+E` for `$EDITOR`, `Ctrl+S` for a sign-off trailer, `Esc` to dismiss a failure if one is showing and otherwise put the draft aside — the two-stage `Esc` the graph screen has (m3/05 review M1).
- `committed` (fn) — a commit that landed: closes the editor and drops the draft.
- `strip_comments` (fn) — drop `#` lines from what an editor or a template gave back, the way `git commit` does.

## update::confirm
The discard confirm box: opened by `update::operate::request_discard`, answered here, modal like the context menu (m3-working-tree-undo-and-stash/04).
- `PendingConfirm` (struct) — `view: graft_ui::confirm::ConfirmView`, `then: ConfirmThen` (what `Enter` on `Confirm` dispatches).
- `ConfirmThen` (enum) — `Operate(graft_ops::operation::Operation)` (the discard box) or `Undo { redo: bool }` (the drift guard: the undo the box was opened for, run with `force`) (m3-working-tree-undo-and-stash/06).
- `confirm_key` (fn) — `(&mut AppState, &mut Vec<Command>, KeyPress, next_id) -> bool`; `Left`/`Right` move the cursor **clamped** (`←` on `Cancel` stays on `Cancel` — the acting choice is never one key-repeat away from where the cursor rests, ADR-0038 §1; m5-04 review Major 1), `Tab`/`BackTab` cycle, `Enter` acts on whichever choice is under it, everything else (`Esc` included) closes without acting; `true` whenever a box was open, so the caller sends it nothing else.

## update::credential
Git's credential question as an in-app prompt (m4-remotes/06, design note §7). Opened by `Progress::CredentialNeeded` from the running remote job, answered by `Enter`, cancelled by `Esc`; modal over everything, drawn last. Every string git sends is sanitised here, before it is kept.
- `PromptField` (enum, m5-history-and-ghost-preview/01, ADR-0038 §2) — `Open(graft_ui::text_input::TextField)` for a username, `Secret(graft_ui::text_input::SecretField)` for a password; `new(masked: bool, pulse_seed: bool)` picks the variant (`pulse_seed` reaches `SecretField::new`; the open field has no pulse and ignores it), and `insert`/`insert_str`/`backspace`/`delete`/`left`/`right`/`home`/`end`/`clear` dispatch to whichever field is live so `credential_key` never matches on the variant itself. `into_bytes(self) -> Zeroizing<Vec<u8>>` moves the typed answer out (`mem::take` on the `Secret` arm, no clone) for `Answer::new`.
- `CredentialPrompt` (struct) — `title`/`prompt` (`SanitizedText`), `field: PromptField`, `job: JobId`. `masked(&self) -> bool` reports `matches!(field, PromptField::Secret(_))` (replaces the old `masked: bool` field: the prompt's kind now lives in which `PromptField` variant it holds, not a separate flag that could drift from it). Its `Debug` prints the title, the mask and the job and **never the field** — an `AppState` dump cannot contain a typed password.
- `Answer` (struct) — `Arc<graft_git::run::askpass::Secret>`, the one secret-carrying value in `Command`: `new(Zeroizing<Vec<u8>>)` (m5-history-and-ghost-preview/01: takes the already-zeroizing buffer `PromptField::into_bytes` hands it, moving the bytes out via `mem::take` rather than cloning), `expose(&self) -> &[u8]`, `into_secret(self) -> Secret` (unwraps the `Arc` when it is the last one, else copies; both allocations zero on drop). `PartialEq` compares contents so `Command` stays comparable; `Debug` prints `Answer(…)`.
- `is_masked_prompt` (fn) — `(&str) -> bool`: does git's wording say this answer is a secret ("password"/"passphrase", case-insensitively)? A username is asked in the open.
- `host_of` (fn) — `(&str) -> Option<String>`: the host out of git's prompt, for the box's title; the user part is dropped.
- `credential_needed` (fn) — `(&mut AppState, JobId, &str, pulse_seed: bool)`: open the prompt, but only for the write actually running and not while it is cancelling. Every test pins `pulse_seed`; the running app draws it.
- `credential_key` (fn) — `(&mut AppState, &mut Vec<Command>, KeyPress) -> bool`: `Enter` → `AnswerCredential` + close + clear, `Esc` → `CancelCredential` + `CancelJob` (the write stops), `Ctrl+U` clears, `Ctrl+C` falls through to quit, **everything else is swallowed** — no global binding can fire from inside the prompt.
- `credential_paste` (fn) — `(&mut AppState, &str) -> bool`: a bracketed paste lands in the field when the prompt is open.

## update::undo
`u`/`U` (ADR-0002), pure: which job the key builds, what the status line says, and the drift guard's box. `graft_ops` decides what an undo does — this only names the job (m3-working-tree-undo-and-stash/06).
- `undo_key` (fn) — `(&mut AppState, redo: bool, next_id) -> Vec<Command>`; a no-op while another write is in flight, otherwise sets `operating` and `Status::Busy("undo · esc cancels")` and dispatches `Command::Undo`/`Redo` with `force` off. `Stoppable::Yes` since m5-history-and-ghost-preview/06 gave `graft_ops::restore::execute` a between-step stop check and a roll-back.
- `undone` (fn) — `(&mut AppState, JobId, jobs::undo::Undone, next_id) -> Vec<Command>`; drops an answer that is not the job in flight. `Done` → `Status::Done(<the entry's own description>)` and the refresh the entry asked for; `Incomplete` → `Status::Sticky` and still the refresh (it ran); `Nothing` → `Status::Done` (nothing to undo is an answer, not a failure); `Drift` → the guard's confirm box; `Cancelled { clean, .. }` → `Status::Done` when the roll-back finished (nothing is wrong, so it is an acknowledgement) and `Status::Sticky` when it did not, plus `Refresh::Everything` either way; `Failed` → `Status::Sticky`, and a refresh only where git may have run.
- `revert_to_forced` (fn) — the journal's time machine past the drift guard: unstoppable for the reason the first attempt was, since a jump is several restores and `Esc` between two of them would leave the repository somewhere nobody asked for (m5/08).
- `reverted_to` (fn) — `(state, JobId, jobs::undo::RevertedTo, next_id) -> Vec<Command>`. `Done` → `Status::Done` and a full refresh; `Drift` → the guard's confirm box, carrying the target entry so the retry aims at the same place; `Failed` → `Status::Sticky` naming how many of how many steps ran, and a refresh only when something did (m5/08).
- `interrupted_notice` (fn) — `(&AppState) -> Option<String>`: the footer line for entries a previous run left part-written. Derived from the journal's own entries (`JournalEntry::is_interrupted`, minus anything a later entry reverses), not from what a read happened to recover, so it survives every re-read until `J` (m3/06 review, Major 1).

## update::preview
Ghost Preview's state machine (PROMPT.md §1.7, ADR-0038, design note hard choices 1, 3, 5), pure. A preview is a **modal overlay over a bounded slice**: it never loads more history, and it never runs anything — only `Confirm` does.
- `PreviewSubject` (enum) — `Operation { op: Box<Operation>, opened_by: &'static str }` or `RevertTo(EntryId)`. `opened_by` is the key that opened it, so a `Moved` refusal can say which key to press again.
- `PreviewRequest` (struct) — `subject` + `resident: HashSet<Oid>`: what to plan, and every commit the app can draw without reading anything.
- `Planned` (struct) — `plan: PreviewPlan` + `anchors: Vec<Commit>`: what the job came back with.
- `PreviewState` (struct) — `subject`, `resident`, `slice_top`, `job`, `plan: Loading<Arc<Planned>>`, `rows: Vec<Row>`, `ghost_index`, `clipped`, `undrawable: Option<Oid>`, `cursor`, `return_to_journal`.
- `HISTORY_CANNOT_STOP` (const) — why `Esc` is not offered for a confirmed history operation.
- `MIN_BOX_ROWS` (const) — 4; mirrors `graft_ui::preview_box`'s crate-private constant.
- `BoxRect` (struct) — `x`/`y`/`width`/`height` + `bottom()`: where the box goes, in plain numbers. `update` names no drawing type (`update::tests::update_cannot_reach_the_world`), so the dock arithmetic is mirrored here and kept in step by `tests::a_click_lands_on_the_button_the_widget_drew`, which renders the real widget and reads the buttons back off the buffer.
- `open_preview` (fn) — `(state, commands, subject, return_to_journal, next_id) -> bool`. Refuses in words (`Status::Sticky`, never `Done`) while a write runs or git is part-way through an operation; silent over an already-open preview. Snapshots the resident rows **before** `state.preview` is set, so the box's own rows have not yet shrunk the window it is measured in.
- `preview_key` (fn) — modal: claims every key but `Ctrl+C`. `←`/`→` choose, `Enter` acts, `Esc` closes (and cancels a plan in flight). Below `PREVIEW_MIN` `Enter` is a no-op — nothing acts on a picture nobody can see.
- `preview_click` (fn) — a click on a button is that choice; every other click is swallowed, because the rows behind a modal picture are not selectable.
- `preview_produced` (fn) — builds the slice, lays it out once with `graft_graph::layout`, fills `rows`/`ghost_index`/`clipped`/`undrawable`. Stale job ids dropped.
- `confirm_preview` (fn) — runs the **same** `Operation` the picture was drawn from, moved across rather than rebuilt (hard choice 3); `RevertTo` becomes `Command::RevertTo`.
- `close_preview`, `preview_badges`, `preview_box`, `preview_box_rect`, `preview_rows_taken`, `fits` (fn).
- `HistoryKey` (enum) + `is_history_key`, `history_key` (fn) — `m` (merge the selected branch into the current one) and `r` (rebase the current branch onto the selection), `docs/design/02-keymap.md` §2.3. Both end in a preview, never in a write. Refusals are sentences: `m needs a commit row`, `r needs a branch checked out`, `already on it`.

## update::preview::slice
The bounded slice (design note hard choice 1), pure over what it is handed.
- `PREVIEW_ANCHOR_CAP` (const) — 64: more commits outside the view than that and the preview says so rather than reading them.
- `Slice` (struct) — `commits`, `ghost_at`, `clipped`.
- `SliceError::MissingParent(Oid)` (enum) — the one case that cannot be drawn: a ghost's real parent that is neither resident nor an anchor. The box then says `preview cannot be drawn here: <sha7> is not loaded · esc` and offers `[ Close ]` — never a truncated shape, never a panic.
- `build_slice` (fn) — `(resident, anchors, plan) -> Result<Slice, SliceError>`. A ghost goes immediately above the topmost slice row among its **transitive** real parents, and a chain comes out newest-first. Anchors go last with their own parents cleared — and counted into `clipped`, because history plainly continues below them. Parents outside the slice are clipped and counted too: `graft_graph::layout` requires every merge parent to be present, so clipping is mandatory, and the count is what the box's `<n> lanes continue outside this view` line reports.
- `BadgeOverlay` (struct) + `badges_for_preview` (fn) — the refs shown where they **would** land: each `RefMove`'s name removed from its `from` commit and added to its `to` (real or ghost); `to: None` is a delete and needs no special case; `HeadMove` moves the `HEAD` mark. Borrows the live badges rather than rebuilding them.
- `short_of` (fn) — `refs/heads/main` → `main`; a remote shorthand is left alone.

## update::remote
`f` fetch, `p` pull, `P` push, and `Esc` on any running write (m4-remotes/05, design note §5). Builds `Operation::{Fetch,Pull,Push}` from `AppState::remotes`; `graft_ops` decides what runs and words most failures.
- `RemoteState` (struct) — `last_rejected_push: Option<PushParams>` (the push the remote refused non-fast-forward; the next `P` offers the lease box; cleared on success or a branch change), `reading` (the facts read in flight), `running: Option<Running>` (the write in flight: id, description, the `Operation` when it is one, `stoppable`, and `opened_by` — the key that drew the Ghost Preview it was confirmed from, so a `Moved` refusal can say which key to press again), `cancelling` (the write `Esc` stopped), `diverged_upstream: Option<(String, Oid)>` (the tracking ref and tip a `p` just refused to fast-forward onto; `m`/`r` aim at it while it is `Some`, whatever row the cursor is on; cleared by dismissing the line and by any write launching — m5-history-and-ghost-preview/08).
- `request` / `request_at_launch` (fn) — ask `Command::ReadRemotes`; the launch read runs from `settle` once the repository has opened and the facts are `Idle`.
- `remotes_arrived` (fn) — lands the facts, dropping a stale id.
- `fetch_key` / `pull_key` / `push_key` (fn) — the keys; refusals are words on the status line (no remote, detached, no upstream), never a no-op.
- `pull_params` / `push_params` (fn) — the params from the facts; `push_params -> Result<PushParams, String>`.
- `push_box` (fn) — `(params, ahead: Option<usize>, local_tip: Option<Oid>) -> ConfirmView`: `Push main → origin/main?` with the ahead count, or the force-with-lease box naming the tip it expects.
- `busy_text` (fn) — remote operations add `ESC_HINT` to the busy line. `ESC_HINT` (const, ` · esc cancels`) — `view::frame` draws a busy line ending in it without its usual `…`.
- `Stoppable` (enum) — whether `Esc` may stop a running write: `Yes`, or `No(&'static str)` with the line that says why not. Set by whoever launches the write.
- `cancel_operating` / `stop_write` (fn) — `Esc` while any write runs: `CancelJob`, `Busy("<desc> · cancelling")`; a second `Esc` is a no-op. The commit editor's `Esc` goes through `stop_write` too. A `Stoppable::No` write is not asked to stop: the line says so and it keeps running.
- `cancelled` (fn) — `Event::JobCancelled` for the cancelled write: clears `operating`, `Done("<desc> cancelled")`, refreshes everything.
- `remote_progress` (fn) — git's progress onto the busy line, narrowed under 100 columns; ignored while cancelling.
- `remote_done` / `remote_failed` (fn) — the success words (`fetched`, `pulled origin/main`, `pushed main → origin/main`) and the failure line: a non-fast-forward push is remembered and gets the `p`/`P` hint; anything else is `Sticky` with `graft_ops`' own explanation.
- `pushed_since` (fn) — the ` · origin/main still has it` suffix `u` adds when the commit it undid was pushed since; `pushed_to` names that ref.

## update::journal
The undo journal as this run has read it (m3-working-tree-undo-and-stash/06). Re-read on launch, after every write, and by `Ctrl+R` — another Graft on the same worktree appends to it.
- `JournalState` (struct) — `shown`, `selected`, `expanded`, `notice_seen`, `entries: Loading<Arc<jobs::undo::JournalRead>>`, `reading: Option<JobId>` (the read in flight, held apart so a re-read does not blank the list it refreshes), `unreadable: Vec<String>` (accumulated across reads: only the read that recovered an entry can report it).
- `JournalState::read` (fn) — `(next_id) -> Vec<Command>`: marks the read pending under an id `update` chose and asks for it.
- `JournalState::newest_first` (fn) — the entries in the order the browser lists them.
- `journal_key` (fn) — `(&mut AppState, &mut Vec<Command>, KeyPress, next_id) -> bool`, the Map view's shape: `J` on the graph opens the browser and re-reads the journal; while open, `j`/`k` move, `Space` opens one row's Command Log, `Esc`/`J` close (cancelling a read still in flight), `Enter` acts, and every other key is swallowed rather than moving the graph underneath. `Enter` is `u` on the `← u` row and `U` on the `← U` row; on any **older** row it is the **time machine** (`docs/design/02-keymap.md` §2.11): `update::preview::open_preview(PreviewSubject::RevertTo(entry.id), return_to_journal: true)`, and the browser is *hidden* rather than closed so `Esc` on the preview puts it back on the same row. Nothing runs from this key — only the preview's `Confirm` does (m5-history-and-ghost-preview/08).

## update::stash
The stash overlay's state (`S`, DESIGN.md §1.11, m3-working-tree-undo-and-stash/06). Pure: every write it starts is an `Operation` the `ops` crate decides, per the seam rule.
- `StashPrompt` (struct) — the save prompt: `message: graft_ui::text_input::TextField`, `staged_only` (`Tab` flips it).
- `StashState` (struct) — `shown`, `selected`, `entries: Loading<Arc<Vec<graft_git::stash::StashEntry>>>`, `preview: Loading<Arc<FileListView>>` (the selected stash's files, under `Space`), `prompt`.
- `StashState::read` (fn) — `(next_id) -> Vec<Command>`: marks the list pending under an id `update` chose and asks for it.
- `StashState::rows` / `selected_entry` (fn) — the list as drawn, and the entry under the cursor.
- `stash_key` (fn) — `(&mut AppState, &mut Vec<Command>, KeyPress, next_id) -> bool`, the journal browser's shape: `S` on the graph opens the overlay and re-reads the list; while open `j`/`k` move (dropping the preview), `Space` asks for the selected stash's files through `Command::ListFiles` with `DiffSpec::Commit`, `s` opens the save prompt, `⏎` applies, `p` pops, `d` asks the confirm box first, `Esc`/`S` close (cancelling a read in flight), and every other key is swallowed rather than moving the graph underneath. `Tab`/`BackTab` are never claimed, so focus can always reach the detail pane.
- `save_refusal` (fn) — `(&AppState) -> Option<&'static str>`: why `s` would refuse right now (`nothing to stash`, or `nothing to stash — nothing is staged` for `staged only`), `None` while the working-tree status is unread. A preview only; `Operation::check` is the decision.

## update::radar
The Mistake Radar's own state: pure, no I/O (m2-details-and-diffs/13, 13b).
- `RadarState` (struct) — `context: Loading<Arc<jobs::radar::RadarContext>>`, `policy: graft_ops::mistake_radar::Policy` (`Policy::default()` until `[radar]` config is threaded in — see KNOWN_GAPS.md).
- `RadarState::force_push` (field) — `HashMap<String, Loading<ForcePushVerdict>>` by short branch name: one verdict per upstream-tracked branch (m2-details-and-diffs/15).
- `RadarState::duplicates` (field) — `Loading<Arc<Duplicates>>`; one repo-wide probe over the newest 500 commits, not per branch (m2-details-and-diffs/16).
- `RadarState::duplicate_findings` (field) — `HashMap<Oid, Finding>`; rebuilt whole each time `duplicates` lands, via `probes::findings_from_duplicates` — this is where `[radar] duplicate = off` takes effect, since the probe itself is policy-blind (m2-details-and-diffs/16).
- `RadarState::ref_findings` (field) — `HashMap<Oid, Vec<Finding>>`; `classify_refs`'s answer by commit, worked out once per refs/context/policy change by `refresh_ref_findings`, not per row per frame (M2 review C1).
- `RadarState::force_push_findings` (field) — `HashMap<Oid, Vec<Finding>>`; the same for force-push verdicts, keyed by branch tip.
- `RadarState::force_push_not_checkable` (field) — `HashMap<Oid, String>`; the `NotCheckable` reason by branch tip, for `RadarLine::ForcePushNotCheckable`.
- `RadarState::force_push_revision` (field) — `u64`; bumped by `force_push_probed`, part of the stamp that invalidates the three maps.
- `refresh_ref_findings` (fn) — `(state)`; called at the tail of every `update` (`update::settle`). Rebuilds the three maps above only when the stamp (`refs`/`context` by `Arc::ptr_eq`, `HEAD`, default branch, `force_push_revision`, policy) changed; otherwise returns after that comparison. This is what makes a frame's radar lookup O(1) per row (M2 review C1).
- `branch_facts`/`head_facts` (fns) — pure fact adapters from `AppState` for `classify_refs`; they live here, not in `view::radar`, because `refresh_ref_findings` is the only caller now (M2 review C1).
- `MAX_FORCE_PUSH_PROBES` (const) — `32`; the most force-push probes (and `git` spawns) one session asks for.
- `request_force_push_probes` (fn) — `(state, commands, next_id)`; called at the tail of every `update` beside `request_context`. Idempotent: once the context is `Ready` and the `force_pushed` kind is on, asks one `Command::ForcePushProbe` per upstream-tracked branch that could be seen — the branch `HEAD` is on and the branches on resident rows, by hash lookup, never by walking every ref — up to `MAX_FORCE_PUSH_PROBES` (m2-details-and-diffs/15; bounded by M2 review C1).
- `force_push_probed` (fn) — `(state, branch, verdict)`; lands one verdict under its branch, whatever its job id — these are not tied to the selection. Bumps `force_push_revision`.
- `request_duplicates_probe` (fn) — `(state, commands, next_id)`; called at the tail of every `update` beside `request_context`/`request_force_push_probes`. Idempotent: once the context is `Ready`, asks one `Command::DuplicatesProbe(id, 500)` while `duplicates` is `Idle`; not gated on the `duplicate` kind, unlike the force-push probe (m2-details-and-diffs/16).
- `duplicates_probed` (fn) — `(state, id, result)`; lands `jobs::radar::run_duplicates_probe`'s answer, dropping it if `id` is stale, and rebuilds `duplicate_findings` (m2-details-and-diffs/16).
- `radar_event` (fn) — `(state, event)`; routes `Product::RadarContext`, `Product::ForcePushProbe` and `Product::DuplicatesProbe` to `context_landed`/`force_push_probed`/`duplicates_probed`.
- `request_context` (fn) — `(state, commands, next_id)`; asks for the radar context once the default branch's tip is known on `state` and nothing is already `Pending`/`Ready`. Called at the end of every `update`, which covers both startup and refresh: neither knows the tip until the refs job answers.
- `context_landed` (fn) — `(state, id, result)`; lands `jobs::radar::run_radar_context`'s answer, dropping it if `id` is stale.
- `reset_for_refresh` (fn) — `Ctrl+R` throws the context, every force-push verdict and the duplicates probe away: a fetch is when a remote can have been rewritten (m2-details-and-diffs/15, 16).

## update::map
The Map view's own state and keys (`M`); built fresh on open, thrown away on close — cheap enough to redo every time. No drawing here — `view::map`, a later ticket, draws whatever `MapState::data` holds (m2-details-and-diffs/18).
- `MapState` (struct) — `shown: bool`, `data: Loading<Arc<crate::map::MapData>>`, `progress: Option<(u32, u32)>` (done, total rows), `selected: usize` (row under the cursor).
- `MapState::open` (fn) — `(&mut self, req: MapRequest, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>`: shows the view, resets selection/progress, returns one `Command::BuildMap` under an id `next_id` hands out.
- `MapState::close` (fn) — `(&mut self) -> Vec<Command>`: hides the view and discards the data either way; a still-`Pending` build gets a `Command::CancelJob`.
- `MapState::selected_tip` (fn) — `(&self) -> Option<Oid>`: the selected row's tip commit, once `data` is `Ready`.
- `buckets_for` (fn) — `(columns: u16) -> usize`: `max(10, columns − 24)`.
- `request` (fn) — `(state: &AppState) -> MapRequest`: `buckets` off the terminal's columns, `clock`/`policy` off `AppState`, `previous_default_tips` from the radar context if `Ready` else empty; `build_map` derives `default_branch`/`remotes` itself from the `RepoStore`, so `MapRequest` carries none of that.
- `map_key` (fn, `pub(super)`) — `(next, commands, key) -> bool`: `M` opens (only on the graph screen, detail focus on the graph); while shown, `M`/`Esc` close (cancelling a pending build), `↑`/`↓`/`j`/`k` move the selection, `Enter` seeks the graph to the selected row's tip and closes the view (`seek_selected`, via `History::seek`); `true` when the key was this view's to answer.
- `map_event` (fn, `pub(super)`) — routes `Product::Map`/`Progress::Map` to `MapState::produced`/`progressed`, dropping a stale id.
- `resized` (fn, `pub(super)`) — while shown, cancels the running build and opens a new one sized for the new width.

## update::strip
The overview strip's only key (`O`, m2-details-and-diffs/20); no other state.
- `strip_key` (fn, `pub(super)`) — `(next, key) -> bool`: `O` on the graph screen (plain, detail focus on the graph, map not shown) flips `AppState::strip_shown`; `true` once handled.

## update::detail
The detail pane's state and transitions; pure (no I/O, no clock). Drawing is `detail-pane-frame`'s. Split into files, public items unchanged (m2 review M8): `state.rs` (`DetailState` and its caches, `PaneSize`, `DetailSpec`, `Loading`, `DiffKey`, `Bounded`), `ops.rs` (`impl DetailState`), `control.rs` (`LocalJobs`, `detail_event`, `effective_layout`, `open_explain`/`close_explain`, the diff-length helpers), `keys.rs` (`detail_key` and its region-local key maps).
- `LOCAL_JOB_BIT` (const) — top bit of every id `update` issues itself, so it never meets a runner-issued id.
- `LocalJobs` (struct) — `next_id`: hands out `LOCAL_JOB_BIT` ids.
- `DetailSpec` (enum) — `Commit(Oid)`, `Compare { base, head }`, `WorkingTree`; `diff_spec` maps it to git's `DiffSpec`.
- `CompareBase` (struct) — `oid`, `row`: a pinned compare base (`x`, m2-details-and-diffs/11); `AppState::compare_base` holds it.
- `toggle_compare` (fn) — `x` on a row, or `Shift`/`Ctrl`+click on a second one: no base pins one (no command); the same `oid` again clears it (no command); a different `oid` opens `DetailSpec::Compare` and keeps the base pinned.
- `Loading<T>` (enum) — `Idle`, `Pending(JobId)`, `Ready(T)`, `Failed(String)`.
- `DiffKey` (struct) — spec, entry, options, max_lines: one file's diff; the diff cache's key.
- `Bounded<K, V>` (struct) — small map that evicts its oldest entry when full; `new`, `get`, `insert`, `clear`.
- `DetailState` (struct) — spec, shown, focus, files (`Loading` of view + git list), diff, `diff_cache` (8), `contained` (64), options, mode, word_tint, max_lines (5,000), body_top, `pane_size` (`PaneSize`, per-orientation, ADR-0028).
- `DetailState::evil` (field) — `Loading<graft_ops::mistake_radar::probes::EvilVerdict>`: the evil-merge verdict for `evil_at`'s commit (m2-details-and-diffs/14).
- `DetailState::evil_cache` (field) — `Bounded<Oid, EvilVerdict>` (16): verdicts already probed; never cleared by `refresh`, because a merge's tree cannot change.
- `DetailState::evil_at` (field) — `Option<Oid>`: which commit `evil` is about; what `view::radar` matches a row against.
- `DetailState::probe_evil` (fn) — `(&mut self, selected: Option<Oid>, &Policy, next_id) -> Vec<Command>`: idempotent, called once at the tail of every `update` (`selection::follow_selection`, which passes the selected merge whether or not the pane is open — M2 review M3, so the graph row is marked too). Nothing when nothing is selected or the evil-merge kind is off (a pending probe is cancelled); cache hit → `Ready`, miss → `Pending` + `Command::EvilProbe`.
- `DetailState::evil_probed` (fn) — stale-dropped by id; caches the verdict and shows it.
- `DetailState::explain_requested` (field) — `Option<Oid>`: `Explain` chosen from the context menu on a marked row (`context-actions-menu`, m2-details-and-diffs/12); read and cleared by `open_explain` (m2-details-and-diffs/17).
- `DetailState::explain` (field) — `Option<graft_ui::detail::ExplainView>`: the open Explain panel, set by `open_explain`, cleared by `close_explain`.
- `DetailState::explain_highlight` (field) — `Vec<Oid>`: the commits `Explanation::highlight` named; `view::entries` fades every other row while `explain` is `Some`.
- `DetailState::kept_cursor` (field) — `Option<KeptCursor>`: where the changes-pane cursor was when a refresh started; set by `refresh`, spent by `files_arrived`, `None` otherwise.
- `KeptCursor` (struct) — `{ path: Vec<u8>, section: Option<WorkingTreeSection>, position: usize, body: usize }`. Without it every write dropped the cursor on the first file — usually one under STAGED, where the next `Space` means the opposite write (m3/04 review M7).
- `PaneSize` (struct) — `right_columns`/`below_rows: Option<u16>`; `for_layout(&DetailLayout)` picks the orientation's value, `None` for `Tab`. Persisted via `crate::layout::load_split`/`save_split` (m2-details-and-diffs/10b).
- `effective_layout` (fn) — the pane's actual `DetailLayout` for a screen size: breakpoint kind first, then that kind's `PaneSize` override folded back in (two-step compute, `view/frame.rs` and `detail_key` both use it).
- `DetailState::resize_pane` (fn) — `<`/`>` (ADR-0028): steps the focused divider by 4 cells, clamped to the pane's and the graph's minimums; `false` (no change, no command) in `Tab`, hidden, or at a bound.
- `DetailState::set_pane_cells` (fn) — divider drag: sets the orientation's cells straight from the pointer, same clamps and no-op cases as `resize_pane`.
- `DetailState::set_options` (fn) — new `DiffOptions`, re-shows the cursor's diff (cache hit or `ReadFileDiff`). Backs `w`/`[`/`]`.
- `DetailState::load_all` (fn) — `max_lines = usize::MAX`, re-shows the cursor's diff. Backs `L`.
- `DetailState::scroll_body` (fn) — clamps `body_top` by a signed delta against the diff's line count. Backs `j`/`k`/PageUp/PageDown/`g`/`G` on the diff body.
- `DetailState::scroll_body_by` (fn) — same move as `scroll_body`, content length looked up from whatever diff is currently loaded. Backs the mouse wheel over the diff body (`detail-resize-and-persistence`).
- `DetailState::dragging` (field) — a divider drag is in progress: set by a mouse `Down` on the seam, cleared by `Up`. Never persisted (`detail-resize-and-persistence`).
- `DetailState::open` (fn) — shows the pane on a spec, cancels a pending list, asks for files (and contained-in once per oid).
- `DetailState::close` (fn) — hides the pane, focus back to the graph.
- `DetailState::selected_entry` (fn) — the files cursor's git entry, if any.
- `DetailState::follow` (fn) — selection moved while shown: re-open on the new spec, keeping focus.
- `DetailState::refresh` (fn) — `Ctrl+R` and every write's reload: clears both caches, notes the changes-pane cursor in `kept_cursor`, re-asks the spec.
- `DetailState::files_arrived` (fn) — stale-dropped by id; `Ok` → `Ready`, first diff asked (or the message shown). On the changes pane the cursor comes back where `kept_cursor` says: same path in the same section, else the same place down that section, else the fresh list's own cursor. The line cursor comes back only when the file did (m3/04 review M7).
- `DetailState::move_files_cursor` (fn) — `j`/`k` on the files list: cache hit → `Ready`, else cancel + `ReadFileDiff`.
- `DetailState::diff_arrived` (fn) — stale-dropped by id; `Ok` cached + `Ready`, `Err` → `Failed`.
- `DetailState::contained_arrived` (fn) — cached whatever the selection now is.
- `DetailState::cycle_focus` (fn) — `Tab`/`Shift+Tab`: Graph→Files→Diff→Graph while shown.
- `open_explain` (fn) — `(&mut AppState, next_id) -> Vec<Command>`: answers `DetailState::explain_requested`, if any this tick; opens the pane on that commit, resolves its finding via `view::radar::findings_for` (the same selected-row lookup `view::detail::commit_header` uses — a marked row is always the cursor row) and `graft_ops::mistake_radar::explain::explain`, and fills `DetailState::explain`/`explain_highlight`. A stale request (no finding) still opens the pane; `explain` is left alone (m2-details-and-diffs/17).
- `close_explain` (fn) — `(&mut AppState)`: clears `explain`/`explain_highlight`, leaving the pane open on the same commit. `Esc`'s first press while Explain is open (`update::detail::escape_priority`, ahead of ADR-0023's own `Escape if detail.shown` arm).

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
Internal split (m1-the-graph/72c, public paths unchanged): `types` (`Config`,
`Ui`, `Graph`, `Radar` and their value enums — `ThemeChoice`, `RendererChoice`,
`IconSet`, `RadarSetting`), `load` (`Loaded`, `ConfigWarning`, `ConfigError`,
`load`, `load_from`, `default_path` and the parsing internals), both
re-exported from `mod.rs`.
- `Config` (struct) — the config file's settings.
- `Config::ui` (field) — `[ui]`: how Graft looks.
- `Config::graph` (field) — `[graph]`: how history is drawn.
- `Config::diff` (field) — `[diff]`: the detail pane's diff options at startup (m2-details-and-diffs/10).
- `Config::radar` (field) — `[radar]`: Mistake Radar per-kind severity overrides (m2-details-and-diffs/13).
- `Config::keys` (field) — `[keys]`: `Keys { suspend_on_ctrl_z }` (ADR-0002), the one key whose meaning is a preference (m3-working-tree-undo-and-stash/05).
- `Radar` (struct) — `[radar]` section: one `RadarSetting` per `MistakeRadarKind` (`foxtrot`, `backward`, `accidental_pull`, `evil`, `duplicate`, `force_pushed`, `risky_state`).
- `Radar::policy` (fn) — the `graft_ops::mistake_radar::Policy` this section describes: `Policy::default()` with only the set keys overridden; loaded into `AppState.radar.policy` by `event_loop::startup::terminal_and_state` (m2-details-and-diffs/13c).
- `RadarSetting` (enum) — `Default`/`Off`/`Info`/`Warning` for one Mistake Radar kind.
- `Diff` (struct) — `[diff]` section: `context: u8` (default 3), `ignore_whitespace: bool` (default false), `word_tint: bool` (default true); loaded into `DetailState::options`/`word_tint` by `event_loop::startup::terminal_and_state`.
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
- `RepoStore::table` (fn) — the repository's one `Arc<CommitTable>`, opened once, usually kept across `refresh` (rebuilt only when the commit-graph's presence changed).
- `RepoStore::opens` (fn) — how many times the repository has been opened from scratch, or its id table rebuilt for a changed commit-graph.
- `RepoStore::common_dir` (fn) — the repository's common directory, shared by every worktree; where `crate::layout`'s split file lives (m2-details-and-diffs/10b).
- `RepoStore::workdir` (fn) — the working tree's root, `None` for a bare repository. Every porcelain path is relative to it, so anything handed to another program is joined onto it first (m3/05 review M2).
- `RepoStore::refresh` (fn) — mark cached refs and conclusions (e.g. default branch) stale; returns `Result<Refreshed, StoreError>`, rebinding the id table to a new `CommitTable` when the commit-graph's presence changed since it was built (m2-details-and-diffs/21, m1-the-graph/54).
- `Refreshed` (struct) — `refresh`'s report: `table_rebound: bool`, true when a new id table replaced the old one.
- `StoreError` (enum) — why the store could not answer (open, refs, branches, id table, commit-graph status).

## cli
- askpass mode — with `GRAFT_ASKPASS_SOCKET` set and exactly one argument (git's prompt), the binary answers git's credential question through `run::askpass::respond` and exits, never loading config and never starting the TUI: 0 answered, 1 the user cancelled (nothing on stdout, so git gives up), 2 anything else, with one `graft: …` line on stderr (m4-remotes/05, wired here in m4-remotes/06).
- `Invocation` (enum) — what the user asked for; `Askpass { socket, prompt }` is the mode above.
- `CliError` (enum) — what the arguments could not be made sense of.
- `USAGE` (const) — what `--help` prints.
- `ASKPASS_SOCKET_VAR` (const) — `GRAFT_ASKPASS_SOCKET`, the env var git's askpass child is given.
- `from_process` (fn) — parse the real process's arguments; askpass mode is decided first.
- `askpass` (fn) — `(Option<OsString>, &[OsString]) -> Option<Invocation>`; the mode's rule on its own, so it is testable without a process.
- `parse` (fn) — parse an arbitrary argument list.

## view
Internal split (m1-the-graph/72b, public paths unchanged): `frame` (tier/screen
pick, `view`/`view_with_graphics`/`tier_in_force`/`draws_pixels`), `rows`
(`graph_view` and its assembly), `entries` (building each `GraphEntry`,
`uncommitted_shown`), `status` (`status_view`), `detail` (`detail_view`,
m2-details-and-diffs/07), `map` (`map_view`, m2-details-and-diffs/19),
`journal` (`journal_view`, m3-working-tree-undo-and-stash/06), `stash`
(`stash_view`, m3-working-tree-undo-and-stash/06), `strip`
(`strip_marks`, m2-details-and-diffs/20) — plus the pre-existing `strips`,
all re-exported from `mod.rs`.
- `preview` (private mod) — drawing one open Ghost Preview (m5/08): `preview_entries(state, preview, now) -> Vec<GraphEntry>`. While a preview is open `view::rows::graph_view` draws the preview's slice instead of the live window and sets `HintContext::Preview`, so the picture the box describes and the rows on screen are the same picture. A ghost row is `RowPaint::Ghost` with `ghost_lanes` = the lanes the ghost node itself touches (its own, those closing into it, every edge leaving it) — a real lane merely crossing the row keeps its solid glyph — and draws `—` for both sha and age, because the commit does not exist yet. `compare_marked`/`marker` are never set on a preview row: the compare pin and the Mistake Radar are about history as it is. `view::frame::draw_preview` docks `PreviewBoxWidget` at the bottom of the graph area, over rows `AppState::graph_rows` already gave up.
- `strip_marks(state) -> graft_ui::overview_strip::StripMarks` — the overview strip's marks over the rows `state.history` holds now: `Merge` on a row with 2+ parents, `Radar(sev)` from `view::radar::marker_for` (already gated on the radar context being `Ready`), `Head` on `RefBadges::head()`'s row, `SearchHit` for each search hit landing on a resident row (hits are collected into a `HashSet` once per frame, not scanned per row — M2 review M10). `rows_total` is `History::rows`, `viewport` is `top..top+graph_rows`. Empty when there is no history yet. Wired into `frame` (m2-details-and-diffs/20): when `state.strip_shown` and not the pixel tier, `view::frame::render_graph_screen` renders `GraphScreen` into a scratch buffer and blits everything but its rightmost column into the frame, then draws `OverviewStrip` into that freed column — columns stay put because `graph_screen::layout::Columns::compute` lays sha/when/author out from `area.right()`, so narrowing the real `Rect` would shift them; the pixel tier draws no strip. `update::strip_bounds(state)` gives that column's `Bounds`; a left click there calls `graft_ui::overview_strip::row_for_cell` and `select_row` instead of the ordinary graph click, and the wheel is untouched (falls through to `scroll()` as before).
- `journal_view(state, now_seconds) -> graft_ui::journal_screen::JournalView` — the undo journal browser's rows, newest first (m3-working-tree-undo-and-stash/06). The `← u` / `← U` / `undone` marks come from `graft_ops::journal::{next_undo, next_redo, in_force_all}`, never from a rule restated here, so the browser and the key cannot disagree. A row's summary names what the entry moved (`1 ref · the index · backup saved`), leads with `failed: <git's first line>` for an `Aborted` entry and `interrupted` for one a previous run left `Pending`; the whole reason and the Command Log are under `Space`.
- `stash_view(state, now_seconds) -> graft_ui::stash_screen::StashView` — the stash overlay's rows, newest first (m3-working-tree-undo-and-stash/06): `stash@{n}`, the message's first line in quotes, the age from the graph's own `relative_date`, and the file count. The save prompt's `refusal` is `update::stash::save_refusal`'s answer and the notice line is `AppState::status`'s, so the overlay and the keys cannot disagree.
- `credential_view(state) -> Option<graft_ui::credential_prompt::CredentialView<'static>>` — the credential prompt as the widget wants it (m4-remotes/06): git's own question, the field masked or in the open, and a footer naming what `Esc` cancels. `counter: None` — how long a password is, is part of what it hides. Drawn after everything else, the confirm box included, at every site (`view::frame::draw_credential`).
- `radar` (`pub(crate)` mod) — turns `AppState`'s facts into `graft-ops::mistake_radar::Finding`s and those into a marker/detail lines (m2-details-and-diffs/13b). `repo_facts(state, ctx, remotes: &[&str])` (fn) — pure fact adapter from `AppState`/`RadarContext`; `remotes` comes from `remote_names(state)`, the remote facts' names (m4-remotes/05) (`branch_facts`/`head_facts` moved to `update::radar`). `findings_for(state, id, parents, subject) -> Vec<Finding>` — empty until `state.radar.context` is `Ready`; the ref-derived part is a lookup in `RadarState::ref_findings`/`force_push_findings`, never a ref walk (M2 review C1). `marker_for(...) -> Option<graft_ui::graph_screen::MarkerView>` — highest-severity finding. `radar_lines_for(...) -> Vec<graft_ui::detail::RadarLine>` — one line per finding plus, on a merge with the kind on, the evil-merge line that `DetailState::evil` says (`EvilChecking`/`EvilClean`/`EvilResolved`/`EvilNotCheckable`, nothing extra when the verdict is already a finding, `EvilUnchecked` when unprobed); called by `view::detail::commit_header` (m2-details-and-diffs/13c). A branch tip whose force-push verdict is `Rewritten` also yields a `ForcePushedRemote` finding, and one whose verdict is `NotCheckable` yields `RadarLine::ForcePushNotCheckable` (m2-details-and-diffs/15). A commit in `state.radar.duplicate_findings` also yields its `DuplicateCommit` finding (m2-details-and-diffs/16).
- `view` (fn) — draw the current state to a frame: the graph screen, or the status screen.
- `view_with_graphics` (fn) — `view`, plus the gfx status in the header at the pixel tier; returns the graph rect the images go in, `None` when no pixels this frame. When `state.detail.shown`, splits the frame with `graft_ui::detail_pane::DetailLayout` and draws `DetailPane` beside/below/instead of the graph screen (`Tab` draws one side full width behind a `[Graph] Detail` strip; the graph's pixel rect is `None` while it is off screen). When `state.map.shown`, the `Screen::Graph` arm renders `graft_ui::map_screen::MapScreen` instead — no graph rows, no detail split, no pixels (m2-details-and-diffs/19); with `map.shown` false the arm is unchanged from before that ticket.
- `map_view(state, now_seconds)` (fn) — build a `graft_ui::map_screen::MapView` from `state.map`'s `Loading<Arc<MapData>>`: `Ready` fills the rows (current branch first for the header title, else the default branch), `Pending`/`Idle`/`Failed` return an empty view with the matching `MapStatus`. `active` marks a row whose tip lands in the newest bucket, is not current and has not merged into the default branch's row. `ascii` is left `false`; `view::frame` sets it from the tier in force, mirroring `graph_view`.
- `detail_view` (fn) — assemble a `graft_ui::detail::DetailView` from `AppState`: header from the selected row's hydrated commit (author/committer/parents in full; co-authors parsed from `Co-authored-by:` message trailers), or from `AppState::working_tree`'s counts for the uncommitted row; body is the commit message when the files cursor is on `✉ message`, else the diff under it, tracking `DetailState::files`/`diff`'s loading state. `explain` comes straight from `AppState::detail::explain`.
- `explain` (`pub(crate)` mod) — the Explain panel's view model (m2-details-and-diffs/17). `build_explain_view(e: &Explanation) -> ExplainView` — title/paragraphs sanitised, sketch from `build_sketch`; `severity` is a placeholder (`Info`) the caller (`update::detail::open_explain`) overwrites from the `Finding`, since `Explanation` itself carries none. `build_sketch(evidence, short) -> Vec<SanitizedText>` — one sanitised line per evidence entry, alternating `──╮`/`──╯` connectors between them.
- `tier_in_force` (fn) — the graph's tier from `Session` + `AppState`: F3's override, else the session's (m1-the-graph/33).
- `draws_pixels` (fn) — graph screen, pixel tier (override else session), Kitty or iTerm2 protocol — or no acked protocol at all (forced `pixel` attempts Kitty, m1-the-graph/24d).
- `graph_view(state, now_seconds)` (fn) — build a `GraphView` from the history window, cursor and status; the selection's `Ancestry` over the window lights lanes, fades rows in Focus mode, and weighs its columns as a fold priority — not a veto (ADR-0025). `now_seconds` (m1-the-graph/53) is the instant every row's date is fitted against; callers pass `state.harness.clock.now_seconds()`. Reads `super::status::graph_problem` (not `problem`) for `GraphView::problem`, so the footer keeps a sticky failure's keys apart from its reason.
- `hint_context(state, rows)` (fn, `view::rows`) — the footer's `HintContext` for this frame (`hint-bar-redesign`): `on_changes_pane` → `ChangesPane { unmerged: the cursor's own file is unmerged }` **before** `on_working_tree` → `WorkingTreeRow` (the pane opens on the uncommitted row and the cursor stays there, so both hold and the pane's arm was unreachable — m5-03 review Critical 1); else the cursor row's (`RowPaint::Highlighted`) own `BadgeKind::Branch` → `BranchRow { current }`, no branch badge → `PlainRow`, wrapped in `Marked` when that row carries a `GraphEntry::marker`. `Banner`/`Preview` are other M5 groups' to produce; `Journal`/`Stash` are never built here. The divider hint is no longer part of this: `GraphView::detail_shown` carries it. Pinned from a real repository through a drawn frame by `tests/it/hint_bar.rs`, one test per reachable context.
- `status_view` (fn) — build a `StatusView` from session and app state.
- `graph_problem(status) -> Option<graft_ui::graph_screen::ProblemLine>` (fn, `view::status`) — the graph footer's own reading of `Status`: reason and keys kept apart (unlike `problem`, which joins them for the status screen/stash overlay) so `graph_screen::hints::fit_status_line` can cut the reason and keep the keys whole (`hint-bar-redesign`, ADR-0037 §4). A failure that carries no keys of its own gets `esc dismiss` (`view::status::DISMISS`) — `Esc` really does clear a `Failed`/`Sticky` status, and a keyless footer is the one outcome ADR-0037 §4 rules out (m5-03 review Major 2; pinned by `tests/it/hint_bar.rs::a_failure_still_names_the_key_that_clears_it`).
- `bound_actions() -> &'static [(&'static str, &'static str)]` (fn, `update::keys`, was `bound_keys()`) — every (key, verb) the graph screen and its panes actually bind, spelled as the footer spells them (`⏎ open`, `j/k move`, `⌃A all`, …), each annotated in place with the dispatcher that answers it. Walked both ways by `update::tests`: `every_hinted_key_is_bound` (every context × `Divider` × 40..200 columns, key *and* verb, no exemptions) and `every_planned_hint_is_still_unbound` (a `HintState::Planned` hint whose key gets bound fails the build until the hint is flipped to `Live`). Hand-written, so it is a promise the tests hold you to, not a derivation.
- `commit_editor_view` (fn) — build a `graft_ui::commit_editor::CommitEditorView` from `AppState`, including `status` (the same `Status` reading the graph footer's `problem` uses, so a hook's refusal is readable without closing the editor — m3/05 review M1): title (`COMMIT ── ⎇ <branch> ── N files staged`, or `AMEND <short> ── …`), the field halves, the option ticks, and `refusal` — the preview of why `[ Commit ]` is dimmed (nothing staged, empty subject, or a merge/rebase/cherry-pick in progress naming the shell command). The refusal is a preview only; `Operation::check` is the decision (m3-working-tree-undo-and-stash/05).

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

## event_loop::outside
Handing the terminal to another program and taking it back
(m3-working-tree-undo-and-stash/05). PROMPT.md principle 8: a wrong restore
breaks the user's terminal after Graft is gone, so every path here restores
newest-first and re-enters the same modes, or ends the loop saying it could
not.
- `Outside` (struct) — what the loop needs to reach outside itself: `common_dir` and an optional `Handover`. `None` in the headless and test loops, which have no terminal to give away.
- `Handover` (struct) — `session`, `mux`, `git_dir`, `workdir` (the working tree's root: the directory `$EDITOR` is started in, and what a porcelain path is joined onto — m3/05 review M2); `edit_message` (write the message to `graft/COMMIT_EDITMSG`, run `$EDITOR`, read back what was saved), `edit_file` (open a path, on a line where the editor takes one), `suspend` (`SIGTSTP` and back).
- `run_outside_terminal` (fn) — pauses the input mux **first** so the reader thread stops consuming stdin (`InputMux::hold` only detaches the listener — the thread keeps reading, and `$EDITOR` would race Graft for keystrokes), then suspends the session, runs the closure, and re-enters both. A failure to re-enter is `OutsideError::Reenter`, which ends the loop rather than drawing into a terminal Graft no longer owns.
- `OutsideError` (enum) — `Restore` (could not give the terminal away; nothing ran) and `Reenter` (could not take it back).
- `editor_command` (fn) — `(git_editor, path, line, workdir) -> Command`. `path` is made absolute against `workdir`, which also becomes the command's directory (m3/05 review M2). A `GIT_EDITOR` carrying shell syntax (`'"$`\;&|<>()*?[`) is run as `sh -c '<editor> "$@"' <editor> <args>`, the way git runs it — a quoted path with a space in it used to be split mid-word (m3/05 review M5). The line argument goes per editor family: `+N` (vim/nvim/vi/nano/emacs/emacsclient/micro/hx/helix), `--goto path:N` (code/code-insiders/codium/cursor/windsurf), `path:N` (zed/zeditor), and no line argument for anything unrecognised — a wrong flag would open a file named after it.
- `stop_this_process` (fn) — `SIGTSTP` to Graft's own process group, so the shell's job control takes over exactly as it would for any other program.

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
- `map` (private mod) — the Map view's job (m2-details-and-diffs/18): `build_map(repos, req, progress, stop) -> Result<Arc<MapData>, String>` runs `crate::map::build_map` and stringifies its error; `JobRunner::build_map` dispatches `Command::BuildMap` under `update`'s own id (`queue_as`, mirroring the detail pane's jobs), reporting `Progress::Map { done, total }` per finished row.
- `radar` (`pub(crate)` mod) — the Mistake Radar's context job (m2-details-and-diffs/13, 13b).
  - `RadarContext` (struct) — the default branch's first-parent ancestry as a `HandleSet` over its own `Arc<CommitTable>` (private; ADR-0026's rule, housekeeping/02: a `HashSet<Oid>` here was 43 of the heavy tier's 76 B/commit heap slope), plus `previous_default_tips: Vec<Oid>` (its reflog history, newest first, capped). `on_default_chain(&Oid) -> bool` / `chain_len() -> usize` / `of_ids(chain, previous_tips) -> RadarContext` (a context over ids, with a table of its own) (fn).
  - `run_evil_probe` (fn) — `(repo, merge, stop) -> EvilVerdict`; the work behind `Command::EvilProbe`, dispatched by `JobRunner::dispatch_detail`.
  - `run_force_push_probe` (fn) — `(repo, branch, current, stop) -> ForcePushVerdict`; the work behind `Command::ForcePushProbe`. Resolves the branch's remote-tracking ref through `RepoHandle::upstream` (config `update` cannot read), then `probe_force_push`; no upstream → `NotCheckable` (m2-details-and-diffs/15).
  - `run_radar_context` (fn) — `(repo, table, default_tip, default_ref, stop) -> Result<Arc<RadarContext>, String>`; the work behind `Command::RadarContext`, dispatched by `JobRunner::dispatch_detail`.
  - `run_duplicates_probe` (fn) — `(repo, window, stop) -> Result<Arc<Duplicates>, String>`; the work behind `Command::DuplicatesProbe`, wrapping `graft_ops::mistake_radar::probes::probe_duplicates` (m2-details-and-diffs/16).
- `preview` (private mod) — Ghost Preview's reading half (m5/08): `run_preview(repos, &PreviewRequest, cancel) -> Result<Arc<Planned>, String>` opens `git`, reads the facts the operation names (`Operation::facts_scope`) and calls `graft_ops::preview::plan`, or plans the time machine from the journal's own entries (`plan_revert_to`); then reads every real commit the picture points at that is not already on screen, as **anchors**, capped at `PREVIEW_ANCHOR_CAP` — past the cap the answer is a sentence the box prints, not a shorter picture. `JobRunner::plan_preview` dispatches `Command::PlanPreview` under `update`'s own id; `JobRunner::run_revert_to` dispatches `Command::RevertTo` as a write, with `Progress::Operating`.
- `undo::journal_entries` (fn) — the journal's entries for a caller that only reads them; the time machine plans from the very entries `revert_to` will run against (m5/08).
- `undo::RevertedTo` (enum) — `Done { description, total, steps }`, `Drift { description, drift, target }`, `Failed { description, done, total, error }`. `total` is read from the plan **before** anything runs, because `UndoError::PartWay` carries only `done` and "2 of 3" is the sentence the user needs (m5/08).
- `undo::run_revert_to` (fn) — `(repos, &EntryId, force, cancel) -> RevertedTo` (m5/08).
- `undo::Undone::Cancelled` (variant) — `{ clean: bool, why: String, refresh: Refresh }`: `Esc` stopped the undo and `graft_ops::restore::execute` rolled back what it had applied. Its own variant rather than a `Failed`, because a cancel is not a failure; `clean` says whether the roll-back finished, and `why` is `restore`'s own sentence. `refresh` is never optional — the journal gained an `Aborted` entry either way (m5-history-and-ghost-preview/06 scope item 5, built in /08).
- `detail` (crate-visible mod) — the detail pane's reads: `assemble_files(&FileList, Option<&[u8]>) -> FileListView` (path tree, `✉` row when the body has a non-trailer line) and `assemble_diff(&FileDiff, word_tint) -> DiffView` (sanitise, pairs, word spans, syntax, CRLF → `␍`), both pure; `read_files`/`read_file_diff`/`read_contained` run them on a worker with the job's `Cancel` as the git `Stop`. The runner queues them under `update`'s ids (`queue_as`).
- `operate` (private mod) — the changes pane's write (m3-working-tree-undo-and-stash/04): `run_operation(repos: &RepoStore, op: &graft_ops::operation::Operation, cancel: &Cancel, hooks: Hooks) -> Operated` (`Hooks` = the progress sink, the job's `AskPass` and the helper path or the reason there is none, m4-remotes/06) opens the system git and the undo journal, builds a `graft_ops::apply::Context` and calls `graft_ops::apply::apply` — one of the two places `app` performs a write (`jobs::undo` is the other, for `u`/`U`); the seam rule (`tests/it/seam_rule.rs`) forbids any other module here from building a write `GitInvocation` or touching the journal. Remote operations get `RemoteHooks` (progress into `Progress::RemoteProgress`, this binary as the askpass helper, `jobs::askpass::ChannelAskPass` answering git's questions from the user's prompt; no helper → the reason on the progress line and git asks nothing) (m4-remotes/05, /06).
  - `Operated` (enum) — `Done(Applied)` (clean); `Incomplete { applied: Applied, error: String }` (the write happened but part of its after-state — `ApplyError::AfterState`'s `unread` — could not be read back; still a completed write, `update::operate::operated` still refreshes); `Conflicted { description: String, kind: graft_git::state::InProgress, paths: Vec<PathBuf>, stalled: Option<String> }` (git stopped part-way through a history operation and left its own state live — **not a failure**: the entry is journaled `Pending`, and the user is in the operation until it is finished or aborted. Carries both `ApplyError::Conflicted` and `ApplyError::ResumeFailed`; `stalled` is git's own line for the second, where the state is live but nothing is unmerged. m5-history-and-ghost-preview/07 seam); `Failed { error: String, refresh: Option<Refresh>, remote: Option<RemoteFailure>, refused: Option<graft_ops::operation::Refusal> }` (`remote` is `ApplyError::Remote`'s classified failure, m4-remotes/05; `refused` is `ApplyError::Refused`'s own value, carried rather than stringified so `update::operate` can put a **key** beside the two history refusals — `Moved` → `<key> again`, `DirtyTree` → `S stash N changed files`, m5-history-and-ghost-preview/08) — `refresh` is `Some` whenever git may have run part-way (`ApplyError::Git` journals the entry `Aborted` after some of the plan applied), `None` only where `ApplyError` itself says nothing was written (`Refused`, `Facts`, `Snapshot`).
- `askpass` (private mod) — the job side of git's credential question (m4-remotes/06): `Waiting` is the runner's registry of jobs blocked on an answer (`answer(JobId, Option<Secret>) -> bool`; `false` means nobody was waiting and the secret was dropped and zeroed). `ChannelAskPass` implements `graft_git::run::askpass::AskPass`: it registers its channel **before** announcing `Progress::CredentialNeeded`, then waits in 100 ms steps so the job's `Cancel` unblocks it; an already-cancelled job never asks. `pulse_seed() -> bool` draws the masked field's starting pulse bit (`getrandom`, falling back to `clock_parity()`, the parity of the microseconds since the epoch — a failed draw only changes which bit is used, so it is not an error to report. The old nanosecond bit was the constant `0` on macOS's microsecond-granular clock, i.e. a constant seed: m5-04 review Major 2), fresh per prompt: a fixed start would leave the drawn row resting at the parity of the password's length (m5-history-and-ghost-preview/04; m5-01 review Major 2).
- `remote` (`pub(crate)` mod) — `RemoteFacts { remotes, head_branch, head_upstream }` and `read_remotes(repos, cancel) -> Result<Arc<RemoteFacts>, String>`: the remote facts behind `f`/`p`/`P` (m4-remotes/05).
- `stash` (private mod) — the stash list read behind `S` (m3-working-tree-undo-and-stash/06): `read_stashes(repos, cancel) -> Result<Arc<Vec<StashEntry>>, String>` runs `graft_git::stash::list` and stringifies its error; `JobRunner::read_stashes` dispatches `Command::ReadStashes` under the id `update` chose (`queue_as`, like the detail pane's jobs).
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
- `JobRunner::cancel` (fn) — request cancellation of a submitted job: sets the token, then drops the job's row-request sender. The history walk parks in `recv` between requests and cannot see its token from there, so closing that channel is what wakes it and gives the worker back; token first, so a walk woken mid-chunk finds `stop()` already true (m5-history-and-ghost-preview/05, moved here from `Command::CancelJob`'s dispatch arm).
- `JobRunner::shutdown` (fn) — stop the pool, waiting up to `grace` before abandoning.
- `JobRunner::with_askpass_helper` (fn) — name the binary git runs as its askpass helper instead of `current_exe`; under `cargo test` `current_exe` is the test binary, which has no askpass mode (m4-remotes/06).
- `JobRunner::answer_credential` (fn) — `(&self, JobId, Option<Secret>) -> bool`: hand a waiting job its answer (`None` = cancelled); `false` means the job had already gone and the secret was dropped. What `Command::AnswerCredential`/`CancelCredential` dispatch to.

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
- `preview_seeds` (fn) — `(Option<&WalkFacts>, &RefBadges) -> Seeds`: the same seeds from what the **app** holds rather than from `&[RefEntry]`/`&Branches`, which it never has. A preview's ghosts land in the live graph's lanes and colours only if the seeds match, so this reads `default_branch`/`integration` off the walk's own facts and the tips off the badges (m5/08).

## working_tree
- `read` (fn) — read working-tree status counts off the repository store (a job's work).
