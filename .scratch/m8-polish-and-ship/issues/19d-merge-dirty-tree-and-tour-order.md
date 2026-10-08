# Merge over unsaved edits sets them aside; the tour's ages stop following the calendar; the whole-tour walk

Status: draft (planner spec-only pass, 2026-10-08; rules on 19c-c NOTES :221-232 and :261-263)
Build with: three parts. a and b have disjoint file sets and may run concurrently (two helpers max); c needs both.
- **19d-a — Opus, xhigh.** Stash- and undo-adjacent: a wrong order or a missed resume path loses edits, and no diff shows
  it. Not Fable: no new structure. It is the third extension of a proven mechanism (11a cherry-pick, 03b ff-pull, both
  on the same `sets_aside` arm and the same table tests), and the round-trip tests below can show whether it is right.
- **19d-b — Sonnet.** A pure parse, one read at launch, one default in the pty harness. Every signature is fixed below.
- **19d-c — Opus.** The 19c-c walk, finished; pty and captures.
Needs: 19c-a, 19c-b (merged), 19c-c partial (merged, 706d988f). 19d-c needs 19d-a and 19d-b.
Scout: 19d-a uses `.scratch/m8-polish-and-ship/scouts/merge-preview-dirty-index.md` (no second scout). 19d-b: none.
MAP :91, :182 and :1116 are enough; the conductor decides.
ADR: **none new.** Nothing here is hard to reverse: the journal field (`put_back`) already exists and is additive
(ADR-0045 §1), and `sets_aside` is one match arm. 19d-a appends an **ADR-0045 amendment** (text in §R1, last bullet),
as 11a and 03b did.

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
  spawns, only when `set_aside` is `Some`. A clean tree spawns none.
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

// 19d-b, graft (beside `Clock`, session; MAP :91)
pub(crate) const NOW_VAR: &str = "GRAFT_NOW";
/// Pure: `None` → Wall; whole seconds → Fixed; anything else → Err(words for stderr).
pub(crate) fn clock_from(var: Option<&std::ffi::OsStr>) -> Result<Clock, String>;
// tests/it/pty.rs
pub const PINNED_NOW: i64; // chosen so no committed capture changes; named in NOTES
// Pty::spawn* sets GRAFT_NOW = PINNED_NOW unless the caller's env names GRAFT_NOW.
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

## 19d-b — the session clock pinned for pty runs (Sonnet, ~25k)
**Crate set:** `crates/graft` (`session` beside `Clock`; the launch path that builds the session clock and
`JobRunner::with_clock`; `tests/it/pty.rs`), graft MAP :91/:1116.
**Verify first:** grep `Clock::Wall` in `crates/graft/src`. If the binary already reads a clock pin, stop and report
(R3's premise is false).
**Acceptance:**
- [ ] `clock_from`: unset → `Wall`; `"1609459200"` → `Fixed(1609459200)`; `"x"`, `""`, `"1.5"` → `Err` with the words
      in R3: `session::tests::clock_from_reads_whole_seconds_only`.
- [ ] The binary obeys it: pty, the same repo with `GRAFT_NOW` = two instants a year apart, draws two different ages
      on one commit row: `tests/it/clock_pin.rs::graft_now_pins_the_drawn_age`. A bad value → exit 2 with the
      words, no screen: `…::a_bad_graft_now_refuses_to_launch`.
- [ ] Journal stamps follow it, and `u` still takes the newest entry with every entry in one second: two commits then
      `u` under the pin undoes the second: `…::u_under_a_pinned_clock_undoes_the_newest`. Red → stop, report.
- [ ] `Pty::spawn*` defaults the pin. The whole suite is green with **no capture changed**
      (`git status docs/shots` clean); `PINNED_NOW` named in NOTES.
- [ ] Sweep: every `docs/shots/` capture that draws a year age (`\b\d+y\b`) maps to a pinned producer (pty default, or
      a `Harness` with `Clock::Fixed`). The list goes in NOTES. More than 10 unpinned harness producers → stop and
      report the count.
**Touches:** every pty test (default env), the 19c-c committed captures (`m8-tour-step1-120x36.txt`,
`m8-tour-80x24.txt`: unchanged), G-rename 27's `GRAFT_*` list, KNOWN_GAPS `M8/19c-capture-age-roll` (the conductor
closes it, naming `graft_now_pins_the_drawn_age`).

## 19d-c — the whole-tour walk (Opus, ~35k)
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
a ~60k · b ~25k · c ~35k. Each fits one context. If one compacts, say so in NOTES.

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
