# pre-publish: delete the Generation table, the names-in-force read and the fixtures' rename marker

Status: draft
Blocked by: 11b3
Runs: alone (core and git signatures change; call sites in every crate)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: once 11b2/11b3 have landed, nothing behavioural is left. This is deletion, with the compiler finding each call site.
Needs: 11b1's `on_disk` consts.
Crate set: `mergewright-core`, `mergewright-git`, `mergewright-fixtures`, plus **call sites only** in any crate that still names a removed item. 11b2/11b3 NOTES list them.

## Decisions
**D1. Removed:**
- from `core::on_disk`: `Generation` (all of it), `GENERATIONS`, `CURRENT_DIR` (callers → `DIR`), `MIGRATED_MARKER`, and the 11b1 equality test;
- from `mergewright-git`: `names_in_force` (`repo/names.rs`), `RepoHandle::on_disk_names` and `SharedRepo::on_disk_names`. Their callers use `on_disk::DIR` / `REF_PREFIX`. If `OpenError::Io` existed only for that stat (grep), it goes; otherwise it stays.
- `RefName::is_mergewright_internal` stays. It is true only under `on_disk::REF_PREFIX`.

**D2. Fixtures stop writing the rename marker.**
- `.git/mergewright/migrated` (private `settled.rs`) goes, and it leaves the three cache keys.
- The keys change, so every fixture cache rebuilds once. That's machine time, not a behaviour change.
- Prove no capture changes: `scripts/walks.sh` green with no capture diff (`git status docs/shots` clean).

**D3. The guard skip goes.**
- `rename_names.rs` no longer skips `mergewright-core/src/on_disk.rs` (:230). That file now spells only current names.
- `OLD_NAMES` (11b1) stays; 11a allow-lists it.

**D4. `OperationKind::Migrate` doc line** (core MAP and source doc): "a rename move an earlier build journaled; kept so those journals still read; never written, never undone". No other change (11b3 D2).

**D5. Record the decision.**
- Append "Amendment (2026-10-25): earlier names dropped" to `docs/adr/0057-tributary-and-what-ships.md`. Five lines:
  - the user's ruling;
  - what went (§2–§8 machinery);
  - what stays (`Migrate` kind, consts in `on_disk`);
  - the cost: repos not opened since the rename keep their history in the old folder, unread; pre-rename journal entries can't be undone; old stashes and practice folders aren't recognised.
- No new ADR.

## Stop conditions
- A capture or snapshot changes when the marker goes. Stop with the list.
- A removed item has a caller that's more than a name swap: report it.

## Scope
In: the crate set; call-site swaps anywhere; MAP.md § on_disk (core, which also fixes the `Mergewright / Tributary / Mergewright` line), § repo (git), the line on the marker and cache keys (fixtures); the ADR amendment.
Out: comments that spell the codename (11a/11c). Fixture identities (12).

## Signatures
```rust
// removed: mergewright_core::on_disk::{Generation, GENERATIONS, CURRENT_DIR, MIGRATED_MARKER}
// removed: mergewright_git::{names_in_force, RepoHandle::on_disk_names, SharedRepo::on_disk_names}
// kept:    mergewright_core::on_disk::{DIR, TITLE, COMMAND, ENV_PREFIX, REF_PREFIX, LINK_TRAILER, BACKUP_EMAIL, MESSAGE_TAG, ASKPASS_SOCKET_VAR, is_own_ref, is_own_message, is_own_trailer}
// kept:    mergewright_core::OperationKind::Migrate (read-only; D4)
```

## Acceptance (tick each with its test, inline)
- [ ] No `Generation`, `names_in_force`, `on_disk_names`, `MIGRATED_MARKER` or `CURRENT_DIR` anywhere in `crates/` ← `git grep -n -E '…'` quoted, empty
- [ ] Guard with no skip, green ← `rename_names::no_old_message_literal`
- [ ] Fixture caches rebuilt, captures unchanged ← `rm -rf <fixture cache>` then `cargo nextest run` count; `scripts/walks.sh` SUMMARY; `git status --short docs/shots` empty
- [ ] Fixtures still deterministic ← the corpus determinism test by name
- [ ] Old-gen hits gone: `git grep -n -i -E 'graft|tributary' -- crates` count before and after quoted, with every remaining hit outside the old-generation class (spot-list 10)
- [ ] Fast and walk tiers green ← counts

## Touches (check each, say how)
- `commit_graph.rs` and its tests (git): its `Generation` use goes to `on_disk::DIR`. `history_reads/unreached.rs`. `remember_declined`'s folder.
- Fixtures cache keys and determinism; `published_repo`.
- `ref_name.rs` doc.
- Every MAP line naming a removed item: `git grep -n -E 'Generation|on_disk_names|names_in_force' -- 'crates/*/MAP.md'` empty.
- Keymap: none. Journal/undo: unchanged (D4).

## Read first
This ticket. 11b1's Signatures. The 11b2 and 11b3 NOTES (their lists of call sites left). `crates/mergewright-core/MAP.md` § on_disk, `crates/mergewright-git/MAP.md` § repo, and the fixtures MAP marker line (:98).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~40k tokens.
