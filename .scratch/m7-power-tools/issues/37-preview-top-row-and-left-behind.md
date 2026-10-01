# 37-preview-top-row-and-left-behind — the preview shows its result when the target is the top row, and names every branch a rewrite leaves on old commits

Status: ready
Source: usability walk `docs/reviews/m7-usability.md` M1 (`answer.rs:127-131`: ghosts above `view_start` are never
drawn) and M6 (`rewrite/dependents.rs:56-60`: only a branch whose first-parent chain reaches the tip follows; the
preview is silent about the rest).
Build with: **blank — scout first.** Suggested after the scout: Sonnet (two pure functions and one box row, each
decided below).
Scout: **yes, one, ≤1 page**: `preview_produced` and how `view_start`, `selected_row`, `ghost_index` and the label row
are set (`update/preview/answer.rs`); where the picture is drawn from `view_start` (`view::preview`, `label_entry`); how
many graph rows the picture gets under the box (`preview_rows_taken`); where `PreviewPlan::published` is filled in
graft-ops; every surface that draws `line_rows::ref_rows` (`· built on yours · follows`) — the rebase/`a` preview box,
and whether the rewrite editor's AFTER side draws it too.
Needs: none.
Blocked by: none
Crates / files (wave 1; disjoint from the other fix groups):
- graft-ops: `preview/effects.rs` (`PreviewPlan`), the file that fills `published`, `rewrite/dependents.rs`.
- graft: `update/preview/answer.rs`, `update/preview/line_rows.rs` (or its file), `view/preview*`, their tests.
  **Not** `preview/boxes.rs` (36-offer-keys-and-daily-wording).
- graft-ui: only if the scout finds the rewrite editor's AFTER draws ref rows itself — then its one file, named.

