# Capabilities — M0 Foundations

M0 built the scaffolding everything later stands on: a terminal session that
starts and ends cleanly, a probe that works out what the terminal can draw
before the first frame, a theme system, a config file, a job runner so
nothing blocks the screen, and enough of a repo-open path to show a status
screen. Most of it is invisible on its own — nobody sees "a job runner" —
but a handful of things are directly visible from the first run: the app
opens a repository, shows what it found, adapts to the terminal's real
capabilities, and survives a crash or a signal without leaving the terminal
broken.

**Invisible foundations** (not given their own row): the seven-crate
workspace layout with an enforced dependency graph (`graft::tests::it::
dependency_edges::internal_dependency_edges_match_architecture_md` — fails
loudly if a crate reaches across a layer it shouldn't); the `Event → update
→ Commands → view` event-loop spine every later feature adds an arm to;
`graft-core`'s minimal identity types (`Oid`, `RefName`, `JobId`, `Event`);
synchronized-output (DEC 2026) wrapped around every draw; 0×0/1×1 terminal
sizes clamped instead of panicking.

## Capabilities

### M0-01. Opens a Git repository from a path, or the current directory

- **How:** `graft [path]`, or `graft` with no argument (uses the current
  directory).
- **Behaviour:** Discovery (via `gix`) walks up from the given directory to
  find a repository, so running from a subdirectory works. Distinguishes: a
  normal branch checkout, a detached `HEAD`, an unborn branch (brand-new
  empty repo), a bare repo, a linked worktree, and a submodule's working
  directory. A path that isn't a repository prints a clean error to stderr
  and exits 1 without ever entering the alternate screen — the terminal is
  never left dirty. Opening the same path twice (e.g. two jobs racing)
  shares one open repository handle rather than opening it again.
- **Proven by:**
  `graft::jobs::tests::opening_a_repository_reports_what_it_found`
  `graft::jobs::tests::a_path_that_is_not_a_repository_comes_back_as_an_answer`
  `graft::jobs::tests::two_jobs_share_one_open_repository`
  `graft::update::tests::a_path_that_is_not_a_repository_ends_the_run`
  `graft::update::tests::a_failed_refresh_of_an_open_repository_only_reaches_the_status_line`
  `graft_git::version::tests::the_real_git_binary_is_run_and_its_output_parsed`
- **Decided in:** `PROMPT.md` §4 "Repositories" (only the "open from cwd or
  path" bullet — `init`, `clone` and recent-repos are out of M0's scope,
  deferred to a later milestone).
- **Status:** works. Non-UTF-8 repo paths are typed for (`RepoHandle` is
  `Path`/`PathBuf` end to end) but unverified here — the fixture can't be
  built on this machine's APFS filesystem (`KNOWN_GAPS.md`, expiry: M1 heavy
  tier on Linux).

### M0-02. The terminal's real drawing capability is detected before the first frame

- **How:** automatic at startup; also runnable standalone via
  `cargo run -p graft --example capability_probe`.
- **Behaviour:** reads cheap environment hints first (`$TERM`,
  `$TERM_PROGRAM`, `$COLORTERM`, `$KITTY_WINDOW_ID`, `$TMUX`, `$SSH_TTY`,
  `$NO_COLOR`), never trusted alone. Then sends active queries together and
  reads whatever answers arrive inside a 200 ms budget (500 ms over SSH):
  Kitty graphics protocol, Sixel (`DA1`), cell size in pixels (`CSI16t`),
  Kitty keyboard protocol, synchronized-output support, and `OSC 11`
  background colour for the light/dark theme hint. Every query falls back
  independently on silence, never as a group: cell size falls back to
  `TIOCGWINSZ`, then to an assumed 8×16 px marked "approximate"; a silent
  `OSC 11` defaults to the dark theme, flagged "unknown". Tier order is
  Kitty → Unicode → iTerm2 → Sixel → ASCII (ADR-0016) — Unicode is the
  terminal's default, everyday tier, not a fallback, because Zed and VS
  Code (the user's daily terminals) have no image protocol at all. iTerm2 is
  the one tier allowed to rely on an environment hint alone
  (`$TERM_PROGRAM == "iTerm.app"` + a minimum version), since it has no
  query protocol of its own.
- **Proven by:**
  `graft_term::detect::tests::a_terminal_that_answers_everything_gets_the_pixel_tier`
  `graft_term::detect::tests::a_silent_terminal_falls_back_per_query_and_says_so`
  `graft_term::detect::tests::kitty_wins_over_sixel_when_both_answer`
  `graft_term::detect::tests::tmux_too_old_to_pass_anything_through_falls_back_to_unicode`
  `graft_term::detect::tests::tmux_with_passthrough_reaches_the_outer_terminal`
  `graft_term::detect::tests::ssh_buys_the_terminal_an_extra_round_trip`
  `graft_term::detect::tests::a_light_background_gives_the_light_theme`
  (46 tests total in this module, per the ticket; these seven were
  individually confirmed to exist.)
- **Decided in:** `docs/design/04-terminal-capability-detection.md` §4.1–§4.4;
  ADR-0016 (tier order).
- **Status:** partial. Confirmed live in a real terminal exactly once: at
  the M0 checkpoint, in Ghostty, the user watched the app pick the pixel
  tier (Kitty protocol), cell size 17×37, dark theme from the terminal's own
  background, truecolor, probe answered in 367 µs — approved by eye, but
  that session was never captured to a file (`docs/shots/` captures don't
  start until M1 ticket 08; M0 predates the convention). Every other real
  terminal — iTerm2, Kitty, WezTerm, Terminal.app, Zed, VS Code, tmux (both
  passthrough states), SSH — remains unverified for tier detection
  specifically (`KNOWN_GAPS.md` "No real terminal has been seen answering
  the capability probe", expiry M0 review; the file's own preamble
  reconciles this against the Ghostty session above).

### M0-03. The status screen shows the repo, the rendering tier, the theme and any warnings

- **How:** shown automatically once a repository opens successfully.
- **Behaviour:** displays the working directory path, `HEAD` state,
  rendering tier with its provenance (e.g. "detected" vs "assumed"), the
  active theme with its provenance, a warning if the system `git` is older
  than 2.38, and any config-load warnings. Every repo-derived string (paths,
  branch names) passes through the sanitiser (M0-04) before it reaches the
  screen.
- **Proven by:** `graft::view::tests::every_graph_renderer_config_value_draws_as_it_says`
  (insta snapshots `renderer-{Auto,Pixel,Unicode,Ascii}.snap`, including a
  branch name carrying a raw ANSI escape, which renders inert). Manually
  confirmed under a bare pty (not a real emulator): `graft .` shows the
  right fields, `q` exits 0, `graft /tmp` errors and exits 1, `SIGINT`
  restores cleanly, `NO_COLOR=1` produces no SGR colour at all.
- **Decided in:** `docs/design/01-view-mockups.md`.
- **Status:** partial. A known, named bug ships unfixed here: the tier line
  always prints "detected", even when nothing answered a probe and the
  value is really a guess or a fallback — the exact "a status field reads
  as fine when nothing is fine" pattern this project's own rules ban.
  Logged in `KNOWN_GAPS.md`, expiry M1. This is **broken**, not unverified:
  the mislabel is confirmed, not merely unproven.

### M0-04. Repo-derived text can't attack the terminal

- **How:** automatic — applied to every branch name, commit subject, path or
  other string that comes from the repository before it's drawn.
- **Behaviour:** strips or replaces C0 controls, `DEL`, the C1 block, `ESC`,
  bidi override/isolate characters, and the whole Unicode `Cf` (format)
  category plus U+2028/U+2029 line/paragraph separators — wider than
  DESIGN.md's original five named ranges, a deliberate widening confirmed
  with the user (ADR pending). Forbidden characters become `␛` (U+241B) for
  C0/DEL/C1, or `�` (U+FFFD) for format-category/line-separator characters.
  Sanitising is idempotent and length-preserving per character in, character
  out. `SanitizedText` cannot be constructed outside `graft-term` (a type
  boundary, not just a function). A known, accepted side effect: multi-part
  emoji (family, profession sequences) break apart, because the zero-width
  joiner (U+200D) is itself in `Cf` and gets stripped.
- **Proven by:**
  `graft_term::sanitize::tests::ansi_colour_injection_in_a_commit_message_comes_out_inert`
  `graft_term::sanitize::tests::an_author_name_carrying_an_escape_sequence_comes_out_inert`
  `graft_term::sanitize::tests::osc_8_hyperlink_comes_out_inert`
  `graft_term::sanitize::tests::a_bidi_spoofed_branch_name_comes_out_inert`
  `graft_term::sanitize::tests::bidi_isolates_come_out_inert`
  `graft_term::sanitize::tests::a_zero_width_payload_comes_out_inert`
  `graft_term::sanitize::tests::invisible_characters_outside_design_md_s_list_are_neutralised_too`
  `graft_term::sanitize::tests::newline_and_tab_pass_through_untouched`
  `graft_term::sanitize::tests::every_other_c0_control_is_replaced_visibly`
  `graft_term::sanitize::tests::del_and_the_c1_block_are_replaced_visibly`
  `graft_term::sanitize::tests::sanitising_is_idempotent`
  `graft_term::sanitize::tests::invalid_utf8_bytes_become_visible_replacements`
  `graft_term::sanitize::tests::a_non_utf8_os_str_path_is_sanitised_lossily`
  `graft_term::sanitize::tests::a_newline_or_tab_in_a_file_name_cannot_reach_a_single_line_row`
  plus property tests `output_is_inert_length_preserving_and_idempotent`,
  `arbitrary_path_bytes_are_inert_too`, `single_line_output_cannot_move_the_cursor`
  in the same file.
- **Decided in:** `docs/design/08-architecture.md` §8.2 (the `SanitizedText`
  boundary).
- **Status:** works (deliberately wider than the original spec, confirmed
  with the user). Gap, not planned unless it becomes a problem in daily use:
  homoglyphs, combining-mark stacks, East-Asian ambiguous width, and
  non-`Cf` invisibles (variation selectors, Hangul fillers) all still pass
  through unsanitised — a readability/spoofing risk, not a terminal-attack
  one, and out of scope for M0's threat model.

### M0-05. Theme adapts to the terminal's background and colour depth

- **How:** automatic — `OSC 11` background hint, `ui.theme` config override,
  and `NO_COLOR`.
- **Behaviour:** three themes (dark, light, high-contrast), each 23 tokens
  (11 chrome colours + 12 lane hues). A config override always beats the
  terminal hint; an unanswered hint defaults to dark. The 12-hue lane
  palette is one fixed, colour-vision-deficiency-safe set, independent of
  which theme is active — enforced structurally (`Theme` has no `lane`
  field a theme could override). At reduced colour depth, truecolor values
  map down to 256 or 16 colours via OKLab distance with 2× chroma
  weighting, which keeps near-neutral chrome tokens from picking up a false
  hue. Under `NO_COLOR`, chrome tokens collapse to no colour at all — the
  only SGR left in the byte stream is the reset.
- **Proven by:**
  `graft_ui::theme::tests::no_theme_can_change_a_lane_hue`
  `graft_ui::theme::tests::near_neutral_chrome_does_not_pick_up_a_hue_at_sixteen_colours`
- **Decided in:** `PROMPT.md` §5 (tokens, themes, `NO_COLOR`); `docs/design/
  04-terminal-capability-detection.md` §4.6 (`NO_COLOR` interaction).
- **Status:** partial. Numerically and unit-tested correct; unverified by
  eye. `KNOWN_GAPS.md`: at 16 colours the 12 lane hues collapse toward 6
  distinguishable ones, and dark chrome flattens toward one background —
  never seen as thin graph lines, never judged by a colour-vision-deficient
  viewer (two open rows, expiry M1 checkpoint). Per this project's rules, a
  captured terminal stream can't prove colour either way — this stays
  unverified until the user looks.

### M0-06. A config file controls theme, renderer and icon style

- **How:** `${XDG_CONFIG_HOME:-~/.config}/graft/config.toml`, read once at
  startup.
- **Behaviour:** keys are `ui.theme` (`auto`/`dark`/`light`/
  `high-contrast`), `graph.renderer` (`auto`/`pixel`/`unicode`/`ascii`,
  always wins over detection when set), and `ui.icons` (`plain`/`nerd`, no
  consumer yet in M0). A missing file silently uses the defaults. A
  malformed file never falls back silently and never panics — it produces a
  typed error naming the file, line and column. An unknown key warns but
  still loads the rest of the file (forward-compatible on purpose;
  `deny_unknown_fields` was deliberately rejected). `XDG_CONFIG_HOME` wins
  over `$HOME` when both are set; no `$HOME` at all means no config path is
  even attempted.
- **Proven by:**
  `graft::config::tests::a_missing_file_is_the_defaults_and_says_nothing`
  `graft::config::tests::broken_toml_names_the_file_the_line_and_the_problem`
  `graft::config::tests::a_value_that_isnt_a_known_spelling_is_an_error_not_a_default`
  `graft::config::tests::an_unknown_key_warns_and_the_rest_still_loads`
  `graft::config::tests::xdg_config_home_wins_over_home`
  `graft::config::tests::no_home_at_all_means_no_path`
  (29 tests total in this module; none touch the real `~`.)
- **Decided in:** `docs/design/04-terminal-capability-detection.md` §4.2
  (`graph.renderer` override semantics).
- **Status:** works.

### M0-07. A panic reports itself and never leaves the terminal broken

- **How:** automatic — any unrecoverable panic, in the UI thread or inside a
  background job.
- **Behaviour:** the panic hook restores the terminal *first*, then writes a
  crash report to `${XDG_STATE_HOME:-~/.local/state}/graft/crash-
  <timestamp>.log` (message, location, and a backtrace via
  `Backtrace::force_capture()`, independent of `RUST_BACKTRACE`), then
  prints exactly one plain stderr line naming the report's path. A second
  panic in the same second gets its own file rather than overwriting the
  first. A job's panic is caught at the worker boundary and reported as a
  failed-job event — the panic hook is deliberately "stood down" while a job
  is running so it doesn't also fire and tear down the terminal mid-frame.
- **Proven by:**
  `graft::crash::tests::a_report_file_holds_the_message_location_and_backtrace`
  `graft::crash::tests::a_second_report_in_the_same_second_does_not_overwrite_the_first`
  `graft::crash::tests::an_unwritable_report_is_printed_instead_and_a_restore_failure_is_named`
  `graft::crash::tests::a_thread_inside_catching_tells_the_hook_to_stand_down`
  `graft::crash::tests::a_panic_that_escapes_catching_clears_the_mark_behind_it`
  `graft::crash::tests::timestamps_cover_the_epoch_leap_years_and_century_rules`
  `graft::jobs::tests::a_panicking_job_is_reported_and_the_pool_survives`
- **Decided in:** `docs/design/08-architecture.md` (panic-hook/terminal
  ownership boundary).
- **Status:** partial. Confirmed under a bare pty: exit code 101, termios
  byte-identical before and after, the restore sequence appears exactly
  once, the crash log has the right content, and a real backtrace comes
  back even with `RUST_BACKTRACE=0`. Never fired the real, installed
  process-wide panic hook inside an actual terminal emulator —
  `KNOWN_GAPS.md`, expiry M0 review ("panic a job by hand in a real
  terminal; check the screen survives").

### M0-08. Quitting — by key, Ctrl-C, or a signal — always leaves the terminal clean

- **How:** the quit key, `Ctrl-C`, `SIGINT`, `SIGTERM`, `SIGHUP`.
- **Behaviour:** teardown undoes setup in exact reverse order (raw mode →
  alt-screen → mouse → bracketed paste → focus reporting → hidden cursor →
  Kitty keyboard push, each undone in the opposite order it was applied),
  and is idempotent — a double drop or a failed mid-setup drop still leaves
  the terminal in a sane state. Signal handlers only set a flag or wake the
  event loop; the loop then returns normally so the ordinary `Drop` path
  restores the terminal, never restoring from inside the handler and never
  calling `exit()` from it.
- **Proven by:**
  `graft::terminal::tests::teardown_undoes_every_step_in_reverse_order`
  `graft::terminal::tests::suspend_undoes_every_mode_and_resume_puts_the_same_ones_back`
  `graft::terminal::tests::restoring_twice_emits_nothing_the_second_time`
  `graft::terminal::tests::a_setup_that_fails_midway_undoes_what_it_managed_to_apply`
  `graft::terminal::tests::a_setup_that_fails_at_the_first_step_leaves_nothing_behind`
  `graft::terminal::tests::a_failing_undo_does_not_stop_the_rest_of_the_teardown`
  `graft::terminal::tests::a_poisoned_lock_still_restores`
- **Decided in:** `PROMPT.md` §3 "Undo and redo" note on `Ctrl+Z` (adjacent
  signal-handling rule: `Ctrl+Z`/`SIGTSTP` is captured for undo by default,
  restorable via `keys.suspend_on_ctrl_z = true` — the undo feature itself
  isn't built until M3, but M0 already reserves the key's behaviour).
- **Status:** works, strongly verified — a real measurement, not just green
  tests. Under a bare pty, `.scratch/m0-foundations/pty-signal-check.py`
  compared the entire termios struct (every flag word, both speeds, every
  `cc` slot) byte-for-byte before launch and after each of `SIGINT`,
  `SIGTERM` and `SIGHUP`, and found them identical in all three cases. This
  closed a real bug found during M0: relying on each signal's default
  disposition left `ECHO`/`ICANON`/`ISIG` off after a raw kill; the fix
  catches the signals explicitly instead. Only ever run under a bare pty,
  never a real terminal emulator.

### M0-09. Long-running work never freezes the screen

- **How:** invisible in itself — observed indirectly whenever the app is
  doing something in the background (opening a repository, `Ctrl+R` manual
  refresh): the UI stays responsive, and a failure shows up as a status-line
  message instead of a frozen screen.
- **Behaviour:** a fixed pool of at most 4 workers (the laptop-safe budget
  from `PROMPT.md` §7.1), fed by a FIFO channel. Cancellation is
  cooperative: a job that hasn't started yet is dropped instantly rather
  than run and discarded. A panicking job is caught and reported as a
  failed-job event without killing the rest of the pool. Shutdown is
  time-bounded — it never blocks on a join; a worker that won't stop is
  reported as `Stalled{workers}` instead of hanging the process.
- **Proven by:**
  `graft::jobs::tests::a_finished_job_comes_back_as_an_event`
  `graft::jobs::tests::six_jobs_on_a_pool_of_two_all_finish`
  `graft::jobs::tests::a_pool_of_two_runs_two_jobs_at_once`
  `graft::jobs::tests::a_running_job_stops_when_it_is_cancelled`
  `graft::jobs::tests::a_job_cancelled_before_it_starts_never_runs`
  `graft::jobs::tests::a_panicking_job_is_reported_and_the_pool_survives`
  `graft::jobs::tests::shutdown_gives_up_on_a_job_that_ignores_its_token`
  `graft::jobs::tests::shutting_down_an_idle_pool_is_clean`
  `graft::jobs::tests::the_pool_is_never_empty_and_never_larger_than_the_cap`
  `graft::jobs::tests::the_default_pool_fits_a_laptop`
- **Decided in:** `PROMPT.md` §7.1 (laptop-safe resource budget).
- **Status:** works.

## Deliberately not built

- **Sixel encoder.** Sixel can win the terminal-tier decision (§4.2's table)
  with nothing that can actually encode to it — a gap carried forward, owner
  M1 ticket 14. Not an M0 defect: M0 only detects; it doesn't render pixels
  at all yet.
- **CI, and criterion benchmarks.** `PROMPT.md` §7 item 6 asks for these but
  there is no GitHub repository this codename can be pushed to yet.
  `KNOWN_GAPS.md`: not planned until that changes.
- **`ui.icons` has no consumer.** The config key parses and validates, but
  nothing in M0 draws an icon that reads it — the graph screen that would
  use it doesn't exist until M1.
- **Homoglyph, width and non-`Cf`-invisible sanitising.** Named explicitly
  above (M0-04) as a scoped-out threat, not an oversight: it's a
  readability/spoofing question for later, not a terminal-attack one.
- **Any repo write (init, clone, commit, etc).** M0 only opens and reads.
  Every write operation is out of scope until later milestones.
