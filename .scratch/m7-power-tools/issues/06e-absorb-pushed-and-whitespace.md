# 06e absorb-pushed-and-whitespace — review 06c's M1, M2, M3, Minors 2, 3, 5, 6 and both Questions

Status: done
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
- [x] M1: both mutants shown red, with the tests named (06c's fallback tests and ruling test 4). Mutant A (only remote *tips* pushed: `rev-list` output minus `for-each-ref refs/remotes/` tips, no `--not --remotes`) → red: `absorb_pushed::a_teammates_branch_on_mine_marks_my_commits_pushed` (test 4), `absorb_pushed::a_teammates_branch_on_main_marks_my_commits_pushed` (new, the fallback's probe p3), `absorb_pushed::absorb_and_a_agree_under_a_teammates_branch`. Mutant B (`--remotes=*/main`) → red: those three plus tests 1, 2, 3 and `a_tracked_branch_pushed_to_its_tip_is_all_pushed`, `absorb_and_a_agree_when_pushed_without_an_upstream` (10 tests red). **06c's own fallback tests kill neither**: they put `origin/main` on the first-parent chain, where the walk stops at the tip itself (A survives) and the ref is `*/main` (B survives) — hence the new fallback test p3. Both mutants reverted (`git checkout` of `unpushed.rs`, then the diff checked empty).
- [x] M3: `fix` and `error` cases as tests, red first: `absorb_config::a_stay_with_trailing_spaces_stays_staged_under_apply_whitespace_fix` (red: index `old.txt` blob `6012f1a…` ≠ staged `f731907…`) and `…_under_apply_whitespace_error` (red: the run failed at the put-back). Fix at the cause: `apply/set_aside/put_back.rs` `apply` pins `apply.whitespace=nowarn` via `GIT_CONFIG_COUNT/KEY/VALUE` (`AS_THEY_WERE`), as `CLEANUP` does. Both green.
- [x] Minors 3, 5, 6: each change named with its test. Minor 3: `AlreadyPushed.by` dropped (`absorb_unpushed::a_into_below_a_remote_ref_is_already_pushed`, `absorb_pushed::a_into_a_pushed_commit_with_an_upstream_is_already_pushed`). Minor 5: `each_target_last_changed_its_lines(repo, range, …)` takes the range; `absorb_oracle` passes `main..feature`; new row `absorb_printed::the_oracle_agrees_on_the_fallback_range` (`HEAD~10..HEAD`; green from the start — it pins, it did not find a bug; not mutation-tested). Minor 6: `absorb_and_a_into_agree_on_the_range` split into `absorb_unpushed::absorb_and_a_agree_on_main_without_a_remote`, `…_on_main_pushed_part_way`, `…_on_main_past_ten_back`, `…_above_a_merge`, each asserting its count.
- [x] Ruling: tests 1–10 named, each seen red first. Red against the old code (API stubbed to compile, old behaviour): 1 `absorb_pushed::a_into_a_pushed_commit_with_an_upstream_is_already_pushed`; 2 `absorb_pushed::absorb_with_an_upstream_leaves_a_pushed_hunk_staged`; 3 `absorb_pushed::pushed_without_an_upstream_counts_as_pushed`; 4 `absorb_pushed::a_teammates_branch_on_mine_marks_my_commits_pushed`; 5 `absorb_pushed::a_new_branch_at_mains_tip_has_no_commits_of_its_own`; 8 `absorb_unpushed::beyond_reach_names_the_branch` (renamed from `a_into_eleven_back_is_beyond_reach`, new words); 9 `absorb_pushed::a_into_a_pushed_head_plans_like_amend`; 10 `absorb_pushed::absorb_and_a_agree_with_an_upstream`, `…_when_pushed_without_an_upstream`, `…_under_a_teammates_branch`, `…_on_a_new_branch_at_mains_tip` (all four red) plus 06c's four (split, Minor 6; green, as they should be). Also red first: `absorb_unpushed::absorb_on_main_stops_at_a_remote_ref` (c2's stay now `AlreadyPushed`, stay rule 1). **Already green:** 7 `absorb_pushed::a_second_parent_target_is_merge_in_range_on_a_feature_branch` — the old code reached `refuse_merges` over `s1^..HEAD`, which held the merge, so the same refusal came from a different place; 6 `absorb_unpushed::absorb_on_the_only_branch_places_into_its_commit` (stays green, as asked). Test 1 **replaces** `a_into_with_an_upstream_is_unchanged` (a spec change, not a disabled test; comment left at its place). `BeforeYourBranch` stays pinned by `absorb_unpushed::absorb_without_a_remote_keeps_before_your_branch` (no remote, `branch()`), which replaces `absorb_with_an_upstream_keeps_its_range`. Detached HEAD: `absorb_pushed::a_detached_head_never_reaches_the_fallback` → `NotOnABranch` for both keys (green from the start: `head_of` refuses first).
- [x] `R` unchanged: `rewrite_plan::open_runs_from_head_down_to_where_the_branch_left_main` green (workspace run); `R` does not call `your_commits` — grep: callers are `rewrite/absorb.rs:39` (`absorb_base`) and `rewrite/plan.rs` `refuse_out_of_reach` (called from `amend_into` only).
- [x] Gates workspace-wide: `cargo fmt --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok; `cargo nextest run --workspace` 4080/4080 (1 skipped); `cargo test --doc --workspace` ok.

## NOTES (builder, 2026-09-27)
- No compaction.
- **Deviation 1 (for true words), flagged for the planner:** Q1 read literally gives a tracked branch pushed to its tip (upstream at `HEAD`, so no base below `HEAD`) `NotItsOwn` → `feature has no commits of its own yet`, which is false. Built: with no base below `HEAD` and not the default/only branch, `HEAD` itself still gets the `Pushed` check first (decision 1's order), so it says `AllPushed`. Test: `absorb_pushed::a_tracked_branch_pushed_to_its_tip_is_all_pushed`.
- **Deviation 2, stay rule 1 approximated by the walk's stop.** Blame is range-restricted (`base..HEAD`), so a line below the range is blamed to the boundary, not the commit that last changed it. The stay reason comes from the stop: `Pushed` → `AlreadyPushed` (exact: everything below a pushed commit is pushed); `Base` → `BeforeYourBranch`; else `OutsideYourCommits`. Where it differs from the rule: a line below an **unpushed** base, last changed by a commit a remote holds (e.g. `origin/main` behind local `main`, the branch cut from the unpushed part) says `BeforeYourBranch` where the rule says `AlreadyPushed`. Both are true. Exact would need an unrestricted blame per file. One remote read per absorb, as asked.
- **Keymap text applied verbatim** (`docs/design/02-keymap.md:199`). The substitution leaves `…; a pushed one is refused in words, previewed, \`y\`.` — it reads as if the refusal is previewed. Suggest the planner reorder: `…put the staged changes into it, previewed, \`y\`; a pushed one is refused in words.`
- **Behaviour change beyond the tests named:** with a base, a merge above the base now ends absorb's range (was: hunks below it were placed, then the whole absorb refused `MergeInRange` at `refuse_merges`). Now those hunks stay `OutsideYourCommits` and the rest absorbs. This is the ruling ("absorb places only into the taken commits"); no test asserted the old refusal (full run green).
- M3's pin is in the shared put-back, so every set-aside operation (not only absorb) now puts edits back ignoring `apply.whitespace`. Intended: a put-back restores edits as he left them. The other sites KNOWN_GAPS M7/06c-apply names (`operation/stage.rs`, `preview/stash.rs`) were **not** probed — out of scope.
- Cost: every `a` plan (not `HEAD`) and every absorb now runs `rev-list <chain> --not --remotes` on every base, plus one `for-each-ref --count=2 refs/heads/` on the fallback path, plus one `rev-list --parents` for the base's parents. **Unmeasured** on the 50k-ref fixture; the milestone review measures it.
- Planner's open question (a reaching his own pushed feature branch with a force-push warning) not built, per the ruling.
- Scout: none (ticket says none; the builder read `rewrite/{unpushed,plan,absorb}.rs`, `absorb/{mod,judge}.rs`, `apply/set_aside/put_back.rs` directly — the edits needed exact lines).

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
