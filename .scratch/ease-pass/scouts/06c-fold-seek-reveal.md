# Scout 06c fold-seek-reveal (HEAD ec7d4c3f)

## What exists
- **Seek**: `History::seek` `graft/src/history/scroll/query.rs:79-96`. It checks the rows on hand, then `find` (`:102-115`) asks `Wanted::Find{commit,rows,fallback}`, sets `jump` and `seeking`. `Wanted::Find` doc `history/state.rs:51-63`.
- **Serve**: `history/serve.rs:187-188`. Find stops at 0 if `layout.row_of` has it, else `usize::MAX` (the full layout). Answer range `:221-227`: the row's rows, else `fallback`.
- **Answer applied**: `scroll/apply.rs:155-171`. Cursor clamps; with `seeking` and the id in the window → cursor on it; else the comment "Not in this view: the cursor stays" (`:166`). Then `jump`/`seeking` are cleared (`:176-177`). There is no miss enum: a miss is inferred as "selected != target".
- **Fold queries**: `FoldWalk::behind(CommitHandle)->Option<CommitHandle>` (`graft-graph/src/inclusion/fold_walk.rs:254`), `inside` (`:265`). `Included::walked/shown/fold` (`inclusion/included.rs:119,125,133`). `reinclude` (`:145`). `handle_of(&Oid)` (private?, `:332`). `OpenedMerges::open` (`inclusion.rs:91`). `Inclusion::opened_mut` (`:134`).
- **Space open path**: `ViewChange::Fold` (`update/keys/view_change.rs:41-57`) toggles one merge via `opened_mut`. Then `history.change_view` and an optional `seek(merge)` (`update/keys.rs:447-454`). `space` is claimed here.
- **Search**: `search.rs:273` holds `missed: bool`; footer `line` (`:327-340`) appends `"  (not in this view)"` (`:332-336`). Seek in `update/search.rs:178`; `missed` set at `:180-182` and in `seek_landed` (`:189-209`).
- **Other seek callers**: `map::seek_to` (`update/map.rs:202-212`), called from Map `:196`, picker `update/picker.rs:266`, palette `update/palette/run.rs:62`. Detail `update/detail/landing.rs:119,142-160`; miss words `:237-239` ("can't show … not in this view"). Banner `land_on_head` (`update/banner/stop.rs:351`, seek `:378`, `Landing::Commit` `:359`). Tour `update/tour.rs:178`. Re-seek on promotion `scroll/apply.rs:93`. `space`'s own landing seek (`keys.rs:453`).
- **Unchanged-by-test files**: strip `view/strip.rs:17-24` doc matches. `view/preview.rs:1-9` slice from rows already sent.

## Mismatches with the ticket
1. `behind` takes and returns a **`CommitHandle`**, not an `Oid` (`fold_walk.rs:254`). `whereabouts(Oid)` needs `handle_of`, which is a private trait fn at `included.rs:332`, so access could not be confirmed.
2. `seek` is at `query.rs:79-96`, not `:71-111`.
3. Serve is at `serve.rs:187-188` / `:221-227`, not `:180-181` / `:213-219`.
4. The miss path is at `apply.rs:155-171`, not `:156-161`.
5. "The seek's miss answer (names follow `history::state`)": no miss type exists. A miss is `selected != target` (`update/search.rs:181,206`) plus the detail landing at `:237`.
6. Palette is at `run.rs:62`, not `:53-56` (those lines are editor/theme/tour).
7. `stop.rs:348-375` is the `land_on_head` doc and body; the seek is at `:378`. Also "`J` put-back" → `operate.rs:199-200` is a `Recover` `MoveRef`/`Recreate`. `DropPutBack` gives no landing (`operate.rs:232`). Could not confirm that `J` maps to Recover.
8. Unlisted seek callers: picker `⇥` (`picker.rs:266`) and the promotion re-seek (`apply.rs:93`).
9. The `space` open path opens **one** merge per `ViewChange`. A multi-merge open has no existing helper.
10. `search.rs:333`: the ticket says `:332-336`, which matches; the review's `:333` also holds.

## Tests today
- `update/detail/landing/tests.rs:287-294` (miss words).
- `update/place/tests/esc_back.rs:275,340`.

## Build with
Opus xhigh, as ticketed. The seek, serve, inclusion and six routes span two crates.
