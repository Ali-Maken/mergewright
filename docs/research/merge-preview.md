# Merge preview — prior art

Question: before a merge runs, what do the best-loved Git tools show (what comes in, which files clash and where, refusals), and how many steps from "I want to merge X" to merged?

Trigger: review finding M1 (`docs/reviews/ease-pass-05-10.md:56-71`). Built screen: `.scratch/ease-pass/issues/10-merge-preview.md` (ticket 10); accepted mock `prototypes/ease-mocks/mergewright-merge-screen.html:130-139`.

Rule: facts carry a link. **Question** marks anything not confirmed from a primary source. Researched 2026-10-18. Step counts start with the source branch selected (or the dialog's branch picker) and the target checked out, and end when the merge runs; a picker choice counts as one step.

## Summary

- **Nobody found shows a pre-merge clash list with places.** GitLens shows a clash **count** (paid tier); GitKraken shows "overlapping files and lines" but as a background alert, not a merge dialog; Tower's dialogs say clean / will conflict; GitHub shows a yes/no box. JetBrains, lazygit and (as far as public material shows) Sublime Merge show nothing before the merge runs.
- **Nobody found lists incoming commits inside the merge confirm.** GitLens puts the **count** in the confirm line ("Will merge N commits from X into Y"). Others rely on the graph / branch compare.
- **Fast-forward vs merge commit is named before running** in lazygit (menu label), GitLens (per-option detail line), JetBrains (dialog flags). Tower/Sublime: **Question**.
- **Unrelated history:** JetBrains and Sublime Merge offer `--allow-unrelated-histories` as a flag; none found refuses up front with a sentence.
- Step counts: lazygit 2 (`M`, `m`), JetBrains 2 via the branch popup, GitLens 3–4. Ours: 2 (`m`, `y`) per ticket 10.

## Per tool

### Sublime Merge
- **Merge entry points:** "Merge With…" context menu with flags (`--squash`, `--no-ff`) added build 1079; "New Merge Dialog" build 1103; unrelated histories via `--allow-unrelated-histories` in the Merge command, build 2068 — [changelog](https://www.sublimemerge.com/download).
- **Preview content:** **Question.** The online docs pages tried (`/docs/merging`) returned 404 and the changelog says nothing about a commit list or conflict forecast in the dialog. Not confirmed either way.
- **Complaints:** found issues are all about resolving after the stop (e.g. three-way view replaces the repo tab, [#1869](https://github.com/sublimehq/sublime_merge/issues/1869); no bulk take-theirs/mine, [#316](https://github.com/sublimehq/sublime_merge/issues/316)). No issue found asking for a pre-merge forecast.

### GitKraken Desktop — Conflict Prevention
- **What:** detects "overlapping edits in committed changes that haven't yet merged into the target branch"; without org members it "checks your current branch against the target branch". An alert icon opens a view of "overlapping files and lines"; clean state reads "Up to Date with Merge Target" — [help: conflict prevention](https://help.gitkraken.com/gitkraken-desktop/conflict-prevention/).
- **Shape:** a standing background alert per target branch, **not** a step in the merge action.
- **Limits:** Pro tier or higher; org-member detection needs the cloud org — same page. Started as "Automatic Conflict Detection" (preview) in 10.6, renamed in 10.8 — [GitKraken blog 10.8](https://www.gitkraken.com/blog/stop-merge-conflicts-before-they-happen-with-gitkraken-desktop-10-8) (vendor source).
- **User praise/complaints:** none found from users (search returned vendor pages only). **Question.**

### Tower
- **What:** "Dialogs which involve merging another branch or revision (e.g. merge, rebase, pull, …) will now show if merging will result in any conflicts or merge cleanly." — [Tower Mac release notes](https://www.git-tower.com/release-notes/mac). Branch Review filters branches by "Fully Merged / Mergeable / Merge Conflicts" — same page.
- **Cost complaint (vendor-admitted):** "mergeability checks could previously cause Tower to become unresponsive" in repos with large conflicts — same page.
- **Whether the dialog names files or places:** **Question.** The merge help page URL tried returned 404; release note says only "conflicts or merge cleanly".

### JetBrains (IntelliJ) "Merge into"
- **Steps:** branch popup → branch → "Merge <branch> into <target>" (no options), or Git | Merge dialog → pick branch → Modify options (`--no-ff`, `--ff-only`, `--squash`, `-m`, `--no-commit`, `--no-verify`, `--allow-unrelated-histories`) → Merge — [IntelliJ docs](https://www.jetbrains.com/help/idea/apply-changes-from-one-branch-to-another.html).
- **Preview:** none documented. Conflicts appear after the run under a Merge Conflicts node; dirty files trigger a "Smart merge" offer (stash, merge, unstash) — same page.
- **Complaints:** users ask for commit context during the 3-way merge ([IDEA-102522](https://youtrack.jetbrains.com/issue/IDEA-102522)) and a non-modal merge window ([IJPL-99530](https://youtrack.jetbrains.com/projects/IJPL/issues/IJPL-99530/Non-modal-Merge-changes-UI)) — both after the stop. A PhpStorm user reports getting a fast-forward when he wanted a merge commit because the quick path skips the dialog — [JetBrains community](https://intellij-support.jetbrains.com/hc/en-us/community/posts/207048285/comments/115000603390).

### lazygit
- **Steps:** on a branch, `M` opens the Merge menu; `m` runs the first item — 2 keys. Source: `MergeRefIntoCheckedOutBranch`, [merge_and_rebase_helper.go](https://github.com/jesseduffield/lazygit/blob/4e1eebe278790f257a875da6633234391a5a85b3/pkg/gui/controllers/helpers/merge_and_rebase_helper.go) (read at master `4e1eebe2`, ~l.518-610).
- **Fast-forward wording:** items "Regular merge (fast-forward)" / "Regular merge (with merge commit)"; the fast-forward item is disabled with "Cannot fast-forward '<cur>' to '<sel>'" when HEAD is not an ancestor; tooltips "Fast-forward '<cur>' to '<sel>' without creating a merge commit." — [english.go](https://raw.githubusercontent.com/jesseduffield/lazygit/master/pkg/i18n/english.go) l.1507-1516.
- **Preview:** none — no commit list, no conflict check in the menu (same source).
- **Users asking:** dry-run request "before I push … check if a branch I'm on will have merge conflict errors"; maintainer agreed, preferring a dry-run item in the merge menu over a new key "to preserve muscle memory" — [discussion #3004](https://github.com/jesseduffield/lazygit/discussions/3004). Not found implemented in the menu source above.

### GitHub pull request merge box
- **How:** GitHub builds a test merge commit; `mergeable` is true/false/null, null meaning a background job is still computing — [REST pulls docs](https://docs.github.com/en/rest/pulls/pulls).
- **Shows:** when conflicts exist the "Merge pull request" button is deactivated — [about merge conflicts](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/addressing-merge-conflicts/about-merge-conflicts); a "Resolve conflicts" button leads to an editor with the conflicting files listed; complex conflicts "use the command line" — [resolving on GitHub](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/addressing-merge-conflicts/resolving-a-merge-conflict-on-github). Whether the box itself lists files: **Question** (docs don't say).
- **Complaints:** the check hangs at "Checking for ability to merge automatically" — [#130262](https://github.com/orgs/community/discussions/130262), [#121015](https://github.com/orgs/community/discussions/121015), [#140901](https://github.com/orgs/community/discussions/140901); GitHub Desktop hung on the same check for a huge diff — [desktop#14629](https://github.com/desktop/desktop/issues/14629).

### VS Code + GitLens (merge command)
- **Steps:** command → pick branch → confirm step choosing an option (merge / squash / …) — 3–4 picks. Source `src/commands/git/merge.ts` at `59a73144` ([file](https://github.com/gitkraken/vscode-gitlens/blob/59a73144fac642396eadec62b40b4d3f480b291c/src/commands/git/merge.ts)).
- **Count, not list:** detail line "Will merge {n} commits from {src} into {dst}, fast-forwarding if possible" / "…, always creating a merge commit" / "…, only if it can fast-forward" (l.411-431). Nothing to merge → "{dst} is already up to date with {src}" (l.345-355).
- **Clash forecast:** paid/trial only (`isSubscriptionTrialOrPaidFromState`, l.529-535): spinner "Detecting Conflicts...", then "No Conflicts Detected", or "Conflicts Detected — Will result in {n} conflicting files that will need to be resolved", or "Unable to Detect Conflicts" (l.537-584). A **count** of files, no names, no places.
- **Users:** none found. **Question.**

### Others
- **git itself:** `git merge-tree --write-tree` gives the conflicted file list and per-path messages ("Auto-merging", "CONFLICT (…)"); exit 0 clean, 1 conflicts; "Do NOT interpret an empty Conflicted file info list as a clean merge; check the exit status"; errors out on no common history unless `--allow-unrelated-histories` — [git-merge-tree docs](https://git-scm.com/docs/git-merge-tree). A small Mac client runs this when its merge sheet opens to list would-conflict files — [Spoon PR #19](https://github.com/zunda-pixel/Spoon/pull/19) (not a well-loved tool; shown as a pattern).
- **Not checked:** Fork, GitUp, Magit, jj. **Question** whether any previews.

## Comparison with what we built (facts, not decisions)

| Aspect | Ours (ticket 10 / mock) | Others found |
|---|---|---|
| Counts vs file lists | Counts for changed / combined files; clashing files named (`10-merge-preview.md` M2 box; mock l.135-138) | GitLens: commit count + clash-file count, no names. GitKraken: files and lines (alert view). GitHub: yes/no, files in the editor after. Tower: clean / conflicts. |
| Separate incoming list | `⏎` opens a list of incoming commits (M3) | None found inside a merge confirm; GitLens gives a count only. |
| Clash forecast with places | Path + `(p places)` per clashing file | Only GitKraken shows lines, outside the merge action and paid. Nobody puts places in the confirm. |
| Unrelated-history refusal | Box stays open, `y` withheld, "Can't merge: … share no history." | JetBrains, Sublime Merge offer `--allow-unrelated-histories`; no up-front refusal sentence found. git `merge-tree` errors by default. |
| Fast-forward vs merge commit | Not distinguished: a fast-forward also reads "After y the merge is committed." (`10-merge-preview.md:131`, builder note) | lazygit, GitLens, JetBrains all name it before running. |
| Steps | `m`, `y` = 2 | lazygit 2, JetBrains 2 (quick path), GitLens 3–4. |
| Cost | 12 git spawns clean merge, 10 fast-forward; ff `merge-tree` wasted (`10-merge-preview.md:134`) | Tower and GitHub both have documented hangs/unresponsiveness on the check. |
