# graft-graph — map
One line per public item. Update when a public item changes.

## lib.rs
- `ancestry`, `fold`, `identity`, `inclusion`, `lane`, `layout`, `merge_name`, `raster`, `row`, `seeds` (mods) — re-exported at the root (`Ancestry`, `Kin`, `Fold`, `Follow`, `Included`, `Inclusion`, `Invalidated` too).

## identity
- `BranchIdentity` (struct) — stable FNV-1a hash of a branch name (or a commit id); the source of lane colour.
- `BranchIdentity::of_name` (fn) — identity of a named branch.
- `BranchIdentity::of_short_sha` (fn) — fallback identity from a commit id when no name survives.
- `BranchIdentity::slot` (fn) — which of N palette entries this identity draws in.

## lane
- `Lane` (struct) — one open column: `index`, `identity`, `waiting_for`, `reserved`.

## row
- `Row` (struct) — one laid-out commit.
- `Row::commit` (field) — the commit drawn.
- `Row::lane` (field) — the commit's own column.
- `Row::incoming` (field) — lanes closing into the node, ascending.
- `Row::outgoing` (field) — one `(column, parent)` per parent, first parent first.
- `Row::lanes` (field) — identity drawn in each column on this row; `None` where no edge; trailing `None`s trimmed.
- `Row::marker` (field) — Mistake Radar kind; always `None` from `graph`.

## seeds
- `BranchSeed` (struct) — a branch tip and its identity.
- `Seeds` (struct) — `default_branch`, `integration_branches`, `known_branches` a cursor starts from.

## merge_name
- `recover_branch_name` (fn) — branch name from a merge subject, git's conventional forms.

## ancestry
- `Kin` (enum) — `Selected`, `Ancestor`, `Descendant`, `Unrelated`: how a row stands to the selection.
- `Ancestry` (struct) — the selection's thread over one window of rows; a paint-time lens, one pass, no layout or walk call (DESIGN.md §5.2).
- `Ancestry::of` (fn) — the thread of a commit through a row window; `None` if it is not there.
- `Ancestry::kin` (fn) — a window row's relation to the selection.
- `Ancestry::lit` (fn) — real columns on a window row carrying the thread.
- `Ancestry::columns` (fn) — every lit column: the protected set handed to `Fold::plan`.

## fold
- `Fold` (struct) — which real columns a window of rows draws and which fold behind `+N`; a lens over laid-out rows, never a re-layout (DESIGN.md §3.5).
- `Fold::plan` (fn) — decide the fold for a window: at most `max_lanes` drawn (marker included); folds least-recently-touched first, never lane 0, a pinned column, or a column holding a node in the window; a column open in no row is dropped, not counted.
- `Fold::is_empty` (fn) — nothing folded and nothing dropped: the window draws as laid out.
- `Fold::folded` (fn) — real columns behind the marker, ascending.
- `Fold::kept` (fn) — real columns drawn, in drawn order.
- `Fold::drawn` (fn) — where a real column draws, `None` if folded or dropped.
- `Fold::real` (fn) — the real column at a drawn position, `None` at the marker (dash parity keys on this).
- `Fold::marker` (fn) — the drawn column the `+N` marker takes, if anything folded.
- `Fold::drawn_columns` (fn) — columns drawn, marker included.
- `Fold::view` (fn) — a row remapped to drawn columns; edges into folded lanes go to the marker, whose cell carries the joining lane's identity.

## inclusion
- `Follow` (enum) — `AllParents` or `FirstParent`; fixed per layout (it changes the seeds).
- `Inclusion` (struct) — `hidden_stashes`, `folded` merges; can change while a layout lives.
- `seeds_for` (fn) — the seeds a view lays out from (first-parent drops integration lanes).
- `apply` (fn) — slice form: the commits a view shows, parents trimmed, and its seeds.
- `Included` (struct) — `CommitSource` wrapper letting through only what a view shows; remembers walked ids so a change re-reads, never re-walks.
- `Included::new` / `seeds` / `follow` / `inclusion` / `inner` (fn) — build and query.
- `Included::reinclude` (fn) — switch inclusion; the first row whose drawing can change, or `None`.
- `reinclude` (fn) — switch a `StreamingLayout<Included<_>>`'s inclusion and truncate from the first changed row; returns `Invalidated`.

## layout
- `layout` (fn) — lay out a whole commit slice in one call.
- `LayoutCursor` (struct) — resumable lane state; a clone is a checkpoint.
- `LayoutCursor::new` (fn) — cursor seeded with lane 0 and integration lanes.
- `LayoutCursor::extend` (fn) — lay out the next batch, recording into an `EmittedOrder`.
- `LayoutCursor::rows_emitted` (fn) — rows laid out so far.
- `LayoutCursor::approximate_bytes` (fn) — checkpoint cost estimate.
- `LayoutCursor::active_lanes` (fn) — current occupant of each column.

