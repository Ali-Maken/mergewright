# Review — m7-power-tools 06c absorb-config-and-oracle

Reviewer, 2026-09-26. Merge `3ef0801c` (four commits, `e06a9ad3..32c6bc9f`), read as `git diff 3ef0801c^1 3ef0801c`.
Every check ran in a detached worktree at `3ef0801c` in the session scratchpad, with
`CARGO_TARGET_DIR=$TMPDIR/graft-target-m7-rev06c`. The worktree has since been removed. Probes were a throwaway test
module (`probe_rev06c.rs`, not committed) and shell checks. Mutants were applied one at a time and reverted before the
heavy tier ran. Every behaviour quoted below is what those runs printed (git 2.54.0). Findings already filed by reviews
06 and 06b are not repeated.

**Counts: Critical 0 · Major 3 · Minor 6 · Questions 2.**

## Gates (one at a time, worktree at `3ef0801c`)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass (164.6 s, cold) |
| `cargo clippy --workspace --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` (run 1) | **3933/3933 pass**, 1 skipped, 337.5 s (465.6 s wall, cold build). Load **61** at the start (other builders). Max RSS 1.05 GB |
| `cargo nextest run --workspace --status-level pass` (run 2) | 3933/3933 pass, 442.2 s, load 21–31 |
| `cargo test --doc --workspace` | pass (3 doctests) |
| Heavy tier (`cargo test --features heavy-tests -- --test-threads=2`) | graft lib 1349 pass. **graft `it`: 532 pass, 4 fail**, 607 s (722 s wall). Max RSS 1.05 GB. 3 are fixture builds timing out at 30 s (`git update-ref --stdin`, load): `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs`, `many_refs::first_paint_with_50k_real_refs_is_within_budget`, `many_refs::reading_50k_real_refs_is_within_budget`. 1 is `heavy.rs:217`, known (KNOWN_GAPS M7/heavy217). cargo stopped after that binary |
| Release perf (`frames:: heavy::budgets_hold`, alone) | **12 pass, 2 fail**, load 12 → 8, 95.8 s. No verdict under this load; numbers below |
| Fuzz, benchmarks | Not run. No fuzz target covers the code changed here |

