# Scout 06h graph-mode (main checkout, at 2d542fbd)

## Item 4 first: how a view change keeps the cursor
- `History::change_view` (`crates/graft/src/history/scroll/apply.rs:247`) sets `self.reanchor = selected commit` (:251), then `self.wanted(height, true)` (:274). No seek, no anchor on the request.
- When rows come back, `apply.rs:54-58`: if `reanchor` is among the new rows the cursor moves there; **otherwise "the cursor keeps its index"** (comment :52-53). No merge landing.
- `Sought.anchor_behind` is filled only from `request.promote.anchor` (`history/serve.rs:331-337`) and read only in the promotion path (`apply.rs:143-150`). `change_view` requests carry no `promote`, so it never applies to a view change.
- `space` close (G4): `apply_view_change` (`update/keys.rs:447-454`) does `history.change_view(...)`, then `history.seek(merge, height).or(request)` with `land_on = inside.merge` (`update/keys/view_change.rs:55`). **This is a `History::seek`.** The target is the merge itself, which is shown, so in practice it hits the on-hand branch (`history/scroll/query.rs:89-95`) or a `find` that should not reveal. Could not confirm what a find for a shown merge leaves in `take_reveal`.
- **Contradiction (stop):** the ticket says switching to folded with the cursor on a folded-away commit "lands on the merge that holds it… the same landing as `space` closing (G4) and as the promotion anchor". No such route exists for `change_view`: reanchor fails and the cursor keeps its index (`apply.rs:52-58`). The only existing merge landings are a seek (keys.rs:453) or a promotion anchor (serve.rs:331). The new landing has to be built: either the request carries an anchor that `serve.rs` answers through `whereabouts`, or the caller works out `behind` itself, which needs graft-graph `Whereabouts` before the request. The ticket rules out graft-graph changes and seeks.

## change_view callers (item 3)
`update/keys.rs:447` (`space`/`1`/`H`, all through `apply_view_change` :436); `update/history.rs:92` (reveal, ask at :90, then seek :95); `update/tree_answers.rs:57` (`head_known`, HEAD keep); `update/tree_answers.rs:287` (refs read, stash/HEAD view). `RowRequest` is built inside `change_view` (`apply.rs:274` / `:265-271`), so the ask must come before `keys.rs:447`. The tree_answers sites are view changes too: the ticket does not say whether they re-ask.

## View::folded() / History build (item 2)
- Production: only `history/state.rs:349`, in `History::started()` (:337). No config is passed in. `History::started()` production caller: `update/tree_answers.rs:97`. A re-walk copies `previous.view` (:105-107), so the mode carries over by itself.
- **Mismatch:** `1`/`H` do not reset to `View::folded()`. They toggle `follow` / `hidden_stashes` on a clone of the current view (`view_change.rs:38, 61-79`). They already keep the fold. Tests asserting `View::folded()`: `update/tests/graph_toggles.rs:27,143,198`.

## Config (item 1)
- `[graph]` typed `config/types.rs:71-93` (`renderer`, `default_branch`, `max_lanes`), serde Deserialize. Known keys `config/load.rs:350`; unknown → `ConfigWarning::UnknownKey` (:85).
- Problem list: `keymap_from` (`config/keys.rs:70`) + `theme_problems`, joined at `event_loop/startup/arrival.rs:45-46` into `config_problems`, seeded by `state.seed_keys` (:63). **Mismatch:** a second list (theme) also feeds it; the ticket names only `keymap_from`.

## space (Shown)
`ViewChange::Fold` (`view_change.rs:40-58`): `fold_at(cursor)?` → `None` exits. With `fold: None`, `opened_mut()?` is `None`, so nothing happens. Not tested here.

## Header (item 5)
`lane_phrases(tally)` `graft-ui/src/graph_screen/header_tail.rs:51`, single caller `layout.rs:247`. `Filters` (`graph_screen/mod.rs:189-197`: `first_parent`, `stash_hidden`, `focused`) reaches layout as `self.view.filters` (`layout.rs:215`). Tests: `graph_screen/tests/lanes_in_view.rs:73-78`.

## Step-1 grep
`grep -rn 'inclusion.folded\|folded: \['` over crates: empty.

## Build with
Sonnet once the cursor-landing contradiction is settled. Until then, stop: it goes to the planner.
