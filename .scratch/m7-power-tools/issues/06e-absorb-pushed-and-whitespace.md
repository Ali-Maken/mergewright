# 06e absorb-pushed-and-whitespace — review 06c's M1, M2, M3, Minors 2, 3, 5, 6 and both Questions

Status: ready
Blocked by: 30 (holds graft-ops; M3 is in the put-back 30 rewrites).
Crates: graft-ops only.
Build with: Opus (the team-safety rule, and a put-back).
Scout: none. Source: `docs/reviews/m7-06c-absorb-config.md`.
Split by: conductor, 2026-09-26. Ruling on M2, Minor 2 and Q1–Q2 added by the planner, 2026-09-27 (below).
Size: ~70k with the ruling. Past budget → the conductor runs "Ruling" as its own graft-ops slice (06f), unchanged, after the rest.

## Scope (each at the cause, red first)
- **M1:** the pushed stop is unpinned. A mutant that counts only remote *tips* as pushed passes all graft-ops tests. A mutant that counts only `*/main` passes the 12 `absorb_unpushed` tests, and a probe showed it rewrites a teammate's commit. Add tests that kill both mutants: a pushed commit below a remote tip, and a commit on a non-`main` remote branch. Show each mutant red. After the ruling below, run both mutants against the ruling's tests too (the stop now binds every base).
- **M3:** with `apply.whitespace`, `fix` strips a stay's staged spaces and `error` fails the run (`apply/set_aside.rs:508-514`; re-read after 30, since the lines will have moved). Fix at the cause, as 06c did for absorb's own apply.
- **Minor 3:** `AlreadyPushed.by` only ever holds `AmendInto`, and `by = Open` would render an untrue sentence. Drop the field, or document it. (Still true after the ruling: absorb refuses with `AllPushed`, never `AlreadyPushed`.)
- **Minor 5:** the oracle hard-codes `main..feature` (`absorb_printed.rs:72`). Take the range as a parameter, and add one oracle row on the fallback range.
- **Minor 6:** split `absorb_and_a_into_agree_on_the_range` into one test per fixture.
- **The ruling below** (M2, Minor 2, Q1, Q2).

## Ruling (planner, 2026-09-27) — one range for `a` and absorb, on every base

**Facts relied on** (review `docs/reviews/m7-06c-absorb-config.md`, lines as it cites them; re-read each before editing):
- With a base of its own, `your_commits` gives `Above(base)` (`rewrite/unpushed.rs:67-68`) and `a`'s range check returns `Ok` for it (`rewrite/plan.rs:245-247`): `a` plans into any ancestor, pushed or below the base (review :101-104, :163-165).
- Absorb with an upstream takes `base..HEAD` where the base is the upstream's merge-base, so it already leaves his own pushed commits out (review :168-170, `absorb_unpushed.rs:216-219`). `a` on the same fixture plans into them (`:309-318`).
- Pushed without `-u` (probe p7): both keys rewrite pushed commits (review :171-173).
- A new branch at an unpushed `main`'s tip falls into the fallback, because it tests `base == HEAD` alone (`unpushed.rs:68`, probe p4, review :245-254).
- A second-parent target reaches the range check and is refused `MergeInRange` (`plan.rs:186`, pinned by `absorb_unpushed.rs:302-305`).
- The design already calls "his branch" on a tracked branch his unpushed commits: R's rows run "down to … the merge-base … with the upstream if set" (`docs/design-notes/m7-power-tools.md:467-468`). R2's user is "him, weekly, while reviewing his own branch **before a push**" (`:539`).
- `a` on HEAD with nothing built on it is today's plain amend, not `AmendInto` (ticket 09 "Decided"); with a dependent it is `AmendInto { target: HEAD }` (journey 7).

**Decisions:**
1. **(M2, Q2) The pushed stop binds every base.** "Your commits" = HEAD's first-parent commits, walked from HEAD. At each commit the walk stops, checking in this order: reachable from any `refs/remotes/*` ref → `Pushed`; it is the base → `Base`; a merge → `Merge`; the root → `Root`; and — **only when there is no base below HEAD** — `ABSORB_FALLBACK_DEPTH` already taken → `Cap`. Otherwise it is taken. Absorb places only into the taken commits; `a` accepts a target only among them. One function, both keys. The old `Above(base)` pass-through goes.
   - `Pushed` is checked before `Base` because an upstream base is itself pushed: `a` into it must say `already pushed`, not `isn't on it`, which would be false.
   - Why: 06c's own principle, "rewriting a commit a remote ref holds is the harm" (06c ticket :101-102), held only on the fallback. He works in a team on dependent branches (memory `works-in-a-team-on-dependent-branches`); a teammate's branch built on his holds his pushed commits. And absorb already treats his pushed commits as not his; `a` disagreeing is the "one of them is lying" case (06c ticket :95-97, ux-principles consistency).
   - Q2 dissolves: `a` no longer depends on whether he has one unpushed commit.
   - `R` is unchanged: it rewrites pushed commits, and its preview says `on origin: <b> · P will need force after` (note :494).
   - **Question for the user** (reported by the planner, not built): whether `a` should reach his own pushed feature branch with that force-push warning. Built as above until he answers. If yes, the change is local to this function: "pushed" excludes his branch's own upstream, never the default branch's.
