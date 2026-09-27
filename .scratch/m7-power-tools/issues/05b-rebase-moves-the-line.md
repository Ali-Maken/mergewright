# 05b rebase-moves-the-line — `r` moves his whole line and drops what main already has, as its preview says

Status: done (builder, 2026-09-27; compacted once, see NOTES)
Blocked by: 05-rewrite-preview (its partial, worktree commit b1f1c26b, merges first)
Split from: 05 (spec-only pass, planner, 2026-09-27). 05 keeps everything ticked in its Acceptance; this ticket is 05's
three open items, the engine switch they were blocked on, and the fork question.
Build with: **Opus**. The approach is decided below. The truth of every change is checked by an oracle (a real `rebase -i`
in a copy) or a byte check on the refs, not by reading the diff. Not Fable: it adds no interface other tickets build on.
02's signature (Fable) fixed the engine; this ticket only stops bypassing it.
Needs:
- 05: `preview::suggest_drops` → `Option<DropSuggestion>` (`Drop(AlreadyIn)` | `NotChecked(why)`),
  `PreviewPlan::{already_in, published, messages}`, `ConflictForecast::StopsAt`, `preview/chain.rs` (the chained
  forecast), graft `update/preview/line_rows.rs`;
- 02/02b: `rewrite::BranchLine`, `rewrite::dependents`/`line_of` (`rewrite/dependents.rs`), `apply::engine::prepare`,
  `Refusal::{ForkedDependents, DependentElsewhere, GitTooOld, Moved}`.
Scout: none new. 05's scout (`.scratch/m7-power-tools/scouts/05.md`) and 05's NOTES name every line this touches
(`apply/engine.rs:158` switch point, `operation.rs:1290` `tips()`). If a line has moved, re-find it by the name in the
comment (`SWITCH POINT — ticket 05`).
Crates: graft-ops, graft.

