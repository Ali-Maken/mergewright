# 26r — rename review fixes: a failed move retries cleanly, an older build's writes are never deleted unread

Status: done — Part A (builder-opus, 2026-10-10); Part B (builder-sonnet, 2026-10-10)
Blocked by: 26
Build with: Part A **Opus**: the order is fully set out below, and every window gets a test that injects a writer at
that step, so correctness can be judged by the tests. Fable only if the conductor's budget has room. Part B **Sonnet**:
a small config folder rule plus fixtures, following Part A's rule. Brownfield: the review file's line refs serve as
the scout. The conductor confirms both.
Needs: 26 merged (`settle_names`, `SettleOutcome`, `MigrateError`, `migrate_config_dir`). Part B needs no signature
from Part A. Disjoint files: run them concurrently, two builders at most.
Spec-only ruling: planner, 2026-10-10, on `docs/reviews/m8-26-rename.md` Major M1–M3 and Questions 1–3.

## Read first
- `docs/reviews/m8-26-rename.md`: Major M1, M2, M3; Minor 1–5 and 9; Questions
- `.scratch/m8-polish-and-ship/issues/26-rename-on-disk-migration.md`: Decisions (Marker, Two processes, Status row), conductor note 2026-10-09
- `crates/graft-ops/MAP.md` `## rename`, `## journal (file handle)`; `crates/graft/MAP.md` config, rename; `crates/graft-fixtures/MAP.md`
- Source, Part A: `graft-ops/src/rename.rs` (`copy_everything`, `stay_old`, `remove_old`, `clean_up`), `rename/trees.rs`, `rename/refs.rs`. Part B: `graft/src/config/folder.rs`, `graft-fixtures/src/{synthetic,team,corpus}.rs`

## Rulings
- **M1: before the switch, the new names are the move's own scratch.** The marker is absent, so no run has ever
  read them as the user's. Each attempt therefore makes the new side an **exact copy** of the old:
  - a new file that differs is overwritten (temp file, sync, rename);
  - a new file or ref with no old counterpart is deleted (`lock` and `migrated` excepted);
  - a new ref at another oid is updated to the old oid, with the read value as the expected value.
  - Not rollback-on-failure: rollback cannot run after a kill, and this rule covers a kill too. "Refuse on
    conflict" stays for the post-switch pass below.
  - Why the extras are deleted: a stale `rewrite/<id>/plan.json` from a failed attempt would otherwise bring back a
    paused rewrite that has since finished.
- **M2: (b) as a byte rule plus (a) as detection, joined in one stray pass. No journal merge.**
  - **The stray pass** (`settle_strays`) replaces `remove_old` (step 8) and `clean_up`'s removal. It runs under the
    exclusive lock. Pass dir `P` = `<common>/tributary/older-version/<n>/`, `n` = the first unused integer.
    - **Files.** Each old file is first **renamed** into `P/<path relative to common>`. Same filesystem, so this is
      atomic, and an older build that opens by path after this recreates the old path for the next pass. Then:
      - equal to its new copy → deleted;
      - no new copy → copied forward, read back, then deleted;
      - differs → **stays in `P`**.
    - Then empty old dirs, an empty `P` and an empty `older-version/` are removed. Created dirs and their parents
      are fsynced (Minor 2).
    - **Refs.** One `update-ref --stdin` transaction per pass, read back after:
      - no new ref → create the new ref and delete the old one at its oid;
      - same oid → delete the old one;
      - different oid → move it to `refs/tributary/older-version/<n>/<rest>` and delete the old one.
  - **Detection on `clean`, no spawn.** Check for a **non-empty** `<common>/graft/`, `<common>/refs/graft/` and the
    opening worktree's `<git_dir>/graft/` (stat plus `read_dir`). Any found → take the lock, run the pass. Empty →
    `Done`. The zero-spawn tick holds.
  - **No `Migrate` entry for a stray pass.** It is not an operation.
  - **Why not merge the journals.** An older window's entries interleave in time with this build's. Merging means
    rewriting the append-only file while that window may still append. The conductor note already accepts that
    histories don't cross the version boundary. What's new here is that nothing is **deleted** unread, and nothing
    goes **unsaid**.
  - **Not a spec change.** Old copies are still deleted once they read back the same, with no keep-old period and
    no move-back. `older-version/` only ever holds bytes an older build wrote **after** their copy was taken, and
    those never had a read-back copy. This is the ticket's own "never deletes … before its new copy reads back the
    same", applied literally (it also closes Minor 1).
  - **Residual, stated rather than fixed.** These go to `KNOWN_GAPS.md` as **unverified**:
    - an older build that holds its file open across the rename and writes between the compare and the delete;
    - a linked worktree's fetch-only journal with no ref or layout write is not detected until something else
      triggers the pass.
