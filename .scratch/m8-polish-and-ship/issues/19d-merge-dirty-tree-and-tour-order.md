# Merge over unsaved edits sets them aside; the tour's ages stop following the calendar; the whole-tour walk

Status: draft (planner spec-only pass, 2026-10-08; rules on 19c-c NOTES :221-232 and :261-263). Amended 2026-10-08,
planner follow-up on NOTES 19d-b: §R4, §R5 and part 19d-b2 added, and 19d-c gains C+3. Amended again 2026-10-08,
planner follow-up on NOTES 19d-a (branch `worktree-agent-a9afa698c484ca6f4`, commit bebdddd5, not merged): §R6 and
part 19d-a2 added. Amended a third time 2026-10-08, planner follow-up on NOTES 19d-a2 (branch
`worktree-agent-a67f6d8f405f4ef37`, commit 47f5a389, not merged): §R7 and part 19d-a3 added.
Build with: six parts. a/a2/a3 and b/b2 have disjoint file sets and may run concurrently (two helpers max). a2
continues a's branch and a3 continues a2's. b2 comes after b. a3's re-shoot waits for b2. c comes last and needs all
five.
- **19d-a — Opus, xhigh.** Stash- and undo-adjacent: a wrong order or a missed resume path loses edits, and no diff shows
  it. Not Fable: no new structure. It is the third extension of a proven mechanism (11a cherry-pick, 03b ff-pull, both
  on the same `sets_aside` arm and the same table tests), and the round-trip tests below can show whether it is right.
- **19d-a2 — Opus, xhigh.** D1 and D2 are in `apply/close.rs`, the close every set-aside kind shares. That path
  decides whether `u` and the stop words are right after a stop. Not Fable: both defects carry their lines and red
  tests, the fix is local to the close, and the tests can show whether it is right.
- **19d-a3 — Opus, xhigh.** The stop-outcome path again (`apply/stopped.rs`), plus one field on a public error variant.
  Not Fable: a single line names the defect, and a red test can prove the fix.
- **19d-b — Sonnet.** A pure parse, one read at launch, one default in the pty harness. Every signature is fixed below.
- **19d-b2 — Sonnet.** Mechanical: one constant moves, one test seam is fixed, and 19 producers and one pty default are
  pinned. No production API. The rulings are §R4 and §R5.
- **19d-c — Opus.** The 19c-c walk, finished; pty and captures. **Last.**
Needs: 19c-a, 19c-b (merged), 19c-c partial (merged, 706d988f). 19d-a2 needs 19d-a (bebdddd5; the two merge
together). 19d-a3 needs 19d-a2 (47f5a389; a, a2 and a3 merge together); its capture re-shoot also needs 19d-b2 merged.
19d-b2 needs 19d-b (merged, partial). 19d-c needs 19d-a, 19d-a2, 19d-a3, 19d-b and 19d-b2.
Scout: 19d-a uses `.scratch/m8-polish-and-ship/scouts/merge-preview-dirty-index.md` (no second scout). 19d-a2: NOTES
19d-a is its scout. 19d-a3: NOTES 19d-a2 is its scout. 19d-b: none. MAP :91, :182 and :1116 are enough; the conductor
decides. 19d-b2: the sweep in NOTES 19d-b is its scout.
ADR: **none new.** Nothing here is hard to reverse: the journal field (`put_back`) already exists and is additive
(ADR-0045 §1), and `sets_aside` is one match arm. 19d-a appends an **ADR-0045 amendment** (text in §R1, last bullet),
as 11a and 03b did. 19d-a2 corrects its spawn count (§R6).

## Need
- He merges colleagues' branches into his own while work is in progress. He works in a team on dependent branches
  (memory), and field notes :67 record pick 16: "switch with unsaved edits → set them aside and put them back
  (clash lands on the conflict screen)". Merge was not named in that pick, so it fell through.
- Today the preview reads `no conflicts expected` and offers `y`. Then git refuses: `error: Your local changes … would
  be overwritten by merge`, with git's text in the status line (19c-c NOTES :225-228). This breaks the M5 promise,
  "you see the result before you confirm". The cause is that the forecast reads only the two commits
  (`graft-git/src/history_reads/forecast.rs:60`, `:80-99`), `check_merge` has no clean-tree rule
  (`graft-ops/src/operation/history.rs:582-585`), and `sets_aside` has no `Merge` arm (`graft-ops/src/operation.rs:1494`).
- git refuses a non-fast-forward merge over **any** staged change, even one the merge doesn't touch (19c-c reproduced
  this on git 2.54). It also refuses over an unstaged change to a path the merge changes.

## Prior art (take / reject / invent)
- **Graft's own precedent: take.** Rebase, cherry-pick, a previewed switch and a fast-forward pull with overlapping
  edits all set the edits aside and put them back in one journal entry (ADR-0045 §4; amendments 10b, 11a, 03b;
  `operation.rs:1477`; `docs/design-notes/m6-stash-put-back.md:110`). Cherry-pick sets aside **every** changed tracked
  file, not only the touched paths, because `sets_aside` reads no commit (ADR-0045 amendment 11a). Merge does the same.
- **lazygit: take the shape.** On a switch blocked by local changes it asks "Autostash?" and then does stash → op →
  pop. That is +1 key over a clean switch, 2 in all (`commit-stash.md:276-280`; `checkout-cherry-pick.md:19`). The
  study does not cover lazygit's merge over local changes.
- **JetBrains: take the landing.** Smart Checkout shelves, switches and unshelves; a clash goes straight to the merge
  dialog (`commit-stash.md:290-295`). The study does not cover its merge over local changes.
- **Sublime Merge:** the study covers only its `pull --rebase --autostash` hiding a failed put-back, issue #1199, open
  (`commit-stash.md:288-289`). That is a complaint against trusting git's exit code. ADR-0045 already avoids it by
  reading the index. The study does not cover its merge over local changes.
- **git `--autostash` / `merge.autoStash`: reject.** Its stash is visible only as `autostash`, and a clash exits 0
  (`commit-stash.md:238-248`, A4 :322). ADR-0045 rejected it for the same reason.
- **Refuse `DirtyTree` (branch-create precedent, `history.rs:658`): reject for merge.** It costs about 9 steps
  (`S s ⏎`, back, `m y`, `S p y`) against 2, and pick 16 chose set-aside. The refusal stays where it answers a
  different need: the instant switch and branch-from-stash.
- **Invent: none.**
- Covered here: the three tools' switch and pull behaviour. Not covered by the repo's research: lazygit, Sublime Merge
  and JetBrains on **merge** over local changes. No new study: the in-house precedent decides it, and its own study
  (`commit-stash.md` A3-A4) was the input.

## Journeys (step counts; the 19d-c walk asserts the tour's)
1. **Merge a branch while he has unsaved edits** (cursor on the branch row): `m`, which opens the preview and draws the
   set-aside row and the put-back forecast; then `y`. **2.** Clean tree: 2, unchanged. lazygit switch with changes: 2.
   He lands on the graph with the cursor on the new `HEAD` (`moves_head`), the done status plus `your edits are back`
   (`edits_back`, graft MAP :645), and the staged/unstaged split as before (`stash apply --index`).
2. **The same merge, which stops on a conflict.** `m y` → the resolver, as today, with the stop line plus
   ` · edits aside` (graft MAP :504) → resolve → finish `y` → put back → graph, `your edits are back`. No extra step.
   `a` (abort) puts them back onto the old tip (ADR-0045 §4).
3. **The put-back clashes** (merge clean, his edits overlap what it brought in). He lands on the resolver with the
   set-aside side names (ADR-0045 §2-3); finishing drops the copy, and `u` brings it back. Same as rebase.
4. **Today** (for the record): `m y` → git's refusal in the status line → he leaves to the shell, or goes through
   `S s ⏎ Esc m y S p y` (9).
5. **A merge stopped in a git hook while his edits are aside** (13c): he lands where 13c lands him, paused, with `c`
   and `a` offered. His edits wait in the set-aside copy until he chooses, as for a stopped rebase. `c` or `a` puts them
   back. No extra step.

## Rulings
### R1. Merge over changed tracked files: preview shows the set-aside, `y` sets aside and puts back (19d-a)
- `Operation::sets_aside` gains a `Merge` arm with the cherry-pick rule: `Some(SetAside { files })` when any tracked
  file is changed (staged or unstaged), otherwise `None`. Untracked files are never set aside. Fast-forward or not,
  `--ff-only` or not, the rule is the same. No `DirtyTree` refusal is added to `check_merge`.
