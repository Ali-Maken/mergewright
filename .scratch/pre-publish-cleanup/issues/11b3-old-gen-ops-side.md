# pre-publish: ops forgets earlier names (the move, the journal ref mapping and the old-tag readers go)

Status: draft (amended twice 2026-11-10: after 11b2, names in force are always `CURRENT` and the move tests are gone. Pass 3: the D3 fence is a parity test, not "writes nothing")
Blocked by: 11b2
Runs: alone (it changes pub ops signatures and updates their app call sites). **Continue on branch `ppc-11b3`** (worktree `/Users/devuser/dev/sandbox/gitter/.claude/worktrees/ppc-11b3`, commit c9ce6cea). Read the NOTES in the worktree's copy of this ticket first.
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: the one behavioural risk (journals in his repos) is fenced by a parity test pinned before the change. The rest is deletion and ports that, after 11b2, are behaviour-neutral.
Needs: 11b1's `on_disk` consts and predicates; 11b2's `names_in_force` → `CURRENT`.
Crate set: `mergewright-ops` (src, tests). Plus the **call sites only** in `mergewright` of the ops signatures changed below.

## Prior attempt (builder, 2026-11-10; reproduced by the conductor)
The first fence demanded that "`u` refuses and writes nothing". That was never today's behaviour; the ref mapping never touches `refs/elsewhere/`. The red test `tests/it/journal_unmapped.rs` (not registered in `tests/it/main.rs`) shows two cases:
1. A Stage entry whose backup ref is deleted: `u` still undoes it, from the oids the entry records, not from the ref.
2. A Discard entry whose objects are gc-pruned: the journal loads and the entry lists. `u` returns `Err` ("bad object … rolled back part-way"), but first it appends a pending undo line and makes a new backup ref.

**Conductor ruling:** undo isn't fixed in this pass. Case 2 is filed separately, as a question. This ticket only proves that removing the mapping and the old-tag readers changes nothing about those cases.

## Decisions
**D1. The ops `rename` module source goes whole.**
- That's every item in `crates/mergewright-ops/MAP.md` § rename: `settle_names`, `Settled`, `SettleOutcome`, `KEPT_ASIDE_DIR`, `OldNamesHold`, `MigrateError`, `LOCK_FILE`, `map_ref`/`map_entry`/`map_entries`, `description`.
- Its tests went in 11b2. Confirm by grep that nothing calls these items. Before deleting `LOCK_FILE`, show by grep that nothing outside `rename` takes that lock.

**D2. `OperationKind::Migrate` stays (core, no edit here). `undoable_kind` keeps it `false`.**
- Why: every repo he has opened since the rename holds a `Migrate` entry. Removing the kind would make those journals fail to parse, and that loses all of his undo history.
- The kind spells no old name. Nothing writes it any more; 11b4 updates its doc line.

**D3. Journal lines are read as written. The ref mapping goes (ADR-0057 Amendment §7). The fence is parity.**
- Turn `journal_unmapped.rs` into a **green characterization test**, registered in `tests/it/main.rs`. Commit it **before** any D1/D3/D4 change, green on the code as it is.
- It pins today's result for both cases from "Prior attempt":
  - case 1: the result is `Ok` and the files and index after `u` are the same as today's;
  - case 2: the `Err` variant and its words, the journal lines appended (count and kinds), and the refs created under `on_disk::REF_PREFIX` (count).
- Pin observed values, not desired ones. The test entry names refs under a prefix that isn't `on_disk::REF_PREFIX` (spell it `refs/elsewhere/`; never an old name).
- A comment at the top of the file names the open question: "undo of a pruned entry leaves a pending line and a backup ref before failing; question filed, not fixed here".
- The test stays green, **unchanged**, after the mapping and the old-tag readers go. Any edit to its expected values after that first commit is a stop.

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
- After this, no ops code calls `on_disk_names()`.

**D5. Signatures lose their `Generation` parameter** (below), including any ops fn the app calls with `Generation::CURRENT` (11b2's NOTES list them). Update the app call sites in the same commit.

## Stop conditions
- The parity test's expected values have to change after its first commit, i.e. the removal changed what happens. Stop with the diff.
- The parity test can't be made green on today's code without changing product code: stop. The fence pins today's behaviour; it doesn't fix it.
- 11b2's `fresh_repo_names::journal_and_backups_use_current_names` goes red: stop.
- A test whose subject isn't earlier names fails after the port: report it.
- Anything needs git, core or fixtures changed: `blocked: needs <crate>`.

## Scope
In:
- the ops crate: delete `src/rename*`; the D4 ports; the parity test; `tests/it/support.rs` and the other test files still naming `Generation` or `on_disk_names`;
- the app call sites of changed signatures;
- `crates/mergewright-ops/MAP.md`: drop § rename, update `backup_ref_name`, `practice_prefix`, `LinkKind`.

Out:
- **Changing undo's behaviour on pruned or ref-less entries** (separate question).
- git (`names_in_force`, `on_disk_names` definitions), the core table, the fixtures marker (11b4).
- `test@graft.invalid` (12). Other `graft` comments (11c).

## Signatures
```rust
// crates/mergewright-ops
pub fn backup_ref_name(worktree: &str, id: Ulid) -> RefName; // was (Generation, worktree, id); keep today's types for worktree/id
pub fn practice_prefix() -> String;                           // was (Generation); = format!("{}-practice-", on_disk::DIR)
// removed: rename::* (D1)
// tests/it/journal_unmapped.rs (registered in tests/it/main.rs)
#[test] fn ref_gone_entry_undoes_as_today();
#[test] fn pruned_entry_fails_as_today();
```

## Acceptance (tick each with its test, inline)
- [ ] Parity test green on the code before the change; its commit hash in NOTES ← `journal_unmapped::ref_gone_entry_undoes_as_today`, `pruned_entry_fails_as_today`
- [ ] The same tests green after the change, with expected values unchanged ← same tests; `git diff <parity commit> -- crates/mergewright-ops/tests/it/journal_unmapped.rs` empty
- [ ] A `migrate` entry still parses and `u` passes over it ← existing undo test by name (grep `Migrate` in ops tests; if none, add `journal_keeps_migrate_entries`)
- [ ] Fresh repos still write under the current names ← `fresh_repo_names::journal_and_backups_use_current_names` (11b2's)
- [ ] No `Generation`, `on_disk_names`, `settle_names` or `map_entr` left in ops ← `git grep -n -E 'Generation|on_disk_names|settle_names|map_entr' -- crates/mergewright-ops` quoted, empty
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- Journal: reader and writer path; the time machine (`plan_revert_to`) over the same two entries (same parity file, if it reaches them with no new fixture).
- Undo: `undoable_kind`. Snapshot/backup ref names. Tour practice repos.
- App call sites of the changed ops fns: grep quoted.
- Captures: none expected. If a walk capture changes, stop.
- Keymap: none.

## Read first
This ticket, then the NOTES in the worktree's copy. 11b2's Signatures and NOTES. 11b1's Signatures. `crates/mergewright-ops/MAP.md` § rename, § restore, the `undoable_kind`, `LinkKind`, `backup_ref_name` and `practice_prefix` lines. `crates/mergewright-core/MAP.md` `OperationKind` line. ADR-0057 Amendment §7 only.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~50k tokens remaining: the parity test (exists, red → pin), the `rename` src deleted (cheap), ~13 files ported.