- **M3:** `fn clean_up_steps(repo) -> Result<(usize, usize), MigrateError>`, and `clean_up` matches on it. Same
  shape as `copy_everything`.
- **Q1:** decided above: the status row plus the stray pass.
- **Q2: same byte rule for the config folder.** A warning appears only on the launch that moved something, never
  on every launch.
  - Both folders present: each old file is equal → deleted; absent in new → copied forward; differs → moved to
    `<config_root>/tributary/older-version/<n>/`. Then the old folder is removed when empty.
  - Hold an flock on `<config_root>/tributary.lock` around the whole move (Minor 4).
  - A failed removal carries the `io::Error` words (Minor 3).
- **Q3:** no study needed. The move is invisible when it works and has no journey. The conductor adds one line to
  the design note's Prior art: `on-disk rename: not user-facing; invent (no journey)`.
- **Conductor's, not this ticket's:** Minor 6 and 11 (ticket text), 7 (ratify nix `fs`), 8 (check against 27's
  list), 10 (file the gap rows, plus the two residuals above).

## Signatures
```rust
// graft-ops::rename
pub enum SettleOutcome {
    Done,
    Migrated { refs: usize, files: usize, not_journaled: Option<String>, kept_aside: Option<PathBuf> },
    CleanedUp { refs: usize, files: usize, kept_aside: Option<PathBuf> },
    OlderWrote { refs: usize, files: usize, kept_aside: Option<PathBuf> }, // new: strays found on `clean`
    KeptOld(MigrateError),
    LeftOld(MigrateError), // a stray pass that fails or is Busy lands here too
}
pub const KEPT_ASIDE_DIR: &str = "older-version"; // under <common>/<New.dir()>/ and refs/<New.dir()>/
// crate-private
pub(crate) struct Strays { pub refs: usize, pub files: usize, pub kept_aside: Option<PathBuf> }
fn settle_strays(repo: &RepoHandle) -> Result<Strays, MigrateError>;      // step 8, clean_up, on-clean detection
fn old_names_present(repo: &RepoHandle) -> std::io::Result<bool>;        // stat + read_dir only
fn clean_up_steps(repo: &RepoHandle) -> Result<(usize, usize), MigrateError>;
// graft (pub(crate)), Part B
pub(crate) enum ConfigDirMoved { /* existing variants */ OlderWrote { kept_aside: Option<PathBuf> } }
```

## Status row words (graft `rename::words`; nothing on `Done`)
- `OlderWrote` / `Migrated` / `CleanedUp`, with `kept_aside: Some(p)`: `rename: an older version wrote under the old names · close it; kept in <p relative to the repo root>`
- `OlderWrote`, `kept_aside: None`: `rename: an older version wrote under the old names · close it`
- Config: `config: an older version wrote <old folder> · close it; kept in <p>` (or without `; kept in`).

