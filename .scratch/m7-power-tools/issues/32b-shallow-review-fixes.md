# 32b shallow-review-fixes — review m7-32-20i's graft-git Minors (1, 2, 4)

Status: done
Blocked by: **18f** (building in graft-git now; its spec was fixed before these were found, so they are not appended
there).
Crates: graft-git only (`reflog.rs`, `shallow.rs`, `tests/it/shallow_history.rs`, graft-git `MAP.md`). Needing graft
(an exhaustive `ReflogError` match) → commit at the seam, report `blocked: needs graft` naming the site, stop.
Build with: **Sonnet**. Three small fixes, each decided below; one follows a rule 32 already set.
Scout: none. The review names every line (`docs/reviews/m7-32-20i.md` Minors 1, 2, 4, at `cb065156`); 18f moves
`blame*` only, but re-read each site.
Needs: `graft_git::shallow::ShallowList::read(&RepoHandle) -> Result<ShallowList, ShallowError>`,
`ShallowList::contains` (32).
Split by: planner, 2026-09-27, from review m7-32-20i (gap rows M7/32i-m1, -m2, -m4).

## Minor 1: `reflog::first_parent_distance` walks past the shallow boundary
`reflog.rs:108` walks `commit.parent_ids()`; MAP `:182` says every reader goes through `ShallowList`. **Ruling: fix the
code, keep the MAP line** (git's rule is the oracle; a distance across a cut git can't see is wrong).
Read the list once per call; a listed commit has no parents, so the walk stops there (`Ok(None)` if `to` wasn't
reached). An unreadable list is an error (rule 3, as 32): `ReflogError::Shallow(ShallowError)`.

## Minor 2: four entry points with no unreadable-`shallow` test
`CommitTable::open` (`table.rs:53`), `commit_graph::status` (`commit_graph.rs:119-121`), `ahead_behind`
(`divergence.rs:63`), `first_parent_chain` (`read.rs:139`), plus `first_parent_distance` (Minor 1). Extend
`an_unreadable_shallow_file_fails_the_walk`'s setup (`shallow_history.rs:330-357`) to each; each asserts its own
`Shallow` variant, not just `is_err()`.

## Minor 4: `ShallowList::read_in`'s doc names a caller that doesn't exist
**Ruling:** fold `read_in` into `read`. If a test calls it, keep it private and write the doc as "the reader behind
`read`, given the common directory"; say which in NOTES.

## Signatures
```rust
// graft-git::reflog — gains one variant; its Display is ShallowError's ("couldn't read <path>: <source>")
pub enum ReflogError { /* existing variants unchanged */ Shallow(ShallowError) }
```
`first_parent_distance(repo, from, to, limit) -> Result<Option<usize>, ReflogError>`: signature unchanged (MAP
`:205`). `ShallowList::read`: unchanged.

## Acceptance (name the verifying test at tick time)
- [x] **Red first** (`shallow_history::first_parent_distance_stops_at_a_shallow_boundary`; red quoted in NOTES) `first_parent_distance_stops_at_a_shallow_boundary`: full history, a hand-written `shallow` listing
      a middle commit (as 32's graph test does), `to` below it → `None`. The same repo without the file → `Some(n)`
      (non-vacuous). Quote the red.
- [x] `shallow_history::first_parent_distance_fails_on_an_unreadable_shallow_file` → `Err(ReflogError::Shallow(_))`.
- [x] Minor 2 (`shallow_history::an_unreadable_shallow_file_fails_the_walk`, extended; mutations in NOTES): `an_unreadable_shallow_file_fails_the_walk` (or one test per entry, named in NOTES) covers all four;
      each asserts its variant. **Mutation:** one `?` → `.unwrap_or_default()` per entry, each red; say which.
- [x] Minor 4 (grep in NOTES): `grep -n 'read_in' crates/` in NOTES.
- [x] MAP.md (lines 177, 185, 207; read back by grep) (edited, read back): `first_parent_distance` "stops at a shallow boundary", `ReflogError::Shallow`,
      `ShallowList` (no `read_in`, or its doc corrected).
- [x] Gates (nextest 4164/4164, 3 skipped; fmt, clippy, doc clean) workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
      `cargo test --doc`.

## Touches (check each; NOTES says how)
- **`ReflogError` matches** in graft (the lost-work / journal reads, m7/22–24): grep; a wildcard or `to_string` needs
  nothing; an exhaustive match → blocked (above).
- **`first_parent_distance` callers:** grep; results change only in a shallow repo with the boundary's parent present.
- **Perf:** one `shallow` read per call (a stat when absent). `docs/perf/` not re-run; claim nothing.
- **graft-ui, captures, keymap, journal, undo:** none.
- **For the conductor:** closes M7/32i-m1, -m2, -m4.

## Size
~25k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## NOTES (builder, 2026-09-27)
- Scout: none (ticket says none; every site re-read directly).
- Red: compile first (`no variant ... Shallow found for enum ReflogError`, shallow_history.rs:404); after adding the
  variant: `first_parent_distance_stops_at_a_shallow_boundary` -> `left: Some(4) right: None`; unreadable test panicked at :403.
- Fix: `reflog.rs` reads `ShallowList` once, breaks when `at` is listed. `ReflogError::Shallow(#[from] ShallowError)`
  (transparent) in `history/read.rs`. Cause fix.
- Fixture: `FeatureBranches` depth-1000 clone (full history), `.git/shallow` = `main~2`, `to` = `main~4`.
- Mutations (`?` -> `.unwrap_or_default()`), each red at its own assert: table.rs:53 -> :361; commit_graph.rs:120 -> :367;
  divergence.rs:63 -> :373; read.rs:139 -> :384; reflog.rs -> distance test :403.
- Minor 4: `read_in` folded into `read` (no test called it). `grep -rn 'read_in' crates --include=*.rs` -> no hits
  (only unrelated `read_info`/`inside`).
- Touches: `ReflogError` outside graft-git: only `graft-ops/src/facts.rs:92` `#[from]` (no match) -> nothing.
  `first_parent_distance` caller `graft-ops/src/journal_list.rs:197` (`?`) -> differs only in shallow repos. Perf not
  re-run; no claim. UI/captures/keymap/journal/undo: none.
