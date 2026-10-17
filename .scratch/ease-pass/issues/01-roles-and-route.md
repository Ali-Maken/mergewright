# roles-and-route — what colour a lane is, and what a branch was built on

Status: ready
Blocked by: none
Build with: Fable (signature) — every graph pane (rows, connectors, pixel tier, branch pane) draws from these types; a wrong shape is structural.
Needs: none (calls existing reads only; facts from `.scratch/ease-pass/scouts/01-roles-and-route.md`)

## Read first
- `docs/design-notes/ease-pass.md`: Prior art (rows "Which line is which", "Branch status"), Hard-to-reverse choice 1, Mock behaviours G1, G9, G12, Interfaces and data shapes.
- `.scratch/ease-pass/scouts/01-roles-and-route.md` (whole, ≤1 page) — its file:line facts are the ground this spec stands on.
- `crates/graft-graph/MAP.md`: `BranchIdentity`, `LaneIdentity`, `row` (`Row`), `seeds`.
- `crates/graft-ops/MAP.md`: top-level list, `Tip`. `crates/graft-git/MAP.md`: `history_reads` (`count_between`, `commits_between`, `CommitSummary`, `Range`), `Stop`, `RunError`, `RepoHandle`.
- `ARCHITECTURE.md` crate rules (graph depends on core only; ops on git + core).

## Scope
In:
- graft-graph: new module `role` — `LaneRole`, `Route`, `made_on`. Pure; no I/O.
- graft-ops: new module `branch_status` — base choice, route walk, the four answers of the right pane (pushed, merged, behind, who). Reads only, through **existing** graft-git reads; no new git read, no graft-git edit.
- Unit + fixture-repo tests (fast tier), MAP.md entries for both modules.
Out: drawing (G, X, B), the app job that calls these (B), caching per tips (B), colour tokens (V), mapping names to `BranchIdentity` (B).
Scout: done (file above).

## Decisions fixed by the scout (not latitude)
- **Names are full ref names.** `Candidate.name` = `refs/heads/main` form, same as `Tip.name` (`graft-ops/src/operation/history.rs:46`). Display uses `short_name`. Which form the layout hashes with `BranchIdentity::of_name(&[u8])` is B's to match (09 has the acceptance line).
- **Counts are `usize`** — `count_between` returns `usize` (`compare.rs:66`).
- **Every read takes `stop: &dyn Stop` and fails with `RunError`.** So: `Apart`, `Behind`, `Merged` and `Pushed` all come from `count_between(repo, exclude, include, stop)`; `Who` and the stacked check from `commits_between`. **Not used:** `ahead_behind` (no stop, `WalkError`, `divergence.rs:52`) and `merge_base` (no stop, `history_reads.rs:37`). If `commits_between` turns out to take no stop, check `stop.stopped()`-equivalent before and after it and say so in NOTES — do not add a stop to graft-git here.
- **`made_on` reads `Row.lanes`, not `LaneIdentity`.** `Row.lanes: Vec<Option<BranchIdentity>>` (`row.rs:45`), `Row.lane: usize` (`row.rs:24`).

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
/// The branch a row was made on: `row.lanes[row.lane]`; `None` when that lane is unnamed or out of range.
pub fn made_on(row: &Row) -> Option<BranchIdentity>;
```
graft-ops `branch_status` (names, not identities — ops does not depend on graph):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateKind { Trunk, Integration, Other }   // tie order
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Candidate { pub name: String /* full ref name */, pub tip: Oid, pub kind: CandidateKind }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apart { pub branch_only: usize, pub base_only: usize }

/// `refs/heads/x` → `x`, `refs/remotes/o/x` → `o/x`, `refs/tags/t` → `t`; anything else unchanged.
pub fn short_name(full: &str) -> &str;

/// Pure. Excludes the candidate whose `name == branch` and those with `branch_only == 0`;
/// minimum `branch_only`; ties by `kind`, then by name.
pub fn choose_base<'a>(branch: &str, measured: &'a [(Candidate, Apart)]) -> Option<&'a Candidate>;

/// Bases nearest first, ending at the trunk when reached. Depth cap 8, cycle guard.
pub fn route(repo: &RepoHandle, branch: &Candidate, candidates: &[Candidate], stop: &dyn Stop)
    -> Result<Vec<Candidate>, RunError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchStatus { pub route: Vec<Candidate>, pub pushed: Pushed, pub merged: Merged, pub behind: Behind, pub who: Who }
pub enum Pushed { NoUpstream, InSync, ToPush(usize), ToPull(usize), Both { push: usize, pull: usize } }
pub enum Merged { MainLine, Into(String), NotYet { into: String } }            // full ref names
pub enum Behind { NoBase, UpToDate { base: String }, Behind { base: String, count: usize } }
pub struct Who { pub first: Vec<Vec<u8>>, pub others: usize, pub everyone: bool, pub last: Timestamp }

pub fn branch_status(repo: &RepoHandle, branch: &Candidate, candidates: &[Candidate],
    upstream: Option<&Tip>, stop: &dyn Stop) -> Result<BranchStatus, RunError>;
```
All derive `Clone, Debug, PartialEq, Eq`. `Timestamp` is `graft_core::Timestamp`; `Tip` is graft-ops' own (`operation::history`).