## Part A: graft-ops `rename`, graft `rename::words`
Acceptance (each box names its test, all in `graft-ops/tests/it/rename.rs` unless stated). Ticked by the Part A
builder, 2026-10-10. The new tests live in the submodule file `graft-ops/tests/it/rename/review.rs`, so their full
names are `rename::review::<name>`.
- [x] Copy fails part-way (test hook), then the old journal grows by one entry and an old `rewrite/<id>/plan.json` is
      removed. The next settle gives `Migrated`. The new journal is byte-equal to the grown old one, there is no
      plan for `<id>` under the new names, and `git fsck` is clean — `rename::review::a_failed_copy_retries_cleanly`.
      Caveats: the part-way failure is a symlink in a linked worktree's old folder, not a hook. The common dir's
      folder is copied first, and the test asserts those copies exist. The new journal is the grown old one
      **followed by the move's own `Migrate` entry**, which step 6 appends. The test asserts the byte prefix and
      exactly one `Migrate` entry after it.
- [x] Before the switch, a new ref at another oid is updated, and a new ref with no old counterpart is deleted —
      `rename::review::a_stale_new_ref_before_the_switch_is_overwritten`.
- [x] A hook between copy and step 8 appends to an old journal. The new journal equals the copy, and
      `older-version/1/<path>` is byte-equal to the appended file. The outcome is `Migrated { kept_aside: Some }` and
      the old tree is gone — `rename::review::an_old_file_written_after_its_copy_is_kept_aside`. The hook is the git
      shim on the stray pass's first spawn: the one `for-each-ref` naming both prefixes, after `switched` and the
      `Migrate` entry. The new journal is asserted to start with the copy, because the `Migrate` entry follows.
- [x] A hook right after the rename into `P` writes the old path again. The next settle finds it (`OlderWrote`), and
      its bytes are in `older-version/2/` — `rename::review::a_write_after_the_rename_is_found_next_open`. The hook
      fires at the pass's ref transaction, which comes after every old file has been renamed into `P` and compared.
      No spawn falls between one file's rename and its compare.
- [x] With the marker `clean`, an older build's `<common>/graft/layout` (differs), a new `refs/graft/x` and a linked
      worktree's old journal give `OlderWrote { refs: 1, .. }`. The ref is under `refs/tributary/`, the layout is kept
      aside, no old name is left, and a second settle is `Done` with zero spawns —
      `rename::review::an_older_build_writing_after_clean_is_carried_or_kept`.
- [x] An empty `<common>/refs/graft/` with the marker `clean` gives `Done` and zero spawns —
      `rename::review::an_empty_old_refs_dir_spawns_nothing`. `clean_marker_spawns_nothing` stays green.
- [x] A ref already under `refs/tributary/` at another oid goes to `refs/tributary/older-version/1/…`. Nothing is
      deleted unread — `rename::review::a_differing_stray_ref_is_kept_aside`.
- [x] Minor 5: the copied journal is parsed with `map_entries(New)`. A `backup_ref` it names that is missing under
      `refs/tributary/` gives `KeptOld(ReadBack)` —
      `rename::review::a_copied_journal_naming_a_missing_backup_is_refused`. The tautological check is deleted.
      This ruling is questioned in NOTES.
- [x] Minor 2: each created dir and its parent is synced before `switched` —
      `rename::trees::tests::every_created_dir_is_synced` (a pure function that lists the dirs to sync).
- [x] M3: no closure is left in `clean_up`. `switched_marker_cleans_leftovers` stays green. Its `CleanedUp` literal
      gained `kept_aside: None`. Its `layout` now has no new copy, so it is carried forward rather than deleted, and
      the counts are unchanged.
- [x] Words for every outcome, the new ones included — `graft/tests/it/rename_status.rs::every_outcome_has_its_words_or_none`
      (extended, plus its helper `older_version_words`).
- [x] Every existing 26 test stays green. `read_back_mismatch_keeps_old` still gives `KeptOld` on **that** attempt,
      and its relaunch now migrates (assert added) — `rename::read_back_mismatch_keeps_old`, together with the
      workspace nextest run (see NOTES).

