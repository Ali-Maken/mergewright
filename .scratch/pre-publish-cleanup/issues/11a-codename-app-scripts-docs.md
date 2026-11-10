# pre-publish: the codename out of the app crate, the shipped scripts and the shipped docs, behind a guard written first

Status: draft
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
- [ ] Arrival recount (post-11b): hits by class for this file set ← `git grep -c -i -E 'graft|tributary|trib_'` over the file set, in NOTES
- [ ] Guard red on arrival, count quoted ← `codename_absent::no_shipped_file_names_the_codename` (red run in NOTES)
- [ ] Guard green; allow-list counted by reason ← same + `every_allowed_codename_occurrence_still_exists`
- [ ] Renamed captures keep their writers, word-only diff ← `scripts/shot-drift.py`; `git diff -M --stat docs/shots`
- [ ] Snapshot diffs header-only ← `git diff -- 'crates/mergewright/**/*.snap'` summary
- [ ] Env pairs complete ← `git grep -n 'GRAFT_' -- crates/mergewright scripts` empty; cross-boundary list in NOTES
- [ ] m3 script per D7 ← grep in NOTES
- [ ] No `over_400.txt` row raised ← `file_size` green; diff of `over_400.txt`
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

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
