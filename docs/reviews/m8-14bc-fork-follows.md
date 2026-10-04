# Review — m8 14b (fork follows: engine) + 14c (fork follows: preview words)

Reviewed: `f68ccd16^1..f68ccd16` (14b), `77b31aca^1..77b31aca` (14c), at main `973cfdbf`.
Reviewer: one round, both tickets as one group. Scouts read: `scouts/14b-fork-follows.md`, `scouts/14b-reword-fork.md`.

**Counts: 0 Critical · 3 Major · 9 Minor.**
**Ticks: 19 checked, 19 held** (14b 10/10, 14c 9/9: each test named was found, read and run, see "Ticks").

## What I verified, and how

- **Gates** (own detached worktree, own `CARGO_TARGET_DIR`): `cargo fmt --check` ok; `cargo clippy --workspace
  --all-targets -- -D warnings` ok; the same with `--features heavy-tests` ok.
- **Targeted tests run green:** graft-ops `it` `fork_follows::` + `rewrite_oracle::` + `rewrite_preview::` (53),
  graft-ops lib `rewrite::`/`preview::` (24), graft `it` `m7_walk::fork_rows` + `m7_walk::rewrite_editor` (10), graft
  lib fork/banner/editor tests (23), graft-ui `banner`/`rewrite_editor` (35).
- **Not run by me:** full `cargo nextest run`. I relied on the conductor's run on main after 09 (`docs/log/m8.md`:
  5460 passed), which includes both merges. Heavy tier and release perf were not run: the conductor said no perf runs,
  and a builder was using cargo. Fast-suite time, slowest tests, peak memory and `target/` size were **not measured**.
- **Probes**: five throwaway tests in my worktree, deleted afterwards. Truth read with git. Results are quoted under
  M1–M3.
- **Hand-run git 2.54**: a `rebase -i` todo with `update-ref refs/heads/G`, where `G` is checked out in a linked
  worktree. Git **moved `G`** and printed "Updated the following refs with --update-refs: refs/heads/G". Git does not
  refuse.
- **Feature pipeline:** the design note "Pass 2c" has Need (Problem/User), Prior art (row :71, citing
  `docs/research/stacked-branches.md` §1.2–1.4) and a Journey (`a`, `y` → 2+1; jj 0 extra; branchless +1). The prior-art
  study exists. No new screen, so no prototype was owed. No walk counts this journey's steps (m5 below).

## Critical

None.

## Major

### M1 — Branches on a following fork are moved without being weighed: elsewhere, pushed and default-branch rules bypassed
- **Where:** `crates/graft-ops/src/preview/forks.rs:128-131` (a `claimed` candidate is skipped, never weighed) and
  `:231-248` (`Fates::follow` claims every candidate whose tip is among the follower's commits as `inside`, with no fate
  check). `crates/graft-ops/src/preview/effects.rs:232-233` (the `Elsewhere` doc says git "would refuse to move" such a
  branch: false, see the hand-run above).
- **What happens (probes, git truth):** `feature-x` = x1 x2 off c2, `feature-w` at x1.
  - `feature-w` is checked out in a linked worktree; `a` into c1 with `c1.txt` staged. Fates list only `feature-x`
    (Follows), and ref moves include `refs/heads/feature-w`. After the run, the linked worktree's `git status --short`
    shows **`M  c1.txt`**. That is a staged change the teammate never made: the reverse of the amend. If he commits in
    that worktree, he undoes the amend.
  - `feature-w` is pushed (`origin/feature-w` = x1): it is moved off its pushed commit, and no fork row says so.
- **Why it matters:** ADR-0059 §2 says a fork checked out elsewhere or pushed **stays**, and ADR-0047 refuses
  `DependentElsewhere` for the line for exactly this reason. The bypass reaches a teammate's worktree. Nothing is lost
  (`u` restores refs, and his files are untouched), but his tree now silently disagrees with his `HEAD`. The M5 promise
  also fails: the preview never names `feature-w`.
- **Fix:** weigh each `inside` branch with the same gates as a fork (default, elsewhere, pushed). If any fails, the
  fork stays and names it, e.g. `feature-x stays on <at> · feature-w is checked out in <path>`. Fix the `Elsewhere`
  doc. Add a git-truth test for each of the three cases.

