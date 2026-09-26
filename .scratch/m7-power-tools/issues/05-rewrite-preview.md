# rewrite-preview — one preview names every branch that moves, the chained forecast, and a squash-merged branch's drop

Status: ready
Blocked by: 02-rewrite-engine
Crates:
- graft-ops (`preview`);
- graft-ui (the preview box: new rows);
- graft (only the view mapping from `PreviewPlan` to the box's view, and any exhaustive match on `ConflictForecast`).

Source: note "Pass 2a" R1 (AFTER block data), R2, R3; "Interfaces" (`graft-ops::preview`); KNOWN_GAPS :548 (chained
forecasts).
Build with: Opus. It extends an existing preview with reads whose truth an oracle checks (the forecast against a real
replay; the drop suggestion against a real squash-merge). The approach is decided here.
Scout: `scout` first (ADR-0035).
Needs: rewrite-engine (`RewriteParams`, `BranchLine`, `Operation::Rewrite`, `RebaseParams::drops`).

## Decided
- **`preview::plan` for `Operation::Rewrite` and `Operation::Rebase`:**
  - Ghosts for every rewritten commit.
  - One `RefMove` per branch in `line`, bottom first.
  - `head` = his branch, after.
  - `messages` = each resulting commit whose message Graft writes (reword or squash group), keyed by its ghost
    `Target`.
- **Chained forecast (:548):**
  - Replay the todo in order with `git merge-tree --write-tree --merge-base=<parent> <result-so-far> <commit>`.
  - Each step's result tree is written as a commit object (`commit-tree`, no ref) to be the next `<result-so-far>`.
  - The first conflicting step → `ConflictForecast::StopsAt{step, of, commit, paths}`. Nothing past it is checked; the
    words say so.
  - Git < 2.40, a failed spawn, or more than `FORECAST_CAP` steps → `NotChecked(reason)`, never a clean bill.
  - `stop` is checked between spawns.
- **Squash-merge suggestion (`suggest_drops`),** for `Rebase` only:
  - For the bottom branch of the line, compare `git patch-id` of `diff <split>..<tip>` with each commit on `onto`
    since the split. Cap: 200 commits; past the cap, `None` + `not checked` words.
  - A match → `AlreadyIn`, and the plan's todo gets `Drop` for those commits (`RebaseParams::drops`).
  - Never silent: the box always shows the row.
- **`published`:** every branch in the line whose upstream contains any rewritten commit.
- **Box rows** (graft-ui), words exactly as note R2/R3:
  - `branches:` one row per `RefMove` (`built on yours · follows` for dependents);
  - `already in main: … · dropped · R to keep them`;
  - `may stop at step i of n (<action> <sha7> <subject>): <paths> · not checked past it`;
  - `on origin: <b> · P will need force after`.
  - Cut order at narrow widths: paths, then subject, then `· not checked past it`. The branch name is never cut.

## Read first
- Note "Pass 2a" R1 (the AFTER block list), R2 (box mocks), R3; "Interfaces" `graft-ops::preview`.
- `docs/adr/0047-…` §1, §6; `docs/adr/0044-…` §1 (the preview confirm).
- `crates/graft-ops/MAP.md` `## preview` (`plan`, `PreviewPlan`, `ConflictForecast`, `FORECAST_CAP`, `replayed_commits`,
  `forecast_merge`); `crates/graft-ui/MAP.md` preview box entries; `crates/graft/MAP.md` `update::preview`.
- `docs/research/stacked-branches.md` §3 (squash-merge detection).

## Scout brief (≤ 1 page)
1. How `plan` builds ghosts and `ref_moves` for `Rebase` today, and where the per-commit forecast runs.
2. Every exhaustive match on `ConflictForecast` and every construction of `PreviewPlan` (`grep -rn` across crates).
3. How graft-ui draws `ref_moves` with more than one entry. Is there a precedent row?
4. `forecast_merge`: its args and whether it accepts a tree or needs a commit.
5. Existing captures of a rebase preview (`docs/shots/**/rebase*`, `**/preview*`) and the tests that write them.

## Signatures
```rust
pub struct AlreadyIn { pub branch: RefName, pub commits: Vec<Oid>, pub matches: Oid, pub subject: String }
pub enum ConflictForecast { /* existing */ StopsAt { step: usize, of: usize, commit: Oid, paths: Vec<PathBuf> } }
// PreviewPlan gains: pub already_in: Option<AlreadyIn>, pub published: Vec<RefName>, pub messages: Vec<(Target, String)>
pub fn suggest_drops(repo: &Repo, git: &Git, line: &BranchLine, onto: Oid, stop: &dyn Stop)
    -> Result<Option<AlreadyIn>, FactsError>;
```

## Acceptance
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
- [ ] graft-ui (`preview_box` tests):
  - `the_box_draws_one_row_per_branch`;
  - `already_in_row_says_dropped_and_r`;
  - `a_stop_forecast_names_the_step`;
  - `cut_order_never_cuts_a_branch_name` (80 columns).
- [ ] **Captures** (rule 6; colour **unverified**, rule 5), through `check_shot`:
  - `docs/shots/m7/rebase-line-preview-187x50.txt`. **Right:** three `branches:` rows, `feature-b` with
    `built on yours · follows`; `no conflicts expected`; `y` on the bar.
  - `docs/shots/m7/rebase-squash-merged-preview-120x40.txt`. **Right:** `already in main: feature-a's 3 commits match
    <sha> "<subject>" · dropped · R to keep them`.
  - `docs/shots/m7/rebase-stop-forecast-80x24.txt`. **Right:** `may stop at step 3 of 5`, with the branch name whole.
- [ ] MAP.md for graft-ops and graft-ui. KNOWN_GAPS :548 closed by the conductor (NOTES names the test).

## Touches (check each, report how)
- **Every other preview kind:** unchanged. Check: all existing preview tests + captures byte-identical (`check_shot`).
- **The `r r` / `m m` confirm path:** unchanged. Check: the m5/m6 preview walks green.
- **graft's exhaustive matches** on `ConflictForecast` (scout Q2): each site gets the new arm. Listed in NOTES.
- **Journal / undo:** none (read only).
- **Keymap:** none. The `R to keep them` words name a key `rewrite-editor` binds. Until then the words are true only
  once that lands: note it for the conductor's sequencing.
- **Contact sheet:** the m7 sheet is started (`scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`), with
  captions rows for the three captures.

## Rules / Gates
As rewrite-engine. Worktree only; `CARGO_TARGET_DIR` in the worktree; the four gates workspace-wide.

## Size
Medium-large (~60k). Split if needed:
- **-a:** ops (forecast, drops, published, oracle);
- **-b:** ui rows and captures.