- **Both suite runs are over 7.1's 3 minutes**, under loads of 21–61 from concurrent builders. Not this group's cost.
- **5 slowest (run 2):** `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 43.4 s,
  `conflict_ops::whole_file_shapes_resolve_like_git` 23.5 s, `m5_walk::every_scenario::merge_resolved_with_the_keys`
  18.3 s, `diff_fixture::parse_unified_matches_numstat_for_every_shape` 16.5 s,
  `history_ops::every_preview_refuses_exactly_what_check_refuses` 12.7 s.
  - This group's slowest is `absorb_unpushed::absorb_and_a_into_agree_on_the_range`, **10.9 s** (Minor 6). Every other
    absorb test takes under 2.7 s.
- **`target/`:** the main repo's is 17 GB. The review's own is 8.2 GB (debug + release).
- **Release perf numbers:**
  - First paint (budget 150 ms): 35.4 ms (commit-graph), **155.3 ms (no commit-graph, fail)**, 106.1 / 144.4 ms
    (realistic, with and without), 96.0 ms (50k refs).
  - Pixel, 100k: **fail**, "1 keys were drawn before their rows arrived" (key 168, PageDown, 7 of 38 rows on hand).
  - Pixel p99: 2.5 ms (50k refs). Unicode p99: 1.1 ms (100k); 1.0 ms closed / 1.7 ms diff / 1.6 ms detail / 0.9 ms map
    (50k refs).
  - graft-ops draws nothing, so neither failure is this group's. Both are load-sensitive and both passed on 06b's run
    at load 5–8. **Rerun on an idle machine before the M7 stop.**

## Ticks spot-checked: 6 of 6 held

- **M1:** both tests exist (`absorb_config.rs:16`, `:32`). The fix is `--no-ignore-revs-file` (`absorb/read.rs:179`).
  - **Mutant:** with the flag removed, both tests go red.
  - Red first can't be seen in history: the tests and the fix are one commit (`7d535055`).
  - The builder's deviation holds, checked in the shell: `git -c blame.ignoreRevsFile=nope blame --ignore-revs-file= f`
    → `fatal: could not open object name list: nope`, and `--no-ignore-revs-file` blames.
- **M4, M5:** the three tests exist and are green. `--whitespace=nowarn` is at `absorb/build.rs:102` and `absorb/judge.rs:71`.
- **M7:** `wc -l` matches the tick line for line. All files are ≤ 400.
- **M8:**
  - **Mutant "placed into `HEAD`"** (`absorb/mod.rs:215`): red in all 6 `absorb_oracle` rows, at `absorb_printed.rs:128`,
    which is the `git log -L` target check.
  - **Mutant "any below"** (`absorb/judge.rs:42`): red in `a_hunk_over_the_branch_and_older_lines_stays` and
    `an_insertion_below_a_branch_line_and_above_an_older_one_stays`.
  - Both as the tick says.
- **Minor 1 / ruling:** the 12 `absorb_unpushed` tests exist and are green. The five `a`-into names match the ruling.
- **Gates:** 3933/3933, 1 skipped, as the tick says.

## The focus questions

### Does the ruling's test set pin the rule? Partly. The pushed stop, the part that protects his team, is not pinned
- **Pinned:**
  - the cap, both sides (`absorb_on_main_reaches_ten_back`, `a_into_eleven_back_is_beyond_reach`);
  - the stop above a merge (`absorb_on_main_stops_above_a_merge`);
  - "fallback only when `base == HEAD`": the two upstream regressions go red under "always fall back";
  - one range for both keys (`absorb_and_a_into_agree_on_the_range` goes red if `a` keeps the old rule).
- **Not pinned:** every remote in the tests is `refs/remotes/origin/main` at a first-parent commit of `main`. Two
  plausible wrong rules pass **all 12 tests** (A also passes all 716 graft-ops tests, 231 lib + 485 `it`; B was run
  against `absorb_unpushed` only):
  - **Mutant A**, where pushed means "is a remote-tracking tip" rather than "reachable from one". It was run with
    `for-each-ref refs/remotes` in `not_on_a_remote`.
  - **Mutant B**, where only the same-named remote branch counts (`--remotes=*/main`). Under B, probe `p3` (below)
    rewrites the commit a teammate's branch holds. See Major 1.
- **The builder's 8/12 is honest but thin:**
  - `a_into_on_main_without_upstream_takes_an_unpushed_commit` and `a_into_below_a_merge_is_merge_in_range` pin
    nothing new. Both were green before 06c.
  - The merge case also passes if the merge itself were taken into the range. What kills that mutant is only the
    agreement test's `side` commit.

### "`a`-into on main was never `NotOnYourBranch`": **verified, the builder is right**
- **Before**, at `3ef0801c^1`, `rewrite/plan.rs:171`: `amend_into` asked only `ancestor(target, HEAD)` (`merge-base
  --is-ancestor`). It never called `base_of` (`:154-215`).
  - So on `main` with no upstream, `a` planned into **any** non-root ancestor. That included pushed commits and commits
    any depth back.
  - A merge in `target^..HEAD` refused `MergeInRange` (`:182-183`).
  - The ruling's premise ("today `NotOnYourBranch`", and "it uses `base_of`" at ticket :81) was wrong on both counts.
- **After:** `plan.rs:192` calls `refuse_out_of_reach`, which uses `your_commits` → `unpushed_range`.
  - On a branch with no base of its own, `a` now **loses** reach: `AlreadyPushed`, `BeyondReach`, `MergeInRange`,
    `RootCommit`.
  - The ruling framed this as widening. For `a` it is a narrowing; for absorb it is a widening (it was always
    `NothingToAbsorb`).
- **KNOWN_GAPS M7/a-into-main is false** as written (Minor 1).

### Absorb and `a` on one range: **yes on the fallback, no elsewhere**
- **The fallback:**
  - Both keys call `your_commits` (`rewrite/absorb.rs:36`, `rewrite/plan.rs:245`), and both read the one
    `unpushed_range`.
  - The agreement test holds over 4 fixtures.
- **With a base of its own** (`YourCommits::Above`, `unpushed.rs:67-68`):
  - Absorb reaches `base..HEAD`.
  - `a` reaches every ancestor, because `refuse_out_of_reach` returns `Ok` (`plan.rs:245-247`).
  - They differ on pushed commits and on commits below the base. See Major 2.

### Pushed commits are never rewritten: **true only on the fallback**
- **The fallback read is right:** `rev-list <newest> ^<below> --not --remotes` (`unpushed.rs:168-177`).
  - Probe `p3`: `main`, no upstream, `refs/remotes/origin/teammate` = a child of c4 that is off `main`.
  - Absorb places the c5 hunk, and the c3 hunk stays `OutsideYourCommits`.
  - `a` into c4 → `bc05611 is already pushed · a only changes commits you haven't pushed`.
  - Probe `p6`: `origin/main` ahead of local `main` (fetched, not pulled) → `AllPushed`. Right.
