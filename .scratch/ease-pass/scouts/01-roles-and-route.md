# Scout 01 roles-and-route (read-only, main @ 14021deb)

## graft-graph
- `BranchIdentity`: `identity.rs:33`, derives `Clone, Copy, Debug, PartialEq, Eq, Hash` (:32) — fits `LaneRole`/`Route` use. `of_name(&[u8])` `identity.rs:38` (bytes, not `&str`).
- `LaneIdentity { Known(BranchIdentity), Merge{..} }` `lane.rs:15`; `known()` `lane.rs:31`.
- **`Row` does not hold `LaneIdentity`.** `Row.lanes: Vec<Option<BranchIdentity>>` `row.rs:45`; `Row.lane: usize` `row.rs:24`. `LaneIdentity` lives on `RowShape.lanes` `row.rs:66` (pre-assembly; `Row::resolve` `row.rs:165`). So `made_on(&Row)` reads `row.lanes[row.lane]`; spec's "`LaneIdentity::known`" does not apply to `Row`.
- `lib.rs:11-33`: modules + re-exports; no `role` module. `lane` is `pub mod` but only `Lane` re-exported (:26).
- Deps: core, thiserror, raster crate (`Cargo.toml:9-15`) — no git.

## graft-ops / graft-git reads the spec names
- `merge_base(repo,a,b) -> Result<Option<Oid>,RunError>` `graft-git/src/history_reads.rs:37` — **takes no stop** (runs with `&()`).
- `count_between(repo, exclude, include, stop) -> Result<usize,RunError>` `history_reads/compare.rs:66` — **usize, spec uses u32**.
- `compare_span(repo, base, tip, stop)` `compare.rs:36` — merge-base with stop; the `stop: &dyn Stop` form.
- `ahead_behind(handle, local, upstream) -> Result<AheadBehind, WalkError>` `history/divergence.rs:52` — **no stop, error is `WalkError` not `RunError`**; fields `ahead/behind: usize` (:27-31).
- `Stop` trait `run.rs:118`; `RunError::Stopped` `run.rs:186` — exists, the "Stopped-equivalent" is literal.
- `CommitSummary` `history_reads.rs:60`: id, parents, subject, author: `Identity`. `Identity{name: Vec<u8>, email, when: Timestamp}` `graft-core/src/commit.rs:33` — `Who.last` obtainable from `author.when`; names are bytes.
- `Tip { name: String /* full ref name */, oid }` `graft-ops/src/operation/history.rs:46`, re-exported via `operation.rs:51`. Full ref names (`refs/heads/main`) vs `Candidate.name` form: could not confirm which form spec intends.
- graft-ops deps: core, git, serde, serde_json (`Cargo.toml:9-15`). No `branch_status` module.

## Name clashes (grep `*.rs`, -w)
- `LaneRole`, `Route`, `BranchStatus`, `branch_status`, `choose_base`, `made_on`, `Candidate`, `CandidateKind`, `Apart`, `Pushed`, `Behind`: none.
- `Who`: `graft-ui/src/blame.rs:34` (other crate, no clash in ops).
- `Merged`: `IndexState::Merged` variant `graft-core/src/journal.rs:229` (variant only; no type clash).

## Keys / drawing / tests
None — no screen, key, or job touched. No existing tests cover these names.
