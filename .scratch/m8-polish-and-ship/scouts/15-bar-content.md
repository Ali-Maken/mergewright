# Scout 15: bar content (code at b4724fee)

## Edit stop (ruling 1)
- Context: `HintContext::OperationLive{row,c,tab_to_banner,space_stages}`. The bar is built at `graft-ui/src/graph_screen/hints.rs:627-641`: `whole_bar(row)` gives the **row's full list**, then `lists::said_live_on_the_graph` (`lists.rs:500-516`). That function only rewrites `c` and puts `a amend it` in front (`said_live`, `lists.rs:458-488`). **It removes no write key.** So `m merge` (`branch_row_list` `lists.rs:338`, `plain_row_list` `:377`) and `u undo` (`:345`, `:362`, `:378`, TAIL `:214`) go through unchanged.
- Ticket cites `lists.rs:445-468` as where `u` and `m` are drawn. Those lines are `said_live` doc and code. `u`/`m` come from the row lists above, not from that range.
- Edit-stop contexts are already in `every_context` (`catalogue.rs:37`; `ContinueAmends` at `:242`, `:265`, `:320`).
- `m` refuses on detached HEAD at `crates/graft/src/update/preview/history.rs:105` (from the ticket; not re-read).

## Search (ruling 2): traced
- Opening `/`: `update/search.rs:25` sets `search_prompt`. Esc in the prompt clears it (`:74`). `⏎` also clears it (`:80`), then `:112` sets `next.search = Some(Search…)`. `view/strip.rs:186` also sets `state.search`.
- The search is cleared **only** by `update/keys/escape.rs:30` (`next.search.take()`), and by `search.rs:97`, which replaces it with a new search.
- `view/rows.rs:325-335` `search_line`: Prompt while the prompt is open, **else `Status` whenever `state.search` is Some**.
- `graph_screen/footer.rs:86-90`: any `SearchLine` takes the footer, and `key_line` runs only in the all-None arm (`:92-98`). That is why the bar is held after `⏎` until Esc. Confirmed in code.
- No header-tail code was found for the search. Ticket 5a's tail order was not checked.

## `a amend` on older rows (ruling 3)
- `row_verbs` `view/rows.rs:817-834`: `amend = on_head && !amend_blocked`. HEAD's row only.
- Ahead/behind keeps **counts only**: `AheadBehind{ahead,behind}` (`graft-git/src/history/divergence.rs:27-32`). Cache: `refs/mod.rs:177-178` (`HashMap<(Oid,Oid),AheadBehind>`). Read via `push_box.rs:199` `head_ahead` (a count).
- **The `upstream..HEAD` set is not resident.** `count()` walks with flags (`divergence.rs:73-75`) and throws the set away.

## Stash rows (ruling 4)
- There is no stash-row hint context. `cursor_row_context` `view/rows.rs:789-809` gives `BranchRow` only for a Branch badge, else `PlainRow`, and `Rewritable` wraps it when `rewrite_offered` (`:850ff`). A stash row therefore gets `plain_row_list` (`m`, `r`, `R`). Stash-only marking is drawing only (`rows.rs:140`).

## `u undo` at width 100
- Drop rule: `fitting.rs:119-160` `fit_list` takes the longest prefix of `ladder` (`fitting.rs:61-108`). It stops at the first step that doesn't fit (`:149-155`). Rank is list order, so `u` drops when the hints before it fill 100 columns.
- On marked rows, `? keys` is put after the later of `u`/`/` (`hints.rs:652-660`).
- Held by tests `tests.rs:393` (`resize_is_drawn_only_when_nothing_else_was_dropped`) and `tests.rs:433-445` (marked PlainRow at 100 must contain `u undo`), plus `tests.rs:2747`.
- Adding `o`/`a` via `with_verbs` (`lists.rs:124`) moves `u` later in the list.

## Knock-on
- `RowVerbs` (`hints/context.rs:12-40`, `ALL` holds 4 combinations; the catalogue uses it). `with_verbs` `lists.rs:124`.
- Captures in `docs/shots/` with a stop, search or stash bar: `m8-help-rebase-stop-120x36.txt`, `m7-proto-1b-edit-stop.txt`, `m8-banner-bar-q-{80x24,187x50}.txt`, `m8-quit-stop-now-80x24.txt`, `m8-quit-stopped-merge-relaunch-80x24.txt`, `m2-18-git-git-search-enter-{80x24,160x40}-unicode.txt`, `26-search-far.txt`, `26-search-typing.txt`, `15-stash-hidden.txt`. The test `footer.rs:334` `a_search_line_still_wins_over_a_stop` pins the current search precedence.
- Palette on `?`/`:`/`⌃P`: **could not confirm** (not traced).