## Part B: graft config folder, graft-fixtures
Ticked by the Part B builder, 2026-10-10.
- [x] Both folders present: equal → deleted; absent → carried forward; differs → `older-version/1/`, warned on that
      launch only. A second launch with no new old files gives no warning —
      `graft config::folder::tests::both_present_files_carry_or_go_aside` (also: a third launch after a new old
      write lands in `older-version/2/`).
- [x] Two moves at once (two threads, two fds) move once, with no `tributary.moving` cut short —
      `config::folder::tests::two_launches_move_the_config_once` (20 rounds).
- [x] A failed old-folder removal says why (the `io::Error` words) —
      `config::folder::tests::a_failed_old_folder_removal_says_why`.
- [x] Minor 9: synthetic and team fixtures are built with the marker. The cache key rebuilds once —
      `graft-fixtures synthetic::tests::built_with_the_marker` (heavy-tests feature only; run and passed),
      `settled::tests::team_built_with_the_marker` (moved out of `team.rs`: the file-size ratchet holds it at 525).
- [x] Replace `config_dir_moves_both_present`, which asserted the old warn-every-launch behaviour (say so in NOTES) —
      replaced by `both_present_files_carry_or_go_aside`.

## Touches
- graft-ops `rename{,/trees,/refs,/marker}`: `SettleOutcome` gains fields and a variant. Every match on it must be
  updated: graft `rename.rs`, `repos.rs`, `places::drive`, and the tests.
- graft-ops MAP `## rename`: the stray pass, `older-version/`, pre-switch overwrite.
- graft `rename::words`; graft MAP: config folder rule.
- graft `config/folder.rs`, `ConfigWarning::ConfigFolder` words, `config/tests`.
- graft-fixtures `synthetic.rs`, `team.rs`, MAP: the cache key changes, so there is one rebuild per target dir.
- `rename_opens::no_old_dir_literal`: `older-version` is not `"graft"`. Confirm by running it.
- `cli/man.rs` path text: 27 names `older-version/`. Add it to the G-rename list in NOTES.
- No captures, keys, journal kinds or undo semantics change. The `Migrate` J row is unchanged. Check with the
  journal tests and `migrate_is_transparent_to_undo`.
- `docs/design-notes/m8-polish-and-ship.md` Q3 line: the conductor's.
- `KNOWN_GAPS.md`: the residuals, filed by the conductor.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
A ~50k, B ~25k. Two contexts.

## NOTES — Part A (builder-opus, 2026-10-10)
No compaction. Scout: ticket 26's scout plus the review's file:line refs; no new scout (I read the rename sources directly, since every one of them is edited).

**Built.**
- Pre-switch mirror: `refs::mirror_and_read_back` and `trees::mirror`, plus `trees::new_without_old` for a new folder whose old one is gone.
- Stray pass: new `rename/strays.rs`, holding `settle_strays` and `old_names_present`.
- `clean_up_steps` replaces the closure.
- Backup check: `rename/journals.rs`.
- `dirs_to_sync` and the folder fsyncs.
- `words(outcome, root)`.
- `map_ref`/`map_entry`/`map_entries` moved to `rename/map.rs` and re-exported unchanged, to keep `rename.rs` under the file-size ratchet's 400 lines.

**Deviations, for the conductor:**
- **Journal compare rule.** In the stray pass, an old journal counts as "equal" when its bytes are a **prefix** of the new one. The literal byte rule kept every migrated journal aside: step 6 appends the `Migrate` entry before step 8, so old ≠ new, and every move warned "an older version wrote". The journal is append-only, so a prefix means the old file holds nothing the new one lacks. An older build's append after the copy still differs and is still kept. `trees::holds_the_same`.
- `clean_up_steps` returns `Strays`, not `(usize, usize)`. `CleanedUp` needs `kept_aside`.
- `old_names_present` returns `Result<bool, MigrateError>`, not `io::Result`, so the error carries its path.
- `kept_aside` when only refs were set aside holds the ref prefix as a path: `refs/tributary/older-version/<n>/`. The words print it as is.
- Status words: `Migrated` with both `kept_aside` and `not_journaled` joins the two: `<older words> · not in the journal: <why>`.

