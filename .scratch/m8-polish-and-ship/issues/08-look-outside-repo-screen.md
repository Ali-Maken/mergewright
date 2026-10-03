# G-look 5c — outside a repository: the mock-up's empty state

Status: done 2026-10-04 (re-specced 2026-10-03 after scout `.scratch/m8-polish-and-ship/scouts/08-look-outside-repo.md`)
Build with: **Opus** — one structural move (terminal + probe hoisted out of `run`, decided below), judged by pty walks
that count alt-screen entries and probe queries; not Fable (fails "can't be judged from tests").
Blocked by: 06, 07 (07 shares `graft-ui/src/lib.rs`, `graft-ui/MAP.md` and the `selection` paint; build after it merges)
Needs: 06 `APP_NAME`, bar form; 07 `Theme::legible_on`, the `▸` gutter + `selection` cursor-row paint (RECENT's cursor
row is painted exactly as the graph's).
Crates: graft, graft-ui, graft-ops (new `init` module); graft-git only if its runner can't spawn without a repo
(smallest repo-less entry, named in NOTES); graft-fixtures only if the pty harness needs the config-dir isolation below.

## Read first
- `docs/design/01-view-mockups.md` §1.15 (whole); `docs/design/02-keymap.md` §2.13 and the text-field paragraph (:78)
- `docs/adr/0056-…` Consequences (`Exit::Open`)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys rows 14, 21; "Where he lands" rows startup-outside / open / init;
  fence ids `open-recent-outside`, `open-second-recent`, `init-here`; Interfaces "graft-ui::empty_state" and
  "graft — recents, terminal and in-process switch (5c)", "graft-ops::init"
- The scout file (whole — file:line for every seam below)
- `docs/shots/m8-proto-3a-empty.txt`, `m8-proto-3c-clone-prompt.txt` (where a prompt sits); `real-vs-mockup.md` diff 4
- `crates/graft/MAP.md` (event_loop §`run`, `Exit`, `Outside`, `Session`, config dir); `crates/graft-ui/MAP.md`
  (`TextField`/`FieldView`, `FieldWidget`, any relative-age formatter); `crates/graft-ops/MAP.md` head (seam rule);
  `crates/graft-git/MAP.md` `run` + env scrubbing

## Decisions (made here; the scout's four findings)
1. **Re-entry: the terminal outlives the repo.** Today `run` opens the session, crash hook and probe each call
   (`startup.rs:95-99,136`). Hoist them into a `Terminal` opened **once** by `main`; a driver loops over places.
   Alt-screen entered once, probe once, crash hook once, `wind_down` once. `run(Startup) -> Exit` **stays** as
   open-run-close for the gallery and existing tests (they keep re-probing; unchanged).
2. **Where the decision is made:** `main` locates before touching the terminal. No repo found / folder missing →
   `Outside`; any other open error → `Exit::Fatal` exactly as today (a broken repo is not "no repository here").
   **Non-tty stdout outside a repo → today's `graft: …not inside a git repository`, exit 1** (pipes and scripts get
   an error, not a screen; the headless path has no empty state).
