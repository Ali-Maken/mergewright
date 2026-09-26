# 06c absorb-config-and-oracle — review 06's remaining Majors

Status: done
Blocked by: none (06b merged). Both planner rulings are below.
Crates: graft-ops only (`absorb.rs` and its split, tests). Another crate needed → commit at the seam, report `blocked:
needs <crate>` with the signature.
Build with: Opus (config-sensitive git reads; an oracle that must not share the code under test).
Scout: none. The review cites every site; re-read each before editing.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-06-absorb.md`. No new decision.

## Scope
Each fix is the one the review's section states, at the cause, red first.
- **M1** (§M1): `blame.ignoreRevsFile` makes absorb fail (missing file) or misplace (present). Absorb's blame must not
  read his ignore-revs config.
- **M4** (§M4): a staged submodule bump fails the whole absorb.
- **M5** (§M5): `apply.whitespace` changes what is absorbed or fails the map.
- **M7** (§M7): split `absorb.rs` (571 lines) and its test file (659) under 400 each. No behaviour change.
- **M8** (§M8): an oracle that computes targets independently of the map under test, and tests for the three
  placement rules without one.
- **Minor 1 / Question 2** (what "his branch" is on `main` with no upstream): build only the planner's ruling, appended
  below when it lands. If it has not landed when you reach it, leave that box open and say so.

## Acceptance
- [x] M1: both cases (file missing, file present) as tests, red first. `absorb_config::absorb_ignores_a_configured_ignore_revs_file_that_is_missing` (red: `fatal: could not open object name list`), `absorb_config::absorb_ignores_a_configured_ignore_revs_file_that_is_present` (red: stayed, `left: []`). Fix: `--no-ignore-revs-file` (see NOTES: the review's `--ignore-revs-file=` does not clear a missing file).
- [x] M4, M5: each as a test, red first. M4 `absorb_config::a_staged_submodule_bump_stays_and_the_rest_is_placed` (red: `cat-file <head>:sub: bad file`; now `StaysWhy::Submodule`). M5 `absorb_config::apply_whitespace_fix_does_not_change_what_is_absorbed`, `absorb_config::apply_whitespace_error_does_not_fail_the_map` (both red with `--whitespace=nowarn` removed from `build.rs`/`judge.rs`).
- [x] M7: `wc -l` of every file after the split, all ≤ 400. `src/absorb/{mod 235, read 266, judge 131, build 203}`; `src/rewrite/{plan 272, absorb 205, unpushed 182}`; `tests/it/{absorb 343, absorb_oracle 313, absorb_printed 138, absorb_config 115, absorb_staged 376, absorb_unpushed 360}`. Behaviour unchanged: the 31 absorb tests green right after the split commit (e06a9ad3).
- [x] M8: the independent oracle named; each of the three rules' tests named, each seen red under a mutation. Oracle: `absorb_printed::each_target_last_changed_its_lines` (targets by `git log -L n,n:path main..feature`, never the code's blame), run inside every `absorb_oracle::absorb_agrees_with_the_trees_*` (new: `_three_targets_one_file_twice`); mutant "placed into HEAD" red in all 6 (it survives the old checks). Rules: all-below→any-below red in `absorb::a_hunk_over_the_branch_and_older_lines_stays` (+ the insertion test); insertion-by-line-above-only red in `absorb::an_insertion_below_a_branch_line_and_above_an_older_one_stays`; every-apply-error-as-no-commute red in `absorb::judge::tests::an_apply_failure_other_than_does_not_apply_is_an_error`.
- [x] Minor 1: per the planner's ruling (below), test named. `absorb_unpushed::*` (12 tests, names per ruling; see NOTES for which were red).
- [x] Gates workspace-wide. fmt, clippy `--workspace --all-targets -D warnings`, `nextest run --workspace` 3933/3933 (1 skipped), `test --doc --workspace` ok.

## Touches
MAP.md (graft-ops) for moved code; every caller of `absorb` (grep: graft's `A` key path). Name how each was checked.

## Ruling (planner, Question 2) — 2026-09-26

**Decision: (b) bounded by (a).** When absorb's base comes out as HEAD itself (his branch *is* the default, or the only
branch, with no upstream), "his branch" = **his unpushed commits, at most the last 10**. Not (c).

- **Why not (c):** today every absorb refuses on that branch (review Minor 1, `plan.rs:227-232`), including the
  review's probe (only branch `work`). Journey 2b (note :77, 4 steps) would simply not exist there. A refusal that
  explains itself is still a dead end.
- **Why bounded:** git-absorb's rule is the last 10 commits (`docs/research/interactive-rebase.md` :135). jj's absorb
  only moves changes into *mutable* ancestors (same study :118), and jj treats pushed commits as immutable. He works in
  a team (memory `works-in-a-team-on-dependent-branches`): absorb must never rewrite a commit a teammate may hold. The
  preview (`y`) and `u` stay as they are; this only widens what can be placed.
- **Pure (a) rejected:** "since the last push" has no answer with no upstream; the reflog point is guesswork he cannot
  see. Any remote-tracking ref is a fact he can see.

### What 06c builds (graft-ops only)
- Applies **only when** the existing base resolution gives `base == HEAD`. Any other base (upstream, a real default
  branch) is unchanged. Do **not** change `base_of` or `default_branch_of`: `a`-into, `R` and others call them.
- The fallback walks first-parent from HEAD and takes commits while **all** hold:
  1. fewer than 10 taken (`const ABSORB_FALLBACK_DEPTH: usize = 10;`, named, doc-commented with the git-absorb cite);
  2. the commit is not reachable from any remote-tracking ref (`refs/remotes/*`);
  3. the commit is not a merge (ADR-0047 §5 refuses merges in the range);
  4. the commit is not the root (whether the engine takes `--root` is unverified; the root stays out, and the
     now-dead "first commit" fallback at `plan.rs:227-232` is removed or replaced by this, not left beside it).
  The base is the parent of the last commit taken.
- Remote-tracking refs: use a read graft-git already exposes (check `crates/graft-git/MAP.md` first). None there →
  `blocked: needs graft-git` naming `fn remote_tracking_tips(repo: &Repo) -> Result<Vec<Oid>, FactsError>`; stop.
- A hunk whose blame lands on a commit **below** the range stays, with its own reason. If `StaysWhy` has no reason
  that says so truthfully, add `StaysWhy::OutsideYourCommits` (Minor 3's point: do not reuse `NoCommitChangedIt`).
- Range empty (every commit is pushed, or HEAD is a merge/root): refuse with its own reason, not `NothingToAbsorb`.
  Words (the keys ticket renders them; put them in the reason's doc comment):
  `every commit on <branch> is already pushed · absorb only changes unpushed commits · c commits them`

### Tests (red first, each named at tick time)
1. The review's probe: only branch `work`, no remote, a `y.txt` hunk w1 made → placed into w1 (was
   `Refused(NothingToAbsorb)`).
2. `main`, no upstream, a remote ref `refs/remotes/origin/main` at c3 of c1..c6: a hunk c5 made → placed; a hunk c2
   made → stays, `OutsideYourCommits`.
3. 12 commits, no remote: a hunk from 10 back → placed; from 11 back → stays.
4. A merge 3 back → range stops above it; a hunk from below the merge stays.
5. Every commit on a remote ref → the new refusal, not `NothingToAbsorb`.
6. Regression: a branch with an upstream gets the same range as before (existing tests stay green; name one).

### Touches (add to the list above)
- MAP.md (graft-ops) :253: replace the "first commit" fallback line with this rule.
- The keys ticket (`amend-into-and-absorb-keys`): a new `StaysWhy` variant and a new refusal need words there; say so
  in the report.
- `a`-into on the same branch still refuses a commit "not on your branch" (it uses `base_of`). Out of scope; report it
  to the conductor as a follow-up question, don't fix it here. **Superseded by the ruling below.**
- KNOWN_GAPS M7/06 "first-commit fallback" row: the conductor updates it from the report (builders don't read it).

## Ruling (planner): `a`-into on a branch with no upstream — 2026-09-26

**Owner: this ticket (graft-ops), not 09.** The refusal is the engine's: `rewrite::plan` returns
`NotOnYourBranch { commit, by: AmendInto }` (graft-ops MAP :263, :273). Ticket 09 (graft) only renders `explain()` on
the status line, so it changes nothing there. It belongs here because it must share the one range function Question
2's ruling builds.

**Decision: yes, the same rule, one function.** When the base resolution gives `base == HEAD`, `a` accepts a target
exactly when absorb could place a hunk into it: the fallback range above (≤ 10, unpushed, above any merge, not the
root).
- **Why the same:** both keys promise "a commit on your branch" (`RewriteKey::what_it_does()`). If absorb puts a hunk
  into c5 while `a` on c5 says `isn't on it`, one of them is lying (ux-principles: consistency, same meaning across
  keys). One function means they can't drift.
- **Why keep the cap for a pick he points at:** the cap is what "your commits" means with no upstream. Dropping it
  for `a` would make the two sets differ again. The cost is small: `a` on an unpushed commit 11+ back on an
  upstream-less branch is refused, with words that say why. `R` can still reach it (its `from` rule, MAP :263).
- **Why the pushed stop binds `a` too:** he works in a team. Rewriting a commit a remote ref holds is the harm
  absorb's rule exists to prevent. Pointing at the commit doesn't change who else holds it.

**What gets built (graft-ops; `rewrite/plan.rs` + the shared function, wherever the M7 split puts it):**
```rust
pub(crate) struct UnpushedRange { pub base: Oid, pub commits: Vec<Oid> /* newest first */, pub stop: RangeStop }
pub(crate) enum RangeStop { Cap, Pushed, Merge(Oid), Root }        // what ended the walk
pub(crate) fn unpushed_range(repo: &Repo, git: &Git, head: &Tip, stop: &dyn Stop)
    -> Result<UnpushedRange, RewriteError>;                          // absorb's fallback and AmendInto both call it
