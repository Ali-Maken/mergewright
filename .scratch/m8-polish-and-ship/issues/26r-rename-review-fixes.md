# 26r — rename review fixes: a failed move retries cleanly, an older build's writes are never deleted unread

Status: ready
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
Acceptance (each box names its test, all in `graft-ops/tests/it/rename.rs` unless stated):
- [ ] Copy fails part-way (test hook), then the old journal grows by one entry and an old `rewrite/<id>/plan.json` is
      removed. The next settle gives `Migrated`. The new journal is byte-equal to the grown old one, there is no
      plan for `<id>` under the new names, and `git fsck` is clean — `a_failed_copy_retries_cleanly`.
- [ ] Before the switch, a new ref at another oid is updated, and a new ref with no old counterpart is deleted —
      `a_stale_new_ref_before_the_switch_is_overwritten`.
- [ ] A hook between copy and step 8 appends to an old journal. The new journal equals the copy, and
      `older-version/1/<path>` is byte-equal to the appended file. The outcome is `Migrated { kept_aside: Some }` and
      the old tree is gone — `an_old_file_written_after_its_copy_is_kept_aside`.
- [ ] A hook right after the rename into `P` writes the old path again. The next settle finds it (`OlderWrote`), and
      its bytes are in `older-version/2/` — `a_write_after_the_rename_is_found_next_open`.
- [ ] With the marker `clean`, an older build's `<common>/graft/layout` (differs), a new `refs/graft/x` and a linked
      worktree's old journal give `OlderWrote { refs: 1, .. }`. The ref is under `refs/tributary/`, the layout is kept
      aside, no old name is left, and a second settle is `Done` with zero spawns —
      `an_older_build_writing_after_clean_is_carried_or_kept`.
- [ ] An empty `<common>/refs/graft/` with the marker `clean` gives `Done` and zero spawns —
      `an_empty_old_refs_dir_spawns_nothing`. `clean_marker_spawns_nothing` stays green.
- [ ] A ref already under `refs/tributary/` at another oid goes to `refs/tributary/older-version/1/…`. Nothing is
      deleted unread — `a_differing_stray_ref_is_kept_aside`.
- [ ] Minor 5: the copied journal is parsed with `map_entries(New)`. A `backup_ref` it names that is missing under
      `refs/tributary/` gives `KeptOld(ReadBack)` — `a_copied_journal_naming_a_missing_backup_is_refused`. The
      tautological check is deleted.
- [ ] Minor 2: each created dir and its parent is synced before `switched` —
      `rename::trees::tests::every_created_dir_is_synced` (a pure function that lists the dirs to sync).
- [ ] M3: no closure is left in `clean_up`. `switched_marker_cleans_leftovers` stays green.
- [ ] Words for every outcome, the new ones included — `graft/tests/it/rename_status.rs::every_outcome_has_its_words_or_none`
      (extended).
- [ ] Every existing 26 test stays green. `read_back_mismatch_keeps_old` still gives `KeptOld` on **that** attempt,
      and its relaunch now migrates (assert added).

## Part B: graft config folder, graft-fixtures
- [ ] Both folders present: equal → deleted; absent → carried forward; differs → `older-version/1/`, warned on that
      launch only. A second launch with no new old files gives no warning —
      `graft config::folder::tests::both_present_files_carry_or_go_aside`.
- [ ] Two moves at once (two threads, two fds) move once, with no `tributary.moving` cut short —
      `config::folder::tests::two_launches_move_the_config_once`.
- [ ] A failed old-folder removal says why (the `io::Error` words) —
      `config::folder::tests::a_failed_old_folder_removal_says_why`.
- [ ] Minor 9: synthetic and team fixtures are built with the marker. The cache key rebuilds once —
      `graft-fixtures synthetic::tests::built_with_the_marker`, `team::tests::built_with_the_marker`.
- [ ] Replace `config_dir_moves_both_present`, which asserted the old warn-every-launch behaviour (say so in NOTES).

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
