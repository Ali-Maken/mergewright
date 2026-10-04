# ADR-0059 — A branch forked off a rewritten commit follows when its replay is clean

Date: 2026-10-02
Status: **Accepted**, 2026-10-02 (field notes :89, item 5: "go ahead with your recommendation" — the branch moves too
when it moves clean, shown in the preview first). Design note `m8-polish-and-ship.md`, "Where I'd differ", 2c.
**Amended 2026-10-04** (§6; review `docs/reviews/m8-14bc-fork-follows.md` M1–M3, m9): what makes a fork not clean; §6.2 again 2026-10-04 (review m8-14d M1, m1).
Amends: ADR-0047 §1 (todo vocabulary) and §4 (scope of "never a fork").

## Context
- His words (field notes :86, 2026-10-02): "mid-chain branch left behind after `a` → fine until M8". No line of his
  says what it should do instead (grep `fork|mid-chain` in field notes: :86 only).
- Today a branch F forked off a commit inside the replayed range stays on the old commit. The preview names it
  (`left behind: F stays on the old <sha7>`, `rewrite::left_behind`, m7/37). He then rebases F by hand, or forgets.
- `update-ref` moves only refs on commits **inside** the range (ADR-0047 Context; `stacked-branches.md` §1.1). F's own
  commits are outside it, so ADR-0047's todo (`pick reword edit squash fixup drop update-ref`) cannot move F.
- Prior art (`stacked-branches.md`): jj rebases every descendant of a rewritten commit, 0 extra steps (§1.4 :89);
  git-branchless `restack` moves the abandoned descendants (§1.3 :72) and `sync` skips stacks that would conflict
  (:73); Graphite `sync` skips branches that would conflict (§1.2 :56-57); Sapling `restack` (§1.8).
  The study's §4 :210 rejects skip-on-conflict **for the line he asked to rewrite** (a stop there belongs on M6's
  conflict screen). A fork is not what he asked to rewrite: skipping it, in words, is the branchless/Graphite answer.

## Decision

### 1. What a fork is
A local branch F such that: F is not in the line; F is **not a dependent** (ADR-0047 §4: F's first-parent chain does
not hold the rewritten branch's tip — see §5); F's tip is not in the replayed range; `at = merge-base(F, old top)`
is a replaced commit (the existing `LeftBehind.on`); no merge lies in `at..F`. A branch **at** a replaced commit with
no commits of its own is a fork with 0 commits and follows (git's own `--update-refs` moves it, §1.1) — except one at
the old top, which stays a backup as ADR-0047 §4's dependents rule has it.

So a fork leaves the line **below his branch's tip** (his own branch's middle). A branch leaving a dependent's middle
is a dependent, not a fork (§5).

### 2. Fate, decided in the preview and run as previewed
- **Follows** when all hold: the chained forecast through `at`'s row and then F's own commits is `Clean`; F is not
  the default branch; F is not checked out in another worktree; F is not pushed (§6.2); every branch F carries passes
  the same three gates (§6.1); F's base is unchanged by the edit (§6.3); F does not branch off another fork's own
  commits (that fork follows; F stays `Nested` — corrected 2026-10-04, review m9: this clause read "no other fork
  leaves `at..F`"); F is among the first `LEFT_BEHIND_CAP` (8) named.
- **Stays** otherwise, with the reason in words (`would conflict in <path>`, `the rewrite stops below it`, `has a
  merge`, `pushed to <upstream>`, `checked out in <path>`, `forked from <fork>, which moves`, `not checked: <why>`,
  `the default branch`, `past the first 8`, and §6's). A forecast hole is never read as clean (m5 design note hard
  choice 5, MAP graft-ops `preview` `ConflictForecast`).
- `plan_as_run` fixes the fates; the op that `y` runs carries the followers. `check`'s `Moved` covers their tips.

### 3. Engine: one rebase, `label`/`reset` sections (amends ADR-0047 §1)
- The todo may also hold `label` and `reset`, **only** to replay a follower, inline at its fork point:
  `<row that makes at'>` · `label graft-fork-<n>` · F's `pick`s · `update-ref` for F (and any branch on `at..F`'s
  first-parent chain) · `reset graft-fork-<n>` · the line continues. The rebase still ends on the line's top.
- Still one `git rebase -i`: git's stops, abort, kill -9 state and M6's resolver are unchanged.
- `rewrite::progress` counts neither `label`, `reset` nor a follower's `update-ref` as a step or a line boundary
  (it already excludes the line's `update-ref`s from `step`, MAP `progress`). A stop inside a section names the fork.
- `plan.json` gains `forks`; a plan without it (older Graft) reads as none.
- **To verify against git, not assume** (the builder, oracle tests): `label`/`reset` run in a plain `rebase -i`
  without `--rebase-merges`; refs named by `update-ref` are written only when the rebase finishes, so an abort
  inside or after a section leaves F where it was.

### 4. Journal
- Followers join the operation's tips (`rewrite::tips`): `refs_before`/`refs_after` hold them; one `u` restores every
  moved ref (ADR-0047 §2 unchanged in kind).

### 5. Not changed
- ADR-0047 §4's refusal of two lines built on the **tip** (`feature-b and feature-c are both built on yours`).
- That refusal **includes a branch started from a dependent's commit** (say `feature-x` off `feature-b~1`, where
  `feature-b` is built on his branch): its first-parent chain holds his tip, so it is a second dependent, not a fork,
  and the rewrite is refused in words before anything runs, as today. Which of the two is "the line" and which "the
  fork" is a guess, and ADR-0047 §4 says Graft never guesses. (Builder 14b found this in the dependents code; acceptance
  3 of ticket 14b first assumed it followed, corrected 2026-10-02.)