2. **(Q1) The fallback is for the default branch or the only branch.** With no base below HEAD, the walk runs only when HEAD's branch is the default branch (as `plan` resolves it, the app's override included) or the only local branch — 06c's own stated premise (06c ticket :36-37). Otherwise the range is empty, stop `NotItsOwn`: absorb refuses `NoCommitsOfItsOwn`, and `a` refuses `NotOnYourBranch`. Detached HEAD must not reach the fallback; name the test that shows what it gets.
3. **(Minor 2) A second-parent target stays `MergeInRange`.** The target is reachable only through the merge, and the words name the merge, which is the reason. 06c's "`NotOnYourBranch` as today" contradicted "as today" (`MergeInRange`); this ruling corrects it. Rule: a target that is not an ancestor of HEAD → `NotOnYourBranch`; otherwise, if it is not taken, the walk's stop refuses (table below). Every target not taken is an ancestor of the commit the walk stopped at, so `AlreadyPushed { target }` is always true when the stop is `Pushed`.
4. **Words stay true where they fire.** `AlreadyPushed`'s `a only changes commits you haven't pushed` is now true on every commit but HEAD (5). `BeyondReach` fires only on the default or only branch, so it names it: `<short> is more than 10 commits back · on <branch> a reaches the last 10 unpushed commits`.
5. **`a` on HEAD is amend, whatever is built on it.** `AmendInto { target: HEAD }` skips the range check: plain amend (nothing built on HEAD) has none, and the key must not change meaning because a dependent exists (ux-principles consistency). Every other refusal for it is unchanged. This also lifts 06c's `AlreadyPushed` for a pushed HEAD on the fallback. `AlreadyPushed`'s words then say `a only changes commits you haven't pushed`, which is false for HEAD — so they become `<short> is already pushed · a only changes older commits you haven't pushed`.

**Refusal by the walk's stop** (targets other than HEAD):

| Stop | `a` on a target not taken | absorb, range empty |
|---|---|---|
| `Pushed` | `AlreadyPushed { commit: target }` | `AllPushed { branch }` |
| `Base` | `NotOnYourBranch` | — (never empty) |
| `NotItsOwn` | `NotOnYourBranch` | `NoCommitsOfItsOwn { branch }` |
| `Merge(m)` | `MergeInRange { merge: m }` | `MergeInRange { merge: HEAD }` |
| `Root` | `RootCommit { commit: target }` | `RootCommit { commit: HEAD }` |
| `Cap` | `BeyondReach { commit, branch }` | — (never empty) |

Refusal order in `plan`: unchanged (the new one sits with `AlreadyPushed`/`BeyondReach`).

**A hunk that stays because its lines lie below the range** — by the commits its lines were last changed by, first match:
1. every one is reachable from a `refs/remotes/*` ref → `StaysWhy::AlreadyPushed` (new);
2. below the base → `BeforeYourBranch` (as today);
3. otherwise (below the fallback's cap or merge) → `OutsideYourCommits` (as today).
One remote-reachability read per absorb, not one per stay.

### Signatures (graft-ops; match the existing parameter types of `your_commits` / `absorb_plan_map` — the result shape is the contract)
```rust
// rewrite/unpushed.rs — replaces `YourCommits::{Above, Unpushed}` and `UnpushedRange`
pub(crate) struct YourCommits {
    /// Parent of the oldest commit taken; `HEAD` when none were taken.
    pub base: Oid,
    /// First-parent, newest first.
    pub commits: Vec<Oid>,
    /// What ended the walk.
    pub stop: RangeStop,
}
pub(crate) enum RangeStop { Pushed, Base, NotItsOwn, Merge(Oid), Root, Cap }
pub(crate) fn your_commits(repo: &Repo, facts: &Facts, default_branch: Option<&RefName>, stop: &dyn Stop)
    -> Result<YourCommits, FactsError>;       // absorb (`plan`, `absorb_plan_map`) and AmendInto both call it

// operation.rs — Refusal
NoCommitsOfItsOwn { branch: /* AllPushed's branch type */ },
    // "<branch> has no commits of its own yet · c commits them"
BeyondReach { commit: Oid, branch: /* same */ },
    // "<short> is more than 10 commits back · on <branch> a reaches the last 10 unpushed commits"
AlreadyPushed { commit: Oid /* , by: per Minor 3 */ },
    // "<short> is already pushed · a only changes older commits you haven't pushed"

// absorb — StaysWhy
AlreadyPushed,
    // preview words (09 renders): "stays staged · the commit that changed these lines is already pushed"
