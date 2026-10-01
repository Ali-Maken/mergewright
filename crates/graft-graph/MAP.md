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
- `Lane` (struct) — one open column: `index`, `identity` (`LaneIdentity`), `waiting_for` (`CommitHandle`, ADR-0026), `reserved`.
- `LaneIdentity` (enum) — `Known(BranchIdentity)` or `Merge { merge, parent }`: a merge lane's name deferred to assembly (no commit read during layout); `known()`.

## row
- `Row` (struct) — one laid-out commit.
- `Row::commit` (field) — the commit drawn.
- `Row::lane` (field) — the commit's own column.
- `Row::incoming` (field) — lanes closing into the node, ascending; every one is a `Some` in `lanes` (a reserved lane never drawn above its tip closes with no edge, m1-the-graph/68).
- `Row::outgoing` (field) — one `(column, parent)` per parent, first parent first.
- `Row::lanes` (field) — identity drawn in each column on this row; `None` where no edge; trailing `None`s trimmed.
- `Row::marker` (field) — Mistake Radar kind; always `None` from `graph`.
- `Row::assemble` (fn) — `assemble(shape, commit, &names, id_of)`: a shape plus its whole commit → `Row` (parents from the shape, merge lanes resolved); `Err(AssembleError)` if a merge lane was not gathered.
- `RowShape` (struct) — a row before hydration: `commit` handle, `lane`, `incoming`, `outgoing: (column, CommitHandle)`, `lanes: Option<LaneIdentity>`; `Hash` (the chunk fingerprint hashes shapes).
- `MergeNames` (struct) — merge-lane names for one batch, keyed by (merge, parent); `gather(shapes, commits, read, id_of)` reads only merges not on hand, in one batch; `reads`, `resolve`.
- `AssembleError` (enum) — `MergeNotGathered { merge }`.

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
- `Ancestry::columns` (fn) — every lit column: the preferred set a fold weighs above ordinary columns, `Keep::preferred` (ADR-0025) — a priority, not a veto.

## fold
- `Fold` (struct) — which real columns a window of rows draws and which fold behind the rail; a lens over laid-out rows, never a re-layout (DESIGN.md §3.5).
- `Keep` (struct) — `selected` (the selected commit's own column), `reserved` (integration lanes), `preferred` (the selection's thread): budget priorities for `Fold::plan`, not vetoes (ADR-0025).
- `Fold::plan` (fn) — `plan(max_lanes, &keep, rows)`: decide the fold for a window. The cap is a budget: admits columns in priority order — lane 0; any column holding a node in the window (may still exceed the budget); `keep.selected`; `keep.reserved`; `keep.preferred`; everything else — least-recently-touched first within a class. A column open in no row is dropped, not counted.
- `Fold::is_empty` (fn) — nothing folded and nothing dropped: the window draws as laid out.
- `Fold::folded` (fn) — real columns behind the marker, ascending.
- `Fold::kept` (fn) — real columns drawn, in drawn order.
- `Fold::drawn` (fn) — where a real column draws, `None` if folded or dropped.
- `Fold::real` (fn) — the real column at a drawn position, `None` at the marker (dash parity keys on this).
- `Fold::marker` (fn) — the drawn column the rail takes, if anything folded.
- `Fold::folded_count` (fn) — lanes behind the rail, 0 when nothing folds; the screen header's `+N lanes` (ADR-0022).
- `Fold::drawn_columns` (fn) — columns drawn, marker included.
- `Fold::view` (fn) — a `DrawnRow`: the row in drawn columns (borrowed when `is_empty`), edges into folded lanes removed and listed in `folded`; `lanes` never reaches the rail column.
- `Toward` (enum) — `Below` (outgoing edge, branch continues below) / `Above` (incoming edge, came from above).
- `FoldedEdge` (struct) — `identity`, `toward`: an edge a fold took out of a row, drawn as a stub on the node.
- `DrawnRow` (struct) — `row: Cow<Row>`, `folded: Vec<FoldedEdge>` (outgoing first in parent order, then incoming ascending).

