# m1-the-graph/15 — Folding merged branches, first-parent view, stash visibility

Status: done
Build with: **Opus** (high) — an inclusion predicate reusing `layout_chunk` (DESIGN.md's name; in code `layout` / `LayoutCursor::extend`, `graft-graph/src/layout.rs`), not new algorithm design
Blocked by: 14
Leaves runnable: yes — keys fold a merged branch, toggle first-parent view and show/hide stashes.

Why: first-parent view is the clean "what landed on main" history; folding is how a finished branch gets out of the way.

## Read first
- PROMPT.md §1.3 ("Folding" only)
- DESIGN.md §3.6 (last bullet), §3.2 (steps 1 and 3)
- Code: public APIs of `crates/graft-graph/src/layout.rs` (`layout_chunk`), `layout/stream.rs`, `layout/cache.rs`, `layout/checkpoint.rs`

## Scope
In:
- First-parent view and stash show/hide expressed as inclusion predicates over what the walker emits, reusing `layout_chunk`.
- Fold a merged branch into its merge commit, and expand it again.
- A key for each toggle.
- Decide what each toggle invalidates (cache entries, checkpoints) and invalidate only that.

Out:
- Mistake Radar on folded rows (M2).

## Acceptance
- [x] First-parent view of foxtrot and criss-cross is one straight lane (snapshot).
- [x] Fold then unfold returns rows equal to the original (property test).
- [x] The §1.2 invariant property tests pass with each predicate switched on.
- [x] Toggling never invalidates the cache wholesale: the invalidated set is asserted and written into this ticket (graph level; not yet reachable from a key).
- [x] A key for each toggle: `Space` (fold/unfold the merge under the cursor), `1` (first-parent view), `H` (hide/show the stash). Recorded in DESIGN.md §2.3.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## NOTES (build, 2026-09-13, builder-opus)

**Partial.** No compaction. Stopped at a clean seam: the ticket (graph filter + layout rewind + app wiring + keys + captures) was too big for one context.

Done (graph side, `crates/graft-graph`):
- `graph::inclusion`: `Follow {AllParents, FirstParent}`, `Inclusion {hidden_stashes, folded}`, `apply` (slice form), `Included<S>` (a `CommitSource` wrapper), `reinclude(&mut StreamingLayout<Included<S>>, Inclusion) -> Invalidated`.
- Rule: decided in walk order with a frontier of edge counts. Shown if a shown commit/seed points at it through a kept edge; hidden if only hidden commits or dropped edges do; an unpointed commit is a fresh tip (shown) unless it is a hidden stash, or in first-parent view. Folding M drops M's parents[1..]; its branch is hidden until it reaches a commit a shown lane waits for (the fork point).
- `StreamingLayout::truncate(row)` + `source_mut`; crate-private `RowCache::truncate`, `Checkpoints::drop_after`, `EmittedOrder::truncate`.

**Invalidated set** (asserted in `inclusion::tests::folding_invalidates_only_the_rows_from_the_merge_down`):
- Fold/unfold merge M, or stash show/hide: rows from `from_row` = the first row a named commit (M, or a stash) held, down. Chunks: the one holding `from_row` is cut back to rows before it, all later ones dropped (held and evicted fingerprints). Checkpoints: those strictly past `from_row` dropped; at-or-before kept. Order: truncated at `from_row`. The walk is never pulled again (re-reads only). Example: 42 rows, chunks of 4, checkpoints every 8, fold at row 18 → `chunks 4..=10`, `checkpoints [24, 32, 40]`; `[8, 16]` and chunks 0–3 kept, zero chunks rebuilt.
- A change naming no commit decided so far invalidates nothing.
- First-parent toggle: a **separate layout** (it changes seeds, so every row differs). Toggling switches layouts; the other view's cache is untouched.

Verified: 72 graft-graph lib tests incl. 3 directed inclusion tests, 2 proptests (streamed view == slice form over random chunking; fold→unfold == original, folded prefix untouched), `every_invariant_holds_under_every_inclusion` (replay, lane 0, lanes stay put, chunk-determinism under random Follow/folds/hidden stashes), corpus snapshots `first-parent-foxtrot-merge`, `first-parent-criss-cross-merge`. Not observed in the app: nothing is wired.

**Next (15b), exact steps:**
1. `app::history::stream` (crates/graft/src/history/mod.rs:227): wrap `WalkSource` in `Included::new(source, seeds, follow, inclusion)` and build `StreamingLayout::new(included, included.seeds().clone(), ..)`. Stash ids: `RefBadges::stash` / refs/stash reflog (the walk already includes `refs/stash`, graft-git repo.rs:187). First-parent: either `Follow::FirstParent` over the same walk, or graft-git's `Order::FirstParent` walk (history/mod.rs:149) — keep one layout per Follow.
2. Add a request on the history channel (beside `RowRequest`) carrying a new `Inclusion`; `serve` calls `inclusion::reinclude` and re-answers the current window (re-find selection by commit id).
3. Keys in `update.rs` (next to `is_lane_toggle`, ~line 424): fold/unfold selected merge, first-parent toggle, stash toggle — check DESIGN.md §2.3 for the assigned keys first. Footer hints in `graft-ui/src/graph_screen/mod.rs` `expand_hint` (~263).
4. Pty captures under `docs/shots/15-*` (reuse the 14 approach: release binary, pyte, 80x24): folded/unfolded merge, first-parent, stashes hidden.

Gotchas: `Included` must be laid out from `included.seeds()`, not the original seeds. `reinclude` re-reads from the walk's top to the change (gap logged, ticket 27). The KNOWN_GAPS `incoming`-not-in-`lanes` row does not affect this work (inclusion only removes commits/parents before layout); left logged.

## NOTES (continuation 1, 2026-09-13, builder-opus)

**Done.** No compaction.

App side (`crates/graft`):
- `history::View { follow, inclusion }` rides on every `RowRequest`; the newest request alone decides the view.
- `history::Views`: one `StreamingLayout<Included<WalkSource>>` per `Follow`, opened on first ask (the first reuses the walk `stream` started). Inclusion change → `graft_graph::inclusion::reinclude` (the invalidated set above); Follow change → the other layout, cache untouched (tested: `chunks_rebuilt` unchanged on return).
- `serve`: a newer request in another view interrupts `answer` (nothing sent) and is answered next.
- `History::change_view` (Follow switch → top; fold/stash keep the viewport), `History::selected`; refresh keeps the view.
- Keys in `update.rs` (`ViewChange`). Header names what is left out (`graft_ui::graph_screen::Filters`): "first-parent · N merges folded · stash hidden".

Verified: 563 tests; new: fold/unfold/first-parent over one walk on foxtrot + criss-cross (rows equal after unfold and after switching back, 2 sources opened), interruption test (caught a mutation removing the view check), 4 key tests, header text. Real binary in a pty on /tmp/g15/dated: `docs/shots/15-merges-full.txt`, `15-merges-folded.txt`, `15-merges-unfolded.txt`, `15-first-parent.txt`, `15-first-parent-back.txt`, `15-stash-hidden.txt`. Unfold and 1-1 captures are byte-identical to full. User judges by eye.

Gaps: closed the two m1/14–15 rows (keys/wiring; `+N` node lanes now have fold and first-parent as the answer). Logged: tied commit dates put a parent above its child (pre-existing, found while capturing); folded merge has no glyph; `1` goes back to the top; keys not in the footer.
