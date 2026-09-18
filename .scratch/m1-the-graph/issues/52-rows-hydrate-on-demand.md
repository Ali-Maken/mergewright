# m1-the-graph/52 — The layout lays out links: `RowShape`, deferred merge-lane identity, `Row::assemble`

Status: done
Build with: **Opus** (high) — the layout's core seam (links in, shapes out, one assembly step) with a deferred identity threaded through the cursor; judged by the existing snapshots staying byte-identical, so not Fable.
Needs: `Link`, `Parents`, `CommitHandle` (core); `CommitSource::{links, handle_of, id_of}`, `Walk::next_links` (commit-handles-and-table); `EmittedOrder` on handles, `Lane.waiting_for: CommitHandle` (order-and-inclusion-on-handles).
Blocked by: 49, 50, 51, 51b, 51c, 51d

**Amended 2026-09-14 (planner-fable, spec-only pass on the merge-lane identity gap).** The ticket was split: this one is the "layout lays out links" half and keeps every chunk materialised, so behaviour is unchanged and every snapshot stays green; `rows-pass-through` (52b) adds the pass-through, the rebuild that hydrates, the shape fingerprint and the measurements. The old handoff note is kept at the end for the record.

## Decision: merge-lane identity is deferred, not read during layout

`LayoutCursor::open_merge_lane` names a merge lane from `recover_branch_name(merge.subject())`, which only a commit read has. Chosen: **option (b)**. `Lane.identity` and `RowShape.lanes` hold a `LaneIdentity` that is either `Known(BranchIdentity)` or `Merge { merge, parent }` (the two handles that name it). `Row::assemble` resolves `Merge` through a `MergeNames` map gathered once per materialised batch: merges already in the batch cost nothing, merges whose lane opened above the batch cost one read each — bounded by lanes open at the batch's first row, never by history. `Row.lanes` stays `Option<BranchIdentity>`, so snapshots, `Fold`, `Ancestry` and raster do not change. The user sees the same colours; nothing in this decision is visible.

Why not (a) read merges during pass-through: git/git is ~25 % merges, 1M would read ~250k objects and their pack pages on every `G` — that is ADR-0026 item 4's "no commit object is read" given back. (c) subjects from the commit-graph: it has none.

**Guard:** the spec assumes a lane's identity is paint-only — carried into `Row.lanes`, never compared for a layout decision. If, reading `layout.rs`, `Lane.identity` is compared for column reuse, seeding or `known_identities` matching, **stop and hand off**: deferring it would change layout, and that needs a different design.

## Read first
- This ticket's "Decision" and "Signatures" (above/below); ADR-0026 "Decision" item 4, "What it costs"
- docs/design/03-graph-layout-algorithm.md §3.6 (streaming, row cache, checkpoints) — for what `StreamingLayout` does today, not to change it here
- `crates/graft-graph/MAP.md` (`lane`, `row`, `merge_name`, `layout`, `layout::source`, `layout::stream`, `inclusion`), `crates/graft/MAP.md` (`history::source`)
- Ticket 49's amended Signatures (`Walk::next_links`, `LinkChunk`)

