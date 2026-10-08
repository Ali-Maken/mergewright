# Merge over unsaved edits sets them aside; the tour's ages stop following the calendar; the whole-tour walk

Status: draft (planner spec-only pass, 2026-10-08; rules on 19c-c NOTES :221-232 and :261-263). Amended 2026-10-08,
planner follow-up on NOTES 19d-b: §R4, §R5 and part 19d-b2 added, and 19d-c gains C+3. Amended again 2026-10-08,
planner follow-up on NOTES 19d-a (branch `worktree-agent-a9afa698c484ca6f4`, commit bebdddd5, not merged): §R6 and
part 19d-a2 added.
Build with: five parts. a/a2 and b/b2 have disjoint file sets and may run concurrently (two helpers max). a2
continues a's branch. b2 comes after b. c comes last and needs all four.
- **19d-a — Opus, xhigh.** Stash- and undo-adjacent: a wrong order or a missed resume path loses edits, and no diff shows
  it. Not Fable: no new structure. It is the third extension of a proven mechanism (11a cherry-pick, 03b ff-pull, both
  on the same `sets_aside` arm and the same table tests), and the round-trip tests below can show whether it is right.
- **19d-a2 — Opus, xhigh.** D1 and D2 are in `apply/close.rs`, the close every set-aside kind shares. That path
  decides whether `u` and the stop words are right after a stop. Not Fable: both defects carry their lines and red
  tests, the fix is local to the close, and the tests can show whether it is right.
- **19d-b — Sonnet.** A pure parse, one read at launch, one default in the pty harness. Every signature is fixed below.
- **19d-b2 — Sonnet.** Mechanical: one constant moves, one test seam is fixed, and 19 producers and one pty default are
  pinned. No production API. The rulings are §R4 and §R5.
- **19d-c — Opus.** The 19c-c walk, finished; pty and captures. **Last.**
Needs: 19c-a, 19c-b (merged), 19c-c partial (merged, 706d988f). 19d-a2 needs 19d-a (bebdddd5; the two merge
together). 19d-b2 needs 19d-b (merged, partial). 19d-c needs 19d-a, 19d-a2, 19d-b and 19d-b2.
Scout: 19d-a uses `.scratch/m8-polish-and-ship/scouts/merge-preview-dirty-index.md` (no second scout). 19d-a2: NOTES
19d-a is its scout. 19d-b: none. MAP :91, :182 and :1116 are enough; the conductor decides. 19d-b2: the sweep in
NOTES 19d-b is its scout.
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
  right looks like.
- **Clash notice (broken, `crates/graft/src/update/clash.rs` `undone_with_edits`, `_ => "rebase"`):** fix it in the same
  part, since it is the line this milestone touched. An exhaustive match with no `_` arm, worded from the entry's own
  kind name (the journal's `kind_name`, graft-ops MAP :241, `apply/record.rs`). No new vocabulary. A kind name that
  reads oddly in `u undoes the <kind> too` is listed in NOTES, not reworded.
- **Spawns: accept the measured 5 (true merge) / 3 (fast-forward).** R1's "≤ 4" is amended to those numbers. The
  baseless `merge-tree` proposal is rejected. It would change the fixed `merge_landed` signature and reach into
  graft-git (`forecast_merge` returns no tree) to save one spawn, on a preview job that checks `stop` between spawns.
- **Up-to-date merge over edits (question, NOTES 19d-a :303-304): verify first.** Write a test. If the nothing-to-do
  box draws a set-aside row, the cause is the plan. Fix it in graft-ops `preview::plan`: a merge with no ref move
  gets `set_aside: None` and `put_back: None`. If that box offers `y` (so `apply` would set aside for nothing),
  **stop and report**. That is a different rule.

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
- [ ] The tour's case, red before: `main` + a staged change to a file `feature` doesn't touch; `apply(Merge)` commits
      the merge, `notes.txt` is still `M ` staged, and the stash list equals the list before:
      `a_merge_over_a_staged_change_git_would_refuse_goes_through`.
- [ ] Unstaged edits to a path the merge also changes, put back clean: `a_merge_puts_unstaged_edits_back_after`.
- [ ] Clean tree, or only untracked files: `sets_aside` is `None`, no `stash` line in the entry's commands, `touches`
      without `refs/stash`: `a_merge_over_a_clean_tree_sets_nothing_aside`, `untracked_files_alone_set_nothing_aside`.
- [ ] Stops on a conflict: entry `Pending` with `put_back`, `S` listed. `resume(ContinueWith)` after the resolution
      → `Committed`, edits back with the split kept, `S` dropped: `a_stopped_merge_finished_puts_the_edits_back`. Same
      with `Continue`: `…_continued_puts_the_edits_back`. `Abort` → old tip, edits back, `S` dropped:
      `a_stopped_merge_aborted_puts_the_edits_back`.
- [ ] Put-back clash (merge clean, edit overlaps): entry `Committed`, `ApplyError::Clashed { why: SetAside }`, `S`
      kept: `a_merge_put_back_that_clashes_stops_as_a_clash`.
- [ ] Undo/redo round trip on ADR-0045 §7's tree table (staged + unstaged + untracked, staged new file, deletion,
      rename). After `u`: index tree, worktree tree and stash list equal to before. After `U`: equal to after:
      `undo_of_a_set_aside_merge_round_trips`, `redo_of_a_set_aside_merge_round_trips`.
