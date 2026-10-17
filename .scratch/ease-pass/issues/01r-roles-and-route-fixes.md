# roles-and-route fixes — "built on" and "merged" say only what git can prove

Status: done
Blocked by: 01-roles-and-route (merged `862d40d2`)
Build with: Fable (signature). Why: it rewrites the rules and changes the public types (`Who`, `Merged`,
`choose_base`) that the right pane and every lane colour read. A wrong rule can't be judged from the diff or a green
test: four wrong rules passed every tick in 01. It is brownfield, but the 01 scout and the review already pin the code
facts, so the planner sees no need for a new scout. The conductor confirms.
Needs: none. It calls existing reads `count_between`, `commits_between`, `commits_from`, `compare_span` (graft-git
`history_reads`) and adds one graft-git read, `entry_into` (below).

## Read first
- `docs/reviews/ease-pass-01.md`: Majors 1–4, Minors 1, 2, 5, 6.
- `docs/design-notes/ease-pass.md`: Interfaces (graft-ops `branch_status`, graft-git), Hard-to-reverse choice 1.
- `.scratch/ease-pass/issues/01-roles-and-route.md`: "Signatures", "Meaning", and the builder's "Decisions not in the spec".
- `crates/graft-ops/MAP.md` `## branch_status`; `crates/graft-git/MAP.md` `## history_reads`.
- CONTEXT.md **Dependent branch**, **Integration branch**.

## The rules, corrected
Root cause (review): between two ordinary branches, merge-base distance can't tell a base from a sibling, or from a
child cut from an older commit. Git doesn't record who was cut from whom. Prior art: lazygit compares only against its
configured main branches. The stacked-branches study §4 ("parent inferred from the graph, not recorded") and ADR-0032
reject recorded parents. A reflog clue fails too: a sibling's own reflog also starts at the fork point.

So the rules use only what git proves: kind (trunk and integration, by ADR-0001 precedence), containment, and the
first-parent line. **"Merged" must be proven; "not merged" is the safe default.**

**R1. Base choice (fixes Major 1).** `choose_base(branch, measured, at_most)`. A candidate `c` is eligible when:
- `c.name != branch`, and `branch_only > 0`;
- `c.kind <= at_most`;
- `c.kind` is `Trunk` or `Integration`, **or** `base_only == 0`. An ordinary branch is a base only when all of it sits
  under the branch: the CONTEXT **Dependent branch** relation.

Among the eligible: minimum `branch_only`, then kind, then name.

`route` passes `at_most` = the branch's own kind at step 1, then the kind of each base it picks. An integration branch
is never built on an ordinary branch; the trunk is built on nothing. The walked-tip skip stays (review: deviation
accepted).

A chosen base that shares no history with the walker (`compare_span` says unrelated) is dropped and the next is chosen.
Otherwise an orphan like `gh-pages` reads "development has 40 000 new commits".

**R2. Footing.** Where a non-trunk branch `B` stands, decided before its route. Take the first case that applies:
1. **Held by a long-lived line (fixes Major 2).** Holders are the `Trunk`/`Integration` candidates with
   `branch_only == 0`, equal tips included. The first by **kind** (trunk, then integration), then fewest `base_only`,
   then name, decides:
   - equal tip, or `entry_into(B.tip, C.tip)` has `parents[0] == B.tip` → **on `C`'s line**. `B` has no commits of its
     own: it was just made, was left at an older commit, or was fast-forwarded into `C`.
   - otherwise → **merged into `C`**. The entry is the merge that brought `B` in.