Meaning:
- `Apart` for a candidate `c`: `branch_only = count_between(exclude c.tip, include branch.tip)`, `base_only = count_between(exclude branch.tip, include c.tip)`.
- `Pushed` — `count_between` both ways against `upstream.oid`; `None` → `NoUpstream`.
- `Behind` — `route[0]`'s `base_only` (commits on the base since the branch left it); empty route → `NoBase`; the trunk measures against its upstream (`base` = upstream name, count = the pull count).
- `Merged` — trunk → `MainLine`. Else: **containers** = candidates other than the branch with `branch_only == 0` and `tip != branch.tip`, minus those the branch is **stacked under** (the branch tip lies on the container's first-parent line: in `commits_between(exclude branch.tip, include c.tip)`, following `parents[0]` from `c.tip` reaches a commit whose `parents[0] == branch.tip`; a capped range → not a container). Nearest container (fewest `base_only`, ties kind then name) → `Into(name)`. None → `NotYet { into }`, `into` = `route[0].name`, or the trunk's name when the route is empty. Why: `route[0]` always has `branch_only > 0`, so it can never contain the branch; and a teammate's branch stacked on this one must not read as "merged".
- `Who` — authors of the branch-only commits (`commits_between(exclude route[0].tip, include branch.tip)`; empty route → the capped range from the branch tip), grouped by `author.email`, ordered by commit count then newest; `first` = up to 2 `author.name` bytes (the view takes the first word), `others` = the rest; trunk → `everyone: true`, `first` empty; `last` = newest commit's `author.when` (trunk: its tip's).

## Touches
- graft-graph `lib.rs` (new `pub mod role` + re-exports), MAP.md; graft-ops `lib.rs`, MAP.md. No caller yet (G, X, B call these). Name clashes already grepped by the scout: none for the new names; `Who` exists in `graft-ui/src/blame.rs:34` (other crate), `Merged` only as `IndexState::Merged` variant (`graft-core/src/journal.rs:229`) — no clash; re-check the grep at the end and report it.
- No screen, capture, key or journal change. No graft-git file.

## Acceptance
- [ ] `Route::role` table test: focus, trunk, chain member, stranger, focus == trunk → `This`, focus `None`.
- [ ] `made_on` test: a `Row` with `lanes[lane] == Some(id)` → `Some(id)`; `None` lane → `None`; `lane` out of range → `None`.
- [ ] `short_name` table test: heads, remotes, tags, an unprefixed name.
- [ ] `choose_base` property test: result has the minimum `branch_only`, never 0, never `branch` itself; ties follow kind then name.
- [ ] Fixture repo `main ← wallet-release ← sara/fix-toc` (+ unrelated `ops/ci`): `route(refs/heads/sara/fix-toc)` = `[wallet-release, main]` (full names); `route(main)` = `[]`; a branch whose tip equals a candidate's tip skips that candidate.
- [ ] Cycle and depth cap tests (two branches at one tip; chain of 10 stops at 8).
- [ ] `branch_status` fixture tests: each `Pushed` variant; `Merged::Into(wallet-release)` after `sara/fix-toc` is merged into it with a merge commit, `NotYet { into: wallet-release }` before; a branch `sara/fix-toc-2` made on top of `sara/fix-toc` does **not** make `sara/fix-toc` read `Into(sara/fix-toc-2)`; `Behind` count after adding 3 commits to the base; `Who` with 1, 2, 4 authors (4 → `first.len() == 2`, `others == 2`); trunk → `everyone`.
- [ ] `route` and `branch_status` honour `stop`: a stopped `Stop` → `Err(RunError::Stopped)` (`run.rs:186`).
- [ ] MAP.md entries for both modules. Each tick names its test.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium (~55k tokens).