- **Outside the fallback it is not true.** See Major 2.

### The oracle's independence: **holds**
- The target comes from `git log -1 -L n,n:path main..feature` (`absorb_printed.rs:70-78`). The hunks come from
  porcelain `git diff --cached` (`:24-57`).
- It shares no code with `absorb/read.rs`. The mutant above shows it catches what the old checks let through.
- **Limits:**
  - The range is hard-coded to `main..feature` (`:72`), so the new fallback range is never under the oracle (Minor 5).
  - A hunk the oracle names a target for, and the map keeps, is accepted as `TwoCommitsChangedIt` without an
    independent commutation check (`:130-134`).

### Words of the new refusals
- **`AllPushed`** (`operation.rs:983-986`): `every commit on main is already pushed · absorb only changes unpushed
  commits · c commits them`. Exactly the ruling's words, and true wherever it fires.
- **`AlreadyPushed`** (`:987-991`) and **`BeyondReach`** (`:992-997`): the ruling's words. But each states a rule for `a`
  that holds only on a branch with no base of its own:
  - `a only changes commits you haven't pushed`;
  - `a reaches your last 10 unpushed commits`.

  On his tracked feature branch, `a` changes pushed commits at any depth. That is untrue in general. See Major 2.
- **Absorb on a single-commit repo** gives `RootCommit { HEAD }`: `<short> is the first commit · there is nothing below
  it to replay onto`. True. It does not name `c` as `AllPushed` does. Fine.

## Critical

None.

## Major

### M1. The team rule, "not reachable from any remote-tracking ref", has no test that tells it from two wrong rules
- **Where:**
  - The tests: `tests/it/absorb_unpushed.rs:38-49` (`pushed_to`: the only remote fixture, `refs/remotes/origin/main` at a
    first-parent commit).
  - The rule: `rewrite/unpushed.rs:160-182`.
- **What:** mutants A (tip equality) and B (only `*/<his branch>`) pass all 12 `absorb_unpushed` tests. A also passes
  every graft-ops test; B was not run wider.
  - Under B, probe `p3` places a hunk into c4 and `a` into c4 plans. A teammate's `origin/teammate` holds c4.
- **Why it matters:**
  - This is the one rule in the ruling that exists for his team (memory `works-in-a-team-on-dependent-branches`).
  - Today's code is right, but a later change to the read (moving it into graft-git as `contained_in`, as the NOTES
    offer, or an optimisation) could break it with every test green.
