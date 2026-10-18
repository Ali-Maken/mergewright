# mergewright-graph — map
The pure layout engine and the pixel/Unicode/ASCII renderers. May depend on `core` only (ARCHITECTURE.md table).
One line per public item. History lives in `docs/log/` and the ADRs.

## lib.rs
- `ancestry`, `fold`, `identity`, `inclusion`, `lane`, `layout`, `merge_name`, `raster`, `role`, `row`, `seeds` (mod) — the public modules; their main types are re-exported at the root.

## ancestry
- `Kin` (enum) — `Selected`, `Ancestor`, `Descendant`, `Unrelated`: how a row stands to the selection.
- `Ancestry` (struct) — the selection's thread over one window of rows; a paint-time lens, no layout or walk call.
- `Ancestry::of` (fn) — `of(rows, selected: Oid) -> Option<Ancestry>`; the thread through a window, `None` if absent.
- `Ancestry::kin` (fn) — `kin(row) -> Kin`; a window row's relation to the selection.
- `Ancestry::lit` (fn) — `lit(row) -> &BTreeSet<usize>`; real columns on a row carrying the thread.
- `Ancestry::columns` (fn) — every lit column; the preferred set a fold weighs above ordinary columns.

## fold
- `Keep` (struct) — `selected`, `reserved`, `preferred` columns: budget priorities among node columns, then connectors, for `Fold::plan`, not vetoes.
- `Fold` (struct) — which real columns a window draws and which fold behind the rail; a lens, never a re-layout.
- `Fold::plan` (fn) — `plan(max_lanes, &Keep, rows) -> Fold`; drops pass-throughs, then admits lane 0, node columns, connectors by priority up to the budget.
- `Fold::is_empty` (fn) — nothing folded or dropped: the window draws as laid out.
- `Fold::folded` / `kept` (fn) — real columns behind the marker; real columns drawn, in drawn order.
- `Fold::drawn` / `real` (fn) — map a real column to its drawn position and back; `None` when folded or at the marker.
- `Fold::marker` (fn) — the drawn column the rail takes, if anything folded.
- `Fold::drawn_columns` / `folded_count` (fn) — columns drawn (marker included); lanes behind the rail.
- `Fold::open_columns` (fn) — columns open in the window, dropped ones included: the M of `lanes drawn: N of M`.
- `Fold::view` (fn) — `view(&Row) -> DrawnRow`; the row in drawn columns, edges into folded lanes listed apart.
- `Toward` (enum) — `Below` / `Above`: which way a folded edge leaves the node.
- `FoldedEdge` (struct) — `identity`, `toward`: an edge a fold took out of a row, drawn as a stub.
- `DrawnRow` (struct) — `row: Cow<Row>`, `folded: Vec<FoldedEdge>`; a row as one fold draws it.

## identity
- `BranchIdentity` (struct) — stable hash of a branch name (or commit id); the source of lane colour.
- `BranchIdentity::of_name` (fn) — `of_name(&[u8]) -> Self`; identity of a named branch.
- `BranchIdentity::of_short_sha` (fn) — `of_short_sha(Oid) -> Self`; fallback when no name survives.
- `BranchIdentity::slot` (fn) — `slot(slots) -> usize`; which palette entry this identity draws in.

