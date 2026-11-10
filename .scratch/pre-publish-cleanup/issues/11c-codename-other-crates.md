# pre-publish: the codename out of every crate but the app (ui, ops, git, core, fixtures, graph, term)

Status: done (fast/walk box for the conductor)
Blocked by: 06, 11b4. 07–10 are no longer blockers (cut, pending the user).
Runs concurrently with: 11a. Disjoint files: this ticket owns every `crates/*` except `mergewright` and `mergewright-capabilities`, plus the captures whose writers live in those crates.
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: comment, name and header renames under 11a's rules. Nothing new is decided here.
Needs: none at the code level. The rules are 11a's D4 and D6; read them.
Scout: done. Use `scouts/11-codename-hits.md` (counts are pre-11b; recount on arrival).
Replaces: `11-codename-leftovers.md` (the crate half).

## Note to the conductor
The dispatch listed ui, git, core, fixtures, graph and term. **ops is added here.** About 100 of its 276 hits are not old-generation (e.g. `rewrite_oracle.rs` 18), and no other ticket owned them.

## Decisions
**D1. Same replacements as 11a D4/D6**, applied to this file set:
- product words, temp prefixes (`graft-` → `mwr-`, `.graft-tmp` → `.mwr-tmp`, every reader of the same name), test and fn names, MAP.md headings and lines;
- env vars `GRAFT_<X>` → `MWR_<X>`, and an old alias is deleted where an `MWR_` twin exists;
- `.snap` headers: `INSTA_UPDATE=always INSTA_FORCE_UPDATE=1` over **these crates' tests only**, header-only diff;
- captures `*graft-self*` with their writers here → `*mwr-self*`.
- The same mechanical env rule as 11a, so cross-boundary pairs agree after both merge. List the pairs in NOTES.

**D2. Not touched here:**
- fixture and test identities (`Graft Fixture`, `Graft Test`, `Graft Perf`, `test@graft.invalid`): 12;
- Git's own words (`grafted`, `--graft`, `info/grafts`), which 11a allow-lists;
- `docs/shots` bodies: 12.

**D3. 11a's guard is not run green here.** It lists these crates as `PENDING: 11c` and lives in 11a's file. This ticket proves itself by grep (Acceptance), and 12 removes the `PENDING` entry.

## Stop conditions
As 11a:
- a rename changes behaviour;
- an `over_400.txt` ceiling would be passed with no seam (`07-ratchet-app-src.md` § Rules; never raise a row);
- a capture or snapshot diff goes beyond the word or the header.

Plus: on arrival, if the recount shows **over ~300 non-comment, non-identity edits**, commit `mergewright-ui` alone, report, and stop. The conductor splits off the rest.

## Scope
In: `crates/{mergewright-ui, mergewright-ops, mergewright-git, mergewright-core, mergewright-fixtures, mergewright-graph, mergewright-term}/**` (and any other crate except the app and capabilities), their `.snap` headers, their captures' names.
Out: the app and capabilities crates, scripts, docs (11a); identities and capture bodies (12).

## Signatures
None (renames only).

## Acceptance (tick each with its test, inline)
- [x] Arrival recount by class and crate ← `git grep -c -i -E 'graft|tributary|trib_' -- <crates>`, in NOTES
- [x] Only D2 classes left ← `git grep -n -i -E 'graft|tributary|trib_' -- <crates>` output, each line tagged identity or git-word, in NOTES
- [x] Snapshot diffs header-only ← `git diff -- '<crates>/**/*.snap'` summary — 71 files, 71 +/71 −, every changed line `source:` (`git diff -- '*.snap' | grep '^[-+]' | grep -v source:` empty); edited by hand per 11a D4, paths checked to exist
- [x] Renamed captures keep their writers ← `scripts/shot-drift.py`; `git diff -M --stat docs/shots` — empty: all `*graft-self*` writers are in the app crate (gallery.rs), 11a's; no capture renamed or regenerated here; `capture_a_partial_fetch` green
- [x] No `GRAFT_` left in these crates ← `git grep 'GRAFT_' -- <crates>` empty (already none on arrival; no cross-boundary env pairs)
- [x] No `over_400.txt` row raised ← `file_size::every_file_over_400_lines_is_listed_with_its_count` green in the full run
- [x] Fast and walk tiers green ← conductor, main @ 98846487: `cargo nextest run` 6788/6788 (537 s, beside builder 11a), `scripts/walks.sh` SUMMARY walks tests=459 failed=0; fmt, clippy, heavy clippy, doc tests clean

