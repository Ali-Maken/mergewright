# pre-publish: repos always open on the current names; the app stops moving old-name folders; app, ui, term ported

Status: draft (amended 2026-11-10 after the first build stopped: see "Prior attempt")
Blocked by: 11b1. **And the user's prerequisite:** he has opened each of his repos once with today's `mwr` (00-index, "Before 11b2 merges"). The conductor confirms it before merging; the builder doesn't wait on it.
Runs: alone, before 11b3. **Continue in the existing worktree** `/Users/devuser/dev/sandbox/gitter/.claude/worktrees/ppc-11b2`: its WIP is uncommitted. Don't start a fresh one, and don't discard it.
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: one fn changes its answer behind a test written first; the rest is deletion and mechanical ports.
Needs: 11b1's `on_disk` consts and predicates (as merged; the WIP uses them).
Crate set:
- `mergewright` (src, tests, examples), `mergewright-ui`, `mergewright-term`;
- **plus** `mergewright-git` `src/repo/names.rs` and its tests only;
- **plus** `mergewright-ops` **test files only** (deleting move tests, D5, and one new test, D1).

## Prior attempt (builder, 2026-11-10; finding verified by the conductor)
- Premise failed. `mergewright-git/src/repo/names.rs:37` `names_in_force` returns the oldest generation when no `migrated` marker exists. ops writes by `on_disk_names()`: `journal_path` at `ops/src/journal/file.rs:392`; backups in record.rs, restart.rs, snapshot.rs, rewrite/saved.rs, conflict/set.rs, tour.rs.
- Only `settle_names` writing the marker put a fresh repo on current names. So "open without settling" alone would send every new repo to the old names.
- **Hence D1, and why D1 and D2 must land in one commit.** Without D1, D2 sends new repos to the old names. Without D2, D1 leaves `settle_names` running in a mode no test covers, and the move tests red.
- WIP already in the worktree (uncommitted, not compiled):
  - `repos.rs`: no settle, plus a new red test `repos::tests::opens_without_settling`;
  - `places.rs`; `lib.rs` (`rename` gone);
  - `config/load.rs` + `mod.rs` (folder module and `ConfigWarning::ConfigFolder` removed);
  - `first_launch.rs`, `recent.rs`;
  - jobs `operate`/`undo` (`open_as(.., Generation::CURRENT)`);
  - `CURRENT_DIR` → `DIR` and `Generation::CURRENT.x()` → `on_disk::X` across ~45 files.
- Read its NOTES in the worktree's copy of this ticket first.

## Decisions
**D1. The names in force are always the current ones (git, test-first).**
- `names_in_force(common_dir)` returns `Ok(Generation::CURRENT)` without looking at any marker. `RepoHandle`/`SharedRepo::on_disk_names` therefore return `CURRENT`.
- The signatures stay; 11b4 deletes them.
- Tests, written red first:
  - `names::tests::no_marker_is_current` and `old_marker_is_still_current`: an earlier generation's dir and marker present (build the dir name from a test-local value, not a literal old name) → `CURRENT`.
  - ops `tests/it/fresh_repo_names.rs::journal_and_backups_use_current_names`: a fresh repo with no marker; one journaled operation with a backup; the journal is under `<common_dir>/<on_disk::DIR>/`, every backup ref is under `on_disk::REF_PREFIX`, and the only app dir created under `.git` is `on_disk::DIR`.
- Old tests in `names.rs`'s tests that pin marker-based answers go (D5).

**D2. Repos open as they are.**
- The app `rename` module goes (`open_settled`, `Settlement`, `words`). Callers open the repository directly. The status-row words for an unfinished move go with it.
- `ops::rename::settle_names` is no longer called anywhere outside ops. It stays defined, uncalled and untested, until 11b3.

**D3. The config folder is read from `<config home>/<on_disk::DIR>/` only.**
- `config::folder` and its tests are deleted, along with `ConfigWarning::ConfigFolder`. His old config folders stay on disk.

**D4. Port, don't redesign.**
- `Generation::CURRENT.<x>()` → `on_disk::<X>`; `CURRENT_DIR` → `on_disk::DIR`; "is it ours" → 11b1's predicates.
- Calls into another crate's pub fn that still takes a `Generation` (e.g. `open_as`) stay. 11b3 changes the ops ones and 11b4 the git and core ones; list each in NOTES.

