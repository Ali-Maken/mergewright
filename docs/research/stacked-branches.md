# Branch dependency (stacks) and chain rebase: prior art for M7

Researcher, 2026-09-26. Feature pipeline step 2. Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I have not run any of these tools. Step counts are
built from the commands their docs show, one command = one step; typed-command keystrokes are not counted
character by character. Where the docs leave a step out I mark it **(?)**. Sites that refused or 404'd are listed in §5.

**The user's situation.** He works in a team, on branches others build on. His words: *"I changed a branch that others
are built on, now fix the chain."* (ADR-0032 `docs/adr/0032-branch-dependency-and-chain-rebase.md:12-14`). He wants no PR
integration, no external service, no "stack" object to learn (`...0032...md:37-42`). ADR-0032 is **conditional** on real
use (`...0032...md:44-49`). A grep of `docs/field-notes.md` for rebase/chain/stack/squash/depend finds one line (67), about
pull-with-rebase with unsaved edits, **not** about a broken chain. So the field evidence the ADR asks for is not yet in
the notes. That is a question for the conductor, not a finding against the feature.

**The three tasks** (a chain `a ← b ← c` on `main`):
- **T1** main moved; rebase the chain onto it.
- **T2** `a` was squash-merged upstream; drop it, restack `b`, `c` onto main.
- **T3** an edit to `b`; restack `c` above it.

---

## 1. The tools

### 1.1 `git rebase --update-refs` (plain Git, 2.38+)

Source: https://git-scm.com/docs/git-rebase
- **Parent:** neither inferred nor recorded. Git "automatically force-update[s] any branches that point to commits that are
  being rebased". The chain is whatever branches lie inside the range you rebase. Branches checked out in another worktree
  are skipped. `rebase.updateRefs=true` makes it the default.
- **T1:** checkout the top branch, `git rebase main --update-refs`. **2 steps.** The worked example is
  https://andrewlock.net/working-with-stacked-branches-in-git-is-easier-with-update-refs/ . You must know which branch is
  the top. A fork in the chain (two children of `b`) means one rebase per tip, and the second is not covered here **(?)**.
- **T2:** plain `git rebase main` replays `a`'s commits on top of their squashed copy, which gives conflicts. The fix
  is `git rebase --onto main a c --update-refs`, or an interactive rebase that drops `a`'s picks: **2–3 steps, and
  the user has to know the trick.** The article on this (Drew Jaja, Medium) returned 403, so the exact recipe is
  unconfirmed **(?)**. Git does not detect the squash merge.
- **T3:** checkout the top, `git rebase b --update-refs`. **2 steps** (same Andrew Lock article). If you edit `b` inside
  an interactive rebase of `c` with `--update-refs`, it is **1 operation**.
- **Conflict mid-chain:** an ordinary rebase stop. The docs do not say what happens to the pending `update-ref` lines on
  `--abort` **(?)**.
- **Praise:** turns a "long-winded and prone to error" double rebase into one command,
  https://adamj.eu/tech/2022/10/15/how-to-rebase-stacked-git-branches/ .
- **Complaint:** GUIs lag behind it. GitKraken users request it as "the only command run outside of gitkraken UI so
  far", https://feedback.gitkraken.com/suggestions/440456/support-git-rebase-updaterefs .

### 1.2 Graphite (`gt`)

Source: https://graphite.com/docs/command-reference
- **Parent:** **recorded**. `gt track` has you pick the parent, and it is stored as "Graphite metadata". Where exactly
  (git refs or a file) is not stated on that page **(?)**.
- **T1:** `gt sync`: "Sync all branches with remote, prompting to delete any branches for PRs that have been merged or
  closed", then "restacks branches that can be rebased without conflicts". **1 step** plus prompts.
- **T2:** same `gt sync`. It detects the merge from **PR state on the forge**, not from Git. **1 step + 1 prompt.**
- **T3:** `gt modify` "Automatically restacks descendants". **1 step** (edit + amend + restack in one).
- **Conflict:** "prompted to resolve them via interactive rebase", then `gt continue`. `sync` **skips** branches that
  would conflict and leaves them for a later `gt restack`.