- His ruling (field notes :89) is about a branch started from a commit he amends — a replaced commit in his own
  branch, below its tip. That is §1's fork and it follows. With a dependent present, such a fork still follows
  (`at..F` does not reach his tip).
- Sections would make the two-dependents refusal liftable (pick one as the line, the other as a section); that is a
  separate decision, not taken here.

### 6. What "clean" means beyond the forecast (amended 2026-10-04)
His rule is "moves too **when it moves clean**". A clean textual replay is not enough: following must not move a
branch someone else is standing on, diverge a branch from its remote, or change what the branch was built on. Any
doubt → the fork stays and the preview says why. Staying is always safe: nothing of F moves, and he can still `r` it.
1. **Branches F carries** (`inside`: local branches on `at..F`'s first-parent chain, F's tip included as a gate on
   itself) each pass F's gates: not the default branch, not checked out in another worktree, not pushed (§6.2). The
   first that fails keeps the whole fork, named: `feature-x stays on a1b2c3d · feature-w is checked out in <path>`.
   Git does **not** refuse an `update-ref` of a branch checked out elsewhere: it moves it and leaves that worktree's
   index and files on the old commit (hand-run, git 2.54, review M1). The gate is Graft's alone.
2. **Pushed** means any of these, for F and the same for each carried branch W (with `onto..W`):
   a. its upstream is a remote-tracking ref (`refs/remotes/*`) and holds a commit of `onto..F` (the rewrite's
      `onto`: so `at`, the replaced commits under it, and F's own);
   b. a remote-tracking ref of its **own name** (`refs/remotes/<remote>/<its short name>`) holds a commit of
      `onto..F`, upstream set or not — `git push origin F` without `-u` publishes F as surely as with it;
   c. any `refs/remotes/*` ref holds one of F's own commits (`at..F`).
   A ref of **another** name holding only `at` or below (his line's own `origin/feature-a`) is no sign F is pushed:
   counting it would keep every fork. A **local** upstream (`refs/heads/*`, `--track feature-a`) is not a push: that
   branch is weighed by its own role (line, fork, carried, dependent). Own-name match is by suffix
   (`refs/remotes/<anything>/<short>`); a remote whose name holds `/` can make another branch's ref match — that
   errs to staying. Not seen: a push under another name, no upstream, holding none of F's own commits; the remote
   keeps the old commits and the journal every old tip. A fork with 0 commits whose upstream or own-name remote
   holds `at` stays.
   (Was, 2026-10-04: any upstream vs `onto..F`, or any remote ref vs `at..F` — blind to a same-name push without
   `-u` holding only `at` or below, and read a local upstream as pushed; review `m8-14d-fork-safety.md` M1, m1.)
3. **Base unchanged.** Compare, as sets of original commits, the line's commits at or under `at` before the edit with
   the commits in the rows up to and including `at`'s row after it (a squash/fixup group counts every commit folded
   into it; a staged change `a` folds in is not a commit and does not count). Unequal → stays:
   `would lose <sha7>` (a commit under F is dropped, `at` included, or moves after `at`'s row) or `would gain <sha7>`
   (a commit after `at` moves to or under `at`'s row, a fold into it included). Equal → F follows onto the result of
   the **last** row of `at`'s squash/fixup group, never mid-group.
4. Order: §6's gates are reads with no forecast; a fork that fails one is never forecast.

## Rejected
- **A second rebase of F inside the same entry**: two git operations; an abort aborts only the running one; kill -9
  recovery must know a second is owed.
- **merge-tree + commit-tree replay of F** (no stop possible, since only clean forks follow): ADR-0047's rejected
  replay loop by another name; hooks, empty-commit and committer rules would differ from git's rebase.
- **Every fork follows, conflicts stop** (jj-like without stored conflicts): a stop in a branch he never named.
- **Leave it as is**: the alternative had he said *stay*. Then item 2 would have closed as designed and the specs dropped.
- **(§6) Follow a fork whose base changes, and say the loss** (`follows · without 1875dfa`; jj's abandon does this):
  a teammate's branch silently loses code it was built on; one row of words is easy to miss before `y`.
- **(§6) Follow a pushed fork and list it under `published`** (`… will need force after`): a force push nobody asked
  for, on a branch that may be a teammate's.
- **(§6) Follow F and leave a failing carried branch behind**: that branch would be stranded on F's old commits.