**Question (ruling Minor 5):** refusing on a journal `backup_ref` that is missing under `refs/tributary/` also refuses when the **user deleted that backup ref** under the old names (`apply/error.rs:277` treats that as a real state). The move then fails on every open, for good. That is M1's stuck state again, on the status row every launch. It is built as ruled. A parse error in the copy is skipped, with a comment saying why it is safe. The proposed alternative is to refuse only when the old ref exists and the mapped new one does not, which makes the check near-tautological. Conductor's call.

**Residuals (unverified; for KNOWN_GAPS):**
- The two from the ruling.
- A `switched` clean-up after a `LeftOld` run that rewrote `tributary/layout` keeps the old `layout` aside, and shows a false "older version wrote" once. Only append-only journals get the prefix rule.
- Refs that exist only in `packed-refs` are not detected on `clean`.

**Touches:**
- `SettleOutcome` matches: grep `SettleOutcome::` finds only `graft/src/rename.rs` and tests. `repos.rs` and `places::drive` don't match on it. `repos.rs` passes `settled.root`.
- `rename_opens::no_old_dir_literal`: green.
- The journal and undo tests and `migrate_is_transparent_to_undo`: green in the workspace run. `record.rs` is unchanged.
- No captures, keys or journal kinds touched.
- MAPs updated: graft-ops `## rename`, and the graft `rename` line.
- For the G-rename list (27): `cli/man.rs` should name `older-version/`.

**Verified:**
- Each ticked test passed.
- Workspace nextest: 6281/6282 on the first run. The one red was the `file_size` ratchet, fixed by the file split. The second run: 6282/6282, doctests 9/9 ok.
- Not observed: power loss, and a real older binary running concurrently. The shim hooks emulate its writes.

## NOTES — Part B (builder-sonnet, 2026-10-10)
No compaction. Scout: none new; ticket 26's scout + review lines, and `folder.rs` read directly (it is rewritten).

**Built.**
- `config/folder.rs`: flock on `<config_root>/tributary.lock` (graft-git `FileLock`: `try_exclusive`, else wait on
  `shared` and retry — a kernel wait, no poll; no new dependency or nix feature in graft). Taken only when the old
  folder exists; old re-checked after the lock.
- Both present: per-file byte rule, kept aside under `<config_root>/tributary/older-version/<n>/` (`KEPT_ASIDE_DIR`
  from graft-ops). `ConfigDirMoved::BothPresent` replaced by `OlderWrote { old, kept_aside }` and
  `OldLeft { old, why }`.
- graft-fixtures: `settled.rs` holds the one marker writer (moved from `corpus.rs`), now called by synthetic `build`
  and team `borrow`; added to all three digests, so corpus, synthetic and team each rebuild once per target dir.
- `config_dir_moves_both_present` replaced: it pinned "old folder left, warned every launch".

**Deviations.**
- Words print as `config folder: an older version wrote <old> · close it; kept in <p>` — the existing Display prefix
  `config folder:` kept, not the ticket's `config:` (the prefix is shared with the move-failed warning).
- Old folder holding only files equal to the new ones → `Nothing`, silent (nothing an older build wrote is lost).
- A symlink/non-file under the old folder is left in place; the old folder's removal then fails → `OldLeft` with
  the io::Error words.
- `kept_aside` paths are absolute (config root is outside any repo).

**Not verified:** a real older binary racing; two processes (threads with separate fds stand in — flock is per open
file); power loss (dirs for `older-version/` are not fsynced, unlike Part A's tree).

**Touches:** `config/load.rs` `settle_folder` (only match on `ConfigDirMoved`, grep); graft MAP config line;
graft-fixtures MAP; `corpus.rs`, `digest.rs`, `lib.rs`, `synthetic.rs`, `team.rs`; `file_size` ratchet green.
No keys, captures, journal or undo touched.
