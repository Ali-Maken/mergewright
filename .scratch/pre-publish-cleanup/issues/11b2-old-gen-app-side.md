# pre-publish: the app stops moving old-name folders; app, ui, term ported to the current-only names

Status: draft
Blocked by: 11b1. **And the user's prerequisite:** he has opened each of his repos once with today's `mwr` (00-index, "Before 11b2 merges"). The conductor confirms it before merging; the builder doesn't wait on it.
Runs: alone, before 11b3. 11b3 deletes `settle_names`, which this crate calls until this ticket lands.
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: deletions plus mechanical ports to 11b1's consts. What to delete and what to keep is decided below.
Needs: `mergewright_core::on_disk::{DIR, TITLE, COMMAND, ENV_PREFIX, REF_PREFIX, LINK_TRAILER, BACKUP_EMAIL, MESSAGE_TAG, is_own_ref, is_own_message}` (11b1)
Crate set: `mergewright` (src, tests, examples), `mergewright-ui`, `mergewright-term`. Nothing in ops, git, core or fixtures.

## Decisions
**D1. Repos open as they are.**
- The app `rename` module goes (MAP: `open_settled`, `Settlement`, `words`). Callers open the repository directly, the way `open_settled` did after a clean move.
- The status-row words for an unfinished move go with it.
- `ops::rename::settle_names` and `RepoHandle::on_disk_names` are no longer called from this crate. They stay defined for now: 11b3 deletes `settle_names`, 11b4 deletes `on_disk_names`.

**D2. The config folder is read from `<config home>/<on_disk::DIR>/` only.**
- `config::folder` (the one-time move from the earlier folders) is deleted, with `config/folder/tests.rs`.
- His old config folders stay on disk, untouched.

**D3. Port, don't redesign.**
- Every `Generation::CURRENT.<x>()` in the crate set → `on_disk::<X>`. `CURRENT_DIR` → `on_disk::DIR`.
- `of_ref`/`of_message` used as "is it ours" → 11b1's predicates.
- A call into another crate's pub fn that still takes a `Generation` stays as it is. The ticket that owns the fn changes it along with its signature: 11b3 for ops, 11b4 for git and core. List each in NOTES.

**D4. Tests of the move go; tests of current names stay, ported.**
- Delete:
  - `tests/it/rename_status.rs` and `rename_opens.rs`;
  - `tests/it/m8_walk/mergewright/older.rs` and `tributary.rs`, and the parts of `m8_walk/mergewright.rs` / `walk.rs` that set up an old-name repo;
  - every other test whose subject is the move. Name each in NOTES with the reason.
- Port:
  - `release_names.rs`: the Cargo bin and package names pinned to `on_disk::COMMAND` / `DIR`;
  - `rename_names/scripts.rs`;
  - `clock_pin`, `m3_walk`, `gallery`, `tour_walk`, `quit_waits`, `repo_truth`, `radar_bookkeeping`, `m7_walk/*`, `view/journal/tests.rs`, `update/palette/names_tests.rs`, `cli/docs_tests.rs`: each holds 1–3 `Generation` uses (scout counts).
- A deleted walk's captures go too, once `scripts/shot-drift.py` shows them writerless. List each deleted capture in NOTES.
- So do the TESTING.md checklist entries that point at those captures. Edit only those entries.

## Stop conditions
- A test that isn't about the move fails after the port: report, don't change its expectation.
- Something in the crate set needs a change in ops, git, core or fixtures: `blocked: needs <crate>`, naming the signature.

## Scope
In: the crate set above; captures and TESTING.md entries orphaned by deleted walks; `crates/mergewright/MAP.md` (drop § rename and § config::folder; the § cli `COMMAND` and § config lines name `on_disk::COMMAND` / `on_disk::DIR`); `crates/mergewright-ui/MAP.md` (`APP_NAME`, `source_word` lines).
Out: the guards' `OLD_NAMES` (11b1 has it). `rename_docs.rs` (11a). Every other `graft` hit (11a/11c). Docs prose describing the move (11a).

## Signatures
```rust
// removed from crates/mergewright: rename::{open_settled, Settlement, words}, config::folder (whole module)
// RepoStore opens without settling; its pub signature is unchanged unless it exposed Settlement (then the field goes)
```

## Acceptance (tick each with its test, inline)
- [ ] No `Generation` left in the crate set except calls into fns 11b3/11b4 change, all listed ← `git grep -n Generation -- crates/mergewright crates/mergewright-ui crates/mergewright-term` quoted in NOTES
- [ ] No `settle_names` / `on_disk_names` / `open_settled` caller left ← same grep, quoted
- [ ] A fresh repo opens and the journal records no `Migrate` entry ← an existing open test by name, or a new `repos::tests::opens_without_settling`
- [ ] Config is read from `<config home>/mergewright/` ← `config::load` test by name
- [ ] Every deleted capture was writerless, and no remaining capture lost its writer ← `scripts/shot-drift.py` output in NOTES
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- Startup path (`RepoStore`, `repos.rs`), crash reports (`crash.rs`: already new-dir-only per ADR-0057 §8, so a port only), `cli`/man page `COMMAND`.
- Jobs `operate.rs` and `undo.rs` (one `Generation` use each).
- Journal screen legend/view (ui), the header `APP_NAME` (ui), and `term/input/gate.rs` (env prefix).
- m8 walk captures and the contact sheet, if one embeds them. List them; old sheets aren't rebuilt (user rule).
- Keymap: none. Journal/undo: `Migrate` entries already in his journals are untouched here (11b3 keeps the kind).

## Read first
This ticket. 11b1's Signatures. `crates/mergewright/MAP.md` § cli, § config, § config::folder, § rename, § repos. `crates/mergewright-ui/MAP.md` lines naming `Generation`. `scouts/11-codename-hits.md` § Old-generation.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh` (walks deleted and ported).

## Size
~60k tokens. Mostly deletions. If the port list goes past ~30 files, commit at the src/tests seam and report.
