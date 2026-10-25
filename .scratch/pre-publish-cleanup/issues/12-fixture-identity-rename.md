# pre-publish: neutral fixture authors. Every fixture oid changes, and a script proves nothing else did

Status: ready
Blocked by: 11 (its guard lists the `fixture-identity` entries this ticket removes; and transitively 01–10, so it re-shoots captures once, last)
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: one constant set changes, then everything regenerates. A normalisation script, not the eye, judges the thousands of changed bytes, so correctness doesn't rest on reading the diff. **Runs alone**: it rewrites nearly every capture and snapshot.
Scout: **yes**, first. It writes `.scratch/pre-publish-cleanup/scouts/12-fixture-identity.md` (≤1 page):
- the exact identity strings and emails (`Graft Fixture`, `Graft Test`, `Graft Perf`, …) and where each is set (fixtures generator, test helpers, `user.email` config for "you");
- the count of hard-coded oids and sha7s in test source, by file;
- what the fixture cache key covers (git version per M1 row; does it cover generator constants?);
- any code that orders or hashes by oid where equal dates tie (topo walk tie-breaks; anything keyed `oid % n`);
- whether captures draw author initials.

## Closes
- `rename/graft-leftovers` (**broken**), the fixture-identity part: "fixture identities `Graft Fixture/Test/Perf` (seed fixture oids; renaming needs hard-coded oids + snapshots redone)". With 11, the row closes.

## Decisions
**D1. Neutral names of the same length, no product name.** A later rename (the trademark search is still open) then can't touch them.
- `Graft Fixture` → `Fixture Maker`; `Graft Test` → `Test Maker`; `Graft Perf` → `Perf Maker`. Each pair is equal in length.
- Email local parts: `graft` → `maker` (same length), domains unchanged.
- Any other `Graft …` identity the scout finds follows the same pattern, at the same length.
- Why the same length: every column, truncation and `…` cut stays where it was, so captures differ only in letters and hex. D3 depends on that.

**D2. Change once, regenerate everything.**
- Constants and every `user.email` / mailmap that names them change in lockstep ("you" is found through `user.email` and mailmap, 07c). Otherwise the `you` label vanishes.
- The fixture cache key must change. Prove the key covers the identity, or bump it. A stale cache passing on this machine and failing in a clean checkout is the failure to prevent.
- Hard-coded oids and sha7s in tests are updated.
- insta snapshots accepted; captures regenerated across both tiers (`MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter`); PNG goldens and the pinned-row golden regenerated.

**D3. Proof by normalisation.** New `scripts/pp-12-identity-diff.py` (run with `python3 -I`):
- For every file changed under `docs/shots/`, every `*.snap` and every `*.golden` text file, compare the old version (start hash) with the new one after masking: hex runs of 7–40 characters (oids, sha7), the old and new names, emails and initials.
- It writes `docs/perf/pp-12-identity-diff.txt`: files compared, files equal after masking, and the list of files **not** equal, each with its first differing line.
- It prints `SUMMARY compared=N equal=M differ=K`.
- PNGs: compare decoded dimensions only, and list them for his eye. A PNG shows no author text or sha, so a changed PNG means something moved.

## Stop conditions (report, don't work around)
- `differ > 0`. Rows reordered (an oid tie-break), a hash-derived slot or colour changed, or a width changed: stop with the list and the cause per file. The conductor decides; don't accept them in bulk.
- Any PNG's dimensions or bytes change: list it, and stop before accepting it.
- Heavy tier: run it once from a script into `docs/perf/pp-12-heavy.txt`. Its red set must equal the last recorded heavy red set (ticket 16's `docs/perf/16-heavy.txt`, by name). A new red is a stop.
- The cache key can't be shown to cover the identity, and bumping it isn't possible without touching its design: report it.

## Scope
In: `mergewright-fixtures` identity constants and cache key; test sources holding hard-coded oids, names or emails (any crate); regenerated snapshots, captures and goldens; the script and its perf file; 11's allow-list (remove the `fixture-identity` rows).
Out: product code; any other string; capture layout.

## Signatures
```rust
// crates/mergewright-fixtures (names per the scout; constants, not new API)
pub const FIXTURE_AUTHOR: &str = "Fixture Maker";   // was "Graft Fixture"
pub const TEST_AUTHOR: &str = "Test Maker";         // was "Graft Test"
pub const PERF_AUTHOR: &str = "Perf Maker";         // was "Graft Perf"
```
If the identities are inline literals today, they become these constants, with one MAP line in `crates/mergewright-fixtures/MAP.md`.

## Acceptance (tick each with its test or script, inline)
- [ ] 11's guard green with no `fixture-identity` rows left ← `codename_absent::no_shipped_file_names_the_codename`, `every_allowed_codename_occurrence_still_exists`
- [ ] Normalised diff empty: `differ=0` ← `scripts/pp-12-identity-diff.py`, SUMMARY quoted
- [ ] PNGs: none changed, or each listed with dimensions for his eye ← same script's PNG section
- [ ] Cache can't go stale: delete the fixture cache dir, then a full fast run is green ← `cargo nextest run` after `rm -rf <fixture cache>`, count quoted
- [ ] Fast and walk tiers green ← `cargo nextest run` count, `scripts/walks.sh` SUMMARY
- [ ] Heavy red set unchanged ← `docs/perf/pp-12-heavy.txt` vs `docs/perf/16-heavy.txt`, by name
- [ ] Fixture determinism holds (built twice, same oids) ← the fixtures determinism test by name
- [ ] No `over_400.txt` row raised ← `file_size` green

## Touches (check each, say how)
- Every capture and snapshot (the script).
- The `you` label (team fixture captures: the script catches a vanished label).
- Mailmap tests (`mergewright-git` mailmap; "Rui M" there is the licence step's neutral-name ruling, not this one).
- Fixture cache key, and the corpus determinism test.
- Heavy fixtures (100k: regeneration time, not context).
- `published_repo`.
- The contact sheet for the ease pass, if it embeds captures: regenerate only if a test asserts it; otherwise list it (user rule: old sheets are not rebuilt).
- Keymap, journal, undo: none.

## Read first
- This ticket, then the scout file.
- `crates/mergewright-fixtures/MAP.md` (generator, cache, `Captures`).
- TESTING.md: the Captures paragraph and its regeneration command.
- 11's allow-list file.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`, and the heavy run from its script.

## Size
~50k tokens: a few constants, the oid edits the scout counts, one script. Regeneration is machine time. If the hard-coded oid count is over ~150 sites, the conductor splits oid edits by crate (12a fixtures + app, 12b the rest) with the script run last.
