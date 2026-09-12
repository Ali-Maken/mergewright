# m1-the-graph/07 — Compaction (`+N`), folding, and first-parent view

Status: ready
Build with: **Fable** (the graph layout engine)
Blocked by: 05, 06

Why: width is what makes a busy history unreadable. A graph twelve lanes wide on an 80-column terminal fails the checkpoint question on its own, however correct the lanes are.

## Read first
- PROMPT.md §1.2 ("Compact output") and §1.3 ("Folding")
- DESIGN.md §3.5 and §3.6's last bullet (folding and first-parent view as inclusion predicates)
- DESIGN.md §1.3's narrow (80×24) mockup — that is the width this has to survive

## Goal
The graph stays narrow on purpose: rarely-touched lanes fold behind one `+N` column, a merged branch collapses into its merge commit, and first-parent view gives a clean default-branch-only history.

## Scope
In:
- Compaction per §3.5: triggered when open non-reserved lanes exceed `graph.max_lanes` (default 8, configurable), ranked least-recently-touched first, never folding lane 0, a reserved integration lane, or a lane on the selected commit's ancestry path.
- `+N` is a **render-time** decision over `Row` data that keeps every real column, so expanding is instant and never re-runs layout.
- Folding a merged branch into its merge commit, and expanding it again.
- First-parent view and stash show/hide expressed as **inclusion predicates** over what the walker emits, reusing `layout_chunk` rather than a second code path.
- `graph.max_lanes` added to config with the same no-silent-default rules M0's config ticket set.

Out:
- The key bindings and the glyph (`+N` rendering belongs to 08/11/13; this ticket produces the data and a rendering-agnostic description of it).
- Mistake Radar interaction with folded rows (M2).

## Acceptance
- [ ] A fixture with more than `max_lanes` parallel branches renders within the width limit, and the folded set is the least-recently-touched one.
- [ ] Lane 0, reserved lanes and the selected commit's ancestry are never folded (property test).
- [ ] Expanding a `+N` restores exactly the pre-fold columns for the affected rows and changes no other row's column.
- [ ] First-parent view of the foxtrot and criss-cross fixtures is a single straight lane.
- [ ] Fold/unfold and first-parent toggling do not invalidate the row cache wholesale (say what they do invalidate).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