### M2 — The "pushed" rule checks only `at..F`: a fork whose upstream holds `at` follows and diverges from its remote
- **Where:** `crates/graft-ops/src/preview/forks.rs:363-370` calls
  `holds_any(upstream, at, commits)`. `crates/graft-ops/src/preview/rewrite.rs:366-383` reads `rev-list <upstream>
  ^<at>`, so `at` itself and the commits below it are never weighed. For a fork with 0 commits of its own,
  `commits` is empty and the rule can never fire.
- **What happens (probe 3, `R`):** branch `park` sits at c2, and `origin/park` = c2. Result: `park` Follows and is
  moved off `origin/park`; `drawn.published` is `[]`. Before 14b, `park` stayed (`left behind`). The same holds for a
  fork pushed before its own commits were made. For `a` and absorb this is unreachable, because `your_commits` stops at
  any commit reached by `refs/remotes/*`. `R`'s range is not bounded that way.
- **Why it matters:** this is the brief's "a diverged fork must stay and say so". The preview says `follows`, but after
  `y` the branch needs a force push that nobody asked for. `on origin: … P will need force after` is never drawn for
  it.
- **Fix (ADR-0059 §2 amendment, planner):** treat an upstream that holds `at` or any replaced commit as `Pushed`.
  Alternatively, let the fork follow but add it to `published`. Add a test for each.

### M3 — A fork off a dropped (or reordered) commit follows without it; nothing on screen says so
- **Where:** `crates/graft-ops/src/preview/forks.rs:323-331`: `after_row` is `at`'s own row, whatever its action. For
  a `drop`, `crates/graft-ops/src/preview/chain.rs:94,117` gives the result of the row below. The type documents this
  (`effects.rs:199-200`, "or of the row below it, when `at`'s row is dropped"), but ADR-0059 never decides it. The same
  mapping applies to reordered rows: a fork follows after `at`'s row in its **new** position, so its base gains every
  row moved below it.
- **What happens:**
  - Probe 4: `R` with c2 dropped. `feature-x` Follows, and after the run `git show feature-x:c2.txt` fails: the
    teammate's branch has lost c2.
  - The shipped capture `docs/shots/m8/fork-follows-rewrite-editor-187x44.txt` shows exactly this case:
    `drop ✗ 1875dfa feature a: part 1 left out` next to `feature-b follows · 1 commit from 1875dfa`. The row names the
    dropped commit as the fork's base and never says the fork loses it.
