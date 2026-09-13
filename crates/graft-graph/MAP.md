# graft-graph — map
One line per public item. Update when a public item changes.

## lib.rs
- `fold`, `identity`, `lane`, `layout`, `merge_name`, `row`, `seeds` (mods) — re-exported at the root (`Fold` too).

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
