# pre-publish: neutral fixture authors and remote URL, the synthetic cache key made whole, and the guard closed over the full shipped set

Status: built, partial (stops below; final tier-green box left to the conductor) (was draft: rewritten 2026-11-10 to `scouts/12-fixture-identity.md`; amended in pass 4: `graft.git` URL added, 11d a blocker, pending glob relabelled)
Blocked by: 11d (and through it 11a, 11b, 11c, 06). 07–10 are no longer blockers (cut, pending the user).
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: one identity source changes, then everything regenerates. A normalisation script and a lane-slot probe judge the changed bytes; the user judges what they flag. **Runs alone, last**: it rewrites nearly every capture and snapshot.
Needs: 11a's guard (`codename_absent.rs`: `PENDING`, `PENDING_IDENTITY`, allow-list), as 11d left it.
Scout: done: `scouts/12-fixture-identity.md`. Its file:line refs are at main @ 121c167c, and 11b/11c/11d moved lines since, so re-find each by grep before editing. 11c's NOTES (`git show main:.scratch/pre-publish-cleanup/issues/11c-codename-other-crates.md`) list the identity lines it left.

## Closes
- `rename/graft-leftovers` (**broken**), the fixture-identity part. With 11a–11d, the row closes.

## Decisions
**D1. Neutral names, same length, no product name.**

| Was | Becomes | Initials |
|---|---|---|
| `Graft Fixture` | `Maker Fixture` | `GF` → `FM` |
| `Graft Test` | `Maker Test` | `GT` → `TM` |
| `Graft Perf` | `Maker Perf` | `GP` → `PM` |
| `test@graft.invalid` | `test@maker.invalid` | — |
| `graft.git` (fixtures `shapes/mod.rs` `REMOTE_URL`, part of a fixture commit message) | `maker.git` | — |

- Each pair is equal in length, so every column, truncation and `…` stays put.
- The URL change moves that fixture's oids. 11c's NOTES name the snapshots holding it: the graph/ui `accidental-pull-merge` snaps.
- Emails `fixture@example.invalid` and `test@example.invalid` are unchanged. So the "you" email (`ease_walk/fixture.rs` `ME` = `IDENTITY_EMAIL`) and every mailmap stay as they are.
- No other `"Graft <Word>"` identity exists (scout). ui snapshot bodies that show the author column truncated to `Graft` are covered by regeneration.

**D2. One source for the fixture identity.**
- Today it is spelled three times:
  - `mergewright-fixtures/src/git.rs` `IDENTITY_NAME` / `IDENTITY_EMAIL`;
  - `stream.rs` `IDENTITY` (separate literal);
  - `synthetic/reflog.rs` (inline literal).
- `git.rs` gains `IDENTITY` (`"<name> <email>"`), pinned to the other two by a unit test. `stream.rs` and `reflog.rs` use it. Keep today's visibility.
- `Graft Test` (44 lines, 30 files) and `Graft Perf` (3 lines, 2 files) are inline test literals in crates that may not depend on fixtures. Replace them literally; no new shared constant.

**D3. The synthetic cache key covers its sources. Fix it first, in its own commit.**
- `digest.rs`'s SYNTHETIC list names only `synthetic.rs`. It misses `synthetic/reflog.rs`, `synthetic/history.rs`, `scale.rs` and any other file under `src/synthetic/`.
- Add them all. Add `digest::tests::every_synthetic_source_is_keyed`, which lists `src/synthetic/` at test time and fails on any file missing from the list.
- **Question to verify, not a premise:** is `lost_work` cached, and if so under which key? Is `shapes/mod.rs` in the key of every fixture that uses `REMOTE_URL`? Answer both from the code, by grep. Anything cached and outside its key is added, with the same kind of test. If it isn't cached, NOTES says so, with the grep.
- Only then change the identity, so the change itself invalidates the caches.

**D4. Hard-coded oids: find them by failure, not by reading.**
- The scout found 180 hex literals in 68 files. It could **not** tell fixture-derived oids from invented ones (every sample was invented).
- After D1/D2, run the fast tier. Only tests that fail on an oid or sha7 get their literal updated, each to the value the new fixture produces. Invented ids are never touched.
- Count the sites in NOTES. Over ~150 → stop and report; the conductor splits by crate (12a fixtures + app, 12b the rest), with the script run last.

**D5. Regenerate.**
- insta snapshots accepted.
- Captures across both tiers: `MWR_UPDATE_SHOTS=1 cargo nextest run --ignore-default-filter`.
- PNG goldens and the pinned-row golden.

**D6. Proof by normalisation, and the oid tie-breaks named.**
- The scout found three places where an oid decides layout. Changing every oid can legitimately move them:
  - `mergewright-git/src/history/node.rs` topo key, where equal time ties on oid → row order;
  - `mergewright/src/map/walk.rs` branch tie-break on oid → branch order;
  - `mergewright-graph/src/identity.rs` `BranchIdentity::of_short_sha` (used in `fold.rs`, `row.rs`) for unnamed lanes → lane colour, and possibly slot.
