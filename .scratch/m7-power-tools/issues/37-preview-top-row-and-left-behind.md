# 37-preview-top-row-and-left-behind — the preview shows its result when the target is the top row, and names every branch a rewrite leaves on old commits

Status: done
Source: usability walk `docs/reviews/m7-usability.md` M1 (ghosts above `view_start` are never drawn) and M6
(`rewrite/dependents.rs:56-60`: only a branch whose first-parent chain reaches the tip follows; the preview is silent
about the rest).
Build with: **Opus** — the window rule runs for every preview kind in drawn-row positions (labels included); a wrong
floor moves every m5–m7 preview picture, and a passing table test can't show the positions are the right ones.
Scout: done (`scouts/37.md`, `9133428c`). No second scout.
Needs: none.
Blocked by: none. **Wave 1, concurrent with 36 and 38** (file sets disjoint, see "File set").

## File set (exactly these; nothing else without a report)
- graft-ops: `src/preview/effects.rs`, `src/preview/rewrite.rs`, `src/preview/kinds/rebase.rs`,
  `src/rewrite/dependents.rs`, `src/preview.rs`, `src/preview/revert_to.rs`, `src/preview/kinds/refs.rs`,
  `tests/it/rewrite_preview.rs`, `MAP.md` (`preview` and `rewrite` sections only).
- graft src: `view/preview.rs`, `update/preview/line_rows.rs`, `update/preview/plan_lines.rs`,
  `view/rewrite_editor/right.rs`, and the `PreviewPlan` literal sites: `update/preview/tests.rs`,
  `update/preview/slice/tests.rs`, `update/picker/tests.rs`, `update/stash/tests.rs`, `update/amend_into/tests.rs`,
  `view/tests.rs`.
