# pre-publish: delete the Generation table, the names-in-force read and the fixtures' rename marker

Status: done
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
- [x] No `Generation`, `names_in_force`, `on_disk_names`, `MIGRATED_MARKER` or `CURRENT_DIR` anywhere in `crates/` ← `git grep -n -E '…'` quoted, empty — `git grep -n -E '\bGeneration\b|names_in_force|on_disk_names|MIGRATED_MARKER|CURRENT_DIR' -- crates` → 0 lines
- [x] Guard with no skip, green ← `rename_names::no_old_message_literal` — skip and ALLOWED row removed; passes in full run
- [x] Fresh repos still write under the current names; parity unchanged ← `fresh_repo_names::journal_and_backups_use_current_names`; `journal_unmapped::*`, with that file's diff since 11b3 empty — both green in full run; `git diff 89c0c209 -- crates/mergewright-ops/tests/it/journal_unmapped.rs` empty
- [x] Fixture caches rebuilt, captures unchanged ← `rm -rf <fixture cache>` then `cargo nextest run` count; `scripts/walks.sh` SUMMARY; `git status --short docs/shots` empty — `rm -rf target/fixtures`, full run 6788 run / 6787 passed / 1 MAP-length fail, fixed and re-run green; walks SUMMARY tests=459 failed=0; `git status --short docs/shots` empty
- [x] Fixtures still deterministic ← the corpus determinism test by name — `team::tests::build_is_deterministic` green
- [x] Old-generation hits gone: `git grep -n -i -E 'graft|tributary' -- crates` count before and after quoted, with every remaining hit outside the old-generation class (spot-list 10) — 955 before, 899 after; remaining are codename comments, temp prefixes (`graft-reap-`), test literals, snapshot text (11a/11c/12 scope)
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

## NOTES (11b4 build)
- `support::init` (ops tests) still makes the empty `.git/mergewright` folder, marker gone: keeps `fresh_repo_names` and `commit_chosen_scratch` unchanged.
- D1 behaviour change: `is_mergewright_internal` now false for `refs/graft/` and `refs/tributary/`; old backup refs in a pre-rename repo now show in the graph. Tests `only_current_prefix_internal`, `grafts_own_refs_are_not_drawn_but_are_still_readable` (now uses `refs/mergewright/`).
- D4 MAP line shortened to fit the 240-char MAP limit: "`Migrate`: old rename move, read only, never undone". Source doc carries the full wording.
- `OpenError::Io` stays (used by `classify`).
- over_400 rows only lowered.