// Refusal additions (words in explain(), exact):
AlreadyPushed { commit: Oid, by: RewriteKey }   // "<short> is already pushed · <a|absorb> only changes commits you haven't pushed"
BeyondReach  { commit: Oid }                    // "<short> is more than 10 commits back · a reaches your last 10 unpushed commits"
```
(Signatures use graft-ops' own `Repo`/`Git`/`Stop` types as `absorb_map` does; match them, don't invent new ones.)
- Only when `base == HEAD`. With an upstream or a real default branch, `AmendInto` is unchanged.
- A target that isn't a first-parent ancestor of HEAD → `NotOnYourBranch` as today.
- Otherwise the target is in `commits` → plan as today. If not, the refusal comes from `stop`: `Pushed` →
  `AlreadyPushed`; `Cap` → `BeyondReach`; `Merge(m)` → the existing `MergeInRange { merge: m }`; `Root` → the existing
  `RootCommit`.
- Refusal order: the new two sit where `NotOnYourBranch` sits (after `NothingStagedFor`).

**Tests (red first, each named at tick time):**
1. `a_into_on_main_without_upstream_takes_an_unpushed_commit`: c1..c4 on `main`, no remote, target c3 → a plan with
   the `Fixup` row after c3 (today `NotOnYourBranch`).
2. `a_into_below_a_remote_ref_is_already_pushed`: `refs/remotes/origin/main` at c3 of c1..c6. c5 → plan; c2 →
   `AlreadyPushed`.
3. `a_into_eleven_back_is_beyond_reach`: 12 commits, no remote. 10 back → plan; 11 back → `BeyondReach`.
4. `a_into_below_a_merge_is_merge_in_range`.
5. `absorb_and_a_into_agree_on_the_range`: over the fixtures of 1–4 and Question 2's tests 2–4, the set of commits
   `absorb_map` may place into equals the set `AmendInto` plans for.
6. Regression, unchanged: an `a`-into test on a branch with an upstream (name one existing test).

**Touches (check each, report how):**
- `Refusal` matches outside graft-ops: grep `Refusal::` in `crates/graft`. An exhaustive match → `blocked: needs
  graft` naming the arm; a `Display`/`explain()`-only use is fine.
- Ticket 09's `a_off_the_branch_is_refused_in_words`: still true for a commit off the chain. It doesn't cover the new
  words; the conductor adds a 09 test if 09 hasn't landed.
- `R` (`Open`): unchanged. Say so, citing its test.
- Journal/undo, keymap, captures: none (a refusal is status words; a plan runs the existing preview).
- MAP.md (graft-ops) `rewrite`: `AmendInto`'s range rule and the two refusals.

**Size:** about 10k on top of 06c. If 06c is already past its budget, the conductor runs this section unchanged as its
own graft-ops slice after 06c.

## NOTES (builder, 2026-09-26)

No compaction. No scout: the ticket says none, every site was cited, and I read each (absorb.rs, plan.rs, reads.rs, Refusal) myself.

**Deviations, each said:**
- **M1 flag.** The review's fix, `--ignore-revs-file=` (as `graft-git/src/blame.rs:246` does), does **not** clear a configured file that is missing: probed on git 2.54, `git -c blame.ignoreRevsFile=nope blame --ignore-revs-file= …` still dies `could not open object name list: nope` (git keeps the list sorted, so `""` sorts first and `nope` is read after the clear). `--no-ignore-revs-file` works (probed). **Knock-on for 18's owner: `graft-git/src/blame.rs:246` has the same bug for G-blame** (not fixed: graft-git is not mine).
- **M4 read.** A bump's mode shows only on the patch's `index a..b 160000` line, which `graft-git`'s `parse_unified` does not keep (`diff/unified.rs:289-300` reads `old mode`/`new mode` lines only). So a second read, `diff-index --cached --raw -z --no-renames`, finds gitlinks. A new reason `StaysWhy::Submodule` (Minor 3: not `NoCommitChangedIt`).
- **Remote-tracking read (ruling).** The ruling says use a read graft-git exposes. The only one is `contained_in` (`for-each-ref --contains`, one per commit), which graft-git's own MAP measures at ~11 s per commit on 50k refs. I used `rev-list <prefix> --not --remotes` through `GitInvocation` instead (graft-ops already runs rev-list this way): one spawn, git's own reachability. No new graft-git item, so not `blocked`. Conductor: say if you want it moved into graft-git.
- **Signature.** `unpushed_range(repo, head, stop) -> Result<UnpushedRange, FactsError>`: no `git` (unused, review Minor 5's point), `FactsError` (converts into `RewriteError` at the callers).
- **New public fn `rewrite::absorb_plan_map(repo, facts, default_branch, stop)`.** Ruling test 2 needs `OutsideYourCommits` observable, and the absorb preview (09) needs the map over the same base the plan uses; `absorb_map` takes a base the app has no way to compute. `absorb_map`'s signature is unchanged (below-range reason `BeforeYourBranch`).
- **Empty range at `HEAD` a merge / the root:** `MergeInRange { HEAD }` / `RootCommit { HEAD }` (existing words), not `AllPushed`.
- `plan.rs` absorb moved to `rewrite/absorb.rs` (size); `root_of` removed (its only caller was the dead first-commit fallback).

**Red first, honestly:** M1/M4/M5 and the three rule tests were seen red before their fix or under the mutant. The ruling tests were written **after** the code; red was then shown by putting the old base rule back (`Some(base) => Above(base)`): 8 of 12 red. The 4 green under the old rule: the two regressions (expected), `a_into_below_a_merge_is_merge_in_range` (the old `refuse_merges` already says it), and **`a_into_on_main_without_upstream_takes_an_unpushed_commit` — the ruling's "today `NotOnYourBranch`" is wrong**: `amend_into` only asked `ancestor`, so `a` on main with no upstream already planned. What the ruling adds for `a` is the refusals (`AlreadyPushed`, `BeyondReach`).

**Regression tests named:** `absorb_unpushed::absorb_with_an_upstream_keeps_its_range`, `absorb_unpushed::a_into_with_an_upstream_is_unchanged` (new, upstream set); existing `absorb_oracle::absorb_agrees_with_the_trees_one_target`, `rewrite_plan::amend_into_plans_the_index_as_a_fixup_after_its_target` (default-branch base) stay green. `R` (`Open`) unchanged: it calls `base_of` directly, untouched; `rewrite_plan::open_runs_from_head_down_to_where_the_branch_left_main` green.

**Behaviour change to know:** a branch whose upstream equals `HEAD` (all pushed) now gets the fallback: absorb → `AllPushed` (was `NothingToAbsorb`), `a` into a pushed commit → `AlreadyPushed` (was planned). That is the ruling's `base == HEAD` condition applied as written.

**Words the keys ticket (09) needs:** `StaysWhy::Submodule`, `StaysWhy::OutsideYourCommits`; refusals `AllPushed` (`every commit on <branch> is already pushed · absorb only changes unpushed commits · c commits them`), `AlreadyPushed` (`<short> is already pushed · a only changes commits you haven't pushed`), `BeyondReach` (`<short> is more than 10 commits back · a reaches your last 10 unpushed commits`) — all in `explain()` already; the stays need preview words. 09 should draw stays from `rewrite::absorb_plan_map`.

**Not done / out of scope:** M5's knock-on (`operation/stage.rs`, `apply/set_aside.rs`, `preview/stash.rs` also run `git apply` without pinning `--whitespace`) not checked. Minor 2 (nothing staged → own words) untouched. No git-absorb/jj comparison (neither installed).