2. **Merged into an ordinary branch (fixes Major 3).** Only when case 1 found no holder. An `Other` holder `C` counts
   only if its entry `E` is a merge and `E.parents[0]` is an ancestor of `B.tip`. That means `C` sat under `B` (B was
   built on C's tip) when it took `B` in. A teammate who cut from an older commit of `B` and then synced has
   `E.parents[0]` = his own commit, so he doesn't count. Among the holders that count: the one whose tip every other
   counted holder holds, then fewest `base_only`, then name.
3. **Own.** None of the above.

**R3. The four answers.**

| Footing | `route` | `Merged` | `Behind` | `Who` |
|---|---|---|---|---|
| trunk | `[]` | `MainLine` | vs its upstream (today's) | `Everyone { last }` |
| on `C`'s line | `[C]` ++ `route(C)` | `NothingToMerge` | vs `C`: `C`'s `base_only` | `NoOwnCommits` |
| merged into `C` | R1 route | `Into(C.name)` | vs `route[0]` (today's) | authors of `commits_between(exclude E.parents[0], include B.tip)`. `E` is the entry of the **nearest** case-1 holder (fewest `base_only`), or of `C` in case 2. Fixes Major 4. If the nearest holder has `B` on its line, `NoOwnCommits`. |
| own | R1 route | `NotYet { into: route.first() name }` | vs `route[0]`; empty route → `NoBase` | vs `route[0]` (today's). Empty route (orphan) → `commits_from(B.tip, WHO_CAP)`: all of it is its own. |

- The route carries `Apart` for `route[0]`, so there is no second lookup and no silent fallback (Minor 2). The private
  `route_measured` returns `Vec<(Candidate, Apart)>`.
- `MainLine` is for the trunk only (Minor 6).
- `STACK_CAP` goes: `entry_into` is uncapped, honours `stop`, and leaves no "too old" state (Minor 5).

**Known limits.** Correct by these rules and kept. They go in NOTES for B, and the conductor adds them to KNOWN_GAPS.
- An ordinary base that moved on after `B` was cut no longer qualifies. `B` then reads as built on its integration
  branch or the trunk: true, but it doesn't name the teammate's branch. This is the planner's open question to the
  user.
- A backup ref left inside `B`'s own line reads as `B`'s base.
- A branch fast-forwarded into integration reads "nothing to merge".
- Squash and rebase merges read `NotYet`. The mock shows his team merges with merge commits (`Merged PR 16021`, 4
  commits inside; Q13), so this isn't his case.

## Signatures
graft-git `history_reads` (`compare.rs`, beside `commits_from`):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineEntry { pub commit: Oid, pub parents: Vec<Oid> }

/// Where `onto` entered `line`'s first-parent chain: the oldest commit on that chain with `onto` as an ancestor.
/// `None` when `line` doesn't hold `onto`, or `line == onto`. One `rev-list --ancestry-path --parents ^onto line` read,
/// then follow `parents[0]` from `line` while inside that set. Uncapped, honours `stop`.
pub fn entry_into(repo: &RepoHandle, onto: Oid, line: Oid, stop: &dyn Stop) -> Result<Option<LineEntry>, RunError>;
```
Don't combine `--first-parent` with `--ancestry-path`. Whether that pair finds a merge that came in through another
merge (sara → wallet → main) is unverified. The transitive fixture below must pass whatever command is used.

graft-ops `branch_status` (changed items only; everything else stays as 01 built it):
```rust
pub fn choose_base<'a>(branch: &str, measured: &'a [(Candidate, Apart)], at_most: CandidateKind) -> Option<&'a Candidate>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Merged { MainLine, Into(String), NotYet { into: Option<String> }, NothingToMerge }      // full ref names
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Who {
    Everyone { last: Timestamp },                                          // the trunk
    Authors { first: Vec<Vec<u8>>, others: usize, last: Timestamp },       // grouping and order as today
    NoOwnCommits,
}
```
`route` and `branch_status` keep their signatures. `route` now applies R1 and R2: an on-line branch gets `[C, …]`.

## Scope
In:
- graft-git `history_reads/compare.rs`: `entry_into`, its tests, a MAP line.
- graft-ops `branch_status.rs`, `branch_status/status.rs`, `branch_status/tests.rs`, `tests/status.rs`, MAP
  `## branch_status`.
- Doc comments rewritten to these rules.

Out:
- graft-graph `role`: unchanged, `Route::new` takes whatever chain it is given.
- The pane and its words for the new variants: 09 owns them.
- Recorded parents and reflog clues. Rejected above; reopening that is a question for the user, not a build choice.

## Touches
- Callers: none yet. Grep `branch_status`, `Who`, `Merged` in `crates/graft` and report what you find. 09 is unbuilt;
  its spec is updated for these types.
- Spec updates, done by the planner: 09 (pane words, fixture condition), 14 (fixtures), the note's Interfaces and its
  Hard-to-reverse choice 1. 07 and 13 take a `Route`, not these rules, so they don't change.
- Name clashes for `LineEntry` and `entry_into` in graft-git: grep and report.
- Cost: `entry_into` runs once for the deciding case-1 holder, once more for the nearest holder if that is a different
  one, and on `Other` holders only when case 1 found none. Put the spawn count per `branch_status` call in NOTES, for
  B's cache.
- No screen, capture, key, journal or undo change.

## Acceptance
Fixtures are fast-tier scratch repos. Every **red-first** test is committed failing against `862d40d2` before the fix,
with that commit's hash in NOTES, then made green. Each tick names its test.

Red first:
- [x] **M1 tie** `sibling_is_not_built_on`. Repo: `main m1 m2`; `wallet-release w1 w2` off m2; `sara/fix-toc s1 s2` off
  w2; `ali/x t1` off w2; then `w3` on wallet. `route(sara)` never contains `ali/x`. With wallet `Other` it is `[main]`;
  with wallet `Integration`, `[wallet-release, main]`.
- [x] **M1 step 2** `a_bases_base_is_never_its_sibling`. Same repo, wallet `Integration`: `route(sara)[1]` is `main`.
- [x] **M1 reverse** `a_moved_base_is_not_built_on_its_dependent`. Wallet `Other`, with w3: `route(wallet-release)` is
  `[main]`.
- [x] **M1 child** `a_branch_cut_from_an_older_commit_is_not_the_base`. Repo `main ← sara s1 s2`, plus `bob/x x1` cut
  from sara@s1 and not merged: `route(sara)` is `[main]`.
- [x] **M2** `merged_into_main_not_into_a_later_branch`. `sara s1` merged `--no-ff` into main; `bob/y y1` cut from main
  right after; then main gets m2 m3 m4: `merged` is `Into("refs/heads/main")`.
- [x] **M3** `a_teammate_syncing_does_not_merge_the_base`. Repo `main ← sara`; `bob/x x1` cut from sara@s1; sara adds
  s2; bob runs `git merge sara`: sara's `merged` is `NotYet { into: Some("refs/heads/main") }`.
- [x] **M4 merged** `who_of_a_merged_branch_is_its_own_authors`. Replaces `who_of_a_branch_with_no_base`; NOTES names
  the deleted expectation. sara (author Sara) is merged into main, where Alice and Bea have commits: `who` is
  `Authors { first: ["Sara"], others: 0, .. }`.
- [x] **M4 via integration** `who_ignores_the_integration_branchs_own_work`. sara is merged into `wallet-release`
  (`Integration`, Wendy's w1 w2), and main doesn't hold it: `who` names only Sara.
- [x] **M4 fresh** `a_fresh_branch_is_on_its_line`. `foo` is made at main@m2, then main gets m3 m4. Expect: `route` =
  `[main]`, `behind` = `Behind { base: main, count: 2 }`, `merged` = `NothingToMerge`, `who` = `NoOwnCommits`. At equal
  tips, `behind` = `UpToDate { base: main }`.
- [x] **Orphan** `an_unrelated_branch_has_no_base`. `gh-pages` shares no history: `route` = `[]`, `behind` = `NoBase`,
  `merged` = `NotYet { into: None }`.

Regression (green before and after):
- [x] `a_dependent_on_an_unmoved_branch_is_built_on_it`. Wallet `Other`, no w3: `route(sara)` =
  `[wallet-release, main]`.
- [x] `merged_into_an_unmoved_ordinary_base`. Wallet `Other` and unmoved, and it merges sara `--no-ff`:
  `Into(wallet-release)`.
- [x] The 01 tests pass. Any whose expectation changes is listed in NOTES with the rule that changed it.
  Verified by: the whole `branch_status::` set (37 tests, all green) inside the workspace `cargo nextest run`
  (6515/6515); changed expectations in NOTES.

Reads and minors:
- [x] `entry_into` (graft-git `tests/it/entry_into.rs`):
  - a direct `--no-ff` merge → that merge; `a_direct_merge_is_the_entry`
  - a fast-forward → the commit whose `parents[0] == onto`; `a_fast_forward_enters_at_the_commit_on_top`
  - transitive (sara → wallet → main, both `--no-ff`) → main's merge of wallet;
    `a_transitive_merge_enters_at_the_lines_own_merge`
  - `line == onto` → `None`; `none_when_the_line_is_the_commit_or_does_not_hold_it`
  - `line` not holding `onto` → `None`; same test (diverged, and line an ancestor of onto)
  - a stopped `Stop` → `Err(RunError::Stopped)`. `entry_into_honours_stop`
- [x] `choose_base` proptest updated: the result is never an `Other` with `base_only > 0`, never above `at_most`, and
  otherwise has the minimum `branch_only`. `choose_base_picks_the_nearest` (now also draws `at_most`).
- [x] **Minor 1** `stop_mid_walk`: a `Stop` that trips after N checks (N = 1, 3, 10) makes `route` and `branch_status`
  return `Err(RunError::Stopped)`. A large N lets both complete.
- [x] MAP.md: `## branch_status`, one line per rule; `## history_reads`, `entry_into` and `LineEntry`.
  No test: a doc. Checked by reading both sections after the last edit; no line over 240 bytes in either section.

## NOTES (builder, 2026-10-17)

**Commits.** `21db35ee` entry_into (graft-git). `9297164b` **red commit**: the red-first tests, with the types moved to
the 01r shapes but the rules still `862d40d2`'s (`choose_base`'s `at_most` unused there). Then the fix commit.
Observed red at `9297164b` (nextest, 11 failed of 35 `branch_status::` tests: the 10 red-first plus the proptest):
- `sibling_is_not_built_on`: left `["refs/heads/ali/x", "refs/heads/wallet-release", "refs/heads/main"]`, right `[main]`
- `a_bases_base_is_never_its_sibling`: left `Some("refs/heads/ali/x")`, right `Some("refs/heads/main")`
- `a_moved_base_is_not_built_on_its_dependent`: left `[ali/x, sara/fix-toc, main]`, right `[main]`
- `a_branch_cut_from_an_older_commit_is_not_the_base`: left `[bob/x, main]`, right `[main]`
- `merged_into_main_not_into_a_later_branch`: left `Into("refs/heads/bob/y")`
- `a_teammate_syncing_does_not_merge_the_base`: left `Into("refs/heads/bob/x")`
- `who_of_a_merged_branch_is_its_own_authors`: left `(["Bea Brandt", "Alice Ames"], ..)`, right `(["Bea Brandt"], ..)`
- `who_ignores_the_integration_branchs_own_work`: left `(["Sara Sahin", "Wendy Wu"], ..)`
- `a_fresh_branch_is_on_its_line`: route `[]`, right `[main]`
- `an_unrelated_branch_has_no_base`: route `[main]`, right `[]`
The two regression tests and `stop_mid_walk` were green there, as the spec expects.

**Changed 01 expectations.**
- `who_of_a_branch_with_no_base` deleted; it expected `first == ["Bea Brandt", "Alice Ames"]` (Alice only made main's
  `m1`). Replaced by `who_of_a_merged_branch_is_its_own_authors` (R3 merged row).
- `route_does_not_cycle`: 01 had `a`, `a-twin`, `b` as `Other`, expecting `[b]`. Under R1 an `Other` with commits of
  its own is never a base, so that fixture's route is `[]`. Kinds now `Integration` so the cycle is still possible
  (each twin sits on the other's line, R2); expectation `[a-twin, b]`.
- `merged_into_its_base_after_a_merge_commit`, `a_branch_stacked_on_top_is_not_a_merge`: `NotYet { into }` now
  `Some(..)` (type change only). `who_*` tests read `Who::Authors` / `Who::Everyone` (type change only).
- Route tests moved from `tests.rs` to `tests/route.rs` (tests.rs went over 400 lines after fmt), unchanged.

**Decisions not in the spec.**
- R2 case 2: an ordinary holder whose entry's first parent **equals** B's tip does not count (it was built straight on
  B, then merged something else). The spec says "an ancestor of B.tip"; B's tip is its own ancestor in git's sense,
  and counting it would call B merged. Test: `a_branch_on_the_tip_that_merges_something_else_has_not_merged_it`;
  mutation-checked (removing the guard turns that test red; observed).
- R1 unrelated check runs only for a pick with `base_only > 0`: a candidate with none of its own lies under the
  walker, so it shares history. Costs `compare_span`'s two spawns (merge-base + count) per such pick.
- Extra test `who_of_a_branch_merged_through_integration_reads_the_nearest_entry` (sara → wallet → main): `merged` is
  `Into(main)`, `Who` is Sara only, read from wallet's entry. Not mutation-checked; reasoned only.
- `entry_into` reports a walk past the runner's 64 MiB output cap as `Unparsable`, never a partial answer. A holder the
  counts say holds B with no entry is also `Unparsable` (git contradicting itself), not a silent fallback.

**Spawns per `branch_status` call (non-trunk), for B's cache.** N = candidates at a tip other than B's, d = route
length. Upstream: 2 if set. Step 1: 2N counts; footing case 1: 1 `entry_into` (0 at an equal tip), +1 when the
nearest long-lived holder is another branch and B is merged; case 2 (only when case 1 found no holder): per `Other`
holder 1 `entry_into` + 1 count when its entry is a merge, + k(k−1) counts among the k that count. Each route step
after the first: 2·(N − walked) counts + 0–1 `entry_into`. Each R1 pick with `base_only > 0`: 2 (`compare_span`).
`Who`: 1 rev-list, 0 for `NoOwnCommits`. Typical stacked branch, 4 candidates, d = 2: about 8 + 6 + 2–4 + 1 ≈ 17–20.
Not measured; counted from the code.

**Known limits** (correct by these rules; for the conductor to add to KNOWN_GAPS, as the spec says):
- An ordinary base that moved on after B was cut no longer qualifies; B reads as built on its integration branch or
  the trunk (true, but it doesn't name the teammate's branch). Q15, option (a), as specced.
- A backup ref left inside B's own line reads as B's base.
- A branch fast-forwarded into integration reads "nothing to merge".
- Squash and rebase merges read `NotYet`.
- A teammate cut from an older commit of B **with no commits of his own**, who then merges B `--no-ff`, counts as
  merging B (his entry's first parent is that older B commit). Found while writing R2 case 2; reasoned, untested.

**Touches.** `grep -rn "branch_status|choose_base|BranchStatus" crates --include='*.rs'` outside
`graft-ops/src/branch_status*`: only `graft-ops/src/lib.rs` (the `pub mod`) and a doc comment in
`graft-graph/src/role.rs:7`. `Who` in `crates/graft/src/update/place/rows.rs:26` is blame's own `Who`, not this one.
No caller in `crates/graft`. `LineEntry` / `entry_into`: no clash in the workspace before this ticket (grep).
graft-git `tests/it/main.rs` gained one `mod entry_into;` line. No screen, capture, key, journal or undo change.
`walk_unheld` in `compare.rs` now shares a `parse_parents` helper with `entry_into`; its tests stay green.

**Scout.** None new: the 01 scout and the review pinned the code facts, and the conductor's brief named them. I read
the files I changed directly.

**Compaction.** None.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium (~45k): about 150 lines of rules, 13 fixtures, one graft-git read.
