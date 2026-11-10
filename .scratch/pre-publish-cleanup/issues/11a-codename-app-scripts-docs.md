# pre-publish: the codename out of the app crate, the shipped scripts and the shipped docs, behind a guard written first

Status: done (fast/walk box for the conductor)
Blocked by: 06, 11b4 (old-generation code is gone, so nothing needs allow-listing for it). 07–10 are no longer blockers (cut, pending the user).
Runs concurrently with: 11c (disjoint files: this ticket takes no file under any crate but `mergewright` and `mergewright-capabilities`)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: renames behind a guard written first; every choice is made below.
Needs: none (11b's consts already exist)
Scout: done. Use `scouts/11-codename-hits.md`. Its counts are pre-11b, so recount on arrival (Acceptance 1).
Replaces: `11-codename-leftovers.md` (app + scripts + docs half; 11c takes the other crates).

## Closes (with 11c and 12)
`rename/graft-leftovers` (this file set) and `rename/m3-pty-script` (D7).

## Decisions
**D1. Shipped set.**
- `docs/release/export-inventory.md` §8 globs, plus the conductor ruling: `DESIGN.md` and `docs/design/**` ship. One `const` lists them, with a comment citing §8.
- **TESTING.md is outside the guard** until the licence step splits it; a comment says so. This ticket still fixes its checklist hits (D5).
- Private files keep their history mentions.

**D2. The guard comes first, and runs red.**
- New `crates/mergewright/tests/it/codename_absent.rs`.
  - It scans `git ls-files` within `SHIPPED`, minus `PENDING`, for the `WORDS`, case-insensitive, **in content and in paths**.
  - It fails listing `path:line`.
- `WORDS = ["graft", "tributary", "trib_"]`: every earlier name (ADR-0057 Amendment §2; the same list `rename_docs.rs` uses today).
- `PENDING` lists globs another ticket clears, each with its ticket:
  - every `crates/*` except `mergewright` and `mergewright-capabilities` → `11c`;
  - `docs/shots/**` (content and paths) → `12`.
- `PENDING_IDENTITY` lists substrings skipped everywhere until 12: `Graft Fixture`, `Graft Test`, `Graft Perf`, `@graft.invalid`.
- Exceptions live only in `crates/mergewright/tests/it/codename_allowed.txt`.
  - One occurrence per line: `<path>\t<exact substring>\t<reason>`.
  - Reasons: `git-word` (Git's own `grafted`, `--graft`, `info/grafts`; add every one in the repo now, including those under PENDING globs, so 12 needn't) and `guard` (the word lists in this guard and in `rename_names.rs`'s `OLD_NAMES`).
  - A row whose substring no longer occurs fails, so stale rows can't pile up.
- Run it red on arrival and quote the hit count.

**D3. `rename_docs.rs::old_name_lines` is deleted, not kept beside the guard.**
- The new guard covers README, CHANGELOG and every shipped script. The private scripts it also scanned don't ship.
- Other tests in `rename_docs.rs` stay.

**D4. Replacements in code.**
- Product mentions → `Mergewright` / `mwr`.
- Temp-dir/file prefixes `graft-` → `mwr-`, `.graft-tmp` → `.mwr-tmp`. Grep every reader of the same name.
- Test and fn names → `mergewright`. Walk fns keep their `_walk` suffix and module path.
- If the app's lib target is still named `graft` (MAP line 1: "lib `graft`"), drop the `[lib] name` so it takes the package name, then change every `graft::` path in tests and examples. The root `Cargo.toml` hit gets the same check.
- MAP.md headings and lines in the app crate.
- Env vars `GRAFT_<X>` → `MWR_<X>`, with every setter and reader in this file set in the same commit.
  - If an `MWR_<X>` twin already exists, the old name is deleted: an alias is old-generation code.
  - 11c applies the same mechanical rule, so pairs that cross the boundary agree. List each cross-boundary pair in NOTES.
- A production literal writing an earlier name into git or onto disk → the `on_disk` const. List each with file:line. 11b's guard should have left none.
- `.snap` files in this crate: regenerate with `INSTA_UPDATE=always INSTA_FORCE_UPDATE=1` over **this crate's tests only** (a regeneration scope, not a gate; insta 1.48 rewrites headers, scout). The diff must be `source:` lines only. Otherwise, edit the header line by hand.

**D5. Replacements in docs.**
- README, ARCHITECTURE, CONTEXT, CAPABILITIES, DESIGN.md + `docs/design/`, `docs/man/` (regenerate with `--generate-man` if generated), `docs/capabilities/`, `fuzz/{Cargo.toml,README.md,fuzz_targets}`, `packaging/`, `.github/workflows/`, `clippy.toml`: swap the word, keep the wording.
- Prose describing the old-name move or the generation table goes; that code is gone (11b).
- CHANGELOG: entries only about the rename or the move go; elsewhere `Graft` → `Mergewright`.
- TESTING.md: checklist hits fixed; the "Times I left" section untouched (private history).

**D6. Captures named `*graft-self*`** whose writer is in this crate → `*mwr-self*`, renamed together with the writer.
- Regenerate a capture only if its test fails or `shot-drift.py` flags it. The content diff must be the renamed word only.
- Body hits otherwise stay for 12. Captures whose writer is in another crate are 11c's.

**D7. The m3 pty script** (`.scratch/…/pty-walk-m3.py:32,43,57`) is private.
- If a shipped file or the TESTING.md checklist tells someone to run it, fix the three lines.
- Otherwise leave it and close the row "private record, not shipped, not run", with the grep in NOTES.

## Stop conditions
- A rename changes behaviour beyond D4's named classes.
- A line grows and pushes a file past its `over_400.txt` ceiling with no coherent seam. The split rules are in `07-ratchet-app-src.md` § Rules. Never raise a row.
- A capture's content changes beyond the renamed word.
- A `docs/capabilities` rename breaks a `mergewright-capabilities` parse test in a way a word swap can't fix.

## Scope
In: `crates/mergewright/**`, `crates/mergewright-capabilities/**`, the shipped `scripts/` list, the top-level and docs files in D5, the guard and its allow-list, this crate's captures (D6), the m3 script (D7).
Out: other crates (11c); fixture identities and `docs/shots` bodies (12); private files; licence and export files.

## Signatures
```rust
// crates/mergewright/tests/it/codename_absent.rs (test module; no library API)
const SHIPPED: &[&str] = &[/* export-inventory §8 + DESIGN.md, docs/design/** */];
const WORDS: &[&str] = &["graft", "tributary", "trib_"];
const PENDING: &[(&str, &str)] = &[/* (glob, ticket that clears it) */];
const PENDING_IDENTITY: &[&str] = &["Graft Fixture", "Graft Test", "Graft Perf", "@graft.invalid"];
#[test] fn no_shipped_file_names_the_codename();
#[test] fn every_allowed_codename_occurrence_still_exists();
```

## Acceptance (tick each with its test, inline)
- [x] Arrival recount (post-11b): hits by class for this file set ← `git grep -c -i -E 'graft|tributary|trib_'` over the file set, in NOTES
- [x] Guard red on arrival, count quoted ← `codename_absent::no_shipped_file_names_the_codename` (red run in NOTES)
- [x] Guard green; allow-list counted by reason ← same + `every_allowed_codename_occurrence_still_exists`
- [x] Renamed captures keep their writers, word-only diff ← `scripts/shot-drift.py`; `git diff -M --stat docs/shots`
- [x] Snapshot diffs header-only ← `git diff -- 'crates/mergewright/**/*.snap'` summary
- [x] Env pairs complete ← `git grep -n 'GRAFT_' -- crates/mergewright scripts` empty; cross-boundary list in NOTES
- [x] m3 script per D7 ← grep in NOTES
- [x] No `over_400.txt` row raised ← `file_size` green; diff of `over_400.txt`
- [x] Fast and walk tiers green ← conductor, main @ aedf0607: `cargo nextest run` 6789/6789 (367 s, quiet), `scripts/walks.sh` SUMMARY walks tests=459 failed=0; fmt, clippy, heavy clippy, doc tests clean

## Touches (check each, say how)
- `rename_names.rs` (`OLD_NAMES` → `guard` rows), `rename_docs.rs` (D3), `rename_names/scripts.rs`, `release_names.rs`.
- Capture writers and the drift script; `published_repo`; the contact-sheet script.
- `docs/capabilities` and the capabilities parser.
- Man page generation.
- The walk partition (`.config/nextest.toml` filter by name).
- `over_400.txt`.
- Keymap, journal, undo: none.

## Read first
This ticket. `scouts/11-codename-hits.md`. `docs/release/export-inventory.md` §8 and "Conductor rulings". `crates/mergewright/MAP.md` § tests/it lines. `07-ratchet-app-src.md` § Rules (split rules only).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~65k tokens at the scout's counts: app ~200 non-old-gen lines (pre-11b), scripts 54, docs ~90 outside `docs/shots`. Mostly comments and words. If the arrival recount is over ~300 non-comment edits, commit at the docs/code seam and report.

## NOTES (builder, 2026-11-10)
Scout: used `scouts/11-codename-hits.md` (no new scout; recount below replaces its counts).

- **Arrival recount** (`git grep -c -i -E 'graft|tributary|trib_'`): app crate 241 lines; capabilities crate 0; shipped scripts 15 (8 files); docs/top-level 117 (incl. TESTING.md 12, docs/capabilities 51, CHANGELOG 13). Classes: comments/prose most; temp prefixes ~25; test literals ~30; old-gen leftovers (m8-accept S1–S3, clone-git-git tributary move, README/CHANGELOG move prose).
- **Guard red on arrival**: `codename_absent::no_shipped_file_names_the_codename` → `324 shipped lines name the codename`.
- **Guard green**: `codename_absent::{no_shipped_file_names_the_codename, every_allowed_codename_occurrence_still_exists, the_word_test_blanks_allowed_text}` pass. Allow-list 71 rows: guard 66, git-word 0 (none exist in the repo: `git grep -i -E 'grafted|--graft|info/grafts'` hits only the guard tests), **11c 5**.
- **Deviation: reason `11c`** added to the allow-list (spec allowed only git-word/guard). Five occurrences name things another crate owns, so renaming them here would break the build/checker before 11c lands: `fork_rows.rs` `graft-fork-1` (must match ops `FORK_LABEL`); capabilities m3 `RefName::is_graft_internal`, `grafts_own_refs_are_not_drawn_but_are_still_readable`; m5 `a_shell_merge_is_not_adopted_as_grafts_own_entry`, `sha1("graft-ghost…")`. After 11c merges, rename these and drop the rows (the stale-row test forces it).
- **Captures**: `35-graft-self-{ascii,unicode}` → `35-mwr-self-*` (gallery writer, pure rename, `git diff -M --stat` 0/0). Regenerated, word-only diffs: m5-ui-preview-time-machine ×2, m6/fix-status-under-a-box ×2, m8/21c-*-partial-fetch ×4, m8-tour-187x50, m8-tour-finished-120x36, ease-pass/main-pane-own-120 + contact sheet. Diff lines are the renamed word plus the truncation/padding it shifts on the same line. `shot-drift.py --base main` lists the two renamed 35- files as drift (it compares by path); content unchanged. `43-graft-self-varied`, `m2-17-graft-self-*` left: writers are private pty scripts → 12.
- **Snapshots**: no `.snap` in this crate named a word; 0 changed.
- **Env**: `git grep -n 'GRAFT_' -- crates/mergewright scripts` empty. Cross-boundary: none in this file set (`MWR_SCRATCH_ROOT` comment in fixtures/scratch.rs still says `/tmp/graft-capture` → 11c; mwr_sandbox.py now uses `/tmp/mwr-capture`).
- **D3**: `rename_docs.rs` deleted whole (its only other test exercised the deleted finder); `user_docs_and_scripts_name_the_command` removed; m8-accept.sh S8 now cites `codename_absent`.
- **m8-accept.sh**: S1–S3 removed (their tests were deleted in 11b2), and `rename_opens::no_old_dir_literal` dropped from S8 (that test no longer exists); prints `5 scenarios`. TESTING.md M8 checklist matched.
- **D7**: TESTING.md:1424 tells someone to run `pty-walk-m3.py`, so lines 32, 43, 57 (+58–59, the config dir it writes) fixed to `mergewright-fixtures`/`mwr-walk-m3-`/`mergewright`. `py_compile` ok; **not run** (unverified).
- **Bug found and fixed (cause)**: `quit_waits/pty_session.rs::assert_no_crash` read `~/.local/state/graft`, so it never saw a crash report; now `on_disk::DIR`. Tests still green.
- **over_400**: `file_size` green; `over_400.txt` diff empty (two growths reverted by shortening lines).
- fuzz/Cargo.lock: names only edited by hand (a resolve needs network and pulls new packages; the lock was already stale).
- Gates: fmt, clippy -D warnings, heavy clippy, `cargo test --doc` green; `cargo nextest run --no-fail-fast` 6789/6789; `scripts/walks.sh` SUMMARY tests=459 failed=0.
