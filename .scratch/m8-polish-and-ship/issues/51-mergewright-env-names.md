# Mergewright 4: every environment name is `MWR_*`

Status: ready
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
- [ ] `rename_names::no_old_env_name`: no `GRAFT_` or `TRIB_` (from the table) in the file set; allow-list count in NOTES.
- [ ] Askpass: `askpass_mode::binary_answers_over_socket`, `…binary_exits_1_with_nothing_on_stdout_when_cancelled`,
      `…binary_exits_2_naming_the_error_when_graft_is_gone`, `graft-git run::tests::askpass_env_is_set` green; grep for
      `"MWR_ASKPASS` over `crates/` returns `on_disk.rs` only (quoted).
- [ ] Old name not read: `clock_pin::the_old_now_name_is_not_read` checks both `GRAFT_NOW` and `TRIB_NOW` set alone.
- [ ] `clone-git-git.sh`: a dry run against a temp `HOME` with only `~/.cache/tributary/git-git.git` (an empty bare
      repo) moves it and does not clone (output to file, last line quoted). No network.
- [ ] Captures: `MWR_UPDATE_SHOTS=1` run, then a plain run, green. Drift summary quoted
      (`.scratch/m8-polish-and-ship/mwr-env-shot-drift.txt`); drift-files 0 expected (a shorter name); any drift listed.
- [ ] `27-shots-left.txt` re-run: count in NOTES.

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
