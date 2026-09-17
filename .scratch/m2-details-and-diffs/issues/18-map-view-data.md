# map-view-data: the Map view's per-branch bucket counts, as a cancellable job, plus its state and keys — no drawing

Status: ready
Blocked by: 01, 04, 06, 13, 13b

Status: draft
Build with: Sonnet — one bounded walk per branch into fixed buckets, wired through the job pattern `jobs::detail` already set; every rule below is decided, every transition testable.
Needs: `git-runner` (`Stop`), `detail-model-and-jobs` (`Loading<T>`, the `jobs::detail` Task/Product/Command pattern and its `Cancel`→`Stop` adapter), `radar-context-and-markers` (`first_parent_chain`, `RadarState::policy`, `RadarContext::previous_default_tips`), `mistake-radar-detection` (`classify_commit`, `CommitFacts`, `RepoFacts`, `Policy`, `Severity`)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Sizing from M1; Interfaces, the `app::map` line; Proposed changes row "Map view draws text glyphs at every tier"; New UI "Map view (`M`)" in full; this ticket's row in §8 coverage
- `DESIGN.md` §5.1 lines 1159–1181 (Shows / Toggle / Cost — the walk is per branch, no lanes, built on toggle-on, discarded on toggle-off) and §2.3 line 434 (`M`)
- `crates/graft/MAP.md` lines 183–193 (`repos`: `RepoStore::shared`/`refs`/`branches`/`table`), 86–131 (`update`: `Task`, `Product`, `Progress`, `Command`, `AppState::screen`, `Harness::clock`), 241–262 (`jobs`), 25 (`History::seek`), 56–60 (`RefBadges::head`)
- `crates/graft-git/MAP.md` lines 33–61 (`history`: `Walk::from_tips`, `Order`, `Chunk`, `next_chunk`, `CommitTable`, `read_commit`) and 76–95 (`RefSnapshot`, `RefEntry`, `RefTarget`, `HeadState`)
- `crates/graft-core/MAP.md` lines 19–29 (`Commit`, `Timestamp`, `CommitHandle`, `HandleSet`), 48–60 (`mistake_radar`, `Oid`, `RefName`)
- `crates/graft-graph/MAP.md` lines 7–11 (`BranchIdentity::of_name`)
- `.scratch/m2-details-and-diffs/issues/radar-context-and-markers.md` — Signatures block (`first_parent_chain`, `RadarContext`) and its `Task`/`Product`/`Command` naming
- `crates/graft/MAP.md` line 44 (`Clock`): `now_seconds()` is the one clock read; the job reads it, `update` never does

## Scope

In:
- `crates/graft/src/map.rs` (new top-level module, `pub mod map`; the pure data + the builder, no job plumbing):
  - Types below. `MapData::buckets` is fixed by the request; every `MapBranch::counts` has exactly that length.
  - `build_map`: reads local branches from `RepoStore::refs()` (`refs/heads/*` only — remotes are not rows in M2, per the note), the default branch from `RepoStore::branches()`, HEAD from `RepoHandle::head`, tags from `RefSnapshot::peeled` (`refs/tags/*`). Row order: current branch first, then the default, then the rest by tip date, newest first; at most `MAX_ROWS` (200) rows, the rest counted in `more`. Tip dates via `read_commit` (one per local branch).
  - Time span: `to = now`, `from = min(tip dates)` clamped to `[now − 366 days, now − 1 day]`. Bucket of a date = `((date − from) * buckets / (to − from))` clamped to `0..buckets`.
  - Default chain: `first_parent_chain(repo, table, default_tip, stop)` → `HandleSet` (empty when no default branch).
  - Per row: `Walk::from_tips([tip], Order::Date, table)` → `next_chunk` until the walk ends, `stop` fires, or a chunk's oldest commit is older than `from − 36 h` (skewed dates: `Shape::SkewedDates` has a commit a day ahead of its children). Per commit in the span: the default row counts every commit; any other row skips commits on the default chain (so a merged branch keeps the bars it had before its merge, and a fresh branch shows only its own work). Cap `MAX_COMMITS_PER_ROW` (50,000) → `complete = false` on that row and on `MapData`. If `Chunk`'s rows carry no committer date (check the MAP line; one scout summary if unclear), hydrate through `read_commit` — say so in the report.
  - `merged_into`: `Some((default_row, bucket_of(tip_date)))` when the row's tip is in the set of commits the default row's walk visited (so only "merged into the default branch" exists in M2 — say so in the module doc; the elbow sits at the tip's own bucket, not the merge commit's).
  - Marks: `MapMark::Tag(name)` at the bucket of each tag whose peeled commit the row counted; `MapMark::Radar(severity)` from `classify_commit` over each counted commit (`CommitFacts` from the walked commit and the default-chain set; `RepoFacts` from `req.default_branch`, `req.remotes`, `req.previous_default_tips`; `req.policy`). At most one radar mark per bucket per row, `Warning` wins. Everything else in `Finding` is dropped: the map shows a glyph, the graph explains.
  - Progress callback `(done, total)` once per finished row.
