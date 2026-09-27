# rewrite-preview — one preview names every branch that moves, the chained forecast, and a squash-merged branch's drop

Status: ready
Blocked by: 02-rewrite-engine, 02b-rewrite-engine-b
Crates:
- graft-ops (`preview`, `operation/history` for `RebaseParams`);
- graft-git (`history_reads`: a merge-tree read that returns the result tree — only if the red test below says
  `forecast_merge` can't chain; see "Forecast input" under Decided);
- graft (`update/preview.rs`: **the box's row text is built here**, `plan_lines` :946, `ref_line` :1026 (scout 05 Q3);
  the view mapping; every exhaustive match on `ConflictForecast`; every `PreviewPlan`/`RebaseParams` literal);
- graft-ui only if `preview_box.rs::fitting_line` (:352) needs a new cut step for the ladder below; otherwise untouched.

Files: see `parallel-plan.md` line for 05 (amended 2026-09-27 after scout 05).

Source: note "Pass 2a" R1 (AFTER block data), R2, R3; "Interfaces" (`graft-ops::preview`); KNOWN_GAPS :548 (chained
forecasts). Scout: `.scratch/m7-power-tools/scouts/05.md` (done, HEAD 80330a41).
Build with: Opus. It extends an existing preview with reads whose truth an oracle checks (the forecast against a real
replay; the drop suggestion against a real squash-merge). The approach is decided here.
Needs: rewrite-engine (`RewriteParams`, `BranchLine`, `Operation::Rewrite`) — 02a + 02b. **`RebaseParams::drops` (and `line`) are added by this ticket** (conductor, 2026-09-26: 02 couldn't without graft).

## Decided
- **`preview::plan` for `Operation::Rewrite` and `Operation::Rebase`:**
  - `Operation::Rewrite` has no arm today (falls to `NotPreviewable`, `preview.rs:406`): add it.
  - Ghosts for every rewritten commit.
  - One `RefMove` per branch in `line`, bottom first (today `kinds::rebase` makes exactly one, `kinds.rs:153-157`).
  - `head` = his branch, after.
  - `messages` = each resulting commit whose message Graft writes (reword or squash group), keyed by its ghost
    `Target`.
- **Chained forecast (:548):** replaces today's per-commit `replay_forecast` (`kinds.rs:177-189`, `ours = onto`,
  never chained) for these two kinds only.
  - Replay the todo in order with `git merge-tree --write-tree --merge-base=<parent> <result-so-far> <commit>`.
  - Each step's result tree is written as a commit object (`commit-tree`, no ref) to be the next `<result-so-far>`.
  - The first conflicting step → `ConflictForecast::StopsAt{step, of, commit, paths}`. Nothing past it is checked; the
    words say so.
  - Git < 2.40, a failed spawn, or more than `FORECAST_CAP` steps → `NotChecked(reason)`, never a clean bill.
  - `stop` is checked between spawns.
- **Forecast input — settle by test first, before any chaining code.** `forecast_merge` (`graft-git/src/
  history_reads.rs:341`) takes plain `Oid`s and, on `Clean` (:384), returns no tree (scout Q4). Write two red tests in
  graft-git first:
  - `forecast_merge_accepts_a_commit_tree_result_as_ours` — a `commit-tree` commit as `ours` gives the same answer as
    the equivalent real commit;
  - `a_clean_merge_tree_returns_its_tree` — the chaining read returns the result tree, and that tree equals the tree git
    writes for the same real merge.
  - If `forecast_merge` already does both: use it, no graft-git change, say so in NOTES with the test names.
  - Otherwise add, in graft-git, **a new read beside it** (do not change `Forecast` or `forecast_merge`: put_back and
    stash call it — `put_back.rs:12`, `stash.rs:19`):
    ```rust
    pub enum MergeStep { Clean { tree: Oid }, Conflicts { paths: Vec<PathBuf> }, NotCheckable(String) }
    pub fn merge_tree_step(repo: &Repo, git: &Git, base: Oid, ours: Oid, theirs: Oid, stop: &dyn Stop)
        -> Result<MergeStep, /* forecast_merge's error type */>;
    pub fn commit_tree(repo: &Repo, git: &Git, tree: Oid, parent: Oid, stop: &dyn Stop)
        -> Result<Oid, /* same */>;   // only if graft-git has no commit-tree wrapper (grep first)
    ```
    Same ≥ 2.40 gate as `forecast_merge` (:349-354).
- **Squash-merge suggestion (`suggest_drops`),** for `Rebase` only:
  - For the bottom branch of the line, compare `git patch-id` of `diff <split>..<tip>` with each commit on `onto`
    since the split. Cap: 200 commits; past the cap, `None` + `not checked` words.
  - A match → `AlreadyIn`, and the plan's todo gets `Drop` for those commits (`RebaseParams::drops`).
  - Never silent: the box always shows the row.
- **`published`:** every branch in the line whose upstream contains any rewritten commit.
- **Box rows — built in graft `update/preview.rs` `plan_lines`**, words exactly as note R2/R3, except the keep key:
  - `branches:` label, then one row per `RefMove` through the existing `ref_line` (`built on yours · follows` for
    dependents);
  - `already in main: … · dropped` — **no `· R to keep them` here.** The key is decided (keymap §2.3b `R`, below) but
    needs the rewrite editor (08); ticket `rebase-preview-keeps-drops` appends the words and binds it. A row naming an
    unbound key is false.
  - `may stop at step i of n (<action> <sha7> <subject>): <paths> · not checked past it`;
  - `on origin: <b> · P will need force after`.
  - Cut order at narrow widths: paths, then subject, then `· not checked past it`. The branch name is never cut. Use
    graft-ui's `fitting_line` ladder (:352) if it fits; extend it only if it can't express this order.

## Read first
- Scout `.scratch/m7-power-tools/scouts/05.md` (whole, one page).
- Note "Pass 2a" R1 (the AFTER block list), R2 (box mocks), R3; "Interfaces" `graft-ops::preview`.
- `docs/adr/0047-…` §1, §6; `docs/adr/0044-…` §1 (the preview confirm).
- `crates/graft-ops/MAP.md` `## preview`; `crates/graft/MAP.md` `update::preview`; `crates/graft-git/MAP.md`
  `history_reads` (`forecast_merge`); `crates/graft-ui/MAP.md` `preview_box` (`fitting_line` only).
- `docs/research/stacked-branches.md` §3 (squash-merge detection).

## Signatures
```rust
// graft-ops::preview
pub struct AlreadyIn { pub branch: RefName, pub commits: Vec<Oid>, pub matches: Oid, pub subject: String }
pub enum ConflictForecast { /* existing */ StopsAt { step: usize, of: usize, commit: Oid, paths: Vec<PathBuf> } }
// PreviewPlan gains: pub already_in: Option<AlreadyIn>, pub published: Vec<RefName>, pub messages: Vec<(Target, String)>
pub fn suggest_drops(repo: &Repo, git: &Git, line: &BranchLine, onto: Oid, stop: &dyn Stop)
    -> Result<Option<AlreadyIn>, FactsError>;
// graft-git::history_reads — only if the red tests require it (above): MergeStep, merge_tree_step, commit_tree
```

## Acceptance
- [ ] graft-git first (red, then settled): `forecast_merge_accepts_a_commit_tree_result_as_ours`,
      `a_clean_merge_tree_returns_its_tree`. NOTES says which way it settled.
- [ ] graft-ops (`tests/it/rewrite_preview.rs`):
  - `a_three_branch_line_moves_three_refs`;
  - `the_forecast_chains_through_earlier_picks` (red first: a conflict that exists only after step 2 is applied);
  - `the_forecast_stops_at_the_first_conflict`;
  - `a_squash_merged_bottom_branch_is_suggested_dropped`;
  - `no_match_suggests_nothing`;
  - `past_the_cap_is_not_checked`;
  - `published_names_branches_on_their_upstream`;
  - `squash_message_is_in_the_plan`;
  - `the_preview_writes_no_ref_index_or_file` (as 86's: index bytes, `for-each-ref`, `status --porcelain=v2
    --no-optional-locks`; objects may be written, KNOWN_GAPS :505);
  - `the_preview_honours_stop`;
  - `an_old_git_leaves_it_not_checked`.
- [ ] **Oracle** `the_chained_forecast_agrees_with_git`, on four rows: clean, a stop at step 1, a stop at step 3, and a
      stop that exists only chained. `plan`, then the real rewrite in a copy: `StopsAt{step}` ⇔ git stops at that
      step with those paths.
- [ ] graft (`update/preview/tests.rs`, where `plan_lines` is tested):
  - `the_box_draws_one_row_per_branch`;
  - `already_in_row_says_dropped` (and asserts no `R` in the row);
  - `a_stop_forecast_names_the_step`;
  - `cut_order_never_cuts_a_branch_name` (80 columns).
- [ ] **Captures** (rule 6; colour **unverified**, rule 5), through `check_shot`:
  - `docs/shots/m7/rebase-line-preview-187x50.txt`. **Right:** a `branches:` label and three rows under it,
    `feature-b` with `built on yours · follows`; `no conflicts expected`; `y` on the bar.
  - `docs/shots/m7/rebase-squash-merged-preview-120x40.txt`. **Right:** `already in main: feature-a's 3 commits match
    <sha> "<subject>" · dropped`, nothing after `dropped`.
  - `docs/shots/m7/rebase-stop-forecast-80x24.txt`. **Right:** `may stop at step 3 of 5`, with the branch name whole.
- [ ] MAP.md for graft-ops and graft (graft-git and graft-ui only if touched). KNOWN_GAPS :548 closed by the conductor
      (NOTES names the test).

## Touches (check each, report how)
- **Every other preview kind:** unchanged. Check: all existing preview tests + captures byte-identical (`check_shot`),
  including `m5-01-preview-rebase-{80x24,160x48}` (`m5_walk/preview.rs:80`) and `m5-ui-preview-rebase-{80x24,160x48}`
  (`tests/it/preview.rs:261,295`) — a plain one-branch rebase must draw as today, or NOTES lists each changed line and why.
- **The `r r` / `m m` confirm path:** unchanged. Check: the m5/m6 preview walks green.
- **`ConflictForecast` exhaustive matches:** `graft/src/update/preview.rs:1131-1150` `conflict_line`; graft-ops
  `preview.rs:607` `fold`; test `graft-ops/tests/it/stash_preview.rs:542-552`. Each gets the new arm.
- **`PreviewPlan` literals:** scout Q2's list (graft-ops preview.rs :387 :455 :746, kinds.rs :422 :433 :470 :526; graft
  tests stash/tests.rs:161, preview/tests.rs :99 :130 :1084 :1104, preview/slice/tests.rs:47, view/tests.rs:3508).
- **`forecast_merge` callers (put_back, stash):** unchanged — the new read sits beside it. Check: their tests green.
- **Journal / undo:** none (read only).
- **Keymap:** none bound here. §2.3b gains `R` (row below), bound by `rebase-preview-keeps-drops`.
- **Contact sheet:** the m7 sheet is started (`scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`), with
  captions rows for the three captures.

## Keymap row (conductor applies to `docs/design/02-keymap.md` §2.3b before `rebase-preview-keeps-drops` builds)
| `R` | In a rebase preview showing `already in main: … · dropped`: open the rewrite editor (§2.7) on that rebase, those commits pre-marked `d`; `p` keeps one, `y` starts, `Esc` back to the graph. Same meaning as §2.3 `R` (opens the rewrite editor, §2.1 UPPERCASE opens a place). Not offered in any other preview (planner, 2026-09-27, scout 05) |

## Rules / Gates
As rewrite-engine. Worktree only; `CARGO_TARGET_DIR` in the worktree; the four gates workspace-wide.

## Size
Medium-large (~60k). Split if needed:
- **-a:** graft-git red tests (+ read if needed), ops (forecast, drops, published, oracle);
- **-b:** graft rows and captures.
