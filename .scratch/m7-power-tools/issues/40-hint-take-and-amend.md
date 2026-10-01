# 40-hint-take-and-amend — `o take` and `a amend` on the bar where they act

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M4: `o`/`a` on no bar at 187 columns; the guard
`every_bound_action_is_hinted_live_or_is_named_here` reads `crates/graft/src/update/keys/bound.rs`, which lists `o` only
as the resolver's take (`:96-98`); `a` sits in `NEVER_HINTED` (`crates/graft/src/update/tests.rs:5726`, `a` row
:5750-5757) as an "open question". Keymap §2.1:45-56: a bound row verb on no bar is a finding; `NEVER_HINTED` holds
navigation globals only.
Build with: **Sonnet** — the `o` and `a` rules are settled below; the work is a predicate move, a context field and tests.
Scout: done, `.scratch/m7-power-tools/scouts/40.md` (HEAD 0387e7fb). Read it first; its line numbers are used here.
Needs: none.
Blocked by: none (36 merged).
File set: graft — `src/view/rows.rs`, `src/update/context_menu/verbs.rs` (+ `slots.rs` only if the call changes),
`src/update/amend_into.rs`, `src/update/keys/bound.rs`, `src/update/tests.rs`, `src/update/context_menu/tests.rs`,
`crates/graft/MAP.md`; graft-ui — `src/graph_screen/hints.rs`, `src/graph_screen/hints/{context,lists,catalogue,tests}.rs`,
`src/graph_screen/tests.rs`, `crates/graft-ui/MAP.md`; `docs/shots/m7/*`. **Not graft-ops.** Disjoint from
39-ops-file-size-and-raw-names (graft-ops + `ARCHITECTURE.md` + design §6).

## Read first
Scout `40.md` (whole). graft MAP: `update::context_menu::verbs`, `update::amend_into`, `view::rows` /
`hint_context`, `update::keys::edit_stop::space_stages_here` (the one-predicate-two-callers precedent). graft-ui MAP:
`graph_screen::hints`. Keymap `docs/design/02-keymap.md` §2.1 (:45-56), §2.3 `a` :202 and `o` :205, §2.4 :265.

## Scope
Decided:
- **`o take` rule: the bar follows the menu.** `o` is hinted on a commit row exactly when the `.` menu's Cherry-pick slot
  is live — today: every commit row except HEAD's own row and merges (`pick_slot`, verbs.rs:144-165, via
  `PickParams::check_commit(id, id == head.oid, parents.len())`). That includes ancestors of HEAD, where the menu is
  deliberately live (verbs.rs:146-147). **Why:** one key, one meaning across surfaces (ux-principles: consistency); the
  bar never hints a row `o` refuses, since `o` runs the same check (`preview/take.rs:112`); and graft holds no
  spawn-free "on HEAD's branch" fact in `hint_context` (scout 40.md:4). Narrowing ancestors out is a later change to the
  one predicate, so both readers move together. This ticket does not change or verify what `o` does on an ancestor.
- One predicate, two readers: `take_check` lives in `update/context_menu/verbs.rs`; `pick_slot` reads its `Err` for the
  dim words (unchanged), `hint_context` reads `can_take`. `preview/take.rs` is untouched (it already calls
  `check_commit`).
