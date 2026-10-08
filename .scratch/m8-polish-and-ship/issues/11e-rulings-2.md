# 11e — rulings 2: the builder's two stops and three open points (planner spec-only pass, 2026-10-08)

Status: draft. Amends `11e-doorway-footers-and-widths.md` and `11e-rulings.md` only where named; the rest stands.
Inputs: builder 11e summary (NOTES on branch `worktree-agent-aaabcad8cbbc98e96`; lines below are the builder's, rule 7),
`scouts/11e-footers.md` Q1/Q4, keymap §2.1 :38–60, §2.9 :475–486, 09's key table row (`09-keymap-table-and-translate.md:242`:
"`b` … §2.9 says planned, not yet built; the code built it").

## Rulings

**G1 — `b` (stash.branch) goes on the stash footer; not `NEVER_HINTED`.** Cause: `b` is bound (`update/stash.rs:229`)
and was "hinted" only by `HintContext::Stash`, a list no screen drew (11e F2) — so he has never seen it. `NEVER_HINTED`
holds two classes only (§2.1 :49–56: navigation/meta globals; verbs not valid at the cursor); `b` is neither, and §2.1
:57–58 makes a bound row verb on no bar a finding. So:
- S footer, **Ready with a stash selected only**, gains the `b` clause after `drop`, before `files` (§2.9's row order).
  Words: what `Keymap::key_verbs` spells for `stash.branch` from the table — no new word; NOTES quotes it.
  Not on Loading / Empty / Failed (nothing selected: §2.1 mode rule).
- It sheds by E1 like every middle clause; the doorway still outlives it.
- `every_bound_action_is_hinted_live_or_is_named_here` gains the S footer as a hint source (11e's box), and passes with
  **no** `NEVER_HINTED` row added.
- Keymap §2.9 :483 is stale (09 found it): status text becomes "built (m3/m5 name prompt, `update/stash.rs`)"; the
  key, action and meaning are unchanged. That one cell is the only `02-keymap.md` edit allowed.

**G2 — Map Empty / Failed draw only keys that act there.** Cause: the ready footer is drawn for Empty and Failed
(scout Q4: only Loading branches, `render.rs:129`), so `j/k select` (`render.rs:145`) is drawn where there is no row to
select. §2.1's mode rule: a key whose verb is not valid there is not drawn. Footer per state:
- Ready: unchanged (`j/k select  ⏎ zoom to commits  M/esc back  ? keys` + notes, E2).
- Empty, Failed: drop `j/k select`; drop `⏎ zoom to commits` too **if** the sweep shows it is a no-op there (the
  builder reported j/k only — not assumed; NOTES says which, with the handler line). `M/esc back  ? keys` stays.
- Not changed here: what `j`/`k`/`⏎` do at Empty/Failed (the handler), or any retry key at Failed (none is designed).

**G3 — Map Loading draws the doorway.** Builder observed `?` opens the palette at Loading; E3's check-first resolves to
`esc cancels  ? keys`. Knock-on to prove: `esc` with the palette open closes the palette only — the load keeps running
(a cancelled load from closing help would be a trap, ux H3).

**G4 — journal box test: drive the state, not a key.** No J footer key opens a box (builder). The test sets the
preview the way J's view reads it (`state.preview` → `box_open`, `view/journal.rs:63`) and asserts on the drawn frame.
S's case is driven by its real key (`⏎` opens the apply preview, §2.9). NOTES answers, by `rg`, which path (if any)
sets `preview` while J is shown; none → filed as a **question** (possible dead branch at `journal_screen.rs:391`),
nothing deleted.

**G5 — the text-field test edit: allowed only if setup-only.** Acceptable when every `assert*` line of
`a_text_field_inside_a_place_draws_no_keys_doorway` is byte-identical to base and the edit is fixture/setup forced by
the deleted variants or a changed signature. Any assertion changed, weakened or removed → revert it and stop.

## Acceptance (adds to 11e's; each tick names its test, at tick time)
- [x] (`graft_ui::stash_screen::tests::the_branch_key_is_drawn_only_with_a_stash_selected`, green; red by mutation) G1 drawn: S Ready (stash selected) footer at 120 and 200 has the `b` clause between `drop` and `files`; absent at
  Loading/Empty/Failed. (`graft_ui::stash_screen::tests::the_branch_key_is_drawn_only_with_a_stash_selected`)
- [x] (`graft::update::tests::place_footers::every_key_a_place_footer_draws_is_answered`, green; asserts `name_prompt` opens and that `b` was pressed) G1 answers: `every_key_a_place_footer_draws_is_answered` presses `b` at S Ready and the stash branch name prompt
  opens. (that test)
- [x] (`graft::update::tests::every_bound_action_is_hinted_live_or_is_named_here`, green; changed-line grep → 0, ticket NOTES builder 2) G1 no exemption: `every_bound_action_is_hinted_live_or_is_named_here` green; `NEVER_HINTED` diff vs base empty
  (`git diff <base> -- crates/graft/src/update/tests.rs | grep -c NEVER_HINTED` → 0, in NOTES). (that test, the grep)
- [x] (`graft::update::tests::place_footers::every_key_a_place_footer_draws_is_answered`; base failure quoted in ticket NOTES builder 2) G2 red first: `every_key_a_place_footer_draws_is_answered` over Map Empty and Failed fails on base on `j` (quote
  it), green after. (that test)
- [x] (`graft_ui::map_screen::tests::empty_and_failed_draw_no_keys_that_do_nothing`, green) G2 drawn: Map Empty and Failed footers at 48, 80, 120, 200 contain no `j/k`; `M/esc back` and `? keys` present.
  (`graft_ui::map_screen::tests::empty_and_failed_draw_no_keys_that_do_nothing`)
- [x] (`graft::update::tests::place_footers::the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`, green) G3: `?` at Loading opens the palette; footer ends `esc cancels  ? keys`; `?` then `esc` leaves the Map still
  loading (job not cancelled). (`graft::update::tests::the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`)
- [x] (`graft::update::tests::place_footers::a_box_over_a_place_draws_no_keys_doorway`, green) G4: `a_box_over_a_place_draws_no_keys_doorway` — J via state, S via `⏎`; bottom row has no `? keys`. NOTES has
  the `rg` answer. (that test)
- [x] (the diff, described in ticket NOTES builder 2 G5; no assert line changed) G5: NOTES quotes the test's diff hunk vs base and states "no assert line changed". (the diff)
- [x] (capture tests, unset run green; list in ticket NOTES builder 2) Captures (adds to allowed diffs): S shots gaining the `b` clause (`m3-06-stash-list-*`, `m3-stash-list`,
  new `m8/doorway-stash-*`); Map Empty/Failed shots losing `j/k select`, if any exist (NOTES lists). Bar rows his to judge
  (rule 6); colour unverified (rule 5).

## Touches (adds)
- `docs/design/02-keymap.md` §2.9 :483 status cell only (G1). Keymap table / `keymap_ids.txt`: none.
- `stash_screen.rs` footer + its tests; `map_screen/render.rs` `footer_line` state branches + tests.
- `update/tests.rs` sweeps (hint source added; `NEVER_HINTED` untouched).
- graft-ui MAP `stash_screen` / `map_screen` lines: the per-state footer words.
- Contact sheet: S and Map shots re-assembled by the conductor.