- **Fix:** add `a_teammates_remote_branch_marks_its_commits_pushed`. `main` c1..c6, no upstream, and
  `refs/remotes/origin/teammate` at a new commit whose parent is c4 (probe `p3`'s fixture).
  - Absorb: a c5 hunk is placed, a c3 hunk stays `OutsideYourCommits`.
  - `a` into c4 → `AlreadyPushed`.
  - It kills both mutants.

### M2. On a branch with an upstream, or pushed without one, `a` still rewrites pushed commits, and the new words say it can't
- **Per the ruling's scope** (ticket :50, :115: "Any other base … unchanged"). **This is a spec gap for the planner,
  not a builder defect.** It is filed Major because it breaks the ruling's own two principles in his everyday
  configuration.
- **Where:** `rewrite/unpushed.rs:67-68` (`Above(base)` whenever `base_of` is below `HEAD`) and `rewrite/plan.rs:245-247`
  (`Above` → no check).
- **What:**
  1. **Tracked feature branch.** `a_into_with_an_upstream_is_unchanged` (`absorb_unpushed.rs:309-318`) pins that `a`
     into f1, which `origin/feature` holds, **plans**.
     - On the same fixture, absorb keeps a hunk f1 made as `BeforeYourBranch` (`absorb_with_an_upstream_keeps_its_range`,
       `:216-219`).
     - Absorb says f1 is not on his branch; `a` rewrites it. That is the ruling's own "one of them is lying" (ticket :96).
  2. **Pushed without `-u`.** Probe `p7`: `feature` at f3, `refs/remotes/origin/feature` = f3, no `branch.feature.*`.
     - Absorb places a hunk into pushed f2, and `a` into f2 plans.
     - Both run from the default-branch base.
  3. **The words.** `AlreadyPushed` says `a only changes commits you haven't pushed`, and `BeyondReach` says `a reaches
     your last 10 unpushed commits` (`operation.rs:987-997`). On cases 1 and 2, `a` does neither.
- **Why it matters:**
  - "Rewriting a commit a remote ref holds is the harm absorb's rule exists to prevent. Pointing at the commit doesn't
    change who else holds it" (ticket :101-102).
  - The everyday team case, a feature branch tracking `origin/feature`, is exactly where that harm is.
- **Suggested (planner):**
  - Apply the `--not --remotes` stop on **every** base: a commit of `base..HEAD` that a remote ref reaches is refused by
    `a` as `AlreadyPushed` and stays for absorb.
  - Or, at the least, scope the words (`… on a branch with no upstream`).
  - Either is a spec change through the conductor.

### M3. `apply.whitespace` still changes what stays staged, in the put-back: `fix` unstages a stay, `error` fails the run
- **Where:** `apply/set_aside.rs:508-514`, the 06b put-back `git stash apply --index <rebuilt>`.
  - git's stash re-applies the index by running `git apply --cached` as a child process, which reads his
    `apply.whitespace`.
  - Nothing pins it.
- **Probe `p5`:** the `branch()` fixture. A b.txt hunk is placed. An `old.txt` hunk staged as `o2   ` (trailing spaces)
  stays `BeforeYourBranch`.
  - **`fix`:** the run succeeds. The index's `old.txt` is `o1|o2|o3|o4|o5`, which is `HEAD`'s. The spaces are stripped.
    The working tree keeps `o2   `.
    - His staged change is silently no longer staged. 06b's promise "stays stay staged" breaks, with no words.
  - **`error`:** the rewrite runs, then the put-back fails:
    `put staged changes into 91c4093: <stdin>:8: trailing whitespace. · your edits are in stash@{0} · u restores the backup`.
    - Recoverable by `u`, so nothing is lost. But absorb fails on a stay he did not ask it to touch.
- **Why it matters:**
  - This is KNOWN_GAPS **M7/06c-apply** ("unverified"), now **verified broken** at this site.
  - It is the same class as review 06 M5, which 06c fixed only for absorb's own `apply` calls.
  - The user's config sets neither (review 06 checked), so it does not bite him today.
- **Fix:**
  - Pin `apply.whitespace=nowarn` for that `stash apply` through `GIT_CONFIG_COUNT/KEY/VALUE` (the mechanism `CLEANUP`
    already uses).
  - Tests: `a_stay_with_trailing_spaces_stays_staged_under_apply_whitespace_fix` / `_error`.
  - Then probe the other sites the gap row names (`operation/stage.rs`, `preview/stash.rs`).
  - graft-ops owns `set_aside.rs`, so this is one slice.

## Minor

1. **KNOWN_GAPS M7/a-into-main is false.** `a` never refused on `main` with no upstream (`3ef0801c^1`
   `rewrite/plan.rs:171`). Since 06c it refuses by the fallback's stop, as the ruling asked. Close the row, and correct
   the ruling's premise in the ticket.
2. **A second-parent target is `MergeInRange`, not the ruling's `NotOnYourBranch`, and the NOTES don't say so.**
   - The ruling (ticket :116) says a target that "isn't a first-parent ancestor of HEAD → `NotOnYourBranch` as today".
   - `plan.rs:186` still asks `--is-ancestor`, so `side`'s s1 reaches `refuse_out_of_reach` → `MergeInRange`.
   - `a_into_below_a_merge_is_merge_in_range` pins this (`absorb_unpushed.rs:302-305`).
   - The ruling contradicts itself ("first-parent" vs "as today", which is `MergeInRange`). The builder chose "as today"
     silently. Say it in NOTES; the planner confirms.
3. **`AlreadyPushed.by` is dead generality with a false branch.**
   - Only `RewriteKey::AmendInto` constructs it (`plan.rs:251-254`).
   - `by = Open` would render `R only changes commits you haven't pushed`, which is untrue: `R` has no such rule.
   - `RewriteKey::key()` (`operation.rs:363-370`) exists only for this.
   - Drop the field, or keep it and say in the doc why.
4. **The graft-git knock-on is now confirmed at the flag level.**
   - `graft-git/src/blame.rs:245-246` passes `--ignore-revs-file=` with the comment "An empty name clears every list,
     the configured one included". git 2.54 still dies on a configured file that is missing (shell probe above).
   - `:242-244` (`(None, true)`) also leaves a missing configured file to fail.
   - G-blame itself was not run.
   - No KNOWN_GAPS row names it. File it for 18's owner.
5. **The oracle never runs on the fallback range.**
   - `last_to_change_line` hard-codes `main..feature` (`absorb_printed.rs:72`). Every `absorb_oracle` row uses
     `branch()`.
   - The no-upstream placement rests on the 12 hand-picked expectations.
   - Take the range as a parameter, and add one oracle row on `pushed_to(6, 3)`.
6. **`absorb_and_a_into_agree_on_the_range` takes 10.9 s** (run 2, load ~25).
   - It runs `plan` once per commit on each fixture: 4 + 6 + 12 + 7 plans, each a fresh facts read and a
     `commit-tree`.
   - It is fine for 7.1, but it is 4× the next absorb test. Four `#[test]`s, one per fixture, would at least run in
     parallel.

## Questions

1. **A new branch at the default branch's tip falls into the fallback.** `your_commits` falls back on `base == HEAD`
   alone (`unpushed.rs:68`). So `git switch -c fix` on an unpushed `main`, stage, absorb → places into **main's**
   commits.
   - Probe `p4`: plan `pick c3 / fixup! c3 / pick c4`. After the run, `main` is still `cdf925b` and `fix` is a new
     `989e471`.
   - Before 06c this was `NothingToAbsorb`. Now `fix` carries rewritten copies of main's commits, and a later merge of
     `fix` into `main` meets both versions.
   - With a remote (main pushed), it is `AllPushed`, which is fine.
   - The ruling says "his branch *is* the default, or the only branch". Should the fallback also require `HEAD`'s
     branch to be the default or the only branch, and otherwise give `NothingToAbsorb`?
2. **The builder's behaviour change** (NOTES: a branch whose upstream equals `HEAD` now gets `AllPushed` and `a` →
   `AlreadyPushed`) is the ruling applied as written, and I agree it is safe.
   - But combined with M2 it makes `a` depend on whether he has one unpushed commit. One unpushed commit on the
     feature branch → `Above` → `a` into any pushed commit plans. None → every pushed commit is refused.
   - Confirm that is intended, or fold it into M2's fix.

