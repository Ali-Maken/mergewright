# refresh-reopens-walk: `Ctrl+R` reopens the walk on the commit-graph path after `B` wrote one (KNOWN_GAPS row 54)

Status: ready
Blocked by: none

Status: draft
Build with: Sonnet — a rebind on refresh with one decision already made here (rebind the table, restart the walk, re-anchor the selection); testable end to end on a fixture.
Needs: none from M2 (M1 code only: `RepoStore`, `CommitTable`, `commit_graph::status`, `History`, `CommitGraphLine`)

## Read first

- `KNOWN_GAPS.md` line 54 (the row this closes) and line 179 (a walk holds a worker until `Ctrl+R` drops its sender — so a refresh already ends the running walk)
- `docs/design-notes/m2-details-and-diffs.md` — §8 coverage row "KG 54"; Sizing from M1 rules (1), (4), (5)
- `crates/graft/MAP.md` lines 183–193 (`repos`: `RepoStore::table` "kept across `refresh`", `RepoStore::opens`, `RepoStore::refresh`), 100–104 (`AppState::repository`, `commit_graph`, `CommitGraphOffer::Written`), 140–149 (`RepositoryFacts::commit_graph`, `CommitGraphFact`), 264–308 (`history`: `History::walk`, `WalkFacts::order`/`is_provisional`, `History::reanchor`, `History::show`, `History::view` "kept across refresh"), 128 (`Command`)
- `crates/graft-git/MAP.md` lines 23–28 (`commit_graph::status`, `write`, `CommitGraphStatus`), 38 (`Walk` reads with/without a graph), 46 (`Walk::has_commit_graph`), 50–52 (`CommitTable::open`/`has_commit_graph`), 83–86 (`SharedRepo`)
- `crates/graft-ui/MAP.md` line 55 (`CommitGraphLine::Written` wording per width)
- One scout summary (haiku) of: where the `Ctrl+R` key arm is, which `Command` it emits, and which job/loop code calls `RepoStore::refresh` and restarts the history walk — a page or less; never `update/keys.rs` or `event_loop` whole.

## Scope

In:
- `crates/graft/src/repos.rs`: `RepoStore::refresh` also compares `commit_graph::status(&handle)` (`Present` vs `Absent`) with `self.table().has_commit_graph()`; when they differ it opens a new `CommitTable` (`CommitTable::open`) and replaces the shared `Arc`, bumping `opens`. If `SharedRepo`/gix caches the graph's absence so that a new table on the old `SharedRepo` still binds nothing (check `CommitTable::open`'s MAP line; verify with the fixture test below before deciding), reopen the `SharedRepo` too — `RepoStore::new` from scratch is acceptable; say which in the report. `refresh` returns `Refreshed { table_rebound: bool }` so the caller knows.
- The refresh path (wherever `Ctrl+R`'s command is carried out, per the scout): when `table_rebound`, the history walk restarts as at startup on the new table — the old `History` rows are keyed by handles of the old table and must not be reused — with `History::reanchor` set to the selected commit's `Oid` (via `id_of` on the old table before it is dropped) so the cursor lands back on it, and `History::view` kept (already the rule). When not rebound, refresh does what it did in M1. The detail pane's `Ctrl+R` rule from `detail-model-and-jobs` (caches cleared, spec re-asked) is untouched.
- `crates/graft-ui` `CommitGraphLine::Written { commits }` gains ` · Ctrl+R to use it` at ≥ 100 columns (the < 100 wording stays as is: no room). The `Written` footer line is cleared by the next key already; `Ctrl+R` is that key.
- `KNOWN_GAPS.md` row 54: mark closed by this ticket (the same strike-through form the other closed rows use).
- Tests: see Acceptance. MAP.md lines (`graft`: `RepoStore::refresh`/`Refreshed`; `graft-ui`: `CommitGraphLine`).

Out: auto-reopening when `Product::CommitGraph(Ok)` lands (the user asks with `Ctrl+R`; an unasked restart mid-scroll is worse than a footer hint); anything about the decline path; `Walk::full_topological` residual (KG 227, "not planned" per Proposed changes).

## Signatures

```rust
// crates/graft/src/repos.rs
pub struct Refreshed { pub table_rebound: bool }
impl RepoStore { pub fn refresh(&mut self) -> Result<Refreshed, StoreError>; }   // was: the M1 signature — keep its error type if it already returns one; add `Refreshed` as the Ok value

// crates/graft-ui (graph_screen section)
// CommitGraphLine::Written { commits } — wording at ≥ 100 cols ends ` · Ctrl+R to use it`
```

If `refresh` today returns `()`, changing it to `Result<Refreshed, StoreError>` is the one signature change; name every caller you touched.

## Acceptance

- [ ] Fixture test (`Shape::Linear`, temp copy, system git present else `skip` with a printed reason): open a `RepoStore` (no commit-graph → `table().has_commit_graph() == false`), run `commit_graph::write`, `refresh()` → `Ok(Refreshed { table_rebound: true })`, `table().has_commit_graph() == true`, `opens()` incremented; a second `refresh()` → `table_rebound: false`.
- [ ] `refresh()` with no graph change → `table_rebound: false`, `opens()` unchanged, `table()` the same `Arc` (pointer-equal).
- [ ] Update/loop test with the scripted `Wakes` (the pattern `event_loop::tests` already uses): after a rebound refresh, the new walk's `WalkFacts::order` is the requested topological order and `is_provisional() == false` (the graph path), and the cursor is back on the commit selected before `Ctrl+R`.
- [ ] `CommitGraphLine::Written` snapshot at 120 cols ends `· Ctrl+R to use it`; at 80 cols the M1 text is unchanged.
- [ ] KNOWN_GAPS row 54 closed; MAP.md updated.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

Two crates: `graft` (`repos.rs` ~40 lines, the refresh path ~40), `graft-ui` (~10). Tests ~120. ≤ 20 tool calls. Named seam: if the history restart on a rebound table needs more than setting `reanchor` and re-issuing the startup walk request (e.g. `History` has no way to drop its rows short of a new `History`), stop after the `repos.rs` change + fixture test, commit, and report `partial` naming exactly what `History` lacks.