- `o take` placed **second**, after `⏎ open`.
- **`a amend` rule: HEAD's row only** (by oid — detached HEAD included), and only when `a`'s own cheap blocks are clear
  (live op P14, pending amend, open editor: `a_key` amend_into.rs:85-90). Extract those into `amend_blocked`; `a_key`
  calls it, `row_verbs` calls it. Older rows: **no `a`** — graft has no spawn-free "not pushed" fact (the pushed check
  runs git in graft-ops `rewrite/unpushed.rs:26-27`; radar's `UnpushedOnDefault` covers the default branch only), and a
  hint on a row `a` may refuse is worse than none. Placed right after `c commit`; where the context has no `c commit`,
  after `⏎ open`.
- **While an operation is live:** `live_wrapped` (rows.rs:221) passes `RowVerbs::default()` — no `o`, no `a`. `a`
  refuses there (P14); `take_check` does **not** look at live ops, and whether `o` refuses during one is not checked
  here, so the bar stays silent rather than hint a row the key may refuse. If the builder finds `o`'s own live-op
  refusal (file:line), note it in NOTES; the rule stands either way.
- `Rewritable(_)` (rows.rs:771, fn :811) wraps the new-shaped `BranchRow`/`PlainRow`; verbs ride inside.
- Graph's `o` added to `BOUND_ACTIONS` (`bound.rs`); `a` removed from `NEVER_HINTED` (tests.rs:5750-5757).
- Verb words: `take`, `amend`. No short forms (prefix table unchanged).
Out: `WorkingTreeRow` and the changes pane (`c commit` leads; `a` there unchanged, unhinted — keymap §2.4:265 lists it;
the guard sees it hinted on HEAD's row). Any other bar ranking. `a` on older rows (gap row below).

**For the conductor — KNOWN_GAPS row to file (missing, not broken):**
> `a amend` is not hinted on older unpushed rows of his branch. `a` acts there (put-in, m7-09) but the bar shows `a`
> on HEAD's row only: graft has no spawn-free per-row "not pushed" fact (the check runs git in graft-ops
> `rewrite/unpushed.rs`), and hinting a row `a` may refuse was ruled out. Fix: a resident per-row pushed fact, then
> `row_verbs` reads it. Source: m7-power-tools/40, scout `scouts/40.md:17`.

## Signatures
```rust
// graft-ui::graph_screen::hints (context.rs)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowVerbs { pub take: bool, pub amend: bool }
pub enum HintContext { /* … */ BranchRow { current: bool, verbs: RowVerbs }, PlainRow { verbs: RowVerbs }, /* … */ }

// graft::update::context_menu::verbs — the one take predicate; `pick_slot` and `can_take` read it.
pub(crate) fn take_check(state: &AppState, row: usize) -> Result<(), /* check_commit's error type, as read */>;
pub(crate) fn can_take(state: &AppState, row: usize) -> bool; // take_check(..).is_ok()

// graft::update::amend_into — a_key's cheap blocks (P14 live op, pending amend, open editor), extracted, not copied.
pub(crate) fn amend_blocked(state: &AppState) -> bool;

// graft::view::rows
fn row_verbs(state: &AppState, row: usize) -> RowVerbs;
// take: can_take(state, row); amend: row is HEAD's oid && !amend_blocked(state)
// live_wrapped passes RowVerbs::default() instead.
```
`every_context` yields all four `RowVerbs` per variant, `Rewritable` arms (catalogue.rs:57-59, :395) included;
`coverage_slot` stays exhaustive (new slots; `COVERAGE_SLOTS` bumped).

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `a_commit_off_his_branch_hints_o_take` (red first): bar's second hint `o take`.
- [ ] `heads_row_has_no_o`, `a_merge_row_has_no_o`: rows the menu dims → no `o`.
- [ ] `an_ancestor_of_head_hints_o_like_the_menu`: the decided rule — ancestor row, menu live, bar draws `o take`.
- [ ] `heads_row_hints_a_amend` (red first); `an_older_row_has_no_a`; `a_pending_amend_hides_a`;
      `a_live_op_row_hints_neither`.
- [ ] `the_menu_and_the_bar_agree_on_o` (no live op): over every row of a fixture holding HEAD, an ancestor, a merge and
      a commit off his branch, `can_take` = menu slot enabled = `o` drawn.
- [ ] `a_key`'s existing tests green unchanged (the extraction is behaviour-neutral) — named in NOTES.
- [ ] `every_bound_action_is_hinted_live_or_is_named_here` green with graph `o` in `BOUND_ACTIONS` and `a` out of
      `NEVER_HINTED` — **red first** with `o` added and the ranking not yet built (quote it).
- [ ] `every_key_the_bar_draws_is_answered`, `every_context_reaches_every_shape`, the floor and 80-column tests green.
- [ ] **Captures** (fast-tier tests): `docs/shots/m7/bar-take-row-187x44.txt` — **right:** cursor on a teammate's
      commit; bar reads `⏎ open  o take …`. `docs/shots/m7/bar-amend-row-187x44.txt` — **right:** cursor on HEAD; bar
      holds `c commit  a amend` and no `o`. Colour unverified.
- [ ] MAP.md for graft-ui (`RowVerbs`, the two variants) and graft (`take_check`, `can_take`, `amend_blocked`,
      `row_verbs`, `bound.rs`).
- [ ] Gates.

## Touches (check each; report how)
- **Every `PlainRow`/`BranchRow` match and build:** graft rows.rs:221 (`live_wrapped`), :469-470 (`footer_context`),
  :566-568, :757-769; graft-ui `hints/lists.rs:40-42`, `hints.rs:388-389` and :558-559, `catalogue.rs` :37-59, :118,
  :143-152, :279-280, :395, :445. Grep in NOTES; list each.
- **`Rewritable(_)`** (rows.rs:771; catalogue.rs:57-59, :395): inner context carries verbs. Check: `every_context`
  tests and the rewrite-offer bar tests.
- **`Marked(..)`/`PaneFocused { row }` wrapping a row context:** verbs ride inside; `PaneFocused` keeps its filter.
  Check: their tests.
- **The `.` menu:** `pick_slot` reads `take_check`, behaviour unchanged. Check: `context_menu/tests.rs` green and the
  `Cherry-pick (o)` capture byte-identical.
- **`a_key`:** calls `amend_blocked`; refusal words and order unchanged. Check: its tests.
- **Bar width:** two more candidates; the shrink ladder drops by rank. Re-taken fast-tier bar captures listed in NOTES
  (e.g. `m5-hints-*`); heavy-only captures listed stale, not hand-edited.
- **Keymap:** none — `o` §2.3:205 and `a` §2.3:202 already bound; §2.1's `NEVER_HINTED` rule is now true.
- **Journal/undo:** none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~50k. One context.

## NOTES (builder, partial — handoff)
Scout used (40.md). Not compacted.

**Live-op check (rule 7):** both keys refuse during a live op, so hiding is right.
- `o`: `keys.rs:381` → `preview::take_key` → `open_preview` → `preview_refusal` (`update/preview.rs:340-341`, called :365): any `in_progress` refuses.
- `a`: `amend_into.rs:85` `refused_by_live_operation` (P14).

**Done (compiles, new tests green):** `RowVerbs` + struct variants (graft-ui `hints/context.rs`); `with_verbs` (lists.rs); catalogue × 4 verbs, live forms only with NONE; `COVERAGE_SLOTS` 111; `take_check(id, parents, head)` + `can_take(state)` (verbs.rs; `pick_slot` reads `take_check`); `amend_blocked` (amend_into.rs, `a_key` order unchanged); `row_verbs` (rows.rs: graph focus, no live op, drawn cursor row only); `o take` in `BOUND_ACTIONS`; `a` out of `NEVER_HINTED`.
Guard red first: "bound, and drawn on no bar in any context: `a amend`, `o take`" — now green.
New tests green (context_menu/tests.rs): `a_commit_off_his_branch_hints_o_take`, `heads_row_has_no_o`, `a_merge_row_has_no_o`, `an_ancestor_of_head_hints_o_like_the_menu`, `heads_row_hints_a_amend`, `an_older_row_has_no_a`, `a_pending_amend_hides_a`, `a_live_op_row_hints_neither`, `the_menu_and_the_bar_agree_on_o`. Boxes NOT ticked: suite not green.

**Deviations:** `take_check` takes `(id, parents, head)`, not `(state,row)`; `can_take(state)` reads the cursor row. `o take` is second only on a plain row; on a non-current branch row it follows `⏎ open` (4th).
Files outside set touched (compile-forced by the enum shape): `graft/src/update/mod.rs` (re-export `can_take`), `graft/src/update/keys/live_c_tests.rs`, `graft-ui/tests/support/screen.rs`.

**Next — 14 failures at last workspace run:**
1. **Spec conflict:** `m5_walk::every_scenario::hints_at_80_show_three_and_more`: current-branch row at 80 cols now `⏎ open  c commit  a amend  . more` — `a amend` evicts `P push` from the floor. Needs a ranking decision (put `a` later?) from planner.
2. Captures/goldens to retake: `view::tests::capture_m2_overview_strip`, graft-ui `capture_hint_bar_for_docs_shots`, it `hint_bar::capture_hint_bar_for_docs_shots`, `m6_stash_put_back::capture_menu_fixed_slots`, `context_menu::capture_context_menu_for_docs_shots`, `m7_walk::picker::menu_slot_two_dim_on_head`, `name_and_drag::capture_the_drop_menu_for_docs_shots`, `history_place::capture_esc_back_to_history_from_the_graph`, `blame_place::capture_esc_back_to_blame_from_the_graph`.
3. Bar assertions in `graft/tests/it`: `hint_bar::a_plain_row_hints_the_plain_context`, `the_tip_row_hints_the_current_branch_context`, `esc_and_the_walk::esc_then_g_on_a_long_history_draws_its_last_rows`, `radar_bookkeeping::a_pull_rebase_marks_no_duplicates` — all outside the file set; not inspected.
4. Then captures `docs/shots/m7/bar-{take,amend}-row-187x44.txt`, MAP.md rows, gates.