## Touches (check each, say how)
- insta snapshots (mostly ui); capture writers and the drift script.
- Temp-file names read elsewhere (e.g. `write_back`'s atomic rename in git): every reader, grep quoted.
- Env pairs across 11a/11c; the walk partition; `over_400.txt`.
- Keymap, journal, undo: none.

## Read first
This ticket. 11a's D4 and D6. `scouts/11-codename-hits.md`. Each touched crate's MAP.md top lines only. `07-ratchet-app-src.md` § Rules.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`.

## Size
~70k tokens: about 600 lines pre-11b (ui 396 with most of the 71 `.snap` headers, ops ~100, git ~65, fixtures ~40 minus identities, graph 25, core ~10, term 2). Mostly comments. The arrival split rule above bounds it.

## NOTES (11c build)
- Recount on arrival (`git grep -c -i -E 'graft|tributary|trib_'`): ui 396, ops 104, git 77, fixtures 39, graph 25, core 16, term 2 = 659. Of these 71 `.snap` headers, ~250 snap body identity, 39 other identity, 151 code lines; under the split rule.
- After: remaining lines, by tag (`git grep -n -I -i -E 'graft|tributary|trib_' -- <crates>`):
  - identity (12): ui snapshot bodies (author `Graft` column), 39 `Graft Fixture/Test/Perf`/`graft.invalid` lines, testdata blobs, `fixtures/src/shapes/mod.rs:34` REMOTE_URL `graft.git` (part of a fixture commit message: renaming changed oids + 2 snapshots, reverted), the graph/ui `accidental-pull-merge` snaps holding it.
  - old-name guard inputs (kept on purpose): `core/src/on_disk/tests.rs`, `core/src/ref_name/tests.rs`, `ui/.../source_column.rs:25,34` (asserts old name never drawn), `ops/tests/it/conflict_set.rs:353,360` (old message shape), `ops/src/journal_list/tests/words.rs:148` (foreign reflog subject), `ops/tests/it/goldens/m5-journal.jsonl` (stored-format golden).
  - behaviour, left (stop rule): `graft-ghost` hash seed (ops preview/ghost.rs:23, effects.rs, MAP:309) changes every ghost id; `graft-fork-` rebase label (ops rewrite/todo.rs:317 + readers/tests) is written into git's rebase todo on disk. Question for conductor.
  - scratch prefix `graft-{prefix}` (fixtures/src/scratch.rs) left: the shorter `mwr-` changed truncation in docs/shots/m8/21c-journal-partial-fetch-120x40.txt beyond the word (stop rule).
  - test fn names cited by docs/capabilities (11a's files): `grafts_own_refs_are_not_drawn_but_are_still_readable` (git repo/tests.rs:603), `a_shell_merge_is_not_adopted_as_grafts_own_entry` (ops history_ops.rs:1440). Rename needs the citation changed in the same commit → 11a/12.
- Renamed: comments/MAP (`graft` crate → `mergewright`), temp prefixes (`.graft-`, `.graft-tmp`, `$1.graft`, askpass dir/thread, `graft-input` thread, diff3/editor/test temps), test strings, `refs/graft/` test data → `refs/mergewright/`. Stale `Graft-Link`/`tributary/lock` comments reworded to current (`Mergewright-Link:`, journal writer's lock).
- Cross-boundary: no env pairs. App's `rename_names.rs` ALLOWED list still names `graft-input`, `graft-askpass`, `graft-marker-`, `.graft-tmp` (11a's file; now stale).
- Gates: fmt, clippy -D warnings, heavy clippy (`--features heavy-tests`), doc tests green. Full `cargo nextest run --no-fail-fast`: 6788/6788 passed. `scripts/walks.sh`: SUMMARY walks tests=459 failed=0.