## Pipeline, architecture, safety

- **Pipeline:** this is a fix group from review 06 with two planner rulings. No new journey. The rulings cite the
  research (`docs/research/interactive-rebase.md:118`, `:135`; both lines checked).
- **Crate rules:** graft-ops only. No new dependency. The remote read goes through `GitInvocation`, as graft-ops'
  other `rev-list` reads do.
- **The app side:** `Refusal` is matched non-exhaustively in the app (`crates/graft/src/update/operate.rs:1116`,
  `other => … explain()`), so the three new variants need no app change.
- **Readability:** `your_commits` / `unpushed_range` / `RangeStop` read plainly. Every new file is under 400 lines.
  `absorb_base`'s `Cap`-with-nothing-taken arm is an honest error, not a swallow (`rewrite/absorb.rs:56-60`).
- **Rule 3:** no new `let _`, `unwrap` or `expect` outside tests. `unwrap_or_default` at `rewrite/absorb.rs:117` now
  carries review 06 Minor 7's comment.
- **Git on the UI thread:** absorb has no caller in `crates/graft` yet (09). Every `a` plan now also runs `base_of`
  and two `rev-list`s (`plan.rs:192`, via `your_commits`). This is unmeasured; the keys ticket runs `plan` off the UI
  thread.
- **Touches:** MAP.md rows match the code (checked: `unpushed`, `absorb_plan_map`, the refusal words, the `absorb/`
  split). `R` (`Open`) is unchanged; it does not call `your_commits` (grep).
