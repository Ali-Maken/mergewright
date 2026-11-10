# pre-publish: ops forgets earlier names (the move, the journal ref mapping and the old-tag readers go)

Status: ready
Blocked by: 11b2 (nothing outside ops calls `settle_names`; `names_in_force` always says `CURRENT`; the move tests are deleted)
Runs: alone (it changes pub ops signatures and updates their app call sites)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the one behavioural risk (journals in his repos) is fenced by a test written first and a stop condition. The rest is deletion and ports that, after 11b2, are behaviour-neutral.
Needs: 11b1's `on_disk` consts and predicates; 11b2's `names_in_force` → `CURRENT`.
Crate set: `mergewright-ops` (src, tests). Plus the **call sites only** in `mergewright` of the ops signatures changed below.

## Decisions
**D1. The ops `rename` module source goes whole.**
- That's every item in `crates/mergewright-ops/MAP.md` § rename: `settle_names`, `Settled`, `SettleOutcome`, `KEPT_ASIDE_DIR`, `OldNamesHold`, `MigrateError`, `LOCK_FILE`, `map_ref`/`map_entry`/`map_entries`, `description`.
- Its tests went in 11b2. Confirm by grep that nothing calls these items. Before deleting `LOCK_FILE`, show by grep that nothing outside `rename` takes that lock.

**D2. `OperationKind::Migrate` stays (core, no edit here). `undoable_kind` keeps it `false`.**
- Why: every repo he has opened since the rename holds a `Migrate` entry. Removing the kind would make those journals fail to parse, and that loses all of his undo history. That cost is beyond the ruling's "open each repo once".
- The kind spells no old name. Nothing writes it any more; 11b4 updates its doc line.

**D3. Journal lines are read as written. The ref mapping goes (ADR-0057 Amendment §7).**
- After 11b2 the mapping maps every earlier generation's refs to the current prefix.
- Removing it means an entry from before the rename names a ref the move deleted, so `u` can't find its backup.
- That must fail **cleanly**: the entry still lists, and `u` refuses with words and writes nothing.
- Test-first:
  - Write `journal_reads_unmapped_ref_entry` before removing the mapping. The test entry names a backup under a prefix that isn't `on_disk::REF_PREFIX` (spell it `refs/elsewhere/`; never an old name).
  - Assert: the journal loads, the entry lists, and `u` on it returns the refusal and writes nothing.
  - Quote the refusal's words in NOTES.

**D4. Write and read paths use the consts, not the handle.**
- Every `repo.on_disk_names().<x>()` in ops becomes `on_disk::<X>`. That's behaviour-neutral, since it has returned `CURRENT` since 11b2. The places:
  - `journal/file.rs` `journal_path`;
  - `apply/record.rs`, `apply/restart.rs`;
  - `snapshot.rs`, `chain.rs`, `plumbing.rs`, `raw.rs`, `temp_index.rs`;
  - `rewrite/saved.rs`, `conflict/set.rs`;
  - `tour.rs`, `tour/build.rs`.
- `LinkKind` reads only `on_disk::LINK_TRAILER`.
- Message and ref recognition use `is_own_message` / `is_own_ref`.
- Practice-repo removal accepts only the current prefix. His old practice folders stay on disk; NOTES states the cost.
- After this, no ops code calls `on_disk_names()`. That's what lets 11b4 delete it.

**D5. Signatures lose their `Generation` parameter** (below), including any ops fn the app calls with `Generation::CURRENT` (11b2's NOTES list them, e.g. `open_as`, if it's ops'). Update the app call sites in the same commit.

## Stop conditions
- `journal_reads_unmapped_ref_entry` shows the journal failing to load, a panic, or any write. Stop with the failure; don't make the reader lenient.
- 11b2's `fresh_repo_names::journal_and_backups_use_current_names` goes red: stop.
- A test whose subject isn't earlier names fails after the port: report it.
- Anything needs git, core or fixtures changed: `blocked: needs <crate>`.

## Scope
In:
- the ops crate: delete `src/rename*`; the D4 ports; `tests/it/support.rs` and the other test files still naming `Generation` or `on_disk_names`;
- the app call sites of changed signatures;
- `crates/mergewright-ops/MAP.md`: drop § rename, update `backup_ref_name`, `practice_prefix`, `LinkKind`.

Out: git (`names_in_force`, `on_disk_names` definitions), the core table, the fixtures marker (11b4). `test@graft.invalid` (12). Other `graft` comments (11c).

## Signatures
```rust
// crates/mergewright-ops
pub fn backup_ref_name(worktree: &str, id: Ulid) -> RefName; // was (Generation, worktree, id); keep today's types for worktree/id
pub fn practice_prefix() -> String;                           // was (Generation); = format!("{}-practice-", on_disk::DIR)
// removed: rename::* (D1)
```
If today's parameter types differ from the ones above, keep today's. Only the `Generation` parameter goes.

## Acceptance (tick each with its test, inline)
- [ ] An unmapped old-ref entry lists, and `u` refuses cleanly ← `journal_reads_unmapped_ref_entry` (refusal words in NOTES)
- [ ] A `migrate` entry still parses and `u` passes over it ← existing undo test by name (grep `Migrate` in ops tests; if none, add `journal_keeps_migrate_entries`)
- [ ] Fresh repos still write under the current names ← `fresh_repo_names::journal_and_backups_use_current_names` (11b2's)
- [ ] No `Generation`, `on_disk_names`, `settle_names` or `map_entr` left in ops ← `git grep -n -E 'Generation|on_disk_names|settle_names|map_entr' -- crates/mergewright-ops` quoted, empty
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- Journal: reader and writer path; the time machine (`plan_revert_to`) over entries with unmapped refs (same test or one more).
- Undo: `undoable_kind`. Snapshot/backup ref names. Tour practice repos.
- App call sites of the changed ops fns: grep quoted.
- Captures: none expected. If a walk capture changes, stop.
- Keymap: none.

## Read first
This ticket. 11b2's Signatures and NOTES. 11b1's Signatures. `crates/mergewright-ops/MAP.md` § rename, § restore, the `undoable_kind`, `LinkKind`, `backup_ref_name` and `practice_prefix` lines. `crates/mergewright-core/MAP.md` `OperationKind` line. ADR-0057 Amendment §7 only.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~55k tokens: the `rename` src deleted (cheap), ~13 files ported, one new test.