## inclusion
- `Follow` (enum) — `AllParents` or `FirstParent`; fixed per layout (it changes the seeds).
- `Inclusion` (struct) — `hidden_stashes`, `folded` merges, `stashes`; can change while a layout lives.
- `Inclusion::with_stashes` / `stashes` (fn) — set/read every stash entry's own commit (`stash@{0..}`).
- Rule: a shown stash entry keeps its **first parent only**; index/untracked parents go to the hidden frontier, so commits reachable only through them are not drawn. A change in the set re-includes from the stash's row.
- `seeds_for` (fn) — the seeds a view lays out from (first-parent drops integration lanes).
- `apply` (fn) — slice form: the commits a view shows, parents trimmed, and its seeds.
- `Included` (struct) — `CommitSource` wrapper letting through only what a view shows; remembers walked handles (ADR-0026: `HandleSet`, not an id list) so a change re-reads, never re-walks.
- `Included::new` / `seeds` / `follow` / `inclusion` / `inner` (fn) — build and query.
- `Included::walked` / `shown` (fn) — the walked and currently-shown handles, as `HandleSet`s.
- `Included::reinclude` (fn) — switch inclusion; the first row whose drawing can change, or `None`; replays through `CommitSource::links`, not whole commits. Trims parents in `pull`/`links`; `read` passes through.
- `reinclude` (fn) — switch a `StreamingLayout<Included<_>>`'s inclusion and truncate from the first changed row; returns `Invalidated`.

## layout
- `HandleLookup` (trait) — `handle_of(&self, id: Oid) -> CommitHandle` and `id_of(handle) -> Oid` (infallible); blanket-implemented for every `CommitSource`. Lets `LayoutCursor` and `layout()` resolve seed/parent ids to handles without needing the whole `CommitSource` seam.
- `layout` (fn) — lay out a whole commit slice in one call; unchanged signature — builds its own private id table internally (ADR-0026) so no caller outside `graft-graph` had to change.
- `LayoutCursor` (struct) — resumable lane state; a clone is a checkpoint.
- `LayoutCursor::new` (fn) — `new(seeds, handles: &impl HandleLookup)`; cursor seeded with lane 0 and integration lanes, by handle.
- `LayoutCursor::extend` (fn) — `extend(links: &[Link], handles: &impl HandleLookup, order: &mut EmittedOrder) -> Vec<RowShape>`; lay out the next batch without reading commits.
- `LayoutCursor::rows_emitted` (fn) — rows laid out so far.
- `LayoutCursor::approximate_bytes` (fn) — checkpoint cost estimate.
- `LayoutCursor::active_lanes` (fn) — current occupant of each column.

## layout::order
- `EmittedOrder` (struct) — `Vec<CommitHandle>` row → handle plus a dense `Vec<u32>` reverse lookup (ADR-0026); `new`, `from_handles` (was `from_ids`), `note`, `row_of`, `handle_at` (was `id_at`), `handles` (was `ids`), `len`, `is_empty`, `approximate_bytes`. `reserve` (pub(crate)) — reserves exact capacity for a batch about to be appended, bounding `Vec::push` doubling slack (m1-the-graph/51d); called once per `LayoutCursor::extend` batch and once in `from_handles`.
- `RepeatedCommit` (struct) — a commit the walk named twice: `commit` (`CommitHandle`), `first`, `again`.

## layout::source
- `CommitSource` (trait) — a walk the layout pulls links from (`pull`), hydrates by handle (`read`, whole commits, same order); `links` (parents as the view shows them), `handle_of`, `id_of` over the repository's id table.
- `Pull` (struct) — one pull: `links`, `done`, `stopped`.

