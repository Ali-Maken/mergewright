# roles-and-route — what colour a lane is, and what a branch was built on

Status: ready
Blocked by: none
Build with: Fable (signature) — every graph pane (rows, connectors, pixel tier, branch pane) draws from these types; a wrong shape is structural.
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: Prior art (rows "Which line is which", "Branch status"), Hard-to-reverse choice 1, Mock behaviours G1, G9, G12, Interfaces and data shapes.
- `crates/graft-graph/MAP.md`: sections for `BranchIdentity`, `LaneIdentity`, `row`, `seeds`.
- `crates/graft-ops/MAP.md`: top-level list; `crates/graft-git/MAP.md`: `merge_base`, `count_between`, `ahead_behind`, `RepoHandle`, `Stop`, `RunError`, `CommitSummary`.
- `ARCHITECTURE.md` crate rules (graph depends on core only; ops on git + core).

## Scope
In:
- graft-graph: new module `role` — `LaneRole`, `Route`, `made_on`. Pure; no I/O.
- graft-ops: new module `branch_status` — base choice, route walk, the four answers of the right pane (pushed, merged, behind, who). Reads only.
- Unit + fixture-repo tests (fast tier), MAP.md entries for both modules.
Out: drawing (G, X, B), the app job that calls these (B), caching per tips (B), colour tokens (V).
Scout: not needed — two new modules; only `lib.rs` export lines and MAP.md change in existing files. Conductor may order one.

## Signatures
graft-graph `role`:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneRole { This, Under, Trunk, Other }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route { /* trunk, chain (nearest base first, trunk excluded), focus */ }
impl Route {
    /// `built_on`: bases nearest first, trunk excluded.
    pub fn new(trunk: BranchIdentity, built_on: &[BranchIdentity], focus: Option<BranchIdentity>) -> Self;
    /// Before the route is read: no chain yet.
    pub fn unread(trunk: BranchIdentity, focus: Option<BranchIdentity>) -> Self;
    pub fn focus(&self) -> Option<BranchIdentity>;
    pub fn trunk(&self) -> BranchIdentity;
    pub fn chain(&self) -> &[BranchIdentity];
    /// focus → This; else trunk → Trunk; else in chain → Under; else Other.
    pub fn role(&self, branch: BranchIdentity) -> LaneRole;
}
/// The branch a row was made on: its lane's known identity (`LaneIdentity::known`).
pub fn made_on(row: &Row) -> Option<BranchIdentity>;
```
graft-ops `branch_status` (names, not identities — ops does not depend on graph; the app maps with `BranchIdentity::of_name`):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidateKind { Trunk, Integration, Other }   // tie order
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate { pub name: String, pub tip: Oid, pub kind: CandidateKind }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apart { pub branch_only: u32, pub base_only: u32 }

/// Pure. Excludes candidates named `branch` and those with `branch_only == 0`;
/// minimum `branch_only`; ties by `kind`, then by name.
pub fn choose_base<'a>(branch: &str, measured: &'a [(Candidate, Apart)]) -> Option<&'a Candidate>;

/// Bases nearest first, ending at the trunk when reached. Depth cap 8, cycle guard.
pub fn route(repo: &RepoHandle, branch: &Candidate, candidates: &[Candidate], stop: &dyn Stop)
    -> Result<Vec<Candidate>, RunError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchStatus { pub route: Vec<Candidate>, pub pushed: Pushed, pub merged: Merged, pub behind: Behind, pub who: Who }
pub enum Pushed { NoUpstream, InSync, ToPush(u32), ToPull(u32), Both { push: u32, pull: u32 } }
pub enum Merged { MainLine, Into(String), NotYet { into: String } }
pub enum Behind { NoBase, UpToDate { base: String }, Behind { base: String, count: u32 } }
pub struct Who { pub first: Vec<String>, pub others: u32, pub everyone: bool, pub last: Timestamp }

pub fn branch_status(repo: &RepoHandle, branch: &Candidate, candidates: &[Candidate],
    upstream: Option<&Tip>, stop: &dyn Stop) -> Result<BranchStatus, RunError>;
```
`stop` takes the crate's existing stop form (as `compare_span` does); that is the only latitude. All derive `Clone, Debug, PartialEq, Eq`.

Meaning: `Merged` — trunk → `MainLine`; else into its nearest base (`route[0]`), by tip reachability. `Behind` — commits on the base since the branch left it (merge-base); the trunk measures against its upstream. `Who` — first names by commit count on branch-only commits, `first` ≤ 2, `others` the rest; trunk → `everyone: true`; `last` = newest commit's date.

## Touches
- graft-graph `lib.rs` exports, MAP.md; graft-ops `lib.rs`, MAP.md. No caller yet (G, X, B call these). Grep `role`/`Route`/`BranchStatus` for name clashes across the workspace and report it.
- No screen, capture, key or journal change.

## Acceptance
- [ ] `Route::role` table test: focus, trunk, chain member, stranger, focus == trunk → `This`, focus `None`.
- [ ] `choose_base` property test: result has the minimum `branch_only`, never 0, never `branch` itself; ties follow kind then name.
- [ ] Fixture repo `main ← wallet-release ← sara/fix-toc` (+ unrelated `ops/ci`): `route(sara/fix-toc)` = `[wallet-release, main]`; `route(main)` = `[]`; a branch whose tip equals a candidate's tip skips that candidate.
- [ ] Cycle and depth cap tests (two branches at one tip; chain of 10 stops at 8).
- [ ] `branch_status` fixture tests: each `Pushed` variant; `Merged::Into` after a merge, `NotYet` before; `Behind` count after adding 3 commits to the base; `Who` with 1, 2, 4 authors; trunk → `everyone`.
- [ ] `route` honours `stop` (returns `RunError::Stopped`-equivalent when stopped).
- [ ] MAP.md entries for both modules. Each tick names its test.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium (~50k tokens).