3. **Init is in scope** (keymap §2.13 `i`, note J21 and Lands rows — approved, not a Proposed change). No init path
   exists → new `graft_ops::init::init_here` (seam rule: app never builds a write's `GitInvocation`). **Not an
   `Operation`, not journaled, not undoable**: there is no journal before it, and reversing it means deleting `.git`,
   which Graft never does. Runs synchronously on the empty-state loop (local, ms; no job runner exists there). Same
   scrubbed git env as every other call — a stray `GIT_DIR` must not redirect it (test).
4. **Open fails / init fails:** the driver re-enters `run_outside` with `problem` set and the cursor kept; same
   terminal, no alt-screen leave, so the screen visibly stays.

## Scope
In:
- `graft-ui::empty_state` widget per §1.15: header `<APP_NAME> · no repository here`; folder line, `~`-abbreviated,
  `· no such folder` when missing; actions row `o Open…  i Init` only (`c`, `t` not drawn — §1.15's last rule);
  RECENT (name bold, path + age muted, newest preselected, hidden when empty); rule; bar
  `o open… · i init · ⏎ open the selected · q quit` (`⏎` hint only when RECENT is non-empty). Missing folder: `i` not
  drawn, not answered (see Questions).
- Prompts in the slot 3c's clone prompt uses: `open › <path>▏` (`~` expanded, relative = against the shown folder,
  `⇥` completes a directory, `⏎` opens, `esc` cancels; bar `⏎ open · ⇥ complete · esc cancel`); init confirm
  `Make <folder> a Git repository?`, bar `y init · esc keep it`; `y` inits, `n`/`esc` decline (Lands row "Init
  declined").
- Keys: `↑↓ j k` move (clamp or wrap exactly as the branch picker does — check, match), `⏎` opens selected, `q`
  quits exit 0, `Ctrl+C`/signals as the graph screen. Resize redraws.
- `problem`: drawn in the bar row (warning) until the next key, which is also acted on; then the bar returns.
- Recents file `<config_dir>/graft/recent`: one line per repo `<unix_secs>\t<canonical path>`, newest first, ≤ 20,
  deduped by canonical path; written by temp file + rename (a crash never leaves half a file). Written on every
  successful open **in a tty run** (startup inside a repo too, `o`, `⏎`, init); headless runs don't write. Missing
  file = empty list. Unparsable lines: skipped and counted → `recent: N unreadable lines skipped` (empty state bar
  row); `remember` drops them and says `recent: N unreadable lines dropped`. `remember` failing → the repo still
  opens; graph status row `recent: not saved · <reason>` once. No config dir → no RECENT, bar row says
  `recent: no config folder` once. Path containing `\n`/`\t` → not remembered, said the same way (rule 3).
- Age words as the mock-up (`yesterday`, `3 days ago`, `last week`); reuse an existing relative-age formatter if a
  MAP names one, else one small fn beside the widget.
- Config problems (`config: N problems`) go to the first repo screen of the process, not repeated after a switch.
Out: clone `c`, first-launch line (18); tour `t`, `Exit::Open` widening (19); `?` doorway (11); keymap-table
translation of these keys (09/10 migrate them; until then a hand dispatcher in `graft::outside`, like today's screens).

## Signatures
```rust
// graft-ui::empty_state (as the note; unchanged)
pub struct RecentRow { pub name: SanitizedText, pub path: SanitizedText, pub age: SanitizedText }
pub struct EmptyStateView { pub folder: SanitizedText, pub missing: bool, pub recent: Vec<RecentRow>,
    pub cursor: usize, pub prompt: Option<EmptyPrompt>, pub problem: Option<SanitizedText> }
pub enum EmptyPrompt { Open(FieldView), ConfirmInit }
pub struct EmptyState<'a> { pub view: &'a EmptyStateView, pub theme: &'a Theme }   // impl Widget

// graft-ops::init — not an Operation, not journaled
pub fn init_here(dir: &Path) -> Result<PathBuf, InitError>;   // `git init` in dir; Ok = the new repo's workdir
pub enum InitError { Missing(PathBuf), Git(String) }          // Display is the bar row's reason

// graft::recent (pub(crate)) — replaces the note's Vec<PathBuf> shape (age + skipped count needed)
pub(crate) struct Recent { pub path: PathBuf, pub opened: SystemTime }
pub(crate) struct RecentList { pub entries: Vec<Recent>, pub skipped: usize }   // newest first, ≤ 20, deduped
pub(crate) fn read_recent(config_dir: &Path) -> Result<RecentList, RecentError>;
pub(crate) fn remember(config_dir: &Path, repo: &Path, now: SystemTime) -> Result<Remembered, RecentError>;
pub(crate) struct Remembered { pub dropped: usize }

// graft::event_loop::startup (pub(crate))
pub(crate) struct Terminal { /* TerminalSession, Capabilities (probed once), crash hook, Inputs */ }
impl Terminal {
    pub(crate) fn open(config: &Config) -> Result<Terminal, String>;  // the only caller of `probe` on this path
    pub(crate) fn close(self);                                        // wind_down
}
pub(crate) enum Place { Repo(RepoStore), Outside(OutsideStart) }
pub(crate) struct OutsideStart { pub folder: PathBuf, pub missing: bool, pub cursor: usize, pub problem: Option<String> }
pub(crate) fn locate(path: &Path) -> Result<Place, String>;          // Err = Fatal as today
pub(crate) fn drive(term: &mut Terminal, first: Place, config: &Config) -> Exit;   // handles Open; never returns it
pub(crate) fn run_repo(term: &mut Terminal, store: RepoStore, config: &Config) -> Exit;
pub(crate) fn run_outside(term: &mut Terminal, start: OutsideStart, config: &Config) -> Exit;
pub(crate) fn run(startup: Startup) -> Exit;                         // kept: open + run_repo + close
// Exit gains Open(PathBuf)

// graft::outside (pub(crate)) — pure, unit-tested
pub(crate) struct OutsideState { /* view inputs, recents, cursor, prompt */ }
pub(crate) enum OutsideStep { Stay, Open(PathBuf), Init(PathBuf), Quit }
pub(crate) fn outside_key(state: &mut OutsideState, key: KeyEvent) -> OutsideStep;
```
Parameter types (`Config`, `Startup`, the session's real names) follow what `run` takes today — match the code, keep
the shape: one `Terminal`, a driver, two per-place runs. Name any rename in NOTES.

## Acceptance
- [x] Widget tests 80×24, 120×36: rows as §1.15; RECENT hidden when empty; `⏎` hint hidden then; missing-folder line,
      `i` absent; `c`/`t` absent; open prompt and init confirm forms — graft-ui `empty_state::tests::`
      `rows_are_as_the_mockup_at_both_sizes`, `an_empty_list_draws_no_recent_block_and_no_enter_hint`,
      `a_missing_folder_says_so_and_offers_no_init`, `the_open_prompt_sits_under_the_list_with_its_own_keys`,
      `the_init_confirm_names_the_folder_and_offers_y` (+ `a_long_missing_folder_keeps_no_such_folder`,
      `a_long_folder_in_the_init_confirm_keeps_the_question`, `the_cursor_row_is_on_the_selection_fill_and_legible`).
- [x] `outside_key` tests: every key in Scope → its step; `i esc`, `i n` → `Stay`, nothing written — graft
      `empty_state::tests::` `enter_opens_the_selected_recent`, `j_then_enter_opens_the_second`,
      `moving_clamps_at_both_ends_like_the_branch_picker`, `q_and_ctrl_c_quit`, `i_then_y_inits_the_folder`,
      `i_then_esc_or_n_declines_and_writes_nothing`, `o_prefills_the_folder_and_enter_opens_what_was_typed`,
      `esc_cancels_the_open_prompt`, `tab_completes_a_directory`, `a_missing_folder_does_not_answer_i`.
- [x] Recents tests: newest first, dedup by canonical path, cap 20, unparsable counted, missing file empty, atomic
      write (no partial file after a failed rename), `\n` path refused — graft `recent::tests::`
      `the_newest_comes_first`, `the_same_repository_is_listed_once_by_its_canonical_path`, `the_list_keeps_twenty`,
      `unreadable_lines_are_skipped_and_counted_then_dropped`, `a_missing_file_is_an_empty_list`,
      `a_failed_rename_leaves_no_partial_file`, `a_path_with_a_newline_is_refused_and_the_file_unchanged`.
- [x] `init_here` tests: creates `.git` in a temp dir; missing dir → `Missing`; `GIT_DIR` set in env → still inits
      `dir` — graft-ops `init::tests::` `init_here_creates_a_git_directory_in_the_folder`,
      `a_missing_folder_is_missing_and_nothing_is_created`, `a_stray_git_dir_in_the_environment_does_not_redirect_it`
      (seen red with the scrub removed).
- [x] J14 `open-recent-outside` = 2: pty walk in a non-repo with a recents file, `⏎` → that repo's graph, cursor on
      HEAD; `open-second-recent` = 3 (`j ⏎`). Walk counts keys against the note's fence —
      `it::outside_repo::open_recent_outside_opens_the_newest_at_head_on_one_terminal`,
      `it::outside_repo::open_second_recent_is_j_then_enter` (both seen red with the HEAD landing switched off).
- [x] J21 `init-here` = 2: `i y` → new repo's graph; `i esc` → empty state, no `.git` (pty) —
      `it::outside_repo::init_here_is_i_then_y`, `it::outside_repo::init_declined_writes_nothing` (`esc` and `n`).
- [x] Re-entry: in the J14 walk, `\e[?1049h` appears **exactly once** and the probe's first query — `ESC _Gi=31,`,
      the kitty-graphics `a=q` that opens `probe::full_round` — exactly once —
      `it::outside_repo::open_recent_outside_opens_the_newest_at_head_on_one_terminal`.
- [x] `o` + a non-repo path → screen stays, cursor kept, bar row reads the reason (pty) —
      `it::outside_repo::opening_a_folder_that_is_not_a_repository_stays_and_says_why`.
- [x] Non-tty stdout outside a repo → exit 1, today's stderr line, no `\e[?1049h` (kept half of `not_a_repo.rs`) —
      `it::not_a_repo::off_a_terminal_a_path_that_is_not_a_repository_is_one_plain_line`; tty half
      `it::not_a_repo::on_a_terminal_a_path_that_is_not_a_repository_shows_the_empty_state`.
- [x] A test named `empty_state_bar_keys_are_answered` — graft `empty_state::tests::empty_state_bar_keys_are_answered`.
- [x] Captures `docs/shots/m8-look-outside-repo-120x36.txt` (with recents), `…-no-recent-80x24.txt`,
      `…-open-prompt-80x24.txt`, from the real binary on a pty — `it::outside_repo::outside_repo_captures`.
      **Colour unverified** (rule 5).

## Touches (each checked, said how in the report)
- `crates/graft/tests/it/not_a_repo.rs:9-31` — rewrite: tty half (empty state, `q` → exit 0, no stderr) + non-tty half.
- Other `Exit::Fatal` tests — `repo_truth.rs`, `m3_walk.rs`, `undo_over_drift.rs`, `m4_walk/run.rs`,
  `m5_walk/{run,resolving}.rs`, `m7_walk/rewrite_editor.rs`: open each, say whether it hits not-a-repo in a tty
  (changes behaviour) or not (unchanged). Unconfirmed by the scout — the builder confirms, per file.
- **Config isolation:** every test that spawns the binary in a pty must set `XDG_CONFIG_HOME` to a temp dir, or
  `remember` writes the real `~/.config/graft/recent`. Grep the harness(es); fix there; name the file.
- The gallery and every other `run` caller (unchanged signature; grep shows them).
- `main.rs:27-72` (startup order, stderr print); `crash::install` (once per process now); `InputMux` reuse.
- graft-ui `lib.rs` mod list and MAP.md (after 07's edits); graft MAP.md (`Terminal`, `drive`, `Place`, `Exit::Open`,
  `outside`, `recent`); graft-ops MAP.md (`init`).
- 18 (clone) and 19 (tour) build on `Exit::Open`, `drive` and the prompt slot: NOTES answers 19's scout question —
  where `Exit::Open` is handled (file:line) and what survives a switch (config, theme, keymap, capabilities).
- 26 G-rename: `<config_dir>/graft/recent` joins its on-disk list (note it in NOTES; don't edit 26).

## Questions (product preference — conductor to the user; build uses the default)
- `trib ~/code/newthing` where the folder doesn't exist: offer `i` (create the folder, then `git init`)? Default: **no**,
  `i` not drawn — a typo shouldn't become a repository.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k tokens. If it compacts, the seam is: recents + widget + `outside_key` first; `Terminal`/`drive` +
init + pty walks second.

## NOTES (builder, 2026-10-04)
- **Compacted once mid-ticket** (after WIP commit 684f40ac, before the pty walks). Nothing remains.
- **User question (missing folder offers `i`?)**: built the default, no. The one switch is the
  `KeyCode::Char('i') if !state.missing` arm in `crates/graft/src/empty_state.rs` `outside_key`; the widget and
  `bar_keys` read the same `missing` flag. Pinned by `it::not_a_repo::on_a_terminal_a_missing_folder_says_so_and_offers_no_init`.
- **Renames vs the spec**: `graft::outside` → `graft::empty_state` (`event_loop::outside` already exists);
  `EmptyPrompt::Open(FieldView)` → `EmptyPrompt::Open { text, cursor }`; `Terminal::open()` takes no config (the
  session is settled per place); `drive(Terminal, …)` consumes the terminal and closes it; `run_repo` takes an
  `Arrival { status, on_head }`.
- **Decision 4, mechanism only**: a failed open/init stays inside the empty-state loop (problem set, cursor untouched)
  rather than re-entering `run_outside`, which would clear and redraw. Same visible result.
- **graft-git repo-less entry**: `GitInvocation::in_folder` + `REPO_LOCATING_ENV` (`crates/graft-git/src/run.rs`) —
  `in_directory` with `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY` removed.
- **Open lands on HEAD** (Lands row): new `update::lands_on_head_at_open` (`crates/graft/src/update/mod.rs`, after
  `opening`) arms the existing owed landing on `opening`'s own refs read and walk. A start *inside* a repository still
  lands on the top row, as before (`it::outside_repo::starting_inside_a_repository_remembers_it`).
- **Hang found and fixed (cause)**: `ratatui::Terminal::clear` asks the terminal where its cursor is (`ESC [6n`) and
  reads stdin, which the input mux owns — the switch hung. `run_outside` now clears through the backend
  (`Backend::clear`), which writes and asks nothing. Found with `sample` on the stuck process.
- **Long paths**: the folder line and the init question cut a long folder at its *start* (`…/code`), so
  `· no such folder` and `a Git repository?` are never what is lost.
- **For 19 (tour) / 18 (clone)**: `Exit::Open` is handled in `crates/graft/src/event_loop/places.rs` `drive`, the
  `Ok(Exit::Open(next))` arm (re-`locate`s; the next repository lands on HEAD). Survives a switch: the one `Terminal`
  (alternate screen, raw mode, `InputMux`, crash hook) and its probed capabilities; the `Loaded` config (warnings on
  the first repository screen only). The `Session` (theme, tier) is re-settled per place from those; the keymap is
  static. The prompt slot: `EmptyPrompt` + `PROMPT_HEIGHT` in the widget.
- **For 26 (G-rename)**: `<config_dir>/graft/recent` joins the on-disk list (`crates/graft/src/recent.rs`
  `recent_file`). 26 not edited.
- **Config isolation**: `crates/graft/tests/it/pty.rs` `Pty::spawn` points `XDG_CONFIG_HOME` and `HOME` at a temp dir
  for every pty run; it is the only Rust harness running the binary on a tty (grep `CARGO_BIN_EXE_graft`: the rest
  use `.output()`, non-tty, which never remembers). **Not fixed, out of crate scope**: the Python pty scripts
  (`scripts/scroll-lag-pty.py`, `.scratch/*/pty-*.py`) run the real binary on a pty without `XDG_CONFIG_HOME`; run
  inside a repository they now add it to the real `~/.config/graft/recent`.
- **Other `Exit::Fatal` tests** (`repo_truth.rs`, `m3_walk.rs`, `undo_over_drift.rs`, `m4_walk/run.rs`,
  `m5_walk/{run,resolving}.rs`, `m7_walk/rewrite_editor.rs`): none names `Exit::Fatal` or not-a-repo (grep); they run
  `event_loop::run` on fixture repositories, unchanged. The examples gained an `Exit::Open` arm that errors.
- **Contact sheet**: no m8 contact sheet exists yet; nothing to re-assemble.
