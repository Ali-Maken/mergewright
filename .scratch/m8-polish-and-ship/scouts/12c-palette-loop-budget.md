# Scout: 12c palette loop budget (badge work per row)
Current main @ be167302. The ticket has no Scout line, so this report goes to the fallback path. Line numbers point at code, not at intent.

## Gathered / ordered
- `RefBadges::on` `crates/graft/src/refs/mod.rs:41`: one hash lookup that returns a `&[RefLabel]` slice. It does **no sort or filter on each call**. The drawing order is fixed once, at the refs read: `refs::badges` `refs/mod.rs:269-327` pushes HEAD, then branches, then `push_remotes_tags_and_stash`. So no all-refs ordering runs on each frame.
- Row builder `crates/graft/src/view/entries.rs:324-329`: an optional `CompareBase` badge, then **every** label on the commit mapped through `badge_view` (`entries.rs:374`). Each one calls `sanitize_path_bytes` and `BranchIdentity::of_name`, and the results are collected into `GraphEntry::badges`. This is O(refs on commit) for every drawn row on every frame.
- `mark_stash_only` `crates/graft/src/view/rows.rs:141-157` **appends** the `OnlyInStash` badge **last**. Its comment says it is last so that `geometry::badge_at` counts refs from the front. Any cut-to-width would drop this badge on a piled row. That is a contradiction with "same cells", and the ticket does not mention it.

## Measured / cut (graft-ui `graph_screen/render.rs`)
- `badges_width` :556-570 calls `badge.text()` (one String allocation) for **every** badge and sums them before it applies the half-column budget. That is O(N) allocations. It is called from `draw_badges` :475, from `badge_spans` :530 and from `subject_truncated` :591 (the footer reveal, `footer.rs:62`). So it runs at least **3 times per drawn row**.
- `badge_spans` :519-551 does `Vec::with_capacity(N)` and then breaks at the first zero width (:543-545). The loop itself is bounded by width, but the allocation and the `badges_width` call inside it are not.
- `draw_badges` :451-477 draws only the returned spans. It calls `badge.text()` again for each drawn badge.
- `hit::badge_index_at` `crates/graft/src/view/hit.rs:42-56` rebuilds the **whole** `graph_view` and then calls `badge_spans`. Its index maps one for one onto `refs.on()`, with a -1 offset for the compare base (`update/geometry.rs:475-485`).

## Other full readers of badges / `refs.on`
- `cursor_row_context` `view/rows.rs:774-780` takes the first `Branch` in `cursor.badges`. HEAD and branches come first, so a cut keeps it, but this was **not tested**.
- Preview: `view/preview.rs:236-237` maps every overlay label through `badge_view`. `update/preview/slice.rs:229-233, 276, 339` read `refs.on`.
- Detail: `view/detail.rs:193` `side_name` does a `find_map` over `refs.on`. This is the compare side only, not a per-frame row.
- Radar: `update/radar.rs:440-455` `force_push_candidates` walks `refs.on(id)` for each resident row. It is O(refs) per row, on the update side. `view/radar.rs:287,554` calls `refs::badges` (could not confirm whether those calls are test-only).
- Pixel tier: `rows.rs:128` says `view::strips` paints its own rows. A grep of `view/strips*` for `badges` finds nothing.
- Picker: no `badge` or `refs.on` reads in `view/picker.rs`.

## Caches
None for badges. `GraphEntry`s are rebuilt every frame. The `refs` read is the only one-time step.

## Perf script `scripts/perf-palette.sh`
It builds `$SCRATCH/refs-50000`: 3 commits carrying 40k `team/*` branches and 10k `v1.N` tags, packed. It runs `perf-palette-pty.py` at 120x36 with `GRAFT_PERF_TRACE`. It computes `loop_us` (update+draw+write) p50/p95/max and `update_us` p95. It writes `docs/perf/m8-palette.txt` plus `m8-trace-summary.sh` output. It has **no fixture argument** yet, and the fixture path is hardcoded.

## Knock-on
- Capture: `docs/shots/11-ref-badges.txt`.
- Tests: `graph_screen/tests.rs:698,750,2161` (`badge_spans_match_the_drawn_cells`); `tests/it/many_refs.rs:59`; `frames/run.rs:340`.
- Ticket 05: Status reads "held until his trace run". Its scope (`05:18-23`) is per-frame work that grows with refs, so it is the same class. Its known candidate `view/strip.rs` is disjoint.
- 09h and 12b name none of `render.rs`, `entries.rs`, `rows.rs` or `hit.rs` (grep of both tickets). 09h touches `graft-ui/src/keymap` and `update/edge.rs`.