## layout::stream
- `StreamConfig` (struct) — `chunk_rows`, `checkpoint_every`, `max_checkpoints`, `resident_chunks`, `buffer_rows`.
- `StreamError` (enum) — why streaming could not go on; `Assemble` when a row could not be assembled.
- `Advance` (struct) — one advance: `rows_emitted`, `done`, `stopped`.
- `Anchor` (struct) — selected `commit` and its viewport `offset`.
- `Resident` (struct) — held rows, chunks, checkpoints and their byte estimates; `passed_through` (rows laid out, never materialised).
- `Work` (struct) — pulls, re-reads (links re-served), replayed rows, rebuilt chunks, `hydrated` (commits read for rows), `merge_reads` (commits read only to name a lane), `links_read` (links the source served).
- `StreamingLayout` (struct) — chunked layout with a bounded resident set.
- `StreamingLayout::new` (fn) — stream a walk.
- `StreamingLayout::from_order` (fn) — `from_order(source, order: Vec<CommitHandle>, seeds, config)`; lay out a known order (ADR-0026: handles in, not ids).
- `StreamingLayout::advance` (fn) — lay out up to a budget of rows past the frontier; reads commits only for chunks meeting viewport ± `buffer_rows`, the rest pass through (shapes fingerprinted, nothing read).
- `StreamingLayout::set_viewport` / `viewport` (fn) — rows on screen.
- `StreamingLayout::rows` (fn) — rows in a range, rebuilding chunks not held (evicted or passed through) from the nearest checkpoint; every replayed chunk checked against its shape fingerprint, only the asked-for chunks read.
- `StreamingLayout::is_resident`, `rows_emitted`, `expected_rows`, `is_done`, `row_of`, `id_at` (fn) — queries.
- `StreamingLayout::reanchor` (fn) — viewport that keeps an anchor's offset.
- `StreamingLayout::resident`, `checkpoint_rows`, `work`, `source`, `config` (fn) — introspection.
- `StreamingLayout::checkpoint_stride` (fn) — rows between kept checkpoints; doubles each time `max_checkpoints` was reached (m1-the-graph/60), never shrinks.
- `StreamingLayout::source_mut` (fn) — the source, mutably.
- `StreamingLayout::truncate` (fn) — forget rows from a row on (chunks cut, later checkpoints dropped, order truncated), frontier replayed to it; a mid-chunk cut re-notes the kept prefix's fingerprint (`cache.renote`), so a later rebuild of that chunk is still checked (tests `truncate_keeps_a_fingerprint_for_the_kept_prefix`, `a_rebuild_after_truncate_is_checked`); returns `Invalidated`.
- `Invalidated` (struct) — `from_row`, `chunks`, `checkpoints` a truncate threw away.

## layout::cache (pub(crate))
- `fingerprint` (fn) — `fingerprint(&[RowShape]) -> Fingerprint` (row count + hash of shapes); what a pass-through leaves and a rebuild must match.

