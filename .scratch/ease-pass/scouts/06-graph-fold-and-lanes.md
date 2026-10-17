# Scout 06 graph-fold-and-lanes (main d2af08f2)

## Today's opt-in fold
- `Inclusion` is a struct, `pub folded: BTreeSet<Oid>` (`graft-graph/src/inclusion.rs:55-61`), plus `hidden_stashes`, `stashes`. A folded id keeps only parent 0 (`first_parent_only`, inclusion.rs:200-208); hidden commits fall out via `Frontier` edge counts, decided one commit at a time in walk order (inclusion.rs:135-166). `Included`/`reinclude` re-exported at inclusion.rs:37.
- `Space` → `ViewChange::Fold` (`graft/src/update/keys/view_change.rs:28-38`): removes the selected id from `folded` if present, else inserts it only if `parents.len() >= 2`; on a non-merge it does nothing (`None`). Then `apply_view_change` (`update/keys.rs:438-455`) calls `history.change_view` and `fold_count::request`.
- `jobs::fold_count::count_folded` (`jobs/fold_count.rs:13-23`): `rev-list --count M ^M^1` minus 1 (octopus union counted once). Cached per session in `AppState::fold_counts`, asked **only** from `Space` (`update/fold_count.rs:1-9, 29-39`). With fold-by-default, nothing asks for counts of merges folded at first paint (knock-on).
- `FoldMark` (`graft-ui/src/graph_screen/mod.rs:411-430`) draws `+` / `+N` / `+?`; carried as `GraphEntry.fold` (mod.rs:404), filled at `graft/src/view/entries.rs:367-376` only when `inclusion.folded.contains(id)`.
- Header already says `N merges folded` from `Filters.folded` = `inclusion.folded.len()` (all folded, not in view) (`view/rows.rs:141`, `graph_screen/mod.rs:297-319`). Spec only replaces `+N lanes`; this existing wording would sit beside `K merged branches folded`.

## Emitted order (question 2)
**Not guaranteed children-before-parents.** No commit-graph → provisional order is `Order::CommitDate` (`graft-git/src/history/walk.rs:34-37`: "not necessarily a topological order"). Tie classes are Kahn-ordered, but "A clock that ran backwards still can put [a parent above its child]: that is the provisional order's known limit, and promotion corrects it" (`graft-git/src/history/simple.rs:12-21`). Strict topo only with commit-graph or after promotion (`history/stream.rs:63-79`, ADR-0008). → builder stops per spec.

## Seeks to a possibly hidden commit
All route through `History::seek` (`history/scroll/query.rs:71-111`): not on hand → `Wanted::Find`; serve lays out to `usize::MAX` (whole history) when the id has no row (`history/serve.rs:180-181`), then answers the `fallback` window (serve.rs:213-219); cursor stays put (`scroll/apply.rs:156-161`, "Not in this view").
- `n`/search: `update/search.rs:178-182, 191-209` → footer `(not in this view)` (`graft/src/search.rs:332-336`).
- Palette commit / Map `⏎`: `PaletteTarget::Seek` → `map::seek_to` (`update/palette/run.rs:53-56`, `update/map.rs:202-208`); no word on a miss: could not confirm any.
- `J` put-back: Recover → `Landing::Commit` (`update/operate.rs:199-200`) → `land_on_head` → `history.seek` (`update/banner/stop.rs:348-375`); silent on a miss.
- Detail parent links: `update/detail/landing.rs:119,142-151` → `history.seek`; result handling at :119 not read in full.
- Overview strip: marks only rows resident in `state.history` window (`view/strip.rs:17-24`); hidden commits get no mark.
- Ghost Preview slice: `view/preview.rs:3-29` draws the preview's own slice; no `Inclusion` reference in graft-ops or view/preview (grep). Could not confirm how it treats a folded commit.
- Also `tour.rs:178` seeks HEAD.

## Lanes
- `+N lanes`: `graft-ui/src/graph_screen/layout.rs:350-359` (`left_out`), N = `Fold::folded_count` (`render.rs:85`).
- `Fold::plan` (`graft-graph/src/fold.rs:73-110`): if `columns <= max_lanes` returns **all** `0..columns` kept, node-less columns included. Over cap, `scan` (fold.rs:287-305) keeps columns that are `open` (any lane `Some` or touched) — node-less pass-throughs are drawn unless they fold behind the rail as low-priority candidates. `noded` only marks required.
- No `open_columns()` method today; `scan`'s local is named `open_columns`.

## Spec vs code
1. Spec premise "walk emits children before parents" — false for provisional date order (simple.rs:19-21).
2. `FoldWalk::new(&[CommitHandle]…)` — `CommitHandle` exists (`graft_core::commit_handle`, used `layout.rs:27`); ok.
3. `FoldMark::words` today returns `+N`, not `▸ N commits from…`.
4. `merge_name` has no `Merged PR` pattern (grep empty); only three forms (merge_name.rs:15-18).
5. `Filters.folded` is a global count, header word is `merges folded` not `merged branches folded`.
6. `Space` on a non-merge opened row does nothing today (view_change.rs:33-35).

## Budget tests
`event_loop/tests/settle.rs:312` (held j), `event_loop/tests/pace.rs:220`, `event_loop/tests/pixel_scroll.rs:192`. First-paint fast-tier test: could not confirm which pins it (`tests/it/budget_run.rs` appears heavy tier).