## inclusion
- `Follow` (enum) — `AllParents` or `FirstParent`; fixed per layout because it changes the seeds.
- `Inclusion` (struct) — hidden stashes, stash entries and `fold: Option<Folding>`; may change while a layout lives.
- `Inclusion::folded_by_default` (fn) — `folded_by_default(keep: Vec<Oid>, OpenedMerges)`; merged branches hide behind their merge.
- `Inclusion::opened` / `opened_mut` / `keep` / `keep_mut` (fn) — the fold's opened merges and extra keep tips; `None` when not folding.
- `Inclusion::with_stashes` / `stashes` (fn) — set / read every stash entry's own commit.
- `Folding` (struct) — fold by default's state: keep tips beyond the seeds, and the opened merges.
- `OpenedMerges` (struct) — merges the user opened; `open` / `close` (true if changed), `contains`, `iter`.
- `FoldWalk` (struct) — the fold rule as a streaming predicate over handles in walk order; ~5.75 bytes/commit dense.
- `FoldWalk::new` / `mark` (fn) — `new(keep, opened, forced: &[CommitHandle])`; `mark(handle, Mark)` for one known later.
- `FoldWalk::admit` (fn) — `admit(commit, parents) -> Admit`; decide one commit when the walk reaches it.
- `FoldWalk::trim` (fn) — `trim(Link) -> Link`; parents a shown commit draws edges to.
- `FoldWalk::take_late` / `take_rejoined` / `rejoin` / `rejoins` (fn) — late hidden commits to force; merge edges to draw again.
- `FoldWalk::folds` / `behind` / `inside` / `approximate_bytes` (fn) — does a merge fold; the shown merge a hidden commit is behind; the opened merge a shown commit is inside.
- `Admit` (enum) — `Shown { inside }` / `Hidden { behind }`.
- `Mark` (enum) — `Keep` / `Opened` / `Forced`.
- `seeds_for` (fn) — `seeds_for(Follow, Seeds) -> Seeds`; the seeds a view lays out from.
- `apply` (fn) — slice form: the commits a view shows, parents trimmed, and its seeds.
- `Included` (struct) — `Included<S: CommitSource>`: a source wrapper passing only what a view shows; remembers walked handles.
- `Included::new` / `seeds` / `follow` / `inclusion` / `inner` (fn) — build and query the wrapper.
- `Included::walked` / `shown` / `fold` (fn) — walked and currently shown handles, as `HandleSet`s; the fold's walk, if folding.
- `Included::reinclude` (fn) — `reinclude(Inclusion) -> Result<Option<usize>, _>`; switch inclusion, first changed row.
- `Included::repair` (fn) — `repair() -> Result<Option<usize>, _>`; force late commits shown, redraw rejoined edges; first changed row.
- `Included::whereabouts` (fn) — `whereabouts(Oid) -> Whereabouts`; where a commit is in this view, from what is walked, without advancing.
- `Whereabouts` (enum) — `Shown` / `Folded { behind }` (the shown merge, through hidden ones) / `Filtered` (a view he chose hides it) / `NotWalked`.
- `reinclude` (fn) — switch a `StreamingLayout<Included<_>>`'s inclusion and truncate from the first changed row; `Invalidated`.
- `repair` (fn) — run `Included::repair` on a `StreamingLayout` and truncate; `Option<Invalidated>`.
- `advance_folded` (fn) — `advance` then `repair` until neither has work; what a folding view advances with.

## lane
- `LaneIdentity` (enum) — `Known(BranchIdentity)` or `Merge { merge, parent }`, a merge lane's name resolved at assembly.
- `LaneIdentity::known` (fn) — `known() -> Option<BranchIdentity>`; the identity if already known.
- `Lane` (struct) — one open column: `index`, `identity`, `waiting_for` (`CommitHandle`), `reserved`.

## layout
- `order`, `source`, `stream` (mod) — emitted order, the commit source trait, and the streaming layout.
- `HandleLookup` (trait) — `handle_of(Oid) -> CommitHandle`, `id_of(CommitHandle) -> Oid`; blanket for every `CommitSource`.
- `layout` (fn) — `layout(&[Commit], Seeds) -> Vec<Row>`; lay out a whole commit slice in one call.
- `LayoutCursor` (struct) — resumable lane state; a clone is a checkpoint.
- `LayoutCursor::new` (fn) — `new(Seeds, &impl HandleLookup) -> Self`; seeded with lane 0 and integration lanes.
- `LayoutCursor::extend` (fn) — `extend(&[Link], handles, &mut EmittedOrder) -> Vec<RowShape>`; next batch, no commit reads.
- `LayoutCursor::rows_emitted` / `approximate_bytes` / `active_lanes` (fn) — rows so far, checkpoint cost, column occupants.

## layout::order
- `EmittedOrder` (struct) — row → `CommitHandle` and a dense reverse lookup; `new`, `from_handles`, `note`, `len`, `is_empty`.
- `EmittedOrder::row_of` / `handle_at` / `handles` / `approximate_bytes` (fn) — lookups both ways, a row range, size estimate.
- `RepeatedCommit` (struct) — a commit the walk named twice: `commit`, `first`, `again`.

## layout::source
- `CommitSource` (trait) — a walk the layout pulls `links` from and hydrates by handle (`pull`, `read`, `links`, `handle_of`, `id_of`).
- `Pull` (struct) — one pull: `links`, `done`, `stopped`.