- [ ] A stop injected before each of steps 1-6 leaves the edits in the tree or in the stash list, never in neither
      (read both): `a_stop_anywhere_in_a_set_aside_merge_keeps_the_edits`.
- [ ] Preview: `set_aside` `Some`, and `put_back` is `Clean` (ff and true merge), `Likely` (overlap), or
      `NotChecked("the merge may stop first")` (the merge forecast `Likely`). Clean tree → both `None`, and the
      preview's spawn count is unchanged from before: `the_merge_preview_forecasts_the_put_back` (graft-ops).
- [ ] App: a merge preview over a staged change draws the set-aside row and the put-back row, in the words rebase
      uses; `y` lands with `your edits are back`: graft `merge_over_staged_edits_previews_and_puts_back` (harness,
      `Clock::Fixed`).
- [ ] The `clashed` notice (graft MAP :646, `undoes the rebase/switch too`) names a merge when the entry is a merge.
      If it is a fixed string, it reads the kind. Test: `a_merge_put_back_clash_notice_names_the_merge`.
- [ ] Pull probe: an ff pull over a staged change it doesn't touch goes through, staged kept:
      `pull_set_aside::an_ff_pull_over_an_untouched_staged_change_goes_through`. **Red → stop, report** (R1).
- [ ] 19c-b per-task tests green (`tests/it/tour_tasks.rs`); every changed expectation is listed in NOTES with why.
- [ ] ADR-0045 amendment appended, naming the tests above; MAP rows and keymap row `m` updated.
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
- [ ] D1: `stop_hooks::a_merge_stopped_in_its_post_merge_hook_has_landed` green with the edit set aside:
      `stopped_at == Some(StoppedAt::Landed)`, entry `Committed`, edit back, `S` dropped. Pick twin:
      `cherry_pick::a_pick_over_edits_stopped_after_it_committed_has_landed`. Both red before.
- [ ] D2: `stop_hooks::a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back` green: the reason is
      `stopped::BEFORE_PAUSE`'s words, edit back, stash empty, `u` restores. Pick twin:
      `cherry_pick::a_pick_over_edits_stopped_before_git_paused_it_says_so`. Both red before.
- [ ] The 7 paused tests (NOTES 19d-a "Paused") assert R6's claim, names unchanged. While paused: `S` listed by
      commit, the tree at `HEAD`'s copy. After resume: back, split kept, `S` dropped.
      `a_paused_merge_aborted_by_resume_gives_his_edits_back` and
      `a_paused_merge_continues_to_the_merge_git_would_have_made` stay green.
- [ ] Clean-tree twins: `stop_hooks::a_clean_merge_stopped_in_its_prepare_commit_msg_hook_is_paused`,
      `stop_hooks::a_clean_merge_stopped_in_its_pre_merge_commit_hook_is_paused`. They carry 13c's old "untouched"
      claim with nothing set aside.
- [ ] Relaunch after a stopped merge with edits aside offers `c`/`a`. Its stop line is drawn as the screen holds it,
      and NOTES names the rung: `quit_waits::stuck::relaunch_after_a_stopped_merge_offers_continue_and_abort`.
- [ ] Captures re-shot, diff read, only set-aside words changed, captions updated:
      `capture_relaunch_after_a_stopped_merge` and the hangup capture's test.
- [ ] Clash notice: a table over every kind that sets aside (rebase, rewrite, cherry-pick, checkout, switching
      branch-create, pull, recover, merge), each worded from its kind name, no `_` arm. Red before for pick and pull:
      graft `clash::tests::the_clash_notice_names_each_set_aside_kind`.
- [ ] Up-to-date merge over edits: `merge_set_aside::an_up_to_date_merge_over_edits_sets_nothing_aside`. Green as
      found → say so. Red → fixed per R6. `y` offered → stopped and reported.
- [ ] Spawn box re-ticked against 5 / 3: `the_merge_preview_forecasts_the_put_back`. The ADR-0045 19d amendment says
      5 / 3 and names D1/D2's tests. MAP :263's "Not reached: … a set-aside's close" is removed.
