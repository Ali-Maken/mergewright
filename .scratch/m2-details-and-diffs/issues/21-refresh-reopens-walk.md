# refresh-reopens-walk: `Ctrl+R` reopens the walk on the commit-graph path after `B` wrote one (KNOWN_GAPS row 54)

Status: done
Blocked by: none
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

- [x] Fixture test (`Shape::Linear`, writable temp copy via `copy_into_temp` — `borrow`'s fixtures are read-only and system git is already required to build the corpus, so no separate skip path exists): open a `RepoStore` (no commit-graph → `table().has_commit_graph() == false`), run `commit-graph write --reachable`, `refresh()` → `Ok(Refreshed { table_rebound: true })`, `table().has_commit_graph() == true`, `opens()` incremented; a second `refresh()` → `table_rebound: false`. See `repos::tests::a_refresh_rebuilds_the_table_when_the_commit_graph_appears`.
- [x] `refresh()` with no graph change → `table_rebound: false`, `opens()` unchanged, `table()` the same `Arc` (pointer-equal). See `repos::tests::a_refresh_with_no_commit_graph_change_keeps_the_table`.
- [x] Update/loop test — split into two (see Notes below, deviation 5): `jobs::tests::refresh_after_a_commit_graph_write_puts_the_next_walk_on_the_graph_path` (real `JobRunner`, `WalkFacts::is_provisional() == false` after the graph appears) and `update::tests::a_walk_restart_reanchors_on_the_previously_selected_commit` (pure, cursor restoration).
- [x] `CommitGraphLine::Written` snapshot at 120 cols ends `· Ctrl+R to use it`; at 80 cols the M1 text is unchanged. See `graft_ui::graph_screen::tests::every_commit_graph_wording_fits_under_its_breakpoint`.
- [x] KNOWN_GAPS row 54 closed; MAP.md updated (`crates/graft/MAP.md`, `crates/graft-ui/MAP.md`).

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

Two crates: `graft` (`repos.rs` ~40 lines, the refresh path ~40), `graft-ui` (~10). Tests ~120. ≤ 20 tool calls. Named seam: if the history restart on a rebound table needs more than setting `reanchor` and re-issuing the startup walk request (e.g. `History` has no way to drop its rows short of a new `History`), stop after the `repos.rs` change + fixture test, commit, and report `partial` naming exactly what `History` lacks.

## Notes (done)

Investigation confirmed: `gix`'s `commit_graph_if_enabled()` re-reads the
commit-graph file from disk fresh on every call — no caching at
`SharedRepo`/`gix::Repository` level (checked against gix 0.87.1 source
directly). **No `SharedRepo` reopen is needed**, only the `CommitTable`.

Disclosed deviations from the literal spec:

1. **`refresh` stays `&self`.** `RepoStore` is held as `Arc<RepoStore>` and
   already uses interior mutability (`Mutex`) for every cached field; `&mut
   self` was never reachable through the `Arc`. The signature block's
   `&mut self` looks like a slip, not a deliberate ask.
2. **The id table is rebuilt only when a table was already cached.** On the
   very first `refresh()` before anything ever called `table()`, there is
   nothing to rebind, so `table_rebound` is `false` and the table is left to
   build lazily on first use, same as M1. Building eagerly here broke
   `two_jobs_share_one_open_repository` (this crate's own repo already
   carries a commit-graph) by counting a redundant open before anything had
   asked for the table.
3. **`opens()`'s doc/semantics broadened**: it now also counts a table
   rebuild, not only a from-scratch repository open (its own doc comment and
   MAP.md line updated to say so).
4. **`update()` reanchors on every walk restart, not gated on
   `table_rebound`.** `update` is pure and has no way to see the
   synchronously-computed `Refreshed::table_rebound` (that fact lives on the
   job-runner thread, in `RepoStore::refresh`'s return value, which nothing
   currently carries across to `AppState`/`update`) without new `Product`/
   `Task` plumbing that is out of this ticket's signature scope. Reanchoring
   unconditionally is harmless: on an ordinary M1 refresh the new walk's rows
   still contain the previously-selected commit, so `reanchor` finds it and
   the cursor lands in the same place it would have anyway (this is exactly
   `change_view`'s existing idiom, reused here).
5. **The "Update/loop test with the scripted `Wakes`" acceptance item is
   two tests, not one.** `event_loop::tests`' `ScriptedWakes`/`pump` harness
   has no precedent for driving a real `JobRunner`/real git — it only ever
   drives a fake `Dispatch`. Implemented instead as:
   - `jobs::tests::refresh_after_a_commit_graph_write_puts_the_next_walk_on_the_graph_path`
     — real `JobRunner` over a writable fixture copy: dispatches
     `ReadHistory`, cancels it, runs `git commit-graph write --reachable`,
     dispatches `OpenRepository` (the refresh), then `ReadHistory` again and
     asserts `WalkFacts::is_provisional() == false`.
   - `update::tests::a_walk_restart_reanchors_on_the_previously_selected_commit`
     — pure `update()` test: a walk with the merge selected, a fresh
     `AppEvent::Started(_, Task::ReadHistory)`, asserts `history.reanchor ==
     Some(merge)`.

Verified: `cargo fmt --check`, `cargo test` (full workspace, quiet), `cargo
clippy --all-targets -- -D warnings`, `cargo build --quiet` all pass. The
real-git test above is the actual end-to-end evidence for the walk restart —
run against a temp copy of the `Linear` fixture, real `git commit-graph
write --reachable`, real `JobRunner`. Not observed: the on-screen footer
wording change and cursor behaviour in an actual running terminal (no pty
capture was taken for this ticket; it is not visual/layout work, only a
string change and a state-machine change covered by the snapshot and unit
tests above).
