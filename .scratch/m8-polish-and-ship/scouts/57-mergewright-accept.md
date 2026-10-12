# Scout 57: Mergewright walks

**1. Fixtures / Generation**
- `TempRepo::init_on(generation)` is at `crates/graft-ops/tests/it/support.rs:52`. It is **test-only code in the graft-ops integration test binary**, so the graft crate cannot call it: there is no feature and no path to it. What it builds: `git init -b main`. When `generation.has_marker()` (true for everything except Graft, `on_disk.rs:123`) it writes `.git/<dir>/migrated` containing `clean\n`. It makes no refs, no journal and no stash tag ("refs are the caller's", :50-51).
- `Generation` (`crates/graft-core/src/on_disk.rs:20`) has the variants Graft, Tributary and Mergewright. `CURRENT` is Mergewright (:33). `GENERATIONS` is at :36. It has these methods: `dir()` (graft/tributary/mergewright), `title()`, `command()` (graft/trib/mwr), `env_prefix()`, `ref_prefix()` (`refs/<dir>/`), `link_trailer()`, `backup_email()`, `has_marker()` and `older()`. `MIGRATED_MARKER="migrated"` is at :179 and `CURRENT_DIR` at :192.
- Lock: `LOCK_FILE="lock"` (`crates/graft-ops/src/rename.rs:82`). A test takes it at `<common>/mergewright/lock` (`rename_status.rs:54`). The exact text is `MigrateError::Busy`, `"another {title} window is open on this repository"` (rename.rs:161). The status row reads `rename: still using the old names · another Mergewright window is open on this repository`, asserted at `rename_status::kept_old_words` (rename_status.rs:31-78). That test is headless (`RepoStore`). I found **no pty or app-launch test on a Graft-era or Tributary-era repo**. The fixture there is `graft_fixtures::copy_into_temp` plus `update-ref refs/graft/backup/main/1`, with the marker removed (:31-52).

**2. Harnesses**
- pty (`crates/graft/tests/it/pty.rs`):
  - Spawn with `Isolated::new()` / `first_launch()` (:75/:84), then `Pty::spawn(&isolated, arg, ceiling, (cols, rows))` (:141). Variants are `spawn_with_env` (:186), `spawn_with_stderr_on_pty` (:148) and `spawn_without_git_config` (:162). The binary is `CARGO_BIN_EXE_mwr` (:230).
  - Driving it: `send(&[u8])` :277, `screen()` :298, `wait_for(what, |s| ..)` :304, `wait_exit()` :321.
  - `Screen::lines()/text()/has()` are at :507/:515/:520 and `Screen::replay` at :480.
  - Example walk: `m8_walk/clone.rs:37 start`, :65 `quit`.
- Captures: `crate::captures::check_shot(name, body)` (captures.rs:188) handles `MWR_UPDATE_SHOTS=1` (:181). The pty walks instead use `graft_fixtures::Captures::check(path, body)` (tour_walk.rs:174-195).
- Headless: `App::open(repo) -> (App, AppState)` (app_harness.rs:125). Then `app.press(&state, KeyPress)` :186, `app.key(&state, KeyCode)` :194, `app.pump(state, what, pred)`. Frames come from `captures::frame_at(&state, w, h, Some(Clock))` :113. Example: `m8_walk/journeys.rs:51-53`.

**3. Q1/Q2**: none of these are in `tests/it`. All are unit tests.
- Q1 rendering: `graft-ui journal_screen::tests::replaced::{replaced_row_label_and_ink:36, lost_rows_shas_line_up_at_every_width:89, legend_…:105, capture_replaced_for_docs_shots:135}`. The last one writes `m8-q1-journal-replaced-ui-{187x30,80x24}.txt`. These tests use synthetic `JournalRowView`s, not a real rebase.
- Q1 also: `graft update::journal::tests::replaced::*` (:67, :88, :118, :135) and `view::journal::tests::replaced::*` (:25, :49).
- Q2: `graft update::picker::tests::current_row::enter_on_the_current_branch_closes` (:18) and `capture_the_picker_on_the_current_row` (:61). The capture test writes `m8-q2-picker-current-row-ui-120x36.txt`.

**4. quit_waits**: the label is a hard-coded literal, `=== then \`y\`: Graft waits for git ===`, at `crates/graft/tests/it/quit_waits.rs:186`. `check_shot` writes it to `m8-quit-waits-80x24.txt`.
- Accessors: `graft_ui::graph_screen::APP_NAME` (header.rs:20, re-exported at graph_screen/mod.rs:41) and `graft::cli::COMMAND` (cli.rs:54). Both resolve to `Generation::CURRENT`, and `graft_core` is a dependency of the crate.

**5. Grep tests (graft `it` binary)**
- `rename_names::no_old_message_literal` (:229)
- `rename_names::no_old_name_in_user_visible_strings` (:260)
- `rename_names::user_docs_and_scripts_name_the_command` (:315)
- `rename_names::scripts::{no_old_env_name:123, scripts_run_the_command:153, no_old_package_spec:199}`
- `rename_opens::no_old_dir_literal` (:129)

**6. m8-tour `workri`**
- Line 3 of `m8-tour-187x50.txt` is ` ◇ workripractice · …`. `m8-tour-80x24.txt` shows ` ◇ workri…ctice-…` too.
- Code: `header_line` (`graft-ui/src/graph_screen/header.rs:74`). It draws `head` = ` ◇ <repo>` (:53-58), then `location_spans` pushes `Span::raw("  ")` before the path (:133-134). The path is `practice · <tmp> · a throwaway repo, not yours`, from `practised` (`graft/src/view/rows.rs:246`).
- So the code puts **two spaces** where the capture shows `ri`, and the 80x24 caption expects "a gap before the path".
- The pty capture comes from replaying the stream (`pty.rs` Replay, escape handling about :399-466) in `tour_walk.rs:174`. Stale cells from an earlier frame would explain it, but I **could not confirm** which side is at fault (app output or replayer).

**7. Contact sheet**
- Captions live in `docs/shots/m8-contact-sheet-captions.tsv`, one line each in the form `filename<TAB>caption`. It has 103 lines; the m8-q lines are 99-101 and quit-waits is 60.
- `m8_walk::contact_sheet::contact_sheet_captions_cover_every_m8_capture` (:96) enforces it. Every `docs/shots/m8-*.txt` (the sheet excepted) and every `m8/*.txt` must be captioned exactly once, with a non-empty caption that names a file on disk (:56-85).
- `every_part_of_the_m8_sheet_opens_with_colour_unverified` is at :132.
