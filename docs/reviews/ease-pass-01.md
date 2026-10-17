# Review: ease-pass group 01, roles-and-route (signature)

Reviewed against merge `862d40d2` (diff `862d40d2^1..862d40d2 -- crates/`, 16 files, +1516/−2). Read-only.
Inputs: ticket `.scratch/ease-pass/issues/01-roles-and-route.md` (ticks at end), scout
`.scratch/ease-pass/scouts/01-roles-and-route.md`, `docs/design-notes/ease-pass.md` (Interfaces, Hard-to-reverse 1),
ADR-0063, MAP.md of graph/ops/git.

**Gates:** not run. The conductor's order was no cargo, because two builds were using the CPU. Main reported
6488/6488 green. I did not observe it. No heavy tier, no release perf run. This ticket adds no screen and no hot path,
but the next whole-milestone review still owes the release perf run. Where this review makes a behavioural claim, it
rests on code lines at `862d40d2` plus topology checks done with plain `git rev-list` in scratch repos (no cargo).
Rule 1: none of these claims is backed by a failing test. The first fix for each finding is to write that test.

**Skills:** `code-review` (both axes, Standards and Spec, done inline; no sub-agents).

## Verdict

The role rule is right, and so are the types, the stop handling and the module split. The builder followed the
spec's rules faithfully. The problem is in the **rules for "built on" and "merged"**: they give wrong answers on
topologies a team produces every day (siblings off a shared base, a teammate syncing a stacked branch, branches cut
from `main` after a merge). The fixtures cover none of these. Every finding below is a **spec-level rule** that the
build reproduces. They go to the planner, not back to the builder alone.

| Severity | Count |
|---|---|
| Critical | 0 |
| Major | 4 |
| Minor | 6 |

## Role rule vs ADR-0063: holds

- `crates/graft-graph/src/role.rs:80-90`: focus → `This`, else trunk → `Trunk`, else chain → `Under`, else `Other`.
  That gives ADR-0063's green this / amber built-on / blue trunk / grey other. The focus is the cursor row's branch
  through `made_on` (`role.rs:96-98`, `row.lanes.get(row.lane).copied().flatten()`). That makes the role relative to
  the cursor row, as ADR-0063 and note G1 say. When the cursor is on the trunk, the trunk reads `This`
  (`role.rs:81`). The spec says so (`focus_on_the_trunk_is_this`).
- `Route::new` drops a trunk passed in `built_on` (`role.rs:44-48`), so the trunk can never read `Under`. This is
  good defensive code with a named test (`trunk_in_built_on_is_dropped`).
- The crate rule holds. `role` imports only `identity` and `row` (`role.rs:9-10`). graft-ops' new code imports only
  graft-core and graft-git (`branch_status.rs:16-19`, `status.rs:6-13`).

## The deviation: route skips walked tips

The spec asked for a cycle guard (stop on a repeat). The build skips every candidate whose tip is the branch's or a
walked base's tip (`branch_status.rs:131-134`, `:153`). That **beats** stopping on a repeat. With a stop, `main`
would drop off `sara/fix-toc`'s route as soon as `wallet-release` moves on, and
`route_does_not_walk_back_through_the_branch` (`tests.rs:335-347`) shows the case. **Accept it.** It is tested, and
it was reported as a deviation. The skip is by tip alone, though, so it only removes the walked branches
**themselves**. Their siblings and children still win, as Major 1 shows.

