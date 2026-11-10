# pre-publish: delete the Generation table, the names-in-force read and the fixtures' rename marker

Status: draft (amended 2026-11-10: `names_in_force` already answers `CURRENT` since 11b2; ops no longer calls `on_disk_names` since 11b3. Pass 3: the D5 cost wording follows 11b3's parity finding)
Blocked by: 11b3
Runs: alone (core and git signatures change; call sites in every crate)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: once 11b2/11b3 have landed, nothing behavioural is left. This is deletion, with the compiler finding each call site.
Needs: 11b1's `on_disk` consts.
Crate set: `mergewright-core`, `mergewright-git`, `mergewright-fixtures`, plus **call sites only** in any crate that still names a removed item. The app and git ones are listed in 11b2/11b3 NOTES (e.g. any remaining `on_disk_names()` or `open_as(.., Generation::CURRENT)`).

## Decisions
**D1. Removed:**
- from `core::on_disk`: `Generation` (all of it), `GENERATIONS`, `CURRENT_DIR` (callers → `DIR`), `MIGRATED_MARKER`, and the 11b1 equality test;
- from `mergewright-git`: `names_in_force` (`repo/names.rs`), `RepoHandle::on_disk_names`, `SharedRepo::on_disk_names`, and 11b2's `no_marker_is_current` / `old_marker_is_still_current`, which go with the fn. Callers use `on_disk::DIR` / `REF_PREFIX`.
  - If `OpenError::Io` existed only for that stat (grep), it goes; otherwise it stays.
- `RefName::is_mergewright_internal` stays. It is true only under `on_disk::REF_PREFIX`.
- Two ops tests **stay**, unchanged: 11b2's `fresh_repo_names::journal_and_backups_use_current_names`, and 11b3's parity tests in `journal_unmapped.rs`.

**D2. Fixtures stop writing the rename marker.**
- `.git/mergewright/migrated` (private `settled.rs`) goes, and it leaves the three cache keys. Nothing has read it since 11b2.
- The keys change, so every fixture cache rebuilds once (machine time).
- Prove no capture changes: `scripts/walks.sh` green, with `git status docs/shots` clean.

**D3. The guard skip goes.**
- `rename_names.rs` no longer skips `mergewright-core/src/on_disk.rs` (:230). That file now spells only current names.
- `OLD_NAMES` (11b1) stays; 11a allow-lists it.

**D4. `OperationKind::Migrate` doc line** (core MAP and source doc): "a rename move an earlier build journaled; kept so those journals still read; never written, never undone". No other change (11b3 D2).

**D5. Record the decision.**
- Append "Amendment (2026-10-25): earlier names dropped" to `docs/adr/0057-tributary-and-what-ships.md`. Five lines:
  - the user's ruling;
  - what went (§2–§8 machinery);
  - what stays (`Migrate` kind, consts in `on_disk`);
  - the cost: repos not opened since the rename keep their history in the old folder, unread; old stashes and practice folders aren't recognised;
  - pre-rename journal entries behave on `u` exactly as they did before the drop, as 11b3's parity tests pin. Name the open question on pruned entries; don't claim they can or can't be undone.
- No new ADR.

## Stop conditions
- A capture or snapshot changes when the marker goes. Stop with the list.
- A removed item has a caller that's more than a name swap: report it.
- 11b3's parity tests need an expected-value change: stop.

## Scope
In: the crate set; call-site swaps anywhere; MAP.md § on_disk (core, which also fixes the `Mergewright / Tributary / Mergewright` line), § repo (git: remove the `names_in_force` / `on_disk_names` lines), the fixtures marker and cache-key line (:98); the ADR amendment.
Out: comments that spell the codename (11a/11c). Fixture identities (12). Undo behaviour on pruned entries (separate question).

## Signatures
```rust
// removed: mergewright_core::on_disk::{Generation, GENERATIONS, CURRENT_DIR, MIGRATED_MARKER}
// removed: mergewright_git::{names_in_force, RepoHandle::on_disk_names, SharedRepo::on_disk_names}
// kept:    mergewright_core::on_disk::{DIR, TITLE, COMMAND, ENV_PREFIX, REF_PREFIX, LINK_TRAILER, BACKUP_EMAIL, MESSAGE_TAG, ASKPASS_SOCKET_VAR} and 11b1's predicates
// kept:    mergewright_core::OperationKind::Migrate (read-only; D4)
```

## Acceptance (tick each with its test, inline)
- [ ] No `Generation`, `names_in_force`, `on_disk_names`, `MIGRATED_MARKER` or `CURRENT_DIR` anywhere in `crates/` ← `git grep -n -E '…'` quoted, empty
- [ ] Guard with no skip, green ← `rename_names::no_old_message_literal`
- [ ] Fresh repos still write under the current names; parity unchanged ← `fresh_repo_names::journal_and_backups_use_current_names`; `journal_unmapped::*`, with that file's diff since 11b3 empty
- [ ] Fixture caches rebuilt, captures unchanged ← `rm -rf <fixture cache>` then `cargo nextest run` count; `scripts/walks.sh` SUMMARY; `git status --short docs/shots` empty
- [ ] Fixtures still deterministic ← the corpus determinism test by name
- [ ] Old-generation hits gone: `git grep -n -i -E 'graft|tributary' -- crates` count before and after quoted, with every remaining hit outside the old-generation class (spot-list 10)
- [ ] Fast and walk tiers green ← counts

## Touches (check each, say how)
- `commit_graph.rs` and its tests (git): its `Generation` use goes to `on_disk::DIR`. `history_reads/unreached.rs`. `remember_declined`'s folder.
- Fixtures cache keys and determinism; `published_repo`.
- `ref_name.rs` doc.
- Every MAP line naming a removed item: `git grep -n -E 'Generation|on_disk_names|names_in_force' -- 'crates/*/MAP.md'` empty.
- Keymap: none. Journal/undo: unchanged (D4, parity tests).

## Read first
This ticket. 11b1's Signatures. The 11b2 and 11b3 NOTES (their lists of call sites left). `crates/mergewright-core/MAP.md` § on_disk, `crates/mergewright-git/MAP.md` § repo, the fixtures MAP marker line (:98).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~40k tokens.
