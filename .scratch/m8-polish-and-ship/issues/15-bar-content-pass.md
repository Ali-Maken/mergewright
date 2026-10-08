# G-bar — the bar offers what acts: at a stop, after a search, on stash rows, on his unpushed rows

Status: done — A1, A4, A5 and the palette check here; **A2 built in `15b-search-leaves-the-footer.md`** (see its NOTES).
(Amended 2026-10-08 after scout `.scratch/m8-polish-and-ship/scouts/15-bar-content.md`; the
**Amendment** section below overrides the Rulings, Signatures and Acceptance sections where they differ)
Build with: **Opus** — brownfield, four small list/fit changes plus one footer-precedence move, each held by a test;
no structure a wrong approach would lock in (fails Fable test 1).
Blocked by: 10, 10c, 11, 11b, 14
Needs: `keymap-spell-from-the-table` (hints over `ActionId`), `palette-help-and-actions` (`?`/`:`/`⌃P` open the
palette), `palette-doorway-and-hint-rows` (doorway `? keys`; same `hints/*` files — serial; it settles `a amend` in
`NEVER_HINTED` and on HEAD's row, ruling 3 here covers the older rows), `daily-ops-fixes` (ruling 4's refusal exists
first).

## Read first
- **This ticket's Amendment section** (below), then the scout's file above — it carries every file:line used here
- `docs/design/02-keymap.md` §2.1 bullets "The hint bar is a ranked selection" (the two never-hinted classes) and the
  modal-pane rule; §2.2 rows `Esc`, `/`
- `docs/research/ux-principles.md` §3.1 (where he lands; visibility of the next step) — one section
- `crates/graft-ui/MAP.md`: `graph_screen::hints` (`lists.rs`, `RowVerbs`, `with_verbs`), `HintContext::OperationLive`,
  `GraphView::search`/`stopped`, `SearchLine`, `HeaderView`/`header_line`; `crates/graft/MAP.md`:
  `view::rows::row_verbs`, `RefBadges::is_stash`, `refs` (ahead/behind), `search`

## Rulings (planner, original — superseded where the Amendment says so) and the defects they close
1. **Edit stop offers keys that refuse.** At a rebase `edit` stop the bar draws `u` and `m`, both refuse
   (`crates/graft-ui/src/graph_screen/hints/lists.rs:445-468`; `m` refuses `m needs a branch checked out` because the
   detached-HEAD check runs first, `crates/graft/src/update/preview/history.rs:105`). Ruling: at any stop, a write
   key that would refuse is not offered (§2.1 mode rule). The stop's own keys (`c a s`, `?`) lead.
   → **Amended A1** (wrong lines; where the filter goes).
2. **Search holds the bar.** After `/root ⏎` the search line held the bar through `⏎`, `⇥`, `x` and compare; the pane's
   own keys never showed (observed, not traced). Ruling: the search line holds the bar only while the `/` field is
   open. After `⏎` the bar is the focused place's keys led by `n next · N prev`; `3 of 12 · /root` moves to the header
   tail until `Esc` ends the search. Scout traces the current path first (rule 7). → **Amended A2** (traced).
3. **`a amend` missing on older unpushed rows.** `a` acts on unpushed rows of his branch (put-in, m7-09) but the bar
   shows it on HEAD's row only, for want of a spawn-free per-row "not pushed" fact. Ruling: if `upstream..HEAD` is
   already resident (the ahead/behind walk visits it), `row_verbs` reads it and offers `a amend` there; if not
   resident, carry the row past v1 — no spawn per row, no hint that may refuse. → **Amended A3: cut to a gap.**
4. **Stash rows advertise `r m R`**, which `daily-ops-fixes` now refuses. Ruling: a stash row's bar offers `⏎ open`
   and the stash's own verbs, not `m r R`. → **Amended A4** (new context; "the stash's own verbs" restated).

## Amendment (planner, spec-only pass, 2026-10-08)

Line numbers below are the scout's, at `b4724fee`.

