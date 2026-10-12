# Mergewright 1: one table of every name, and one move from whatever is in force

Status: Part A done; Part B ready
Blocked by: 46, 47
Build with:
- Part A: **Fable (signature)**. The generation table is the interface every later rename ticket calls, and every
  `New`/`Old` site is re-decided against it.
- Part B: **Opus**, as with 26r. The order is unchanged from 26/26r and each new window gets a test that injects at
  that step, so the tests can judge it. The conductor may raise B to Fable if the budget has room.
- Brownfield: `scouts/26-rename-on-disk.md` and 26/26r's NOTES serve as the scout. The conductor confirms.

Needs: none. First of the Mergewright set; nothing builds beside it (it touches every crate's names). Runs after the
usability pair (`usability-q1-*`, `usability-q2-*`) lands; the no-cargo `mergewright-protected-lines` may run beside it.

## Read first
- `docs/adr/0057-tributary-and-what-ships.md`: the Amendment (2026-10-09) and the **Amendment (2026-10-11)**, whole
- `issues/26-rename-on-disk-migration.md`: Decisions, Signatures, NOTES "Handoff to Part B"
- `issues/26r-rename-review-fixes.md`: Rulings, Signatures, NOTES Part A "Deviations" and "Residuals"
- MAP.md sections:
  - `crates/graft-core/MAP.md`: `## on_disk`, `## journal`, `RefName`
  - `crates/graft-git/MAP.md`: `## repo`, `file_lock`, `## commit_graph`
  - `crates/graft-ops/MAP.md`: `## rename`, `## snapshot` (`LinkKind`, `BACKUP_AUTHOR_*`), `## journal`
  - `crates/graft/MAP.md`: config, `rename`, `repos`
  - `crates/graft-fixtures/MAP.md`: `settled`
- Then source, only the files these sections name

## Decisions
- **The table** (ADR amendment 2026-10-11 §1–2) replaces `OnDiskNames { New, Old }`. The type is renamed to
  `Generation`, so the compiler lists every site. `ALL` (new first) is deleted and replaced by `GENERATIONS` (oldest
  first). No site may keep an ordering it assumed.
  - **Every `New` becomes `Generation::CURRENT`.**
  - **Every `Old` must name what it reads.**
    - Readers of past spellings accept every generation: the `RefName` "ours" predicate, the trailer reader, git
      message tags, `map_ref`.
    - The move's source is `names_in_force`.
  - NOTES lists each former `Old` site with its new form.
- **Part A changes no behaviour.** `CURRENT = Tributary` in A, and the `Mergewright` row is present but unused. With
  only Graft and Tributary reachable, the suite is green unchanged, apart from tests that assert `ALL`'s order (named
  in NOTES).
- **Part B flips `CURRENT` to `Mergewright`** and generalises the move (ADR amendment §3–§8):
  - In force: the newest marker present, no marker → Graft. Read in graft-git at open, no spawn.
  - Source = in force. A Graft repository goes straight to Mergewright, with nothing `tributary` created.
  - The stray pass covers every generation older than `CURRENT`. Refs kept aside go to
    `refs/mergewright/older-version/<n>/<gen dir>/<rest>`.
  - **Scratch**: a generation newer than the one in force, with no marker, is removed after the switch without compare
    (refs deleted at the value read, as the expected value).
  - **Locks**: the move try-locks every existing generation's lock exclusive. A run on past names holds `CURRENT`'s
    lock shared.
  - `Busy` words: `another Mergewright window is open on this repository`.
  - Config folder: the newest earlier folder present moves; older ones go through the 26r byte rule. The config lock is
    `<config_root>/mergewright.lock`.
- **The `Migrate` row names its target generation.**
  - First find out whether the row's words are stored in the entry at write time or built at read. Say which in NOTES.
  - Stored: nothing changes.
  - Built at read: `JournalEntry` gains `#[serde(default)] migrated_to: Option<Generation>`, and `None` reads as
    Tributary, the only generation that wrote `Migrate` before this.
  - Either way the old journal line is never rewritten.
- **Trailer and identity.**
  - The writer always writes `CURRENT`'s trailer (`Mergewright-Link:`); the reader accepts every generation.
  - `BACKUP_AUTHOR_NAME`/`EMAIL` come from `CURRENT.dir()`.
