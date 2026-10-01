# 39-ops-file-size-and-raw-names — five graft-ops files back under 400 lines, `raw` names only bytes, and the docs describe `raw/`

Status: ready
Source: review `docs/reviews/m7-30p-to-30w.md` Major M1 (files over ~400 lines, PROMPT.md 6.2:436), M2 (`raw` names a
failure and a tree), M3 (`ARCHITECTURE.md:41`, `docs/design/06-undo-journal-design.md:85-107` predate ADR-0052).
Grouped: M1 and M2 share `set_aside/put_back.rs`, `answer.rs`, `apply.rs`; M3 names the files M1 moves.
Build with: **blank — scout first.** Suggested after the scout: Sonnet (pure moves and renames, rule given below).
Scout: **yes, one, ≤1 page**: line counts now of the five files; each file's seams (below) at real line numbers; every
`raw` binding/field with its type (tree/bytes vs failure); which renamed fields are `pub` and read **outside graft-ops**
(grep graft) — that decides whether graft joins the crate set.
Needs: none. **Wave 2** (after 36-offer-keys-and-daily-wording, which edits `apply/remote.rs`, has merged).
Blocked by: 36
Crates / files: graft-ops `snapshot.rs`, `snapshot/raw*`, `apply/error*`, `apply/set_aside/{put_back,answer,returned}*`,
`apply.rs` (renames only), graft-ops `MAP.md`; `ARCHITECTURE.md`; `docs/design/06-undo-journal-design.md` §6.3. graft:
only the match sites the scout names, if any.

## Scope
**1. Splits (pure moves; public paths unchanged, re-exported).**
- `snapshot/raw.rs` (466) → `snapshot/raw.rs` (what to keep: `not_given_back`, `raw_tree`) + `snapshot/raw/converted.rs`
  (which paths git converts: `converted`, `converting_paths`); its `mod tests` → `snapshot/raw/tests.rs`.
- `snapshot.rs` (426) → one cohesive group out to a sibling the scout names (candidate: the commit writers,
  `commit_trees` and the join); no logic change.
- `apply/error.rs` (493) → word builders (`not_as_they_were_words`, `staging_not_kept_words`, `journal_failed`, and any
  `*_words` with them) to `apply/error/words.rs`; inline tests to `apply/error/tests.rs` if that is still needed.
- `set_aside/put_back.rs` (449), `set_aside/answer.rs` (436): inline tests → `put_back/tests.rs`, `answer/tests.rs`.
- Each ends ≤ 400 lines. One that doesn't after its named move: split once more at a seam the builder names in NOTES.
**Not split in M7 (decided):** `operation.rs` (1835), `apply.rs` (705), `undo.rs` (833) — over before this range; a
split of `operation.rs` is a module redesign, which is the post-M7 `improve-codebase-architecture` pass (CLAUDE.md
Skills). **For the conductor:** one KNOWN_GAPS row per file naming that pass.
**2. Names (M2).** `raw` means a tree or bytes, nothing else. Rule: a binding or field of type `NotAsTheyWere` (or
`Option<_>` of it) → `not_as_they_were`; any other failure-typed `raw` (e.g. `apply.rs:409`'s `Option<AfterState>`)
→ `raw_failed`. Sites from the review: `PutBackRan::Whole { raw }` (`put_back.rs:78,84`), `Returned::Back`,
`Kept::Git`, `Kept::Unlisted { raw }` (`returned.rs:25,43,54`), `answer.rs:79-144`, `apply.rs:409-419`. Kept as `raw`:
`HisBytes.raw: Oid`, `Step::RestoreWorktree { raw }`, `worktree_raw_before`. The comment at `apply.rs:407-408` that
explains the name goes (the name does it now).
**3. Docs (M3), two sentences each, pointing at ADR-0052.**
- `ARCHITECTURE.md:41`: the snapshot writes `index/` or `unmerged/`, `worktree/`, and, when bytes differ from what git
  gives back, `raw/`; name `snapshot/raw.rs` (+ `raw/converted.rs`) and `restore/raw.rs` beside `restore/outcome.rs`.
- Design §6.3: the backup also keeps `raw/` — the bytes the index/checkout path would not give back — and a put-back
  writes them byte-exact (ADR-0052); replace "tracked content via the ordinary index/checkout path" accordingly.

## Signatures
No new API. Field renames only (types unchanged): `PutBackRan::Whole { not_as_they_were, .. }`,
`Returned::Back { not_as_they_were, .. }`, `Kept::Git { not_as_they_were, .. }`, `Kept::Unlisted { not_as_they_were, .. }`
— exact shapes as the scout reads them.

## Acceptance (each box names its test at tick time)
- [ ] `wc -l` of the five files and every new file ≤ 400, pasted in NOTES (the check: the numbers, not a test).
- [ ] Pure move: `git diff --stat` + `git diff -M` show moves; no test changed except its `use` lines. Named check:
      the full graft-ops suite green **unchanged in count** (before/after nextest totals in NOTES).
- [ ] Renames: `grep -nw raw crates/graft-ops/src` in NOTES, every hit a tree or bytes.
- [ ] Byte-exact promise unchanged: `restore_raw`/put-back tests by name green (the scout lists them, e.g. the 30p
      walk `journal_put_back.rs`), and the M7 acceptance's raw-byte scenarios green.
- [ ] Docs: the two edits, quoted in NOTES; `ARCHITECTURE.md` names only files that exist (`ls` each in NOTES).
- [ ] graft-ops MAP.md: `snapshot` file list (`raw/converted.rs`), `apply::error` (`error/words.rs`), set-aside tests.
- [ ] Gates.

## Touches (check each; report how)
- **Other crates matching the renamed fields:** grep graft for `Whole {`, `Returned::`, `Kept::`; an exhaustive
  destructure needs the new name (in the crate set only if the scout named it; else `blocked: needs graft`).
- **Re-exports:** `snapshot::*`, `apply::error::*` paths callers use: unchanged. Check: workspace builds untouched
  outside graft-ops.
- **Doc readers:** `ARCHITECTURE.md` points ops readers at §6.3 first — both now agree with ADR-0052.
- **Captures, keymap, UI, journal format:** none (no behaviour change).
- **For the conductor:** closes review m7-30p-30w M1–M3; three KNOWN_GAPS rows (operation.rs, apply.rs, undo.rs).

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~40k. One context.
