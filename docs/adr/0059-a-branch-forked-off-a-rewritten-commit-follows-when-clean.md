# ADR-0059 — A branch forked off a rewritten commit follows when its replay is clean

Date: 2026-10-02
Status: **Accepted**, 2026-10-02 (field notes :89, item 5: "go ahead with your recommendation" — the branch moves too
when it moves clean, shown in the preview first). Design note `m8-polish-and-ship.md`, "Where I'd differ", 2c.
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
  the default branch; F is not checked out in another worktree; F's upstream (if any) holds none of `at..F`; no other
  fork leaves `at..F`; F is among the first `LEFT_BEHIND_CAP` (8) named.
- **Stays** otherwise, with the reason in words (`would conflict in <path>`, `the rewrite stops below it`, `has a
  merge`, `pushed to <upstream>`, `checked out in <path>`, `forked from <fork>, which moves`, `not checked: <why>`,
  `the default branch`, `past the first 8`). A forecast hole is never read as clean (m5 design note hard choice 5,
  MAP graft-ops `preview` `ConflictForecast`).
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

## Rejected
- **A second rebase of F inside the same entry**: two git operations; an abort aborts only the running one; kill -9
  recovery must know a second is owed.
- **merge-tree + commit-tree replay of F** (no stop possible, since only clean forks follow): ADR-0047's rejected
  replay loop by another name; hooks, empty-commit and committer rules would differ from git's rebase.
- **Every fork follows, conflicts stop** (jj-like without stored conflicts): a stop in a branch he never named.
- **Leave it as is**: the alternative had he said *stay*. Then item 2 would have closed as designed and the specs dropped.