**A1 — Edit stop (replaces ruling 1's cited lines and placement).** `u`/`m` are not drawn by `said_live`
(`lists.rs:458-488`). They come from the row's full list (`branch_row_list` `lists.rs:338` `m`, `:345` `u`;
`plain_row_list` `:362`, `:377` `m`, `:378` `u`; `TAIL` `:214` `u`), passed unchanged through
`said_live_on_the_graph` (`lists.rs:500-516`), built at `hints.rs:627-641`. Decision:
- `said_live_on_the_graph` removes every hint whose `ActionId` is in one new const, `REFUSED_WHILE_LIVE`, after its
  existing `c`/`a amend it` rewrite. Placed in graft-ui `hints/lists.rs` because that is where the live list is made;
  held against the app by the sweep below, so the const cannot drift from what refuses.
- Seed set: `u` (`app.undo`), `m` (merge). Also `r`, `R`, `o` **if and only if** the builder finds the line that
  refuses each while an operation is live (`undo::live_refusal_words`, `operate::claim_write`, `preview_refusal`,
  `history.rs:105`). Every member listed in NOTES with its refusing file:line; a key with no refusing line stays
  drawn (rule 7).
- Applies to every `OperationLive` form (rebase/merge/cherry-pick/revert stops and bisect), not edit only.
- The stop's own keys keep the order `said_live` already gives them (`a amend it`, `c <LiveC>`, `space stage`,
  `⇥ banner` per MAP); this ticket does not reorder them. "`c a s`" in ruling 1 was shorthand, not an order.