## Scope
In:
- `CommitSource::pull` yields `Link`s (handles, parents as the view shows them); `read` takes handles and returns whole `Commit`s in the same order. `Included` trims parents in `pull` and `links`; `read` passes through untouched (the shape supplies the trimmed parents at assembly).
- `HandleLookup` gains `id_of` (the blanket impl forwards to `CommitSource::id_of`). `LayoutCursor::new` resolves `known_branches` to handles so `extend` matches known identities by handle.
- `LaneIdentity` in `lane`; `Lane.identity: LaneIdentity`. `open_merge_lane` stores `Merge { merge, parent }` and reads nothing. Every other identity the cursor computes today stays `Known` (`of_name`, `of_short_sha(id_of(..))`).
- `LayoutCursor::extend` takes links and returns `RowShape`s (no `Commit`). `RowShape.lanes: Vec<Option<LaneIdentity>>`.
- `MergeNames::gather` + `Row::assemble` in `row`: `commit.parents := shape.outgoing`'s parent ids in order (the view's trimmed parents, exactly what rows hold today); `lanes` resolved to `Option<BranchIdentity>`. The identity rule is byte-for-byte today's: `recover_branch_name(subject).map(of_name)` else `of_short_sha(id_of(parent))`.
- `StreamingLayout` materialises **every** chunk as today, but through the new path: `extend` → shapes; `read` the chunk's handles; `MergeNames::gather` (chunk commits on hand, the rest read); `assemble` each. Same for the rebuild-from-checkpoint path. `Work` gains `hydrated` (commits read for rows) and `merge_reads` (commits read only to name a lane).
- `layout()` (slice form) keeps its signature: it has every commit on hand, so `gather` reads nothing.
- `graft::history::WalkSource` implements `pull` (`next_links`, the amendment in ticket 49) and `read` (`read_commit` per handle; a batched read is 52b's if the numbers want it). `serve`/`Views` unchanged.
- MAP.md for `graft-graph` (`lane`, `row`, `layout`, `layout::source`, `layout::stream`, `inclusion`) and `graft` (`history::source`).

Out: pass-through, viewport-bound materialisation, fingerprint over shapes, `Resident.passed_through`, `Work.links_read`, measurements, docs/design/03-graph-layout-algorithm.md §3.6 — all 52b. The walk's internals; the table; the strip cache; search.

## Signatures
```rust
// graft-graph::lane
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneIdentity {
    Known(BranchIdentity),
    Merge { merge: CommitHandle, parent: CommitHandle },   // named by the merge's subject; resolved at assembly, never during layout
}
impl LaneIdentity {
    pub fn known(self) -> Option<BranchIdentity>;   // `Known` only
}
pub struct Lane { pub index: usize, pub identity: LaneIdentity, pub waiting_for: CommitHandle, pub reserved: bool }

// graft-graph::layout::source
pub struct Pull { pub links: Vec<Link>, pub done: bool, pub stopped: bool }
pub trait CommitSource {
    fn pull(&mut self, stop: &dyn Fn() -> bool) -> Result<Pull, /* existing error type */>;
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, /* existing */>;
    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, /* existing */>;   // whole commits, same order as asked
    fn handle_of(&self, id: &Oid) -> Option<CommitHandle>;
    fn id_of(&self, handle: CommitHandle) -> Oid;
}

// graft-graph::layout
pub trait HandleLookup {
    fn handle_of(&self, id: Oid) -> CommitHandle;
    fn id_of(&self, handle: CommitHandle) -> Oid;   // new; blanket impl forwards to CommitSource::id_of
}
impl LayoutCursor {
    pub fn new(seeds: &Seeds, handles: &impl HandleLookup) -> Self;   // unchanged; known_branches resolved to handles here
    pub fn extend(&mut self, links: &[Link], handles: &impl HandleLookup, order: &mut EmittedOrder) -> Result<Vec<RowShape>, LayoutError>;
}

// graft-graph::row
pub struct RowShape {
    pub commit: CommitHandle,
    pub lane: usize,
    pub incoming: Vec<usize>,
    pub outgoing: Vec<(usize, CommitHandle)>,
    pub lanes: Vec<Option<LaneIdentity>>,
}
pub struct MergeNames { /* HashMap<CommitHandle, BranchIdentity>; reads: usize */ }
impl MergeNames {
    /// Names every distinct `Merge { merge, .. }` in `shapes[..].lanes`. `commits[i]` is `shapes[i].commit`'s
    /// commit (same length, same order); a merge on hand there costs nothing, the rest go to `read` in one batch.
    pub fn gather<E>(
        shapes: &[RowShape],
        commits: &[Commit],
        read: impl FnMut(&[CommitHandle]) -> Result<Vec<Commit>, E>,
        id_of: impl Fn(CommitHandle) -> Oid,
    ) -> Result<Self, E>;
    pub fn reads(&self) -> usize;   // commits `gather` had to read (not on hand)
    pub fn resolve(&self, identity: LaneIdentity) -> Result<BranchIdentity, AssembleError>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssembleError { MergeNotGathered { merge: CommitHandle } }
impl Row {
    /// `commit.parents` := the shape's outgoing parents' ids, in order; `lanes` resolved through `names`.
    pub fn assemble(shape: RowShape, commit: Commit, names: &MergeNames, id_of: &dyn Fn(CommitHandle) -> Oid) -> Result<Row, AssembleError>;
}

// graft-graph::layout::stream
pub enum StreamError { /* existing */ Assemble(AssembleError) }
pub struct Work { /* existing */ pub hydrated: usize, pub merge_reads: usize }
// `advance`, `rows`, `from_order`, `set_viewport`: signatures unchanged; every chunk still materialised.

// graft-graph::layout
pub enum LayoutError { /* existing */ Assemble(AssembleError) }   // `layout()` signature unchanged
```
`WalkSource::pull` calls `next_links` with the `limit` it passes to `next_chunk` today and maps `LinkChunk` to `Pull` field for field.

## Acceptance
- [x] Every existing `graft-graph` test and every corpus snapshot green, **unchanged** (`Row`s field by field, `lanes` included): the proof that a deferred identity resolves to today's identity.
- [x] `layout()` over a slice ≡ `StreamingLayout` rows over the same order, field by field, on every corpus shape and the 5k synthetic pair (the test that exists today, on the new path).
- [x] New test, `layout::stream::tests`: a merge whose lane stays open across a chunk boundary (`chunk_rows` small). The chunk holding the merge assembles with `MergeNames::reads() == 0`; the next chunk assembles with `reads() == 1`; both chunks' `lanes` equal `layout()`'s. `Work.merge_reads` counts it.
- [x] New test, `row`: `assemble` with a `Merge` identity missing from `names` → `Err(MergeNotGathered)`; nothing panics.
- [ ] `graft::history` tests green (done); `frames::` rows-on-hand check still passes in release — **not run**: `frames` is heavy-tier (`#[cfg(feature = "heavy-tests")]`) and builders run the fast tier only; left for the M1 review's release run.
- [x] The guard above checked and stated in the report: `Lane.identity` is never compared for a layout decision (say where you looked).
- [x] MAP.md for both crates updated; no `unwrap_or_default`/`let _` on a read result.

## Notes (builder, 2026-09-14)
- Guard checked: in `layout.rs`, `Lane.identity` is only copied into `RowShape.lanes` (`assign_row`, `own`, `set_column`); column reuse, seeding and `known_identities` (looked up by commit id) never compare it.
- Deviations: no `LayoutError` exists, so `extend` stays infallible; `pull` keeps its `limit`; `known_identities` stays keyed by `Oid` (a known tip may be absent from the table); `MergeNames` is keyed by (merge, parent) so an octopus merge's short-sha fallback stays per parent.
- `tests/it/streaming.rs` read-count assertions changed by design: every materialised row is now read by handle, so `reads == hydrated + merge_reads` (was `reads == commits_reread`). Row equality assertions unchanged.
- No compaction during the build itself; the session was continued once from a summary.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`.

## Size
One context, under ~80k tokens. Seam if needed: (a) `lane`/`row`/`layout` (`LaneIdentity`, `RowShape`, `MergeNames`, `assemble`, `extend` on links, `layout()`); (b) `layout::source`/`inclusion`/`layout::stream`/`WalkSource` (the trait change and the materialise path). Stop at the seam with a handoff note rather than compacting.

## Handoff (2026-09-14, builder-opus, kept for the record): the gap that led to the decision above
`LayoutCursor::open_merge_lane` (layout.rs ~l.393) sets a merge lane's identity from `recover_branch_name(commit.subject())`, falling back to `of_short_sha(parent)`; the subject needs a commit read, `Link` has handles only, and the identity is copied into every later row's `lanes` while the lane stays open. Other identities work through `id_of`. Read-first already done by that builder: source.rs, stream.rs, cache.rs, checkpoint.rs, layout.rs, row.rs. Callers of `extend`/`next`/`read`: `inclusion.rs`, `layout/tests.rs`, `layout/stream/tests.rs`, `graft::history::source`. Gotcha carried to 52b: `RowCache` fingerprints `&[Row]` today; pass-through needs one fingerprint kind over shapes.