## layout::order
- `EmittedOrder` (struct) — commit per row plus reverse lookup; `new`, `from_ids`, `note`, `row_of`, `id_at`, `ids`, `len`, `is_empty`, `approximate_bytes`.
- `RepeatedCommit` (struct) — a commit the walk named twice: `id`, `first`, `again`.

## layout::source
- `CommitSource` (trait) — a walk the layout pulls from and re-reads by id.
- `Pull` (struct) — one pull: `commits`, `done`, `stopped`.

## layout::stream
- `StreamConfig` (struct) — `chunk_rows`, `checkpoint_every`, `max_checkpoints`, `resident_chunks`, `buffer_rows`.
- `StreamError` (enum) — why streaming could not go on.
- `Advance` (struct) — one advance: `rows_emitted`, `done`, `stopped`.
- `Anchor` (struct) — selected `commit` and its viewport `offset`.
- `Resident` (struct) — held rows, chunks, checkpoints and their byte estimates.
- `Work` (struct) — pulls, re-reads, replayed rows, rebuilt chunks.
- `StreamingLayout` (struct) — chunked layout with a bounded resident set.
- `StreamingLayout::new` / `from_order` (fn) — stream a walk / lay out a known order.
- `StreamingLayout::advance` (fn) — lay out up to a budget of rows past the frontier.
- `StreamingLayout::set_viewport` / `viewport` (fn) — rows on screen.
- `StreamingLayout::rows` (fn) — rows in a range, rebuilding evicted chunks.
- `StreamingLayout::is_resident`, `rows_emitted`, `expected_rows`, `is_done`, `row_of`, `id_at` (fn) — queries.
- `StreamingLayout::reanchor` (fn) — viewport that keeps an anchor's offset.
- `StreamingLayout::resident`, `checkpoint_rows`, `work`, `source`, `config` (fn) — introspection.
- `StreamingLayout::source_mut` (fn) — the source, mutably.
- `StreamingLayout::truncate` (fn) — forget rows from a row on (chunks cut, later checkpoints dropped, order truncated), frontier replayed to it; returns `Invalidated`.
- `Invalidated` (struct) — `from_row`, `chunks`, `checkpoints` a truncate threw away.

## raster
- `rasterise` (fn) — a `Strip` in, a premultiplied `core::RasterFrame` out; pure, no cache; background never painted (ADR-0003).
- `Strip` (struct) — `rows`, `above` (row before the strip), `lane_columns`, `marker`, `cell_width`/`cell_height`, `palette`, `muted`.
- `RasterRow` (struct) — a row in drawn columns plus `ring`, `emphasis`, `thread` (drawn columns on the selection's thread).
- `RasterRow::plain` (fn) — `Ring::of` the row, normal emphasis, no thread.
- `Ring` (enum) — node style by shape: `Commit`, `Merge`, `Root`, `Head`, `Stash`, `Uncommitted`; `Ring::of` (fn) from parent count.
- `Emphasis` (enum) — `Normal`, `Dimmed` (faded), `Highlighted` (heavier stroke).
- `Marker` (struct) — the `+N` column: `column`, `folded`.
- `RasterError` (enum) — `EmptyPalette`, `TooLarge`.
- `CELLS_PER_LANE` (const) — cells per lane column (2), matching the Unicode tier.

## raster::cache
- `StripCache` (struct) — bounded LRU of rasterised strips keyed on (row range, layout hash, theme id, cell size) (DESIGN.md §3.8).
- `StripCache::new` / `capacity_for` (fn) — make one; strips a viewport plus buffer needs at any offset.
- `StripCache::strips_for` (fn) — aligned strip ranges covering a viewport.
- `StripCache::get` (fn) — a strip from the cache, rasterised on a miss; refuses a misaligned strip. An `Uncommitted` ring row does not count toward the strip height.
- `StripCache::contains` (fn) — is a strip already rasterised; touches neither counters nor recency (idle prefetch, m1-the-graph/30h).
- `StripCache::flush` (fn) — drop everything on theme, cell-size, re-detection or resize.
- `StripCache::len`, `is_empty`, `resident_bytes`, `stats`, `strip_rows` (fn) — introspection.
- `STRIP_ROWS` (const) — 32, chosen by measurement.
- `Flush` (enum) — `Theme`, `CellSize`, `Redetection`, `Resize`.
- `CacheError` (enum) — `Misaligned`, `Raster`.
- `CacheStats` (struct) — `hits`, `rasterised`, `evicted`, `flushes`.
- `layout_hash` (fn) — hash of what the rasteriser draws from a strip, colours and cell size aside.