- So `differ=0` is **not** expected. The script sorts every change, and the user judges what it flags.
- New `scripts/pp-12-identity-diff.py` (run `python3 -I`). If 11d already added a hex-only checker, extend it rather than duplicate it.
  - Takes every file changed under `docs/shots/`, every `*.snap` and every text `*.golden`, old (start hash = 11d's merge) vs new.
  - In the old version, substitute D1's strings and whole-word initials old→new. Then mask hex runs of 7–40 chars in both.
  - Classes:
    - `equal`;
    - `reordered`: the same multiset of lines in a different order;
    - `other`.
  - Writes `docs/perf/pp-12-identity-diff.txt` (each non-equal file with its first differing line) and prints `SUMMARY compared=N equal=E reordered=R other=O`.
  - PNGs: compare dimensions and bytes, and list every changed one.
- Lane-slot probe:
  - A test-scoped example run once before D1/D2 and once after: `docs/perf/pp-12-lane-slots-before.txt` / `-after.txt`.
  - For each corpus fixture, each row keyed by (subject, nth occurrence) → `BranchIdentity::slot(<palette len>)` of every lane drawn on it.
  - The script diffs the two by key and adds `lane-slot changes=K` to SUMMARY.
  - This is the colour check that captures can't make (rule 5).
- What the human judges: `reordered` files, lane-slot changes and changed PNGs go into one list, `docs/shots/pp-12-identity-review.md`. Each entry has its capture path and its cause: which of the three sites, plus the fixture commits with equal times (for row order) or the unnamed lane (for slot).
- The builder doesn't tick these "right". The conductor puts the list to the user.

**D7. Close the guard.**
- Remove from `codename_absent.rs`:
  - `PENDING_IDENTITY`;
  - every remaining `PENDING` glob (the crates glob 11d relabelled `12`, and `docs/shots/**`), deleting the `PENDING` const if it ends up empty;
  - any allow-list rows that held identity strings.
- The guard is then green over the full shipped set, and `codename_allowed.txt` holds only `guard` and `git-word` rows.
- Any capture body still holding a word after regeneration is a stop. List it: the capture has no live writer, or the word is from another class.

## Stop conditions (report, don't work around)
- `other > 0`. List each file with its cause. Don't accept them in bulk.
- A `reordered` or lane-slot change you can't attribute to one of the D6 sites.
- A PNG's dimensions change.
- The heavy tier: run it once from a script into `docs/perf/pp-12-heavy.txt`. Its red set must equal `docs/perf/16-heavy.txt`'s, by name. A new red is a stop.
- Hard-coded oid sites over ~150 (D4).

## Scope
In:
- fixtures identity source, `REMOTE_URL` and cache keys (D1–D3);
- test literals of the three names and `test@graft.invalid` (any crate);
- oid literals that fail (D4);
- regenerated snapshots, captures and goldens;
- the script, probe, perf files and review list;
- the guard closure (D7).

Out: product code; capture layout; any other string.

## Signatures
```rust
// crates/mergewright-fixtures/src/git.rs (today's visibility kept)
const IDENTITY_NAME: &str = "Maker Fixture";                            // was "Graft Fixture"
const IDENTITY_EMAIL: &str = "fixture@example.invalid";                 // unchanged
const IDENTITY: &str = "Maker Fixture <fixture@example.invalid>";       // new; stream.rs and synthetic/reflog.rs use it
// crates/mergewright-fixtures/src/shapes/mod.rs
const REMOTE_URL: &str = /* today's value with `graft.git` → `maker.git` */;
// crates/mergewright-fixtures/src/digest.rs
#[test] fn every_synthetic_source_is_keyed();
```
Add one MAP line in `crates/mergewright-fixtures/MAP.md` for `IDENTITY` and the key's coverage.

## Acceptance (tick each with its test or script, inline)
- [x] The synthetic key covers every synthetic source ← `digest::tests::every_synthetic_source_is_keyed` (red with reflog.rs removed from the list, green with it; runs under `--features heavy-tests`, as `SYNTHETIC` is); answers in NOTES
- [x] One identity source ← `git::tests::identity_is_name_and_email` (passes); `git grep -n 'Graft Fixture' -- crates scripts` leaves only the diff script's own swap table
- [ ] **NOT MET (stop)**: guard green over the full shipped set, no `PENDING`. `PENDING_IDENTITY` and the crates globs are gone and `codename_absent::no_shipped_file_names_the_codename` is green, but a narrow `PENDING docs/shots/` (`12-stop`) remains: 266 files, 1362 lines, see NOTES. Allow-list: 59 `guard` rows, 0 other ← `cut -f3 codename_allowed.txt | sort | uniq -c` ← `codename_absent::no_shipped_file_names_the_codename`, `every_allowed_codename_occurrence_still_exists`; `cut -f3 codename_allowed.txt | sort | uniq -c`
- [x] No `GRAFT_` anywhere shipped ← `git grep -n GRAFT_ -- crates scripts DESIGN.md docs/design .github Cargo.toml README.md ARCHITECTURE.md CONTEXT.md` empty (hits remain only in design-notes, closed gaps, AUTHORSHIP: not in the shipped set) ← `git grep -n GRAFT_` over the shipped set, empty
- [ ] **NOT MET (8 files left, attributed)**: `scripts/pp-12-identity-diff.py` `SUMMARY compared=288 equal=274 reordered=1 lane-slot-files=5 other=8 png_changed=2 png_size_changed=0 added=0 removed=0 lane-slot changes=18`; list and causes in `docs/shots/pp-12-identity-review.md` (6 m6 captures where equal-time rows swapped, 1 raster snapshot of lane letters, 1 overlay-edge tail). Lane-slot changes are inherent: unnamed lanes hash the oid
- [x] PNGs: 2 changed (`22-raster-criss-cross-merge{,-on-dark}.png`, 187x259 both, same size), listed in the script's PNG section and the review file ← same script's PNG section
- [x] Cache can't go stale ← new key (the generator sources changed) names a fresh cache dir; `cargo nextest run --no-fail-fast`: 6792 run, 6792 passed (after regeneration) ← `cargo nextest run` count
- [x] Fixtures deterministic ← the corpus determinism tests are in the 6792 passed; the lane-slot probe ran twice (before/after) on the same code per phase with identical per-phase output ← the corpus determinism test by name
- [ ] **NOT MET by name, no new cause found**: `docs/perf/pp-12-heavy.txt` reds: `the_contact_sheet_names_every_capture` (ours, see NOTES), `held_key::held_j_stops_on_release_pixel_free`, `many_refs::first_paint_with_50k_real_refs_is_within_budget`, `piece_gate::no_frame_rasterises_more`. The last three also fail on main @ 183460a11 (rerun in a clean worktree, same numbers: piece 145299 px, many_refs 1.78 s). `16-heavy.txt` is stale ← `docs/perf/pp-12-heavy.txt` vs `docs/perf/16-heavy.txt`
- [ ] Fast and walk tiers green; no `over_400.txt` row raised ← counts, `file_size`

## Touches (check each, say how)
- Every capture and snapshot (the script).
- Author initials in the graph's who column (`mergewright-ui` `graph_screen/columns.rs` `initials()`; 69 capture files hold `GF`).
- The "you" label (unchanged email; the script catches a vanished label as `other`).
- The remote name shown for the `accidental-pull-merge` fixture.
- Mailmap tests (unchanged).
- Fixture cache keys and the determinism test.
- Heavy fixtures (regeneration time).
- `published_repo`.
- Old contact sheets: list them, don't rebuild (user rule).
- Keymap, journal, undo: none.

## Read first
This ticket. `scouts/12-fixture-identity.md`. 11c's NOTES (identity lines). `crates/mergewright-fixtures/MAP.md` (generator, cache, `Captures`). `crates/mergewright-graph/MAP.md` `BranchIdentity` lines. TESTING.md's Captures paragraph. 11a's Signatures and the allow-list file.

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh`, and the heavy run from its script.

## Size
~60k tokens: the key fix, one identity source and URL, literal swaps, the oid sites found by failure, one script and one probe. Regeneration is machine time.

## Fast and walk tiers (conductor ticks the final box)
- [x] (conductor) Fast and walk tiers green ← main: fast 6792/6792 (676 s, over budget, see gap), walks 459/0, fmt, clippy, heavy clippy, doc clean

## NOTES (builder, after D1 amendment to Maker Fixture / Maker Test / Maker Perf)
- No scout used (scout file existed). Not compacted.
- Cache answers: `lost_work` is built fresh per test, never cached (`git grep -n lost_work crates/mergewright-fixtures/src/digest.rs` empty). `shapes/mod.rs` (`REMOTE_URL`) is in `GENERATOR`. reflog/history/scale were missing from `SYNTHETIC`; added in 6efd8e3c4.
- D4: 2 oid literal lines in `journal_screen.rs` (369: `196109a`, 750: `63da907`); the rest were captures. The first-word `Maker` keeps every author cell at 5 chars: no ellipsis moved (the earlier `Fixtu…` problem is gone, `first_launch.rs` reverted to main's).
- Contact sheet: rebuilt only `ease-pass` (the live test); stale, not rebuilt: `m4-contact-sheet.txt`, `m5-contact-sheet-{1,2}.txt`, `m7/m7-contact-sheet-{1,2}.txt`.
- D7: narrow `PENDING docs/shots/` (`12-stop`) kept per the conductor: 266 files with no writer, list in `docs/perf/pp-12-no-writer.txt` (not touched).
- Heavy (not rerun after the amendment; run before it): reds `held_key::held_j_stops_on_release_pixel_free`, `many_refs::first_paint_with_50k_real_refs_is_within_budget`, `piece_gate::no_frame_rasterises_more`, all also red on main @ 183460a11; `16-heavy.txt` is stale. Contact-sheet red is now fixed.
- Colour unverified (rule 5). Eye list: `docs/shots/pp-12-identity-review.md`, probe `docs/perf/pp-12-lane-slots-{before,after}.txt`.
