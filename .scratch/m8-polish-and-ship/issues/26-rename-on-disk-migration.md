# G-rename 1 — his refs, journal and config move to the new name, once, safely

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus, alone**: it moves his live undo history; a
migration that half-runs must leave the old names in force. Nothing builds beside it (ADR-0057 Consequences).
Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25
Needs: every M8 build ticket and the G-health tickets from `fable-architecture-pass` merged. Before
`rename-names-sweep`.

## Read first
- `docs/adr/0057-tributary-and-what-ships.md` §2 and Consequences
- `docs/adr/0010-journal-per-worktree-backup-refs-namespaced.md` — Decision only
- `docs/design-notes/m8-polish-and-ship.md`: Hard-to-reverse choice 3
- `crates/graft-ops/MAP.md` `## journal`, `## journal (file handle)` (`journal_path`), `## apply` (backup refs);
  `crates/graft/MAP.md` `layout`, `config` (`default_path`), recents, first-launch; `crates/graft-git/MAP.md` refs writes
- Every M8 ticket's NOTES "G-rename list" line (the conductor collects them into this ticket's NOTES before dispatch)

## Decisions (planner)
- **What moves**: `refs/graft/**` → `refs/tributary/**`; `<git-path>/graft/` (journal, per worktree) and
  `<common_dir>/graft/` (layout etc.) → `…/tributary/`; config dir `<config>/graft/` → `<config>/tributary/` (config,
  `recent`, `launches`). Env `GRAFT_*` → `TRIB_*` is the sweep's (names only, no data).
- **When**: on opening a repository, before any read of the journal or backup refs; config dir at startup. Per
  repository and per worktree, idempotent, cheap when done (one marker read).
- **How**: refs by one `git update-ref --stdin` transaction creating the new names at the same oids, then verified by
  reading back, then a second transaction deleting the old names with their old oids as expected values. Journal and
  dir files copied, read back through the real reader and compared entry by entry, then the old copy removed. Journal
  entries naming `refs/graft/…` are read through one mapping (old prefix → new) at read time; the file's bytes are not
  rewritten.
- **Recorded**: one journal entry `rename to Tributary (N refs, M files)`, kind `Migrate`, not undoable (`u` passes
  over it like a fetch), so J shows it happened.
- **Fails part-way**: whatever step failed, the old names stay in force for this run (`OnDiskNames::Old`), nothing
  old is deleted, and the status row says `rename: still using the old names · <reason>`. The next launch tries again.
- **Never**: deletes an old ref or file before its new copy reads back the same; touches the working tree or index.

## Signatures
```rust
// graft-ops::rename
pub enum OnDiskNames { New, Old }
impl OnDiskNames { pub fn ref_prefix(self) -> &'static str; pub fn dir(self) -> &'static str; }
pub struct Migrated { pub refs: usize, pub files: usize, pub in_force: OnDiskNames }
pub enum MigrateError { Refs { words: String }, Files { path: PathBuf, words: String }, ReadBack { what: String } }
pub fn migrate_repo(ctx: &Context) -> Result<Migrated, (OnDiskNames, MigrateError)>;
pub fn map_old_ref(name: &str) -> std::borrow::Cow<'_, str>;
// graft (pub(crate))
pub(crate) fn migrate_config_dir(config_root: &Path) -> Result<ConfigDirMoved, std::io::Error>;
// Operation gains `Migrate`; every exhaustive match updated (`moves_head` false, `undoable_kind` false).
```
Every reader and writer of the ref prefix and the dir name takes `OnDiskNames` (or the constant it yields) — the scout
lists them; no literal `"graft"` path segment survives outside `rename`.

## Acceptance
- [ ] A repo with 3 backup refs and a 20-entry journal (built by the M7 walk's fixtures, old names): after open, refs
      under `refs/tributary/`, none under `refs/graft/`, journal reads the same 20 entries + `Migrate`, `u` undoes the
      newest real operation byte-exactly (test, truth by git).
- [ ] Kill after the create transaction (test hook): relaunch completes; no ref lost, none duplicated afterwards.
- [ ] Read-back mismatch injected → old names in force, nothing old deleted, status line shown (test).
- [ ] Second open: no spawn beyond the marker read (test counts spawns).
- [ ] Linked worktree: its own journal migrates; shared refs once (test).
- [ ] Config dir: copied, read back, old removed; a config dir that exists under both names keeps the new and warns
      once (test).
- [ ] `git fsck` clean after migration (test).

## Touches
- Every ref-prefix and dir-name site (scout list); journal reader; backup ref writers; layout file; recents; first-
  launch file; tour temp name (already new); J rows (new `Migrate` kind words); undo/redo skipping; `Operation`
  exhaustive matches; M3–M7 walks that assert ref names (grep `refs/graft`); MAP.md graft-ops, graft, graft-git.

## Scout questions
- Every literal `refs/graft`, `"graft"` path segment and config-dir name in non-test code (file:line, by crate).
- Does anything outside graft-ops write under `refs/graft/` (e.g. stash pins, chain backups)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k tokens.

## Conductor addition (from 25 architecture pass, docs/reviews/m8-architecture.md M4 — as found, no new decision)

- On-disk names are format contracts. Two the pass named for 26/27: the `Graft-Link:` backup trailer — readers accept the old and new spelling, or 27's grep test allow-lists it, with a reason; and the rewrite's `plan.json` directory — migrated before a relaunch reads it. Each gets a box with a test in whichever of 26/27 owns it.
- Order (conductor): health 33–38 merge before 26; 39–42 need 27's final names, so they run after 27 (they are the exception to "G-health merged" above). 05 is deferred to last by the user (2026-10), so it is not a blocker here.

## Conductor note (2026-10-09)

User approved, asked at the stop-before-26: migrate **as specified**: copy, read back, then delete the old names. No keep-old-copies period, no move-back command. An older graft binary afterwards sees no undo history in a migrated repo; that is accepted.
