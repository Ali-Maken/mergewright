# Mergewright 4: every environment name is `MWR_*`

Status: done
Blocked by: 50
Build with: **Sonnet**. A token rename (`TRIB_` → `MWR_`) held by one grep test; no behaviour changes.
Brownfield: 27b's NOTES are the scout (same 45 names, one prefix later).
Needs: `mergewright-app-words` merged (same test files; sequential). Alone: it touches tests in every crate.

## Read first
- ADR-0057 Amendment (2026-10-11) §1 (Environment row), §2 (env exception), §12 (dev caches)
- `issues/27b-rename-env-names.md`: Decisions, Done, NOTES
- `crates/graft-core/MAP.md` `ASKPASS_SOCKET_VAR`; `crates/graft-fixtures/MAP.md` `UPDATE_VAR`; `crates/graft/MAP.md`
  `## cli`, `## trace`

## Decisions
- **Every `TRIB_` token → `MWR_`**, identifier or string, over 27b's file set: `crates/**` (src, tests, examples,
  benches), `scripts/**` (the `.patch` files too, whole, as 27b did), `.scratch/*/pty-walk*.py`, `docs/shots/*.py`,
  `crates/*/MAP.md`, `ARCHITECTURE.md`, README.md and TESTING.md instruction lines (records keep their names: 27c's
  `27c-testing-kept.txt` classes).
- **Old names are not read** (§1): neither `GRAFT_*` nor `TRIB_*` shipped.
- **README:** the line saying `GRAFT_*` variables are no longer read is deleted, not extended: nothing shipped under
  either, so a user has nothing to unlearn.
- **Grep test from the table:** `rename_names::no_old_env_name` fails on any `Generation::CURRENT.older()`
  `env_prefix()` (`GRAFT_`, `TRIB_`) over the file set above. Allow-list empty, or each entry with its reason.
- **Dev caches** (§12): `scripts/clone-git-git.sh`'s `~/.cache/tributary/git-git.git` → `~/.cache/mergewright/…`;
  the script moves the old dir once with `mv` when the new one is absent and the old one exists, so git/git is not
  re-cloned. `TRIB_GIT_GIT_DIR` → `MWR_GIT_GIT_DIR`. Caches that say `graft` stay (code names).
- `.claude/**`, CLAUDE.md, PROMPT.md: grep them for `TRIB_`; any hit goes to NOTES for `mergewright-protected-lines`,
  not edited (27b found none for `GRAFT_`).

## Scope
In: the token rename; the README line; the cache move in `clone-git-git.sh`; TestBackend captures that print a name;
the grep test; MAP/ARCHITECTURE lines. Out: everything else in README/TESTING.md (`mergewright-docs-sweep`), pty
captures, history docs.

## Signatures
```rust
// graft-core::on_disk (value changes, name unchanged)
pub const ASKPASS_SOCKET_VAR: &str = "MWR_ASKPASS_SOCKET";
```
Values change, names unchanged: `UPDATE_VAR = "MWR_UPDATE_SHOTS"` (graft-fixtures), `MWR_PERF_TRACE`,
`MWR_FRAME_TRACE`, `MWR_NOW`, `MWR_PRACTICE_EPOCH`, `MWR_FIXTURES_DIR`, `MWR_INIT_TARGET`, and the rest of 27b's 45.

## Acceptance (each box: the test named at tick time)
- [x] `rename_names::no_old_env_name`: no `GRAFT_` or `TRIB_` (from the table) in the file set; allow-list count in NOTES.
- [x] Askpass: `askpass_mode::binary_answers_over_socket`, `…binary_exits_1_with_nothing_on_stdout_when_cancelled`,
      `…binary_exits_2_naming_the_error_when_graft_is_gone`, `graft-git run::tests::askpass_env_is_set` green; grep for
      `"MWR_ASKPASS` over `crates/` returns `on_disk.rs` only (quoted).