Files (ADR-0051: a file outside this list → stop, report `blocked: needs <file>`):
- graft-ops src: `apply/engine.rs`; `rewrite.rs`; `rewrite/dependents.rs`; `operation.rs`; `operation/history.rs`;
  `operation/history/tests.rs`; `operation/tests.rs`; `preview.rs`; `preview/kinds.rs`; `preview/chain.rs`;
  `preview/drops.rs`; `preview/tests.rs`; `preview/put_back.rs` (only if the put-back forecast replays the rebase itself
  and not chain's result; say which in NOTES).
- graft-ops tests (`crates/graft-ops/tests/it/`): `rewrite_run.rs`, `rewrite_preview.rs`, and the files that hold a
  `RebaseParams` literal: `in_the_way/replay.rs`, `put_back_index.rs`, `history_ops.rs`, `set_aside.rs`,
  `put_back_forecast.rs`, `conflict_fixtures.rs`, `stash_preview.rs`.
- graft src: `jobs/preview.rs`; `update/{preview.rs, preview/tests.rs, preview/line_rows.rs, preview/slice/tests.rs,
  stash/tests.rs, operate.rs, drag.rs, drag/tests.rs, context_menu/verbs.rs, context_menu/verbs/tests.rs,
  remote/tests.rs}`; `view/{preview.rs, tests.rs}`.
- graft tests: `crates/graft/tests/it/m7_walk/rebase_line.rs` (**new**; the conductor adds it in the seam commit with its
  `mod` line in `m7_walk.rs`). Run, not edited: `m5_walk/preview.rs`, `tests/it/preview.rs`.
- docs: `docs/shots/m7/{rebase-line-preview-187x50, rebase-squash-merged-preview-120x40, rebase-refused-fork-120x40}.txt`;
  `docs/shots/m7/m7-contact-sheet-captions.tsv` (append rows); MAP.md (graft-ops, graft).
- Not held: JOBS, HINTS, `view/rows.rs`, graft-fixtures, graft-git, graft-ui. Needing any → blocked.

**Order against the other holders (parallel-plan.md terms).** 05b takes 05's place in the plan, so it goes **first**
among every ticket that shares a file with it:
- `operation.rs`: **11a** (wave 3) starts only after 05b merges, then **03b** (wave 4) after 11a, as the plan already has
  them. This is the plan's own reason for 11a in wave 3: "released by 05 and 10b". Read "05" as "05b".
- `operation.rs`, `operation/history.rs`, `preview.rs`, `preview/kinds.rs`, graft `update/preview.rs`: **10b** waits
  for 05b.
- graft `update/preview.rs`, `preview/tests.rs`: **09** waits for 05b. Its `Blocked by: 05` reads 05b. So do **24**
  and **15**, as before.
- `rewrite/dependents.rs`: **30e** (wave 6, doc comments), unaffected in order.
- 05b may run beside **16** and **13b** (wave 2): none of its files is on their lists (16 holds JOBS, `view/rows.rs`,
  `update/state.rs`, `update/tests.rs`; 05b holds none of them).

## The fork question: decided, refuse
`r` when two branches both build on his (a fork): **refused, in words, before `y`**. His branch does not move alone.
- ADR-0047 §4 (`docs/adr/0047-…md:68-70`): "The dependents found must form one line … **A fork is refused in words**,
  with both branches named … Graft never guesses which one follows."
- ADR-0047 §6 (:83-86): "**Every rebase goes through this engine** … `r` builds a todo of `pick`s." So §4 binds `r`.
- Design note, hard-to-reverse choice 3 (`docs/design-notes/m7-power-tools.md:301`): "one line of branches per rewrite.
  A fork is refused in words, with both named."
- `r_with_two_branches_on_top_is_not_refused_after_y` pinned a **stopgap**, not a decision. See 02's review fix,
  Major 1 (`02-rewrite-engine.md:422-433`): `r` moved his branch alone "until 05", because the preview could not yet
  show the line. 02b had already made `r` refuse a fork (02 NOTES -b item 2, :373-374). This ticket restores that.
- Words: `Refusal::ForkedDependents`'s own: `feature-b and feature-c are both built on yours · Graft moves one line`.
  - A dependent in another worktree gets `DependentElsewhere`'s words.
  - git < 2.38 with dependents gets `GitTooOld`'s words.
  - Each is the same refusal `R` gives, in the same order as `rewrite::plan`.
- Cost (for the conductor, not a question): in that repo shape, `r` in Graft is closed to him. In M6 it moved his branch
  alone. If he hits it, it goes in `docs/field-notes.md` / "Times I left" and the next design pass takes it.

## Decided
- **`RebaseParams` gains `line: BranchLine` and `drops: Vec<Oid>`.**
  - `RebaseParams::new(branch, onto, onto_tip)` gives `line` = his branch alone and `drops` empty.
  - Every graft construction site calls `new`. Only graft-ops tests may write the literal.
- **The line and the drops are filled in graft-ops, once, when the preview is planned.** They are never filled by graft
  and never re-derived at `y`.
  - The new `preview::plan_as_run` is the preview job's entry point for every subject that is an `Operation`.
  - For a `Rebase`, in order:
    1. `line_of` (the refusals above → `PreviewError::Refused`);
    2. `suggest_drops` over that line;
    3. write `line`, and `drops` = the `Drop(AlreadyIn)`'s commits (empty for `None`/`NotChecked`);
    4. then `plan` over the **completed** op, and set `plan.already_in` from step 2.
  - For every other kind: `op` back unchanged, plus `plan`.
  - Why this and not a new job: the preview job already reads git. `confirm_preview` already moves the op the picture
    was drawn from (graft MAP `update::preview`, hard choice 3). So the op he confirms is the op the preview completed.
    No JOBS change.
- **graft carries the completed op.**
  - `Planned` gains `runs`.
  - `preview_produced` moves `runs` into `PreviewSubject::Operation { op }`.
  - `confirm_preview` is unchanged: it moves that op across.
  - A `Refused` answer closes the box and sets the status, as today (m6-conflicts/67). That is where the fork refusal
    lands.
- **`plan` over a `Rebase` is pure in its params:**
  - one `RefMove` per branch of `params.line`, bottom first;
  - ghosts for his picks plus the dependents' picks, with `params.drops` left out;
  - the chained forecast over that same todo, drops skipped; `StopsAt{step, of}` counts as git's pick-like rows do
    (`rewrite::progress`, graft-ops MAP :272);
  - `published` over every branch of the line.
- **The command line** (`Operation::plan`, `operation/history.rs`): `git rebase -i … <onto> <top short>` when the line
  has 2+ branches (02b's `<top>` rule). A one-branch line is byte-identical to today. This closes 02b NOTES item 2's
  mismatch between the preview and the Command Log.
- **`Operation::tips()` for `Rebase`** pins every branch of `params.line`, so a branch moved between preview and `y` is
  refused `Moved { name }`, as today.
- **The engine** (`apply/engine.rs` `prepare`, the switch point):
  - `Rebase` runs `params.line`: the top of the line, `update-ref` rows, go home. This is the path `Rewrite` already
    takes.
  - Rows are `rewrite::rebase_rows`, each commit in `params.drops` marked `Drop`.
  - Before anything is written:
    - `line_of` is read again. A line whose names differ from `params.line` → `Refusal::Moved { name: <the first branch
      that differs> }`. A new branch built on his since the preview is not left behind silently (hard choice 3: the
      picture and the act are one act).
    - A `drops` oid not among the rows → `Moved { name: <his branch> }`.
  - Delete the switch-point comment.
- **A line whose every row is dropped** (his squash-merged branch, nothing built on it):
  - First check git in a throwaway script: does `rebase -i` with an all-`drop` todo move the branch to `onto`, or refuse
    ("nothing to do")? Quote the output in NOTES.
  - If it moves: run it. The branch lands on `onto` (all of it is in main).
  - If it refuses: `plan_as_run` refuses `BadTodo(EverythingDropped)`, with R1's words.
  - Either answer is safe; NOTES says which.
- **Default-branch override:** out of scope. `plan_as_run` passes `None`, as `apply::Context::open` does today (02b's
  open seam, NOTES -b item 3).

## Read first
- This ticket. Then 05's `Decided` and `NOTES` (worktree `.claude/worktrees/agent-aea006994197cc683`,
  `.scratch/m7-power-tools/issues/05-rewrite-preview.md`), and scout 05.
- `.scratch/m7-power-tools/issues/02-rewrite-engine.md` :368-374 (item 2) and :418-435 (review fix Major 1).
- ADR-0047 §2–§6. Note R3 (`docs/design-notes/m7-power-tools.md:579-598`).
- `crates/graft-ops/MAP.md` `## apply` (`apply::engine`), `## preview`, `## rewrite`; `crates/graft/MAP.md`
  `## update::preview`, `## jobs` (`preview`).

## Signatures
```rust
// graft-ops::operation (operation/history.rs)
pub struct RebaseParams { pub branch: Tip, pub onto: Source, pub onto_tip: Oid,
                          pub line: BranchLine,   // his branch first, bottom up; every branch `y` moves
                          pub drops: Vec<Oid> }   // rows the todo marks `drop` (a squash-merged bottom branch)
impl RebaseParams { pub fn new(branch: Tip, onto: Source, onto_tip: Oid) -> Self; }  // line = [branch], drops = []

// graft-ops::preview
/// The operation as `y` will run it, and its picture. Rebase: line + drops found and written here; else `op` unchanged.
pub fn plan_as_run(repo: &Repo, git: &Git, op: Operation, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<(Operation, PreviewPlan), PreviewError>;

// graft-ops::rewrite — visibility only: `line_of` pub(super) → pub(crate), re-exported for `preview`.

// graft::update::preview
pub struct Planned { pub plan: PreviewPlan, pub anchors: Vec<Commit>,
                     pub runs: Option<Box<Operation>> }  // plan_as_run's op; None for RevertTo
```
`graft::jobs::preview::run_preview` calls `plan_as_run` where it called `plan`. It calls nothing else new.

## Acceptance (each box names its test at tick time; red quoted in NOTES for every new test)
- [x] graft-ops, `tests/it/rewrite_preview.rs`, red first — each test below by name, all in that file:
  - `r_plans_every_branch_of_its_line`: `plan_as_run` on `RebaseParams::new` with two dependents returns `line` of 3,
    three `RefMove`s bottom first, ghosts for all picks.
  - `r_over_a_fork_is_refused_before_y`: `Refused(ForkedDependents{a, b})`, both named. This replaces
    `rewrite_run::r_with_two_branches_on_top_is_not_refused_after_y`, which is deleted, with the reason in NOTES.
  - `r_with_a_dependent_elsewhere_is_refused`.
  - `a_squash_merged_bottom_fills_drops_and_already_in`: `drops` == `AlreadyIn.commits`, and no ghost replays them.
  - `the_chained_forecast_skips_drops`.
  - `plan_as_run_leaves_every_other_kind_alone`: merge, pick and checkout ops come back `==`, and the plan is equal to
    `plan`'s.
  - `a_line_whose_every_row_is_dropped`: whichever of the two outcomes git decided.
- [x] (`rewrite_preview::the_chained_forecast_agrees_with_git` → `line_rows_agree_with_git`: `conflicted_line` StopsAt 3 of 4 on b1 ⇔ git stops there, same paths; `squash_merged` clean, `lands_as_drawn` ref by ref) **Oracle** `the_chained_forecast_agrees_with_git` gains two rows, both driven through `plan_as_run` + a real `y`
      in a copy: a stop inside a dependent's commits (`StopsAt{step}` ⇔ git stops there, same paths), and a clean line
      with drops (every branch lands where the ghosts said: ref-by-ref equal).
- [x] graft-ops, `tests/it/rewrite_run.rs`, red first — each test below by name, all in that file:
  - `r_moves_every_branch_of_its_line`. It replaces `r_moves_his_branch_alone_until_the_preview_draws_the_line`
    (deleted, reason in NOTES).
  - `r_records_and_undoes_every_branch_of_its_line` (`refs_before` holds the line; `u` puts each back).
  - `an_r_line_that_stops_continues_home`.
  - `a_branch_built_on_his_after_the_preview_is_refused_moved`.
  - `a_drop_not_among_the_rows_is_refused_moved`.
  - `r_drops_the_squash_merged_commits` (the todo git ran holds `drop` for each; the rest land).
- [x] graft-ops: `operation/history/tests.rs` `a_line_rebase_names_its_top` (`… <onto> <top>`; a one-branch line is
      byte-identical to today's string). `operation/tests.rs` `rebase_tips_pin_every_branch_of_the_line`.
- [x] graft, `update/preview/tests.rs`, red first: `the_confirmed_rebase_is_the_one_the_job_completed` (a scripted
      `Planned { runs: Some(line of 3) }`, then `y` → the launched op's `line` has 3).
- [x] (the five tests named below, `update/preview/tests.rs`; reds in NOTES; `cut_order_never_cuts_a_branch_name` rewritten) **Mutation proofs for 05's four graft tests**, written after the code (05 NOTES). For each: revert the one line it
      guards, run it, quote the red in NOTES with the file:line reverted, then restore. A test that stays green is
      rewritten until it goes red, and NOTES says so. Tests: `the_box_draws_one_row_per_branch` (draw only the first
      `RefMove`), `a_one_branch_rebase_draws_as_before` (show the `branches:` label for one branch),
      `already_in_row_says_dropped` (drop the `· dropped` words), `a_stop_forecast_names_the_step` (draw `StopsAt` with
      `Likely`'s words), `cut_order_never_cuts_a_branch_name` (put the branch name first in the cut ladder).
- [x] graft walk, `tests/it/m7_walk/rebase_line.rs` — the three tests below by name (routes: `j r y`, 3 each):
  - `r_y_moves_the_line`: keys `r`, `y` from a cursor on `main`; every branch's ref read after equals the ghost it was
    drawn as. Steps ≤ the note's `rebase-line-onto-main` count, 3.
  - `r_y_drops_the_squash_merged_branch` (≤ `squash-merged-bottom`, 3).
  - `r_over_a_fork_lands_on_the_graph_with_the_words` (box closed, status line holds both names, no ref moved).
- [x] (`m7_walk::rebase_line::capture_rebase_line`; 4.7 KB, 2.8 KB, 0.8 KB; captions appended) **Captures** (rule 6: the user judges by eye; colour **unverified**, rule 5), through `check_shot`, from
      `m7_walk/rebase_line.rs`:
  - `docs/shots/m7/rebase-line-preview-187x50.txt`: `branches:` label, 3 rows bottom first, the two above his marked
    `built on yours · follows`.
  - `docs/shots/m7/rebase-squash-merged-preview-120x40.txt`: the `already in main: feature-a's N commits match <sha7>
    "<subject>" · dropped` row, **no `R`**, and no ghost for a dropped commit.
  - `docs/shots/m7/rebase-refused-fork-120x40.txt`: the graph, the status line naming both branches.
  - Three rows appended to the captions TSV, each saying what right looks like.
- [x] (full workspace nextest 4327/4327 incl. `m5_walk::preview`, `it::preview`, every `put_back_forecast`/`set_aside`; `git status docs/shots` shows only the three new files and the TSV) Regression: `m5-01-preview-rebase-{80x24,160x48}` and `m5-ui-preview-rebase-{80x24,160x48}` byte-identical
      (`m5_walk/preview.rs:80`, `tests/it/preview.rs:261,295`). The m5/m6 rebase walks and every
      `put_back_forecast`/`set_aside` test green, or NOTES lists each changed assertion and why.
- [x] (doc; no test) MAP.md: graft-ops `apply::engine` (delete "his branch alone … until ticket 05" and "`r` refuses nothing over
      dependents"), `RebaseParams`, `preview::plan_as_run`; graft `Planned::runs`, `jobs::preview`.

## Touches (check each; the report's Touches line says how)
- **Every `r` entry point**: the graph `r`, the context menu `Rebase <cur> onto <b> (r)` (`verbs.rs`), drag
  (`drag.rs`), and `p`'s diverged-upstream `r` (`remote/tests.rs`). All build `RebaseParams::new` and pass through the
  preview job. Check: a grep for `RebaseParams {` in `crates/graft/src` returns 0.
- **Every other preview kind:** unchanged. Check: `plan_as_run_leaves_every_other_kind_alone`, and every existing
  preview test + capture byte-identical.
- **Journal / undo:** an `r` entry now records every branch of the line. Check: `r_records_and_undoes_every_branch_of_
  its_line`. Kind stays `Rebase`, so the journal format is unchanged (`a_rewrite_entry_round_trips` green).
- **Stops, banner, relaunch** (M6; 07 builds the chain line): an `r` line stops with HEAD detached inside the line,
  from the saved plan, as `R` does. Check: `an_r_line_that_stops_continues_home`, the m6 rebase-stop walks green, and
  02b's kill+relaunch test (`line_onto_main`) green.
- **Set-aside around `r`:** the put-back forecast must see the landed tree with drops skipped. Check: `put_back_forecast`
  green, plus one row with drops if `preview/put_back.rs` is touched.
- **`rebase-preview-keeps-drops`:** takes the completed `RebaseParams` (`FromRebase`) from the preview subject, which
  is now the completed op. Its capture re-takes `rebase-squash-merged-preview-120x40` with `· R to keep them`.
- **KNOWN_GAPS** (conductor, not the builder): 02's cost "`r` leaves a teammate's branch on the old commits" closes. The
  fork refusal's cost is new (above). 05's `on origin` remote-name gap stays open.
- **Keymap / hints:** none.
- **ADR-0047:** unchanged. This ticket makes `r` obey §4/§6.

## Gates
The four, workspace-wide, in the worktree with its own `CARGO_TARGET_DIR`. Experiments with git go in a script file
run by `sh` (02's gotcha: inline git in temp dirs is refused in an isolated worktree).

## Size
Medium (~55k). It can't split across crates: `RebaseParams`'s new fields break graft's build. If compacted, split after
the graft-ops boxes, with graft on `RebaseParams::new` already. The remainder is the graft job, `Planned::runs`, the
mutation proofs, the walk and the captures.

## NOTES (builder, 2026-09-27)
**Compacted once**, after the graft `Planned::runs` work and the first mutation. Nothing remains: every box is
ticked above. Scout: none new (05's reused, as the ticket says).

**Reds quoted (test-first):**
- graft-ops integration tests (every new `rewrite_preview`/`rewrite_run` test): compile red, "unresolved import
  `graft_ops::preview::plan_as_run`".
- `a_line_rebase_names_its_top`: left `…--empty=drop refs/heads/feat`, right `… feat feature-c`.
- `rebase_tips_pin_every_branch_of_the_line`: tips `[main]` against `[main, feature-b, feature-c]`.
- `the_confirmed_rebase_is_the_one_the_job_completed`: compile red, "struct `Planned` has no field named `runs`";
  then with the `clone_from` in `preview_produced` removed, "the line the job completed left: 1 right: 3".
- `put_back_forecast::a_rebase_with_drops_measures_the_put_back_from_where_it_lands`: forcing `Landing::Replayed`
  gives left `Some(NotChecked("the rebase may stop first"))`, right `Likely{x.txt}`.

**Mutation proofs (05's graft tests; each file restored after, `git diff` of `line_rows.rs` empty):**
- `the_box_draws_one_row_per_branch` — `update/preview/line_rows.rs:34` `.iter()` → `.iter().take(1)`: red,
  panic at `update/preview/tests.rs:1389`; lines hold only `branches: feature 0000000 → 1 new commit`.
- `a_one_branch_rebase_draws_as_before` — `line_rows.rs:24` `> 1` → `>= 1`: red at `tests.rs:1399`.
- `already_in_row_says_dropped` — `line_rows.rs:62` ` \u{b7} dropped` removed: red at `tests.rs:1419`.
- `a_stop_forecast_names_the_step` — `line_rows.rs:97` head → `may conflict: {step} {of}` (Likely's words): red at
  `tests.rs:1449`.
- `cut_order_never_cuts_a_branch_name` — **stayed green** under the `take(1)` mutation: its `contains("topic")` was
  satisfied by the stop row's subject `topic work`. **Rewritten** to assert the branch rows themselves (a row starting
  `branches: feature `, and one starting with the 10-space indent + `topic ` ending `built on yours · follows`). Now red
  under `take(1)` at `tests.rs:1473`, green restored. The ticket's literal mutation ("put the branch name first in the
  cut ladder") has no line to revert: `plan_lines` ladders only the stop row (`forecast_rungs`) and the set-aside row;
  branch rows are never laddered, so a cut can't reach them. `take(1)` (a branch row missing) is the failure the test
  guards.

**git, all-drop todo** (script in the scratchpad, git 2.54): a todo of `drop` rows only → "Successfully rebased and
updated refs/heads/feature-a.", exit 0; feature-a == main, HEAD stays `refs/heads/feature-a`. So it **runs**; no
`EverythingDropped` refusal (`a_line_whose_every_row_is_dropped` plans and runs it).

**Deviations / decisions to know:**
- `drops` = the suggestion's commits **that `rebase_rows` has a row for**. `rebase_rows`/`replayed_commits` use
  `--cherry-pick --right-only --no-merges`, so a commit git already leaves out (e.g. a one-commit squash, patch-identical)
  has no row; writing it would make the engine refuse `Moved` on every such rebase. `already_in` still names all of them.
- `plan_as_run` runs `op.check` (the shared `checked` gate) before `line_of`, so a branch switched/moved since the key
  refuses first; then clones the facts and inserts the dependents' tips, so `check`/`tips()` pin the line drawn.
- `preview/put_back.rs` **touched**: it replayed the rebase itself (one `merge-tree HEAD <onto>`, which also counted
  dropped commits). Now `kinds::rebase` returns his landed commit from the chain (drops skipped) and the put-back
  uses it; the merge-tree stand-in remains only when the chain didn't reach it. One new row (above).
- Deleted `rewrite_run::r_moves_his_branch_alone_until_the_preview_draws_the_line` and
  `…::r_with_two_branches_on_top_is_not_refused_after_y` (+ its `forked()` fixture): both pinned 02's stopgap (review
  02 Major 1), superseded by `r_moves_every_branch_of_its_line` and `r_over_a_fork_is_refused_before_y`.
- `rewrite_run::the_plan_dir_is_removed_when_the_entry_closes` now uses a completed op (`r_onto_main`): an uncompleted
  `RebaseParams::new` over a line is refused `Moved` at `y` — the intended new behaviour.
- clippy `too_many_lines`: `kinds::rebase` split into `line_ghosts` + `todo_steps`; `preview::plan`'s check moved to
  `checked` (shared with `plan_as_run`). No behaviour change (full suite green).
- The walk file and its `mod` line were created by the builder (no seam commit), as the dispatch said.
- No `RebaseParams` literal was found in 16's or 01b's files.

**Question (rule 7, not a defect claim):** in `rebase-line-preview-187x50.txt` the follower rows read
`feature-b … → 3 new commits`, `feature-c … → 4 new commits` — the count is every ghost under that branch's tip
(`crates/graft/src/update/preview.rs:1085`, `new_commits_at`: "the ghost it lands on and every ghost below it"), not the branch's own commits (1 each). Is that the wording wanted?

**Captures:** colour unverified (rule 5); the user judges by eye (rule 6).
