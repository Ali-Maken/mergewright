# pre-publish: the codename out of every shipped file, with a guard so it stays out

Status: ready
Blocked by: 06, 07, 08, 09, 10 (the ratchet moves code first; renames lengthen strings, so they go on files already split. 06 deletes stale captures first)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: renames of comments, test names, temp prefixes and capture names, behind a guard test written first. Anything that changes stored or on-disk behaviour is a stop, not a build decision.
Scout: **yes**, first. It writes `.scratch/pre-publish-cleanup/scouts/11-codename-hits.md` (≤1 page):
- `git grep -n -i graft` over the shipped set (D1), counted by class × crate. Classes: comment; test/fn name; temp-dir prefix; capture file name; `.snap` `source:` header; doc text; env var; Git's own word (`grafted`, `--graft`, `info/grafts`); old-generation code (finds or moves old-name folders, ADR-0057); string written into git or onto disk; fixture identity.
- existing guards (`tests/it/rename_names.rs` `no_old_message_literal`, `rename_docs.rs`): what they scan;
- the installed insta version (does it rewrite headers on `--force-update-snapshots`?).
If more than ~400 hits need edits outside comments, the conductor splits by crate (11a app, 11b the rest).

## Closes
- `rename/graft-leftovers` (**broken**), **except the fixture identities (12)**: "871 `graft` hits remain: migration names (kept on purpose), fixture identities `Graft Fixture/Test/Perf` (seed fixture oids; renaming needs hard-coded oids + snapshots redone), lowercase comments/temp prefixes, .snap `source:` headers. Public repo must not carry the codename".
- `rename/m3-pty-script` (**broken**, if run without `MWR_FIXTURES_BIN`): "`.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py:32,43,57` still build/run `graft-fixtures`, `graft-walk-m3-`".

## Decisions
**D1. What ships decides the scope.**
- Shipped set: `docs/release/export-inventory.md` §8 globs plus its conductor rulings. `DESIGN.md` and `docs/design/` ship; TESTING.md ships (it gets split later); ADRs are private.
- Private files (`docs/log`, `docs/reviews`, `docs/adr`, `.scratch`, KNOWN_GAPS, PROGRESS, `prototypes`) keep their history mentions. Exception: D5.

**D2. Guard first, red.**
- New `crates/mergewright/tests/it/codename_absent.rs`, or an extension of the existing guard if the scout finds one already scanning shipped files (no duplicate). It scans `git ls-files` restricted to the shipped globs (one `const`, with a comment citing export inventory §8) for case-insensitive `graft` **in content and in paths**. It fails listing `path:line`.
- Exceptions only in `crates/mergewright/tests/it/codename_allowed.txt`: one occurrence per line, `<path>\t<exact substring>\t<reason>`. Reason is one of:
  - `git-word`: Git's own vocabulary;
  - `old-generation`: code that finds or moves old-name folders, ADR-0057 (pending user question U1 in 00-index);
  - `fixture-identity`: removed by 12.
- An allowed substring that no longer occurs also fails the guard, so stale entries can't pile up.
- Run it red on arrival and quote the hit count.

**D3. Replacements.**
- Product mentions → `Mergewright` / `mwr` (ADR-0057).
- Temp-dir prefixes `graft-` → `mwr-`.
- Test and fn names containing `graft` → `mergewright`.
- Capture files named `*graft-self*` (`35-graft-self-*`, `m2-17-graft-self-*`, `43-graft-self-varied.txt`, …) → `*mwr-self*`, with their writers renamed in the same commit. `scripts/shot-drift.py` confirms each has its writer.
- `.snap` `source:` headers: regenerate with insta if the installed version rewrites headers. Otherwise edit the header line only; the content stays byte-identical.
- `CHANGELOG.md` (13 hits): `Graft` → `Mergewright`, other wording unchanged.
- `docs/capabilities/`: the same word swap.

**D4. Not a comment rename, so a stop.** Strings written into the user's repo or read back from disk (stash message `graft: your edits, set aside for …`, backup commits `graft {id} …`: row `M8/27-git-messages-graft`, its own expiry), on-disk folder names, and env vars a user might set. List them in NOTES and leave them. A test-only env var may be renamed only if every setter and reader in the repo changes in the same commit (grep shown).

**D5. The m3 pty script** is private (`.scratch`), and the user's rule (2026-10-19) is that past-milestone material isn't updated.
- If any shipped file or TESTING.md tells someone to run it: fix lines 32, 43 and 57 to the `mergewright-fixtures` / `mwr-walk-m3-` names.
- Otherwise: leave it, and close the row as "private record, not shipped, not run" with the grep in NOTES.

## Stop conditions (report, don't work around)
- A hit changes behaviour when renamed (D4 classes, or anything the guard can't tell from a comment).
- Renaming lengthens a line and pushes a file past its `over_400.txt` ceiling, and no coherent seam exists. Split per `07-ratchet-app-src.md` § Rules; if there's no seam, stop.
- A capture's content changes beyond the renamed word.

## Scope
In: shipped files with `graft` hits (D1) except fixture identities and D4 strings; the guard and its allow-list; capture renames with their writers; snapshot headers; CHANGELOG; `docs/capabilities`; the m3 script per D5.
Out: fixture identities (12); D4 strings; private history; licence and export files (next step).

## Signatures
```rust
// crates/mergewright/tests/it/codename_absent.rs (test module; no library API)
const SHIPPED: &[&str] = &[/* globs from docs/release/export-inventory.md §8 */];
#[test] fn no_shipped_file_names_the_codename();          // content + paths; allow-list honoured
#[test] fn every_allowed_codename_occurrence_still_exists(); // no stale allow-list rows
```

## Acceptance (tick each with its test, inline)
- [ ] Guard red on arrival, hit count quoted ← `codename_absent::no_shipped_file_names_the_codename` (red run in NOTES)
- [ ] Guard green after; allow-list counted by reason (`git-word` n, `old-generation` n, `fixture-identity` n) ← same test + `every_allowed_codename_occurrence_still_exists`
- [ ] Renamed captures still have their writers, with no content diff except the word ← `scripts/shot-drift.py` output; `git diff -M --stat docs/shots` (renames)
- [ ] Snapshot changes are header-only ← `git diff -- '*.snap'` shows only `source:` lines (summary in NOTES)
- [ ] D4 strings listed, none renamed ← NOTES list with file:line
- [ ] m3 script handled per D5 ← grep quoted in NOTES
- [ ] No `over_400.txt` row raised ← `file_size` green; `git diff <start> -- crates/mergewright/tests/it/over_400.txt`
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

## Touches (check each, say how)
- `rename_names.rs` / `rename_docs.rs` (existing guards; may already pin some words).
- Capture writers and the drift script; insta snapshots; `published_repo`.
- `scripts/` shipped list (names inside scripts).
- The man page (`docs/man/mwr.1`), README, CHANGELOG, `docs/capabilities`, and the capabilities parser that reads `docs/capabilities` (`mergewright-capabilities` `parse.rs`). A renamed heading may break a parse: check its tests.
- Walk partition (renamed test fns keep their module path).
- `over_400.txt`.
- Keymap, journal, undo: none (D4 is a stop).

## Read first
- This ticket, then the scout file.
- `docs/release/export-inventory.md` §5, §8 and "Conductor rulings".
- ADR-0057 (the name and the stored-name migration).
- `crates/mergewright/MAP.md` § tests/it; `crates/mergewright-fixtures/MAP.md` (capture names).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh` (capture names change).

## Size
~60k tokens if the scout's counts hold (most hits are comments). Over ~400 non-comment edits → the conductor splits by crate before dispatch.
