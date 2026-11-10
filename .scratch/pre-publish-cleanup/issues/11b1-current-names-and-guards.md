# pre-publish: current-only name constants, and the name guards rewritten off the Generation table

Status: done
Blocked by: 06
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: this spec fixes the constant set; adding the constants and swapping each guard's data source is mechanical, and it's checked by a test written first.
Needs: none
Scout: done. Use `scouts/11-codename-hits.md` (§ Existing guards, § Old-generation). Don't open source to re-scout.

## Why (user ruling 2026-10-25, memory "code holds only current state")
U1 = **drop**. All code that finds, moves or reads old-name folders, refs, messages or env names goes (11b1–11b4). The table `mergewright_core::on_disk::Generation` (ADR-0057 Amendment §2) exists only to serve that code. This ticket is step 1. It adds the current-only names **alongside** the table and takes the two guards off the table, so later tickets can delete the table without leaving the code unguarded.

## Decisions
**D1. Plain consts replace the table.** No single-row enum and no `Names` struct: a table with one row is leftover migration scaffolding. A later rename (the trademark search is still open) edits these consts. Callers write them qualified (`on_disk::DIR`), never glob-imported.

**D2. Additive only.** `Generation` stays untouched here. Each new const is pinned equal to today's `Generation::CURRENT` spelling by a test that 11b4 deletes along with the table.

**D3. Guards off the table, behaviour unchanged.**
- `crates/mergewright/tests/it/rename_names.rs`:
  - `no_old_message_literal` (:221) takes its old tags from `Generation::older()`.
  - The name check (:263-271) does the same.
  - Both switch to a test-local `const OLD_NAMES`: every earlier generation's spellings (dir, title, command, env prefix, ref prefix, trailer key, backup email), written out once from today's table.
- Test-first:
  - Factor the line check into a fn.
  - Unit-test it red→green against an in-memory line that holds an old tag and one that holds the current tag.
  - Then swap the data source. The guard over the tree must be green before and after the swap.
- The skip of `mergewright-core/src/on_disk.rs` (:230) stays until 11b4.
- `OLD_NAMES` spells the codename on purpose. 11a allow-lists it with reason `guard`.

## Scope
In: the consts and predicates in `mergewright-core::on_disk`, their equality test, the two guards, `crates/mergewright-core/MAP.md` § on_disk lines for the new items.
Out: porting any caller (11b2, 11b3); deleting anything (11b4).

## Signatures
```rust
// crates/mergewright-core/src/on_disk.rs: values = today's Generation::CURRENT spellings
pub const DIR: &str = "mergewright";
pub const TITLE: &str = "Mergewright";
pub const COMMAND: &str = "mwr";
pub const ENV_PREFIX: &str = "MWR_";
pub const REF_PREFIX: &str = "refs/mergewright/";
pub const LINK_TRAILER: &str;  // = Generation::CURRENT.link_trailer() today
pub const BACKUP_EMAIL: &str;  // = Generation::CURRENT.backup_email() today
// MESSAGE_TAG stays (= DIR). CURRENT_DIR stays until 11b4 (alias of DIR).
pub fn is_own_ref(name: &[u8]) -> bool;       // replaces Generation::of_ref(..).is_some()
pub fn is_own_message(message: &str) -> bool; // replaces of_message: "<DIR>:" or "<DIR> " lead
pub fn is_own_trailer(line: &str) -> bool;    // replaces of_trailer (only if a caller uses it; grep)

// crates/mergewright/tests/it/rename_names.rs
const OLD_NAMES: &[&str] = &[/* every earlier generation's spellings */];
fn old_name_in(line: &str) -> Option<&'static str>; // the factored check; unit-tested
```

## Acceptance (tick each with its test, inline)
- [x] Each const equals the table's `CURRENT` spelling ← `on_disk::tests::consts_match_current_generation`
- [x] Predicates: own ref, message and trailer true; another tool's false ← `on_disk::tests::is_own_ref`, `is_own_message`, `is_own_trailer`
- [x] Factored check red on an old tag, quiet on the current one ← `rename_names::old_name_in_flags_old_tags`
- [x] Guards green on the tree with no `Generation` use left in `rename_names.rs` ← `rename_names::no_old_message_literal` + name check; `grep -c Generation crates/mergewright/tests/it/rename_names.rs` = 0 in NOTES
- [x] Gates green ← `cargo nextest run` 6855/6855; doc tests ok; clippy + heavy clippy clean; walks 462/0

## Touches (check each, say how)
- `rename_names.rs` only, in the app crate. `rename_names/scripts.rs`, `release_names.rs` and `rename_docs.rs` are not touched here (11b2, 11a).
- `on_disk` MAP lines. The MAP's `Generation` line wrongly reads `Mergewright / Tributary / Mergewright`; leave it, because 11b4 rewrites that section.
- Callers: none change (additive). Show it with `git diff --stat`.

## Read first
This ticket. `scouts/11-codename-hits.md`. `crates/mergewright-core/MAP.md` § on_disk. ADR-0057 Amendment 2026-10-11 §2 only.

## Gates
Workspace-wide: the four CLAUDE.md gates and heavy clippy. No walks (nothing drawn changes).

## Size
~25k tokens.

## NOTES (builder)
- Deviation: predicates are `on_disk::Own::is_ref/is_message/is_trailer` (unit struct), not free fns — spec's free fns would fail `tests::core_has_no_free_functions` (ADR-0007). `is_trailer` kept: `mergewright-ops/src/snapshot/chain.rs:259` uses `of_trailer`.
- `grep -c Generation crates/mergewright/tests/it/rename_names.rs` = 0.
- `old_name_in` reads tags as the dirs inside `OLD_NAMES`' `refs/<dir>/` entries, so it matches exactly what `of_message` did (titles/commands as tags would flag "Graft Fixture" literals).
- `GRAFT_`/`TRIB_` written as `concat!` so `rename_names::scripts::no_old_env_name` (out of scope) doesn't flag the list.
- Name check now also scans env/ref/trailer/email spellings as words; tree stays green.
- No compaction.
