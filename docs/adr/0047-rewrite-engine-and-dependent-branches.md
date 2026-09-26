# ADR-0047 — Every rewrite runs git's own `rebase -i` on a todo Graft writes; dependent branches are inferred and follow on one line

Date: 2026-09-26
Status: **accepted** (conductor, 2026-09-26, on the user's M7 decisions; drafted by planner, M7 pass 2a).
- Direction from the user: Decision 1 "the dependent branch follows" (field notes :76), which meets ADR-0032's condition.
- Mechanism: this ADR. The signature ticket `rewrite-engine` may amend the "open to the builder" points, in its NOTES, with evidence.

Design note: `docs/design-notes/m7-power-tools.md`, hard-to-reverse choices 2 and 3, and "Pass 2a". Studies:
`docs/research/interactive-rebase.md` §4.1–4.6 and §4.12; `docs/research/stacked-branches.md` §1.1, §1.9, §4, and :30-33.
Builds on: ADR-0032 (no stack object, whole-chain preview) and ADR-0045 §4 (edits set aside around a rebase).

## Context
- M7 rewrites history in six ways:
  - squash, fixup, reword, reorder, drop and edit in the editor (`R`);
  - amend staged changes into an older commit (`a`);
  - absorb;
  - rebase onto a moved main (`r`);
  - amend with dependents following;
  - drop a squash-merged bottom branch.
- Stops, abort, kill -9 recovery, M6's banner and M6's resolver already read git's `rebase-merge/` state
  (`graft-ops` `state::InProgressDetail`, `apply::resume`). A second engine would need its own copy of each.
- Today's rebase runs `rebase --no-update-refs --no-autostash <onto>`, because `rebase.updateRefs` moved branches the
  journal entry never recorded, so `u` could not put them back (M2 review fix).
- `--update-refs` moves only refs whose commits lie **inside** the rebased range (`stacked-branches.md` :30-33).
  Branches built on his branch lie **above** it.

## Decision

### 1. One engine: `git rebase -i`, fed a todo Graft generated
- Every rewrite plans a `Vec<TodoLine>`, oldest first. That includes `r` and `a`-into, and `a` with dependents.
  - `a`-into and absorb first write the staged hunks as commit objects with no ref, then plan them as `fixup` lines.
- Git runs the todo:
  - `GIT_SEQUENCE_EDITOR` replaces git's todo with Graft's file.
  - The todo holds only `pick reword edit squash fixup drop update-ref`, never `exec`. So git's `msgnum/end` count
    is the user's own rows.
- **Messages are written before `y`.** Any message git asks for (reword, and the head of a squash group) comes from
  Graft's saved messages, through a `GIT_EDITOR` helper keyed by the todo line git is on. The editor only opens where
  the user chose to type.
  - **Open to the builder:** how the helper names the line. The last line of `rebase-merge/done` is the candidate;
    whether git stores full or short ids there must be checked against git, not assumed.
  - **Rejected:** `exec git commit --amend -F`. It adds todo lines that change the step count in the banner, and it
    runs hooks differently from git's own reword.
- The plan is saved under `.git/graft/rewrite/<entry-id>/`: the todo, the messages, the line of branches and the branch
  HEAD returns to. It is saved **before** the journal's Pending line. So after a kill -9, Graft can continue, name
  the chain in the banner, and abort.

### 2. Every moved ref is recorded
- The todo's `update-ref refs/heads/<b>` lines are Graft's own, one after each branch's tip. Graft never uses the
  `--update-refs` flag or the `rebase.updateRefs` config: it passes `-c rebase.updateRefs=false` and writes the lines
  itself.
- The entry's `refs_before` holds every branch in the line and HEAD's symbolic ref. Undo's Rebase inverse
  (`06-undo-journal-design.md:117`, a CAS for every recorded ref, then the index and the worktree) puts all of them back
  as one step.

### 3. A line of dependents runs from the top, and HEAD comes home
- When branches are built on his, git rebases the **top** of the line (`rebase -i <onto> <top>`), with an `update-ref`
  line for each branch below the top.
- After the finish or an abort, Graft switches back to the branch he was on. That switch is part of the same entry.
- A stop (conflict or `edit`) happens with HEAD detached inside the line. The banner names the branch the step belongs
  to (`feature-b, branch 2 of 3`), from the saved plan.

### 4. Dependents are inferred, one line, and never a fork
- A **dependent branch** (on screen: `built on yours`) is a local branch B that meets all of these:
  - B's first-parent chain contains the rewritten branch's tip;
  - no merge lies between the two;
  - B is not the default branch;
  - B is not checked out in another worktree.
- The dependents found must form **one line** (each built on the one below).
- **A fork is refused in words**, with both branches named: `feature-b and feature-c are both built on yours · Graft
  moves one line`. Graft never guesses which one follows.
- A dependent checked out in another worktree is refused in words, and its path is named.
- No stack object and no recorded parent (ADR-0032). The graph is the record.
- git older than 2.38 (no `update-ref` todo command): a rewrite with dependents is refused in words. One without
  dependents runs.

### 5. Refused before anything runs
- Refused: a merge inside the range (`--rebase-merges` is not in M7), a detached HEAD, a commit not on his branch,
  and `a`-into or absorb with nothing staged. Each has its own words.
- Unsaved edits are not refused: they are set aside and put back inside the entry (ADR-0045 §4).
  - An absorb hunk that fits no commit comes back as it was.
  - If the put-back loses the staged/unstaged split (KNOWN_GAPS :804), the preview says so.

### 6. Every rebase goes through this engine
- `r` builds a todo of `pick`s. Where the preview found them, the todo also holds the suggested `drop`s of a
  squash-merged bottom branch, shown before `y` and never silent.
- One path: the M5/M6 rebase tests are the regression net.

## Consequences
- The M6 banner, resolver, relaunch, abort and journal keep working unchanged, because git's own state is the truth.
  The banner gains only the chain line.
- A shell `git rebase --continue` during a Graft rewrite behaves as git always does. Only the messages differ: git
  opens his editor, because the helper is not in his shell's environment.
- The helper file and the saved plan are Graft files inside `.git/graft/`. They are removed when the entry closes.
- The first rewrite after an upgrade runs a different command line from M6's rebase. The oracle tests in
  `rewrite-engine` compare the result with a hand-run `git rebase -i` on the same repo.

## Rejected
- **Graft's own cherry-pick loop** (a replay engine): it duplicates stops, abort and recovery, and a kill -9 leaves no
  state git understands.
- **`git replay`**: experimental, and it stops on no conflict (it fails instead), so an `edit` stop is impossible.
- **Recorded parents or a stack file** (Graphite, git-branchless): rejected in ADR-0032. A recorded parent goes stale
  the moment a teammate rebases in the shell.
- **Moving every branch that contains the commits**: it would move main, or a teammate's branch merged long ago.
