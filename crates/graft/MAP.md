# graft — map
The application (ARCHITECTURE.md `app` layer), the only crate that may see every other one: event loop, `AppState`, the pure `update`, the job runner, config, and the view models `ui` draws.
A map of modules: one `##` heading per module to depth 2, path from the crate root; a purpose line; the items other top-level modules enter through. `main.rs` is the thin binary over `lib.rs`.
Tests: `tests/it` (one binary, with the pty harness); `examples/` holds the terminal probes, the gallery, the budgets and the milestone walks.

## lib.rs
- `cli` (mod) — CLI argument parsing.
- `empty_state` (mod, `pub(crate)`) — the empty state's keys and view (m8-polish-and-ship/08): `OutsideState`, `Prompt` (`Open(TextField)`, `ConfirmInit`), `OutsideStep` (`Stay`, `Open(path)`, `Init(folder)`, `Quit`, m8/19b `Practise` — `t`, and the palette's `Practice tour`), `outside_edge(state, &Keymap, key)` (m8-09b: the outside screen's edge — the open prompt takes keys as typed, else `translate(EmptyState, …)` — `translate(Confirm, …)` while the init box is open, m8/09d D6; an unbound default only clears the problem line; swept by `empty_state::sweep_tests`), `outside_key(state, key)` (the `i` arm is gated on `!missing` — the one switch for offering init in a missing folder), `view(state, now) -> EmptyStateView`; m8/11: `OutsideState::{palette, config_problems}` — `?`/`⌃P`/`:` open the palette on the list (not over the init box; typed in the open prompt), rows from `update::palette::listing` in `EmptyState`, `⏎` runs a row's default chord through the edge's own step, a theme row is `OutsideStep::Retheme(kind)` (the loop calls `Session::choose`); `palette_rows`, `outside_paste` (palette query or open prompt), `render(state, keymap, theme, ascii, now, area, buf)` (the list, the palette over it, its keys on the bar row); `o`'s path resolution (`~`, relative to the folder) and `⇥` directory completion. m8/18b (clone host; keys and drawing are 18c): `Prompt::Clone(CloneState { url, run: CloneRun })`, `CloneRun` (`Typing`, `Running { job, into, progress: Option<String> }`, `Ended { words }`), `OutsideState::credential` (git's question, modal over every key), `OutsideStep::{StartClone, StopClone, Credential(JobId, Option<Answer>), Cloned { path, notice }}`, m8/18e `OutsideStep::FirstLaunchDone(dir)`, `clone_request(folder, url) -> Result<CloneRequest, String>`, `clone_started(state, job, into)`, `outside_job(state, &Wake) -> OutsideStep` (the clone job's events; others ignored). m8/18c: `c` (gated `!missing`) opens `Prompt::Clone` typing; the clone prompt is a text field at the edge (as `o`'s); `⏎` → `clone_request` → `StartClone`, or the reason as `CloneRun::Ended` (URL kept; an edit returns to `Typing`); running, `esc` → `StopClone` and every other key is swallowed (`⌃C` quits); `esc` not running closes it; paste lands in the field (trimmed) unless running; `view` maps it to `graft_ui::empty_state::CloneView` (`into` = the running target, or `<folder>/<name>`, `<folder>/…` while none) and sets `EmptyStateView::credential`; `render` draws the credential box last, its band cleared full width, footer `… esc cancel the clone`. `view::credential::prompt_view(prompt, footer)` (`pub(crate)`) — the box's view from a `CredentialPrompt`, shared with `credential_view`. Files (m8/18e B1, move-only, paths unchanged): `empty_state/clone.rs` holds `CloneState`, `CloneRun`, `clone_request`, `clone_started`, `outside_job`, `clone_prompt_key`, `clone_view`; `empty_state/open_prompt.rs` holds `o`'s keys, `resolve` and `complete`; both re-exported from `empty_state`. m8/21b-4 (21b-5 ruling 1, move-only): `empty_state/palette.rs` holds `act`, `source_of`, `palette_rows`, `palette_key`, `outside_paste` (re-exported; its cursor is `update::palette::first_runnable`, now `pub(crate)`), `render_palette`.
- `first_launch` (mod, `pub(crate)`, m8/18d) — `<config_dir>/<config::config_names().dir()>/launches` (`<n>` or `done`): `SHOWN_FOR` (5), `FirstRunError` (`Read`/`Garbled`/`Write`, `words()` = `first-launch: <path>: …`), `count_launch(dir) -> Result<bool, _>`, `first_launch_done(dir)`, `launches_file(dir)`, `for_this_launch(Option<&Path>) -> (FirstLaunch, Option<warning>)` (only `places::drive` calls it), `FirstLaunch` (the process's switch, held by `places::drive` only, m8/18e — no state holds it; `shown(dir)`, `line() -> Option<PathBuf>` (what each arrival is seeded with), `end() -> Option<PathBuf>` — hides it in every later place, gives the dir to write `done` under once; `Default` = never shown; no `PartialEq`). `OutsideState::first_launch: Option<PathBuf>`, `AppState::first_launch: Option<Box<PathBuf>>` (boxed: `AppState` size), `Arrival::first_launch: Option<PathBuf>` — plain values; the first palette open sets them `None` and asks the loop: the graph pushes `Command::FirstLaunchDone(dir)` (carried by `running::carry`, which ends the switch lent through `Outside::first_launch`, then writes; with no switch lent it writes the named dir), the empty state returns `OutsideStep::FirstLaunchDone(dir)` (carried by `places::carry_out`; a failed write is the problem line). Test seams `test_config`, `test_read`.
- `recent` (mod, `pub(crate)`) — the recent-repository list, `<config_dir>/<config::config_names().dir()>/recent`, lines `<unix secs>\t<canonical path>`, newest first, at most `MAX_RECENT` (20): `read_recent` (missing file → empty; bad lines skipped and counted), `remember` (dedup, cap, atomic temp-file + rename), `config_dir()`, `RecentError`.
- `config` (mod) — config file loading and types.
- `crash` (mod) — panic handling and terminal restore on crash.
- `event_loop` (mod) — the run loop, input, signals.
- `history` (mod) — history walk, layout, streaming.
- `search` (mod) — `/` search over commit subject, author, sha prefix and body (m1-the-graph/26).
- `search::first_hits(store, query, limit, stop) -> Result<Vec<CommitHit>, String>` (fn, m8/12b) — the palette's COMMITS: `/`'s own match over the same walk (`scan` shares `walk_matching`), stops at `limit`. `CommitHit { id, subject }`.
- `jobs` (mod) — background job runner.
- `refs` (mod) — refs on each commit and ahead/behind, for the ref badges.
- `layout` (mod) — the detail pane's saved split, `<app_dir>/layout` (`RepoStore::app_dir`) (m2-details-and-diffs/10b, ADR-0028).
- `map` (mod) — the Map view's per-branch bucket counts: pure data, no drawing (m2-details-and-diffs/18).
- `repos` (mod) — repository store, opened once and cached.
- `rename` (mod, m8/26) — every open settled first: `open_settled(path)` (`pub(crate)`; reopens when the names changed), `Settlement` (`pub(crate)`; the outcome, the repository root and the old-names hold, kept for the store's life), `words(&SettleOutcome, root: &Path) -> Option<String>` (`rename: still using the old names · …`, `rename: old names left behind · …`, `rename: done · not in the journal: …`, and for `OlderWrote` or any `kept_aside: Some(p)` `rename: an older version wrote under the old names · close it[; kept in <p relative to root>]` (26r); `None` on a clean move).
- `repository` (mod) — facts about one open repository.
- `session` (mod) — startup and per-run session state.
- `terminal` (mod) — raw mode and terminal session setup/restore.
- `update` (mod) — app state and the update function.
- `view` (mod) — draws state to a frame. It is the **layout site** ADR-0040 clause 3 names: `view::frame` asks the same `GraphScreen` the rows were drawn with for its `column_boundaries` and snaps the confirm box onto them (`graft_ui::graph_screen::snap_to_columns`), clearing its band full width only if no snap fits inside the frame; the credential prompt's band it clears outright, at every width, footer untouched. Neither widget learns the graph's geometry. The three modal boxes (confirm, name prompt, credential) are drawn by one private `draw_modal_boxes`, called once in `view_with_graphics` **after** whichever screen or overlay drew — so the resolver, the commit editor and the status screen draw a live box too (m6-07 F10; pinned by `view::frame::tests::a_live_confirm_box_is_drawn_over_every_overlay`). A bottom row that stands in for the graph footer — the resolver's, and in the sub-100-column tab layout the Detail tab's hint row and the focused banner's key row — goes through the private `draw_status_or_hints`: problem, else notice, else keys (m6-07 F9; `…::the_detail_tab_below_100_columns_draws_the_status_line`), each indented one column like the graph footer's (rulings 07b Q3; `…::a_status_on_a_standin_footer_starts_at_column_one`). **Under a live box the bottom row is the box's keys and nothing else** (rulings 07b Q1): `draw_box_bars`, one site before `draw_modal_boxes`, redraws every bar row (`bar_rows`: the resolver's, else the primary pane's bottom row; none on the status screen) with `view::rows::box_context` — credential, else name prompt, else confirm — which `footer_context` and `hint_context` also return first (`…::the_bottom_row_under_a_live_box_offers_only_its_keys`). The confirm box over the resolver clears its band full width rather than snapping (rulings 07b Q2, ADR-0040; `…::the_start_over_box_clears_its_band_over_the_resolver`).
- `examples/gallery` (dev-only example, not in the binary) — walks every test repo with `]`/`[`, grouped as a progression; a three-line banner under the header names the stop, group, keys and tier, then `shows:` and `differs:` (m1-the-graph/38); `--list` prints the same lines, `--matrix` writes `docs/shots/35-coverage-matrix.txt` (m1-the-graph/35). Its pure half is `tests/it/gallery.rs`, included by `#[path]`; `Entry::clock()` (m1-the-graph/53) says which clock each stop views its dates at — `Fixed(graft_fixtures::CORPUS_NOW)` for a corpus shape, `Wall` for a real repository — and `show` passes it into the `Harness`. `--group graph|details` picks the progression (m2/23; default graph); `--list` prints both. `tests/it/gallery.rs`: `Group`, `stops(group)`, `keys_label`, `capture_stem`, `GIT_GIT_RELEASE_MERGE`. `--group details` walks M2's 19 key-prelude stops; `--stems` prints their capture stems (`tests/it/gallery.rs::capture_stem`, the coverage matrix's details rows); `--matrix` writes the whole matrix through `tests/it/coverage.rs::write_matrix` — the gallery's two tables (`tests/it/gallery.rs::gallery_matrix`) plus the M3 and M4 walks' tables from the stem constants `coverage::{M3_STEMS, M4_STEMS}`, so no walk is compiled here and `gallery`, `m3_walk` and `m4_walk --matrix` write the same file (m2-details-and-diffs/23, m4-remotes/07 fix).
- `examples/m5_walk` (dev-only example) — M5's acceptance walk (m5-history-and-ghost-preview/11): all 49 scenarios the ticket lists (m5/37, m5/38, m5/39 closed the last of them; `TESTING.md` §History numbers them), each a `Scratch` copy of a corpus shape — or, when `Scenario.remote` is `fixture::Remote::Local`/`Http`, a `LocalRemote` work clone whose `origin` is a bare over `file://` or that same bare over Basic-auth HTTP (`HttpRemote`, the one transport that makes git ask for a password) — with a history scripted by git (`fixture.rs`: `forked`, `forked_on_main`, `fast_forwardable`, `conflicting`, `three_to_stage`, `diverged`, `twice_conflicting`, `ahead_to_merge_and_push`, `one_to_push`; `USER`/`PASSWORD`/`SHORT`/`SECRETS`, `root()`, `requests()`), the app driven headless by `Step`s — keys, `Type`/`TypeSecret`, `CursorTo`/`CursorToSubject`, `MenuPick`/`MenuPickOpen`, a mouse `DragBadgeOnto`, `Claim`, `Confirm`/`Choose`, `KeyFor` (a key taken for its commands only, so `e`'s `$EDITOR` never starts), `Cancel` (`Esc` on the write in flight, the real `CancelJob` path), `Relaunch` — over the real job pool, then **truth read by running git** (`truth.rs`: `Truth` = `RepoTruth` + `head_symbolic` + `index_tree` + in-progress marker + journal + bare refs + statuses). The one `AppState` read is the claim (`PreviewState.plan`), judged by `claim.rs::ghost_equals_git` (ref moves, ghost parents/subjects, patch-ids — reversed for a revert — HEAD, worktree paths ⊆ changed files); `every_scenario::ghost_equals_git_is_the_only_claim_check` greps the scenario files for any other plan read. Same CLI as `m4_walk`. Split: `mod.rs`, `steps.rs`, `fixture.rs`, `truth.rs`, `claim.rs`, `run.rs` (`Walk`, `wait_for`, `Seen`, the askpass wrapper script that logs a helper's argv and environment before handing over, and `no_secret_leaked` — run after **every** scenario over the askpass log, every event, status line and open-prompt `{AppState:?}`, and every byte of every file the fixture leaves behind), `gestures.rs` (cursor, menu, drag, stage), `vocabulary.rs`, scenarios by promise (`preview.rs`, `merge_rebase.rs`, `pick_revert_reset.rs`, `conflicts.rs`, `time_machine.rs`, `remotes.rs`, `regressions.rs`), `every_scenario.rs` (one test per scenario, the captures/matrix check, `contact_sheet_captions_cover_every_m5_capture`). Its table is `tests/it/coverage.rs::history_matrix` over `M5_STEMS` (the fifth table). Captures: `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py`, whose `SECRETS` is the same two strings as `fixture::SECRETS`. The binary also answers **askpass mode** itself (`cli::askpass` → `run::askpass::respond`), as `m4_walk` does — without it the HTTP scenario's push fails `authentication failed` and git never asks.
- `tests/it/m5_walk/` **M6 additions** (m6-conflicts/07) — the same table gains M6's 19 scenarios (`TESTING.md` §Conflicts walk): `resolving.rs` (`Shape::StoppedMerge`: takes, whole-file cards, CRLF, base, `n`/`N`, start over, `$EDITOR`, mergetool; `STOPPED` is git's status order of its 13 paths) and `resolved_operations.rs` (rebase stops twice, stash pop with and without the app, `kill -9`, `u` after `c`, the re-push lease, modality, the tiny frame). New `Step`s: `Kill` (drop the app with no shutdown and reopen; asserts `truth::operation_on_disk` and `truth::last_journal_line_is_pending`), `Resolve(path)` (open the resolver from the changes pane), `HandOver(label, key)` (carry out `e`/`M` with the loop's own `event_loop::outside` builders and git's own editor lookup, `GIT_EDITOR`/`VISUAL`/`EDITOR` stripped from that lookup). New `Wait`s: `ConflictOpened`, `ResolvedPath(path)`, `ConflictedPath(path)`, `ResolverClosed`. `fixture.rs` adds `as_built`, `merge_aborted` (the stop's merge aborted, so the walk begins it in the app — ruling F5), `with_an_editor`, `with_a_mergetool` (`EDITED`, `TOOL_WROTE`), `stash_to_pop_over_a_conflict`, `twice_conflicting_in_two_places`; `truth.rs` adds `resolved_bytes`, `side_bytes` (`git checkout --ours|--theirs` on a fresh copy), `unmerged`, and re-exports `graft_ops::operation::Side`. The M6 table is `tests/it/coverage.rs::conflicts_matrix` over `M6_STEMS` (the sixth table; `--matrix` also writes it alone to `M6_MATRIX_PATH`, `.scratch/m6-conflicts/matrix.txt`); `every_scenario::contact_sheet_captions_cover_every_m6_capture` keeps `docs/shots/m6/` captioned. Captures: `.scratch/m6-conflicts/pty-walk-m6.py` (`m6-06` relaunches the real `graft` after a real `SIGKILL`, finding the scratch through `TRIB_CAPTURE_REPO_OUT`, which `examples/m5_walk --capture` writes).
- `tests/it/m5_walk/` **M6 journeys** (m6-conflicts/21) — `journeys.rs`: `parse_counts` reads the one ```` ```journey-counts ```` fence of `docs/design-notes/m6-conflicts.md` (`Row{id, keys, parts}`; `-superseded` ignored; a stray token is an error), `journey_id` (`name` or `name@variant`), `within_the_note` (run by `run_scenario` on every scenario: a journey sending more inputs than its row fails naming both numbers and the keys), `inputs_of` (UX 1.3: key/chord/click/`Type` = 1, waits and truth reads 0), `frame_sees`, `click_file` (clicks the list row as drawn, `…`-cut names too; `$TRIB_WALK_FRAMES` dumps a failing frame). `conflict_journeys.rs`: 24 scenarios walking all 19 rows, landing asserted from the drawn frame after every event; ten of them carry the pty captures `m6-10`…`m6-19` (`captured`: keys from the app's first frame to the landing; `m6-18` is the launch itself, no keys — m6-conflicts/43); its tests `journey_counts_match_the_note` (ids both ways, whole = sum of parts, prints the route table) and `one_key_over_the_note_fails`. New `Step`s `Route(bool)`, `Click(path)`, `Frame(what, &[Sees])` (`Sees::{Has, Lacks, BottomStarts, Row, Above}`); new `Wait`s `LandedOnHead`, `OnTheBanner`, `BackInProgress`, `FrameSettled`. `fixture.rs` adds `three_blocks`, `two_files`, and the shell-begun `*_in_the_shell` variants plus `two_files_pulled_in_the_shell`.
- `tests/it/m7_walk.rs` **M7 acceptance** (m7-power-tools/26) — `noted_keys(id)` reads both `journey-counts` fences of `docs/design-notes/m7-power-tools.md` through `m5_walk::parse_counts`; `WALKS` maps each of the 29 ids to (file, test); `every_m7_journey_id_has_a_walk` fails on an id with no walk, a stale entry, or a file lacking the id literal or the test fn. `m7_walk/promise.rs` (`tree_bytes`: every working file's bytes) holds the promise scenarios `rewrite.rs` (11 keys), `trace.rs` (8), `recover.rs` (16, `core.autocrlf=true`, byte-exact `u`; a pinned-clock child like `journal::the_three_lost_work_journeys`) and `guard.rs` (a refusal's status line at 80 columns). `m7_walk/pull.rs::pull_current_branch` walks `pull-current-branch`; `blame::Route::keys` returns a route's keys. Run by `scripts/m7-accept.sh`.
- `tests/it/quit_waits/pty_session.rs` **hang-up harness** (m8/13g, split out and made safe in m8/13l) — `Session` (the real binary as a pty session leader: `spawn` — git's `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` pinned to `preview::FORK_NOW`, so what the app writes (a set-aside's `S`) has the same id and age every run, 19d-a3 —, `send`, `wait_for`, `hang_up` closes every master fd, `hang_up_again` is `kill -HUP`, `exit_within`), `Stderr::{Pty, File}`, `Leftovers` (kills held pids on drop, by pid), `hook`, `wait_for_line`, `assert_no_crash`, `PATIENCE`. `session_command(program, args, slave) -> Command` re-execs the test binary into `session_exec_entry` (args in `SESSION_EXEC_VAR`/`_ARGC`/`_ARG_<n>`, stdout null), which `setsid`s, takes fd 0 as its controlling terminal (`rustix::process::ioctl_tiocsctty`), points fds 1/2 at it and execs the program under its own pid; no `pre_exec`. The `it` crate root is `#![forbid(unsafe_code)]`. Used by `quit_waits/hangup.rs` only.
- `tests/it/tour_tasks.rs` **tour ticks by doing** (m8/19c-b) — the real binary on a 300×50 pty (no crate-external seam seeds a practice arrival headless), one tour per test from the empty state's `t`, `TRIB_PRACTICE_EPOCH` fixed, `TMPDIR` a short `/tmp` folder (git's askpass socket lives there; `SUN_LEN`). `Tour` drives each task by its keys (`stage`, `commit`, `undo`, `stash_and_back`, `branch`, `merge_feature`, `merge_clash_to_the_stop`, `resolve_and_finish`, `push`), each waiting on a loaded-state marker before its next key; `assert_bits` waits for the drawn ticks row (`drawn_ticks`, parsed from `graft_ui::tour_card::ticks_text`'s form), checks the card names the first open task, then `graft_ops::tour::tour_progress` on the practice repo's journal.
- `tests/it/tour_walk.rs` **tour captures and `P` under a long `TMPDIR`** (m8/19c-c) — `the_tour_captures_match` (step 1 at 120×36 from his repo by `⌃P tour ⏎`, at 80×24 from the empty state's `t`; `TMPDIR` = a random `/tmp/graft-capture/<scenario>-<6>/practice-tmp`, the random suffix written as `X`s by `steady`); `push_in_the_tour_fits_the_askpass_socket_under_a_default_length_tmpdir` (C+2: `TMPDIR` exactly macOS's 49 chars, `P y` lands, task 8 ticks); `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` (19d-c: all eight tasks by the card's keys from his repo at 120×36, the two merges' set-aside rows and `M  notes.txt` kept after each, 7 ticks drawn at task 8, home on his commit with the finish status; his repo, journal, `.git/graft` and state folder byte-equal before/after; captures step 7 and finished).
- `examples/m4_walk` (dev-only example) — M4's acceptance walk (m4-remotes/07): 21 scenarios, each a `graft_fixtures::remote::LocalRemote` (a bare over `file://` and its work clone) or the same bare over `HttpRemote` (Basic auth) or a `HungRemote`, the app driven headless by `Step`s over the real job pool with an askpass helper, then **truth read by running git on both the work clone and the bare**. Same CLI as `m3_walk` (`--only`, `--list`, `--stems`, `--capture`, `--matrix`); the binary also answers **askpass mode** itself (`cli::askpass` → `run::askpass::respond`), so a captured push can ask for a password. Pure half `tests/it/m4_walk/` (split by seam, m4-remotes/07 fix): `mod.rs` (`scenarios()`, `stems_line`, `capture_prelude`, re-exports `Scenario`, `run_scenario`, `prepare_capture`), `fixture.rs` (`RemoteKind`, `Fixture` — `build`, `change_on_remote`, `many_objects_on_remote` — `USER`/`PASSWORD`/`WRONG`/`SECRETS`), `truth.rs` (`Truth`: work `RepoTruth` + bare refs + journal + every status line + the remote's counters; `truth`, `diff_truth`, `Marks`), `steps.rs` (`Step` — `TypeSecret` types a password the walk never prints — `Wait`, `Scenario`, `Capture` with `secret` and `When`), `run.rs` (`run_scenario`, `prepare_capture`, `askpass_helper` — `CARGO_BIN_EXE_trib` under test, `current_exe` in the example — `no_secret_leaked`, run after every scenario: `PASSWORD`/`WRONG` in no event, status line or journal byte), `vocabulary.rs` (the step and assertion helpers), the scenarios by promise (`fetch_pull.rs`, `push.rs`, `credentials.rs`, `hangs_and_undo.rs`) and `every_scenario.rs` (one test per scenario under `cargo test`, `#[cfg(test)]`). The remotes table is `tests/it/coverage.rs::remotes_matrix` over `M4_STEMS`; `--matrix` writes all four tables through `coverage::write_matrix`, so the three `--matrix` commands agree.
- `examples/m3_walk` (dev-only example) — M3's acceptance walk (m3-working-tree-undo-and-stash/07): 22 scenarios, each a `Scratch` copy of a corpus shape, a scripted dirty state, the app driven headless by `Step`s over the real job pool, then repository truth read by running git. `ok <name>` / `FAIL <name>: <≤20 lines>`, exit 1 on failure; `--only <name>`, `--list`, `--stems` (capture stem, scenario, expected text, key label), `--capture <name>` (setup + headless prefix, then the interactive loop with the capture's keys as the prelude — what `pty-walk-m3.py` records), `--matrix` (all four tables, through `tests/it/coverage.rs::write_matrix`). Pure half `tests/it/m3_walk.rs`: `Step`, `Capture`, `Scenario`, `Marks`, `scenarios()`, `run_scenario`, `prepare_capture`; under `cargo test`, one test per scenario (`every_scenario::*`). Git as evidence is shared with the M4 walk in `tests/it/repo_truth.rs` (`RepoTruth`, `git_truth`, `diff_truth`, `git_path`, `run_git`, `neutralise_global_config`, `fsck_clean`, `keys_label`); its table is `tests/it/coverage.rs::working_tree_matrix` over `M3_STEMS`.
- `examples/budgets` (dev-only example) — `budgets <repo>` times first paint, G, g, RSS through the real jobs (`tests/it/budget_run.rs`); `budgets --phases <repo>` (m1-the-graph/55, `examples/budgets/phases.rs`) times first paint phase by phase on one thread with no jobs, plus discriminator laps. Each phase prints its peak-RSS and footprint rise; `refs.ref_snapshot` also prints `TipStats` (m1-the-graph/58b). Without a commit-graph `--phases` continues through `examples/budgets/no_cg.rs` (m1-the-graph/59): open, first paint, date walk to end, `topological_order`, promote swap (labelled "synchronous path, not the app's": `Views::promote`, which the app no longer takes, m7/12e), `g`, each with footprint, interner, order and resident bytes.

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

## jobs::rewrite_open
The read behind `R` (m7-power-tools/08). `Command::OpenRewrite { id, req: RewriteRequest }` (`Open { from }` on the graph, `FromRebase(params)` from a rebase preview, rebase-preview-keeps-drops) → `Task::OpenRewrite` → `Product::RewriteOpened(Result<RewriteOpened, String>)`.
- `RewriteOpened` (enum) — `Editor(Box<OpenedRewrite>)` | `Refused(Refusal)`.
- `OpenedRewrite` (struct) — `params` (`plan(req)`), `whole: HashMap<Oid, String>` (every row's whole message, `history::read_commit`), `base: BaseRow`.
- `BaseRow` (struct) — `oid` (`onto`), `subject`, `label` (`Open`: upstream's short name if `onto` is the merge-base with it, else the default branch's, else the short sha; `FromRebase`: `preview::onto_label(params.onto)`).
- `open_rewrite` (fn, `pub(crate)`) — `(repos, req: &RewriteRequest, default_branch, stop)`: the read itself.

## history
- `Wanted::Find { commit, rows, fallback }` — lay out until `commit` has a row, answer around it (search's far hit); `fallback` when the walk ends without it.
- `History::seeking` (field) — the commit a pending `Find` is to select.
- `History::seek` (fn) — select a commit: at once if on hand, else a `Find` request (a jump, so Esc withdraws it).
- `History::restore` (fn) — put cursor/top back (search cancelled) and ask for that window.
- `keep_cursor_in_view` (fn, `pub(crate)`, `update::in_view`, m7-power-tools/48) — `(before, after, commands)`: last-but-one step of `update`. When `graph_rows` shrank (a dirty status pinned the uncommitted row) and the cursor was on screen before, moves `top` so it stays drawn and asks `History::show` for the rows. A cursor the wheel left off screen stays off (m7/37); nothing while `on_working_tree`.
- `walk_is_reading` (fn, `pub(crate)`, `update`, m6-conflicts/85 A) — `(&AppState) -> bool`: `awaiting` and (no history yet, or `History::is_reading`). The one reading of "the walk is reading": `update::keys::escape_key`'s (`keys/escape.rs`) last arm cancels the walk only when it is true (a parked or complete walk: `Esc` does nothing, no command, no status), and the header's `WalkView` draws reading/jumping exactly then.
- `serve` (fn) — the history worker's loop: answers each `RowRequest`; between requests, `Views::idle_step` a chunk at a time to the end of history (every `stop` check also takes a waiting request, so a request or cancel ends a step within one pull), then parks in `recv` — no wake, no CPU (m7-power-tools/12 Part B). Lives in `history/serve.rs` (moved out of `views.rs`, 12c); `Views` in `history/views/mod.rs`, its promotion state machine (`begin_promotion`, `promote`, the pending layout, swap, `reach`, `aim`) in `history/views/promotion.rs` (12f). A request carrying `promote` starts `Views::begin_promotion` and, until the swap, is answered from the provisional layout with `promote` cleared (12 Part C).
- `Views::idle_step` (fn, `pub(crate)`) — one bounded step nobody asked for: while a promotion is pending, one chunk of the promoted layout (swapping and re-answering the last request once ready); otherwise ≤ one `advance(chunk_rows, stop)` of the view last asked for, `HistoryUpdate::LaidOut` once per layout at the end (again after a fold or promotion). Returns `IdleWork`.
- `Views::begin_promotion` (fn, `pub(crate)`) — `(view, &Promote) -> Result<bool, StreamError>`: `true` if that order is already in force; else keeps (or opens) the pending promoted layout, re-aims its anchor, and swaps at once if ready. Ready = the layout is done, or has ≥ one chunk laid out and a row for the anchor (12c: a swap before the first chunk left the race short of `Complete`).
- `IdleWork` (enum, `pub(crate)`) — `Advanced { rows }`, `Promoting { rows }` (a pending promotion took the step), `Caught` (end reached or nothing asked: park), `NobodyListening`.
- `WindowOrder` — not a type: `Window::promoted` is the spec's `WindowOrder` (true = rows in the promoted order; the UI swaps order and re-anchors only on such a window, 12 Part C).
- `HistoryUpdate::LaidOut { rows }` — the walk reached the end of history unasked; `rows` is every row of the view. Not a request's answer: the header's "reading" and `Esc` ignore it.
- `History::laid_out` (field) — `Option<usize>`, the last `LaidOut` total heard; nothing is drawn from it yet.
- `History::is_reading` (fn) — streaming and working on something asked (a jump, an outstanding request, or no window yet). With `awaiting`, what `Esc` cancels and what the header calls reading (`update::walk_is_reading`, m6-conflicts/85). Once cancelled or failed, motions clamp to the held window.
- `DetailState::file_written` (fn) — a file written outside any operation (a resolver take): drops that path's cached working-tree diffs and re-asks the cursor's diff when it is that path (m6-conflicts/85 B).
- `History::strip_rows` (field) — pixel tier's strip height; requests widen to whole strips plus the row above. `None` (text tiers) asks exactly as before.

## session
- `Startup` (struct) — what `main` decided before touching the terminal; `harness` is what a wrapper around `run` asks for (`update::Harness`, default in the binary).
- `Session` (struct) — what Graft is drawing with, for as long as this run lasts.
- `Session::icons` / `Session::max_lanes` — **gone** (m5-history-and-ghost-preview/33). Both moved to `AppState`: they decide how wide a badge is and how wide the graph column is, so they decide where a *drag* has to hit a badge, and the mouse hit-test has no `Session`. See `update::AppState::icons`.
- `Session::settle` (fn) — resolve capabilities, theme, rendering info at startup.
- `Session::apply` (fn) — take in a re-detected cell size or theme; `true` when anything changed. Never touches the tier. A theme change goes through `retheme`.
- `Session::retheme` (fn, `pub(crate)`, m8-16) — rebuild the theme as `kind` with that kind's `[theme.*]` overrides (kept privately from `settle`'s `&Loaded`); the only mid-run rebuild path; `true` if the theme changed. Palette theme rows call it through `choose`.
- `Session::choose` (fn, `pub(crate)`, m8/11) — `retheme(kind)` and remember it as his choice for the run: a later re-detected background (`apply`) keeps it. Called by the loop for `Command::Retheme` (`event_loop::running::turn`, `rethemed`, which flushes `Flush::Theme`).
- `TerminalChange` (enum) — a mid-session answer that changed: `CellSize(CellSize)`, `Theme(ThemeHint)`.
- `Clock` (enum) — `Wall` (default) or `Fixed(i64)`; `now_seconds()` is the one `SystemTime::now()` read on the date path (m1-the-graph/53). `now_millis()` is the same instant at millisecond resolution, read for the focus-refresh debounce only (m3-working-tree-undo-and-stash/06). `update::Harness::clock` carries it; `view::graph_view` takes the result. **The one `SystemTime::now()` read for anything a capture can show** (m5/25): the journal's `at_unix_ms` comes through it too, via `JobRunner::with_clock`, so a pinned clock pins the ages the journal browser draws: **in-process only** (tests, examples). The shipped binary stamps from `Wall` always (m8/19e §R1).
- `launch_clock() -> Result<Clock, String>` (fn, m8/19d-b) — the binary's session clock, read once at launch from `NOW_VAR` = `TRIB_NOW` (`pub(crate)`) via `clock_from(Option<&OsStr>)` (`pub(crate)`, pure): unset → `Wall`, whole seconds → `Fixed`, else `Err("TRIB_NOW must be whole seconds since 1970, got <v>")` (sanitised); `main` prints it and exits 2 before the terminal is touched. The **drawn clock only** (m8/19e §R1): it reaches `Harness::clock` (both launch paths) and the practice repo's `now` (`places::Carried::clock`, a temp repo). The journal's stamps stay on the wall clock (`startup::run_repo` builds its `JobRunner` without `with_clock`), so a stray pin writes no false history. The pty harness pins it by default to `graft_fixtures::PINNED_NOW` (`tests/it/pty.rs`), and dates git's commits an hour before it (`GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE`, a caller's env wins; m8/19d-b2). Test-side: `app_harness::App::open_with_clock` pins the frame (`state.harness.clock`) as well as the journal; `view::tests::draw_pinned` draws a wall-clock state at `PINNED_NOW`; `clock_pin::no_capture_producer_runs_on_the_wall_clock` scans capture-writing sources for `Clock::Wall` / an unpinned `Harness::default()`.

## event_loop
- `Wake::Terminal` (variant) — a re-detected answer changed; `pump` applies it to the session and redraws.
- `Wake::TerminalFailed` (variant) — a re-query could not be written or read; ends the loop with the error.
- `pump` (fn) — idle, while nothing waits, rasterises the strips `PageDown`/`PageUp` would show (30h). Now takes `&mut Session`, so re-detection can update it.
- `input::spawn_reader` (fn) — forwards input; on `FocusGained` also asks the redetector for the theme; returns the `Forwarder`. Fourth arg `stamps: Option<Sender<Instant>>` (m8 G-trace): while tracing, each key/mouse event's forwarding time is sent just before the event.
- `input::Forwarder` (struct) — the running forwarder; `finish` waits for it to end (after `InputMux::hold`, a bounded drain, never a job wait).
- `signals::watch_resizes` (fn) — resize wakes; each also asks the redetector for the cell size.
- `redetect::Redetector` (struct) — handle to the re-query worker; `spawn(host, cell_size, theme, budget, wakes)`, `ask(Query)`.
- `redetect::Query` (enum) — `CellSize` or `Theme`.
- `Terminal` (struct, m8-polish-and-ship/08) — one terminal session for the process: crash hook, `InputMux`, one capability probe; `open()`, `close(Ran) -> io::Result<Exit>`. Every place `drive` runs shares it, so the alternate screen and the probe happen once.
- `Ran` (struct) — a place's outcome and its job-pool shutdown; `Ran::without_jobs`.
- `Exit::Open(Open)` (variant) — a screen asks to open another place; `drive` handles it (re-`locate`s), never returns it. `run` (gallery, walks) treats it as a broken promise. m8/19b: `Exit::Practise { from: Open }` (into the practice tour) and `Exit::LeaveTour { finished }` (out of it), likewise `drive`'s only.
- `Open` (struct, m8/19b) — `path` and `cursor: Option<Oid>` (`pub(crate)`), `Open::at(path)` (lands on `HEAD`), `path()`: where an exit goes, and the commit to land on there (only the tour's way back sets it).
- `places::Place` (enum) — `Repo { repos, path }` or `Outside(OutsideStart)`. `places::OutsideStart` (struct) — `folder`, `missing`, `cursor`, `problem`.
- `places::locate(path, &Loaded) -> Result<Place, String>` (fn) — before the terminal is touched: missing path → `Outside{missing}`; `NotARepo` → `Outside`; another open error → `Err` (printed, exit 1).
- `places::drive(Terminal, Place, &Loaded, Clock) -> io::Result<Exit>` (fn; `Clock` = `session::launch_clock`, m8/19d-b) — runs places on one terminal until one ends Graft; remembers each opened repository (tty only); builds the one keymap from `[keys]` (`config::keymap_from`) and hands it to every place (`Arrival::keymap`); config problems seeded on the first repository screen only (`Arrival::config_problems`, then `AppState::seed_keys`), and shown in the empty state's palette while there (m8/11); the outside screen's keys go through `empty_state::outside_edge` (m8-09b); a place reached by a launch or an open lands on `HEAD` (`update::lands_at_open`, m8/12d: every place, the first included; m8/19b: the tour's way back lands on the commit he left, `Carried::at`); m8/19b: the practice slot (`Practice { repo, back: Open }`) — `Exit::Practise` builds the repo under `std::env::temp_dir()` (failure: `tour: …` notice, back to `from`) at `places::practice_at(var, now)` (`pub(crate)`, pure, m8/19c): `PRACTICE_EPOCH_VAR` = `TRIB_PRACTICE_EPOCH`, read at the tour's start — unset → `PracticeAt::Now` (the session clock, m8/19d-b), whole seconds → `Fixed`, else the same failure path with `tour: TRIB_PRACTICE_EPOCH must be whole seconds since 1970, got <v>`, `Exit::LeaveTour` removes it and goes `back`, `Exit::Open` from inside removes it first, any ending removes it after `term.close` (failure: one stderr line); the practice repo is never put on the recent list; `t`/the palette row on the empty state is `Outcome::Practise`; each empty-state frame is one DEC 2026 update (`places::draw_whole`, m8/13h, as `running::frame::draw_frame`). `main` uses it on a tty; off a tty it uses `run`, as before. m8/18b: the empty state gets a lazy one-worker `JobRunner` (`places::OutsideJobs`, started on the first clone) on the loop's own channel; job wakes go to `empty_state::outside_job`; the pool leaves with the place (`Left { outcome, pool }`) — stopped at once on an open, by `wind_down` on quit (no waiting line: a clone is stoppable). A landed clone's notice rides `Arrival::notice` to the next place's first frame, once (`startup::arrive` seeds it as a sticky status). m8/18e: `Carried` holds the process's `FirstLaunch`; every arrival is seeded with `line()`; `run_repo(…, first_launch: Option<&FirstLaunch>)` lends it to the repository's loop (`Outside::first_launch`), `Carried::shown` to the empty state's; `places::answer_ask(pool, job, answer)` — the password box's answer: `esc` stops the job, then refuses the ask (order held by `places::tests::esc_in_the_box_stops_before_it_refuses`, behind the private `AnswersAsks` seam). Files (m8/18e B1): `places/outside.rs` holds the empty-state place's loop (`run_outside`, `outside_loop`, `Left`, `Outcome`, `Next`, `leave_outside`, `read_recents`, `draw_whole`); m8/21b-5 5j (move-only): `places/outside/carry.rs` holds `OutsideJobs`, `carry_out` and its steps (`init_and_open`, `start_clone`, `end_first_launch`), `AnswersAsks`/`answer_ask`, `try_open` (every caller is in `carry_out`), `pub(in crate::event_loop::places)`, re-exported from `outside`; `places/practice.rs` holds the `Practice` slot, `moved`, `practise`, `leave_practice`, `end`, `PRACTICE_EPOCH_VAR`, `practice_at`.

## refs
- `RefBadges` (struct) — refs on every commit that has one, plus the `HEAD` and stash commits.
- `RefBadges::on` (fn) — the refs on one commit, in drawing order.
- `RefBadges::head` (fn) — the commit `HEAD` resolves to; `None` while unborn.
- `RefBadges::stash` (fn) — the newest stash entry's commit.
- `RefBadges::count` (fn) — labels over every commit; the frame trace's `refs=` (m8 G-trace).
- `RefBadges::remotes` (fn) — `(Oid, &[u8])` per remote-tracking branch, short name (`origin/main`): what `history::preview_seeds` colours ghost lanes from (m5/08).
- `RefBadges::tags` (fn) — `(Oid, &[u8])` per tag, short name (`v1.2.0`): what the name prompt checks a typed tag name against, so `t` can say `exists` before it runs anything (m5-history-and-ghost-preview/10).
- `RefBadges::stashes` (fn) — `&[(Oid, usize)]`: every stash entry's commit and index, newest first, so each one gets its own badge rather than only `stash@{0}` (m3-working-tree-undo-and-stash/06).
- `RefBadges::is_stash` (fn) — `(id) -> bool`: whether `id` is any stash entry's own commit; `selection::follow_selection` uses it to keep stash commits away from the evil-merge probe (m6-conflicts/66).
- `RefLabel` (enum) — one ref before sanitising: detached HEAD, branch, remote, tag, `Stash(usize)` (the entry's index, drawn `stash@{n}`; m3-working-tree-undo-and-stash/06).
- `Upstream` (enum) — a branch against its upstream: none, in sync (remote), apart (counts).
- `AheadBehindCache` (struct) — counts keyed by (branch tip, upstream tip), shared by refs jobs.
- `AheadBehindCache::get_or_count` (fn) — cached counts, walking only for a new pair.
- `read` (fn) — read the repository's refs into `RefBadges` (a job's work); reads the stash entries through `graft_git::stash::commits` (reflog only, no `diff-tree`).
- `badges` (fn) — the pure half of `read`: refs, HEAD, upstreams, stashes, a counter → `RefBadges` (the `stashes` argument added m3-working-tree-undo-and-stash/06).

## crash
- `install` (fn) — install a panic hook that restores the terminal before dying. Reports go to `${XDG_STATE_HOME:-~/.local/state}/` + `OnDiskNames::New.dir()` (`tributary`, m8/27a; old `graft/` reports left where they are); the report's first line and stderr say `trib` (`cli::COMMAND`).
- `Panicked` (struct) — a panic that was caught rather than allowed to end the process.
- `catching` (fn) — run `work`, catching a panic as `Panicked` instead of aborting.

## trace (m8-polish-and-ship G-trace, `pub(crate)`)
- `VAR` (const) — `TRIB_PERF_TRACE`; renamed in G-rename (`TRIB_PERF_TRACE`).
- `FrameRecord` (struct) — one drawn frame's numbers: `at_us`, `wakes`, `motions`, `queued`, `update_us`, `commands`, `draw_us`, `bytes`, `write_us`, `fence_us` (how long the previous frame's fence waited for its answer; `-` when none came), `input_age_us`.
- `Header` (struct) — tier, columns, rows, `history_rows` (laid out so far), `refs` (`RefBadges::count`), term word; written before the first frame and on change.
- `FrameTrace` (struct) — the file; `from_env`/`from_var` (`Ok(None)` unset or empty), `writing_to`, `header`, `record` (flushed per line).
- `term_word` (fn) — `$TERM_PROGRAM` cut to `[A-Za-z0-9._-]`, ≤32, `-` when unset.
- `Meter` (struct), `Metered<W>` (struct) — bytes and time spent in the writes that reach the terminal; `Metered` sits under a `LineWriter`.
- `TerminalOut` (enum) — the terminal's writer: `Plain(Stdout)` or, tracing, `Metered(LineWriter<Metered<Stdout>>)`; `for_meter`.
- `Tracer` (struct) — the loop's side: `off`, `from_env`/`open` → `Opened { tracer, meter, stamps }`, `on`, `clock`, `took`, `updated`, `drew`, `fenced` (records the answer's wait for the next line), `unreported` (the failure reason, once).
- Tests: `trace::tests` (format, header, meter, stamps, failures); `scripts/m8-trace-summary.sh` (`--self-test` on `scripts/testdata/m8-trace-sample.trace`); `scripts/m8-trace-monorepo.sh` (real pty run on MONOREPO → `docs/perf/m8-trace-monorepo.txt`).

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
The pure `update(state, event) -> (state, [Command])`: the only place `AppState` changes; no clock, file, thread or drawing. Re-exports the state and event types.

## update::amend_into
`a` on an older commit and absorb from the `.` menu: put the staged changes into the commit they belong to, through the Ghost Preview and `y`.

## update::answer
`answer`: one arm per job answer, each handed straight to its module.

## update::answers
What `answer` hands a started job, a mergetool, the journal, a stash, an undo, a rewrite, the commit editor or an operation.

## update::banner
The operation banner: which operation git is part-way through, and the conflict interim's keys.

## update::clash
A stash put back that clashed: whether one is live, what landing on it does, and its words. Not a `Stop`: it has no git state.

## update::commit_editor
The commit editor's state and keys (`c`, `a`): message, template and amend, all pure.

## update::commit_graph
The commit-graph offer: whether to offer after a slow topological sort, and the `B`/`X` state machine.

## update::confirm
The confirmation box opened by `d` in the changes pane; it answers its own keys first.

## update::context_menu
The `.`/right-click popup: opening it, answering its keys, carrying out what it dispatches.

## update::credential
The credential prompt: git asks through the askpass helper, the user answers in one modal field.

## update::detail
The detail pane's state: what it shows, the jobs it waits on, and the caches that make stepping back free.

## update::dispatch
A key's way through the dispatchers, and the core events: resize, focus, job cancelled/failed/ended, rows the viewport asks for.

## update::drag
Dragging a branch badge onto a row: the three-event state machine and the menu the drop opens.

## update::edge
Where a pressed key meets the user's `[keys]` remaps, ahead of every dispatcher.

## update::events
What flows through `update`: events in, commands out, and the job-runner vocabulary between them.

## update::facts
Facts beside the graph arriving (system git, refs, working-tree counts), their read failures, and the name prompt's key claim.

## update::finish
The finish summary: `c` on a fully resolved merge shows what will land; `y` commits it.

## update::fold_count
A folded merge's `+N`: counted once by a job, kept for the session, never recounted.

## update::geometry
Pure screen geometry a mouse event needs: where the detail pane, its regions, the context menu and a ref badge land.

## update::history
The history walk's share of `update`: progress landing from the walk and the topological-sort promotion.

## update::in_view
Keeping a shown cursor shown when the graph pane loses history rows without a key.

## update::journal
The undo journal screen's state (`J`): cursor, marks, and the read that keeps it current.

## update::keys
The graph's own keys: movement, view changes and toggles other than search and the commit-graph offer.

## update::map
The Map view's state and keys (`M`): built fresh on open, thrown away on close.

## update::mouse
What a mouse event does once `geometry` says where it landed: clicks, wheel, divider drag, menu clicks.

## update::name_prompt
The one-field modal naming something new: a branch (`b`), a tag (`t`), or the branch a stash becomes.

## update::op_keys
graft-ops' key words spelled from the user's live keymap: the one `KeyNames` answer.

## update::operate
Working-tree writes from the changes pane: stage, unstage, stage all, and opening the discard box; one write at a time.

## update::palette
The command palette and help (`?`, `⌃P`, `:`): one state, one listing from the live key table, one filter.

## update::picker
The branch picker (`C`): type part of a name, `⏎` switches through `switch`, `Tab` shows it on the graph.

## update::place
Places: the blame and history screens opened over the graph, and the `^` trail `Esc` walks back.

## update::prelude
The harness's key prelude: keys a gallery stop presses before its first frame, each once the state is settled.

## update::preview
Ghost Preview: every history-changing key opens this first; the answer is drawn as ghost rows, `y` runs it.

## update::quit
`q`: quit, unless a write is running; then a box says what quitting does to it.

## update::radar
The Mistake Radar's state: the default-branch context its rules need, and the resolved `[radar]` policy.

## update::recover
A put-back from the journal once it has run: the line it leaves and the tags it puts on the graph.

## update::relaunch
Landing on an operation git is part-way through: the banner takes the keyboard and the changes pane opens where the work is.

## update::remote
Fetch, pull and push (`f`, `p`, `P`): the push box, `Esc` cancelling a running write, progress and failure lines.

## update::resolver
The conflict resolver: a modal over one conflicted file; `o`/`t`/`b` take a side per block, `⏎` marks it resolved.

## update::rewrite_editor
The rewrite editor (`R`): the branch as rows, the plan and squashed message shown before `y` runs the rewrite.

## update::row_tags
Tags a graph row carries after its subject until the next write starts, e.g. `✎ changed`.

## update::search
`/`: the prompt, the running search and stepping between hits.

## update::selection
The uncommitted-changes row's one-bit selection, kept beside `History`'s cursor.

## update::stash
The stash overlay's state (`S`) and its keys: save, apply, pop, drop.

## update::state
Everything Graft knows as one value (`AppState`), plus the small types it is built from.

## update::status_read
The working-tree status read the app is waiting on after a write, and the line that waits for it.

## update::strip
`O`: the overview strip's only key.

## update::switch
The switch rule every switch goes through: `.` Checkout, the name prompt's `⇥`, the picker's `⏎`.

## update::tour
The practice tour inside the repository Graft built for it: ticks read from that repository's journal.

## update::tree_answers
What `answer` hands the repository opening, the history walk starting, the working-tree read and the stash entries.

## update::undo
`u`, `Ctrl+Z` and `U`: when a key is a no-op, the drift guard, and the `Command::Undo` it becomes.

## view
The pure `view(state)`: reads and draws, never changes anything; everything from a repository crosses into `ui` as `SanitizedText`.

## view::commit_editor
The commit editor's view model for `graft_ui::commit_editor`, every repository string sanitised.

## view::credential
The credential prompt's view model; `secret` decides how the text is drawn, no character counter.

## view::detail
The detail pane's view model, reshaped from what `update` keeps.

## view::entries
One frame's `GraphEntry` rows: the selection's thread, the uncommitted row, each row's cells and badges.

## view::explain
A radar `Explanation` turned into the sanitised `ExplainView` the Explain panel draws.

## view::finish
The finish summary's view model; keys, mouse and draw build the same view.

## view::frame
The top of the draw: picks the tier and screen, then hands off to `rows` and `status`.

## view::hit
Which drawn thing a screen cell belongs to: the hit-test half that needs the view's layout.

## view::journal
The undo journal screen's view model (`J`): rows, with marks answered by `graft_ops`.

## view::map
The Map screen's widget input, built from `state.map`.

## view::name_prompt
The name prompt's view model: the title says what is being named and where.

## view::picker
The branch picker's view model for `graft_ui::picker`; row order is `graft_ops`' answer.

## view::preview
Drawing an open Ghost Preview: the slice around the cursor with ghosts laid in and refs where they would land.

## view::radar
`AppState` facts turned into radar `Finding`s, then into the graph marker and the detail pane's radar lines.

## view::resolver
The conflict resolver's `DocView`: the one sanitisation site for a conflicted file.

## view::rewrite_editor
The rewrite editor's view, its area and its bar's context; every non-refusal word is chosen here.

## view::rows
Building `GraphView`: the rows the walk last sent, folded and painted for this frame.

## view::stash
The stash overlay's view model (`S`): rows from the last read; decides nothing.

## view::status
Building `StatusView`: what was settled at startup and what has happened since.

## view::strip
Building `StripMarks`: the overview strip's marks, read off the rows `History` holds now.

## view::tour
The tour card's words: one line per tour task, every key spelled from the live keymap.

## view::strips
The pixel tier's rows: the viewport's history cut into whole aligned strips, painted like the Unicode tier.

## search
- `Query` (struct) — a case-insensitive literal needle; `parse` (None when blank), `text`, `matches(id, subject, author, body)` (sha prefix only when all hex).
- `SearchState` (enum) — `Running{hits}` / `Done{hits}` / `Cancelled{hits}` / `Failed(String)`; `describe` words each distinctly ("N matches so far", "no matches", "search cancelled, N matches found", "search failed: …").
- `Scanned` (enum) — how `scan` ended: `Finished` or `Stopped`.
- `scan` (fn) — walks every ref chunk by chunk, handing each chunk's hits to `found`; stops on `stop` or when `found` returns false.
- `Search` (struct) — the search on screen: query, owning job, hit batches (`Arc` so state clones stay cheap), current hit, prior cursor/top for Esc, `seeking`/`missed`; `hits`, `hit`, `step(forward)` (wraps), `line(keys: &Keymap)` (m8/10h; keys from `search::keys_words(running, hits, keys)`, pub fn: `n/N next/prev  esc close|cancel`) (footer text).

## repository
- `RepositoryFacts` (struct) — what one open repository told Graft.
- `RepositoryFacts::path` (field) — working tree root, or the repo dir itself if bare.
- `RepositoryFacts::bare` (field) — no working tree.
- `RepositoryFacts::head` (field) — where `HEAD` points.
- `RepositoryFacts::mailmap` (field) — the repository's resolved `.mailmap`, for the author chip.
- `RepositoryFacts::commit_graph` (field) — `CommitGraphFact` (m1-the-graph/54).
- `CommitGraphFact` (enum) — `Present { commits }`, `Absent { declined }`, `Shallow` (a non-empty `ShallowList`: git reads no commit-graph there, so the offer is `NotNeeded` and `B`/`X` are not taken; m7/20j), `Unreadable(msg)` (shown as a problem, no offer; also an unreadable shallow file).
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
- `Config::theme` (field, m8-16) — `[theme.<kind>]`: `ThemeOverrides`, read off the raw table by `config::theme::overrides` (serde skip).
- `ThemeOverrides` (struct, m8-16) — `dark`/`light`/`high_contrast: Vec<(Token, Rgb)>`, key order; `for_kind(ThemeKind)`. Kinds `dark`, `light`, `high-contrast` or `high_contrast` (`config/theme.rs`).
- `theme_problems` (fn, `pub(crate)`, m8-16) — `(&ThemeOverrides, no_color) -> Vec<String>`: every contrast miss in all three kinds as `[theme.<kind>] <ContrastMiss::words>`; empty under `NO_COLOR`. Appended after `keymap_from`'s list by `Arrival::first(config, no_color)`.
- `Config::keys` (field) — `[keys]`: `Keys { suspend_on_ctrl_z }` (ADR-0002), the one key whose meaning is a preference (m3-working-tree-undo-and-stash/05).
- `Radar` (struct) — `[radar]` section: one `RadarSetting` per `MistakeRadarKind` (`foxtrot`, `backward`, `accidental_pull`, `evil`, `duplicate`, `force_pushed`, `risky_state`).
- `Radar::policy` (fn) — the `graft_ops::mistake_radar::Policy` this section describes: `Policy::default()` with only the set keys overridden; loaded into `AppState.radar.policy` by `event_loop::startup::terminal_and_state` (m2-details-and-diffs/13c).
- `RadarSetting` (enum) — `Default`/`Off`/`Info`/`Warning` for one Mistake Radar kind.
- `Diff` (struct) — `[diff]` section: `context: u8` (default 3), `ignore_whitespace: bool` (default false), `word_tint: bool` (default true); loaded into `DetailState::options`/`word_tint` by `event_loop::startup::terminal_and_state`.
- `Ui` (struct) — `[ui]` section.
- `Ui::theme` (field) — `ui.theme`.
- `Ui::icons` (field) — `ui.icons`.
- `Ui::overview` (field) — `ui.overview` (m8/17): the overview strip starts shown (`AppState::strip_shown` seeded in `seed_config`); `O` still toggles per run.
- `Graph` (struct) — `[graph]` section.
- `Graph::renderer` (field) — `graph.renderer`.
- `Graph::default_branch` (field) — forced default branch full ref name; `None` lets detection decide.
- `Graph::max_lanes` (field) — `graph.max_lanes`: lanes drawn before folding behind `+N`. `None` (unset, the default) follows the terminal's width; a set value below 2 is a parse error (m1-the-graph/44).
- `DEFAULT_MAX_LANES` (const) — 8.
- `ThemeChoice` (enum) — `ui.theme` values.
- `ThemeChoice::overridden` (fn) — the override to hand `ThemeKind::select`.
- `RendererChoice` (enum) — `graph.renderer` values.
- `RendererChoice::overridden` (fn) — the tier the user forced, or `None` for detected.
- `IconSet` (enum) — `ui.icons` values. `IconSet::glyphs() -> graft_ui::graph_screen::Icons` is the one translation into what the widgets take; read once into `AppState::icons` (m5-history-and-ghost-preview/33).
- `Loaded` (struct) — what `load` found.
- `Loaded::config` (field) — settings in effect: file values over defaults.
- `Loaded::path` (field) — file the values came from, or `None`.
- `Loaded::warnings` (field) — unrecognised keys, in file key order.
- `Keys::remaps` (field, m8-09b) — `Vec<(String, Vec<String>)>`: each `[keys.<ctx>] <verb> = "chord"` (or a list) as `("<ctx>.<verb>", chords)`, for `Keymap::with_overrides`. In **sorted** order (toml without `preserve_order`), so a collision's winner follows that order, not the file's. `serde(skip)`: filled by `load`'s typed parse (`config/keys.rs`).
- `keymap_from` (fn, `pub(crate)`, m8-09b) — `(&Loaded) -> (Keymap, Vec<String>)`: the keymap and every config problem as words, the file's warnings first, then `KeyWarning::words`. Passes `KeySwitches { suspend_on_ctrl_z }` from `[keys]` (m8/09c D2).
- `ConfigWarning` (enum) — a config problem worth telling the user, non-fatal. `NotAKey { path }` (m8-09b): a `[keys.<ctx>]` value that is not a string or a list of strings. `LaneColour { path }` (m8-16): a `[theme.<kind>] lane-N` line. `BadColour { path, value }` (m8-16): a theme value not `"#RRGGBB"`. Unknown theme kind/token → `UnknownKey`.
- `ConfigError` (enum) — a config problem that stops startup.
- `load` (fn) — load config from the default location, or defaults if none.
- `load_from` (fn) — load config from an explicit path (tests, future `--config`).
- `default_path` (fn) — where `load` looks: XDG or `~/.config` path.
- `config_names` (fn, `pub(crate)`) — the `OnDiskNames` the config root uses for `config.toml`, `recent`, `launches`: decided once by `load` (`New`; `Old` when the folder move failed this run); `New` when nothing decided (tests).
- `load` moves the folder first (m8/26, `config/folder.rs`: `migrate_config_dir(root) -> Result<ConfigDirMoved, io::Error>` — under an flock on `<root>/tributary.lock`; staged copy, read back, rename into place, old removed; both present (26r) → byte rule per old file: equal deleted, absent carried forward, differs kept in `tributary/older-version/<n>/`; `ConfigDirMoved::{Nothing, Moved, OlderWrote { old, kept_aside }, OldLeft { old, why }}`). `ConfigWarning::ConfigFolder { words }`: an older version wrote the old folder (that launch only), the old folder could not be removed (io::Error words), or the move failed. Test seam `load_settling(env)`.

## repos
- `RepoStore` (struct) — the open repository, and what has been worked out once.
- `RepoStore::new` (fn) — build a store for a path, with an optional forced branch.
- `RepoStore::path` (fn) — the path Graft was pointed at.
- `RepoStore::shared` (fn) — the shared open repository handle. The first open settles the on-disk names (`rename::open_settled`, m8/26) and counts as one open.
- `RepoStore::rename_words` (fn) — what the first open's name settle could not finish, for the first frame's status row; `None` when nothing to say.
- `RepoStore::refs` (fn) — the refs and their tips (`RefSnapshot`), read once, dropped by `refresh`; first paint's branches, seeds, walk tips and `refs::read`'s badges share it.
- `RepoStore::branches` (fn) — the repository's branches, resolved from `refs`.
- `RepoStore::table` (fn) — the repository's one `Arc<CommitTable>`, opened once, usually kept across `refresh` (rebuilt only when the commit-graph's presence changed).
- `RepoStore::opens` (fn) — how many times the repository has been opened from scratch, or its id table rebuilt for a changed commit-graph.
- `RepoStore::common_dir` (fn) — the repository's common directory, shared by every worktree; `app_dir` is under it (m2-details-and-diffs/10b).
- `RepoStore::app_dir` (fn) — `<common_dir>/<on_disk_names().dir()>`: the app's own dir, where `crate::layout`'s split and `Ctrl+E`'s `COMMIT_EDITMSG` live; not created here.
- `RepoStore::workdir` (fn) — the working tree's root, `None` for a bare repository. Every porcelain path is relative to it, so anything handed to another program is joined onto it first (m3/05 review M2).
- `RepoStore::refresh` (fn) — mark cached refs and conclusions (e.g. default branch) stale; returns `Result<Refreshed, StoreError>`, rebinding the id table to a new `CommitTable` when the commit-graph's presence changed since it was built (m2-details-and-diffs/21, m1-the-graph/54).
- `Refreshed` (struct) — `refresh`'s report: `table_rebound: bool`, true when a new id table replaced the old one.
- `StoreError` (enum) — why the store could not answer (open, refs, branches, id table, commit-graph status).

## cli
- askpass mode — with `TRIB_ASKPASS_SOCKET` set and exactly one argument (git's prompt), the binary answers git's credential question through `run::askpass::respond` and exits, never loading config and never starting the TUI: 0 answered, 1 the user cancelled (nothing on stdout, so git gives up), 2 anything else, with one `graft: …` line on stderr (m4-remotes/05, wired here in m4-remotes/06).
- `Invocation` (enum) — what the user asked for; `Askpass { socket, prompt }` is the mode above; `Usage(String)` (`--help`, `usage(FLAGS)`); `GenerateMan` (`--generate-man`, main prints `man_page`; m8/23).
- `CliError` (enum) — what the arguments could not be made sense of; `UnknownOption`/`TooManyPaths` end with `usage(FLAGS)` (words pinned by `cli::tests::refusals_keep_their_words_and_carry_the_usage`).
- `Flag` (struct, m8/23, `cli/flags.rs`) — one flag: `long`, `short`, `value`, `help`, `hidden` (out of `--help`, still in the man page).
- `FLAGS` (const) — every flag `parse` accepts; `parse` finds a row, then acts on it, so a flag with no row is `UnknownOption` (both ways: `cli::docs_tests::every_flag_row_parses_in_each_spelling_and_every_action_has_a_row`).
- `usage` (fn) — `&[Flag] -> String`, what `--help` prints (replaced the `USAGE` const; same words).
- `man_page` (fn, m8/23, `cli/man.rs`) — `(&[Flag], &[Binding], &[ConfigKeyDoc]) -> String`: the roff page, ASCII only (non-ASCII as `\[uXXXX]`), lines ≤ 80; KEYS per `ContextId` with chords as `[keys]` spells them. Committed as `docs/man/trib.1` (`.TH TRIB 1`; FILES/ENVIRONMENT paths from `OnDiskNames::New.dir()`, `cli::docs_tests::man_page_names_the_config_folder_in_force`), kept equal by `cli::docs_tests::committed_man_page_is_the_generated_one`.
- `ConfigKeyDoc` (struct, m8/23, `cli/config_keys.rs`) — one config key: `section`, `key`, `values`, `default` (TOML literal or `(words)`), `help`.
- `CONFIG_KEYS` (const) — every key the loader reads plus the `keys.<context>` and `theme.<kind>` patterns; equal to `config::load::SCHEMA` + `[keys] suspend_on_ctrl_z` (`cli::docs_tests`), each literal default loads to `Config::default()`.
- `COMMAND` (const, m8/27a) — `"trib"`, the binary's name (`[[bin]] name = "trib"`; package and lib stay `graft`): usage line, `--version` (`version()` → `trib <version>`), stderr prefix, crash text, man NAME/SYNOPSIS. The product's name in prose is `graft_ui`'s `APP_NAME`.
- `ASKPASS_SOCKET_VAR` (re-export of `graft_core::on_disk::ASKPASS_SOCKET_VAR`) — `TRIB_ASKPASS_SOCKET`, the env var git's askpass child is given.
- `from_process` (fn) — parse the real process's arguments; askpass mode is decided first.
- `askpass` (fn) — `(Option<OsString>, &[OsString]) -> Option<Invocation>`; the mode's rule on its own, so it is testable without a process.
- `parse` (fn) — parse an arbitrary argument list.

## event_loop
Internal split (m1-the-graph/71, public paths unchanged): `startup` (terminal
setup, `run`), `headless` (no-tty path, `run_headless`), `running` (the frame
loop, `pump`/`pump_with`), plus the pre-existing `graphics`, `redetect`,
`row_clear` — all private, re-exported from `mod.rs` where still public.
- `input` (mod) — input reader thread and event translation.
- `signals` (mod) — signal watcher thread.
- `Exit` (enum) — why the loop stopped: `Quit`, `HandedBack(KeyCode)` (a `Harness::hand_back` key), `Signal(i32)`, `Fatal(String)`, `StoppedHalfWay { job, what, after: Duration }` (m8/13b review fix; 13d: `wind_down` hands it to `Pool::stop` as the `Boxed` write, which prints its `update::quit::exit_line` once the pool is down and its answer known; `main` exits 0). **A hang-up is `Signal(SIGHUP)`** (13g): the kernel's `SIGHUP`, or a terminal read failing with `EIO` or end of input (`input::hung_up`), whichever comes first; the exit then waits as for `SIGTERM` and dies of `SIGHUP`. A restore the dead terminal refuses, and a stalled-jobs line it cannot print, do not change that exit (`startup::finish_to`, `wind_down`) — only when the failure is the dead terminal's: both guards call `startup::wind_down::excused_by_hangup(exit, error)` = `Signal(SIGHUP)` **and** `input::hung_up(error)` (m8/21b-4); any other restore or print error is still the exit's.
- `waiting_line(what)` (fn, `event_loop::exit_wait`, re-exported) — `<bin>: waiting for <op> to finish (up to 60 s) · send the signal again to stop it half-way`, printed on **any** process exit with writes running (13g, review m8-13cd Minor 6), not only a signalled one; never on `Ending::NextPlace`; `<bin>` is `CARGO_PKG_NAME`, `<op>` sanitised. (13b's `stopped_line` is gone: every stopped write's line is `update::quit::exit_line`, 13d.)
- `exit_wait::Pool` (struct, private) — the pool and both watchers, handed out of `run_repo` so `wind_down` stops them **after** `session.restore()` (13b review fix: no wait longer than `SHUTDOWN_GRACE` before the terminal is given back). `stop(ending: Ending, boxed: Option<&Boxed>)` / `stop_to(stderr, …)` (`Ending` `Signalled | Exited | NextPlace`, 13g): any ending but `NextPlace` → one waiting line per running write (write errors dropped: after a hang-up the words have nowhere to go), `shutdown(SHUTDOWN_GRACE, EXIT_WRITE_WAIT, &again)` with `again` taken **before** the line (`Signalled`: `again_after_one`, so a hang-up's own late-counted `SIGHUP` is not the signal again), then `boxed`'s `exit_line` (always; its `at` from `Shutdown::forced`, `None` if absent; a failed write ignored, as 13b's `main` did) and one `exit_line` per other forced write. Holds `undo: String`, the process keymap's spelling of `app.undo` (ADR-0055), spelled in `run_repo`. `Boxed<'a> { job, what, after }` (struct, `pub(super)`): the stop box's write, from `Exit::StoppedHalfWay`.
- `running::quit_clock::QuitClock` (struct, private) — the quit wait's count: `due(state, now)` starts at first sight of `quit::offer_pending`, gives `since + STOP_OFFER_AFTER`, restarts for another job, drops with the wait; `after(now)` for the exit line. The turn adds `due` to the deadlines it waits on and calls `quit::waited_long` when it passes.
- `Wake` (enum) — something that wakes the loop.
- `Wakes` (trait) — a seam so `pump` can be driven by a scripted list in tests.
- `Wake::Fence` (variant) — the terminal answered a frame's fence (`CSI 0 n`), sent by the mux's fence listener (`startup::answer_fences`) only when `answers_dsr` (m8 G-pace).
- `Waited` (enum) — `Woke(Wake)`, `TimedOut`, `Ended`: what `next_wake_by` got.
- `Wakes::now` (fn, default `Instant::now()`, G-settle) — the loop's one clock (pacer, batch budget, rest); a test source hands out wakes on a virtual timeline (`event_loop/tests/settle.rs` `Timed`).
- `rest::Rest` / `rest::Mover` / `BURST_GAP` (150 ms) / `QUIET_MIN` (50 ms) (`pub(crate)`, G-settle) — when the cursor has rested: `in_burst(now)`, `moved(now, Mover)`, `rest_first(&Wake)` (an input that is not the burst's key: `Rested` is applied before it), `deadline()` (last motion + clamp(2 × widest of the last 4 gaps, 50, 150 ms)), `rested()`. The loop waits on the earlier of `Pacer::deadline` and `Rest::deadline` while `follow_behind`.
- `Wakes::next_wake_by` (fn) — required: block until a wake or `deadline`. The loop waits here while a frame is held behind an unanswered fence or the 16 ms cap. `ChannelWakes` uses `recv_timeout` on its wake channel.
- `pace::Pacer` (`pub(crate)`, m8 G-pace, ADR-0054) — when the next frame may be written: one fence unanswered at most; 500 ms timeout (`FENCE_TIMEOUT`) draws anyway; 3 timeouts with no answer between them stop fencing for the run (any answer, however late, ends the run of timeouts — review m8-02 Major 1), then one frame per 16 ms (`FRAME_CAP`), as for a terminal that never answered DSR. `new`, `fenced`, `may_draw`, `drew`, `answered` (→ how long the fence waited), `deadline`, `owed`, `forget` (after a handover).
- `graphics::Graphics` strip cache (m8 04): `STRIP_BUDGET` (96 MiB across the three layers) with `capacity_for(STRIP_ROWS, graph height, BUFFER_ROWS)` as the per-frame floor (`keep_at_least`); `layers` fetches a strip's backdrop/emphasis (or iTerm2 band) with their `UploadKey`s. Tests: `with_strip_budget`, `strip_budget_report`, `strip_stats`. `STRIPS_KEPT` now bounds only the terminal's image counts.
- `running::Screen` (`pub(super)`) — `{ terminal, graphics }`: where a frame goes, text and images, one synchronized update. `pump_with(screen, wakes, jobs, session, state, outside, tracer)` takes it (review m8-01 Major 2: replaces the `too_many_arguments` allows). `startup::run` split into `listen` (forwarder, trace, fence listener), `outside` and `wind_down`.
- `running` split (review m8-02 Major 3; private, paths kept by re-export): `running.rs` the entry points (`pump`, `pump_with`, `pump_keeping`, `prefetch`, the free at exit); `running/turn.rs` the paced turn (`frames`, `Run`: `turn` → `idle`/`paced`, `take` → `fence_answered`/`terminal_changed` (flushes `Flush::Theme` only when the theme itself changed — a new hint under his palette choice or a forced `ui.theme` owes a frame, not a re-raster; m8/11)/`input`, `apply`, `settle_fences`, `draw`); `running/carry.rs` what the loop carries out itself (`carry_out`, `Carried`, `hands_over`, the six handovers, OSC 52); `running/frame.rs` one frame's bytes (`draw_traced`, `draw_frame`). Loop-level pace tests on a virtual timeline: `event_loop/tests/pace.rs` `Line`.
- Paced turn (`running::turn::Run`): wakes arriving while a frame is held are applied, not drawn; non-joining ones (clicks, resize…) wait until the held frame is out. Before a handover the loop settles owed fences so `$EDITOR` never reads `CSI 0 n`; after it, `forget` and draw at once. Each frame: `SYNC_BEGIN` · text · images · `SYNC_END` · fence (`running::frame::draw_frame`).
- `Wakes::waiting` (fn) — is a wake already here, never blocking; the loop asks before each idle-prefetched strip (m1-the-graph/30h). `ChannelWakes` keeps a wake it saw for `next_wake` and `unhandled_input`.
- `Wakes::joining_waiting` (fn) — required, never blocks. Is a wake queued, and does it `joins_frame`? `pump` then applies it in the same turn and draws once: a turn's wakes share one frame, up to 8 ms of updates; a handover command or a quit ends the turn (m7-power-tools/43). `ChannelWakes` peeks its `seen` wake.
- `joins_frame` (fn) — `(&Wake) -> bool`: job events, keys (press/repeat), wheel scrolls. Not clicks, drags, hover, paste, resize, focus, signals or terminal wakes: those start their own turn.
- `is_job_event` (fn) — `(&Wake) -> bool`: `Started`/`Produced`/`Progress`, or `Core::JobFinished/JobFailed/JobCancelled`; keys, mouse, resize, signals and terminal wakes are not.
- `Wake::label` (fn) — `&'static str`: the variant and the kind inside it (`Produced::Refs`, `Progress::History::Window`), never the payload; for the `frames` startup trace (m7-power-tools/12).
- `Wakes::mark` (fn) — `(&mut self, Phase)`, empty default: `pump` calls it as each phase of a frame ends. A timing seam for the `frames` perf harness (m7-power-tools/04); `ChannelWakes` keeps the empty default, so the product pays nothing.
- `Phase` (enum) — `Drained`, `Updated`, `CarriedOut`, `Drawn`: where the frame for one wake stands, in order.
- Frame trace (tests, m7-power-tools/04): `tests/it/frame_trace.rs` holds `Gate` (the `frames` verdict: asserts p99 ≤ 16 ms, prints p50/p99/max and the keys over), `FrameTrace` (`TRIB_FRAME_TRACE=<file>`: one line per key of its phases; off when unset) and `WriteClock`, and `KeyFeed` / `SentLog` / `Sent` (m7-power-tools/04b): the harness's input thread, sleeping the script's pauses and sending each key into the loop's `Wake` channel as `event_loop/input.rs` `spawn_reader` does; a key's latency runs from its send (`a_key_sent_after_idle_is_received_within_1ms_of_its_send`). `frames` shares one `Wake` channel between the jobs and the keys (`budget_run::open_into`, `pump_until_from`) and reads `TRIB_FRAME_CONTROL` (`a` no idle pause, `b` a core held busy, `c` no idle prefetch) and has `the_machine_alone_after_idle_and_warm`. Driven by `scripts/m7-perf-key.sh`, which exits 1 when any run failed or any `spread` line is not `ok` (`FAIL(<run> failed)`, `MISSING(k of 3)`, `OVER`; checked on canned outputs by `scripts/m7-perf-key-selftest.sh`, 12d). `frames` is split (12d): `frames.rs` (the scroll tests, `script`, `check_scale`, `check_idle_gaps`), `frames/run.rs` (`scroll`, `ready_to_scroll`, `Run`), `paced.rs` (`Paced`, the loop's stamped wakes; `Watching`), `readings.rs` (latencies, trace, startup lines, `waited`, and `unasked_wakes`/`idle_gaps` — m7/12e `idle_gap_wakes`: wakes nothing asked for in each idle pause after `LaidOut`, asserted 0), `sink.rs` (the in-memory terminal), `controls.rs` (`TRIB_FRAME_CONTROL`, control D), `verdict.rs` (`judge`, `print_details`). `frames::after_idle::g_after_idle` (m7/12e): 100k cg, 50k refs, Unicode — laid out to the end (`LaidOut`) and startup settled, 2 s idle, `G`; latency from `G`'s send to the frame showing the last row, asserted ≤ 16 ms in release, `idle_gap_wakes=0` asserted; `TRIB_FRAME_NO_IDLE_ADVANCE` (the "before" build of `scripts/m7-12-final-perf.sh`, `scripts/m7-12-no-idle-advance.patch`) records without asserting. `frames::journal_find` (m7-power-tools/30p-b): `j_with_a_broad_find_holds_the_frame_budget`, J at its caps (`with_reflog_of(10k cg, RECORDS_CAP, LOST_CAP)`: 2 425 rows, 425 `✗`) at 187×50 Unicode, `/step⏎` (2 000 hits) then `n`/`N`/`j`×40; two pumps (`J`, then `/`…) with the stage 2–3 wait between, unmeasured; premises fail the test; `frames: tier=Unicode refs=<rows> pane=JournalFind … hits= lost_rows=`. Driven alone by `scripts/m7-30p-perf.sh`.
- `drain_sink` (tests, m7-power-tools/42; fast tier, root of `it`): `DrainSink`, a bounded pipe drained by a thread at a fixed rate, blocking the writer when full; keeps counts and offsets (`written`, `drained_at`, `quiet_for`) and `KittyTransmit`s found in the stream (`kitty_transmits`), never the bytes. **m8 G-pace:** `answering_fences(answer)` — the drain thread calls `answer` once per `CSI 5 n` when it has drained past it (split-safe `FenceScan`); `fences_written`. Test `answers_each_fence_once_drained_past`.
- `held_key` (tests, m7-power-tools/42; heavy, root of `it`, outside the `frames::` filter): held `j` ×188 at 16 ms on `TeamSpec::MONOREPO` at 187×50 into a `DrainSink` (16 KiB; Unicode 500 KB/s, Pixel 40 MB/s), scenario B adds `J`; `SUMMARY held_key` / `VERDICT held_key` lines; red until 43 (Unicode) and 46 (Pixel). Pixel SUMMARY carries `fold_changes_held=unread` and `strip_transmits_held` is printed `unjudged` (46c: the harness cannot read the drawn fold). Driven by `scripts/fix-scroll-held-key.sh`; also by `scripts/47-scroll-acceptance.sh` (m7-power-tools/47), which runs the raw pair apart and judges it against the `M7/46-kitty-no-deflate` known-gap list. **m8 G-pace:** `answers_dsr = true` and the drain answers fences (`Wake::Fence` on the harness channel), so it measures the paced loop; `Paced` implements `next_wake_by` and drops a look's stamp when a fence answer releases a frame; new `held_j_stops_on_release_{unicode_40mb,unicode_free,pixel_free,pixel_deflate_free}` (`FREE_RATE` = `drain_sink::FREE`: drained as written, no tick). Run by `scripts/m8-pace.sh` → `docs/perf/m8-pace.txt`.
- `ghostty_shot` (tests, m7-power-tools/47; heavy): `fix_scroll_ghostty_png` writes `docs/shots/m7/fix-scroll-ghostty.png` — held `j` ×188 on `TeamSpec::MONOREPO` at 187×50, every strip on screen composited through `StripCache::composited` (ADR-0053) with `kitty_deflate` on, stacked, cropped to the graph column and the cursor ±10 rows inside the pane, over `#1E1E2E`; checked by `capture::check_bytes`. Prints `SUMMARY ghostty_png … cursor_on_screen=`. Run by `scripts/47-scroll-acceptance.sh` step 5.
- `event_loop::tests::pixel_scroll` (unit tests, m7-power-tools/46, 46c): two-layer pixel strips through `draw`; `a_capped_graph_resends_strips_only_when_the_fold_changes` guards ADR-0053 on a graph wider than the cap (fixtures `view::strips::tests::{capped, fold_plan}`).
- `frames::sink::Sink::draining` (tests, 42): a `Sink` forwarding every write to a `DrainSink`, keeping no byte (`count` panics on it); `sent`/`write_spent` as for `Sink::timed`.
- `frames::run::ready_to_scroll_at` (tests, 42): `ready_to_scroll` at a size passed in, never `TRIB_REAL_SIZE`; `ready_to_scroll` delegates to the same body with the env's value.
- `ChannelWakes` (struct) — the real wake source: one channel, two feeder threads.
- `ChannelWakes::new` (fn) — build the channel and the sender its producers clone.
- `ChannelWakes::unhandled_input` (fn) — the terminal input queued after the wake that ended the loop, in order; the rest is dropped with the loop.
- `run` (fn) — run the event loop for a session, start to `Exit`; may be called again by a wrapper (the gallery): stdin has one reader per process, and the keys this run never handled go to the next.
- `pump` (fn) — drive the loop; testable independent of a real terminal. Draws Kitty strips at the pixel tier as two layers per strip, the layout-only backdrop and the selection's emphasis over it (ADR-0053, 46), zlib-deflated when the probe found `kitty_deflate`; drains image replies each wake (a refused deflated upload turns deflate off), frees every upload of both layers on exit (m1-the-graph/24c). Theme/cell-size changes flush every cache layer; a resize flushes nothing and retransmits nothing — both layers re-place, only changed keys rasterise (25). iTerm2 protocol: one composited band per strip, re-sent per frame, encoded once per paint (25, 46). Idle prefetch draws backdrops only.

## event_loop::outside
Handing the terminal to another program and taking it back
(m3-working-tree-undo-and-stash/05). PROMPT.md principle 8: a wrong restore
breaks the user's terminal after Graft is gone, so every path here restores
newest-first and re-enters the same modes, or ends the loop saying it could
not.
- `Outside` (struct) — what the loop needs to reach outside itself: `app_dir` (`RepoStore::app_dir`), an optional `Handover`, and (m8/18e, `pub(crate)`) `first_launch: Option<&FirstLaunch>`, the switch `drive` lends. `None` in the headless and test loops, which have no terminal to give away.
- `Handover` (struct) — `session`, `mux`, `git_dir` (where `git var GIT_EDITOR` is asked), `app_dir` (where the message file goes), `workdir` (the working tree's root: the directory `$EDITOR` is started in, and what a porcelain path is joined onto — m3/05 review M2); `edit_message` (write the message to `graft/COMMIT_EDITMSG`, run `$EDITOR`, read back what was saved), `edit_file` (open a path, on a line where the editor takes one), `suspend` (`SIGTSTP` and back).
- `Handover::edit_conflict` (fn) — `(&self, path) -> Result<(), String>`: `$EDITOR` on a conflicted file, opened at its first `<<<<<<< ` (`graft_git::history_reads::first_marker_line`, `None` when the file is already clean). The read lives here, not in `update`, which is pure (m5-history-and-ghost-preview/09).
- `edit_message_in` (fn) — `(app_dir, message, open: FnOnce(&Path) -> Result<(), String>) -> Result<String, String>`: `Handover::edit_message`'s write / open / read-back without the terminal — the message written to `<git_dir>/graft/COMMIT_EDITMSG`, `open` run on it, the file read back exactly. The loop passes `Handover::open`; `tests/it/finish.rs` passes the fixture editor, so the test goes through the real call (m6-conflicts/36).
- `Handover::mergetool` (fn) — `(&self, path) -> Result<(), String>`: `git mergetool --no-prompt -- <path>` in `workdir`, with the terminal handed over. **Every** exit is inside `run_outside_terminal`: a non-zero tool, a tool that cannot start, a tool that is killed, and a panic all go back through the same `Reenter` guard. Nothing runs the tool outside that call, which is the property `event_loop::outside::tests` asserts — on the raw terminal transcript (teardown, then the identical setup again), not on a call site.
- `Handover::continue_in_git` (fn) — `(&self, InProgress) -> Result<ExitStatus, String>`: `git <kind> --continue` in `workdir`, terminal handed over as for the mergetool; exit status returned as is (07c).
- `continue_command` (fn) — `(InProgress, workdir) -> Command`: that invocation.
- `mergetool_command` (fn) — `(path, workdir) -> Command`: the invocation, apart from the hand-over so a test can read it. `--` before the path: a file named `--tool` is a file, not a flag.
- `run_outside_terminal` (fn) — pauses the input mux **first** so the reader thread stops consuming stdin (`InputMux::hold` only detaches the listener — the thread keeps reading, and `$EDITOR` would race Graft for keystrokes), then suspends the session, runs the closure, and re-enters both. A failure to re-enter is `OutsideError::Reenter`, which ends the loop rather than drawing into a terminal Graft no longer owns.
- `OutsideError` (enum) — `Restore` (could not give the terminal away; nothing ran) and `Reenter` (could not take it back).
- `editor_command` (fn) — `(git_editor, path, line, workdir) -> Command`. `path` is made absolute against `workdir`, which also becomes the command's directory (m3/05 review M2). A `GIT_EDITOR` carrying shell syntax (`'"$`\;&|<>()*?[`) is run as `sh -c '<editor> "$@"' <editor> <args>`, the way git runs it — a quoted path with a space in it used to be split mid-word (m3/05 review M5). The line argument goes per editor family: `+N` (vim/nvim/vi/nano/emacs/emacsclient/micro/hx/helix), `--goto path:N` (code/code-insiders/codium/cursor/windsurf), `path:N` (zed/zeditor), and no line argument for anything unrecognised — a wrong flag would open a file named after it.
- `stop_this_process` (fn) — `SIGTSTP` to Graft's own process group, so the shell's job control takes over exactly as it would for any other program.

## event_loop::signals
- `WATCHED` (const) — signals watched: SIGINT, SIGTERM, SIGHUP.
- `watch` (fn) — start the signal-watching thread; returns a `Watch` (`close()`; `again() -> Again`, a `graft_git::run::Stop` that is true once another watched signal arrives — the "send the signal again" of the exit wait; `again_after_one()` (13g), the same counting the loop's own ending as one signal even if not yet counted). The thread counts every signal and keeps watching after the loop has gone.
- `watch_resizes` (fn) — watch `SIGWINCH`, waking the loop with `Event::Resize` at the new size.
- `reraise` (fn) — restore default disposition and re-raise a signal.

## event_loop::input
- `spawn_reader` (fn) — forward the input mux's `Inputs` into the loop's wakes, on a thread of its own.
- `hung_up(&io::Error)` (fn, `pub(super)`, 13g) — a terminal read failed because the terminal closed: `UnexpectedEof` (the mux's read of 0) or `EIO`. The repository loop (`running/turn.rs`) and the places loop end on it as `Exit::Signal(SIGHUP)`; headless never reads a terminal and is unchanged.
- `startup::put_away` (fn, private, 13g) — shows the cursor ratatui hid before its `Terminal` drops, handling the failure (`RowClear::cursor_given_up`): ratatui's own `Drop` `eprintln!`s it, which panicked on a hung-up pty (exit 101, ratatui-core 0.1.2 `terminal.rs:483`). The places screen's plain `Terminal` still drops the old way (13g NOTES).
- `running::pump_with` gains `tracer: &mut trace::Tracer` (m8 G-trace; `Tracer::off()` in tests); `draw_traced` wraps every `draw` in `frames`, and puts a trace failure on the status row as `Status::sticky("trace: <reason>")`. Tests: `event_loop::tests::perf_trace`.

## jobs
Internal split (m1-the-graph/71, public paths unchanged): `types` (`Outcome`,
`Work`, `Cancel`, `JobEvents`, `Delivery`, `Dispatch`), `pool` (`Shutdown`; `forced`: `Forced`,
plus the private queue/worker bookkeeping), `kinds` (the work behind each
command), `runner` (`JobRunner`) — re-exported from `mod.rs`.
- `runner/` (m8/35, move-only split of `runner.rs`; `jobs::JobRunner` unchanged): `runner/mod.rs` the struct, `start`/config, `cancel`, `force_stop`, `shutdown`; `runner/queue.rs` `submit`, `announce`, `queue_job`, `queue_as`, `dispatch_detail`/`dispatch_history` and the radar probes; `runner/reads.rs` open, refs, working tree, conflict, finish, commit message, rewrite, row tags; `runner/operations.rs` resume, operate, clone, credential, history, undo, preview, journal, tour, recovery; `runner/lists.rs` stash, switch list, palette, index, search, remotes, leaves, folds, map, rows; `runner/dispatch.rs` `impl Dispatch` and `not_a_job`. Job fns are `pub(super)` so `dispatch.rs` reaches them.
- `map` (private mod) — the Map view's job (m2-details-and-diffs/18): `build_map(repos, req, progress, stop) -> Result<Arc<MapData>, String>` runs `crate::map::build_map` and stringifies its error; `JobRunner::build_map` dispatches `Command::BuildMap` under `update`'s own id (`queue_as`, mirroring the detail pane's jobs), reporting `Progress::Map { done, total }` per finished row.
- `radar` (`pub(crate)` mod) — the Mistake Radar's context job (m2-details-and-diffs/13, 13b).
  - `RadarContext` (struct) — the default branch's first-parent ancestry as a `HandleSet` over its own `Arc<CommitTable>` (private; ADR-0026's rule, housekeeping/02: a `HashSet<Oid>` here was 43 of the heavy tier's 76 B/commit heap slope), plus `previous_default_tips: Vec<Oid>` (its reflog history, newest first, capped). `on_default_chain(&Oid) -> bool` / `chain_len() -> usize` / `of_ids(chain, previous_tips) -> RadarContext` (a context over ids, with a table of its own) (fn).
  - `run_evil_probe` (fn) — `(repo, merge, stop) -> EvilVerdict`; the work behind `Command::EvilProbe`, dispatched by `JobRunner::dispatch_detail`.
  - `run_force_push_probe` (fn) — `(repo, branch, current, stop) -> ForcePushVerdict`; the work behind `Command::ForcePushProbe`. Resolves the branch's remote-tracking ref through `RepoHandle::upstream` (config `update` cannot read), then `probe_force_push`; no upstream → `NotCheckable` (m2-details-and-diffs/15).
  - `run_radar_context` (fn) — `(repo, table, default_tip, default_ref, stop) -> Result<Arc<RadarContext>, String>`; the work behind `Command::RadarContext`, dispatched by `JobRunner::dispatch_detail`.
  - `run_duplicates_probe` (fn) — `(repo, window, stop) -> Result<Arc<Duplicates>, String>`; the work behind `Command::DuplicatesProbe`, wrapping `graft_ops::mistake_radar::probes::probe_duplicates` (m2-details-and-diffs/16).
- `preview` (private mod) — Ghost Preview's reading half (m5/08): `run_preview(repos, &PreviewRequest, clock, cancel) -> Result<Arc<Planned>, PreviewFailure>` opens `git`, reads the facts the operation names (`Operation::facts_scope`) and calls `graft_ops::preview::plan_as_run` (m7/05b: the op completed as `y` will run it, returned as `Planned::runs`, and its picture), or plans the time machine from the journal's own entries (`plan_revert_to`); then reads every real commit the picture points at that is not already on screen, as **anchors**, capped at `PREVIEW_ANCHOR_CAP` — past the cap the answer is a sentence the box prints, not a shorter picture. `JobRunner::plan_preview` dispatches `Command::PlanPreview` under `update`'s own id; `JobRunner::run_revert_to` dispatches `Command::RevertTo` as a write, with `Progress::Operating`.
- `undo::JournalRead::live` (field) — `Vec<EntryId>`: the entries recovery left `Pending` **on purpose**, because git is still part-way through that very operation (`graft_ops::apply::Recovered::live`, m5-history-and-ghost-preview/14). A live entry and an interrupted one are the same bytes on disk — a `Pending` line with no outcome — so the difference is decided here, at **read** time, against git's own state; nothing on disk is migrated and an older journal reads correctly. Carried across reads unlike `Recovered::closed`, because it is a fact about git *now* that every read re-establishes.
- `undo::JournalRead::interrupted_edits` (field, 30c) — `Vec<(EntryId, graft_ops::EditsAt)>`: `Recovered::set_aside` of this read; non-empty only on the read whose recovery closed the entry.
- `undo::journal_entries` (fn) — the journal's entries for a caller that only reads them; the time machine plans from the very entries `revert_to` will run against (m5/08).
- `undo::RevertedTo` (enum) — `Done { description, total, steps, replaced, presses }` (the steps' `Applied::replaced` unioned; `presses` = total − the first step that replaced anything, since `U` walks the jump back from its last step), `Drift { description, total, drift, target }` (m6-conflicts/77: also reached from `PartWay { done: [], Drift }`, which is how `revert_to` reports step 0's drift — before 77 that fell to `Failed`), `Failed { description, done, total, error }`. `total` is read from the plan **before** anything runs, because `UndoError::PartWay` carries only `done` and "2 of 3" is the sentence the user needs (m5/08).
- `undo::run_revert_to` (fn) — `(repos, &EntryId, force, clock, cancel) -> RevertedTo` (m5/08).
- `undo::Undone::Nothing` (variant) — `{ reason: String, redo: Option<String> }` (a tuple before m6-conflicts/77): `redo` is the waiting redo's root operation description (`next_redo` then `root_operation`, from the journal `u` just read), set only for `u`'s `NothingToUndo`.
- `undo::Undone::OperationLive` (variant) — `{ kind: InProgress }`: `graft_ops`' undo gate refused (`UndoError::Nothing(NotInvertible::OperationLive)`) because the step is from before the live operation (m6-conflicts/57). Its own variant rather than `Nothing`, which the app words as a quiet `Done`: this is a refusal, worded `Sticky` by `update::undo::undone`.
- `undo::Undone::Cancelled` (variant) — `{ clean: bool, why: String, refresh: Refresh }`: `Esc` stopped the undo and `graft_ops::restore::execute` rolled back what it had applied. Its own variant rather than a `Failed`, because a cancel is not a failure; `clean` says whether the roll-back finished, and `why` is `restore`'s own sentence. `refresh` is never optional (`Everything`): usually the journal gained an `Aborted` entry, but a stop before any step (m7/01b, gap :495) may have written none — the re-read is then spare and harmless (m5-history-and-ghost-preview/06 scope item 5, built in /08).
- `conflict` (crate-visible mod) — the resolver's jobs (m6-conflicts/04, /05), thin over `graft_ops::conflict`. `open(store, path, stop) -> Result<Opening, OpenFailure>` (`Opening::Unmerged(Arc<OpenedFile>)` / `Opening::Done(Arc<DoneFile>, StartOver)`, routed on that status: unmerged → `open_conflict`, else `open_done`; `OpenFailure.done: Option<StartOver>` marks the done route, m6-conflicts/25). On the done route `StartOver::{Can { kind: RestartKind }, Cannot { why }}` is `graft_ops::apply::restart_gate`'s answer (`kind` from its `RestartPlan`, m6-conflicts/58), asked **before** the file is read so a failed read still carries it; a gate that could not be read is `Cannot { why: "could not check: …" }` (m6-conflicts/28) reads the working-tree status **itself**, fresh, rather than taking `AppState`'s copy: the re-read after `e`/`M` has to be able to answer "no longer unmerged", which a cached status cannot. `take(store, path, file, block, side) -> Result<ConflictFile, TakeFailure>` passes the document the user is looking at, never a re-read (re-reading drops every `Resolved` region and re-indexes the blocks, so a retake would land on the wrong one). Both failures are **typed** — `OpenFailure { words, not_unmerged }`, `TakeFailure { words, reread }` — so `update` never parses a sentence to decide what to do next. `OpenedFile.start_over: Option<StartOver>` is the same gate's answer for a whole-file shape (`file: None`), `None` for a document (m6-conflicts/30). `OpenedFile.unreadable: Option<String>` (m7/01b, gap :572): the file is there but could not be read — `can't read <path>: <reason>`, landed as the resolver's problem notice by `update::resolver::answers`; a path that is not there is `None` both ways. `OpenedFile.on_disk: Option<OnDisk>` (m6-conflicts/05; capped by the m6-05 review, Major 4) carries a whole-file shape's card (`file: None`) a glimpse of the working-tree file — read directly off disk (`worktree_glimpse`), never through `graft_git::conflict::read`, which parses markers this shape does not have; `None` for a shape with nothing on disk (`BothDeleted`, or a side that removed it) and always `None` when `file` is `Some`. `OnDisk { size, head: Arc<[Vec<u8>]>, cut }`; `glimpse(read, size, binary) -> io::Result<OnDisk>` reads at most `CARD_LINES` (500) lines and `CARD_BYTES` (64 KiB), dropping a fragment the byte cap cut through, and nothing at all of a binary file. `Arc` because every event clones the state that holds it. `load_base(store, path, file) -> Result<(ConflictFile, usize), (String, bool)>` runs `B`'s `graft_ops::conflict::base_for`. `conflict_set(store, stop) -> Result<ConflictSet, String>` reads the facts and `operation_conflicts` for the file list (m6-conflicts/13).
- `detail` (crate-visible mod) — the detail pane's reads (`detail.rs`: `read_files`, `read_file_diff`, `read_contained`, `body_of`), with three children split in m7-power-tools/34, re-exported at `jobs::detail::*`: `detail/blame.rs` the blame job (`read_blame`, `blame_failure`, `path_bytes`; tests `detail/blame_tests.rs`); `detail/working_tree.rs` the changes pane's list (`assemble_working_tree`, `UnmergedShapes`, `shapes_of`, `sections_of`, `diff_spec_for`); `detail/assemble.rs` the pure assembly (`assemble_files`, `assemble_diff`, and `push_tree`/`status_view`, shared with `working_tree`). Items: `assemble_files(&FileList, Option<&[u8]>) -> FileListView` (path tree, `✉` row when the body has a non-trailer line) and `assemble_diff(&FileDiff, word_tint) -> DiffView` (sanitise, pairs, word spans, syntax, CRLF → `␍`), both pure. `assemble_working_tree(staged, unstaged, &UnmergedShapes) -> (FileListView, FileList)` builds the changes pane's sectioned list: `CONFLICTS` first, then `STAGED`, `UNSTAGED`, `UNTRACKED`, each over its own files as a directory tree. A file that is both staged and unstaged appears **twice**, once per section, because it is two different changes; a **conflicted** file appears **once**, in `CONFLICTS`, and is excluded from the other sections **by path** — git reports an unmerged path in the worktree diff as an ordinary modification too, so filtering on `FileStatus::Unmerged` alone leaves it listed a second time as though it were a change to stage (m5-history-and-ghost-preview/09). A `CONFLICTS` row says the **shape** of its conflict (`both modified`, `deleted by them`) rather than the word `unmerged`: the diff does not carry git's `XY` pair, so `read_files` reads the working-tree status alongside the two diffs and hands `shapes_of` it as `UnmergedShapes` (`HashMap<Vec<u8>, UnmergedHow>`, keyed by the same raw bytes the diff carries). A status that cannot be read costs those rows their wording and nothing else, so it is not the job's failure (m6-conflicts/04, KNOWN_GAPS row 472). `diff_spec_for(spec, section) -> DiffSpec` picks the read one row shows: `STAGED` → index against `HEAD`, `CONFLICTS` → **worktree against `HEAD`** (m5/21 — against the *index* there is no two-way answer, because the index holds three stages, so git replies with a combined `diff --cc` that the unified parser skips and the body drew 0 hunks), everything else → worktree against the index. `read_files`/`read_file_diff`/`read_contained` run them on a worker with the job's `Cancel` as the git `Stop`. The runner queues them under `update`'s ids (`queue_as`). **Submodules** (m7-power-tools/13): `assemble_working_tree` is now test-only and wraps `assemble_changes(staged, unstaged, &UnmergedShapes, &SubmoduleRows)`; `SubmoduleRows` (`working_tree.rs`) maps raw path → `SubmoduleView` per section (`of(section)`), and `push_tree` pushes those rows after the section's tree, at base depth, full path as name (a commit's list passes none, so its gitlinks keep their letter). `read_files` builds them in `submodule_rows`: staged pointer from the staged diff; an unstaged entry with none of C/M/U is dropped (a clean submodule has no row); one with facts that `diff-files` did not list (untracked inside only) is added; `graft_git::status::submodules_not_set_up` adds `not set up` rows — its failure fails the job. `pointer_of(&FileDiff)` reads `Subproject commit` lines (`-dirty` stripped); `short_id(Oid)` the 7-char id. `read_file_diff` on a gitlink (`submodule_diff`) sets `FileStatusView::Submodule` with the log from `graft_ops::submodule::submodule_log` (pulled in, then rewound); exit 128 → `NotHere`, other errors fail the job.
- `operate` (private mod) — the changes pane's write (m3-working-tree-undo-and-stash/04): `run_operation(repos: &RepoStore, op: &graft_ops::operation::Operation, clock: session::Clock, cancel: &Cancel, hooks: Hooks) -> Operated` (`Hooks` = the progress sink, the job's `AskPass` and the helper path or the reason there is none, m4-remotes/06) opens the system git and the undo journal, builds a `graft_ops::apply::Context` and calls `graft_ops::apply::apply` — one of the two places `app` performs a write (`jobs::undo` is the other, for `u`/`U`); the seam rule (`tests/it/seam_rule.rs`) forbids any other module here from building a write `GitInvocation` or touching the journal. `run_resume(repos, what: graft_ops::apply::Resume, cancel) -> Operated` is the same shape for `--continue`/`--skip`/`--abort`, over `graft_ops::apply::resume`; both go through one private `in_context` so the journal is opened and the `Context` built exactly once, in one place (m5-history-and-ghost-preview/09). Remote operations get `RemoteHooks` (progress into `Progress::RemoteProgress`, this binary as the askpass helper, `jobs::askpass::ChannelAskPass` answering git's questions from the user's prompt; no helper → the reason on the progress line and git asks nothing) (m4-remotes/05, /06).
  - `Operated` (enum) — `Done(Applied)` (clean); `Incomplete { applied: Applied, error: String }` (the write happened but part of its after-state — `ApplyError::AfterState`'s `unread` — could not be read back; still a completed write, `update::operate::operated` still refreshes); `Conflicted { description: String, kind: graft_git::state::InProgress, paths: Vec<PathBuf>, stalled: Option<String>, entry: graft_core::journal::EntryId }` (git stopped part-way through a history operation and left its own state live — **not a failure**: the entry is journaled `Pending`, and the user is in the operation until it is finished or aborted. Carries both `ApplyError::Conflicted` and `ApplyError::ResumeFailed`; `stalled` is git's own line for the second, where the state is live but nothing is unmerged. m5-history-and-ghost-preview/07 seam); `Failed { error: String, refresh: Option<Refresh>, remote: Option<RemoteFailure>, refused: Option<graft_ops::operation::Refusal>, edits: Option<graft_ops::EditsAt> }` (`edits`: where `graft_ops` says a failed set-aside left the edits — `ApplyError::edits_at()`, no stash-list read of the app's own; m6-conflicts/84, was `edits_in`, 74) (`remote` is `ApplyError::Remote`'s classified failure, m4-remotes/05; `refused` is `ApplyError::Refused`'s own value, carried rather than stringified so `update::operate` can put a **key** beside the two history refusals — `Moved` → `<key> again`, `DirtyTree` → `S stash N changed files`, m5-history-and-ghost-preview/08) — `refresh` is `Some` whenever git may have run part-way (`ApplyError::Git` journals the entry `Aborted` after some of the plan applied), `None` only where `ApplyError` itself says nothing was written (`Refused`, `Facts`, `Snapshot`). `StagingNotKept { applied: Applied, paths: Vec<PathBuf> }` (30b) — the write ran and was read back, the edits are back, but git could not reinstate the staging of `paths`: `answer` maps `ApplyError::AfterState` whose `unread` is exactly `[AfterState::StagingNotKept]` here (anything else stays `Incomplete`); everything a `Done` does, plus a sticky line. `Clashed { entry: EntryId, clash: graft_ops::clash::Clash }` (m6-conflicts/72b) — a stash put back that clashed, `ApplyError::Clashed`: entry `Committed`, git kept the stash; `update::clash::clashed` lands it.
  - `Operated::Conflicted::rewrite` (field, 07b) — a rebase stop's `RewriteStop` read on the job thread (`read_rewrite_stop`); `None` for other kinds. `working_tree::WorkingTree::rewrite` (field) is the same read on every working-tree read while a rebase is live (the relaunch's).
- `askpass` (private mod) — the job side of git's credential question (m4-remotes/06): `Waiting` is the runner's registry of jobs blocked on an answer (`answer(JobId, Option<Secret>) -> bool`; `false` means nobody was waiting and the secret was dropped and zeroed). `ChannelAskPass` implements `graft_git::run::askpass::AskPass`: it registers its channel **before** announcing `Progress::CredentialNeeded`, then waits in 100 ms steps so the job's `Cancel` unblocks it; an already-cancelled job never asks. `pulse_seed() -> bool` draws the masked field's starting pulse bit (`getrandom`, falling back to `clock_parity()`, the parity of the microseconds since the epoch — a failed draw only changes which bit is used, so it is not an error to report. The old nanosecond bit was the constant `0` on macOS's microsecond-granular clock, i.e. a constant seed: m5-04 review Major 2), fresh per prompt: a fixed start would leave the drawn row resting at the parity of the password's length (m5-history-and-ghost-preview/04; m5-01 review Major 2).
- `remote` (`pub(crate)` mod) — `RemoteFacts { remotes, head_branch, head_upstream }` and `read_remotes(repos, cancel) -> Result<Arc<RemoteFacts>, String>`: the remote facts behind `f`/`p`/`P` (m4-remotes/05). `read_leaves(repos, sent, expected, cancel) -> Result<Leaves, String>`: `commits_between` capped at 3 plus `count_between` (m7-power-tools/16b).
- `fold_count` (`pub(crate)` mod, m8/17b) — `count_folded(repos, merge, cancel) -> Result<usize, String>`: `read_commit` for the merge's first parent, then `count_between(M^1, M) − 1` — one `rev-list --count` covering every later parent, so an octopus's overlapping sides count once. A shallow-boundary merge (no parents read) is an error.
- `stash::read_index_state` (fn, private mod) — `(repos, cancel) -> Result<IndexState, String>`: `graft_ops::snapshot::index_state` in the journal's scratch dir (m6-conflicts/82); `JobRunner::read_index_state` dispatches `Command::ReadIndexState { id }` (`Task::ReadIndexState`), answered as `Product::IndexState`, routed to `update::clash::index_answered`.
- `switch_list` (private mod) — `read_switch_list(repos, cancel) -> Result<Arc<SwitchList>, String>`: `graft_ops::switch_list` off the loop (m7-power-tools/10). Dispatched by `runner::read_switch_list` for `Command::ReadSwitchList`.
- `stash` (private mod) — the stash list read behind `S` (m3-working-tree-undo-and-stash/06): `read_stashes(repos, cancel) -> Result<Arc<Vec<StashEntry>>, String>` runs `graft_git::stash::list` and stringifies its error; `JobRunner::read_stashes` dispatches `Command::ReadStashes` under the id `update` chose (`queue_as`, like the detail pane's jobs).
- `MAX_WORKERS` (const) — worker thread cap (PROMPT.md §7.1).
- `SHUTDOWN_GRACE` (const) — `graft_git::run::STOP_WITHIN` + 500 ms (4.55 s): how long shutdown waits for stoppable jobs after cancelling them, covering a git child's SIGTERM → SIGKILL → reap (m8-13b). Paid only when a child is slow to stop. Still the stoppable grace; a force-stopped write is reaped within it too.
- `EXIT_WRITE_WAIT` (const, 13b review fix) — 60 s: how long shutdown waits for a `Stoppable::No` write before stopping it half-way.
- `FORCED_WAIT` (const, 13c item 11 / 13d) — `SHUTDOWN_GRACE` + 3 s: how long shutdown waits for a **forced** write's answer (where its stop left git, read after its git is stopped). Not measured as a bound; the 13d walk took 54 ms (merge) and 13 ms (rebase). Past it the write is abandoned and its line is the `None` one.
- `Outcome` (enum) — how a job ended, as the job itself sees it. `Answered(Product)` (m8/18b): delivered as `Produced` **even when the job was cancelled** (every other product of a cancelled job becomes `JobCancelled`) — a stopped clone still says what it left behind, and (m8/21c B+1) a stopped fetch, pull or push what its fetch moved.
- `operate::outcome(Operated) -> Outcome` (`pub(super)`, m8/21c B+1) — the operate job's outcome: `Answered` for `Operated::Failed { remote: Some(RemoteFailure::Cancelled), .. }` (a remote write a stop ended after git ran: its words reach `update::remote::remote_failed` instead of `cancelled`'s bare line), `Produced` for every other answer.
- `clone` (mod, `pub(crate)`, m8/18b) — the empty state's clone job: `CloneRequest { url, into }`, `CloneAnswer` (`Done { path, notice: Option<String> }` — notice `cloned · <graft_ops::apply::askpass_words>` when the askpass helper refused another user's program or its socket could not be cleaned up (m8/20b B6, `askpass_notice`); `Ended { words }` — `CloneError::explain`, sanitised). `JobRunner::clone_repo(CloneRequest) -> JobId` queues it (no repository read; askpass through the pool as an operation's).
- `operate::job_hooks(sink, id, waiting, &Cancel, helper) -> Hooks` / `askpass_helper(Option<&PathBuf>) -> Result<PathBuf, String>` / `remote_hooks(Hooks) -> Option<RemoteHooks>` (`pub(super)`, m8/18b) — the one progress-and-askpass hooks builder, shared by `JobRunner::operate` and `clone_repo`; the remote-or-not gate stays in `run_operation`.
- `Work` (type) — a boxed, cancellable unit of job work.
- `Cancel` (struct) — cooperative cancellation flag checked by a job's own code. Since 13d it also carries a slot for where a stopped write left git: `left_at(Option<StoppedAt>)` (`pub(super)`, set by `operate::in_context` from `apply`'s answer, only when `Some`) and `stopped_at()` (`pub(super)`, read by `Live::finish` for a forced job). `run_job`'s event mapping is unchanged: a cancelled job is still `JobCancelled`; the answer travels beside it.
- `Cancel::is_cancelled` (fn) — has cancellation been requested?
- `JobEvents` (trait) — how a worker reports job results back.
- `Delivery` (enum) — what became of a delivered event.
- `Dispatch` (trait) — hands a command off with nothing a caller waits on.
- `Shutdown` (struct, 13b review fix) — what happened when the runner was asked to stop: `stalled` (jobs still running at the grace), `forced: Vec<Forced>` (every write a stop was forced on — the stop box's `y` or shutdown's own — in id order, with its answer, 13d); `Shutdown::CLEAN`.
- `Forced` (struct, 13c item 11 / 13d / 13f, in `jobs/forced.rs`) — `{ job: JobId, what: String, at: Option<StoppedAt>, started: bool }`: a forced write and where its stop left git; `at` `None` when it did not answer within `FORCED_WAIT`, never ran (forced while queued: answered at once, `started: false`, line `<bin>: <op> did not start · nothing changed`), or said nothing. A forced undo jump (`jobs::undo`) never sets `Cancel::left_at`, so its line is always the `None` one. This is decided (13e R4): the line claims nothing false, and a specific line would need undo's roll-back answer carried through `Cancel`.
- `JobRunner` (struct) — owns the worker pool and hands work to it.
- `JobRunner::start` (fn) — start a pool with a given worker count.
- `jobs::palette` (mod, `pub(crate)`, m8-polish-and-ship/12) — `⌃P`'s finds off the loop: `read_branches(repos, cancel)` (the picker's `switch_refs`, read once per open; `Command::ReadPaletteBranches(id)` → `Product::PaletteBranches(Result<Arc<SwitchRefs>, String>)`); `match_names(&NamesRequest, stop) -> Option<NameHits>` (`NamesRequest { query, branches, tags }`; `NameHits { branches, tags }`, each `Option<Ranked<RefHit>>` — `None` = that source not carried, still `reading…`; `Ranked { top (best `TOP`=50), total }`, `RefHit { name, tip, remote }`; branches by `fuzzy_score` tier, local before remote, newer tip, as `filter_switch_rows`; tags by tier then name; `stop` polled every 4096 names → `None`; `Command::PaletteNames(id, req)` → `Product::PaletteNames(Arc<NameHits>)` or `Cancelled`).
- `jobs::finish` (mod, m6-conflicts/19) — `summary(repos)` wraps `graft_ops::finish::finish_summary`; `staged_diff(repos, path, stop)` is `path`'s index-vs-HEAD diff (an empty view when it has none: staged but equal to `HEAD`, as a kept-yours file is — not an error, m6-conflicts/36), assembled like the detail pane's.
- `JobRunner::with_default_size` (fn) — start a pool sized for this machine.
- `JobRunner::worker_count` (fn) — how many worker threads this pool is running.
- `JobRunner::submit` (fn) — queue work; returns immediately with a `JobId`.
- `JobRunner::cancel` (fn) — request cancellation of a submitted job: sets the token, then drops the job's row-request sender. The history walk parks in `recv` between requests and cannot see its token from there, so closing that channel is what wakes it and gives the worker back; token first, so a walk woken mid-chunk finds `stop()` already true (m5-history-and-ghost-preview/05, moved here from `Command::CancelJob`'s dispatch arm).
- `JobRunner::shutdown` (fn) — `(self, grace, writes, force: &dyn graft_git::run::Stop) -> Shutdown`: cancel every job but the unstoppable ones and drop the queue (a queued write never starts), wait up to `writes` for the unstoppable ones — ending early when `force` says so — then force-stop any still running, then wait up to `FORCED_WAIT` for every forced write's answer (`Shutdown::forced`), then up to what is left of `grace` for everything. **No path waits without a bound** (13b review fix). `shutdown_answered_within(self, grace, writes, answers, force)` (fn, `pub(crate)`, 13d): the same with the answers' bound given — the seam the forced-wait test uses.
- `JobRunner::force_stop` (fn, 13b review fix) — `(id)`: lift the job's unstoppable mark and cancel it (its git stopped as graft_git::run stops a write: hook-first for a local write, the group stop for a rebase or a remote op; reaped within STOP_WITHIN). Reached by `Command::ForceStop`, which only the stop box's `y` emits; shutdown's bound or second signal does the same inside the pool. Nothing else lifts the mark.
- `JobRunner::writes_running` (fn) — descriptions of the unstoppable writes still running, for the exit wait's lines.
- `JobRunner::unstoppable` (fn, m8-13b) — `(id, what)`: mark a job `Stoppable::No`, with its description; `cancel` and shutdown's `cancel_all` skip it. Reached by `Command::Unstoppable(id, what)`, which `update::remote::launched` returns for a `Stoppable::No` write and every launch site queues ahead of the job's own command.
- `JobRunner::with_clock` (fn) — `(self, session::Clock) -> Self`, builder-style: the clock every job that **writes a journal entry** stamps it from (`operate`, `undo`, `preview`). `Clock::Wall` by default; a test that pins it pins the ages the journal browser then draws (m5/25). Callers: in-process tests and examples only. The shipped binary keeps `Wall`, whatever `TRIB_NOW` says (m8/19e §R1).
- `JobRunner::with_askpass_helper` (fn) — name the binary git runs as its askpass helper instead of `current_exe`; under `cargo test` `current_exe` is the test binary, which has no askpass mode (m4-remotes/06).
- `JobRunner::answer_credential` (fn) — `(&self, JobId, Option<Secret>) -> bool`: hand a waiting job its answer (`None` = cancelled); `false` means the job had already gone and the secret was dropped. What `Command::AnswerCredential`/`CancelCredential` dispatch to.

## jobs::rewrite
The put-in's reads (m7-power-tools/09).
- `RewritePlanned` (enum) — `Plan(Box<PlannedPutIn>)`, `Refused(Refusal)`, `NothingBuiltOnHead`, `RewordHead` (m8-polish-and-ship/14: `a` on `HEAD` with nothing staged and a line built on it — the app opens the editor, then plans `RewriteRequest::RewordHead`). Answered only after `RewordHead { message: "" }` plans clean (`jobs::rewrite::reword_or_refused`): a reword that cannot run is `Refused` at `a`, before any editor opens (review m8-14 Major 2); `Product::RewritePlanned`'s answer to `Command::PlanRewrite`.
- `PlannedPutIn` (struct) — `params` (what `y` runs), `offsets` (each target's first-parent steps below `HEAD`), `words: DoneWords` (`what`, `also`), `shown: BoxWords` (`title`, `confirm`, `rows`).
- `PUT_IT_IN` / `PUT_THEM_IN` (consts) — the confirm verbs `y` names.
- `offsets`, `parse_numstat`, `amend_into_words`, `absorb_words`, `stays_words`, `reword_words` (fns, `pub(crate)`; the words live in `jobs/rewrite/words.rs`, re-exported) — pure: the tag offsets from a plan, and the box and landing words (one `stays staged · <why>` per `StaysWhy`).
- `Command::ResolveRowTags(id, offsets)` → `Product::RowTagsResolved(Result<Vec<Oid>, String>)`: one `rev-list --first-parent` below the new `HEAD`.

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
- `Promotion` (enum) — where promoting to topological order is: `Sorting`, `Swapping { order }` (the swap re-finds the commit the cursor is on when it lands, 13m), `Kept` (stopped or failed; order stays provisional).
- `topological_order` (fn) — the whole range in strict topological order, as handles sharing the store's `CommitTable` (a job's work); `Ok(None)` when stopped.
- `Views` (struct) — one `StreamingLayout<Included<_>>` per `Follow`, opened on first ask; `Views::new`, `Views::layout` (opens or re-includes), `Views::promote` (synchronous: lay a promoted order out to the anchor, then swap; idempotent; kept for `examples/budgets`), `Views::begin_promotion` (the worker's non-blocking form, see above). `history/views/mod.rs`; promotion methods in `history/views/promotion.rs`.
- `Open` (type) — boxed closure opening a fresh source over the same walk, or over a promoted order when given one.
- `Window` (struct) — rows answering one request: `request`, `first`, `rows`, `laid_out`, `done`, `promoted` (the first answer in a promoted order); `end()`.
- `HistoryUpdate` (enum) — what a walk tells the screen: `Started(facts)`, `Progress { request, laid_out }`, `Window(window)`, `LaidOut { rows }` (see above).
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

## history::source
- `WalkSource` (struct) — a walk paired with the repo handle it re-reads from, one worker thread; `pull` yields links (`Walk::next_links`), `read` hydrates by handle through `read_commit`.
- `WalkSource::new` (fn) — build a source that streams `walk` and re-reads from `handle`; handles from the walk's table.
- `WalkSource::in_order` (fn) — build a source that streams a known order of handles, reading each commit; takes the `Arc<CommitTable>` the order's handles were interned against.

## history::seeds
- `seeds_for` (fn) — compute walk seeds from refs and branches; symbolic refs excluded.
- `preview_seeds` (fn) — `(Option<&WalkFacts>, &RefBadges) -> Seeds`: the same seeds from what the **app** holds rather than from `&[RefEntry]`/`&Branches`, which it never has. A preview's ghosts land in the live graph's lanes and colours only if the seeds match, so this reads `default_branch`/`integration` off the walk's own facts and the tips off the badges (m5/08).

## working_tree
- `read` (fn) — read working-tree status counts off the repository store (a job's work).