- `crates/graft/src/jobs/map.rs` (new; `jobs::map`, same shape as `jobs::detail`): `Task::Map`, `Product::Map(Result<Arc<MapData>, String>)`, `Progress::Map { done: u32, total: u32 }`, `Command::BuildMap(JobId, MapRequest)`. Runs `build_map` on `RepoStore::shared().handle()` with the job's stop flag; one `Arc<MapData>` back.
- `crates/graft/src/update/map.rs` (new): `AppState::map: MapState`, `Default` hidden.
  - `M` on the graph screen (not while the search prompt is open, not while the detail pane has focus off `Graph` — those keys stay theirs): `shown = true`, `data = Pending`, emits `Command::BuildMap` with `buckets = max(10, columns − 24)`, `policy = state.radar.policy.clone()`, `previous_default_tips` from `state.radar.context` when `Ready` (else empty — no backward marks until the context lands; a comment says so), `remotes`/`default_branch` from what `AppState` already holds (`RefBadges`/`RepoStore::branches` via the same path `view::radar::repo_facts` uses — cite it). `M` again or `Esc`: `shown = false`, `selected = 0`, `Command::Cancel(id)` if pending, `data = Idle` (discarded on toggle-off, per DESIGN).
  - `Product::Map`: stale ids dropped; `Ok` → `Ready`; `Err` → `Failed(line)`. `Progress::Map` → `progress = Some((done, total))`.
  - `↑`/`↓`/`j`/`k` move `selected` over the rows on hand; `Enter` on a row: `shown = false`, `data = Idle`, then `History::seek(tip)` (the existing `Find` request path, `MAP.md` line 25), returning its command. A resize while shown re-requests with the new width (cancel + `BuildMap`).
  - Add `Command::BuildMap`/`Cancel` dispatch in the loop exactly where `jobs::detail`'s commands are dispatched.
- Tests: see Acceptance. MAP.md lines (`graft`: `map`, `jobs::map`, `update::map`, `AppState::map`, `Task`/`Product`/`Progress`/`Command` lines).

Out: any drawing or `MapView` (`map-view-screen`), the pixel-tier density curve (deferred, Proposed changes), remote rows and "collapses into its upstream" (M2 shows local branches only), merges into a non-default branch, the Overview strip's hide/restore (`overview-strip` simply doesn't draw while `map.shown`).

## Signatures

```rust
// crates/graft/src/map.rs
pub const MAX_ROWS: usize = 200;
pub const MAX_COMMITS_PER_ROW: usize = 50_000;
#[derive(Clone, Debug)] pub struct MapRequest { pub buckets: usize, pub clock: Clock, pub policy: Policy, pub previous_default_tips: Vec<Oid> }
pub enum MapMark { Tag(Vec<u8>), Radar(Severity) }
pub struct MapBranch { pub name: RefName, pub identity: BranchIdentity, pub tip: Oid, pub tip_date: i64, pub counts: Vec<u32>, pub is_current: bool, pub is_default: bool,
    pub merged_into: Option<(usize, usize)> /* (row, bucket) */, pub marks: Vec<(usize, MapMark)>, pub complete: bool }
pub struct MapData { pub branches: Vec<MapBranch>, pub more: usize, pub from: i64, pub to: i64, pub buckets: usize, pub complete: bool }
impl MapData { pub fn bucket_of(&self, date: i64) -> usize; pub fn max_count(&self) -> u32; }
pub enum MapError { Store(StoreError), Walk(WalkError), Read(String), Stopped }   // Display: one line
pub fn build_map(store: &RepoStore, req: &MapRequest, progress: &mut dyn FnMut(u32, u32), stop: &dyn Stop) -> Result<MapData, MapError>;

// crates/graft/src/jobs/map.rs
// Task::Map; Product::Map(Result<Arc<MapData>, String>); Progress::Map { done: u32, total: u32 }; Command::BuildMap(JobId, MapRequest)

// crates/graft/src/update/map.rs
pub struct MapState { pub shown: bool, pub data: Loading<Arc<MapData>>, pub progress: Option<(u32, u32)>, pub selected: usize }
impl MapState { pub fn open(&mut self, req: MapRequest, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>; pub fn close(&mut self) -> Vec<Command>; pub fn selected_tip(&self) -> Option<Oid>; }
// AppState::map: MapState
```

`Policy` must be `Clone` for `MapRequest`; if `mistake-radar-detection` left it without the derive, add it and say so (a derive, not a signature change).

## Acceptance

- [ ] `build_map` on `Shape::FeatureBranches` (fixture clock `CORPUS_NOW`, `buckets = 40`): rows `main` (current, default, first), `feature/one`, `feature/two`, `feature/three` by tip date; `feature/one` and `feature/two` have `merged_into == Some((0, b))` with `b == bucket_of(tip_date)`; `feature/three` has `None`; every `counts.len() == 40`; `main`'s counts sum to its full commit count in the span; `feature/one`'s sum equals its own commits (2), none of main's.
- [ ] `Shape::Tags`: a `MapMark::Tag(b"v1.0")` on `main`'s row at the bucket of that commit's date.
- [ ] `Shape::FoxtrotMerge`: one `MapMark::Radar(Warning)` on the row holding the merge, at its bucket (default policy).
- [ ] `Shape::UnbornBranch`: rows for `main`/`other`, none current, no panic; an empty repo → `branches.is_empty()`, `more == 0`.
- [ ] A `Stop` that fires after the first chunk → `Err(MapError::Stopped)`, the job's product is an `Err` line, and a stale product changes nothing.
- [ ] `update`: `M` → `shown`, one `BuildMap` whose `buckets == columns − 24` (min 10); `M`/`Esc` → hidden + `Cancel`; `Product::Map(Ok)` → `Ready`; `Enter` on row 1 → hidden and a `Find` request for that tip's commit; resize while shown → `Cancel` + new `BuildMap`.
- [ ] Purity guard passes for `update/map.rs` (no clock: `MapRequest::clock` is copied from `state.harness.clock`).
- [ ] MAP.md updated (`graft`).

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

One crate: `map.rs` ~230 lines, `jobs/map.rs` ~50, `update/map.rs` ~110, tests ~200. ≤ 26 tool calls. Named seam: commit `map.rs` + its fixture tests first; if the job + update wiring then passes 70 % of budget before the resize/`Enter` arms are done, stop and report `partial` with `update/map.rs`'s remaining arms as the remainder.
