# m1-the-graph/15 — Folding merged branches, first-parent view, stash visibility

Status: ready
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
- [ ] First-parent view of foxtrot and criss-cross is one straight lane (snapshot).
- [ ] Fold then unfold returns rows equal to the original (property test).
- [ ] The §1.2 invariant property tests pass with each predicate switched on.
- [ ] Toggling never invalidates the cache wholesale: the invalidated set is asserted and written into this ticket.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