## Scope
**1. Top row (M1), decided.** Every preview (merge, rebase, `a`, `o`, rewrite): when the outcome label row
(`outcome_label`, drawn above the first ghost) lies above `view_start`, the picture starts there instead. Rule, one pure
fn:
- label row ≥ `view_start` → unchanged.
- label row < `view_start`, and label row … `selected_row` fits in the picture's rows → start = label row.
- does not fit → start = `selected_row + 1 − visible` (the target row is the picture's last row, ghosts fill above it),
  and the label is **pinned**: drawn on the picture's top row in place of the ghost there. The label is never clipped.
- no ghosts (a refusal, an undrawable box) → unchanged.
The preview stays modal: `↑`/`PgUp` still swallowed (§2.3b).

**2. Left behind (M6), decided.** ADR-0047 §4 stands: only the line follows. The preview now says who doesn't:
- A **left-behind branch** = a local branch, not on the line (not HEAD's branch, not a dependent), whose tip contains a
  commit the operation replaces. Mid-chain forks (`feature-b` off `dd21dca`) and branches parked on a rewritten commit
  (a hand-made backup at the tip included) both count — each truly stays on the old copy.
- Read: commits replaced are linear (a merge in range is refused, §5), so any such branch contains the **oldest**
  replaced commit: one `for-each-ref --contains <oldest> refs/heads/`, minus the line; `on` = `merge-base(branch, top)`
  per remaining branch. Cap 8 named; the rest counted.
- Box row, under the `branches:` row and above `on origin:`:
  - one: `left behind: feature-b stays on the old dd21dca`
  - more: `left behind: feature-b, feature-c stay on old commits` (`+N` past what fits)
  - fitting ladder: full → names only → `left behind: <first> +N`.
- Drawn by every surface that draws `ref_rows` (scout lists them). Nothing changes what `y` runs.
Out: making such a branch follow. **For the user (scope):** "a branch forked mid-chain follows too" (jj rebases every
descendant) is a change to ADR-0047 §4 and the engine — not M7. Conductor files a KNOWN_GAPS row for the next design
pass, prior-art study first.

## Signatures
```rust
// graft::update::preview — pure.
pub(crate) struct PictureStart { pub view_start: usize, pub label_pinned: bool }
pub(crate) fn picture_start(view_start: usize, label_row: Option<usize>, selected_row: Option<usize>, visible: usize) -> PictureStart;
// PreviewState gains `label_pinned: bool`; view::preview draws the label on the top row when it is set.

// graft_ops::preview (effects.rs), beside `published`:
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftBehind { pub branch: RefName, pub on: Oid } // `on`: the old commit it stays on (merge-base with the top)
pub struct PreviewPlan { /* … */ pub left_behind: Vec<LeftBehind>, pub left_behind_more: usize }
// graft_ops::rewrite::dependents — repo/git/stop: exactly the parameter types `dependents` takes.
pub(crate) fn left_behind(repo, git, oldest_replaced: Oid, top: Oid, line: &BranchLine, stop) -> Result<(Vec<LeftBehind>, usize), FactsError>;
// graft::update::preview::line_rows
fn left_behind_row(plan: &PreviewPlan) -> Option<FittingLine>; // the line type `published_row` returns
```

## Acceptance (each box names its test at tick time; red first where marked)
- [ ] `picture_start` table test `picture_start_table`: label below start; label above, fits; label above, does not
      fit (pinned, target last); no ghosts.
- [ ] `a_rebase_onto_the_top_row_shows_after_rebase` (red first): cursor on `main` at row 0, `feature-a` +
      `feature-b` on it, `r` → the drawn picture holds `after rebase` and every `○` row, and `main`'s row.
- [ ] `a_preview_too_tall_pins_its_label`: a ghost block taller than the picture → target row last, `after rebase` on
      the top row.
- [ ] `a_preview_with_room_is_unchanged`: target mid-screen → `view_start` unchanged (today's acceptance shot
      `docs/shots/m7/rebase-line-preview-187x50.txt` byte-identical).
- [ ] graft-ops `left_behind_names_a_branch_forked_mid_chain` (red first): `feature-b` off `feature-a~1`, `a` into
      `feature-a~1` → `[LeftBehind { feature-b, on: <old feature-a~1> }]`; the line's branches never listed.
- [ ] graft-ops `left_behind_names_a_branch_parked_on_a_replaced_commit` and `left_behind_is_empty_for_a_clean_line`.
- [ ] graft-ops `left_behind_caps_at_eight`: 10 branches → 8 named, `more` 2.
- [ ] `the_box_says_who_stays_behind` (red first): the walk's world (`a y` on `dd21dca`), the box draws
      `left behind: feature-b stays on the old dd21dca`; the fitting ladder at 80 columns names `feature-b` or `+N`.
- [ ] **Captures** (re-taken by these tests):
      `docs/shots/m7/rebase-preview-target-top-187x44.txt` — **right:** `main` on the picture's first graph rows, the
      `after rebase` row and `○` rows for `feature-a`'s and `feature-b`'s new commits above it, box saying
      `feature-a → 3 new commits`, `feature-b → 4 new commits · built on yours · follows`.
      `docs/shots/m7/amend-into-left-behind-187x44.txt` — **right:** the box names `feature-a … 3 new commits` and a
      row `left behind: feature-b stays on the old dd21dca`. Colour unverified.
- [ ] MAP.md for graft-ops (`PreviewPlan` fields, `left_behind`), graft (`picture_start`, `label_pinned`,
      `left_behind_row`).
- [ ] Gates.

## Touches (check each; report how)
- **`PreviewPlan` constructors and matches:** every `PreviewPlan { .. }` literal (ops and app tests) gains the fields.
  Grep; list.
- **Preview cost:** one `for-each-ref` + ≤ 8 `merge-base` spawns per rewrite/rebase plan. Measure nothing; say so.
- **Every preview kind's picture** (merge, rebase, `a`, `o`, stash, revert-to, J's put-back): `picture_start` runs for
  each; only those whose label sits above `view_start` move. Check: every m5–m7 preview capture a fast-tier test writes
  is byte-identical or re-taken and listed with why.
- **`preview_click`/`preview_box_rect`:** the box doesn't move; the rows behind it do. Check:
  `a_click_lands_on_the_button_the_widget_drew` green.
- **Rewrite editor AFTER:** draws the row iff it draws `ref_rows` (scout).
- **Journal/undo, keymap:** none. `y` runs the same op (hard choice 3).
- **Docs:** design note / ADR-0047 untouched; the KNOWN_GAPS row is the conductor's.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~55k. One context.
