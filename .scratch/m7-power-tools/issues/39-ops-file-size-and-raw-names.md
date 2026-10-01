# 39-ops-file-size-and-raw-names — five graft-ops files back under 400 lines, `raw` names only bytes, and the docs describe `raw/`

Status: ready
Source: review `docs/reviews/m7-30p-to-30w.md` Major M1 (files over ~400 lines, PROMPT.md 6.2:436), M2 (`raw` names a
failure and a tree), M3 (`ARCHITECTURE.md:42`, `docs/design/06-undo-journal-design.md:84-107` predate ADR-0052).
Grouped: M1 and M2 share `set_aside/put_back.rs`, `answer.rs`, `apply.rs`; M3 names the files M1 moves.
Build with: **Sonnet** — pure moves and mechanical renames; every name is decided below, no design left.
Scout: done, `.scratch/m7-power-tools/scouts/39.md` (HEAD 0387e7fb). Read it first; its line numbers are the ones used here.
Needs: none.
Blocked by: none (36 merged).
File set: graft-ops only — `src/snapshot.rs`, `src/snapshot/raw.rs`, new `src/snapshot/raw/{converted,tests}.rs`, new
`src/snapshot/commit.rs`, `src/apply/error.rs`, new `src/apply/error/{words,tests}.rs`,
`src/apply/set_aside/{put_back,answer,returned}.rs`, new `src/apply/set_aside/{put_back,answer}/tests.rs`,
`src/apply/set_aside.rs`, `src/apply.rs`, `crates/graft-ops/MAP.md`; plus `ARCHITECTURE.md`,
`docs/design/06-undo-journal-design.md`. **Not graft, not graft-ui** (scout 39.md:24 — every renamed type is
`pub(super)`). Disjoint from 40-hint-take-and-amend (graft + graft-ui).

## Read first
Scout `39.md` (whole, 1 page). graft-ops `MAP.md`: `snapshot`, `apply::error`, `apply::set_aside`. ADR-0052.
`ARCHITECTURE.md:42` only. Design `06-undo-journal-design.md` §6.3 (:84-107) only.

## Scope
**Order: two commits.** (a) the moves, no renames — so `git diff -M` reads as moves; (b) the renames and docs.

**1. Splits (pure moves; public paths unchanged, re-exported).** Line numbers from the scout.
- `snapshot/raw.rs` (466): `converted` (:159), `converting_paths` (:188) and the helper only they call
  (`split_nul_keeping_empty` :218, if so — else it stays) → `snapshot/raw/converted.rs`; `mod tests` (:410-466) →
  `snapshot/raw/tests.rs`. `raw_record` (:80) stays and stays re-exported at `snapshot.rs:41`.
- `snapshot.rs` (426): the commit writers `commit_trees` (:353), `commit_joined` (:367), `stash_create_on_copy` (:383),
  `scratch_commit` (:406), `move_backup_ref` (:416) → `snapshot/commit.rs` (≈73 lines out → ~353). `mod tests;` stays.
- `apply/error.rs` (493): `not_as_they_were_words` (:299), `staging_not_kept_words` (:320), `staging_clause` (:331),
  `staging_named` (:340), `journal_failed` (:352), `joined` (:365) → `apply/error/words.rs`; `mod tests` (:450-493) →
  `apply/error/tests.rs`. `reason_for`, `as_apply_error` stay.
- `set_aside/put_back.rs` (449): `mod tests` (:336-449) → `put_back/tests.rs`. `set_aside/answer.rs` (436): `mod tests`
  (:237-436) → `answer/tests.rs`.
- Each ends ≤ 400 lines. One that doesn't: split once more at a seam the builder names in NOTES.
**Not split in M7 (decided):** `operation.rs` (1835), `apply.rs` (705), `undo.rs` (833) — over before this range; a
split of `operation.rs` is a module redesign, the post-M7 `improve-codebase-architecture` pass (CLAUDE.md Skills).

**2. Names (M2).** `raw` means a tree or bytes, nothing else. Every failure-typed `raw` is named for what it holds:

| Site (scout 39.md:13-14) | Type | New name |
|---|---|---|
| `PutBackRan::Whole { raw }` put_back.rs:78, 84, 107-108, 128 | `NotAsTheyWere` | `not_as_they_were` |
| `Returned::Back`, `Kept::Git`, `Kept::Unlisted` fields + uses, returned.rs:25, 43, 54, 63, 65, 169-171, 184, 189, 191 | `NotAsTheyWere` | `not_as_they_were` |
| `Outcome` `raw` fields answer.rs:161, 175 (variant names as read) + uses in answer.rs:79-220 | `NotAsTheyWere` | `not_as_they_were` |
| `Returned::Back{unstaged, raw}` → `Outcome::Back{.., raw}` set_aside.rs:266 | `NotAsTheyWere` | `not_as_they_were` |
| **fn** `not_as_they_were(raw: NotAsTheyWere)` answer.rs:208; calls :106, :145, :220 | fn | **`answer_not_as_they_were`** — renamed first, or the new binding shadows it (locals shadow items) |
| fn `with_raw` answer.rs:218 | fn | `with_not_as_they_were` |
| test helper `raw_failed()` (answer tests, used :327-419) | returns `NotAsTheyWere` → | `sample_not_as_they_were()`; any other type → `sample_failure()` |
| `Option<AfterState>` apply.rs:409, 419, 431 | failure | `after_failed` |
| formatted failure set_aside.rs:296-297 | `String`/`Option<String>` | `failed_words` |

Kept as `raw` (tree/bytes): `HisBytes.raw: Oid` (put_back.rs:45), set_aside.rs:264, `Step::RestoreWorktree { raw }`
(undo.rs:67, :341), `worktree_raw_before` (restore.rs:301). The comment at `apply.rs:407-408` explaining the name goes.
`Kept::Ignored` in graft's `tests/it/in_the_way_status.rs` is a different type — untouched.

**3. Docs (M3), two sentences each, pointing at ADR-0052.**
- `ARCHITECTURE.md:42`: the snapshot writes `index/` or `unmerged/`, `worktree/`, and, when bytes differ from what git
  gives back, `raw/`; name `snapshot/raw.rs` (+ `raw/converted.rs`) and `restore/raw.rs` beside `restore/outcome.rs`.
- Design §6.3 (:84-107): the backup also keeps `raw/` — the bytes the index/checkout path would not give back — and a
  put-back writes them byte-exact (ADR-0052); replace "ordinary index/checkout path" (:104) accordingly.

## Signatures
No new API, no type change. Field renames (exact variant shapes as read):
`PutBackRan::Whole { not_as_they_were, .. }`, `Returned::Back { not_as_they_were, .. }`,
`Kept::Git { not_as_they_were, .. }`, `Kept::Unlisted { not_as_they_were, .. }`,
`Outcome::Back { .., not_as_they_were }` and the `Outcome` variant at answer.rs:175.
Fns: `fn answer_not_as_they_were(not_as_they_were: NotAsTheyWere) -> /* unchanged */`,
`fn with_not_as_they_were(..)` (was `with_raw`, signature otherwise unchanged).

## Acceptance (each box names its test at tick time)
- [ ] `wc -l` of the five files and every new file ≤ 400, pasted in NOTES (the check: the numbers, not a test).
- [ ] Pure move (commit a): `git diff -M --stat` shows moves; no test body changed. Named check: graft-ops suite green
      **unchanged in count** (before/after nextest totals in NOTES, after each commit).
- [ ] Renames (commit b): `grep -nw raw crates/graft-ops/src` and `grep -n 'raw_\|_raw' crates/graft-ops/src` in NOTES,
      every hit a tree or bytes (or a kept site above). Tests changed only by `use` lines and the renamed identifiers.
- [ ] Byte-exact promise unchanged: graft-ops `tests/it/{undo_bytes,snapshot_raw,put_back_index,set_aside,
      put_back_forecast}.rs` green, test names listed in NOTES; the M7 acceptance's raw-byte scenarios green.
      (`journal_put_back.rs` does not exist — scout 39.md:22.)
- [ ] Docs: the two edits quoted in NOTES; `ARCHITECTURE.md` names only files that exist (`ls` each in NOTES).
- [ ] graft-ops MAP.md: `snapshot` file list (`raw/converted.rs`, `commit.rs`), `apply::error` (`error/words.rs`),
      set-aside tests files, the renamed fields/fns.
- [ ] Gates.

## Touches (check each; report how)
- **Other crates:** none expected — renamed types are `pub(super)`. Check: workspace builds; `grep -rn
  'not_as_they_were\|PutBackRan\|Returned::\|Kept::' crates/graft crates/graft-ui` in NOTES. A hit → `blocked: needs
  graft`, stop.
- **Re-exports:** `snapshot::*` (incl. `raw_record`), `apply::error::*` paths unchanged. Check: workspace builds untouched
  outside graft-ops.
- **Doc readers:** `ARCHITECTURE.md` points ops readers at §6.3 first — both now agree with ADR-0052.
- **Captures, keymap, UI, journal format:** none (no behaviour change; `worktree_raw_before` kept).
- **For the conductor:** closes review m7-30p-30w M1–M3; three KNOWN_GAPS rows (`operation.rs`, `apply.rs`, `undo.rs`
  over 400 lines → post-M7 `improve-codebase-architecture` pass).

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~40k. One context.
