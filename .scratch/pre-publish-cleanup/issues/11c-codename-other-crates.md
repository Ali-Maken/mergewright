# pre-publish: the codename out of every crate but the app (ui, ops, git, core, fixtures, graph, term)

Status: draft
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
- [ ] Arrival recount by class and crate ← `git grep -c -i -E 'graft|tributary|trib_' -- <crates>`, in NOTES
- [ ] Only D2 classes left ← `git grep -n -i -E 'graft|tributary|trib_' -- <crates>` output, each line tagged identity or git-word, in NOTES
- [ ] Snapshot diffs header-only ← `git diff -- '<crates>/**/*.snap'` summary
- [ ] Renamed captures keep their writers ← `scripts/shot-drift.py`; `git diff -M --stat docs/shots`
- [ ] No `GRAFT_` left in these crates ← grep empty
- [ ] No `over_400.txt` row raised ← `file_size` green
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY

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