## layout::stream
- `StreamConfig` (struct) — `chunk_rows`, `checkpoint_every`, `max_checkpoints`, `resident_chunks`, `buffer_rows`.
- `StreamError` (enum) — why streaming could not go on: source error or a row that could not be assembled.
- `Invalidated` (struct) — `from_row`, `chunks`, `checkpoints` a truncate threw away.
- `Advance` (struct) — one advance: `rows_emitted`, `done`, `stopped`.
- `Anchor` (struct) — selected `commit` and its viewport `offset`.
- `Resident` (struct) — held rows, chunks, checkpoints and byte estimates; `passed_through` rows.
- `Work` (struct) — counters: pulls, re-reads, replayed rows, rebuilt chunks, `hydrated`, `merge_reads`, `links_read`.
- `StreamingLayout` (struct) — `StreamingLayout<S: CommitSource>`: chunked layout with a bounded resident set.
- `StreamingLayout::new` / `from_order` (fn) — stream a walk; or lay out a known `Vec<CommitHandle>` order.
- `StreamingLayout::advance` (fn) — lay out up to a row budget past the frontier; reads commits only near the viewport.
- `StreamingLayout::set_viewport` / `viewport` (fn) — the rows on screen.
- `StreamingLayout::rows` (fn) — `rows(Range) -> Result<Vec<Row>, StreamError>`; rebuilds chunks not held from a checkpoint.
- `StreamingLayout::is_resident` / `expected_rows` / `is_done` / `row_of` / `id_at` (fn) — queries.
- `StreamingLayout::reanchor` (fn) — `reanchor(&Anchor, height) -> Option<Range>`; viewport keeping the anchor's offset.
- `StreamingLayout::resident` / `checkpoint_rows` / `checkpoint_stride` / `work` / `config` (fn) — introspection.
- `StreamingLayout::source` / `source_mut` (fn) — the wrapped source.
- `StreamingLayout::truncate` (fn) — `truncate(row) -> Result<Invalidated, _>`; forget rows from `row` on.

## merge_name
- `recover_branch_name` (fn) — `recover_branch_name(&[u8]) -> Option<Vec<u8>>`; branch name from a merge subject; `PR <n>` from Azure DevOps' `Merged PR <n>: …`.