- [x] Old name not read: `clock_pin::the_old_now_name_is_not_read` checks both `GRAFT_NOW` and `TRIB_NOW` set alone.
- [x] `clone-git-git.sh`: a dry run against a temp `HOME` with only `~/.cache/tributary/git-git.git` (an empty bare
      repo) moves it and does not clone (output to file, last line quoted). No network.
- [x] Captures: `MWR_UPDATE_SHOTS=1` run, then a plain run, green. Drift summary quoted
      (`.scratch/m8-polish-and-ship/mwr-env-shot-drift.txt`); drift-files 0 expected (a shorter name); any drift listed.
- [x] `27-shots-left.txt` re-run: count in NOTES.

## Touches
- **Callers:** every test or example reading or setting an env var; askpass child env (graft-git `run.rs`); trace,
  clock pin, practice epoch, fixtures dir, init target; every script exporting one.
- **Screens / keymap / journal / undo:** none.
- **Captures:** TestBackend ones that print an env name; pty ones to `mergewright-recaptures`' list.
- **Docs:** README (env lines + the deleted line), TESTING.md instruction lines naming an env var, MAP lines,
  ARCHITECTURE.md, `docs/man/mwr.1` regenerated (its env lines).
- **Crates:** all (tests); graft-core `on_disk`; graft-fixtures `UPDATE_VAR`.
- **His machine:** a dev who exports `TRIB_*` in a shell rc loses it silently. Listed in the report for the conductor.
- Each is checked by the named grep or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. Capture runs to files; summary lines only.

## Size
One context, ~40k tokens (sed-shaped).

## NOTES
- Scout: none dispatched; 27b's NOTES served (ticket says so), plus direct greps. No compaction.
- Ticks: `rename_names::no_old_env_name` now reads `Generation::CURRENT.older().env_prefix()` (`GRAFT_`, `TRIB_`) over crates/**, scripts/**, pty-walk + `docs/shots/*.py`, ARCHITECTURE.md, README.md; **allow-list 1 entry** (`graft-core/src/on_disk` + its tests: the table spells each prefix). TESTING.md is not swept by the test (dated records keep `GRAFT_UPDATE_SHOTS`, ~lines 2310/2318); its 16 `TRIB_` instruction lines renamed. Askpass four tests green; `grep '"MWR_ASKPASS' crates/` → `on_disk.rs:173` only. `clock_pin::the_old_now_name_is_not_read` loops both old prefixes, each set alone.
- clone-git-git.sh dry run (fake `git` on PATH logging args, temp HOME, no network): `.scratch/m8-polish-and-ship/mwr-clone-git-git-dry-run.txt`, last line `clone called: 0`; old dir gone, new is the bare repo. Move skipped when `MWR_GIT_GIT_DIR` is set.
- Captures: update run, then plain run. `shot-drift.py`: `changed 10 · name-only 2 · drift-files 8 · png 0`. Not 0: 7 m5-ui TestBackend captures' header comment (`MWR_UPDATE_SHOTS`), and m8-tour-step1 header + caption path now `mergewright-practice-` (the folded-in finding; path cut from the left shifts by one char). Plain run green after the ratchet: over_400 raised tour/tests.rs 846→893, clock_pin.rs 443→448, tour_walk.rs 928→929.
- `27-shots-left.txt` re-run: **450** (unchanged).
- Folded in (conductor; review m8-mergewright-50 Major 1): `PRACTICE_PREFIX` const → `graft_ops::tour::practice_prefix(Generation) -> String` (`<dir>-practice-`); new roots use CURRENT; the removal guard accepts every generation's prefix. Red first: `tour::tests::a_new_practice_folder_takes_the_current_name`, `…removal_accepts_every_earlier_generations_folder`. Grep test: `says_old_name` also matches `old.dir()` as a word; `the_word_test_reads_words_only` red then green on `"tributary-practice-"` (its `tributary-ish` case became `tributary_ish`). Closes gap M8/50-practice-temp-prefix.
- `.claude/**`, CLAUDE.md, PROMPT.md: 0 `TRIB_` hits.
- His machine: a `TRIB_*` export in a shell rc is now silently ignored.
