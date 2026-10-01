# 40-hint-take-and-amend — `o take` and `a amend` on the bar where they act

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M4: `o`/`a` on no bar at 187 columns; the guard
`every_bound_action_is_hinted_live_or_is_named_here` reads `crates/graft/src/update/keys/bound.rs`, which lists `o` only
as the resolver's take (`:96-98`); `a` sits in `NEVER_HINTED` (`crates/graft/src/update/tests.rs:5588-5595`) as an "open
question". Keymap §2.1:51-56: a bound row verb on no bar is a finding; `NEVER_HINTED` holds navigation globals only.
Build with: **blank — scout first.** Suggested after the scout: Sonnet.
Scout: **yes, one, ≤1 page**: `view::rows::hint_context` — what it knows per graph row (HEAD's row, branch badges,
first-parent chain, any pushed/unpushed or "on HEAD's branch" fact) without a spawn; the `.` menu's Cherry-pick slot
predicate (it dims with `o`'s refusal words — graft MAP `context_menu`); `a_key`'s cheap checks (graft MAP :240); graft-ui
`HintContext::{PlainRow, BranchRow}` match sites and `catalogue::every_context` / `coverage_slot`.
Needs: none. **Wave 2** (after 36-offer-keys-and-daily-wording: both read the pane bar's tests).
Blocked by: 36
Crates / files: graft `view/rows*` (`hint_context`), `update/keys/bound.rs`, `update/tests.rs` (`NEVER_HINTED`), the
hint tests; graft-ui `graph_screen/hints/{context,lists,catalogue}.rs` and their tests. Disjoint from
39-ops-file-size-and-raw-names (graft-ops).

## Scope
Decided:
- **`o take`** on a commit row exactly when the `.` menu's Cherry-pick slot is not dimmed — one predicate, two readers
  (move it where both call it; no copy). Placed **second**, after `⏎ open` — on a row off his branch it is the daily
  verb there.
- **`a amend`** (keymap §2.3:202's verb; the box explains the older-commit case) on: HEAD's row; and an older row of
  his branch that graft can already tell `a` would act on — on HEAD's first-parent chain and not pushed — **without a
  spawn**. Placed right after `c commit`. If the scout finds no spawn-free "not pushed" fact: HEAD's row only, and the
  conductor files a KNOWN_GAPS row "`a` on older unpushed rows is unhinted" — never a hint on a row `a` refuses.
- Graph's `o` added to `BOUND_ACTIONS`; `a` removed from `NEVER_HINTED`.
- Verb words: `take`, `amend`. No short forms (prefix table unchanged).
Out: `WorkingTreeRow` and the changes pane (`c commit` leads; `a` there unchanged, unhinted — keymap §2.4:265 lists it,
so the guard must see it hinted somewhere: HEAD's row is that place). Any other bar ranking.

## Signatures
```rust
// graft-ui::graph_screen::hints (context.rs)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowVerbs { pub take: bool, pub amend: bool }
pub enum HintContext { /* … */ BranchRow { current: bool, verbs: RowVerbs }, PlainRow { verbs: RowVerbs }, /* … */ }
// graft — the shared predicate (module per the scout; the `.` menu calls it too).
pub(crate) fn can_take(state: &AppState, row: usize) -> bool;
pub(crate) fn row_verbs(state: &AppState, row: usize) -> RowVerbs;
```
`every_context` yields all four `RowVerbs` per variant; `coverage_slot` stays exhaustive (new slots; `COVERAGE_SLOTS`
bumped).

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `a_commit_off_his_branch_hints_o_take` (red first): bar's second hint `o take`.
- [ ] `a_commit_on_his_branch_has_no_o` and `heads_row_has_no_o`: rows the menu dims → no `o`.
- [ ] `heads_row_hints_a_amend` (red first); `an_unpushed_older_row_hints_a_amend` (or, if no fact, NOTES says so and
      the row is the conductor's); `a_pushed_row_has_no_a`.
- [ ] `the_menu_and_the_bar_agree_on_o`: over every row of a fixture, `can_take` = menu slot enabled = `o` drawn.
- [ ] `every_bound_action_is_hinted_live_or_is_named_here` green with graph `o` in `BOUND_ACTIONS` and `a` out of
      `NEVER_HINTED` — **red first** with `o` added and the ranking not yet built (quote it).
- [ ] `every_key_the_bar_draws_is_answered`, `every_context_reaches_every_shape`, the floor and 80-column tests green.
- [ ] **Captures** (fast-tier tests): `docs/shots/m7/bar-take-row-187x44.txt` — **right:** cursor on a teammate's
      commit; bar reads `⏎ open  o take …`. `docs/shots/m7/bar-amend-row-187x44.txt` — **right:** cursor on HEAD; bar
      holds `c commit  a amend`. Colour unverified.
- [ ] MAP.md for graft-ui (`RowVerbs`, the two variants) and graft (`can_take`, `row_verbs`, `bound.rs`).
- [ ] Gates.

## Touches (check each; report how)
- **Every `PlainRow`/`BranchRow` match** (graft `view::rows`, graft-ui lists/catalogue/coverage): grep; list.
- **`Marked(..)`/`PaneFocused { row }` wrapping a row context:** the row's verbs ride inside; `PaneFocused` keeps its
  filter. Check: their tests.
- **The `.` menu:** reads `can_take`, behaviour unchanged. Check: menu tests and the `Cherry-pick (o)` capture
  byte-identical.
- **Bar width:** two more candidates; the shrink ladder drops by rank. Re-taken fast-tier bar captures listed in NOTES
  (e.g. `m5-hints-*`); heavy-only captures listed stale, not hand-edited.
- **Keymap:** none — `o` §2.3:205 and `a` §2.3:202 already bound; §2.1's `NEVER_HINTED` rule is now true.
- **Journal/undo:** none.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~45k. One context.
