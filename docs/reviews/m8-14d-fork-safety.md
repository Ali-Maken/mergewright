# Review — m8 14d (fork follows only when safe: carried branches, pushed, base unchanged)

Reviewed: `git diff ec125561^1 ec125561` (merge commit `ec125561` on main). Spec: ADR-0059 §6 (amended 2026-10-04),
ticket `.scratch/m8-polish-and-ship/issues/14d-fork-follows-safety.md`.
Reviewer: one round, own detached worktree at `ec125561`, own `CARGO_TARGET_DIR`, both removed afterwards.

**Counts: 0 Critical · 2 Major · 7 Minor.**
**Ticks: 15 checked, 15 held.**

## What I verified, and how

- **Gates** (workspace-wide): `cargo fmt --check` ok; `cargo clippy --all-targets -- -D warnings` ok;
  `cargo clippy --all-targets --features heavy-tests -- -D warnings` ok; `cargo nextest run --workspace` **5483 passed,
  10 skipped, 0 failed**, 618.8 s (wall 696 s, build included); `cargo test --doc` ok.
- **Load caveat on the time:** load average was 24.5 when the run ended (other agents' cargo on the machine). The 618 s
  is not a measurement of this suite on a quiet laptop and is not compared against PROMPT.md 7.1.
- **Peak memory:** 1.18 GB max RSS for the `cargo nextest` process tree root (`/usr/bin/time -l`); this is cargo/nextest's
  own peak, not the sum of test processes. **`target/`:** 4.9 GB (dev profile, workspace, fresh).
- **5 slowest tests: not measured.** The repo's nextest profile prints no per-test timing for passes and flagged no
  test as SLOW. I did not rerun the 10-minute suite to get them.
- **Not run, on the conductor's instruction (fast tier only):** heavy tier, release perf run, fuzzing, benchmarks.
- **Probes:** five throwaway tests appended to `fork_follows.rs` in my worktree (never committed; deleted with the
  worktree), truth read with git. Results quoted under M1 and m1:
  - P1 `park` at c2, 0 own commits, pushed **without `-u`** (`origin/park` = c2): fates `park: Follows`. After `y`:
    `merge-base --is-ancestor origin/park park` → **false** (diverged).
  - P2 `feature-x` = c2 + x1, pushed at c2 **without `-u`** (`origin/feature-x` = c2): `Follows`. After `y`:
    `origin/feature-x` not an ancestor of `feature-x` → **diverged**.
  - P3 carried `feature-w` pushed without `-u`: `feature-x` stays `Pushed { refs/remotes/origin/feature-w }` (safe).
  - P4 `feature-x` tracking the **local** `feature-a` (`--set-upstream-to feature-a`): stays
    `Pushed { refs/heads/feature-a }`, words `pushed to feature-a`.
  - P5 carried `feature-w` whose upstream holds only c2: `Carries { feature-w, Pushed { origin/feature-w } }` (correct).
- **Read:** the whole of `preview/forks.rs` after the merge, `holds_any` (`preview/rewrite.rs:360`),
  `off_line` (`rewrite/dependents.rs:186`), every test added, the graft diff, MAP, CONTEXT, shot diff.
- **Feature pipeline:** this ticket fixes the 14b/14c review's Majors; Need / Prior art / Journeys were checked in
  `docs/reviews/m8-14bc-fork-follows.md` and the journey (`a`, `y`) is unchanged by 14d. No new screen, no prototype owed.

### Every "any doubt → the fork stays" path (ADR-0059 §6), walked in `Weigh::fate` (`forks.rs:398-445`)

| Doubt | Where | Outcome |
|---|---|---|
| Fork is the default branch / checked out elsewhere | `:406` → `placed` `:449-461` | stays |
| Merge among its own commits | `:409-414` | stays `HasMerge` |
| `at` not a row, or not on the line's first-parent chain | `:415-420` | stays `NotChecked` |
| Its upstream holds any of `onto..F` (`under` + own) | `:425` → `upstream_holding` `:467-483` → `holds_any` | stays `Pushed` |
| A carried branch is default / elsewhere / upstream holds its `onto..W` | `:428` → `carries` `:488-515` | stays `Carries` |
| Any `refs/remotes/*` holds F's oldest own commit | `:433-437` → `remote_holding` `:576-595` | stays `Pushed` |
| Base set changed (drop, move, fold) | `:438` → `base_change` `:523-544` | stays `WouldLose`/`WouldGain` |
| Shares a commit with a follower | `:441` | stays `Nested` |
| Forecast stops below / hole / conflict | `replayed_fate` `:600-643` | stays |
| Any read fails | `?` on every read | the preview fails (never `Follows`) |
| Upstream configured, ref missing | `holds_any` `--ignore-missing` (`rewrite.rs:372`) | holds nothing; the remote half still runs |

Every gate returns before `replayed_fate` (§6.4 holds by construction). The one hole found is M1 (a remote ref of F's
own name that holds `at` or below, with no upstream set).

### Can it lose a commit?

No path found. A follower's own commits are replayed by picks; carried branches move by `update-ref` only when listed in
`inside` (`forks.rs:255-258`), which is exactly the gated `carried` set; anything not listed is not moved. §6.3's check
keeps any fork whose base set would change (`base_change`), and the "fold a later commit in" and "drop the fork point"
cases are covered by tests that read the result with git. The worst outcomes found are divergence from a remote (M1)
and a false reason (m1, m2) — neither loses a commit; the journal keeps every old tip.

## Critical

None.

## Major

### M1 — A remote ref of the fork's own name that holds its fork point (or below) is missed when no upstream is set: the fork follows and diverges (spec gap in ADR-0059 §6.2)
- **Where:** `crates/graft-ops/src/preview/forks.rs:431-437` — the "any remote-tracking ref" half checks only
  `commits.first()` (F's oldest own commit) and is skipped entirely when F has 0 own commits. This is exactly what
  ADR-0059 §6.2 (`docs/adr/0059-…md:84-86`) says: "any `refs/remotes/*` ref holds one of F's own commits (`at..F`)".
  The upstream half (`:425`) checks `onto..F`. The two halves are asymmetric.
- **What happens (probes P1, P2, git truth):** `park` (0 commits) pushed with `git push origin park` (no `-u`) follows
  and leaves `origin/park` off its history; `feature-x` pushed at its fork point without `-u`, then x1 committed, follows
  and diverges from `origin/feature-x`. The same branch with `-u` stays (`a_fork_with_no_commits_whose_upstream_holds_its_fork_point_stays`,
  `a_fork_whose_upstream_holds_only_its_fork_point_stays`). Whether `-u` was typed decides whether he needs a force-push.
- **Why it matters:** ADR-0059 §6's own premise is "any doubt → the fork stays"; a remote branch of the fork's own name
  is the strongest sign it is published. The prior review's M2 was this same class (upstream half). Not data loss:
  the old commits stay on the remote and in the journal.
- **Why it is not simply "check every remote ref for `under`":** `origin/feature-a` (his line, pushed) holds `under`,
  so every fork would stay. The asymmetry is deliberate for foreign refs, wrong for F's own name.
- **Suggested fix (Proposed change to ADR-0059 §6.2, then code):** also count as pushed any `refs/remotes/<remote>/<F's
  short name>` (and each carried branch's) that holds a commit of `onto..F`, upstream set or not — one extra
  `for-each-ref refs/remotes/*/<name>` plus the existing `holds_any`. Tests: P1 and P2 above, after-run refs unchanged.
  Conductor/spec decision, not a builder one.

### M2 — `preview/forks.rs` grew from 467 to 687 lines (PROMPT.md 6.2: files under about 400)
- **Where:** `crates/graft-ops/src/preview/forks.rs` (whole file; `git show ec125561^1:…/forks.rs | wc -l` = 467).
- **Why it matters:** readability is a hard rule; this file now holds candidate collection, the fates loop, the
  section builder, every §6 gate, three git reads and the forecast. The new gate code is cleanly separable.
- **Suggested fix:** split with `codebase-design`: `forks/gates.rs` (`placed`, `upstream_holding`, `carries`,
  `base_change`, `remote_holding`, `line_before`, `upstreams`) beside `forks.rs` (candidates, `Fates`, `forks`,
  `replayed_fate`). No behaviour change; suite count unchanged.

## Minor

- **m1 — A local upstream now reads "pushed to <his branch>" and keeps the fork** (P4). `forks.rs:646-667`
  (`upstreams`) keeps any `%(upstream)`, `refs/heads/*` included; `forks.rs:478-481` now tests it against `under` too,
  and his line branch holds `under`. Before 14d (`at..F` only) the same fork followed. `WhyStays::Pushed`'s own doc
  (`effects.rs:231-233`) says "the remote-tracking ref that holds it". Safe direction, false words. Reachable with
  `git switch -c feature-x --track feature-a` (stacked-branch habit). Fix: count only upstreams under `refs/remotes/`
  in `upstream_holding`; test with P4.
- **m2 — Question: a `FromRebase` rewrite whose suggested drop sits under a fork point reads `would lose <sha>`.**
  `forks.rs:533` drops every `Drop` row from `after`; `rewrite::plan` `FromRebase` marks `params.drops` (commits
  already in `onto`) as `Drop`. If so, the fork stays with "would lose c1" when c1's change is already in the new
  base. Safe direction; words possibly misleading. Not probed — filed as a question (rule 7).
- **m3 — Test gaps.** (a) The remote-ref half for a **carried** branch (P3 path, `forks.rs:433-437` reached via a
  carried branch) has no test. (b) `a_fork_whose_upstream_holds_only_its_fork_point_stays`,
  `…_a_replaced_commit_under_it_stays` and `a_fork_pushed_without_an_upstream_stays` assert the fate only, never run
  `y` and read refs with git (the ticket asked only for the fate; the file header promises git truth).
  (c) `a_failed_gate_wins_over_the_forecast` does not assert its control (without the worktree, the same fixture is
  `WouldConflict`); the builder's red-run note is the only evidence. Fix: add (a); add the after-run `rev-parse`
  to (b); add the control half to (c).
- **m4 — Names `Weigh` and `Weighed` differ by two letters** (`forks.rs:297`, `:312`). One is the shared reads, the
  other one fork under weighing; a reader confuses them at call sites like `weigh.fate(&env, &weighed, …)` (`:143`).
  Each has a doc comment, hence Minor. Suggest `Weighed` → `ForkInHand` or `Weighing`.
- **m5 — `remote_holding` cost is unmeasured.** `forks.rs:576-595` runs one `for-each-ref --contains` per fork that
  reaches it (≤ 8), each walking from every remote ref. On a repo with thousands of remote branches this can take
  seconds per fork on the preview job. Off the UI thread, so no freeze, but preview latency. Ties to the 14bc review's
  m8 (fork forecast cost unmeasured). Fix: measure on the large fixture; if slow, one call with all oldest commits.
- **m6 — `tests/it/fork_follows.rs` is 1190 lines.** Over PROMPT.md 6.2's ~400; precedent rows exist in KNOWN_GAPS
  (M6/07). Split the 14d block (`// ── 14d` onward, ~550 lines) into `fork_safety.rs`.
- **m7 — Question: a branch on a follower's chain past `LEFT_BEHIND_CAP` is neither gated nor moved.**
  `rewrite/dependents.rs:204` names only the first 8 off-line branches; `Fates::carried` (`forks.rs:231-240`) draws
  only from those. Such a branch stays on the old commits (safe — not moved, so a worktree on it is untouched) while its
  fork follows, splitting the two; the preview counts it only in "+N more". Ties to the 14bc review's m3 (`PastCap`
  never constructed). Is staying-without-a-row acceptable, or should any uncounted branch on `at..F` keep F?

## Ticks (15 checked, 15 held)

Each named test was found (`grep fn <name>()`: one each; `a_fork_kept_by_a_safety_gate_is_muted` two, box and editor,
as the tick says), read against its tick's wording, and passed in the full run.

- M1 linked worktree (`a_fork_carrying_a_branch_checked_out_elsewhere_stays`): fate, words, no `RefMove`, refs
  unchanged, `status --porcelain` empty in the worktree, line moved — held.
- M1 fork itself elsewhere (`a_fork_checked_out_elsewhere_stays_and_its_worktree_is_untouched`) — held.
- M1 carried pushed (`a_fork_carrying_a_pushed_branch_stays`) — held.
- M1 control (`a_fork_carrying_a_clean_branch_follows_with_it`): `feature-w == feature-x~1`, ancestry — held.
- M1 doc grep: `effects.rs` `Elsewhere` doc has no "refuse"; `dependents.rs:64-67` is Graft's own refusal — held.
- M2 park (`a_fork_with_no_commits_whose_upstream_holds_its_fork_point_stays`): fate + `park` unchanged after `y` — held.
- M2 without upstream (`a_fork_pushed_without_an_upstream_stays`) — held (fate only, as ticked).
- M2 control (`a_fork_whose_upstream_holds_only_what_is_under_onto_follows`) — held.
- M3 drop c2 (`dropping_the_fork_point_keeps_the_fork`): fate, words, ref unchanged, `show feature-x:c2.txt` — held.
- M3 fold control (`folding_the_fork_point_into_the_commit_under_it_follows_onto_the_fold`): `feature-x~1 ==
  feature-a~2`, tree == old c2's, x1 diff same — held.
- M3 amend the fork point (`amending_the_fork_point_follows_onto_the_amended_commit`): `after_row` == fixup row,
  hand-run `rebase -i` trees equal — held.
- Order (`a_failed_gate_wins_over_the_forecast`): held as worded (no spawn count, said so).
- Words table (`the_new_reasons_read_as_words`) — held.
- Row tone, box and editor (`a_fork_kept_by_a_safety_gate_is_muted` ×2): muted, no `!` — held.
- Shot retake (`the_rewrite_editor_names_each_fork`): shot shows `feature-b follows` and `feature-c stays on 86eea6e ·
  would lose 86eea6e`, muted; the `!` row still in `fork-follows-preview-187x44.txt:35` — held. Colour **unverified**.
- MAP.md (`WhyStays` three variants, `after_row` group note) and CONTEXT.md **Fork** entry — present (doc ticks, no
  test, counted with the doc-grep tick above).

## Checked, no finding

- No `unwrap`/`expect` added outside tests; `Weighed::of` indexes `candidates[index]` under the loop's own bound.
- Git work: every new read runs inside `preview::plan_as_run` (the preview job), not the UI thread.
- Crate rules: the graft change is one exhaustive match arm in `update/preview/line_rows.rs:136-148`; no new
  dependency.
- Terminal text: new words are a branch shorthand (git ref rules forbid control bytes) and `operation::short`; the
  worktree path route (`Elsewhere`) is unchanged from 14b.