- **Fixtures.**
  - graft-fixtures `settled` writes `CURRENT`'s marker. The cache key changes, so each target dir rebuilds once.
  - The graft-ops test support helper `init_old_names` becomes `init_on(Generation)`. It builds a repository laid out
    under any generation: refs, dirs and marker (`clean`) when `has_marker()`.
- **Status words**: the `rename:` prefix and the 26r shapes are unchanged. Only the product word changes, from
  `CURRENT.title()`.

## Scope
- **In.** graft-core `on_disk` (table), `ref_name` predicate; graft-git marker read, `on_disk_names`, the lock paths;
  graft-ops `rename` (+`marker`, `refs`, `trees`, `strays`, `journals`, `map`), snapshot trailer and identity, journal
  read mapping; graft `config/folder.rs`, `rename::words`, `repos`; graft-fixtures `settled`; every test asserting
  `refs/tributary` or `.git/tributary` (they now assert `CURRENT`'s spelling, built from the table, never a literal);
  MAP lines for all of these.
- **Out.**
  - Package, `[[bin]]`, `COMMAND`, `-p graft`, scripts' binary paths and the sandbox script:
    `mergewright-package-and-command`.
  - `APP_NAME` and the product word in prose, help and keymap text: `mergewright-app-words`.
  - Env: `mergewright-env-names`.
  - Git-message writers and the journal column: `mergewright-git-messages-and-journal-column`.
  - This ticket only adds the table's `generation_of_message` reader they call.

## Signatures
```rust
// graft-core::on_disk  (pure; replaces OnDiskNames)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Generation { Graft, Tributary, Mergewright }
impl Generation {
    pub const CURRENT: Self;                         // A: Tributary · B: Mergewright
    pub const GENERATIONS: [Self; 3];                // oldest first
    pub const fn dir(self) -> &'static str;          // "graft" | "tributary" | "mergewright" — also git-message tag, backup author
    pub const fn title(self) -> &'static str;        // "Graft" | "Tributary" | "Mergewright"
    pub const fn command(self) -> &'static str;      // "graft" | "trib" | "mwr"
    pub const fn env_prefix(self) -> &'static str;   // "GRAFT_" | "TRIB_" | "MWR_"
    pub const fn ref_prefix(self) -> &'static str;   // "refs/graft/" | "refs/tributary/" | "refs/mergewright/"
    pub const fn link_trailer(self) -> &'static str; // "Graft-Link:" | "Tributary-Link:" | "Mergewright-Link:"
    pub const fn has_marker(self) -> bool;           // false for Graft only
    pub fn older(self) -> impl Iterator<Item = Self>; // generations before self, newest first
}
pub fn generation_of_ref(name: &str) -> Option<Generation>;       // by ref_prefix
pub fn generation_of_trailer(line: &str) -> Option<Generation>;   // by link_trailer
pub fn generation_of_message(message: &str) -> Option<Generation>; // message starts "<dir>:" or "<dir> "
pub const MIGRATED_MARKER: &str = "migrated";                      // unchanged
// graft-core::ref_name — RefName::is_graft_internal: generation_of_ref(..).is_some() (name kept: code name)
// graft-git::repo
impl SharedRepo { pub fn on_disk_names(&self) -> Generation; }
impl RepoHandle { pub fn on_disk_names(&self) -> Generation; }
// graft-ops::rename — settle_names, Settled, SettleOutcome, MigrateError, KEPT_ASIDE_DIR: signatures unchanged
pub fn map_ref(name: &str, in_force: Generation) -> std::borrow::Cow<'_, str>; // every generation older than in_force → in_force
pub fn description(to: Generation, refs: usize, files: usize) -> String;     // "rename to <to.title()> (N refs, M files)"
// graft-ops::snapshot
pub const BACKUP_AUTHOR_NAME: &str;  // = Generation::CURRENT.dir()
pub const BACKUP_AUTHOR_EMAIL: &str; // "<CURRENT.dir()>@backup", built by the existing const shape or a const fn
```

## Part A (commit 1, no behaviour change; `CURRENT = Tributary`)
- [x] Table spells every row: `on_disk::tests::every_generation_spells_its_names`
- [x] Table order: `on_disk::tests::generations_are_oldest_first_and_current_is_last_reachable`
- [x] `generation_of_ref`, `generation_of_trailer`, `generation_of_message` each read all three spellings
      (as `Generation::of_ref`/`of_trailer`/`of_message`, see Deviations):
      `on_disk::tests::each_spelling_reads_back_its_generation`
- [x] `is_graft_internal` is true under all three prefixes: `ref_name::tests::every_prefix_internal`
- [x] No `OnDiskNames`, `::New` or `::Old` left (grep quoted in NOTES), and the former `Old` sites are listed
- [x] Workspace suite green, with only the order-asserting tests changed (named in NOTES): `cargo nextest run`
      6391 tests, 6390 passed + `file_size` ratchet fixed and re-run (838/838 in graft-ops it + file_size);
      `cargo test --doc` green

## Part B (commit 2: Mergewright in force)
All in `graft-ops/tests/it/rename/generations.rs` unless stated. Truth comes from git (`for-each-ref`, `fsck`) and the
journal reader, never from the code under test.
- [ ] **Graft → Mergewright direct.** 3 backups and a 20-entry journal under Graft names. After open: refs under
      `refs/mergewright/`, none under `refs/graft/` or `refs/tributary/`; no `tributary` dir anywhere; the journal has
      the 20 entries and one `Migrate`; `u` restores the newest real operation byte-exactly; `fsck` clean.
      Test: `graft_repo_moves_straight_to_mergewright`
- [ ] **Tributary `clean` → Mergewright.** The journal has the 20, `rename to Tributary`, then the moves made under
      Tributary, then `rename to Mergewright`. The J words for both rows are pinned.
      Test: `tributary_repo_moves_and_both_rename_rows_read`
- [ ] **Tributary `switched` with Graft leftovers.**
      - A Graft journal that is a prefix of the new one is deleted.
      - A Graft ref at the same oid is deleted.
      - A Graft ref at another oid lands at `refs/mergewright/older-version/1/graft/<rest>`.
      - Outcome `Migrated { kept_aside: Some }`.
      Test: `switched_tributary_with_graft_leftovers`
- [ ] **Scratch.** A Graft repo holding `refs/tributary/x` and `.git/tributary/{lock,undo-journal.jsonl}` with no
      tributary marker. After the move both are gone, nothing is kept aside, and the outcome words carry no "older
      version". Test: `marker_less_newer_generation_is_scratch`
- [ ] **Locks.** A shared hold on `<common>/tributary/lock` makes the move `Busy` and leaves Graft in force. With the
      hold dropped, the next settle migrates. The same holds for a shared hold on `<common>/mergewright/lock`.
      Test: `any_generations_lock_blocks_the_move`
- [ ] **Strays on `clean`.**
      - A Tributary build writes `<common>/tributary/layout` and a Graft build writes `refs/graft/y` after `clean`.
      - Result: `OlderWrote` covering both.
      - The second settle is `Done` with zero spawns.
      Test: `older_generations_writing_after_clean`
- [ ] **Zero spawns.** `clean` with empty Graft and Tributary dirs gives zero spawns: `clean_marker_spawns_nothing`
      (extended to both generations)
- [ ] **Trailers.** Chains with `Graft-Link:` and `Tributary-Link:` read their kinds, and a new link writes
      `Mergewright-Link:`. Test: `graft-ops snapshot::tests::every_generations_trailer_reads`
- [ ] **Journal mapping.** The golden `m5-journal.jsonl` stays byte-identical and reads its backups under
      `refs/mergewright/`. Test: `rename/tests::the_m5_golden_reads_its_backups_under_the_new_prefix`
- [ ] **Old `Migrate` entry.** A golden Tributary-era `Migrate` line reads `rename to Tributary (…)`. NOTES says
      whether the words were stored or built. Test: `graft view/journal/tests::an_old_migrate_row_keeps_its_target`
- [ ] **Config folder.** With `graft/` alone, `tributary/` alone, and both present (`graft/` differing): one
      `mergewright/`; older files that differ kept aside under `mergewright/older-version/<n>/`; warned on that
      launch only. Test: `graft config::folder::tests::every_earlier_folder_moves_once`
- [ ] **Status words.** Every outcome's status words come from the table, and the `Busy` words are pinned. Test:
      `graft/tests/it/rename_status.rs::every_outcome_has_its_words_or_none`
- [ ] All 26 and 26r tests stay green, re-pointed at `CURRENT`, with the list in NOTES. No test literal spells
      `tributary` or `mergewright` where the table can.
      Test: `graft/tests/it/rename_opens.rs::no_old_dir_literal`, extended to every generation's dir and prefix,
      allow-list being `on_disk.rs` only
- [ ] `git fsck` is clean after each scenario above (asserted inside each test)

## Touches
- **graft-core**: `on_disk`, `ref_name`, journal (`migrated_to` only if built at read) + MAP.
- **graft-git**: the marker read at open, `file_lock` paths, the commit-graph offer dir + MAP.
- **graft-ops**: `rename/*`, snapshot trailer and identity, journal readers and raw readers (`apply/restart.rs`,
  `conflict/set.rs`, `tour.rs`), `rewrite/saved` + MAP.
- **graft**:
  - `repos`, `rename`, `config/{load,folder}`, `recent`, `first_launch`, `layout`, outside editor, `jobs/stash` + MAP.
  - `rename_status`, `rename_opens`.
- **graft-fixtures** `settled` + MAP: the corpus, synthetic and team caches rebuild once.
- **Tests**: every test asserting the old literals (26 B listed 26 graft-ops and 27 graft files).
- The `file_size` ratchet.
- **Undo and journal**: `Migrate` stays transparent to undo. Checked by `migrate_is_transparent_to_undo`.
- **Captures**: none expected. A TestBackend capture that prints a ref or dir name regenerates under the gates, and
  its drift goes in NOTES.
- **Scripts**: `scripts/trib_sandbox.py` seeds `<config>/tributary/launches`; after B the app migrates that seed on
  each sandboxed launch. Not wrong, only noisy; `mergewright-package-and-command` moves the seed. Listed, not edited.
- **Keys**: none.
- Each item is checked by the named grep or test, listed in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide, after each Part.

## Size
A ~40k, B ~60k. Two contexts, two commits at the A/B seam.

## NOTES

### Part A (commit 1, `CURRENT = Tributary`)
Two builders: the first was paused near the end of Part A (user paused the session); the second finished it. No compaction
in the second. Scout: none new; the ticket's named scout (`scouts/26-rename-on-disk.md`) and 26/26r NOTES stood, plus
the first builder's uncommitted diff, read whole.

**Grep (no `OnDiskNames`, `::New`, `::Old` left):**
`grep -rnE 'OnDiskNames|Names::(New|Old)|init_old_names' crates | wc -l` → `0`.

**Deviations**
- `generation_of_ref/_trailer/_message` are associated fns `Generation::of_ref/of_trailer/of_message`, not free fns:
  graft-core exports no free fn (ADR-0007, `tests::core_has_no_free_functions`). Same behaviour, same callers.
- Added `Generation::backup_email()` (`<dir>@backup`, spelled per row: a `const` cannot join strings).
- `graft-git::repo::names_in_force(common_dir) -> io::Result<Generation>` (new `repo/names.rs`): newest marked
  generation up to `CURRENT`, else Graft. Read at open and again by the move under its lock. With `CURRENT =
  Tributary` it is the old one-marker read.
- Early Part B groundwork, behaviour unchanged while one generation is older than `CURRENT`:
  `settle_strays(repo, from)`, `settle_older_strays` (runs `settle_strays` per older generation, each with its own pass
  number — **not yet §5's one pass across generations**), `older_names_present`, `trees::{old_trees,new_without_old}`
  and `refs::{list_both,carry_or_set_aside}` take `from`; `migrate_config_dir(root, from)`; `settle_folder` passes
  `CURRENT.older().next()` (newest earlier only).
- `description(to: Generation, refs, files)` per Signatures; callers pass `Generation::CURRENT`.
- `TempRepo::init_old_names()` → `TempRepo::init_on(Generation)` (marker `clean` when `has_marker()`; refs are the
  caller's). `init()` = `init_on(CURRENT)`; old call sites use `init_on(Generation::Graft)`.

**`Migrate` row words: stored**, not built at read (`rename/record.rs` `append`: `description: description(..)`
written into the entry). So no `migrated_to` field; an old row keeps "rename to Tributary" as written.

**Former `Old` sites → new form**
- `on_disk.rs` `ALL` → `GENERATIONS` (oldest first); match arms → per-row spellings.
- `ref_name.rs` `is_graft_internal`: `ALL` → `GENERATIONS` (reader of every generation).
- `snapshot/chain.rs` trailer reader: `ALL.find_map` → `Generation::of_trailer` (every generation); writer → `CURRENT`.
- `rename/map.rs` `map_ref`: `Old`→`New` strip → every generation older than `in_force` → `in_force`; `map_entry`
  early-out `names == Old` → `in_force.older().next().is_none()`.
- `rename.rs` `copy_everything`: `refs::list(repo, Old)` → `names_in_force(common)` (`from`); `stay_old` →
  `repo.on_disk_names()`.
- `rename/refs.rs` `list_both`, `carry_or_set_aside` strip: `Old` → `from`.
- `rename/strays.rs` `old_names_present`: `Old.dir()` → `from.dir()`.
- `rename/trees.rs` `old_trees`, `new_without_old`: `Old.dir()` → `from.dir()`.
- `graft-git repo/mod.rs` `names_in`: `Ok(false) => Old` → `names_in_force` (no marker → `Graft`).
- `graft config/folder.rs` `migrate_config_dir`: `Old.dir()` → `from`; `config/load.rs` `settle_folder`: `Old` →
  `from` (= `CURRENT.older().next()`).
- Tests: `Old` → `Generation::Graft` in graft-git `repo/tests.rs`, `it/commit_graph.rs`; graft-ops
  `snapshot/tests.rs`, `rename/tests.rs`, `it/rename.rs`, `it/rename/{more,review}.rs`, `it/journal_names.rs`;
  graft `config/folder/tests.rs`, `it/finish.rs`; allow-list text in `it/rename_{names,opens}.rs` (allow-list now the
  `on_disk` table + its tests).

**Tests changed because they asserted `ALL`'s order or the two-row shape:** `on_disk::tests::each_set_spells_its_own_names`
and `the_prefix_is_the_dir_under_refs` (replaced by the three table tests); `ref_name::tests::both_prefixes_internal` →
`every_prefix_internal`; `rename::tests::the_entry_words_count_what_moved` (new `to` arg, Mergewright row added). Every
other test edit is the type rename only.

**Touches checked:** compiler (type rename lists every site); grep above; `file_size` ratchet (`over_400.txt`
`repo/tests.rs` 703→704; `it/support.rs` held at 459); MAP.md lines for graft-core, graft-git, graft-ops; captures:
none drifted (full nextest green). graft-fixtures `settled` unchanged (still `tributary` = `CURRENT` in A).

### Handoff to Part B
Start from this commit. Exact next steps:
1. Flip `Generation::CURRENT` to `Mergewright`; build; the compiler and tests then show what assumed one older gen.
2. `graft-fixtures/src/settled.rs`: literal `tributary` → `mergewright` (crate can't depend on graft-core;
   `graft/tests/it/rename_status.rs` pins the two) + its MAP line.
3. Strays: replace `settle_older_strays`' per-generation passes with §5's one pass; kept-aside ref path must become
   `refs/mergewright/older-version/<n>/<gen dir>/<rest>` (now `<CURRENT prefix>older-version/<pass>/<rest>`,
   `rename/refs.rs:164`).
4. Scratch (marker-less newer generation) removal; locks over every existing generation; `Busy` words.
5. Config: `settle_folder` moves only `CURRENT.older().next()`; older folders need the 26r byte rule.
6. Test literals: e.g. `rename/tests.rs` still spells `"refs/tributary/backup/main/x"`; extend `no_old_dir_literal`.
Gotchas: `file_size` ratchet — `it/support.rs` is at its 459 cap; new tests go in `it/rename/generations.rs`.