**D5. Every test of the move goes, in every crate, in this commit** (D1 makes them all red).
- App:
  - delete `rename_status.rs`, `rename_opens.rs`, `m8_walk/mergewright/older.rs`, `tributary.rs`;
  - first move `crates_dir` / `production_files` out of `rename_opens.rs` into a shared test helper (`rename_names.rs` and `rename_docs.rs` use them);
  - `m8_walk/mergewright.rs` keeps `diverged`, `journal_replaced` and `picker_enter_current`, so move `git`, `git_at`, `init` and `journal` from `older.rs` into `walk.rs` first.
- ops (tests only):
  - delete `src/rename/tests.rs` (remove its `mod tests;` line in `src/rename.rs`; the only src edit allowed in ops), `tests/it/rename.rs`, `tests/it/rename/**`;
  - any other ops test that goes red under D1 whose subject is earlier names (candidates: `set_aside_generations.rs`, `journal_names.rs`, `journal_two_writers.rs`, `aborted_fetch.rs`). Name each one deleted, with its reason.
- Tests that use `.older()` as a guard source (`clock_pin`, `release_names`, `rename_names/scripts`, `update/palette/names_tests`, and `m3_walk`'s `of_message` → 11b1's predicate): switch to a test-local old-names list, the same way `rename_names.rs` does (11b1).
- Deleted walks' captures go once `scripts/shot-drift.py` shows them writerless. TESTING.md entries pointing at them go too; list both.

## Stop conditions
- A test whose subject isn't earlier names goes red after D1/D2: report it, don't change its expectation.
- `journal_and_backups_use_current_names` can't be made green by D1 alone, i.e. some write path doesn't go through `on_disk_names()`. Stop and name the path.
- Anything needs ops `src` beyond the `mod tests;` line, or core or fixtures: `blocked: needs <crate>`.

## Scope
In: the crate set; orphaned captures and TESTING.md entries; MAP.md:
- `mergewright`: drop § rename and § config::folder; `COMMAND` / config lines name the consts.
- `mergewright-ui`: `APP_NAME`, `source_word`.
- `mergewright-git`: the `names_in_force` / `on_disk_names` lines now say "always `CURRENT`; removed in 11b4".

Out: the ops `rename` module source (11b3); ops write paths (11b3); the table (11b4); other `graft` hits (11a/11c).

## Signatures
```rust
// crates/mergewright-git/src/repo/names.rs: signature unchanged, answer fixed
pub fn names_in_force(common_dir: &Path) -> io::Result<Generation>; // always Ok(Generation::CURRENT)
// removed from crates/mergewright: rename::{open_settled, Settlement, words}, config::folder, ConfigWarning::ConfigFolder
```

## Acceptance (tick each with its test, inline)
- [ ] No marker or an old marker → current names ← `names::tests::no_marker_is_current`, `old_marker_is_still_current`
- [ ] A fresh repo's journal and backups land under the current names only ← `fresh_repo_names::journal_and_backups_use_current_names`
- [ ] A repo opens without settling; no `Migrate` entry ← `repos::tests::opens_without_settling`
- [ ] Config read from `<config home>/mergewright/` ← `config::load` test by name
- [ ] No `settle_names` / `open_settled` caller outside ops `src/rename` ← `git grep -n -E 'settle_names|open_settled' -- crates` quoted
- [ ] `Generation` in the app/ui/term crates only in calls 11b3/11b4 change ← `git grep -n Generation -- crates/mergewright crates/mergewright-ui crates/mergewright-term` quoted, each line tagged
- [ ] Deleted tests listed with reasons; deleted captures writerless ← NOTES; `scripts/shot-drift.py`
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- Every ops write path through `on_disk_names()` (covered by the new ops test).
- Fixtures still write the marker. It's harmless now (D1 ignores it); 11b4 removes it. Fixture caches are unchanged.
- Startup (`RepoStore`), crash reports, `cli`/man `COMMAND`, jobs `operate`/`undo`.
- ui journal legend/view and header; `term/input/gate.rs`.
- m8 walk captures and the contact sheet (list; don't rebuild).
- Keymap: none. Journal/undo: existing `Migrate` entries untouched (11b3 keeps the kind).

## Read first
This ticket, then the NOTES in the worktree's copy. 11b1's Signatures. `crates/mergewright-git/MAP.md` § repo (the `names_in_force` / `on_disk_names` lines). `crates/mergewright/MAP.md` § config, § config::folder, § rename, § repos.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~55k tokens remaining: one fn plus two tests in git, one ops test, test deletions (cheap), the helper moves, the guard-test ports, compiling the WIP.