```

### Tests (red first; each named at tick time; in `tests/it/absorb_unpushed.rs` or a sibling ≤ 400 lines)
1. `a_into_a_pushed_commit_with_an_upstream_is_already_pushed`: feature f1..f3 on `main`, upstream `origin/feature` at f1, no remote for `main`. `a` into f1 → `AlreadyPushed`; into a `main` commit → `AlreadyPushed` (`origin/feature` reaches it); f2 → plan. **Replaces** `a_into_with_an_upstream_is_unchanged` (`:309-318`), which pins the behaviour this ruling removes; say so at tick time — a spec change, not a disabled test.
2. `absorb_with_an_upstream_leaves_a_pushed_hunk_staged`: same fixture. A hunk f1 made → stays `AlreadyPushed` (was `BeforeYourBranch`, `:216-219`); f3 → placed. `BeforeYourBranch` stays pinned by a no-remote `branch()` test — name one.
3. `pushed_without_an_upstream_counts_as_pushed` (probe p7): `feature` f1..f3, `refs/remotes/origin/feature` = f3, no `branch.feature.*`. Absorb → `AllPushed`; `a` into f2 → `AlreadyPushed`.
4. `a_teammates_branch_on_mine_marks_my_commits_pushed`: feature f1..f4, upstream at f1, `refs/remotes/origin/teammate` = a child of f3 off `feature`. `a` into f3 → `AlreadyPushed`; f4 → plan; absorb places an f4 hunk, keeps an f3 hunk `AlreadyPushed`. Must go red under both M1 mutants.
5. `a_new_branch_at_mains_tip_has_no_commits_of_its_own` (probe p4): `main` c1..c4, no remote, `switch -c fix`. Absorb → `NoCommitsOfItsOwn { fix }`; `a` into c3 → `NotOnYourBranch`; `main` and `fix` unmoved.
6. `the_only_branch_still_falls_back`: 06c test 1 (only branch `work`) stays green — name it.
7. `a_second_parent_target_is_merge_in_range_on_a_feature_branch`: `side` merged into `feature` above its base; `a` into `side`'s commit → `MergeInRange { merge }`. (`a_into_below_a_merge_is_merge_in_range` covers the fallback.)
8. `beyond_reach_names_the_branch`: 12 commits on `main`, no remote; `a` 11 back → the words above, exactly.
9. `a_into_a_pushed_head_plans_like_amend`: feature f1..f2, upstream at f2, a dependent `feature-b` on f2. `AmendInto { target: f2 }` → a plan (the dependent follows). Same on `main` with `origin/main` at HEAD and a dependent on it.
10. The Minor 6 split: one agreement test per fixture, over 06c's fixtures **and** 1, 3, 4, 5 here: absorb's placeable set == the set `a` plans for, HEAD aside (5).

### Keymap amendment (the conductor applies it before the build)
`docs/design/02-keymap.md` §2.3, the `a` row (:199): "On an older commit of your branch: put the staged changes into it" → "On an older commit of your branch that you haven't pushed: put the staged changes into it; a pushed one is refused in words". Nothing else there changes.

## Acceptance
- [ ] M1: both mutants shown red, with the tests named (06c's fallback tests and ruling test 4).
- [ ] M3: `fix` and `error` cases as tests, red first.
- [ ] Minors 3, 5, 6: each change named with its test.
- [ ] Ruling: tests 1–10 named, each seen red first (1–5, 7–9 against the current code; say which, if any, were already green and why).
- [ ] `R` unchanged: `rewrite_plan::open_runs_from_head_down_to_where_the_branch_left_main` green; `R` does not call `your_commits` (grep).
- [ ] Gates workspace-wide.

## Touches (check each, name how)
- Every `your_commits` / `unpushed_range` caller: `plan`'s absorb, `absorb_plan_map`, `AmendInto` (grep). `base_of` and `default_branch_of` unchanged (R and others call them).
- The put-back callers (M3).
- `Refusal` matches outside graft-ops: the app matches non-exhaustively (`crates/graft/src/update/operate.rs:1116`, review :267) — re-grep `Refusal::` in `crates/graft`; an exhaustive one → `blocked: needs graft`.
- `StaysWhy` matches outside graft-ops: 09 is not built; grep `StaysWhy` in `crates/graft` anyway. One found → `blocked: needs graft`.
- graft's walks for `a` and absorb (grep `fix-into-older`, `absorb-staged`, `amend_into`, `absorb` under `crates/graft/tests`): a fixture whose target is pushed now refuses. Workspace-wide gates will show it; if red, stop and report `blocked: needs graft` with the test name, don't edit.
- Ticket 09: carries the new words (appended there, 2026-09-27).
- Cost: every `a` plan now runs a `--not --remotes` read on every base (before: only on the fallback). Unmeasured on the 50k-ref fixture — say so in NOTES; the milestone review measures it.
- MAP.md (graft-ops) `rewrite` (`:263`, `:271-273`, `:276`) and `absorb` (`:281`): the range rule, the stops, the new refusal and stay, `AlreadyPushed`'s words.
- KNOWN_GAPS rows M7/06c-rule, M7/06c-oracle, M7/a-into-main: the conductor updates them from the report (builders don't read it).