- **Why it matters:** the user ruled "it moves too **when it moves clean**". A textual clean replay is not the same as
  "his teammate's branch keeps what it was built on". A branch can lose code it depends on, and the M5 promise ("you
  see the result before you confirm") is not met by these words. jj's abandon does the same (prior art supports it),
  but it was never chosen here.
- **Fix (ADR-0059 amendment):** decide between Stays(`its fork point is dropped`) and Follows with the loss in the
  words (`· without 1875dfa`). For reordering, decide whether the base is `at`'s row or the rows up to `at` in the old
  order. Add a git-truth test either way.

## Minor

- **m1 — The default-branch check ignores the app's override.** `crates/graft-ops/src/preview/forks.rs:308`
  `default_branch_of(repo, None)`, while `dependents`/`plan` take `default_branch: Option<&RefName>` (MAP `## rewrite`).
  A user-configured default branch that forks off his branch could follow. Pass the override through the preview, or
  record why not.
- **m2 — `plan.json`'s `forks` is written and never read.** `crates/graft-ops/src/rewrite/saved.rs:58`. The only uses
  of `.forks` in graft-ops src are the writer and a test (grep). `progress` names the fork from git's files instead.
  Either read it (relaunch/recovery) or say in MAP that it is diagnostic only. As it stands it is a format contract
  with no consumer.
- **m3 — `WhyStays::PastCap` is never constructed.** `crates/graft-ops/src/preview/effects.rs:249`: forks past the cap
  are only counted (`forks_more`). The variant and its words `past the first 8` are dead. Remove it, or note it as
  reserved.
- **m4 — One predicate in two places.** `crates/graft/src/update/preview/line_rows.rs:50` `follows` and
  `crates/graft/src/view/rewrite_editor/right.rs:311` `is_follower_fork` are the same test. Keep one, e.g.
  `PreviewPlan::fork_follows(name)` in graft-ops (PROMPT 6.2).
- **m5 — No app-level walk proves `y` moves the fork.** `crates/graft/tests/it/m7_walk/fork_rows.rs` only captures
  before `y`. The engine is proven in graft-ops, but the app path from `a` through the preview job (`plan_as_run`,
  `jobs/preview.rs:63`) to `y` is covered only by unit tests (`y_launches_the_planned_rewrite`). The journey "`a`, `y` →
  2+1, fork included" is not step-counted anywhere. Owed by `accept-m8`; say so there.
- **m6 — Glossary.** `CONTEXT.md` has no entry for **fork**, ADR-0059's domain term, nor for its on-screen words
  `follows` / `stays on`, and `left behind` is now `r`-only. Add: "Fork: a local branch started from a commit in the
  middle of his branch… on screen `follows` / `stays on`. Avoid: left behind (except `r`)".
- **m7 — Gap and capture bookkeeping is stale** (the conductor's edits):
  - `KNOWN_GAPS.md:1083` M7/37-midchain-follows is still open, though 14b closes it for rewrites.
  - `:1131` M8/14c-fork-wording is still open, though 14c's `follows` note settles it.
  - `:1103` M8/fork-follows-r lacks R1's "remove `left_behind`/`LeftBehind` once `r` reports forks".
  - Capture names `docs/shots/m7/amend-into-left-behind-187x44.txt` and `rewrite-editor-left-behind-187x44.txt` now
    show a follow.
- **m8 — The fork forecast's cost is unmeasured.** `crates/graft-ops/src/preview/forks.rs:410` runs one
  `chain::chain` per candidate: up to 8 × 50 `merge-tree` spawns, plus `own_commits`/`holds_any` each. It runs on every
  rewrite-editor replan. It is off the UI thread and cancellable, but no timing exists. Measure it in the heavy tier or
  at acceptance.
- **m9 — The nested rule differs from ADR-0059 §2's text** (question). The ADR says Follows needs "no other fork
  leaves `at..F`". The code (`forks.rs:128-145`, `:371-373`) lets the longer fork follow and the nested one stay
  (`a_fork_of_a_fork_stays_nested` asserts `feature-x` Follows while `feature-y` forks off it). The ADR's own words
  (`forked from <fork>, which moves`) agree with the code. Fix the ADR sentence.

## Ticks spot-checked

- **14b (10/10 held):**
  - `a_clean_fork_follows_an_amend_into`, `a_conflicting_fork_stays_and_the_line_moves`.
  - `a_branch_off_a_dependents_middle_is_refused_as_a_second_dependent`, `a_dependent_and_a_fork_off_his_middle_both_follow`.
  - `undo_after_a_follow_puts_every_ref_back`.
  - `rewrite_oracle::a_fork_follows_in_the_same_rebase` (hand-run todo, same refs).
  - `an_abort_at_a_stop_after_a_section_leaves_the_fork`, `progress_inside_a_section_names_the_fork`.
  - The seven stays tests, `a_fork_moved_after_the_preview_is_refused_moved`, `a_stop_resumed_after_relaunch_finishes_the_section`.
  - `saved::tests::a_plan_saved_before_forks_reads_as_none`.

  All bodies were read and assert what each tick says; all were run green.
- **14c (9/9 held):**
  - `a_rewrite_box_draws_one_row_per_fork`, `fork_rows_table` (0/1/N, `+1 more stays`, `+N more stay`).
  - `a_conflicting_fork_row_warns`, `a_rebase_box_keeps_left_behind`.
  - `the_after_side_names_each_fork`, `a_fork_that_stays_for_a_plain_reason_is_muted`.
  - graft-ui `each_fact_draws_in_its_tone` (buffer fg per token).
  - `a_stop_inside_a_fork_names_the_fork` / `a_stop_inside_a_fork_says_the_fork`.
  - The three `fork_rows` walks.
  - B5's grep: `left behind:` appears in `docs/shots` only in the m5 `r` shots.

  Colour is **unverified**, as the tickets say.

## Knock-ons checked

- **Undo/journal:** `rewrite::tips` adds followers and `inside` (`operation/rewrite.rs:52-55`), feeding `check`'s
  `Moved` and `touches`. Undo is proven for a fork; `inside` is untested but on the same path.
- **What `y` runs:** the app always runs `plan_as_run`'s op (`jobs/preview.rs:63`). The editor's `y` runs the plan's
  `runs` (`rewrite_editor/tests.rs:878`).
- **Banner progress read:** on the job thread (`update/banner/stop.rs:41`). The new `rev-parse` runs only inside a
  section.
- **Sanitising:** fork names and reasons pass through `sanitize_line`: box `plan_lines.rs` final map, editor
  `right.rs`, banner `update/banner.rs:236`.
- **`r`:** unchanged (forks empty, `apply/engine.rs:222-224`). `R` from an `r` preview becomes a Rewrite, so its forks
  do follow there.