## raster
- `rasterise` (fn) — a `Strip` in, a premultiplied `core::RasterFrame` out; pure, no cache; background never painted (ADR-0003). Folded edges: a tail from the node's right edge, half a band toward `Toward`, alpha fading to 0 short of the next column's centre.
- `Strip` (struct) — `rows`, `above` (row before the strip), `lane_columns`, `marker`, `cell_width`/`cell_height`, `palette`, `muted`.
- `RasterRow` (struct) — a row in drawn columns plus `ring`, `emphasis`, `thread` (drawn columns on the selection's thread), `stubs` (folded edges, drawn as one fading tail per row). Clone, no longer Copy.
- `Stub` (struct) — `identity`, `toward` (`fold::Toward`): a folded edge for the rasteriser.
- `RasterRow::plain` (fn) — `Ring::of` the row, normal emphasis, no thread.
- `Ring` (enum) — node style by shape: `Commit`, `Merge`, `Root`, `Head`, `Stash`, `Uncommitted`; `Ring::of` (fn) from parent count.
- `Emphasis` (enum) — `Normal`, `Dimmed` (faded). Alpha only, never width: `Highlighted` removed (ADR-0024) — the ancestry thread and Focus dimming are expressed purely by alpha (`OFF_THREAD_ALPHA`, `FADED_ALPHA`), so no line, curve or ring is ever wider than another.
- `stroke_width` (fn, pub(crate)) — `(cell_width / 8.0).clamp(1.5, 2.5)`; the one stroke width for every lane, curve and node ring in a strip.
- `OFF_THREAD_ALPHA` (const, 0.5) — alpha of a lane column off the selection's ancestry thread, when a thread is present.
- `FADED_ALPHA` (const, 0.3) — alpha of an `Emphasis::Dimmed` row/column with no thread in play.
- `Marker` (struct) — the rail column: `column` only; drawn as a 1 px dotted muted vertical, no count (ADR-0022).
- `RasterError` (enum) — `EmptyPalette`, `TooLarge`, `Selection` (a selection without one `Lit` per strip row).
- `CELLS_PER_LANE` (const) — cells per lane column (2), matching the Unicode tier.

## raster::emphasis (re-exported from `raster`; ADR-0053)
- `Backdrop` (enum) — `Full` / `OffThread` / `Faded`: the one alpha a layout-only strip draws every element at; `alpha()` (1.0 / `OFF_THREAD_ALPHA` / `FADED_ALPHA`), `of(focus, thread_on_screen)`.
- `PlainRow` (struct) — a row as the layout draws it: `row`, `ring`, `stubs`; no selection. `PlainRow::of` — `Ring::of`, no stubs.
- `Lit` (struct) — one row's selection paint: `emphasis`, `thread: Option<&BTreeSet>` (`None` ≠ `Some(∅)`).
- `PlainStrip` (struct) — `Strip` with `PlainRow`s; same other fields.
- `Selection` (struct) — `rows: &[Lit]` (one per strip row) and the frame's `backdrop`.
- `Emphasised` (struct) — an emphasis image, `frame` cropped to whole cell columns, placed at `first_column`.
- `rasterise_backdrop` (fn) — a `PlainStrip` at one backdrop alpha; `Full` is byte-identical to `rasterise` with no thread.
- `rasterise_emphasis` (fn) — every element ADR-0024 draws above the backdrop `b`, at `(a − b) / (1 − b)`, same order and geometry as the single pass; cropped; `None` when nothing is above `b` (always under `Full`).
- `composite` (fn) — premultiplied "over" of an `Emphasised` onto its backdrop strip (iTerm2, tests).

## raster::cache
- `StripCache` (struct) — bounded LRUs of rasterised strips keyed on (row range, layout hash, theme id, cell size) (DESIGN.md §3.8). `backdrop`/`emphasis`/`composited` key on layout + backdrop (+ `lit_hash`); legacy `get`/`contains` still key on the selection until 46. Each layer its own LRU of `capacity` entries.
- `StripCache::backdrop` (fn) — a layout-only strip at one `Backdrop`, from the cache or rasterised; counts in `hits`/`rasterised`.
- `StripCache::emphasis` (fn) — the selection's emphasis image over a strip (`None` cached too); counts in `emphasis_hits`/`emphasis_rasterised`.
- `StripCache::composited` (fn) — iTerm2: backdrop with the emphasis drawn over, cached on (backdrop key, `lit_hash`); the backdrop itself when there is no emphasis.
- `StripCache::contains_backdrop` (fn) — is a backdrop strip resident; a look, not a use (idle prefetch draws backdrops only).
- `StripCache::new` / `capacity_for` (fn) — make one; strips a viewport plus buffer needs at any offset.
- `StripCache::strips_for` (fn) — aligned strip ranges covering a viewport.
- `StripCache::get` (fn) — a strip from the cache, rasterised on a miss; refuses a misaligned strip. An `Uncommitted` ring row does not count toward the strip height.
- `StripCache::contains` (fn) — is a strip already rasterised; touches neither counters nor recency (idle prefetch, m1-the-graph/30h).
- `StripCache::flush` (fn) — drop everything, every layer, on theme, cell-size, re-detection or resize.
- `StripCache::len` (legacy + backdrop strips), `is_empty` (no layer holds anything), `resident_bytes` (all four LRUs: legacy, backdrop, emphasis, composite), `stats`, `strip_rows` (fn) — introspection.
- `STRIP_ROWS` (const) — 32, chosen by measurement.
- `Flush` (enum) — `Theme`, `CellSize`, `Redetection`, `Resize`.
- `CacheError` (enum) — `Misaligned`, `Raster`.
- `CacheStats` (struct) — `hits`, `rasterised` (legacy and backdrop strips), `evicted` (any layer), `flushes`, `emphasis_rasterised`, `emphasis_hits`.
- `layout_hash` (fn) — hash of what the rasteriser draws from a strip (stubs included), colours and cell size aside.
- `plain_layout_hash` (fn) — `layout_hash` for a `PlainStrip`: the same fields, no selection to hash.
- `lit_hash` (fn) — hash of a `Selection`: backdrop, and per row the thread's columns if any else the row's emphasis, plus thread presence (a row's own emphasis under a thread is never drawn, so not hashed — as `layout_hash`, 30b).