- graft tests/it: one or two **existing** files under `m7_walk/` (named in NOTES; no new file, so neither
  `m7_walk`'s mod root nor `tests/it/main.rs` is edited — 36 holds `main.rs`).
- graft `MAP.md` (`view::preview`, `update::preview::line_rows`, `view::rewrite_editor` entries only).
- `docs/shots/m7/`: the captures below + fast-tier preview captures this change moves (listed in NOTES).
- **Not edited:** `update/preview.rs`, `update/preview/answer.rs` (the fix is at draw time, below; no other ticket holds
  them, so if the build proves one is needed, edit it and say why in NOTES). **Never** (held by 36): 
  `update/preview/boxes.rs`, `update/state.rs`, `update/place*`, `update/commit_graph.rs`, graft-ui `*`,
  `tests/it/main.rs`, `box_owns_the_bar.rs`, `m6_stash_put_back.rs`. (By 38:) graft-ops `journal_list*`.
  A `PreviewPlan` literal in a file outside this set: commit at the seam, report `blocked: needs <file>`.

## Read first
- `scouts/37.md` (all).
- ADR-0047 §4; `docs/design/02-keymap.md` §2.3b.
- graft `MAP.md` `view::preview`, `update::preview` (line_rows, plan_lines), `view::rewrite_editor`; graft-ops `MAP.md`
  `preview` (`PreviewPlan`), `rewrite::dependents`.

## Scope
**1. Top row (M1), decided — at draw time, in `view/preview.rs`.** No label row is stored in state: `preview_entries`
(:34) inserts `label_entry` (:115) before each ghost run (:52-56); `window_start` (:90-101) takes
`max(at_start, at_selected − (visible−1))`, positions in **drawn entries** (labels included), `visible =
state.graph_rows()`. The `at_start` floor is why ghosts above `view_start` never show. `picture_start` replaces
`window_start`'s body; `PreviewState` and `answer.rs` are unchanged. Rule (all positions drawn-entry indices;
`label_at` = the **topmost** label entry):
- no ghosts (`label_at` None) or no selection, or `label_at ≥ at_start` → today's result, unchanged.
- `label_at < at_start` and `at_selected − label_at < visible` → start = `label_at`.
- does not fit → start = `at_selected + 1 − visible` (the target is the picture's last row, ghosts fill above), and
  the label is **pinned**: drawn on the picture's top row in place of the entry there. The label is never clipped.
The preview stays modal: `↑`/`PgUp` still swallowed (§2.3b). Every preview kind (merge, rebase, `a`, `o`, stash,
revert-to, put-back, rewrite) passes through it; only those whose label sits above `at_start` move.

**2. Left behind (M6), decided.** ADR-0047 §4 stands: only the line follows. The preview now says who doesn't:
- A **left-behind branch** = a local branch, not on the line (not HEAD's branch, not a dependent), whose tip contains a
  commit the operation replaces. Mid-chain forks (`feature-b` off `dd21dca`) and branches parked on a rewritten commit
  (a hand-made backup at the tip included) both count.
- Read: replaced commits are linear (a merge in range is refused, §5), so any such branch contains the **oldest**
  replaced commit: one `for-each-ref --contains <oldest> refs/heads/`, minus the line; `on` = `merge-base(branch,
  old top)` per remaining branch. Cap 8 named; the rest counted. Called beside `published` at
  `preview/rewrite.rs:77` and `preview/kinds/rebase.rs:80`.
- Box row, under the `branches:` rows (`plan_lines.rs:26-28`) and above `on origin:` (`published_row`, :47):
  - one: `left behind: feature-b stays on the old dd21dca`
  - more: `left behind: feature-b, feature-c stay on old commits` (`+N` past what fits)
  - rungs, longest first: full → names only (`left behind: feature-b, feature-c`) → `left behind: <first> +N`; fitted
    the way `line_rows.rs` fits its other `Vec<String>` ladders (~:115-122).
- **Rewrite editor AFTER also draws it** (decided): it doesn't call `ref_rows`, but draws the same facts itself
  (`view/rewrite_editor/right.rs:106-117`, :200-210 `built on yours · follows`; :212-220 `on origin:`). He confirms
  from there too (M5 promise), so the row goes between its follows rows and `on origin:`, from the same
  `left_behind_rungs`. **Stop condition:** if `right.rs`'s `on origin:` is not fed from a `PreviewPlan`, don't invent
  a read: ship the box part, report what feeds it, NOTES names the gap for the conductor.
- Nothing changes what `y` runs.
Out: making such a branch follow. **For the user (scope):** "a branch forked mid-chain follows too" (jj rebases every
descendant) is a change to ADR-0047 §4 and the engine — not M7. Conductor files a KNOWN_GAPS row for the next design
pass, prior-art study first.

## Signatures
```rust
// graft::view::preview — pure; replaces `window_start`'s body. Drawn-entry positions.
pub(crate) struct PictureStart { pub start: usize, pub label_pinned: bool }
pub(crate) fn picture_start(at_start: usize, label_at: Option<usize>, at_selected: Option<usize>, visible: usize) -> PictureStart;

// graft_ops::preview (effects.rs), beside `published`:
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftBehind { pub branch: RefName, pub on: Oid } // `on`: the old commit it stays on
pub struct PreviewPlan { /* … */ pub left_behind: Vec<LeftBehind>, pub left_behind_more: usize }
// graft_ops::rewrite::dependents — repo/git/stop: exactly the parameter types `dependents` (:74) takes.
// `line`: HEAD's branch + the dependents, as the caller already holds them.
pub(crate) fn left_behind(repo, git, oldest_replaced: Oid, old_top: Oid, line: &[RefName], stop) -> Result<(Vec<LeftBehind>, usize), FactsError>;

// graft::update::preview::line_rows — no `FittingLine` type exists; strings, as the file's other rows.
pub(crate) fn left_behind_rungs(left: &[LeftBehind], more: usize) -> Vec<String>; // longest first; empty when none
fn left_behind_row(/* same parameters as `published_row` (:127) */) -> Option<String>; // the box's fitted rung
```

## Acceptance (each box names its test at tick time; red first where marked)
- [x] `picture_start_table` (`view/preview.rs` `tests::picture_start_table`) (`view/preview.rs` test module or `view/tests.rs`): label below start; label above, fits;
      label above, does not fit (pinned, target last); no ghosts; no selection.
- [x] `a_rebase_onto_the_top_row_shows_after_rebase` (`tests/it/m7_walk/rebase_line.rs`; red first: run against HEAD's `view/preview.rs`, failed at the `after rebase` first-row assert): cursor on `main` at row 0, `feature-a` + `feature-b`
      on it, `r` → the drawn picture holds `after rebase`, every `○` row, and `main`'s row.
- [x] `a_preview_too_tall_pins_its_label` (`m7_walk/rebase_line.rs`, 30 commits at 120x40; red against HEAD's view too): a ghost block taller than the picture → target row last, `after rebase` on
      the top row.
- [x] `a_preview_with_room_is_unchanged` — verified by the existing `m7_walk::rebase_line::capture_rebase_line`: `rebase-line-preview-187x50.txt` byte-identical (no new test; `git diff` shows it unchanged): target mid-screen → start unchanged (today's acceptance shot
      `docs/shots/m7/rebase-line-preview-187x50.txt` byte-identical).
- [x] graft-ops `left_behind_names_a_branch_forked_mid_chain` (red first — red was a compile failure, the field did not exist; `tests/it/rewrite_preview.rs`): `feature-b` off
      `feature-a~1`, `a` into `feature-a~1` → `[LeftBehind { feature-b, on: <old feature-a~1> }]`; the line's branches
      never listed.
- [x] graft-ops `left_behind_names_a_branch_parked_on_a_replaced_commit`, `left_behind_is_empty_for_a_clean_line`,
      `left_behind_caps_at_eight` (10 branches → 8 named, `more` 2); plus `left_behind_on_a_rebase_names_the_fork` (the `r` call site). All `tests/it/rewrite_preview.rs`.
- [x] `left_behind_rungs_table` (`update/preview/tests.rs`): one, two, ten; each rung's exact words.
- [x] `the_box_says_who_stays_behind` (`m7_walk/rewrite_editor.rs`; red first: with `plan_lines`' row removed it failed): the walk's world (`a y` on `dd21dca`), the box draws
      `left behind: feature-b stays on the old dd21dca`; at 80 columns a shorter rung naming `feature-b` or `+N`.
- [x] `the_rewrite_editor_after_says_who_stays_behind` (`m7_walk/rewrite_editor.rs`; stop condition not hit: `on origin:` there is fed from `plan.published`) — or NOTES quotes the stop condition and what feeds `on origin:`.
- [x] **Captures** (re-taken by these tests; written by `a_rebase_onto_the_top_row_shows_after_rebase`, `the_box_says_who_stays_behind`, `the_rewrite_editor_after_says_who_stays_behind`):
      `docs/shots/m7/rebase-preview-target-top-187x44.txt` — **right:** `main` on the picture's first graph rows, the
      `after rebase` row and `○` rows for `feature-a`'s and `feature-b`'s new commits above it, box saying
      `feature-a → 3 new commits`, `feature-b → 4 new commits · built on yours · follows`.
      `docs/shots/m7/amend-into-left-behind-187x44.txt` — **right:** the box names `feature-a … 3 new commits` and a
      row `left behind: feature-b stays on the old dd21dca`.
      `docs/shots/m7/rewrite-editor-left-behind-187x44.txt` (if shipped) — **right:** AFTER side, the `left behind:`
      row between the `follows` rows and `on origin:`. Colour unverified.
- [x] MAP.md for graft-ops (`PreviewPlan` fields, `left_behind`), graft (`picture_start`, `left_behind_rungs`,
      rewrite editor row).
- [x] Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` (5024/5024), `cargo test --doc` — workspace-wide.

## Touches (check each; report how)
- **`PreviewPlan` literals** — scout list: graft-ops `preview.rs:121,177`, `preview/revert_to.rs:30`,
  `preview/kinds/refs.rs:93,104,141,195`; graft `update/picker/tests.rs:384`, `update/stash/tests.rs:161`,
  `update/preview/slice/tests.rs:47`, `update/preview/tests.rs:100,134,1151,1171`, `update/amend_into/tests.rs:212`,
  `view/tests.rs:3675`. Re-grep `PreviewPlan {`; any hit outside the file set → report, don't edit.
- **Preview cost:** one `for-each-ref` + ≤ 8 `merge-base` spawns per rewrite/rebase plan. Measure nothing; say so.
- **Every preview kind's picture:** every m5–m7 preview capture a fast-tier test writes is byte-identical or re-taken
  and listed with why. A capture also moved by 36: the conductor re-runs its test after both merge; never hand-merged.
- **`preview_rows_taken`** (`update/preview/rect.rs:13`) and `graph_rows` (`state.rs:478`): read, unchanged.
- **`preview_click`/`preview_box_rect`:** the box doesn't move; the rows behind it do. Check:
  `a_click_lands_on_the_button_the_widget_drew` green.
- **Journal/undo, keymap:** none. `y` runs the same op.
- **Docs:** design note / ADR-0047 untouched; the KNOWN_GAPS row is the conductor's.

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
`cargo test --doc`. Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Size
~60k. One context.

## NOTES (builder, 2026-10-01)

- Scout used (`scouts/37.md`). No compaction.
- **Files outside the set, and why:**
  - graft-ops `src/rewrite.rs` — one line: `pub(crate) use dependents::{left_behind, line_of};`. `rewrite::dependents` is a private module; the preview cannot reach `left_behind` otherwise.
  - graft `update/preview.rs` — one line: `pub(crate) use line_rows::{left_behind_rungs, onto_label};`. `line_rows` is private; `view/rewrite_editor/right.rs` needs the rungs (the ticket allowed it if the build proved it).
  - Walk tests went into `m7_walk/rebase_line.rs` and `m7_walk/rewrite_editor.rs` (two existing files). The box test lives in `rewrite_editor.rs`, beside the editor test, so they share one fixture.
- **Deviations:** `left_behind_row(plan, columns)` takes `columns` as well as `published_row`'s `plan`: picking a rung needs the width. In the rewrite editor's AFTER there is no width at that layer, so it draws the **longest** rung and lets the widget cut the tail. The names lead the row, so a cut does not reach them.
- **Fixture, not the walk's sha:** `dd21dca` came from the reviewer's own world. The fixture here is the same shape (`feature-b` forked off "feature a: part 1"), so its sha is `e662b82`. The tests assert against `feature-a~2`'s short id.
- **Oldest replaced commit:** for a rewrite, the first todo row; for `r`, the first step. The todo includes rows git may skip as unnecessary picks (28c `skip_unnecessary_picks`), so a branch parked on such a row would be named. That matches the ghosts the preview already draws for those rows.
- **Captures this change moves (re-taken, reviewed by diff):**
  - `docs/shots/m6/fix-preview-verb-120x40.txt`, `m6/set-aside-preview-120x40.txt`, `m6/set-aside-preview-80x24.txt`: the M1 fix itself. The ghosts and `after merge`/`after rebase` above the top row are now drawn; the graph's bottom rows give way.
  - `docs/shots/m5-ui-preview-scrolled-80x24.txt`, `m5-ui-preview-scrolled-160x48.txt`: the box gains `left behind: main, redesign stay on old commits` (rebasing `topic` onto `old-release` replays commits `main` and `redesign` hold), so the graph loses one row and the window slides by one to keep the target.
  - `set-aside-preview-*` is written by `m6_stash_put_back.rs`, which 36 holds. Only the capture was re-taken; the test file was not edited. If 36 moves it too, re-run that test after both merge.
- **Preview cost:** one `for-each-ref --contains` plus ≤ 8 `merge-base` per rebase/rewrite plan. Not measured.
- **Unverified:** colour of the new row and of the pinned label (captures carry no colour). Nobody has judged the new pictures by eye yet.
- For the conductor: the KNOWN_GAPS row about "a branch forked mid-chain follows too" (ADR-0047 §4 change) is yours, as the ticket says.