**A2 — Search (replaces ruling 2's mechanics; keeps its outcome).** Traced: the prompt is `search_prompt`
(`update/search.rs:25`, cleared `:74` Esc / `:80` ⏎); the search itself is `state.search`, set `:112` and
`view/strip.rs:186`, cleared only by `update/keys/escape.rs:30` (and replaced at `search.rs:97`).
`view/rows.rs:325-335` maps it to `SearchLine::Status` whenever `Some`; `footer.rs:86-90` lets any `SearchLine` take
the footer, so the bar returns only on Esc. Decision:
- `search_line` yields `Prompt` while `search_prompt` is open, else `None`. `SearchLine::Status` is **removed**
  (the compiler then finds every reader). Its text and `failed` flag move to a new header-tail item, `SearchTail`,
  drawn verbatim as `Status` draws it today (no rewording), warning-styled when `failed` (colour unverified).
- Header-tail order and drop order: tail items left→right `search · <walk/filters/gfx items as today>`, the
  operation banner tail where it is today. When room runs short, drop whole items in this order: gfx, walk, filters,
  **search**, banner — the banner is safety, the search is what he is navigating now. Drawn whole or not at all.
  (5a's tail order was not checked by the scout; the builder reads `header_line` and keeps every existing item's
  relative order — only the search item is new.)
- While `state.search` is `Some` and the prompt is closed, a graph-focused bar is wrapped in a new
  `HintContext::Searching(Box<HintContext>)`: the inner list led by `n next  N prev`. `graft` applies it to the
  graph-row forms (`WorkingTreeRow`, `BranchRow`, `PlainRow`, `StashRow`, `Marked`, `Rewritable`) and **inside**
  `OperationLive`, so at a stop the stop's keys still lead and `n N` follow them. Not applied to `ChangesPane`,
  `PaneFocused`, `Banner`, `Preview`, overlays. `Esc` stays unhinted (§2.1 globals).
- Test `footer.rs:334` `a_search_line_still_wins_over_a_stop` pins the old precedence. It is **replaced**, not
  deleted: `the_search_prompt_still_wins_over_a_stop` (prompt open + stop → prompt drawn) and
  `after_enter_a_stop_row_is_drawn_and_the_search_moves_to_the_header` (stop row + `Searching` bar; header tail
  holds the search text). Named in NOTES.

**A3 — `a amend` on older unpushed rows: cut to a gap.** Not resident: ahead/behind keeps counts only
(`AheadBehind{ahead,behind}`, `graft-git/src/history/divergence.rs:27-32`; cache `refs/mod.rs:177-178`); `count()`
walks the set and discards it (`divergence.rs:73-75`). Keeping it means a new graft-git API and a cache shape change
across three crates — not a polish pass. `row_verbs` (`view/rows.rs:817-834`) is unchanged: `a amend` on HEAD's row
only; the older rows act on `a` but are not hinted (no hint that may refuse holds). **Conductor files a KNOWN_GAPS
row** (unverified-not-broken class: missing hint, behaviour correct): "`a amend` hinted on HEAD's row only; older
unpushed rows act but aren't offered — needs `upstream..HEAD` kept by `divergence::count`; past v1."

**A4 — Stash rows (replaces ruling 4's "the stash's own verbs").** No stash-row context exists:
`cursor_row_context` (`view/rows.rs:789-809`) gives `BranchRow` or `PlainRow`, so a stash row draws
`plain_row_list`'s `m`, `r`, `R`. The stash's own verbs (`p d b ⏎`, keymap §2.9) are bound in the `S` overlay, not
on a graph row, so the graph row cannot offer them. Decision:
- New `HintContext::StashRow`. `cursor_row_context` checks `RefBadges::is_stash(id)` **first** (before the Branch
  badge) and returns it; never wrapped in `Rewritable` (`R` refuses there), may be wrapped in `Marked`/`Searching`/
  `OperationLive` like any row.
- List: `plain_row_list` without `m merge`, `r rebase`, `R rewrite`; no `with_verbs` (`o take` already refuses on
  a stash, `can_take`); `S stash` (`graph.stash`) inserted right after `⏎ open` — the door to the stash's own verbs.
  If `TAIL` already holds `graph.stash`, it moves there in this context rather than being drawn twice.
- `every_context` yields `StashRow` bare and `Marked`; new coverage slots; `COVERAGE_SLOTS` grows; MAP records the
  numbers.

**A5 — `u undo` dropping at width 100: not intended; the fit changes.** Cause: `fit_list` (`fitting.rs:119-160`)
takes the longest **prefix** of the ladder and stops at the first step that does not fit (`:149-155`), so rank is
list order; `with_verbs` (`lists.rs:124`) inserting `o`/`a` pushes `u` later until it falls off at 100. Ruling: `u`
is the visible way back (M3 promise; ux-principles §3.1 recovery; keymap §2.1 already says the doorway "never
displaces" `u undo` on marked rows). So:
- Fit priority is separated from drawn order: in any list holding `u undo`, it is fitted with priority **second**
  (right after the list's first hint). When room runs out, the lowest-priority hints give way; survivors are drawn in
  their list order. `/ search` keeps its §2.1 standing on marked rows (fitted before the doorway).
- Holds at every width where the bar draws its 3-hint floor, for every graph-row form × `RowVerbs::ALL`, bare,
  `Marked`, `Rewritable`, `Searching`. Existing tests `tests.rs:393`, `tests.rs:433-445`, `tests.rs:2747` must still
  pass unchanged.
- **Conductor:** add one sentence to `02-keymap.md` §2.1 "ranked selection" bullet: "`u undo` is fitted second in
  any bar that holds it; what gives way is the lowest-ranked hint, not `u`." (Design decision, made here.)

## Scope
In: A1, A2, A4, A5; confirm the old "`?`, `:` and `⌃P` do nothing" finding is closed by G-palette (a test that
presses each on the graph and asserts the palette opened). Out: A3 (gap); doorway (done); keymap bindings; words of
any existing hint (new hints `n next`, `N prev`, `S stash` are the only words added).

## Signatures
graft-ui (`graph_screen`):
```rust
pub enum SearchLine { Prompt(String) }                 // Status removed (A2)
pub struct SearchTail { pub text: String, pub failed: bool }
pub struct HeaderView<'a> { /* existing fields */ pub search: Option<&'a SearchTail> }
pub enum HintContext { /* existing */ StashRow, Searching(Box<HintContext>) }
// hints/lists.rs, private to the hints module:
const REFUSED_WHILE_LIVE: &[ActionId];
// hints/fitting.rs: fit_list keeps its signature; priority is internal (A5)
```
(`SearchLine::Prompt`'s payload type stays whatever it is today; shown as `String` for shape only.)
graft: `view::rows::search_line(&AppState) -> Option<SearchLine>` (Prompt only); a new
`view::rows::search_tail(&AppState) -> Option<SearchTail>` feeding `HeaderView::search`; `cursor_row_context` gains
the `is_stash` arm; the `Searching` wrap where the hint context is built. Both MAP.md files record each.

## Acceptance
- [x] A1 Edit stop: bar drawn at 80/120/160 has no `u`, no `m`; every drawn key acts (sweep extended with an
      edit-stop context; test named). Each `REFUSED_WHILE_LIVE` member's refusing file:line in NOTES.
      — `graft` `update::tests::no_key_a_live_operation_bar_draws_refuses_while_live` (presses every drawn key of
      every `OperationLive` form, `ContinueAmends` included, at 80/120/160; status must not be a live or
      detached-HEAD refusal; probe: with the filter commented out it reported 1347 refusals — `u` 309, `m` 474,
      `r` 564); capture asserted by `rewrite_stop::capture_m8_bar_edit_stop_120x36`.
- [x] A1 Every other `OperationLive` form (each `LiveC` × `tab_to_banner`) draws no `REFUSED_WHILE_LIVE` key (test).
      — `graft-ui` `hints::tests::no_live_operation_bar_offers_a_key_that_refuses_while_live` (all forms, 80–200,
      both dividers).
- [x] A2 Search (built in 15b): `/root ⏎` then the bar shows `n next  N prev` and the pane's keys; header tail shows the search text
      (was "the match count" — same text, as `Status` drew it); `Esc` clears both (tests).
      — `graft` it `search_hint::after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header` (real
      AppState, corpus octopus, `/topic ⏎`: header holds `Search::line` verbatim, bar starts ` n next · N prev · `
      then `⏎ open`…; `Esc` → `state.search` is `None`, neither header nor bar has it); `graft-ui`
      `hints::tests::a_search_after_enter_leads_the_row_bar_with_next_and_prev` (every row form × 80–160 × both dividers).
- [x] A2 At a stop, `/root ⏎` draws the stop row with the stop's keys leading, `n N` after; the prompt still wins
      while open (the two tests replacing `footer.rs:334`, named).
      — `graft-ui` `footer::tests::the_search_prompt_still_wins_over_a_stop` and
      `footer::tests::after_enter_a_stop_row_is_drawn_and_the_search_moves_to_the_header`; order at every live `c`:
      `hints::tests::at_a_stop_a_search_puts_next_and_prev_after_the_stops_keys`.
- [x] A2 Header narrowing drops gfx, walk, filters, search, banner in that order, each whole (test).
      — `graft-ui` `graph_screen::tests::header_narrowing_drops_gfx_walk_filters_search_banner_in_that_order`
      (140→8 columns, each item whole or absent, never a later one gone while an earlier is drawn, all five steps
      reached) and `graph_screen::tests::header_narrowing_drops_the_search_before_the_banner`.
- [x] ~~`a amend` on the second unpushed row hinted iff resident (test), or NOTES states not resident with
      file:line and the conductor carries the row.~~ **Cut (A3):** not resident, file:line in A3; conductor files
      the gap row. Builder: no work.
- [x] A4 Stash row bar: no `m`, `r`, `R`, `o`; has `⏎ open` then `S stash` (test); every drawn key acts (sweep
      extended with `StashRow`).
      — `graft-ui` `hints::tests::a_stash_row_offers_open_then_stash_and_nothing_that_refuses_on_a_stash`; `graft`
      `update::tests::a_stash_row_is_classified_and_no_key_on_its_bar_refuses_a_stash` (drawn frame classifies
      `StashRow`; each drawn key pressed, no stash refusal); `every_key_the_bar_draws_is_answered` and
      `every_swept_state_is_the_context_it_claims` now sweep `StashRow` (via `every_context`).
- [x] A5 `u undo` drawn on every graph-row form × `RowVerbs::ALL` at 100 and at 80 columns wherever the bar holds
      `u` (test); the three existing fit tests pass unchanged.
      — `graft-ui` `hints::tests::undo_is_drawn_at_100_and_80_on_every_row_form_that_holds_it` (bare, `Marked`,
      `Rewritable`, both dividers). `resize_is_drawn_only_when_nothing_else_was_dropped`,
      `the_hints_resize_once_pushed_off_are_drawn_again` and the `tests.rs:2747` test pass untouched. **Five other
      existing tests were changed** — see NOTES.
- [x] `?`, `:`, `⌃P` on the graph each open the palette (test).
      — `graft` `update::tests::help_colon_and_ctrl_p_on_the_graph_each_open_the_palette` (asserts
      `state.palette.is_some()` after each press).
- [x] Captures: `docs/shots/m8-bar-edit-stop-120x36.txt`, `m8-bar-after-search-120x36.txt`,
      `m8-bar-stash-row-120x36.txt`, `m8-bar-head-row-100x30.txt` (HEAD's row with `a amend`, `u undo` present).
      **Colour unverified** (search-failed warning style).
      — **Partial:** three of four written, each asserted by its test: `rewrite_stop::capture_m8_bar_edit_stop_120x36`,
      `stash_in_history::capture_m8_bar_stash_row_120x36`, `hint_bar::capture_m8_bar_head_row_100x30`.
      `m8-bar-after-search-120x36.txt` belongs to A2 (split off). By-eye check is the user's (rule 6).
      — **Fourth, 15b:** `m8-bar-after-search-120x36.txt` (3.2 KB, no home path), asserted by `graft` it
      `search_hint::after_enter_the_bar_is_the_rows_keys_and_the_search_is_in_the_header`. Colour unverified.

## Touches
- `hints/lists.rs` stop, row and new stash lists; `hints/fitting.rs` (A5); `every_context` + `coverage_slot` +
  `COVERAGE_SLOTS`; `footer.rs` precedence; `header.rs` tail (`header_line`, `tail_room`); `view/rows.rs`
  (`search_line`, `cursor_row_context`, hint-context build); `view/strip.rs:186` (sets search — check still right);
  `every_bound_action_is_hinted_live_or_is_named_here` (must still pass: `u`/`m` remain hinted on rows off a stop).
- Palette/help `?` "every key of the place": grep whether it reads `HintContext`; if so, `StashRow`/`Searching` must
  list there too — report.
- Captures to re-shoot (list each in NOTES, old → new): `m8-help-rebase-stop-120x36.txt`,
  `m7-proto-1b-edit-stop.txt` (prototype — leave, note only), `m8-banner-bar-q-{80x24,187x50}.txt`,
  `m8-quit-stop-now-80x24.txt`, `m8-quit-stopped-merge-relaunch-80x24.txt`,
  `m2-18-git-git-search-enter-{80x24,160x40}-unicode.txt`, `26-search-far.txt`, `26-search-typing.txt`,
  `15-stash-hidden.txt`, plus any 80/100-column graph capture whose bar changes under A5 (grep `docs/shots/` for
  bars missing `u undo`).
- MAP.md graft-ui, graft. `02-keymap.md` §2.1 sentence (A5) — conductor, not builder.

## Scout questions — answered
- Search line lifetime: see A2 (set `search.rs:112`, `strip.rs:186`; cleared `escape.rs:30`).
- `upstream..HEAD` set kept? No — see A3.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~65k tokens (A2 and A5 added work; A3 removed). If it runs over, split A2 off as its own ticket —
it shares `footer.rs`/`header.rs` with nothing else here.

## NOTES (builder, 2026-10-08)

Scout used: `.scratch/m8-polish-and-ship/scouts/15-bar-content.md`. No compaction. Context ran well past 65k
before A2 started, so **A2 is split off per this section** — nothing of A2 is built: `SearchLine::Status`,
`SearchTail`, `HeaderView::search`, `HintContext::Searching`, `footer.rs:334`'s replacement tests and the capture
`m8-bar-after-search-120x36.txt` are all still to do. The search line still holds the bar until `Esc`.
Next step for the A2 ticket: start at `crates/graft/src/view/rows.rs` `search_line` (Prompt only), then
let the compiler find every `SearchLine::Status` reader. The shared seams A2 will touch that this ticket changed:
`every_context`/`coverage_slot` (now 114 slots, `StashRow` = 113) and `view::rows::live_wrapped`/`back_to_place`.

**`REFUSED_WHILE_LIVE` members and their refusing lines** (graft):
- `app.undo` — `update/undo.rs:344` (`start_undo` → `in_progress_refusal`, words `undo.rs:240`
  `live_refusal_words`). Exception kept by the app but not by the bar: a merge begun in the shell, once the
  journal read says so (`undo.rs:173-178`, keymap §2.6a), sends `u`. The bar's context cannot tell that case
  apart, so `u` is not offered there either (a hint left out, not a key refused; `?` still lists it).
- `graph.merge`, `graph.rebase` — `preview::history_key` → `open_preview` → `preview_refusal`
  (`update/preview.rs:380`); at an `edit` stop `m` refuses first on detached HEAD (`update/preview/history.rs:112`).
- `graph.take` — `preview::take_key` → `open_preview` → `preview_refusal` (`update/preview.rs:380`).
- `graph.rewrite` — `rewrite_editor::open_rewrite` → `preview_refusal` (`update/rewrite_editor/open.rs:48`).
Verified by press for `u`, `m`, `r` (the probe above). `o` and `R` are verified by **code reading only**: no
live fixture draws them (`row_verbs` is `NONE` while live; `Rewritable` is never wrapped live), so the filter
is belt-and-braces for them. `b branch`, `t tag` stay drawn: no refusing line found (`name_prompt.rs:104` refuses
only branch-from-stash).

**A4.** The stash test is `RefBadges::is_stash` on the selected commit — the same check `m`/`r` refuse on
(`preview/history.rs:108`). `StashRow` is also wrapped by `OperationLive`/`PaneFocused` (graft wraps any row);
`catalogue` yields those forms so the sweeps cover them. `m6/fix-radar-bookkeeping-120x40.txt` changed to the
stash bar: that fixture's cursor rests on a stash row after the pull (the classifier says so; not re-traced).

**A5 — existing tests changed** (the ticket said only the three named must pass unchanged; these five encoded
"drawn = prefix of the list", which A5 ends):
- `hints::tests::a_hint_is_dropped_only_when_its_words_will_not_fit` — "next" hint is now next in fit order.
- `hints::tests::no_hint_loses_ground_as_the_room_for_it_grows` — matched by key, relative order, not index.
- `hints::tests::marked_leads_with_explain_and_draws_no_actions` — subsequence of the plain list, not prefix.
- `hints::tests::one_rule_puts_the_doorway_before_a_tail_and_last_without_one` and
  `graph_screen::tests::footer_eighty_branch_row` — expected 80-column strings: `t tag`/`c commit` give way to
  `u undo`.

**Palette/help (`?` "every key of the place")**: it does not read `HintContext` — `grep -rln HintContext` over
`graft/src` and `graft-ui/src` finds no palette source file (only the test `update/tests/keymap_edge/palette.rs`).
No non-test code in `graft` names `PlainRow`/`BranchRow` outside `view/rows.rs`, so no other classifier needed
`StashRow`.

**Captures re-shot** (each a one-line footer change; `git diff docs/shots` shows it): `m2-overview-strip`,
`m5-hints-{branch,plain,marked}-80x24`, `m5-hints-ladder`, `m5-lane-cap-160x24`, `m5-ui-banner`,
`m5-ui-context-menu-verbs-80x24`, `m5-ui-drop-menu-80x24`, `m6/fix-c-live-frames-187x50`,
`m6/fix-landing-after-esc-{120x40,187x50}`, `m6/fix-radar-bookkeeping-120x40`, `m6/stash-clash-stop-line-80x24`,
`m7/compare-since-split-80x24`, `m7/rewrite-edit-stop-{120x40,187x50}`, `m7/stash-in-history-{120x40,80x24}`,
`m8-help-80x24`, `m8-look-chrome-80x24`, `m8/fork-follows-banner-187x44`; 26 insta snapshots in
`crates/graft-ui/tests/snapshots` and 4 in `crates/graft/src/view/snapshots` (footer line; `assertion_line`
headers dropped by insta). New: `m8-bar-edit-stop-120x36`, `m8-bar-stash-row-120x36`, `m8-bar-head-row-100x30`.
Not re-shot from the ticket's list (their bars did not change, the capture tests passed):
`m8-help-rebase-stop-120x36`, `m8-banner-bar-q-*`, `m8-quit-*`, `m2-18-git-git-search-enter-*`, `26-search-*`,
`15-stash-hidden` — the search ones change with A2. `m7-proto-1b-edit-stop.txt` is a prototype, left.
