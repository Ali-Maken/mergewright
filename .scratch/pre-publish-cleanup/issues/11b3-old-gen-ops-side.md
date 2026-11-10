# pre-publish: ops forgets earlier names (the move, the journal ref mapping and the old-tag readers go)

Status: draft
Blocked by: 11b2 (the app no longer calls `settle_names`)
Runs: alone (it changes pub ops signatures, and updates their app call sites)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the one behavioural risk (journals in his repos) is fenced by a test written first and a stop condition. The rest is deletion and ports to fixed consts.
Needs: 11b1's `on_disk` consts and predicates.
Crate set: `mergewright-ops` (src, tests). Plus the **call sites only** in `mergewright` of the ops signatures changed below.

## Decisions
**D1. The ops `rename` module goes whole.**
- That's every item in `crates/mergewright-ops/MAP.md` § rename: `settle_names`, `Settled`, `SettleOutcome`, `KEPT_ASIDE_DIR`, `OldNamesHold`, `MigrateError`, `LOCK_FILE`, `map_ref`/`map_entry`/`map_entries`, `description`.
- Its tests go too: `src/rename/tests.rs`, `tests/it/rename.rs`, `tests/it/rename/**`.
- `set_aside_generations.rs` and `journal_names.rs` go if their subject is earlier names; otherwise they're ported. Say which in NOTES.
- Before deleting `LOCK_FILE`, show with a grep that nothing outside `rename` takes that lock.

**D2. `OperationKind::Migrate` stays (core, no edit here). `undoable_kind` keeps it `false`.**
- Why: every repo he has opened since the rename holds a `Migrate` entry. Removing the kind would make those journals fail to parse, and that loses all of his undo history, not just the pre-rename part. That cost is beyond the ruling's "open each repo once".
- The kind spells no old name. Its words were stored at write time. Nothing writes it any more; 11b4 updates its doc line.

**D3. Journal lines are read as written. The ref mapping goes (ADR-0057 Amendment §7).**
- Cost: an entry from before the rename names a ref the move deleted, so `u` on it can't find its backup.
- That must fail **cleanly**: the entry still lists, and `u` refuses with words and writes nothing.
- Test-first:
  - Write `journal_reads_unmapped_ref_entry` before removing the mapping. The test entry names a backup under a prefix that isn't `on_disk::REF_PREFIX` (spell it `refs/elsewhere/`; never an old name).
  - Assert: the journal loads, the entry lists, and `u` on it returns the refusal and writes nothing.
  - Run it red/green as needed and quote the refusal's words in NOTES.

**D4. Only current names are read.**
- `LinkKind` reads only `on_disk::LINK_TRAILER`.
- Message and ref recognition use `is_own_message` / `is_own_ref`.
- Practice-repo removal accepts only the current prefix. His old practice folders stay on disk; list that cost in NOTES.
- The journal file path is `<common_dir>/<on_disk::DIR>/…`, no longer the names in force.

**D5. Signatures lose their `Generation` parameter** (below). Update the app call sites in the same commit.

## Stop conditions
- `journal_reads_unmapped_ref_entry` shows the journal failing to load, a panic, or any write. Stop with the failure; don't make the reader lenient.
- A test whose subject isn't earlier names fails after the port: report it.
- Anything needs git, core or fixtures changed: `blocked: needs <crate>`.

## Scope
In:
- the ops crate: delete `rename`; port snapshot (`snapshot.rs`, `chain.rs`, `plumbing.rs`, `raw.rs`, `temp_index.rs`), `tour` (`tour.rs`, `build.rs`, tests), `journal/file.rs`, `rewrite/saved.rs`, `apply/restart.rs`, `apply/record.rs`, `conflict/set.rs`, and `tests/it/support.rs` plus the other test files with `Generation` uses (scout counts);
- the app call sites of the changed signatures;
- `crates/mergewright-ops/MAP.md`: drop § rename, update `backup_ref_name`, `practice_prefix`, `LinkKind`.

Out: git (`on_disk_names`, `names_in_force`), the core table, fixtures marker (11b4). `test@graft.invalid` in `remote_ops.rs` and `aborted_fetch.rs` (12, identity). Other `graft` comments (11c).

## Signatures
```rust
// crates/mergewright-ops
pub fn backup_ref_name(worktree: &str, id: Ulid) -> RefName; // was (Generation, worktree, id); keep today's types for worktree/id
pub fn practice_prefix() -> String;                           // was (Generation); = format!("{}-practice-", on_disk::DIR)
// removed: rename::* (D1)
```
If today's parameter types for `worktree`/`id` differ from the ones above, keep today's. Only the `Generation` parameter goes.

## Acceptance (tick each with its test, inline)
- [ ] An unmapped old-ref entry lists, and `u` refuses cleanly ← `journal_reads_unmapped_ref_entry` (refusal words in NOTES)
- [ ] A `migrate` entry still parses and `u` passes over it ← existing undo test by name (grep `Migrate` in ops tests; if none, add `journal_keeps_migrate_entries`)
- [ ] No `Generation`, `settle_names` or `map_entr` left in ops ← `git grep -n -E 'Generation|settle_names|map_entr' -- crates/mergewright-ops` quoted
- [ ] Backup refs and practice roots use the current names ← existing snapshot/tour tests by name
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- Journal: reader and writer path; the time machine (`plan_revert_to`) over entries with unmapped refs, covered by the same test or one more.
- Undo: `undoable_kind`. Snapshot/backup ref names. Tour practice repos.
- App call sites of `backup_ref_name` / `practice_prefix`: grep quoted.
- Captures: none expected. If a walk capture changes, stop.
- Keymap: none.

## Read first
This ticket. 11b1's Signatures. `crates/mergewright-ops/MAP.md` § rename, § restore, the `undoable_kind`, `LinkKind`, `backup_ref_name` and `practice_prefix` lines. `crates/mergewright-core/MAP.md` `OperationKind` line. ADR-0057 Amendment §7 only.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~60k tokens: ~20 deleted files (cheap), ~15 ported, one new test.
