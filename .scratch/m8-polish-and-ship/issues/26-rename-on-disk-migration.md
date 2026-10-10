# G-rename 1 — his refs, journal and config move to the new name, once, safely

Status: done (Part A commit 1, Part B commit 2). Was: ready (amended after scout `scouts/26-rename-on-disk.md`, spec-only pass 2026-10-09)
Build with: Part A **Opus** (`builder-opus`); Part B **Fable** (`builder-fable`): lock-and-switch order can't be judged from the diff (conductor, 2026-10-09). Planner expects **Opus, alone**, in **two commits at the Part A / Part B seam**
(recommended: dispatch as two contexts — A ~35k, B ~60k; together they exceed one). Part B's lock-and-switch order is
the one place correctness can't be read from the diff; if the milestone's Fable budget has room, B is the candidate.
Blocked by: 01–04, 06–25, 33, 34, 35, 36, 37
Needs: every M8 build ticket and health 33–37 merged. Before `rename-names-sweep` (27).
Order (conductor, amended 2026-10-09): health 33–37 merge before 26. **38 runs after 40** (needs 40's `Cells`), and
39–42 need 27's final names, so 38–42 run after 27 (the exception to "G-health merged"). 05 is deferred to last by
the user (2026-10), not a blocker.

## Read first
- `docs/adr/0057-tributary-and-what-ships.md` §2 and Consequences; `docs/adr/0010-…` Decision only
- `docs/design-notes/m8-polish-and-ship.md`: Hard-to-reverse choice 3
- `.scratch/m8-polish-and-ship/scouts/26-rename-on-disk.md` (the site list, file:line)
- `ARCHITECTURE.md` lines 249–257 (layers: `git` cannot see `ops`)
- `crates/graft-core/MAP.md` `## ref_name`, `OperationKind`; `crates/graft-git/MAP.md` `## repo`, `## commit_graph`;
  `crates/graft-ops/MAP.md` `## journal`, `## journal (file handle)`, `## snapshot` (`LinkKind`, `backup_ref_name`),
  `## apply` (`Context`); `crates/graft/MAP.md` `layout`, `config`, recents, first-launch
- Every M8 ticket's NOTES "G-rename list" line (conductor collects them into NOTES before dispatch)

## Decisions (planner; scout findings resolved)
- **What moves** (unchanged): `refs/graft/**` → `refs/tributary/**`; `…/graft/` dirs → `…/tributary/`; config dir.
- **The scope is the repository, not the worktree.** One pass moves the refs, `<common_dir>/graft/`, **every**
  worktree's `<common_dir>/worktrees/*/graft/`, and `<git_dir>/graft/` (deduplicated by canonical path; the main
  worktree's `git-path` dir *is* `<common_dir>/graft/`). Why: refs are shared, so a per-worktree flip would leave a
  worktree reading old journal names against moved refs. One state per repository.
- **Whole-tree copy**, every file byte-compared after copy; journal files also parsed through the real reader and
  compared entry by entry. Why: the old tree is deleted afterwards, so a file the migration didn't name would be
  deleted unread. This settles the scout's three unlisted files:
  | Site | Decision | Why |
  |---|---|---|
  | `commit-graph-offer` (graft-git `commit_graph.rs:219`, `<common_dir>/graft/`) | **migrate in 26** (whole-tree copy); `offer_state`/`remember_declined` take the dir from `repo.on_disk_names()` | dropping it re-asks a question he already answered |
  | `rewrite/<id>/plan.json` (graft-ops `rewrite/saved.rs:95`, `<git_dir>/graft/`) | **migrate in 26** | a paused rewrite resumes from it after relaunch (conductor addition item 2) |
  | `COMMIT_EDITMSG` (graft `event_loop/outside.rs:89,103`, `<git_dir>/graft/`) | **migrate in 26** (whole-tree copy, path via names) | it sits in a dir 26 deletes; byte-copy costs nothing |
  | crash reports (graft `crash.rs:164`, `<state_home>/graft`) | **rename-only, 27**; old reports left where they are | his files, not app state; a third root on the riskiest ticket buys nothing |
  | backup identity `"graft"` (`snapshot/plumbing.rs:297,299`) | **rename-only, 27** | old commits are immutable; nothing found reading it (27 confirms) |
  | man text naming paths (`cli/man.rs`) | **27** | user-facing docs |
- **`OnDiskNames` lives in graft-core**, not graft-ops: graft-git's ref filter and commit-graph offer need it and
  cannot see graft-ops. Carried on `SharedRepo`/`RepoHandle`, read at open from the marker's existence (no spawn).
  Every opener, tests and headless paths included, therefore sees the truth: marker → `New`, none → `Old`.
- **Ref prefix, one source.** `GRAFT_REFS` (graft-ops `apply/fetched.rs:32`) is deleted; `fetched.rs:58` uses
  graft-core's predicate. `is_graft_internal` matches **both** prefixes always: both are ours, and a half-moved repo
  must never show either in the graph or count one as fetched.
- **`Graft-Link:` trailer** (conductor addition item 1) — **26 owns it**: writer always writes `Tributary-Link:`;
  `link_of` accepts both. Why: backup commits are immutable objects; the old spelling must read forever.
- **Journal kind:** `OperationKind::Migrate` (graft-core `journal.rs:32`), serde `migrate`. **`Operation` (the
  command enum) is unchanged** — migration isn't run through `apply`; the 10 `Operation` matches are not touched.
  `undoable_kind` is a `matches!`: the builder checks which way it lists and makes `Migrate` false explicitly.
  `next_redo` treats `Migrate` like a fetch (doesn't empty redo); `plan_revert_to` steps over it.
- **Marker** `<common_dir>/tributary/migrated`, one line: `switched` or `clean`.
  Order: lock → create new refs (one `update-ref --stdin`; a new ref already at the old oid is skipped, at another
  oid is `MigrateError::Refs`) → read back → copy trees → read back → **marker `switched`** (the switch point: `New`
  in force from here) → append the `Migrate` entry to the opening worktree's (new) journal → delete old refs (old
  oids as expected values) → remove old trees → marker `clean`. Fails before the switch → `Old`, nothing old
  deleted. Fails after → `New` stays, leftovers removed on the next launch (marker still `switched`).
- **Two processes.** Advisory lock `<common_dir>/tributary/lock` (std `File::lock`/`try_lock`; toolchain predates
  1.89 → stop and report, no new dependency). A run under `Old` holds it **shared** for its life; migration needs it
  **exclusive** via `try_lock`, else `MigrateError::Busy`, run stays `Old`. An exclusive holder (migrating) makes
  the shared request wait for it — a lock wait, not a poll — then the marker is re-read. Why: without it a second
  window still under `Old` keeps writing old names after the switch, and the marker stops anyone noticing.
- **Hook:** graft `repos.rs:92` (`RepoStore::shared`): open `SharedRepo`; if its marker reads absent or `switched`,
  `settle_names(&shared.handle()?)`; after `Migrated`, reopen (marker now present); store the `Settled.hold` beside
  it. No job gets a handle before this returns, whatever the job order (scout's unknown, settled structurally).
  Config: `migrate_config_dir` before the first `config::load`; a `path_in_environment` override is never moved.
- **Journal mapping:** every ref-name field of an entry passes through `map_ref(name, repo.on_disk_names())` at read;
  bytes are not rewritten. Raw readers (`apply/restart.rs:243`, `conflict/set.rs:161`, `tour.rs:240`,
  graft `jobs/stash.rs:46`) take the path from `journal_path` and the mapping, or NOTES says why one doesn't need it.
- **Status row:** `rename: still using the old names · <reason>` (`Busy`: `another trib is open on this repository`);
  after the switch, `rename: old names left behind · <reason>` and `rename: done · not in the journal: <reason>`.
  Shown only then; nothing on `Done`.
- **Never** deletes an old ref or file before its new copy reads back the same; never touches working tree or index.

## Signatures
```rust
// graft-core::on_disk (new, pure)
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum OnDiskNames { New, Old }
impl OnDiskNames { pub const fn ref_prefix(self) -> &'static str; pub const fn dir(self) -> &'static str;
                   pub const fn link_trailer(self) -> &'static str; }
pub const MIGRATED_MARKER: &str = "migrated"; // in <common_dir>/<New.dir()>/
// graft-core::ref_name — is_graft_internal: true under either prefix; GRAFT_INTERNAL replaced by ref_prefix
// graft-core::journal — OperationKind::Migrate
// graft-git::repo
impl SharedRepo { pub fn on_disk_names(&self) -> OnDiskNames; }
impl RepoHandle { pub fn on_disk_names(&self) -> OnDiskNames; }
// graft-ops::rename
pub struct Settled { pub names: OnDiskNames, pub outcome: SettleOutcome, pub hold: Option<OldNamesHold> }
pub enum SettleOutcome { Done, Migrated { refs: usize, files: usize, not_journaled: Option<String> },
                         CleanedUp { refs: usize, files: usize }, KeptOld(MigrateError), LeftOld(MigrateError) }
pub struct OldNamesHold { /* shared lock; drop releases */ }
pub enum MigrateError { Busy, Refs { words: String }, Files { path: PathBuf, words: String }, ReadBack { what: String } }
pub fn settle_names(repo: &RepoHandle) -> Settled;
pub fn map_ref(name: &str, names: OnDiskNames) -> std::borrow::Cow<'_, str>;
// graft (pub(crate))
pub(crate) fn migrate_config_dir(config_root: &Path) -> Result<ConfigDirMoved, std::io::Error>;
```
Every reader/writer of the prefix, dir or trailer takes them from `on_disk_names()`; no literal `"graft"` path
segment survives outside graft-core `on_disk`, except those handed to 27 below.

## Part A (commit 1: names threaded, no behaviour change — no marker anywhere, so everything reads `Old`)
graft-core `on_disk`, predicate, `Migrate` kind; graft-git marker read + `on_disk_names`, offer dir; every site in
the scout list routed through names; `GRAFT_REFS` deleted; trailer reader both / writer new. Whole suite green
unchanged except trailer tests.

## Part B (commit 2: the migration)
graft-ops `rename`, journal mapping, undo/redo/time-machine handling, J words; graft hook, config dir, status row;
graft-ops test fixtures created migrated (marker `clean`) so the suite exercises `New`; tests asserting `refs/graft`
updated; golden `m5-journal.jsonl` **kept byte-identical** — it now proves the read mapping.

## Acceptance (each box: the test the builder writes, named)
- [x] 3 backup refs + 20-entry journal, old names (M7 walk fixtures): after open, refs under `refs/tributary/`, none
      under `refs/graft/`; journal reads the 20 + `Migrate`; `u` undoes the newest real operation byte-exactly —
      `graft-ops/tests/it/rename.rs::old_repo_moves_and_undo_still_restores` (truth by git).
- [x] Kill after the create transaction (test hook): relaunch completes; no ref lost or duplicated — `rename.rs::killed_after_create_completes_next_open`.
- [x] Read-back mismatch injected → `KeptOld`, nothing old deleted, status line words — `rename.rs::read_back_mismatch_keeps_old`; words: `graft/tests/it/rename_status.rs::kept_old_words`.
- [x] Marker `switched` with old refs left → next open deletes them, creates none — `rename.rs::switched_marker_cleans_leftovers`.
- [x] Second open: zero git spawns from `settle_names` — `rename.rs::clean_marker_spawns_nothing` (spawn counter).
- [x] Shared hold blocks migration (`Busy`, `Old`); hold dropped → next settle migrates — `rename.rs::old_names_hold_blocks_migration`.
- [x] Linked worktrees: one settle moves every worktree's journal, including one never opened; refs once — `rename.rs::every_worktree_moves_in_one_pass`.
- [x] `<git_dir>/graft` and `git-path graft` resolve to the same dir in main and linked worktrees, or both are moved — `rename.rs::git_dir_and_git_path_dirs_both_move`.
- [x] An unknown file under each old tree arrives byte-equal — `rename.rs::unknown_files_travel`.
- [x] Saved rewrite plan listed and resumable after migration — `rename.rs::saved_rewrite_plan_survives`.
- [x] Commit-graph decline survives (`offer_state` = `Declined`) — **`graft-ops/tests/it/rename.rs::decline_read_from_new_dir`** (moved from graft-git: the move is graft-ops', which graft-git cannot call).
- [x] A chain written with `Graft-Link: resume` reads as `Resume`; new links write `Tributary-Link:` — `graft-ops snapshot/tests::old_link_trailer_reads`. (Part A; also `conflict_set.rs::a_link_without_a_kind_reads_as_step` reads the written line back by git.)
- [x] `is_graft_internal` true for both prefixes; a leftover old ref is not counted as fetched — `graft-core ref_name tests::both_prefixes_internal`, `graft-ops/tests/it/aborted_fetch.rs::old_prefix_ref_not_fetched`. (Part A.)
- [x] `undoable_kind(Migrate)` false; `next_undo` skips it; `next_redo` not emptied by it; `plan_revert_to` steps over — `graft-ops journal/tests::migrate_is_transparent_to_undo`.
- [x] J row reads `rename to Tributary (3 refs, M files)` — `graft view/journal/tests::migrate_row_words`.
- [x] Config dir copied, read back, old removed; both present → new kept, warned once; env-path override untouched; first `config::load` reads the new path — `graft config::folder::tests::config_dir_moves_before_first_load` (+ `config_dir_moves_both_present`, `config_dir_moves_env_override_untouched`; a failed move: `a_move_that_fails_keeps_the_old_folder_and_says_so`).
- [x] No production repo open in `graft` bypasses `RepoStore` (allow-list with reasons) — `graft/tests/it/rename_opens.rs::every_open_settles_first` (source grep).
- [x] No `"graft"` path segment outside `on_disk` and the 27 hand-off — `graft/tests/it/rename_opens.rs::no_old_dir_literal` (source grep).
- [x] `git fsck` clean after migration — `rename.rs::fsck_clean_after`.

## Handed to 27 (copy into NOTES' G-rename list)
Crash dir `<state_home>/graft` → `tributary`, old reports untouched (27 greps that nothing reads the dir back; if
something does, stop); backup identity `graft` → `tributary` (27 confirms nothing reads it); `cli/man.rs` path text;
`is_graft_internal`/`GRAFT_*` identifiers are code names, 27's choice.

## Touches
graft-core `on_disk`/`ref_name`/`OperationKind` (+ MAP); graft-git repo open, ref filter (`refs.rs:176`), commit-graph
offer (+ MAP); graft-ops snapshot/chain/backup names, journal reader + raw readers, fetched, saved plans, undo/redo/
time machine, `kind_name` and kind words (`record.rs:473`), (+ MAP); graft repos hook, config, recents, first-launch,
layout, outside editor, stash scratch, status row, J rows, `update/clash.rs:373`; tests and walks asserting
`refs/graft` (scout's count list); TESTING.md's 3 mentions. Each checked by grep or test, named in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
A ~35k, B ~60k. Two contexts recommended.

## Conductor addition (from 25 architecture pass) — resolved above
Trailer: 26, read both / write new. `plan.json`: migrated in 26. Order: see header.

## Conductor note (2026-10-09)
User approved, asked at the stop-before-26: migrate **as specified**: copy, read back, then delete the old names. No
keep-old-copies period, no move-back command. An older graft binary afterwards sees no undo history in a migrated
repo; that is accepted.

## NOTES

### Part A (commit 1) — builder-opus, 2026-10-10
No compaction. Scout used: `scouts/26-rename-on-disk.md` (no new scout; its site list was enough, the gaps below I
checked by grep myself).

**What A did.** `graft-core::on_disk` (`OnDiskNames { New, Old }`, `ALL`, `ref_prefix`/`dir`/`link_trailer`,
`MIGRATED_MARKER`); `GRAFT_INTERNAL` deleted, `is_graft_internal` matches both prefixes; `OperationKind::Migrate`
(serde `migrate`, `kind_name` = `"rename"`, added to `clash.rs` kind list). graft-git: `names_in` reads the marker
at open (`try_exists`, error → `OpenError::Io`); `RepoHandle`/`SharedRepo::on_disk_names`; every handle from a
`SharedRepo` carries its names; commit-graph offer under `<common_dir>/<dir>/`. graft-ops: `backup_ref_name(names, …)`,
`journal_path`, `rewrite/saved::root`, `GRAFT_REFS` deleted (`fetched.rs` uses `RefName::is_graft_internal`), trailer
writes `Tributary-Link:` always and reads both. graft: `RepoStore::app_dir()`; `layout` and `Ctrl+E`'s
`COMMIT_EDITMSG` take `app_dir`; `Outside::common_dir` renamed `app_dir`, `Handover` gained `app_dir`;
`config::CONFIG_NAMES` (= `Old`) is the one seam for `config.toml`, `recent`, `launches`.
No marker exists anywhere in A, so every path reads `Old` — byte-identical behaviour except the trailer write.

**Tests (A).** `graft-core on_disk::tests::{each_set_spells_its_own_names, the_prefix_is_the_dir_under_refs}`,
`ref_name::tests::both_prefixes_internal`; graft-git `repo::tests::{no_marker_reads_old_names_from_every_opener,
the_marker_in_the_common_dir_reads_new_names_in_every_worktree, a_marker_in_a_linked_worktrees_own_dir_does_not_count}`,
`tests/it/commit_graph.rs::the_decline_lives_under_the_names_in_force`; graft-ops
`snapshot::tests::{old_link_trailer_reads, the_backup_ref_takes_the_prefix_in_force}`,
`rewrite::saved::tests::plans_live_under_the_names_in_force`, `tests/it/journal_names.rs::{no_marker_writes_the_old_names,
the_marker_writes_the_new_names_and_u_still_restores}`, `tests/it/aborted_fetch.rs::old_prefix_ref_not_fetched`;
graft `repos::tests::the_app_dir_follows_the_names_in_force`, `outside::tests::the_message_file_lives_in_the_app_dir`.
Changed: `conflict_set.rs` now expects `Tributary-Link: step` (the one trailer test the ticket allowed);
`tests/it/finish.rs` helper passes the app dir.

**Touches (A), each checked.** graft-core ref_name/journal/MAP (grep `GRAFT_INTERNAL` → none left); graft-git
repo open (both openers + `handle*`), refs filter `refs.rs:176` (unchanged call, predicate widened), commit-graph
offer, MAP; graft-ops snapshot/chain/backup name, journal_path (raw readers `apply/restart.rs:243`,
`conflict/set.rs:161`, `tour.rs:240`, graft `jobs/stash.rs:46` all call `journal_path`, so they follow — grep), saved
plans, fetched, `kind_name`, MAP; graft repos, layout, outside editor, startup/listen/carry/state, config/recent/
first_launch, `update/clash.rs:373`, MAP; file-size ratchet raised for 8 files (all grew). Grep of non-test source
for `join("graft")`, `"graft/`, `refs/graft`, `"graft"` across graft-core/git/ops/graft/ui: only `crash.rs:164`
(27), `snapshot/plumbing.rs:297,299` identity (27), `graph_screen/header.rs:20` `APP_NAME` (27), and `on_disk.rs`.
Not touched: TESTING.md's 3 mentions, tests asserting `refs/graft` (still true under `Old`) — both B's.

### Handoff to Part B
- **Seams.** Names come from `RepoHandle::on_disk_names()` / `SharedRepo::on_disk_names()`, fixed at open
  (`graft-git/src/repo/mod.rs` `names_in`; `shared.rs` `open`). After `Migrated`, reopen: a `SharedRepo` never
  re-reads the marker. Config root: flip `config::CONFIG_NAMES` (`graft/src/config/load.rs`, const) to whatever
  `migrate_config_dir` decides — it is `Old` today and is the only place `config.toml`/`recent`/`launches` get the
  folder name. App dir in the common dir: `RepoStore::app_dir()` (`graft/src/repos.rs`).
- **Correction to the ticket/scout:** `COMMIT_EDITMSG` is **not** under `<git_dir>/graft/`. The loop passed
  `common_dir` as `Handover::git_dir` (`event_loop/startup/listen.rs`, was `git_dir: common_dir`), so it always
  lived in `<common_dir>/graft/` and now goes to `app_dir`. The whole-tree copy of `<common_dir>/graft/` covers it.
  `run_repo` now reads only `repos.app_dir()` (clippy's 60-line cap); `Handover::git_dir` is `app_dir`'s parent,
  the same common dir as before. After B's reopen, `app_dir` must be read **after** `settle_names`.
- **`undoable_kind` lists the non-undoable kinds** (`graft-ops/src/journal.rs:227-228`,
  `!matches!(kind, Fetch | Push)`), so **`Migrate` is undoable today** — B must add it. Likewise `is_operation`
  (`journal.rs:217-218`, `!matches!(Undo | Redo)`) counts `Migrate` as an operation; `next_undo` `journal.rs:266`,
  `next_redo` `journal.rs:282`, `plan_revert_to` `undo.rs:587` are B's to check. Harmless in A: nothing writes it.
- **`Operation` matches** (scout's 10): untouched and need no arm — `Migrate` is an `OperationKind`, not an
  `Operation`; the compiler flagged only the 2 `OperationKind` exhaustive matches (`apply/record.rs:471`
  `kind_name`, `update/clash.rs:373`), both done. `kind_name(Migrate)` = `"rename"`; J's row words are B's.
- **Read order at launch (scout's unknown), confirmed:** the first open is `places::locate`
  (`graft/src/event_loop/places.rs:79-83`, `repos.shared()`) before the terminal is set up and before any job; the
  launch burst (`update/mod.rs:253-258`) comes later. Every production open in `graft` goes through
  `RepoStore::shared` (`repos.rs:92`, the only `SharedRepo::open`; no `RepoHandle::open` outside `#[cfg(test)]` —
  grep). Also new stores at `startup.rs:81`, `places.rs:79`, `places/outside/carry.rs:49` — all via `shared()`.
- **Gotcha.** `fetched.rs` now builds a `RefName` per listed ref; an invalid name errors as `RunError::Unparsable`
  (git's own names pass the same rules, so expected never).
- **Gates (workspace-wide, after the last reformat).** fmt --check pass; clippy --all-targets -D warnings pass; nextest 6245/6245 passed, 14 skipped; test --doc pass.
- **Compacted.** The builder context compacted once mid-ticket. Part A is complete; what remains is Part B only.

### Part B (commit 2) — builder-opus, 2026-10-10
- **Scout used:** yes, the ticket's (`scouts/26-rename-on-disk.md`) plus Part A's handoff; no new scout.
- **Built:** graft-ops `rename` (+ `marker`, `refs`, `trees`, `record`): the order in the module doc, lock
  `<common>/tributary/lock` (graft-git `file_lock`, nix `flock`). Journal read mapping (`map_entries`) in
  `Journal::open_as` and the three raw readers (`apply/restart.rs`, `conflict/set.rs`, `tour.rs`); bytes never
  rewritten (golden `m5-journal.jsonl` byte-identical, `rename/tests::the_m5_golden_reads_its_backups_under_the_new_prefix`).
  `undoable_kind` excludes `Migrate`. graft: `rename::open_settled` inside `RepoStore::shared` (reopen counted as one
  open); `rename::words` → first frame's status row via `places::drive`; config folder move in `config::load`
  (`config/folder.rs`), `config_names()` replaces the `CONFIG_NAMES` const.
- **Decisions (judgement, reviewer please check):**
  - Nothing old at all → marker `clean`, `Done`, **no J row** (a fresh repo gets `.git/tributary/{lock,migrated}`
    and one `for-each-ref` on its first open, nothing after).
  - A new ref left at a *different* value (read-back mismatch, then relaunch) is refused, never overwritten: the
    repo stays `KeptOld` with words until someone removes it. Safe, but sticky.
  - A crash between the `Migrate` entry's pending and outcome lines leaves a pending `Migrate` (recovered as
    interrupted by the usual path). Tiny window; not tested.
  - "Env override untouched" read as: only the config home in force is moved; "warned once" as one warning per
    launch while both folders exist (the old one is never removed for him).
  - A failed config move keeps `graft/` for the run (`ConfigWarning::ConfigFolder`), next launch retries.
  - The `Migrate` row's summary is empty and it carries no mark: `u` passes it silently. A fetch row says
    `not undoable`; the rename row does not. **Question** for the usability walk, not changed.
  - The `Migrate` entry's time is the wall clock, not the session clock (`GRAFT_NOW`): under a pinned capture clock
    it reads `future`. Only a capture that migrates would show it; none does now.
- **Fixtures (cause fix, beyond the ticket's letter):** the graft-fixtures corpus is now built with the marker
  (`corpus.rs::names_settled`). Without it the first test to open a *borrowed* corpus repo wrote the marker and lock
  into the shared corpus (seen: `target/fixtures/<key>/linear/.git/tributary/` appeared mid-run) while other tests
  copied it — nondeterministic copies and a copy-vs-write race. The literal is pinned to `on_disk` by
  `rename_status::kept_old_words`. Corpus key changes → one rebuild per target dir. Part A's two "Old with no
  marker" tests (`graft-git commit_graph::the_decline_lives_under_the_names_in_force`, `graft-ops
  rewrite::saved::tests::plans_live_under_the_names_in_force`) now remove the fixture's marker first.
  `m7_walk/journal.rs::with_context` settles before driving ops, as the app would (else the fixture's history was
  written under old names and the app's first open added a `rename` J row to the M7 captures).
- **Captures:** none changed (`docs/shots/` untouched); the M7 J captures pass unchanged.
- **Proposed change:** MSRV 1.88 → 1.89 would let `file_lock` use std `File::lock` and drop nix's `fs` feature.
  The bump was refused by the permission classifier (shared root `Cargo.toml`); not worked around.
- **Touches:** graft-core none (Part A). graft-git `file_lock` (new) + nix `fs` + MAP. graft-ops `rename` (new),
  `journal.rs` (`undoable_kind`, `journal_path_in`), `journal/file.rs` (`open_as`), raw readers ×3, `rename/refs`
  clippy fix, `rewrite/saved.rs` test, MAP; 26 test files' literals → new names (`support::init` creates repos
  migrated; `init_old_names` for the old). graft `repos.rs`, `rename.rs` (new), `places.rs`, `config/{load,mod,
  folder}`, `recent.rs`, `first_launch.rs`, MAP; 27 integration test files' literals; `over_400.txt` (rows raised:
  restart.rs 407, journal.rs 404 new, journal/tests 807, snapshot_raw 597, support 459, repos.rs 549,
  ops_execution_hard 1135, rewrite_plan 682, config/tests 550, view/journal/tests 602, m3_walk 1812,
  m7_walk/journal 535, rewrite_editor 507, undo_over_drift 637 — literal rewraps and new tests). graft-fixtures
  `corpus.rs`. TESTING.md lines 1381, 1437, 1518–1520. Checked by: the full suite, `rename_opens` greps,
  `map_coverage`, `file_size`.
- **Handed to 27 (G-rename list):** crash dir `<state_home>/graft` → `tributary`, old reports untouched (27 greps
  that nothing reads the dir back; if something does, stop); backup identity `graft` → `tributary` (27 confirms
  nothing reads it); `cli/man.rs` path text — **now wrong** for config/recent/launches (they live in
  `tributary/`); `graph_screen/header.rs` `APP_NAME`; `is_graft_internal`/`GRAFT_*` identifiers are code names,
  27's choice. All five sit on `rename_opens::no_old_dir_literal`'s allow-list, each with its reason.
- **Not verified:** a real second window racing the move (only the in-process lock test); kill points other than
  after the create transaction; the status row drawn on a real first frame (tested at `RepoStore::rename_words`,
  which `places::drive` joins in — not captured).
- **Compacted.** The builder context compacted once during Part B. Nothing remains open from the ticket.
- **Gates (workspace-wide, after the last reformat).** fmt --check pass; clippy --all-targets -D warnings pass;
  nextest 6273/6273 passed, 15 skipped; test --doc pass.