The conductor approved the graft-git edit (`commits_from`, outside the ticket's "no graft-git edit") mid-ticket
(`docs/log/ease-pass.md:24-26`). It is small (`compare.rs:89-104`), takes a stop, and has two tests. Fine.

## Ticks spot-checked: 9 checked, 9 held

I checked all 9 acceptance groups. Every test named in the handoff exists, and each asserts what its tick claims:
`role_table`, `focus_on_the_trunk_is_this`, `no_focus`, 3× `made_on_*`, `short_name_table`,
`choose_base_picks_the_nearest` (proptest) plus the 3 worked examples, the 3 route fixtures, `route_does_not_cycle`,
`route_stops_at_depth_eight`, the 10 status fixtures, `route_and_status_honour_stop`, `commits_from_honours_stop`.
The MAP entries are present in all three MAP.md files. No false tick.

Two ticks hold only in the narrow sense:
- the stop tick (see Minor 1);
- the fixture's `wallet-release` is set to `Integration` (`tests.rs:255`). That hides Major 1's tie case.

## Critical

None.

## Major

### 1. A sibling branch is chosen as "built on", so a teammate's lane is drawn amber

`crates/graft-ops/src/branch_status.rs:79-81` (minimum `branch_only`, then kind, then name),
`:131-134` and `:153` (skip by walked tip only).

There are two ways this happens.

**(a) A tie at step 1.** Take a sibling that left the base at or after the branch's own fork point. Its `branch_only`
equals the true base's. If the base is `Other` kind, the tie goes **by name**. `refs/heads/ali/x` sorts before
`refs/heads/wallet-release`, so `ali/x` becomes `sara/fix-toc`'s base.

**(b) Step 2 and later.** Once the base has moved on, any branch cut from it earlier has fewer `branch_only` than
the trunk. The skip-by-tip removes `sara` but not `sara`'s siblings.

Topology checked with `git rev-list --count` in a scratch repo: `main m1 m2`, `wallet-release w1 w2` off `m2`,
`sara/fix-toc s1 s2` off `w2`, `ali/x t1` off `w2`, then `w3` on wallet.

- Step 1 (sara): `branch_only` against main = 4, wallet = 2, ali/x = 2. That is a tie.
- Step 2 (wallet): against main = 3, ali/x = 1. ali/x is picked.

So the route reads `[wallet-release, ali/x, main]` when wallet is `Integration`, and `[ali/x, wallet-release, main]`
when it is `Other`. Either way `ali/x` takes the `Under` role (amber), and in the second case `Behind` and
`NotYet { into }` name `ali/x`.

Why it matters: this is the user's daily shape, a team on dependent branches off a shared base
(memory `works-in-a-team-on-dependent-branches`). ADR-0063 only accepted a wrong base "after a rebase onto a
sibling" (note, "Where I'd differ" (a)). This case has no rebase.

Why git counts can't settle it: here, git history alone can't tell `ali/x` cut from `wallet-release`@w2 apart from
`wallet-release` continuing from `ali/x`'s fork. Both first-parent lines run `… → w2 → w1 → m2`.

Suggested fix (planner's decision; the hard-to-reverse choice 1 needs revisiting):
- (i) From step 2 on, consider only candidates whose kind is at or below the current base's (an integration
  branch's base is trunk or integration, never a feature).
- (ii) Where the tie is real, break it with something git does record: the creating reflog entry
  (`branch: Created from …`) when it exists, or a parent the app records when it makes a branch. Prior art records
  parents rather than measuring them (git-town's per-branch parent config, Graphite and git-spice stacks). The note's
  "Branch status: none fits" row did not study this, so send the researcher.
- Add a sibling to the `stack()` fixture, and make `wallet-release` `Other`.

### 2. A branch merged into `main` reads "merged into" a teammate's later branch

`crates/graft-ops/src/branch_status/status.rs:246-257`. Holders are ordered by fewest `base_only`, and the first one
that is not stacked wins.

Every branch cut from `main` after the merge also holds the merged branch. Its first-parent walk leaves the range at
the merge's first parent, so `stacked_on` returns `false` (`status.rs:271-278`) and the branch counts as a container.

Checked with git: `sara s1` merged `--no-ff` into main, `bob/y y1` cut from main right after, main then gets m2 m3 m4.

| Holder | `base_only` | First-parent walk |
|---|---|---|
| main | 4 | m4 m3 m2 Merge |
| bob/y | 2 | y1 Merge |

Both walks leave the range at `m1`, so bob/y is nearer and the answer is `Merged::Into("refs/heads/bob/y")`.

Why it matters: this is the common case for any merged branch once `main` moves on. The pane would say
"merged into bob/y" for work that landed in `main`.

The spec's rule ("nearest container, fewest `base_only`") produces this, and no fixture has a second branch cut
after a merge.

Suggested fix: order containers by **kind first** (trunk, integration, other), then `base_only`. Or identify the
merge commit that brought the tip in, and name its branch with `merge_name::recover_branch_name`. Add the bob/y
fixture.

### 3. A teammate's stacked branch that syncs its base makes the base read "merged"

`crates/graft-ops/src/branch_status/status.rs:264-279`. `stacked_on` answers "stacked" only when the first-parent
walk reaches a commit whose first parent **equals the current tip** (`:273`).

Take a teammate's branch cut from an **older** commit of the branch that later merged the branch to catch up. Its
walk passes through the old commit, which lies outside the range (it is an ancestor of the tip), so the function
returns `false`.

Checked with git: `bob/x` cut from `sara`@s1 with `x1`, sara adds `s2`, bob runs `git merge sara`.
- `branch_only(sara vs bob/x)` = 0.
- The range `^sara bob/x` is {Merge, x1}. The walk goes Merge → x1 → s1. `s1` is not in the range and is not the tip.

So `sara` reads `Merged::Into("refs/heads/bob/x")`.

Why it matters: this is exactly the dependent-branch workflow, and the pane tells him his unmerged branch is merged.
`a_branch_stacked_on_top_is_not_a_merge` (`tests/status.rs:82-97`) covers only the case where the stacked branch was
cut from the current tip.

Suggested fix: when the walk leaves the range, answer "stacked" if the commit it lands on is one of the branch's
**own** commits (the `Who` range against `route[0]`, `status.rs:164`) rather than only `== tip`. A real merge into
`wallet-release` still leaves at `w2`, the fork point, which is not an own commit, so it keeps reading as merged.
Add the fixture.

### 4. A branch with no base shows other people as "who", and "no base" while it is behind `main`

`crates/graft-ops/src/branch_status/status.rs:163-166` (empty route → `commits_from(branch.tip, 2000)`) and
`:143-146` (empty route → `Behind::NoBase`).

The route is empty whenever every candidate holds the branch:
- every branch merged into `main`;
- a fresh branch before its first commit;
- a branch left at an older `main` commit.

`Who` then lists the top authors of `main`'s history. `who_of_a_branch_with_no_base` (`tests/status.rs:229-248`)
**asserts** `["Bea Brandt", "Alice Ames"]`, where Alice only made `m1` on `main`. The test locks in the wrong
answer.

`Behind` reads `NoBase` for a fresh branch even when `main` is 2 ahead.

This is what the spec says, and the builder flagged it ("whether the right pane shows it is B's call"). But B has no
field telling it this `Who` is not the branch's own; it can only infer that from `route.is_empty()`.

Why it matters: right after `git switch -c`, his own pane names strangers. Every merged branch names `main`'s
regulars.

Suggested fix (spec change): for a merged branch, read `Who` between the introducing merge's first parent and the
tip (Major 2's merge search finds it). For a branch held by the trunk on its first-parent line, give `Who` "no
commits of its own" and `Behind` against the trunk. Change the test's expectation.

## Minor

1. **The stop test only reaches the entry guard.** `tests.rs:380-394` starts with a stopped `Stop`, so both calls
   return at `branch_status.rs:104` and `status.rs:118` before any read. A stop that arrives mid-walk is untested.
   Every read below passes `stop` through, so the risk is low. Fix: add a `Stop` that trips after N calls.
2. **A broken invariant falls back silently.** `status.rs:139-146` looks `route[0]` up again in `measured`, and on a
   miss quietly answers `Behind::NoBase`. The comment says it can't happen. Fix: have `route_from` return the base's
   `Apart` along with the route, so there is no lookup and no fallback (rule 3 in spirit).
3. **Question: squash and rebase merges.** `Merged` is ancestry-only (`status.rs:246-258`), so a PR merged by
   GitHub's squash or rebase options reads `NotYet` forever. Which merge method does his team use? The answer
   decides whether this is a gap.
4. **Cost is unmeasured.** `measure` spawns 2 `rev-list --count` per candidate per step (`branch_status.rs:162-165`),
   up to 8 steps. Each holder adds a `rev-list` of up to 10 000 commits (`status.rs:20`, `:265`). With 40 branches
   at depth 3 that is about 240 spawns. Off the UI thread here, but B must measure it under its per-tips cache
   (handoff "For B").
5. **Old merges fall back to "not yet".** Beyond `STACK_CAP` (`status.rs:266-267`), a branch merged more than 10 000
   commits ago reads `NotYet { into: main }`. This is documented, and it matches the spec's "capped → not a
   container". List it in KNOWN_GAPS when B ships the pane.
6. **Question: `MainLine` for a branch that isn't the trunk.** `status.rs:157-159` answers `Merged::MainLine` for a
   non-trunk branch when no trunk is among the candidates. The builder reported this decision, and it fits no
   spec'd meaning. A `NoBase`-style variant would say what is true.

## Readability and architecture

- The files are under 400 lines: `status.rs` 341, `tests.rs` 396 (close to the limit).
- `branch_status` (`status.rs:111-176`) is about 60 lines, at the limit. Its parts are already named helpers. To
  split it, pull out `merged_of(..)`.
- Names are full words and the doc comments are plain. There is no `unwrap` or `expect` outside tests, and no
  ignored `Result`.
- MAP.md entries match the code. ARCHITECTURE.md lists no modules at this level, so nothing has drifted.
- Not applicable: this ticket has no UI, no terminal output, and no journey steps. The journeys are counted in G/B.