- **Praise:** a whole stack in one command; widely used (HN launch, https://news.ycombinator.com/item?id=37570929).
- **Complaint:** "Graphite is quite hostile to collaborating on stacked branches. If one dev restacks and a collaborator is
  working in a dependent branch, they're hosed." https://news.ycombinator.com/item?id=39480095 . **This is the user's
  exact situation**: other people build on his branches. The price is also named, https://news.ycombinator.com/item?id=44350195 **(?, not read in full)**.

### 1.3 git-branchless (`git sync`, `git restack`)

Sources: https://github.com/arxanas/git-branchless/wiki/Command:-git-sync ,
https://github.com/arxanas/git-branchless/wiki/Command:-git-restack
- **Parent:** **inferred** from the commit graph. It works on commit stacks, and branches are just labels on them.
  Rewritten commits are shown with ✕ in the smartlog.
- **T1:** `git sync --pull`. **1 step.** It rebases all stacks onto main without checking them out, in memory.
- **T2:** not handled. A user writes that `git sync` "doesn't currently work for squash-merges",
  https://github.com/arxanas/git-branchless/discussions/1218 . You have to do it by hand with `git move` **(?, steps not documented)**.
- **T3:** amend `b`, then `git restack`. **2 steps.** Restack moves the "abandoned" descendants onto the rewritten commit.
- **Conflict:** `sync` "skips any commit stacks which would have merge conflicts unless you pass the `--merge` option",
  and prints a summary of what succeeded and what was skipped. Nothing stops halfway by default.
- **Praise/complaint:** the in-memory, skip-on-conflict design is the praised part (same wiki). The complaint is squash
  merges (discussion #1218 above).

### 1.4 jj (Jujutsu)

Sources: https://docs.jj-vcs.dev/latest/tutorial/ , https://docs.jj-vcs.dev/latest/github/ ,
https://docs.jj-vcs.dev/latest/cli-reference/
- **Parent:** **inferred**. There is no branch parent at all: commits have parents, and bookmarks point at commits.
  Bookmarks are needed only "when you need to push".
- **T1:** `jj git fetch`, `jj rebase -b <bookmark> -d main` (one per independent branch). **2 steps.**
- **T2:** jj has no squash-merge detection in core that I could find **(?)**. `--skip-emptied` "removes commits that
  become empty after rebasing", which covers the case where `a`'s content is already in main **and** replays cleanly.
  jj-stack's `merge` "rebases your local changes if required (e.g. after a squash merge)",
  https://www.serpentine.com/posts/2026/announcing-jj-stack/ (third party).
- **T3:** **0 extra steps.** Rewriting a commit rebases its descendants automatically: "Rebased 2 commits" (tutorial).
- **Conflict:** **never stops.** "the conflicts did not prevent the rebase from completing successfully, nor did it
  prevent C from getting rebased on top". Conflicts are stored in the commits and resolved later.
- **Praise:** automatic descendant rebase, and conflicts that don't block, e.g.
  https://osamaqarem.com/blog/jj-has-replaced-git-for-me **(?, not read in full)**.
- **Complaint:** a new VCS and a new model, which ADR-0032 rules out ("no new mental model").

### 1.5 spr and ghstack (commit-per-PR tools)

- **spr** https://github.com/ejoffe/spr: "Each commit becomes a pull request". `git spr update` / `git spr sync` / `git spr merge`.
  **Parent = the previous commit on one local branch**, so there are no dependent *branches*. GitHub only.
- **ghstack** https://github.com/ezyang/ghstack: each commit gets `base`/`head`/`orig` branches on the remote. T1 is
  `git rebase origin/main`, **1 step**. Landing needs `ghstack land`, because "Standard GitHub UI merging won't work".
  Don't `git merge`: "ghstack will throw a hissy fit". Complaint: remote branches pile up and "external cleanup
  mechanisms are needed".
- Both reshape the user's model (one branch, many commits) and need the forge. **Out of scope by ADR-0032**, listed for completeness.

### 1.6 Git Town

Sources: https://www.git-town.com/commands/sync , https://www.git-town.com/stacked-changes ,
https://www.git-town.com/preferences/sync-feature-strategy
- **Parent:** **recorded in git config** (`git-town-branch.X.parent`). If none is set, it "looks for a pull/merge request
  for this branch and uses its parent branch".
- **T1:** `git town sync --stack` (or sync all). **1 step.** The default strategy is **merge**, not rebase. Rebase and
  compress are opt-in.
- **T2:** `sync` "deletes branches whose tracking branch was deleted at the remote if they contain no unshipped changes",
  checked by diffing against the parent. **1 step**, but it relies on the remote branch being deleted.
- **T3:** `git town sync --stack` from `b`. **1 step** after the edit **(?, the per-branch walk is not described)**.
- **Conflict:** stops. Then `git town continue` after resolving, or `git town undo`, which brings the repo "back to the state it
  was before you ran the currently suspended Git Town command". **Undo of the whole multi-branch operation** is the
  notable part.
- **Squash-merge, in their own words:** squash-merging an older branch gives **"phantom conflicts"**, where Git sees
  colliding edits that aren't real. Their advice: sync often, enable `rerere`, prefer fast-forward, compress branches,
  "Branches must be shipped oldest-first" (stacked-changes page).

### 1.7 av (Aviator CLI)

Source: https://docs.aviator.co/aviator-cli/manpages/av-sync-1 , search excerpt of
https://docs.aviator.co/aviator-cli/manpages/av-git-interaction-7
- **Parent:** **recorded** in `.git/av/av.db` (from the git-interaction man page as shown in search results. I did not open that page directly **(?)**).
- **T1:** `av sync --all`. **1 step.** "if a part of the stack is merged, the rest of the stack is rebased to the latest trunk commit".
- **T2:** same command. "When a branch is merged, the child branches are restacked to the new parent", and
  `--prune=ask` (default) offers to delete it. How it detects a squash merge (PR state or Git) is **not stated (?)**.
- **T3:** `av sync` from `b` (`--current` does not descend, the default does). **1 step (?)**.
- **Conflict:** "prompts you to resolve the conflict, and continue with `av sync --continue`", or `--abort`.
- **Complaints:** UX threads exist, e.g. https://github.com/aviator-co/av/discussions/302 (not read in full).

### 1.8 Sapling (`sl`)

Sources: https://sapling-scm.com/docs/commands/restack , https://sapling-scm.com/docs/overview/smartlog/
- **Parent:** **inferred** (commit graph, Mercurial-style obsolescence).
- **T3:** after `amend`/`absorb`, `sl restack` "rebase all commits in the current stack onto the latest version of their
  respective parents". **1 step.**
- **T2:** smartlog marks landed commits with `x` and "Landed as YYY" and hides them. So landed detection exists,
  including for rewritten (squashed) landings, at least with Sapling's own server **(?)**.
- **T1 and conflicts:** not on the pages I read **(?)**.

### 1.9 lazygit

Source: https://github.com/jesseduffield/lazygit/blob/master/docs/Stacked_Branches.md
- Builds on `rebase.updateRefs`: "If you then rebase the topmost branch of the stack, the other ones in the stack will
  follow". Branch heads inside a stack get a **cyan asterisk** marker. **Parent inferred** (from Git).
- T1/T3: rebase the top branch (select the top, rebase onto target: about 2–3 keys **(?)**). T2: no detection documented **(?)**.
- **This is the closest peer, and the model ADR-0032 already describes:** show the dependency, then one rebase moves the chain.

### 1.10 GitKraken / GitLens

- GitKraken Desktop: no `--update-refs` support (feedback request above). A stacks request is open,
  https://feedback.gitkraken.com/suggestions/311022/support-for-stacked-diffs-and-prs . Since 11.10 you can "Rebase N
  commits onto [branch]" (range rebase), per search results **(?, not opened)**.
- GitLens: an open issue plans "one action rebasing every branch in the stack from bottom to top",
  https://github.com/gitkraken/vscode-gitlens/issues/5838 . **Not built.**

---

## 2. Step counts side by side

| Tool | Parent known by | T1 main moved | T2 bottom squash-merged | T3 middle edited | Conflict mid-chain |
|---|---|---|---|---|---|
| git `--update-refs` | range (inferred) | 2 | 2–3, manual `--onto` | 2 (1 inside rebase -i) | stops; abort semantics (?) |
| Graphite | recorded (metadata) | 1 | 1 + prompt (forge PR state) | 1 (`gt modify`) | stops; sync skips conflicting |
| git-branchless | graph (inferred) | 1 | **not handled** | 2 | **skips** conflicting stacks |
| jj | graph (inferred) | 2 | partial (`--skip-emptied`) | **0** | **never stops**; stored in commit |
| Git Town | recorded (git config) | 1 | 1 (needs remote branch deleted) | 1 (?) | stops; continue / **undo whole op** |
| av | recorded (`.git/av/av.db`) | 1 | 1 + prompt | 1 (?) | stops; continue / abort |
| Sapling | graph (inferred) | (?) | landed shown `x` | 1 | (?) |
| lazygit | range (inferred) | ~2–3 keys (?) | (?) | ~2–3 keys (?) | git rebase stop |

## 3. Squash-merge detection: the hard part

- A squash merge lands `a` as **one new commit**, so no commit of `a` matches upstream by hash or by per-commit patch-id.
  Only `a`'s **combined** diff can match. A tool's fix for exactly this bug explains it:
  https://github.com/Pajn/Kindra/pull/94 (a third-party stack tool). It says the case appears "once upstream has touched
  any of the branch's files", which is "the normal case".
- Git Town names the result, **phantom conflicts** (https://www.git-town.com/stacked-changes).
- Ways tools know `a` is merged:
  1. **Forge PR state** (Graphite; likely av **(?)**). Needs the network and an account, so it is **excluded** by ADR-0032.
  2. **Remote branch deleted + no unshipped diff vs parent** (Git Town). Local, but depends on the team deleting branches.
  3. **Empty after replay** (jj `--skip-emptied`). Fails when the replay itself conflicts.
  4. **Range patch-id or tree comparison** against upstream (the Kindra fix). Local and forge-free, but must be done
     carefully (same diff options on both sides).
- **Not handled at all:** git-branchless (discussion #1218), plain git.

## 4. Takeaways for Graft

- **Take: parent inferred from the graph, not recorded** (git, jj, branchless, lazygit). Reason: ADR-0032's "no new mental
  model", and recorded metadata (Graphite, av, Git Town) goes stale when teammates without the tool push. That is
  Graphite's named failure, https://news.ycombinator.com/item?id=39480095 .
- **Take: `--update-refs` semantics as the engine** (one rebase of the tip moves every branch in range). Reason: plain Git,
  what lazygit rides on, and no state of our own to break.
- **Take: lazygit's marker for branch heads inside a chain**, drawn on Graft's existing graph. Reason: ADR-0032 item 1,
  "show when branches depend on one another".
- **Take: one operation "rebase this and everything above"** for T1 and T3, previewed as a whole chain (ADR-0032 item 2).
  Target counts: T1 ≤ 3 steps, T3 ≤ 3 steps. Best-in-class is 1 (Graphite/Git Town) or 0 (jj), but those record
  state or change the VCS.
- **Take: Git Town's undo of the whole multi-branch operation.** Reason: M3's promise "you cannot lose work". Mid-chain
  abort must restore **every** branch, not only the current one.
- **Invent: local squash-merge detection** (range diff/patch-id vs upstream, method 4 above). Offer "`a` looks merged into
  main, drop it from the chain" in the preview. Reason: the forge-based methods are excluded by ADR-0032, git-branchless
  and plain git don't do it, and Git Town's needs a deleted remote branch. It must be shown as a **suggestion in the
  preview**, never applied silently, because detection can miss (Kindra #94).
- **Reject: skip-conflicting-stacks (branchless) and never-stop conflicts (jj).** Reason: Graft has M6's conflict screen,
  so a stop should land the user there with "branch 2 of 3" context. Deferring conflicts into commits is a jj-only model.
- **Reject: PR/forge integration, commit-per-PR models (spr, ghstack), recorded parent metadata.** Reason: ADR-0032 "Explicitly out".
- **Question, not a finding:** ADR-0032 is conditional on field notes showing the pain. `docs/field-notes.md` has no
  broken-chain entry as of today (grep, line 67 is the only rebase hit). Should the conductor confirm with the user before step 3?
- **Question:** forks in the chain (two branches built on `b`). `--update-refs` covers only one tip per rebase. Graft
  would need to rebase each tip, or `--rebase-merges`. The behaviour is unverified here.

## 5. Could not confirm / access notes

- Medium (Drew Jaja, squash + update-refs): **403**.
- docs.aviator.co concepts page: **404**. Used the `av-sync` man page instead. The `.git/av/av.db` location comes from a search excerpt only.
- Graphite's metadata storage location, av's merge detection method, Sapling's T1 and conflict flow, and lazygit's exact keys: not in the pages I read.
- `git rebase --update-refs` behaviour on `--abort` is not in the man page.
- The HN pricing thread, the jj blog and the av discussion #302 appear only in search results. I did not read them in full.