## raster
- `cache` (mod) — the strip cache.
- `CELLS_PER_LANE` (const) — cells per lane column (2), matching the Unicode tier.
- `OFF_THREAD_ALPHA` (const) — alpha of a column off the selection's thread.
- `FADED_ALPHA` (const) — alpha of a dimmed row or column with no thread in play.
- `Emphasis` (enum) — `Normal` or `Dimmed`; alpha only, never width.
- `Stub` (struct) — `identity`, `toward`: a folded edge as the rasteriser draws it.
- `Marker` (struct) — the rail column: `column`.
- `RasterError` (enum) — `EmptyPalette`, `TooLarge`, `Selection`.
- `Ring` (enum) — node style: `Commit`, `Merge`, `Root`, `Head`, `Stash`, `Uncommitted`; `Ring::of(&Row)` from git's parents (`Row::is_merge`: a folded merge is `Merge`), stash-blind.
- `Backdrop` (enum) — `Full` / `OffThread` / `Faded`: the one alpha a layout-only strip draws at; `alpha`, `lowest`.
- `PlainRow` (struct) — a row as the layout draws it: `row`, `ring`, `stubs`; `PlainRow::of`.
- `Lit` (struct) — one row's selection paint: `emphasis`, `thread`.
- `PlainStrip` (struct) — everything one strip is drawn from: rows, `above`, lane columns, marker, cell size, palette.
- `Selection` (struct) — `rows: &[Lit]` and the frame's `backdrop`.
- `Emphasised` (struct) — an emphasis image `frame` cropped to whole cells, placed at `first_column`, `first_row` (0 for `rasterise_emphasis`).
- `rasterise_backdrop` (fn) — `PlainStrip` → premultiplied `RasterFrame`, every element at one backdrop alpha.
- `rasterise_emphasis` (fn) — what draws above the backdrop, cropped; `None` when nothing does.
- `composite` (fn) — `composite(&RasterFrame, &Emphasised) -> RasterFrame`; premultiplied "over".
- `PieceKind` (enum) — `Run` (a column's straight edges), `Arc { row }` (one arc), `Node` (nodes, stubs, tails of the column's lane, plus the rail on the marker column).
- `Piece` (struct) — one share of a strip's emphasis: `column`, `kind`, content `key` (ADR-0053 amendment 1, 2026-10-10).
- `emphasis_pieces` (fn) — `(&PlainStrip, &Selection) -> Vec<Piece>`, bottom first: every Run then its Arcs, column by column, then every Node; no rasterising.
  Ownership: a straight edge is its column's Run; an arc its column's `Arc { row }`; a row's node, stubs, tail are its lane's Node.
  The rail is drawn in the row pass (`Pen::row` → `rail`, `raster/pen.rs:265`), so it is the marker column's Node.
  Key = hash(kind, cell size, backdrop alpha, each element's anchor-relative geometry, resolved RGBA and alpha). No strip key, no absolute row or column: equal keys draw equal pixels anywhere.
- `rasterise_piece` (fn) — one piece's pixels at `(a − b)/(1 − b)`, cropped to whole cells in both axes; `None` when it draws nothing.
- `RasterPool` (struct) — strip-sized pixel buffers lent to the rasteriser; `new`, `take`, `give`, `flush`, `allocated`.

## raster::cache
- `STRIP_ROWS` (const) — rows per cached strip (32).
- `ByteBudget` (struct) — `ByteBudget(usize)`: bytes the strip cache may hold across its layers.
- `Flush` (enum) — `Theme`, `CellSize`, `Redetection`, `Resize`: why the cache was emptied.
- `CacheError` (enum) — `Misaligned`, `Raster`.
- `CacheStats` (struct) — hits, rasterised, evicted, flushes, emphasis counters, `buffers_allocated`.
- `StripCache` (struct) — bounded LRUs of rasterised strips keyed on rows, layout hash, theme and cell size.
- `StripCache::with_budget` / `capacity_for` (fn) — build with a `ByteBudget` and strip floor; strips a viewport needs.
- `StripCache::keep_at_least` / `min_strips` / `budget` / `minimum_wins` (fn) — the floor, the budget, floor-over-budget flag.
- `StripCache::strip_rows` / `strips_for` (fn) — strip height; aligned strip ranges covering a viewport.
- `StripCache::backdrop` / `emphasis` / `composited` (fn) — a strip's backdrop, emphasis or composite, cached or rasterised.
- `StripCache::contains_backdrop` (fn) — whether a backdrop strip is resident; a look, not a use.
- `StripCache::backdrop_key` / `paint_key` (fn) — the `u64` upload keys the terminal side matches on.
- `StripCache::flush` (fn) — `flush(Flush)`; drop every layer.
- `StripCache::len` / `is_empty` / `resident_bytes` / `stats` (fn) — introspection.
- `plain_layout_hash` (fn) — `plain_layout_hash(&PlainStrip) -> u64`; hash of what the rasteriser draws, colours aside.
- `lit_hash` (fn) — `lit_hash(&Selection) -> u64`; hash of a selection's paint.

## role
- `LaneRole` (enum) — `This`, `Under`, `Trunk`, `Other`: what a lane's branch is to the focused one; the graph colours by it.
- `Route` (struct) — trunk, chain of bases (nearest first, trunk excluded), focus; `new(trunk, built_on, focus)`, `unread(trunk, focus)`.
- `Route::focus` / `trunk` / `chain` (fn) — the parts back.
- `Route::role` (fn) — `role(BranchIdentity) -> LaneRole`: focus → `This`, trunk → `Trunk`, chain → `Under`, else `Other`.
- `made_on` (fn) — `made_on(&Row) -> Option<BranchIdentity>`: `row.lanes[row.lane]`; `None` when unnamed or out of range.

## row
- `Row` (struct) — one laid-out commit: `commit` (every real parent, under every inclusion), `drawn_parents` (the view's trimmed parents; drawing only), `lane`, `incoming`, `outgoing`, `lanes`, `marker`.
- `Row::is_merge` (fn) — two or more of git's parents; stash-blind: `Ring::of` and mergewright-ui's `NodeKind::of` call it; the app's answer, which excludes stash entries, is `history::parents::is_merge`.
- `Row::assemble` (fn) — `assemble(shape, commit, &MergeNames, id_of) -> Result<Row, AssembleError>`; hydrate a shape.
- `RowShape` (struct) — a row before hydration, by handle; `Hash` for chunk fingerprints.
- `MergeNames` (struct) — merge-lane names for one batch; `gather`, `reads`, `resolve`.
- `AssembleError` (enum) — `MergeNotGathered { merge }`.

## seeds
- `BranchSeed` (struct) — a branch tip and its identity.
- `Seeds` (struct) — `default_branch`, `integration_branches`, `known_branches` a layout starts from.