- [ ] All four gates workspace-wide, including every test NOTES 19d-a lists red.
**Touches (check each, say how):** every set-aside kind's close (run `set_aside.rs`, `cherry_pick.rs`,
`pull_set_aside.rs`, `absorb_staged.rs`, `merge_set_aside.rs`: green, no expectation changed except those named);
`ApplyError::stopped_at` callers in graft (the stop words the app shows after a stop); recovery (a paused set-aside
entry is `live`, left alone by `recover_interrupted`: one assertion in the relaunch test); undo of a `Landed`
set-aside entry (D1 test's `u`); the stop line rungs; the two captures and any contact sheet holding them (the
conductor rebuilds); KNOWN_GAPS (the conductor closes the clash-notice row and files nothing for D1/D2 once green).

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
**Schedule:** after 19d-b. It shares the `crates/graft` tests with 19d-a and 19d-a2. Before running them concurrently,
the conductor checks the file sets are disjoint. 19d-a's app test goes in a new file. `tour_tasks.rs` belongs to
19d-a, and `quit_waits/*` to 19d-a2.
**Read first:** this file §R3-§R5 and NOTES 19d-b; graft MAP :91, :182, :1116; graft-fixtures MAP :32 and the
`capture` lines; ADR-0037 (contact sheets).
**Acceptance:**
- [ ] `graft_fixtures::PINNED_NOW` = `1_791_417_600`, and `tests/it/pty.rs` uses it (no second literal; grep shows one):
      `clock_pin::graft_now_pins_the_drawn_age` still green.
- [ ] `App::open_with_clock` pins the frame. In-process, an `App` opened with `Clock::Fixed(t)` and with
      `Clock::Fixed(t + 1y)` draws two different ages on one commit row:
      `clock_pin::open_with_clock_pins_the_drawn_frame` (red before).
- [ ] All 19 producers in NOTES 19d-b :269-273 build on `PINNED_NOW`, and the guard is green:
      `clock_pin::no_capture_producer_runs_on_the_wall_clock`. It bites: adding `Clock::Wall` to one producer fails it.
      Run that once, revert, and say so in NOTES.
- [ ] An in-app commit in a pty run with no date env draws its age, not `future`:
      `clock_pin::an_in_app_commit_under_the_default_pin_draws_an_age` (row reads `1h`; red before).
      `u_under_a_pinned_clock_undoes_the_newest` still green.
- [ ] Full suite green. `git status --short docs/shots` is empty except `FROZEN.md`, or every changed capture is listed
      in NOTES with only age cells changed. `grep -l future docs/shots` → none, or each hit named in NOTES with why.
- [ ] `docs/shots/FROZEN.md`: every year-age capture with no producer, one line each (§R4), with the method that built
      the list named in NOTES. Each sheet that is a contact sheet is **not** listed.
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
- [ ] Whole tour by the card's keys from his repo, 7 drawn + finish status, cursor home:
      `the_whole_tour_in_a_pty_ticks_all_eight_and_comes_home`.
- [ ] `his_repo_is_untouched`, `state_dir_is_unchanged`, `practice_folder_is_gone`: same test.
- [ ] `notes.txt` staged after tasks 6 and 7: same test, assertion `the_split_survives_both_merges`.
- [ ] Captures `docs/shots/m8-tour-step7-conflict-120x36.txt`, `docs/shots/m8-tour-finished-120x36.txt`, each
      captioned with what right looks like, colour **unverified**, ≤ 200 KB: same test's `shot` calls.
**Touches:** `tests/it/main.rs` (one `mod` if not present), ticket 19 §A9 boxes (the conductor ticks them naming these
tests), KNOWN_GAPS unjudged rows for the two captures (the conductor files them).
**Not ruled here, for the conductor to file from 19c-c NOTES :254-260:** no ticks row at 120×36; `work·` with no gap
at 80×24; `~/code` under the header; card line 1 "top row" while the cursor is on `main`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`:
workspace-wide, every part.

## Size
a ~60k (done but blocked) · a2 ~45k · b ~25k · b2 ~45k · c ~35k. Each fits one context. If one compacts, say so in
NOTES.

## 19d-c additions (conductor 2026-10-08, from 21c Part B hand-back — stale outputs, no new decision)
- [ ] C+1: the M8 contact sheet gains `docs/shots/m8/21c-status-partial-fetch-120x40.txt`, `21c-journal-partial-fetch-120x40.txt`, `21c-journal-partial-fetch-80x24.txt` (ticket order, each under its "what right looks like" line).
- [ ] C+2: rebuild the m7 contact sheet (9 m7 captures were regenerated by 21c B) and the m4 pty capture + m4 sheet 21c B left stale; `TESTING.md:1769` updated to match. Name the test or script that rebuilt each at tick.

## 19d-c addition (planner 2026-10-08, §R5)
- [ ] C+3: the walk keeps its own `GIT_AUTHOR_DATE` / `GIT_COMMITTER_DATE` (`EPOCH + 3600`, draft :87-94). They
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