- When `sets_aside` is `Some`, `touches` adds `refs/stash`. Without it `u` failed for cherry-pick (11a).
- `apply`: §4's order unchanged: `stash create` → `begin` (`put_back: SetAside(S)`) → `stash store` → `reset --hard` →
  `git merge …` (plan unchanged) → `stash apply --index S`, read the index. A merge that **stops** keeps `S` until
  `resume` ends it. The routes are `Continue`, `ContinueWith { message }` (the finish screen's commit) and `Abort`,
  and each runs the same put-back. **This is the one route 11a and 03b did not exercise. Prove it, don't assume it.**
- Preview (ADR-0045 §8): `put_back` is `Some` exactly when `set_aside` is. The landed tree is the source commit for a
  fast-forward. For a true merge it is `merge-tree --write-tree --merge-base=<base> HEAD <source>` (`merge_tree_step`,
  graft-git MAP :202), made a scratch commit as the rebase path does. When the merge forecast is not `Clean`, the
  put-back is `NotChecked("the merge may stop first")`. Then comes the §8 step 3 merge of `S'`. That is ≤ 4 extra
  spawns, only when `set_aside` is `Some`. A clean tree spawns none. (Amended by §R6: ≤ 5 on a true merge, ≤ 3 on a
  fast-forward.)
- The app draws the set-aside row and the put-back row that rebase and cherry-pick draw today, **with no new words**.
  If `plan_lines.rs` gates those rows by kind, add `Merge` there. Never fork the wording.
- A preview change the user sees (one or two rows on a merge preview with edits) that **follows precedent**: the
  previews of rebase, cherry-pick and a previewed switch draw these rows already.
- **Pull checked:** Graft's pull is `fetch` then `merge --ff-only` (graft-ops MAP :65). No pull writes a merge commit,
  and a diverged pull stops with `NotFastForward` words before git merges. The fast-forward pull sets aside on overlap
  (03b). The remaining case is an ff pull over a staged change it doesn't touch, and nobody has run it. **Verify**, test
  below. Green → nothing changes. Red → stop and report: the pull then needs this rule too, and that is a ruling.
- ADR-0045 amendment, appended by the 19d-a builder after its tests are green:
  `## Amended 2026-10-08 (19d) — a merge sets aside` — the rule above, as built, naming the tests in §19d-a's boxes.
  It records two things. Why all files and not only the overlap: git refuses a true merge over any staged change, and
  pick precedent. Why not refuse: pick 16, and a 2-step journey against 9.

### R2. Tour order: R1 alone fixes it; practice repo, task order and card words unchanged
- The cause is not in the tour. Task 3's `u` leaves `notes.txt` staged because the generic inverse puts back the index
  exactly as it was before the commit (ADR-0045 §6, "staged split included"), which is correct. Task 4's pop with
  `--index` keeps the split, also correct. The merge at task 6 was the defect.
- Under R1, task 6's merge sets aside `notes.txt` (and nothing untracked: `scratch.txt` stays). It merges `feature`,
  which touches chef/hours/menu/specials (19c-c NOTES :224), then puts `notes.txt` back staged. Task 7 sets aside the
  same way, stops on `menu.txt`, and puts it back on finish. `notes.txt` is not in `clash`'s change, so no put-back clash.
- The keys on the card stay right: task 6 is still `m … y`. Its preview shows one more row, which teaches the
  behaviour. **No change** to practice repo, task order or card words.
- Knock-on to check (19d-a): 19c-b's per-task tests start fresh tours with `notes.txt` unstaged-modified, so tasks 6 and
  7 now set aside there too. Re-run `tests/it/tour_tasks.rs`. A wait that breaks on new rows or on the ` · edits aside`
  stop line gets its expectation updated **and named in NOTES**, never loosened to pass.

### R3. "Captures go red when ages roll 6y→7y": real, fix the cause (19d-b)
- Why: the drawn age is the time from a fixed instant (`FIXED_DATE` / `GRAFT_PRACTICE_EPOCH`) to the session clock,
  and the pty binary runs `Clock::Wall` (graft MAP :91; Harness default `Wall`, MAP :182). So the age changes with the
  calendar. The day `6y` flips is **not measured**: it depends on the label's rounding. NOTES :261 says "no env pin in
  the binary" without a line, so **verify first** (below).
- Fix: pin the session clock in the binary from an env var, the `GRAFT_PRACTICE_EPOCH` pattern (19c R2), and give the
  pty harness a default pin. **Rejected:** masking ages in `steady()`, which fixes the symptom and would hide a real age
  bug. **Rejected:** dating fixtures from now, which changes commit ids every run (graft-fixtures MAP :32).
- `GRAFT_NOW` (whole seconds since 1970) → `Clock::Fixed(n)` for the whole session: the date path **and**
  `JobRunner::with_clock` (journal stamps), as the in-process `Harness` does (m5/25). Unset → `Wall`. Unparsable → no
  launch: stderr `graft: GRAFT_NOW must be whole seconds since 1970, got <v>` (sanitised), exit 2. Never ignored
  (rule 3). G-rename 27's grep finds it, like `GRAFT_PRACTICE_EPOCH`.
- **Amended 2026-10-09 (19e, review m8-tour-setaside-fetch M1):** GRAFT_NOW pins the drawn clock only (the frame and
  the practice repo's dates). The binary stamps the journal from the wall clock, always: a shipped env var must never
  write false history into a real repo, and J, the switch window and state_at order by the stamp. In-process tests
  still pin stamps through App::open_with_clock / JobRunner::with_clock, which no env var reaches. Pty tests rely on
  the drawn pin, fixture dates and §R5's git dates; the tour relies on journal order.

### R4. The sweep's split (NOTES 19d-b :269-275): pin every producer, freeze the orphans (19d-b2)
- **One constant.** `PINNED_NOW` moves from `tests/it/pty.rs` to `graft_fixtures::PINNED_NOW` (same value,
  `1_791_417_600`). graft-fixtures is the dev-only crate that tests, examples and `#[cfg(test)]` modules already
  reach. `pty.rs` uses it from there.
- **Fix the seam, not 15 call sites' symptoms.** `App::open_with_clock` (`tests/it/app_harness.rs:154`) pins the job
  runner but not the frame. Make its clock reach the `Harness` clock too, so one clock covers the frame and the
  journal stamps, as R3 says for the binary. The signature stays as it is. Then every producer that already goes
  through it is pinned with no change at its site.
- **In-process producers** (the 15 in NOTES :272): each one that writes a year-age capture builds with
  `Clock::Fixed(graft_fixtures::PINNED_NOW)`. That is `open_with_clock` where it opens an `App`, or
  `Harness { clock: Clock::Fixed(graft_fixtures::PINNED_NOW), ..Harness::default() }` where it builds a `Harness`.
  **No new production constructor**, and `Harness::default()` stays `Wall` (the shipped headless path, `main.rs`,
  and `places.rs:144` use it).
- **Examples** m3/m4/m5 walk (`m3_walk.rs:168`, `m4_walk.rs:195`, `m5_walk.rs:214`) and the **gallery** (git-git,
  workspace, 100k: `gallery.rs:53`, `:1007`): the same struct-update with the constant.
- **Guard, so it stays fixed.** A test, `clock_pin::no_capture_producer_runs_on_the_wall_clock`, scans every source
  file under `crates/graft/{tests,examples,src}` that writes a capture (calls `Captures::check`, or a local `shot`
  helper over it). It fails when such a file names `Clock::Wall`, or builds `Harness::default()` without overriding
  `clock`. Its message names the file and line. Why a scan: a rule only a reviewer checks is the "be careful" kind
  that doesn't hold.
- **About 70 captures with no producer: frozen, not re-homed.** No test asserts them, so they cannot go red. They are
  each milestone's record as drawn on its day. Re-homing them would mean writing producers for evidence nobody
  re-checks. Record them in `docs/shots/FROZEN.md`, one line each: `<file> · no producer · last changed <date>`, with
  the date from `git log -1 --format=%cs`. NOTES names the script or command that built the list. **Contact sheets
  are not frozen.** They are derived and are rebuilt from their parts (ADR-0037; 19d-c C+2 already rebuilds m7 and
  m4). A sheet whose parts are frozen keeps those parts' text.
- Expected: with `PINNED_NOW` = 2026-10-08 00:00 UTC, every asserted capture stays byte-identical, because each one
  passed against today's wall clock. A capture that changes is listed in NOTES with its diff's lines. It is
  regenerated only when every changed cell is an age.

### R5. In-app commits under `GRAFT_NOW` draw `future` (NOTES 19d-b :268): pin git's dates in the harness, not in Graft
- Status: observed once by 19d-b in its own test (`u_under_a_pinned_clock_undoes_the_newest`), which commits through
  the app with git's wall date while the pin is earlier. It has not been seen in a committed capture.
- **The tour's captures are not hit.** The 19c-c draft walk sets `GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE` itself, to
  `EPOCH + 3600` (`handoff/19c-c-whole-walk.rs.txt:87-94`). So the commits at tasks 2, 6 and 7 are dated 2021 and draw
  as years. 19d-c keeps that env (C+3).
- **Rejected: Graft sets git's dates from `GRAFT_NOW`.** The shipped binary would write a date a test variable told it
  into the user's commits. `GRAFT_NOW` is the clock Graft **reads**, nothing more. Fixtures already date commits from
  outside the app (`graft_fixtures::git::run_at`, graft-fixtures MAP :32; 19c R2).
- **Ruling: the pty harness dates commits by default.** `Pty::spawn*` sets `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE`
  to `"<PINNED_NOW − 3600> +0000"` unless the caller's env names them, the same rule as its `GRAFT_NOW` default. An
  in-app commit then draws `1h`, and its id is the same every run.
- **In-process producers:** git runs as a child of the test process. If one draws an in-app commit, the full run's
  `grep -l future docs/shots` finds it. Pin it by giving that test's git child the dates. If that needs an env hook in
  production code (the job runner), **stop and report**. Don't add one.

### R6. 19d-a's blocker (NOTES 19d-a, bebdddd5): option (a), fix the cause (19d-a2)
- **(b) rejected.** Setting aside only the files the merge touches would leave git refusing a merge over an untouched
  staged change. That is the tour's case (19c-c NOTES :225-228), which is what R1 exists for. It would also break the
  cherry-pick rule.
- **13c: keep the fixture, change the expectations.** `diverged()`'s unstaged `HIS_EDIT`
  (`crates/graft-ops/tests/it/stop_hooks.rs`) is his daily case: a merge stopped in a hook while he has edits. It now
  exercises set-aside together with the stop, which is exactly where D1 and D2 live. A clean fixture would hide them.
  The 7 paused tests change their claim from "his edit is untouched" to: while paused, `S` is listed (found by commit)
  and the tree holds `HEAD`'s copy; after `resume` (`Continue` or `Abort`), the edit is back with its split and `S` is
  dropped. That is ADR-0045 §4's rule for a stopped rebase, as built. **Plus one clean-tree twin per hook stop**
  (prepare-commit-msg, pre-merge-commit), so 13c's pause is still proven with nothing set aside.
- **D1** (`apply/close.rs:100-101` returns `closed_set_aside` before `:150`'s `stopped::mark`) and **D2**
  (`close.rs:176` drops `PlanFailure.before_pause`, `apply/run.rs:37`) are defects in the close **every** set-aside kind
  shares. Fix them there, never with a merge-only branch, so a cherry-pick gets the same marks. graft-ops MAP :263
  already records D2's effect ("Not reached: … a set-aside's close (reason stays `git was stopped`)"). That clause is
  withdrawn, because the reason now carries through.
- **The two captures** (`docs/shots/m8-hangup-relaunch-120x40.txt`,
  `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`): re-shoot after reading the diff. Every changed cell must be
  the set-aside's own words (the ` · edits aside` stop rung, graft MAP :504). Any other change → stop. Captions say what
  right looks like. (Superseded by §R7.2.)
- **Clash notice (broken, `crates/graft/src/update/clash.rs` `undone_with_edits`, `_ => "rebase"`):** fix it in the same
  part, since it is the line this milestone touched. An exhaustive match with no `_` arm, worded from the entry's own
  kind name (the journal's `kind_name`, graft-ops MAP :241, `apply/record.rs`). No new vocabulary. A kind name that
  reads oddly in `u undoes the <kind> too` is listed in NOTES, not reworded. (Amended by §R7.1.)
- **Spawns: accept the measured 5 (true merge) / 3 (fast-forward).** R1's "≤ 4" is amended to those numbers. The
  baseless `merge-tree` proposal is rejected. It would change the fixed `merge_landed` signature and reach into
  graft-git (`forecast_merge` returns no tree) to save one spawn, on a preview job that checks `stop` between spawns.
- **Up-to-date merge over edits (question, NOTES 19d-a :303-304): verify first.** Write a test. If the nothing-to-do
  box draws a set-aside row, the cause is the plan. Fix it in graft-ops `preview::plan`: a merge with no ref move
  gets `set_aside: None` and `put_back: None`. If that box offers `y` (so `apply` would set aside for nothing),
  **stop and report**. That is a different rule.

### R7. 19d-a2's three questions (NOTES 19d-a2, 47f5a389) (19d-a3)
1. **Clash notice: accept the deviation.** Checkout and a switching branch-create keep `switch`. R6 said "kind name" to
   forbid **new** vocabulary, and `switch` is shipped vocabulary. It is approved by eye in 4 m6 captures, 2 contact-sheet
   entries and `TESTING.md:2598`, `:2724`. It is also the 10b rule's own word ("one rule for every switch",
   ADR-0045 amendment 10b), so reading it from `kind_name` would reword a correct line and re-shoot m6 for nothing.
   `recover` and `rewrite` stay as `kind_name` gives them, listed (as NOTES 19d-a2 did). **The unknown-kind fallback**
   (`clash.rs`, `let Some(kind) = kind else { return "rebase" }`) words a guess as fact. Make it name no kind: drop the
   `the <kind>` clause and keep the notice's other words. The shipped None test changes its expectation to match, and
   NOTES says so. Whether the app can reach it stays **unverified**; the conductor files that row.
2. **The two captures: the new cells are right.** They are true consequences of R6 for a paused merge with edits aside:
   - The stash row (`stash@{0} … graft: your edits, set aside for merge main into feature`) is right: `S` is in the
     stash list while paused (ADR-0045 §4 step 3) and the graph draws every stash.
   - `1 unstaged` / `UNSTAGED (1) M base.txt` going away is right: his edit is in `S`, not the tree. The `2 staged`
     that remain must be the paused merge's own staged paths. **Assert that**, don't assume it: the staged paths equal
     the merge's changed paths.
   - At 80×24 no ` · edits aside` rung shows. That is by design: the part drops first (graft MAP :504). The stash row is
     what tells him there. The conductor puts it on the usability walk's list, and it isn't changed here.
   - **The re-shoot waits for 19d-b2.** Its pty defaults (`GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE`, §R5) date
     `git stash create`, so `S`'s id and age are fixed. After b2 merges: run each producer twice, and both runs are
     byte-identical. No `future` (`futu…`) in either. If the fixture sets its own git dates for the app's child, they
     win, and they must hold too. Unstable after b2 → stop and report. **No masking of the id.**
   - Captions are rewritten for what is drawn now. The 80×24 caption loses "his edit unstaged (`M base.txt`)" and says
     his edit is set aside (the stash row) and comes back on `c` or `a`.
3. **Landed lost when the put-back clashes (broken, `apply/stopped.rs:248`, `Err(other) => Err(other)`): fix in a3.**
   `ApplyError::Clashed` gains `stopped_at: Option<StoppedAt>`, set to `Some(Landed)` on that path and `None`
   everywhere else. `ApplyError::stopped_at()` returns it. This is a signature change on a public variant, inside the
   workspace only. Graft's matches take the field or use `..`. The app's landing is unchanged: a clash goes to the
   resolver (ADR-0045 §2). Grep every graft caller of `stopped_at()`, and in NOTES say what each now shows for this
   case. **No new words.** If a caller would need new words to show both the clash and Landed, stop and report.
- **Filed by the conductor, not built here** (from NOTES 19d-a2 "Findings"): a pick stopped in post-commit with the hook
  outside the group is held `Conflicted` with `HEAD` moved (`apply/stopped.rs:116`, `:125`; `close.rs:95`). It is
  broken, and the fix belongs to 13c's pause rule. Resume's set-aside close words a residual stop `git was stopped`
  (`apply/resume/close.rs:53` vs `:91`). That is unverified.

## Signatures
```rust
// 19d-a, graft-ops — public signatures unchanged; behaviour per R1.
impl Operation {
    pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside>; // + Merge arm (cherry-pick rule)
    // touches(): + refs/stash for Merge when sets_aside is Some
}
// PreviewPlan { set_aside: Option<SetAside>, put_back: Option<ConflictForecast>, .. } — Some for Merge with edits.

// 19d-a, graft-ops preview/put_back.rs (crate-private, new)
pub(crate) enum MergeLanded {
    /// The commit the merge lands on: the source (fast-forward) or a scratch commit of the merged tree.
    At(Oid),
    /// The merge forecast is not Clean → put_back = NotChecked("the merge may stop first").
    MayStop,
    NotChecked(String),
}
pub(crate) fn merge_landed(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Oid,
    params: &MergeParams,
    stop: &dyn Stop,
) -> Result<MergeLanded, RunError>;
// Use the crate's own types of these names as `merge_tree_step` and the rebase landing take them. If one differs,
// keep the shipped type and name it in NOTES. Do not add a parallel type.

// 19d-a2: no signature changes. The fixes are inside `apply/close.rs` (crate-private): the set-aside close path
// carries `stopped_at` into `stopped::mark` (D1) and `PlanFailure.before_pause` into its error (D2), as the plain
// close path does. `Applied::stopped_at` and `ApplyError::stopped_at()` (graft-ops MAP :260-262) keep their shapes.

// 19d-a3, graft-ops apply/error.rs — one field added to the existing variant (§R7.3):
pub enum ApplyError {
    // ...
    Clashed { entry: Box<JournalEntry>, clash: Clash, stopped_at: Option<StoppedAt> },
    // ...
}
// `ApplyError::stopped_at(&self) -> Option<StoppedAt>` (unchanged signature) returns the field for `Clashed`.
// Keep the shipped field types of `entry`/`clash` if they differ from these; add only `stopped_at`.

// 19d-b, graft (beside `Clock`, session; MAP :91)
pub(crate) const NOW_VAR: &str = "GRAFT_NOW";
/// Pure: `None` → Wall; whole seconds → Fixed; anything else → Err(words for stderr).
pub(crate) fn clock_from(var: Option<&std::ffi::OsStr>) -> Result<Clock, String>;
// tests/it/pty.rs
pub const PINNED_NOW: i64; // chosen so no committed capture changes; named in NOTES
// Pty::spawn* sets GRAFT_NOW = PINNED_NOW unless the caller's env names GRAFT_NOW.

// 19d-b2, graft-fixtures (moved from tests/it/pty.rs; same value)
/// 2026-10-08T00:00:00Z: the "now" every capture producer draws ages against.
pub const PINNED_NOW: i64 = 1_791_417_600;
// 19d-b2, crates/graft/tests/it/app_harness.rs — signature unchanged; its clock now also reaches the frame:
//   App::open_with_clock(..., clock: Clock) → JobRunner::with_clock(clock) AND Harness { clock, .. }
// 19d-b2, tests/it/pty.rs — Pty::spawn* also defaults, unless the caller names them:
//   GIT_AUTHOR_DATE = GIT_COMMITTER_DATE = format!("{} +0000", graft_fixtures::PINNED_NOW - 3_600)
```

## 19d-a — merge sets aside (Opus xhigh, ~60k)
**Crate set:** `crates/graft-ops` (`operation.rs`, `operation/history.rs` doc only, `preview/put_back.rs`, resume
only if a route is missing, tests `tests/it/merge_set_aside.rs` new + `main.rs` line), `crates/graft` (preview row
gating in `update/preview/plan_lines.rs` only if kind-gated; the `clashed` notice word below; tests), ADR-0045
(amendment), graft-ops MAP (:38, :63 `DirtyTree` doc, :68), `docs/design/02-keymap.md` row `m` (:257): append
"Unsaved edits are set aside and put back (ADR-0045)", as row `o` (:256) has it.
**Read first:** this file; the scout; ADR-0045 §4-§8 and amendments 11a, 03b; graft-ops MAP :38, :65-68, :241,
:246, :249, :303, :319-320; graft-git MAP :200-202; graft MAP :504, :645-646; the 11a tests
(`crates/graft-ops/tests/it/cherry_pick.rs`, set-aside rows) and `tests/it/set_aside.rs` round trips. Mirror them.
**Out:** pull (except the probe), refusal wording, any new words, the tour walk.
**State:** partial, blocked, on branch `worktree-agent-a9afa698c484ca6f4` at bebdddd5. Its ticks and NOTES are on
that branch. The remainder is 19d-a2.
**Acceptance** (`crates/graft-ops/tests/it/merge_set_aside.rs` unless named; real repos):
- [x] The tour's case, red before: `main` + a staged change to a file `feature` doesn't touch; `apply(Merge)` commits
      the merge, `notes.txt` is still `M ` staged, and the stash list equals the list before:
      `a_merge_over_a_staged_change_git_would_refuse_goes_through`.
- [x] Unstaged edits to a path the merge also changes, put back clean: `a_merge_puts_unstaged_edits_back_after`.
- [x] Clean tree, or only untracked files: `sets_aside` is `None`, no `stash` line in the entry's commands, `touches`
      without `refs/stash`: `a_merge_over_a_clean_tree_sets_nothing_aside`, `untracked_files_alone_set_nothing_aside`.
- [x] Stops on a conflict: entry `Pending` with `put_back`, `S` listed. `resume(ContinueWith)` after the resolution
      → `Committed`, edits back with the split kept, `S` dropped: `a_stopped_merge_finished_puts_the_edits_back`. Same
      with `Continue`: `…_continued_puts_the_edits_back`. `Abort` → old tip, edits back, `S` dropped:
      `a_stopped_merge_aborted_puts_the_edits_back`.
- [x] Put-back clash (merge clean, edit overlaps): entry `Committed`, `ApplyError::Clashed { why: SetAside }`, `S`
      kept: `a_merge_put_back_that_clashes_stops_as_a_clash`.
- [x] Undo/redo round trip on ADR-0045 §7's tree table (staged + unstaged + untracked, staged new file, deletion,
      rename). After `u`: index tree, worktree tree and stash list equal to before. After `U`: equal to after:
      `undo_of_a_set_aside_merge_round_trips`, `redo_of_a_set_aside_merge_round_trips`.
- [x] A stop injected before each of steps 1-6 leaves the edits in the tree or in the stash list, never in neither
      (read both): `a_stop_anywhere_in_a_set_aside_merge_keeps_the_edits`.
- [x] Preview: `set_aside` `Some`, and `put_back` is `Clean` (ff and true merge), `Likely` (overlap), or
      `NotChecked("the merge may stop first")` (the merge forecast `Likely`). Clean tree → both `None`, and the
      preview's spawn count is unchanged from before: `the_merge_preview_forecasts_the_put_back` (graft-ops).
      With edits: +5 spawns on a true merge, +3 on a fast-forward (R1 said ≤ 4; NOTES "Deviation — spawns").
- [x] App: a merge preview over a staged change draws the set-aside row and the put-back row, in the words rebase
      uses; `y` lands with `your edits are back`: graft `merge_over_staged_edits_previews_and_puts_back` (harness,
      `Clock::Fixed`).
- [x] The `clashed` notice (graft MAP :646, `undoes the rebase/switch too`) names a merge when the entry is a merge.
      If it is a fixed string, it reads the kind. Test: `a_merge_put_back_clash_notice_names_the_merge`.
- [x] Pull probe: an ff pull over a staged change it doesn't touch goes through, staged kept:
      `pull_set_aside::an_ff_pull_over_an_untouched_staged_change_goes_through`. **Red → stop, report** (R1).
- [x] 19c-b per-task tests green (`tests/it/tour_tasks.rs`); every changed expectation is listed in NOTES with why.
- [x] ADR-0045 amendment appended, naming the tests above; MAP rows and keymap row `m` updated.
**Touches (check each, say how):** journal/undo (`put_back` on Merge entries; `next_undo`; the StashDrop-on-undo
rule, ADR-0045 §6; `tour_progress` reads Committed Merges, MAP :221, so the tick still fires for a set-aside merge,
proven by 19d-c); recovery (`recover_interrupted`, `Recovered::set_aside`: generic, so grep that no kind list excludes
Merge); the stop line ` · edits aside` (`stop_line_wordings`); `edits_back` / `done_status` for Merge; the finish
screen and `banner::resumed_words` (graft MAP :645); every preview that shares `plan_lines.rs` (rebase, pick,
checkout: captures unchanged); captures that show a merge preview (grep `Preview: merge` in `docs/shots/`: each
fixture's tree must be clean, so no change; list any that changed); keymap row `m`; no key bound or freed.
`MERGE_AUTOSTASH` / `AutostashWaits` don't apply: never `--autostash`.
**Known residual, not this ticket:** the finish screen doesn't forecast the put-back (true of rebase and pick
continue too). The conductor checks KNOWN_GAPS for a row.

## 19d-a2 — the shared set-aside close, 13c's expectations, the clash notice (Opus xhigh, ~45k)
**Base:** branch `worktree-agent-a9afa698c484ca6f4` at bebdddd5 (19d-a, not merged). a and a2 merge together. Write the
NOTES under `## NOTES 19d-a2` in this file on that branch.
**State:** partial on branch `worktree-agent-a67f6d8f405f4ef37` at 47f5a389, with its ticks and NOTES there. Open
boxes: the 2 captures and the gates. They move to 19d-a3.
**Crate set:** `crates/graft-ops`: `apply/close.rs`, `apply/set_aside*` only if the close's caller needs it,
`apply/stopped.rs` only if `mark` must take a new input, `preview` (merge plan, only if the up-to-date test is red),
tests `tests/it/stop_hooks.rs` and `tests/it/cherry_pick.rs` (pick twins), MAP :263. `crates/graft`:
`update/clash.rs`, `tests/it/quit_waits/{stuck,hangup}.rs` and the relaunch capture test, the two captures above, MAP
(the `undone_with_edits` line, if listed). Also ADR-0045's 19d amendment (spawn numbers, plus one sentence: the
set-aside close keeps the stop's mark and reason).
**Read first:** §R1 and §R6; NOTES 19d-a on the branch; graft-ops MAP :249, :257, :259-263; the 13c tests named in
NOTES 19d-a (do not read 13c's whole ticket); graft MAP :504, :646.
**Out:** narrowing R1; any change to 13c's pause rule; graft-git; new words.
**Acceptance:**
- [x] D1: `stop_hooks::a_merge_stopped_in_its_post_merge_hook_has_landed` green with the edit set aside:
      `stopped_at == Some(StoppedAt::Landed)`, entry `Committed`, edit back, `S` dropped. Pick twin:
      `cherry_pick::a_pick_over_edits_stopped_after_it_committed_has_landed`. Both red before. — both seen red
      (`stopped_at` `None`) before `apply/close.rs`'s `stopped::mark` over `closed_set_aside`, green after; each also
      asserts `u` exact.
- [x] D2: `stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back` green: the reason is
      `stopped::BEFORE_PAUSE`'s words, edit back, stash empty, `u` restores. Pick twin:
      `cherry_pick::a_pick_over_edits_stopped_before_git_paused_it_says_so`. Both red before. — both seen red
      (`git was stopped`) before `set_aside::Failed { error, reason }` + `PlanFailure::local_reason`, green after.
- [x] The 7 paused tests (NOTES 19d-a "Paused") assert R6's claim, names unchanged. While paused: `S` listed by
      commit, the tree at `HEAD`'s copy. After resume: back, split kept, `S` dropped.
      `a_paused_merge_aborted_by_resume_gives_his_edits_back` and
      `a_paused_merge_continues_to_the_merge_git_would_have_made` stay green. — **5 of 7 green** on R6's claim:
      `stop_hooks::a_merge_stopped_in_its_{prepare_commit_msg,pre_merge_commit}_hook_is_paused`,
      `stop_hooks::a_resume_stopped_in_its_hook_stays_paused`, `quit_waits::stuck::stop_now_mid_merge_leaves_it_paused`,
      `quit_waits::stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`. The other 2 assert the claim
      (paused half passes) but stay red at their captures: box below. The two named stay green. — **19d-a3: 7 of 7
      green**: `quit_waits::capture_relaunch_after_a_stopped_merge` and
      `quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused` (its post-abort `S`-dropped check now
      reached) green after the re-shoot; full run 6155/6155.
- [x] Clean-tree twins: `stop_hooks::a_clean_merge_stopped_in_its_prepare_commit_msg_hook_is_paused`,
      `stop_hooks::a_clean_merge_stopped_in_its_pre_merge_commit_hook_is_paused`. They carry 13c's old "untouched"
      claim with nothing set aside. — green on first run (twins of shipped behaviour).
- [x] Relaunch after a stopped merge with edits aside offers `c`/`a`. Its stop line is drawn as the screen holds it,
      and NOTES names the rung: `quit_waits::stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`. —
      rung ` · edits aside`; asserts recovery left the entry live (`live_entries == [merge]`, `Pending`, no
      `interrupted_edits`, `S` listed) and `S` dropped after `a y`.
- [x] Captures re-shot, diff read, only set-aside words changed, captions updated:
      `capture_relaunch_after_a_stopped_merge` and the hangup capture's test. — **STOPPED (R6: other cells changed).**
      Diff read; files restored to HEAD, not re-shot. NOTES 19d-a2 "Captures". — **19d-a3: re-shot under §R7.2**
      (which ruled the other cells right), diff read, captions rewritten: `quit_waits::capture_relaunch_after_a_stopped_
      merge`, `quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`. NOTES 19d-a3 "Step 3".
- [x] Clash notice: a table over every kind that sets aside (rebase, rewrite, cherry-pick, checkout, switching
      branch-create, pull, recover, merge), each worded from its kind name, no `_` arm. Red before for pick and pull:
      graft `clash::tests::the_clash_notice_names_each_set_aside_kind`. — red before for rewrite, cherry-pick, pull,
      recover (`rebase`); green after. **Deviation:** checkout and switching create keep the shipped `switch`, not
      `checkout`/`branch` (NOTES 19d-a2 "Clash notice"); needs the conductor's yes.
- [x] Up-to-date merge over edits: `merge_set_aside::an_up_to_date_merge_over_edits_sets_nothing_aside`. Green as
      found → say so. Red → fixed per R6. `y` offered → stopped and reported. — **red as found** (`set_aside` `Some`);
      fixed in graft-ops `preview::plan` (`up_to_date_merge`). No `y`: the app's nothing-to-do box offers Close only
      (graft `update/preview.rs:353-358`, read, not run).
- [x] Spawn box re-ticked against 5 / 3: `the_merge_preview_forecasts_the_put_back`. The ADR-0045 19d amendment says
      5 / 3 and names D1/D2's tests. MAP :263's "Not reached: … a set-aside's close" is removed. — the test asserts
      `PUT_BACK_SPAWNS` (5) and `FF_PUT_BACK_SPAWNS` (3); ADR "19d-a2 (ticket R6)" paragraph; MAP :263 rewritten.
- [x] All four gates workspace-wide, including every test NOTES 19d-a lists red. — fmt, clippy pass; nextest red on
      the 2 capture tests only — 6146/6148, both at `check_shot` (`graft-fixtures/src/capture.rs:147`); doc tests
      pass (NOTES 19d-a2). — **19d-a3 run** (a + a2 + a3 on main fa0111ca): `cargo fmt --check` ok; `cargo clippy
      --workspace --all-targets -- -D warnings` ok; `cargo nextest run --workspace` **6155/6155** (14 skipped); `cargo
      test --workspace --doc` ok. Every test NOTES 19d-a lists red is in that run, green.
**Touches (check each, say how):** every set-aside kind's close (run `set_aside.rs`, `cherry_pick.rs`,
`pull_set_aside.rs`, `absorb_staged.rs`, `merge_set_aside.rs`: green, no expectation changed except those named);
`ApplyError::stopped_at` callers in graft (the stop words the app shows after a stop); recovery (a paused set-aside
entry is `live`, left alone by `recover_interrupted`: one assertion in the relaunch test); undo of a `Landed`
set-aside entry (D1 test's `u`); the stop line rungs; the two captures and any contact sheet holding them (the
conductor rebuilds); KNOWN_GAPS (the conductor closes the clash-notice row and files nothing for D1/D2 once green).

## 19d-a3 — Landed through a clash, the clash notice's fallback, the two captures (Opus xhigh, ~35k)
**Base:** branch `worktree-agent-a67f6d8f405f4ef37` at 47f5a389 (19d-a2, partial, not merged). a, a2 and a3 merge
together. Write the NOTES under `## NOTES 19d-a3` in this file on that branch.
**Order inside the part:** steps 1 and 2 now. Step 3 only after 19d-b2 has merged into the base. If b2 hasn't merged,
commit steps 1-2, report `blocked: needs 19d-b2`, and stop. Don't wait in place.
**Crate set:** `crates/graft-ops`: `apply/stopped.rs`, `apply/error.rs` (the `Clashed` field), the places that build
`Clashed` (`apply/close.rs`, `apply/set_aside*`; grep `Clashed {`), tests `tests/it/stop_hooks.rs`, MAP (`Clashed`
line :258, `stopped_at` :260). `crates/graft`: every match on `ApplyError::Clashed` (grep), `update/clash.rs` (the
fallback only), `tests/it/quit_waits/{stuck,hangup}.rs` and the capture test, the two captures, graft MAP if a line
names them. **Not** `tests/it/pty.rs` (19d-b2's).
**Read first:** §R7; NOTES 19d-a2 on the branch ("Captures", "Clash notice", "Findings"); graft-ops MAP :257-263; graft
MAP :504, :646.
**Out:** the pick-post-commit hold and the resume residual wording (§R7, filed by the conductor); 13c's pause rule;
any new words; masking a stash id.
**Acceptance:**
- [x] Step 1, Landed through a clash, red before: a set-aside merge over an edit that overlaps a path the merge changes
      cleanly, with its post-merge hook stopped, gives `ApplyError::Clashed { stopped_at: Some(StoppedAt::Landed), .. }`.
      The entry is `Committed`, `S` is kept, and the clash is live:
      `stop_hooks::a_merge_that_landed_before_the_stop_and_clashed_on_put_back_says_landed`. `u` is exact (same test).
      Every other `Clashed` constructor gives `None` (`merge_set_aside::a_merge_put_back_that_clashes_stops_as_a_clash`
      asserts it). — red before (`stopped_at()` `None`), green after; `u` → `whole == before`. `None` asserted for
      both constructors: the set-aside answer by `merge_set_aside::a_merge_put_back_that_clashes_stops_as_a_clash`, the
      pop's `close::clashed` by `stash_clash`'s `clashed` helper (11/11).
- [x] Step 1, callers: NOTES lists each graft caller of `stopped_at()` and what it shows for this case. No new words.
      A caller that would need them → stopped and reported. — one caller (`jobs/operate.rs:424`), one reader (the
      quit exit line, `update/quit.rs:298`): now the shipped Landed line, no new words. It does not name the clash:
      filed as a **question** in NOTES 19d-a3, not a stop (keymap §2.2 has the line say where git was left).
      Covered by the full run (`update::quit::tests` exit-line table unchanged, green).
- [x] Step 2, the unknown-kind fallback names no kind (§R7.1). The shipped None test's expectation is changed and named
      in NOTES. Test: `clash::tests::an_unknown_kind_names_no_kind`. Checkout and switch-create still read `switch`:
      `clash::tests::the_clash_notice_names_each_set_aside_kind` unchanged. — red before (`… u undoes the rebase
      too`), green after (`… esc, u puts it all back`); `the_clash_notice_names_each_set_aside_kind` green, unchanged.
      Expectation changed: `a_put_back_clash_lands_on_the_resolver_with_set_aside_names` (`resumed`); fixture changed:
      `banner::tests::resume_words::a_continue_whose_put_back_clashes_lands_on_the_clash` (NOTES 19d-a3).
- [ ] Step 3 (after 19d-b2), relaunch capture: the paused merge's staged paths equal the merge's own changed paths
      (asserted, §R7.2), and the stash row is drawn:
      `quit_waits::capture_relaunch_after_a_stopped_merge`. — **half true, left open.** Staged paths asserted and green
      (`stuck::assert_staged_are_the_merges_own`, called by that test). **The stash row is not drawn** in this 80x24
      capture: the landing is the `Detail` tab (`Graph  [Detail]`). Finding in NOTES 19d-a3; the conductor's call.
- [x] Step 3, hangup capture: the same assertions, plus the post-abort `S`-dropped check, now reached:
      `quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`. — staged paths asserted; the stash
      row drawn (`[stash@{0}] On feature: graft: your edits, set aside for merge main into feature … now  84aed18`);
      `relaunch_and_abort` and `assert!(!listed(&repo, &stash))` reached and green.
- [x] Step 3, stability: each capture test run twice with `GRAFT_UPDATE_SHOTS=1`, both outputs byte-identical. No
      `future` / `futu…`. The diff from HEAD is only the cells §R7.2 names (stash row; `1 unstaged` / `UNSTAGED (1)
      M base.txt` gone; the ` · edits aside` rung at 120×40). Captions rewritten. Files:
      `docs/shots/m8-hangup-relaunch-120x40.txt`, `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt`. Colour
      **unverified**; ≤ 200 KB. — both tests above, twice each under `GRAFT_UPDATE_SHOTS=1`, the second run passing
      with nothing rewritten, `cmp` identical; `grep -c futu` 0; 2049 B, 536 B. The 120x40 needed `S` dated: in
      `quit_waits/pty_session.rs` `Session::spawn` (NOTES). Diff also moves the uncommitted row's glyph one lane (the
      stash takes the lane; named in NOTES 19d-a2 and 19d-a3).
- [x] All four gates workspace-wide; 19d-a2's open gates box ticked here with the run's counts. — fmt ok, clippy ok,
      nextest `--workspace` 6155/6155 (14 skipped), doc tests ok. After that run one test file changed
      (`stash_clash.rs`, the `None` assertion): fmt, clippy (workspace) and `stash_clash` 11/11 re-run, green.
**Touches (check each, say how):** every construction and match of `ApplyError::Clashed` (grep in both crates; the
compiler's exhaustiveness is the check); `ApplyError::stopped_at` callers; the resolver landing on a clash
(unchanged; `a_put_back_clash_lands_on_the_resolver_with_set_aside_names` green); the m6 `switch` captures and
`TESTING.md:2598`, `:2724` (unchanged, by §R7.1); the two captures and any contact sheet holding them (the conductor
rebuilds the M8 sheet); KNOWN_GAPS (the conductor closes the Landed-through-clash row once green, and files the
fallback-reachability row as unverified).

## 19d-b — the session clock pinned for pty runs (Sonnet, ~25k)
**Crate set:** `crates/graft` (`session` beside `Clock`; the launch path that builds the session clock and
`JobRunner::with_clock`; `tests/it/pty.rs`), graft MAP :91/:1116.
**Verify first:** grep `Clock::Wall` in `crates/graft/src`. If the binary already reads a clock pin, stop and report
(R3's premise is false).
**Acceptance:**
- [x] `clock_from`: unset → `Wall`; `"1609459200"` → `Fixed(1609459200)`; `"x"`, `""`, `"1.5"` → `Err` with the words
      in R3: `session::tests::clock_from_reads_whole_seconds_only`. (ticked: that test, + `-3` and overflow)
- [x] The binary obeys it: pty, the same repo with `GRAFT_NOW` = two instants a year apart, draws two different ages
      on one commit row: `tests/it/clock_pin.rs::graft_now_pins_the_drawn_age`. A bad value → exit 2 with the
      words, no screen: `…::a_bad_graft_now_refuses_to_launch`. (ticked: both, `tests/it/clock_pin.rs`; 2024-01-01 draws `4y`, 2025-01-01 `5y`)
- [x] Journal stamps follow it, and `u` still takes the newest entry with every entry in one second: two commits then
      `u` under the pin undoes the second: `…::u_under_a_pinned_clock_undoes_the_newest`. Red → stop, report. (ticked: green first run; 4 entries, all `at_unix_ms` = pin)
- [x] `Pty::spawn*` defaults the pin. The whole suite is green with **no capture changed**
      (`git status docs/shots` clean); `PINNED_NOW` named in NOTES. (ticked: full nextest 6127/6127, `git status --short docs/shots` empty)
- [ ] Sweep: every `docs/shots/` capture that draws a year age (`\b\d+y\b`) maps to a pinned producer (pty default, or
      a `Harness` with `Clock::Fixed`). The list goes in NOTES. More than 10 unpinned harness producers → stop and
      report the count. **Moved to 19d-b2** (planner 2026-10-08): the stop condition fired, and §R4 rules the split.
**Touches:** every pty test (default env), the 19c-c committed captures (`m8-tour-step1-120x36.txt`,
`m8-tour-80x24.txt`: unchanged), G-rename 27's `GRAFT_*` list, KNOWN_GAPS `M8/19c-capture-age-roll` (the conductor
closes it after 19d-b2, naming `graft_now_pins_the_drawn_age` and `no_capture_producer_runs_on_the_wall_clock`).

## 19d-b2 — every capture producer on the pinned clock; in-app commits dated (Sonnet, ~45k)
**Crate set:** `crates/graft-fixtures` (`PINNED_NOW`, MAP line); `crates/graft` (test and example code only:
`tests/it/pty.rs`, `tests/it/app_harness.rs`, `tests/it/clock_pin.rs`, the 15 test files in NOTES 19d-b :272,
`examples/m3_walk.rs`, `examples/m4_walk.rs`, `examples/m5_walk.rs`, `examples/gallery.rs`, the `#[cfg(test)]` modules
`src/view/tests.rs` and `src/update/tests.rs`); `docs/shots/FROZEN.md` (new); graft MAP (the `app_harness` and
`pty` test entries). **No production file.** If one needs to change, stop and report.
**Schedule:** after 19d-b. It shares the `crates/graft` tests with 19d-a, 19d-a2 and 19d-a3. Before running them
concurrently, the conductor checks the file sets are disjoint. 19d-a's app test goes in a new file. `tour_tasks.rs`
belongs to 19d-a, and `quit_waits/*` to 19d-a2 and a3.
**Read first:** this file §R3-§R5 and NOTES 19d-b; graft MAP :91, :182, :1116; graft-fixtures MAP :32 and the
`capture` lines; ADR-0037 (contact sheets).
**Acceptance:**
- [x] `graft_fixtures::PINNED_NOW` = `1_791_417_600`, and `tests/it/pty.rs` uses it (no second literal; grep shows one):
      `clock_pin::graft_now_pins_the_drawn_age` still green. (ticked: `grep -rn 1_791_417_600 crates` → one hit, graft-fixtures lib.rs; that test green)
- [x] `App::open_with_clock` pins the frame. In-process, an `App` opened with `Clock::Fixed(t)` and with
      `Clock::Fixed(t + 1y)` draws two different ages on one commit row:
      `clock_pin::open_with_clock_pins_the_drawn_frame` (red before). (ticked: green; red-before NOT observed — seam fixed before the test ran, see NOTES)
- [x] All 19 producers in NOTES 19d-b :269-273 build on `PINNED_NOW`, and the guard is green:
      `clock_pin::no_capture_producer_runs_on_the_wall_clock`. It bites: adding `Clock::Wall` to one producer fails it.
      Run that once, revert, and say so in NOTES. (ticked: guard green; bite run in NOTES; + `the_wall_clock_scan_names_each_unpinned_shape`)
- [x] An in-app commit in a pty run with no date env draws its age, not `future`:
      `clock_pin::an_in_app_commit_under_the_default_pin_draws_an_age` (row reads `1h`; red before).
      `u_under_a_pinned_clock_undoes_the_newest` still green. (ticked: row reads `60m`, the label for 3600 s; both green; red before not run separately)
- [ ] Full suite green. `git status --short docs/shots` is empty except `FROZEN.md`, or every changed capture is listed
      in NOTES with only age cells changed. `grep -l future docs/shots` → none, or each hit named in NOTES with why.
- [x] `docs/shots/FROZEN.md`: every year-age capture with no producer, one line each (§R4), with the method that built
      the list named in NOTES. Each sheet that is a contact sheet is **not** listed. (ticked: 91 lines; method in NOTES)
**Touches:** every pty test (two more default env vars; a caller naming them wins, which keeps 19c-c's and 19d-c's
dates); every in-process capture test listed; the gallery and walk examples' captures (expected unchanged); G-rename
27 (no new `GRAFT_*` read); KNOWN_GAPS `M8/19c-capture-age-roll` (the conductor closes it); the contact sheets
(unchanged; their parts are unchanged).

## 19d-c — the whole-tour walk (Opus, ~35k) — last
What remains of 19c-c once a and b land: its four unbuilt boxes (19c-tour-remainder.md :195-214, the walk and two
captures). Start from `.scratch/m8-polish-and-ship/handoff/19c-c-whole-walk.rs.txt`:
- Move its test, `Walk` and `Before` into `tests/it/tour_walk.rs`. Drop the `PROBE` eprintln (:507-511). Use the
  committed `tour_tmp` / `settled` / suffix-only `steady`. Rely on 19d-b's pin and don't set `GRAFT_NOW` by hand.
- `merge_feature` and `merge_clash_to_the_stop` also wait for the set-aside row (19d-a's words), and assert
  `git status --porcelain` in `work` reads `M  notes.txt` after task 6 and after task 7's finish (the split kept).
- **Box reworded (ruled):** "all 8 ticked in the drawn card" can't be seen, because `update/tour.rs:100-101` leaves the
  tour in the same update that stores the eighth tick (19c-c NOTES :240-242). It reads instead: 7 ticks drawn and
  journal-read at task 8 (`on_task(7)`), then home with `practice done ✓ · ⌃P tour goes again`, which only an
  all-eight read sets. Same test.
- Run with `GRAFT_UPDATE_SHOTS=1` once, read both new captures, then run without it. The step-7 caption names the
  ` · edits aside` stop line if drawn.
**Acceptance** (`crates/graft/tests/it/tour_walk.rs`):
- [x] Whole tour by the card's keys from his repo, 7 drawn + finish status, cursor home:
      `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home`. (ticked 19d-c: `on_task(n)` waits for each card line
      and asserts `tour_progress` = first n; at task 8 `drawn_ticks == Some(first(7))`; home waits for
      `practice done ✓ · ⌃P tour goes again`, no `practice 8 of 8`, cursor subject = the one he left.)
- [x] `his_repo_is_untouched`, `state_dir_is_unchanged`, `practice_folder_is_gone`: same test. (ticked 19d-c, the
      three calls after `quit_cleanly`.)
- [x] `notes.txt` staged after tasks 6 and 7: same test, assertion `the_split_survives_both_merges`. (ticked 19d-c:
      tracked `git status --porcelain` = exactly `M  notes.txt`, called after task 6's merge and after task 7's finish;
      both merge previews also waited on `your edits: 1 file set aside, put back after`.)
- [x] Captures `docs/shots/m8-tour-step7-conflict-120x36.txt`, `docs/shots/m8-tour-finished-120x36.txt`, each
      captioned with what right looks like, colour **unverified**, ≤ 200 KB: same test's `shot` calls. (ticked
      19d-c: 2029 B and 2131 B; written once under `GRAFT_UPDATE_SHOTS=1`, read, then green twice without it.
      Step-7 caption says the stop line is **not** drawn there — see NOTES.)
**Touches:** `tests/it/main.rs` (one `mod` if not present), ticket 19 §A9 boxes (the conductor ticks them naming these
tests), KNOWN_GAPS unjudged rows for the two captures (the conductor files them).
**Not ruled here, for the conductor to file from 19c-c NOTES :254-260:** no ticks row at 120×36; `work·` with no gap
at 80×24; `~/code` under the header; card line 1 "top row" while the cursor is on `main`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`:
workspace-wide, every part.

## Size
a ~60k (done but blocked) · a2 ~45k (partial) · a3 ~35k · b ~25k · b2 ~45k · c ~35k. Each fits one context. If one
compacts, say so in NOTES.

## 19d-c additions (conductor 2026-10-08, from 21c Part B hand-back — stale outputs, no new decision)
- [ ] C+1 (**not done, 19d-c: no M8 contact sheet exists** — no `docs/shots/m8-contact-sheet*`, no captions TSV;
      ticket 28 / `28-amendment.md` :19-22 builds it. Nothing to gain rows. The three 21c captures and this part's two
      tour captures go in 28's TSV.) the M8 contact sheet gains `docs/shots/m8/21c-status-partial-fetch-120x40.txt`, `21c-journal-partial-fetch-120x40.txt`, `21c-journal-partial-fetch-80x24.txt` (ticket order, each under its "what right looks like" line).
- [ ] C+2 (**partial, 19d-c**: m7 sheet rebuilt by `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`;
      `TESTING.md:1769` fetch words updated to what the re-shoot drew. **Not done:** the m4 pty capture — its re-shoot
      (`.scratch/m4-remotes/pty-walk-m4.py {80x24,160x48} journal-rows`) draws every Graft journal row's age as
      `future` (NOTES); not committed, so `scripts/contact-sheet.sh m4` rebuilt the m4 sheet byte-identical.) rebuild the m7 contact sheet (9 m7 captures were regenerated by 21c B) and the m4 pty capture + m4 sheet 21c B left stale; `TESTING.md:1769` updated to match. Name the test or script that rebuilt each at tick.

## 19d-c addition (planner 2026-10-08, §R5)
- [x] C+3 (ticked 19d-c: `env()` in `tour_walk.rs` sets both to `EPOCH + 3600`; `no_future(&conflict)` and
      `no_future(&home)` in `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home` before each `shot`; green.
      `grep -c future` on both captures: 0.) the walk keeps its own `GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE` (`EPOCH + 3600`, draft :87-94). They
      override 19d-b2's pty default, so the commits from tasks 2, 6 and 7 draw as years. Neither new capture contains
      `future`. Test: the same walk; assert `!screen.has("future")` before each `shot`.

## NOTES 19d-b (builder, 2026-10-08)
- Verify first: premise true. No pin in the binary: `Clock::Wall` hardcoded at `event_loop/places.rs:263` (practice `now`) and `jobs/runner.rs:183`; `Harness::default()` at `places.rs:144` and `main.rs` headless path.
- `PINNED_NOW = 1_791_417_600` (2026-10-08 00:00 UTC), `tests/it/pty.rs`. Set before the caller's `env`, so a caller naming `GRAFT_NOW` wins.
- Deviation from Signatures: `clock_from` is `pub(crate)` as specced, but `main.rs` is a separate binary crate, so it calls a new `pub fn session::launch_clock()` (reads env, sanitises). `places::drive` gains a `Clock` param. The practice repo's `now` now uses the session clock (`Carried::clock`), not `Clock::Wall`, so a pinned tour without `GRAFT_PRACTICE_EPOCH` doesn't date its repo in the pin's future.
- Observed, not a defect of this ticket: a commit made **in-app** during a pinned run carries git's wall-clock date, so it draws `future` when the pin is earlier than now (seen in `u_under_a_pinned_clock_undoes_the_newest`). With the default pin, every run after 2026-10-08 shows this for commits it makes. No committed capture changed in the full run. The capture walks pin `GIT_*_DATE` themselves, or draw such rows as `future`. **Unverified** whether any later re-shoot hits it.
- **Sweep box NOT ticked: stop condition hit.** 19 unpinned producers write year-age captures (>10). The scout report (scratch, not committed) found these:
  - examples m3_walk, m4_walk and m5_walk use `..Harness::default()` (`m3_walk.rs:168`, `m4_walk.rs:195`, `m5_walk.rs:214`). They do not go through `main`, so `GRAFT_NOW` doesn't reach them.
  - gallery git-git, workspace and 100k (`gallery.rs:53`, `:1007`).
  - tests: remote_by_key, partial_fetch, radar_bookkeeping, status_under_a_box, daily_landing, preview.rs (time machine), offer_and_blame:172, src/view/tests.rs, update/tests.rs (doorway), keymap_edge/palette.rs, palette_find.rs, remaps.rs, theme_problems.rs, settle.rs, pixel_preview.rs.
  - `App::open_with_clock` pins only the JobRunner, not the frame (`app_harness.rs:154`).
  - About 70 year-age captures map to no current producer: old numbered shots, 35-/38- tiers, older m1/m2, contact sheets.
  - The conductor decides the follow-up. Scout used: yes, for this sweep only.

## NOTES 19d-b2 (builder, 2026-10-08)
- Scout: none; NOTES 19d-b's sweep was the scout. Instead, an **empirical sweep**: Clock::Wall temporarily shifted +2y in `session.rs` (reverted, `git diff` on it empty), full nextest: 29 failures = every in-process capture still on the wall. Each fixed, rerun green under the shift.
- Seam: `App::open_with_clock` sets `state.harness.clock`. Most in-process producers used `App::open` (Wall) or `..AppState::default()`, which neither the spec's sites nor the guard cover. Fixed per site: `open_with_clock(.., Fixed(PINNED_NOW))` in remote_by_key/partial_fetch/status_under_a_box/daily_landing/preview capture tests and offer_and_blame:100; `view::tests::draw_pinned` (wall-clock state drawn at PINNED_NOW) for view/tests, keymap_edge palette/palette_find/remaps, update doorway, theme_problems; harness clock set in settle.rs and pixel_preview; examples m3/m4/m5; gallery real repos (`corpus_stops_..._real_repos_view_the_pinned_clock`, renamed).
- **Not pinned, deliberately**: `capture_undo_after_push`, `capture_fix_plain_box_after_rebase`, `radar_bookkeeping`, `capture_time_machine`, `commit_editor_writes`, `box_owns_the_bar`: they commit in-process with git's wall date and draw `now`; pinning the frame turns that into `future`/`2y`. Fixing needs dates on the in-process git child: via job runner env = production hook → §R5 says stop. **Open, unverified-when-it-rolls**: they draw `now` only while wall ≈ commit time, so they don't roll; their fixture (2021) rows still roll on the wall clock. Conductor's call.
- Guard scope gap: it flags `Clock::Wall` and unpinned `Harness::default()` only; `App::open(` and `..AppState::default()` in capture files are not caught (flagging App::open hit ~250 sites).
- Bite: `fn bite() -> Clock { Clock::Wall }` appended to daily_landing.rs → guard failed naming `tests/it/daily_landing.rs:236`; reverted.
- R5 label: 3600 s draws `60m`, not `1h`.
- Captures: none changed (`git status --short docs/shots` = only FROZEN.md). `grep -l future docs/shots`: 6 pre-existing hits, unchanged, prose (e.g. m7/landed-after-merge: `YYYY-MM-DD hh:mm +zzzz · future`) — not ages checked for all 6.
- FROZEN.md method: python over `docs/shots/**` (skip names containing `contact-sheet`): file has a word `\d+y`; producer = name, stem, stem without `-CxR`, or without leading `NN-`, found in any `crates/**/*.rs` or `scripts/*`; date `git log -1 --format=%cs`. 91, not ~70: heuristic, names built at run time may be listed as frozen wrongly (unverified).
- **Failing, not mine**: `tour_tasks::task_4_ticks_by_setting_aside_and_bringing_back` fails deterministically, also with the new git-date env removed (checked); no production change here. Cause unknown; 19d-a's file. Full: 6130/6131.
- Red-before for the two new behaviour tests not observed (written after the fix).

## NOTES — 19d-a (builder, 2026-10-08)
**Status: partial — blocked on 9 red 13c stop-in-hook tests outside this crate set.** Every 19d-a box below the
blocker is green. **Compacted twice** mid-ticket. Remaining: the blocker only (rule needed, below).
Skills: `tdd` (Skill tool, confirmed). `implement` / `code-review`: can't confirm after the compactions. Scout used: `scouts/merge-preview-dirty-index.md`.

**Blocker — the ruling meets 13c's fixtures (ticket and code disagree; stopped, not changed).** 13c's stop-in-hook
fixture (`crates/graft-ops/tests/it/stop_hooks.rs`, `diverged()`) leaves an unstaged edit to `notes.txt` (`HIS_EDIT`)
the merge doesn't touch. Under R1 that edit is now set aside. Nine tests go red; the Touches list didn't name 13c.
- **Paused (7 tests: the edit waits in `S`, not the tree, until resume).** graft-ops `stop_hooks::`
  `a_merge_stopped_in_its_prepare_commit_msg_hook_is_paused` (`stop_hooks.rs:243` "his edit is untouched"),
  `…_pre_merge_commit_hook_is_paused`, `a_resume_stopped_in_its_hook_stays_paused` (`:718`); graft `quit_waits::`
  `stuck::stop_now_mid_merge_leaves_it_paused`, `stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`
  (`quit_waits/stuck.rs:319`, `:347`), `capture_relaunch_after_a_stopped_merge`,
  `hangup::a_hangup_then_a_second_hangup_leaves_the_merge_paused`. Captures
  `docs/shots/m8-hangup-relaunch-120x40.txt`, `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt` now differ (not
  re-shot). Resume still puts it back: `a_paused_merge_aborted_by_resume_gives_his_edits_back` and
  `a_paused_merge_continues_to_the_merge_git_would_have_made` stay green. This is the ruling working as built; the
  expectations predate it.
- **D1 — Landed loses `stopped_at` (defect, pre-existing for every set-aside kind).**
  `a_merge_stopped_in_its_post_merge_hook_has_landed`: `stop_hooks.rs:322` expects
  `Some(StoppedAt::Landed)`, gets `None`. Cause: `crates/graft-ops/src/apply/close.rs:100-101` returns
  `closed_set_aside(...)` before `:150`'s `stopped::mark(stopped_at, …)` is reached. Probe: entry `Committed`, edits
  back, `S` dropped — only the mark is lost.
- **D2 — BeforePause loses its reason (defect, pre-existing for every set-aside kind).**
  `a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back`: `stop_hooks.rs:357` expects
  the before-pause words, gets "git was stopped". Cause: `close.rs:176` `failure.map(|failed| failed.error)` drops
  `PlanFailure.before_pause` (`apply/run.rs:37`). Probe: edits back, stash empty, `u` restores (rest of the test green).
- **Options for the conductor:** (a) a follow-up in `apply/close.rs` + `apply/set_aside` fixing D1/D2 (cause), the
  paused expectations rewritten to "the edit waits in `S`, back on resume", the two captures re-shot; or (b) narrow R1
  (e.g. only files the merge touches) — contradicts R1 and the cherry-pick rule. I recommend (a).

**Changed expectation:** `operation/history/tests.rs` `sets_aside_table`: `merge()` moved from the `None` row to the
`Some` row. Why: R1 itself.

**Deviation — spawns.** R1 says "≤ 4 extra spawns"; measured **5** on a true merge (`stash create`, `merge-base`,
`merge-tree`, `commit-tree`, `merge-tree`) and **3** on a fast-forward (`stash create`, `merge-base`, `merge-tree`).
The fixed `merge_landed` signature carries no base, so it spawns its own `merge-base` (the merge forecast's base is not
reachable from it). Clean tree: spawn list unchanged (asserted). Proposal: baseless `merge-tree --write-tree HEAD
<source>` (git picks the base) → 4. `merge_landed` returns `MayStop` when there is no merge base.

**Defect (out of scope, "no new words"):** the clash notice still says "u undoes the rebase too" for cherry-pick and
pull: `crates/graft/src/update/clash.rs` `undone_with_edits`, `_ => "rebase"` arm.
**Question:** an up-to-date merge over edits may draw the set-aside row inside a nothing-to-do box
(`update/preview/plan_lines.rs:50` vs `update/preview.rs:358`). Not run.

**Touches (how checked):** journal/undo — one entry, `u`/`U` exact on the §7 table (`undo_/redo_of_a_set_aside_merge_
round_trips`), StashDrop only when `refs/stash` ended at `S` (same); `touches` adds `refs/stash` (`a_merge_over_a_clean_
tree_sets_nothing_aside` checks its absence). Recovery: `recover.rs` has no kind list (grep). Stop line ` · edits aside`:
generic via `has_edits_aside`/`is_set_aside_entry` (`wording.rs:185-207`). `edits_back`: generic (`clash.rs:452`);
`resumed_words` uses it (`resume.rs:369`). `tour_progress` reads Committed Merges (`tour.rs:271`); `tour_tasks` green,
no expectation changed. `plan_lines.rs`: not kind-gated, unchanged. Merge-preview captures (`m5-02-preview-merge-*`,
`m6/fix-preview-verb-120x40`, `m7/refused-box-single-close-120x40`, contact sheets): clean fixtures, their tests green,
unchanged. Keymap row `m` amended; no key bound or freed. Never `--autostash`.

## NOTES 19d-a2 (builder, 2026-10-08)
**Status: partial — stopped on the two captures (R6 "any other change → stop").** Everything else is built and green.
**Compacted once** mid-ticket. Remaining: the captures' ruling, then re-shoot and the gates box. Scout: none new; NOTES
19d-a served as the scout. Skills: `tdd` (Skill tool); `implement` (SKILL.md read). Base: 19d-a merged in (`bae045e3`).

**Captures (stopped, not re-shot; files restored to HEAD).** Read with `GRAFT_UPDATE_SHOTS=1`, then `git diff`:
- `m8-hangup-relaunch-120x40.txt`: the rung ` · edits aside` (expected), **plus** a new graph row
  `■ [stash@{0}] On feature: graft: your edits, set aside for merge main into feature  GS futu…<id>`, the header
  `Uncommitted changes (2 staged, 1 unstaged)` → `(2 staged)` (its glyph column shifts), `UNCOMMITTED · 2 staged
  1 unstaged` → `· 2 staged`, and the `UNSTAGED (1) / M base.txt` section gone.
- `m8-quit-stopped-merge-relaunch-80x24.txt`: no rung at all (80 cols shows only the keys); only the
  `1 unstaged` / `UNSTAGED (1) M base.txt` removals.
- **Not stable either:** two regenerations gave the stash row `1d83f5a` then `ba8d6d3` — `S` is a wall-clock stash
  commit — and its age reads `futu…` against `FORK_NOW`. So re-shooting needs `S`'s dates pinned too (19d-b2's R5
  harness default, or the fixture's `GIT_*_DATE`) before any capture can be byte-stable. The two tests
  (`quit_waits::capture_relaunch_after_a_stopped_merge`, `quit_waits::hangup::a_hangup_then_a_second_hangup_leaves_
  the_merge_paused`) are red at `check_shot` only: the paused half of each asserts R6's claim and passes. The
  hangup test's post-abort `S`-dropped check sits after the capture and is **unverified** (not reached); the same
  abort path is proven in-process by `stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`.
- Every change is a true consequence of R6 (his edit is in `S`, not the tree). Options: (a) accept these cells as
  "set-aside words" and pin `S`'s dates, then re-shoot and recaption (the 80x24 caption's "his edit unstaged
  (`M base.txt`)" goes); (b) something else. Captions untouched.

**Clash notice — deviation (needs a yes).** `undone_with_edits` (`crates/graft/src/update/clash.rs`) is an exhaustive
match over `OperationKind`, no `_` arm, worded from graft-ops `apply::kind_name` (made `pub`, re-exported from `apply`,
graft-ops MAP line added) — **except** `Checkout | BranchCreate`, which keep `switch`. Why: `kind_name` gives
`checkout` and `branch`; `u undoes the branch too` reads oddly, and `switch` is the shipped, by-eye-approved word in
four committed captures (`docs/shots/m6/m6-23-set-aside-clash-landing-{80x24,120x40,187x50}.txt`,
`docs/shots/m6/set-aside-clash-landing-120x40.txt`), two contact-sheet entries, `TESTING.md:2598`, `:2724`, and tests
outside this part (`m6_stash_put_back/set_aside.rs:828`, `m5_walk/stash_journeys.rs:818`,
`update/resolver/tests.rs:885`). Taking `kind_name` there means re-shooting m6 captures outside the crate set. If the
ruling stands literally, it is one arm plus those files. **Odd-reading names otherwise:** `recover` ("u undoes the
recover too"), `rewrite` — listed, not reworded. **Kept:** a kind not known (the journal read lacks the entry) still
says `rebase` (`clash.rs` `let Some(kind) = kind else { return "rebase" }`); the shipped None test
(`a_put_back_clash_lands_on_the_resolver_with_set_aside_names`, `resumed`) expects it. Whether it is reachable in
the app: unverified (the journal stays `Ready` across refreshes, `journal/answers.rs:47`). No existing expectation
changed.

**Findings (out of scope, not changed).**
- **Landed mark lost on a put-back clash:** `apply/stopped.rs:248` `Err(other) => Err(other)`
  passes `ApplyError::Clashed` through, so a set-aside op that landed before its hook was stopped *and* whose
  put-back clashed answers no `Landed`. `Clashed` has no `stopped_at` field; adding one is a signature change. Not
  reproduced in a test.
- **Pick stopped in post-commit, hook outside the group → held `Conflicted` with `HEAD` moved.** Observed while
  building the D2 pick twin: `CHERRY_PICK_HEAD` dangles after the commit, so `apply/stopped.rs:116` (`live ==
  Some(reads)` but `HEAD` moved) is neither Paused nor Landed (`:125`), and `close.rs:95` `stopped_on_conflict` holds
  it. 13c's pause rule, out of scope (ticket "Out"). The D2 pick twin uses a stuck merge driver instead.
- **Resume's set-aside close still words a residual stop `git was stopped`** (`apply/resume/close.rs:53`
  `resume_reason` vs `:91` `Failed::git`). **Unverified:** no deterministic repro — a merge resume ends paused or
  landed, a pick `--continue` commits through a child `git commit` (landed), a rebase group stop stays live.
- **D2 pick twin's `changed: true`:** a killed merge driver leaves `.merge_file_*` in the tree, so `u` has something
  to restore; the test asserts `u` → `whole == before`.

**Touches (how checked).**
- Every set-aside kind's close: `set_aside`, `cherry_pick`, `pull_set_aside`, `absorb_staged`, `merge_set_aside`,
  `stop_hooks` — 118/118 on that filter; `apply/set_aside/answer/tests.rs` compiles (passes `None`), green.
  No expectation changed beyond the 13c ones named above.
- `apply/run.rs` (`local_reason`), `apply/resume/close.rs` (`Failed::git`, same words as before),
  `apply/set_aside/answer.rs` (`status_for` takes the reason), `preview.rs` (up-to-date rule), `apply/record.rs` +
  `apply.rs` (`kind_name` pub): all graft-ops src, disjoint from 19d-b2's files.
- `ApplyError::stopped_at` callers in graft: unchanged shape; `Git { paused: None }` still maps to `BeforePause`.
  The quit exit line reads `forced.at` (stuck tests green).
- Recovery: asserted in the relaunch test (above). Undo of a `Landed` set-aside entry: the D1 tests' `u`.
- Stop-line rungs: ` · edits aside` drawn at 100x30 (relaunch test), absent at 80x24 (capture diff).
- Captures and contact sheets: none changed in this commit. KNOWN_GAPS: not read (conductor's).
- 19d-b2's files (`tests/it/pty.rs`, `app_harness.rs`, `clock_pin.rs`, `examples/`): not touched (`git status`).

## NOTES 19d-a3 (builder, 2026-10-08)
**Status: done, with one box left open (the 80x24 stash row, below) and one question (the exit line).** Not compacted.
Base: main fa0111ca (R7, 19d-b2, the task_4 fix) + `git merge --no-ff 47f5a389` (a and a2), committed as its own merge;
the ticket conflict kept main's R7/a3 sections and a/a2's ticks and NOTES. Scout: none; NOTES 19d-a2 was the scout
(ticket). Skills: `tdd` (Skill tool), `implement` (SKILL.md read).

**Step 1 — Landed through a clash (cause fixed).** `ApplyError::Clashed` gains `stopped_at: Option<StoppedAt>`;
`stopped::mark` (`apply/stopped.rs`) now carries the mark onto a `Clashed` as it does onto `Ok` and `AfterState`; both
constructors (`apply/close.rs` `clashed`, the pop path; `apply/set_aside/answer.rs` `answer`) build `None`;
`ApplyError::stopped_at()` returns the field. `resume.rs:187` goes through the same `mark`, so a `--continue` that
landed before its post-commit hook was stopped and then clashed says `Landed` too (not separately tested). Red before:
the new test's first assertion, `error.stopped_at()`, read `None` against `Some(Landed)` (run with the destructure
written `{ entry, clash }`, since the field did not exist yet). Compiler-forced `..` in 7 graft-ops test files
(`cherry_pick`, `pull_set_aside`, `put_back_index`, `remote_ops`, `set_aside`, `stash_clash`, `undo_bytes`): no
expectation changed.

**Step 1 — graft callers of `ApplyError::stopped_at()`** (grep `stopped_at()` in `crates/graft/src`): one,
`jobs/operate.rs:424` `cancel.left_at(error.stopped_at())`, read back by `jobs/pool.rs:92` into `Forced::at`, whose one
reader is the quit exit line `update/quit.rs` `exit_line` (`:298`). For this case it now prints the shipped Landed line
`graft: merge … finished · a git hook after it was stopped after N s` (was the `None` line `graft: stopped merge … after
N s · graft here shows where git left it`). No new words. The app's landing is unchanged: `operate.rs` maps
`Clashed { entry, clash, .. }` to `Operated::Clashed` as before (the resolver; `a_put_back_clash_lands_on_the_resolver_
with_set_aside_names` green).
- **Question (not a defect; no line proves wrong logic):** that Landed line says nothing of the clash, and the old
  `None` line at least sent him back to Graft. Keymap §2.2 (`docs/design/02-keymap.md:245`) says the line names where
  *git* was left, which Landed does; the next launch lands on the clash (`relaunch::land_on_the_work`). Naming the clash
  in the shell would need new words, so it is not done here. Conductor's call whether it goes on the usability walk.

**Step 2 — the unknown-kind fallback (cause).** `clash.rs` `undone_with_edits` now answers `Option<&str>`, `None` for a
kind not known. The notice keeps `your edits clashed in N files` and takes the pop's shipped clause `esc, u puts it all
back` (true: `u` undoes the whole entry) — no new words. Ruling read: "drop the `the <kind>` clause" left `u undoes …
too` with no object, so the clause that names no kind is the shipped one; if a different reading was meant, it is one
line in `notice_words` / `undo_clause_words`. `notice_words` gains `set_aside: bool` (it used to infer "your edits" from
the kind word being there).
- **Changed expectation (named, per R7.1):** `clash::tests::a_put_back_clash_lands_on_the_resolver_with_set_aside_names`,
  its `resumed` half: `… esc, u undoes the rebase too` → `… esc, u puts it all back`.
- **Changed fixture, expectation kept:** `update::banner::tests::resume_words::a_continue_whose_put_back_clashes_lands_
  on_the_clash` also reached the fallback (its `AppState` had no journal read), red after the fix with `puts it all
  back`. It now holds the rebase's `Pending` set-aside entry, as the app's journal read does while a rebase is stopped,
  and still asserts `undoes the rebase too`. Outside the ticket's named file list; the only other test that hit the
  fallback (full run).
- Reachability of the fallback in the app: still **unverified** (as 19d-a2 said).

**Step 3 — the two captures (re-shot).**
- `assert_staged_are_the_merges_own` (`quit_waits/stuck.rs`): `git diff --cached --name-only` (the index) equals
  `git diff --name-only HEAD...MERGE_HEAD` (the commits) and the literal `main-one.txt`, `main-two.txt`. Called by both
  capture tests; green.
- **120x40 (hangup) was unstable as 19d-a2 found** (`futu…8ee854d`, wall-dated `S`): the binary there runs under
  `quit_waits/pty_session.rs` `Session::spawn`, not `Pty::spawn`, so 19d-b2's pty defaults never reached it. Fixed at
  the fixture, as R7.2 allows: `Session::spawn` sets `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` to `@FORK_NOW +0000`
  (the instant the capture is drawn at). `S` now draws `now  84aed18`. Not `PINNED_NOW − 3600`: drawn at `FORK_NOW`
  (2021) that would read `future`. `pty_session.rs` is outside the named list (quit_waits/*, this part's); graft MAP
  line 30 updated.
- **80x24 (relaunch) needed nothing:** in-process, `S` is wall-dated, but this landing is the `Detail` tab and draws
  no stash row, so no `S` cell is in it.
- Stability: run 1 (pre-pin) rewrote both; run A (pin) — 80x24 passed under `GRAFT_UPDATE_SHOTS=1` (identical), 120x40
  rewritten; run B — both passed under `GRAFT_UPDATE_SHOTS=1`, and `cmp` against copies of run A: identical. `grep -c
  futu` 0 in both. No home or temp path in either (`grep Users|/private|/tmp|/var/folders`: none; `~/code/api` is the
  fixed `/code/api` drawn, as before). 2049 B and 536 B.
- Diff from HEAD, 120x40: the stash row `[stash@{0}] On feature: graft: your edits, set aside for merge main into
  feature  GS now  84aed18`; `Uncommitted changes (2 staged, 1 unstaged)` → `(2 staged)` **and its glyph moves from
  lane 2 to lane 1** (`▸   ◌` → `▸ ◌`) — the stash takes the lane, the layout consequence 19d-a2 reported with the
  header ("its glyph column shifts"); the banner gains ` · edits aside`; `UNCOMMITTED · 2 staged 1 unstaged` → `· 2
  staged`; `UNSTAGED (1)` / `M base.txt` gone; one blank row moves. 80x24: `· 2 staged`, the `UNSTAGED` pair gone, two
  blank rows at the foot. Nothing else.
- **Finding (textual, stands on the capture):** R7.2 says at 80x24 "the stash row is what tells him there". At this
  landing it is not drawn: `docs/shots/m8-quit-stopped-merge-relaunch-80x24.txt` shows `Graph  [Detail]` and no stash
  row, and the rung does not fit. So at 80x24 nothing on the landing says his edit is aside (the graph tab, `⇥`, may
  draw it: **unverified**). Its box is left open; the caption says so.
- Captions rewritten (`quit_waits.rs` `capture_relaunch_after_a_stopped_merge`, `hangup.rs` `capture_the_relaunch`).
  Colour **unverified**.

**Touches (how checked).**
- Every construction/match of `ApplyError::Clashed`: the compiler (`cargo check --workspace --all-targets` listed 8
  sites, all updated); grep `Clashed {` in graft-ops src: 2 constructors.
- `ApplyError::stopped_at` callers: above (one; one reader).
- The resolver landing on a clash: `a_put_back_clash_lands_on_the_resolver_with_set_aside_names` green.
- m6 `switch` captures and `TESTING.md:2598`, `:2724`: unchanged (`switch` arms untouched;
  `the_clash_notice_names_each_set_aside_kind` green, unchanged).
- The two captures: re-shot; no contact sheet holds them (grep of `*.md`/`*.txt`: issues, a review, a scout, and
  KNOWN_GAPS, not read). The conductor rebuilds the M8 sheet.
- `clock_pin::no_capture_producer_runs_on_the_wall_clock`: green in the full run, with `Session::spawn`'s new pin.

**Gates (workspace-wide, `CARGO_TARGET_DIR` in this worktree).** `cargo fmt --check` ok; `cargo clippy --all-targets
-- -D warnings` ok; `cargo nextest run --workspace` 6155/6155 passed (14 skipped); `cargo test --doc` ok. After that
run, `stash_clash.rs` gained its `None` assertion: fmt, clippy (workspace) and `stash_clash` 11/11 re-run, green; the
full nextest was not re-run for that one test-only edit.
- Scout: none. NOTES 19d-a2 and the files R7 names served; the change is to three graft-ops functions and one graft
  module.
- Not compacted mid-build; the session compacted once **after** the gates, before the ticks (ticks re-checked against
  this NOTES, not from memory).

## NOTES 19d-c (builder, 2026-10-08)
**Status: partial — the walk, its captures and C+3 done; C+1 not possible as worded; C+2's m4 half stopped.** Not
compacted. Scout: none — the draft (`handoff/19c-c-whole-walk.rs.txt`) and NOTES 19c-c served; the change is one test
file. Skills: `implement`/`tdd` not loaded (test-only part; the walk was the test). Base: main 9357e18e.

**Built.** The draft's walk moved into `tests/it/tour_walk.rs` on the committed helpers (`tour_tmp`, `settled`,
suffix-only `steady`, `on_task`); `PROBE` dropped; no `GRAFT_NOW` set (19d-b's pty default). Split into
`off_head`, `Walk::tasks_one_to_six`, `Walk::task_seven` for clippy's line limit. Ran green through task 8 and home on
the first full run after one change (below). Captures written once with `GRAFT_UPDATE_SHOTS=1`, read, recaptioned,
then green on two plain runs (stable). The draft's step-1 shot is not repeated (it is `the_tour_captures_match`'s).

**Finding — defect (behavioural, with line): a merge finished from the finish summary does not say `your edits are
back`.** Journey 2 (this ticket :73-74) says finish `y` → put back → graph, `your edits are back`. The walk drew only
`merged clash into main ✓` (the edits were back: `Uncommitted changes (1 staged, 1 untracked)`, porcelain
`M  notes.txt`). Cause: `crates/graft/src/update/banner/resume.rs:447-449` — `resumed_done` sets
`resumed_words(how, applied)` (whose `also` is `clash::edits_back(applied)`, :369), then on
`FinishAnswer::Merged(Some(words))` replaces the whole status with `Status::done(words)`, dropping `also`. Production
code, not touched. The walk waits for `merged clash into main ✓` plus the staged row and asserts the split from git;
when fixed, `resolve_and_finish` can also wait for `your edits are back`.

**Finding — textual (capture): on the step-7 resolver nothing says his edits are aside or that he is practising.**
`docs/shots/m8-tour-step7-conflict-120x36.txt`: the resolver fills the screen — no practice header, no card, no stop
line, so no ` · edits aside` rung. The caption says so; the old draft caption ("practice header still on top") was
not what is drawn. Whether the resolver should carry either is a design question.

**C+1 not done:** there is no M8 contact sheet or captions TSV (`find docs -iname '*m8*contact*'`: none). Ticket 28
builds it (`28-amendment.md` :19-22). Its TSV should take the three `m8/21c-*` captures and the two new tour captures.

**C+2 m4 half stopped — the re-shoot draws `future`.** `pty-walk-m4.py {80x24,160x48} journal-rows` under
`GRAFT_UPDATE_SHOTS=1` changed, in each file, the three Graft rows' age `now` → `future` and the fetch row
`remote-tracking refs · not undoable` → `origin/main moved · not undoable` (the 21c change, expected). The `future` is
new since 19d-b2: `crates/graft/examples/m4_walk.rs:197` pins the frame to `PINNED_NOW` (2026-10-08 00:00Z), while
the journal rows carry the instant the walk's operations ran (`JournalEntry::at_unix_ms`,
`graft-ops/src/journal/lines.rs:35`), which is wall time, later than the pin. Where that stamp is taken was not
traced (**unverified** which line). Reverted both captures; m4 sheet rebuilt byte-identical. Options for the
conductor: pin the job runner/journal clock in the example (`JobRunner::with_clock`, `jobs/runner.rs:205`, if the
journal stamp reads it — unverified), or leave the example's frame on the wall for this capture. `TESTING.md:1769`
updated to the fetch words the run drew, noting the capture still lags.

**C+2 m7:** `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7` → part 1 changed (173931 B), part 2
unchanged (54181 B). Beyond the 9 `remote-tracking refs` → `origin/main moved` lines, part 1 picked up captures that
other tickets re-shot since its last assembly (e.g. rows `Commit 200…` replaced by `◆ api ~/code/api · main  5
commits` rows): the script assembles the committed captures as they are.

**19c-c open boxes:** the walk proves box 1 (reworded per this ticket: 7 drawn + finish status), box 2, box 3 and
box 4's first half; box 5's two missing captures now exist. Ticked there naming this test.

**Touches (how checked).** `tests/it/tour_walk.rs` only in code (`git status`); `tests/it/main.rs` already had
`mod tour_walk`; graft `MAP.md` line 32 (the stale "not here: red at task 6" — one line, may meet 18e Part B's MAP
edits at merge); `docs/shots/m8-tour-step7-conflict-120x36.txt`, `m8-tour-finished-120x36.txt` new;
`docs/shots/m7/m7-contact-sheet-1.txt` rebuilt; `TESTING.md:1769`; the existing tour tests
(`the_tour_captures_match`, the C+2 push probe, `the_ticks_row_reads_as_bits`) green in the same run, their captures
unchanged. No production file (`git status`). No home or OS-temp path in either new capture beyond the
`/tmp/graft-capture/…` convention already in `m8-tour-step1-120x36.txt`.
