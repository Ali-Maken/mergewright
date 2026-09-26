# Branch comparison and submodule display: prior art for M7

Researcher, 2026-09-26. Feature pipeline step 2 for two M7 items (PROMPT.md:594 "submodule display, branch comparison").
Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I have not run any of these tools. Step counts come
from their docs or issue text; where a step is not documented it is marked **(?)**. Claims about Graft carry a repo
`file:line`. Refused or empty pages are listed at the end.

**The need, in the user's terms.** His daily list is history, diffs, staging, fetch/pull/push, branch/checkout,
conflicts, stash, cherry-pick (memory `graft-daily-workflows`). He works in a team on dependent branches (memory
`works-in-a-team-on-dependent-branches`). The field notes hold no entry on branch comparison or submodules yet
(`docs/field-notes.md`, searched for "compare" and "submodule": none). So the need below is inferred from his workflow,
not in his words — **question for step 1: does he ask "what will my PR contain?" daily, and does any repo he uses have
submodules?**

**Counting rule:** `docs/research/ux-principles.md` §1.3. One step = one deliberate user action (key, click, menu pick).

---

## A. Branch comparison

### Git's own vocabulary (the thing every tool is wrapping)

- `git diff A...B` = `git diff $(git merge-base A B) B`: "the changes on the branch containing and up to the second
  commit, starting at a common ancestor of both" — https://git-scm.com/docs/git-diff
- `git diff A..B` = `git diff A B`: tip against tip. If `main` moved on since you branched, its new work shows up
  **reversed** in your diff.
- Commits: `git log main..HEAD` (mine, not on main) and `git log HEAD..main` (theirs, not on mine). For commit lists the
  two-dot form is the useful one; for the combined diff the three-dot form is. This mismatch confuses every tool below.

### Which diff users want: three-dot

GitHub uses three-dot for PRs by default, because it "keeps showing the changes introduced by the topic branch since the
branches diverged"; two-dot makes introduced changes "harder to understand" when the base moves —
https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-comparing-branches-in-pull-requests

Users of other tools ask for exactly that view, by name:
- lazygit #3767: "I would like to have opportunity to see all files changes like in git hub pull request view", i.e.
  `git diff origin/master...HEAD` — https://github.com/jesseduffield/lazygit/issues/3767 (open)
- JetBrains: "Option to compare git branch to merge-base instead of current revision" —
  https://youtrack.jetbrains.com/issue/IJPL-106178 ; "Triple dot diff (compare with branch)" —
  https://youtrack.jetbrains.com/issue/IDEA-190498 (page rendered empty to me; title only) ; forum "Show merge-base diff"
  — https://intellij-support.jetbrains.com/hc/en-us/community/posts/24887316382866-Show-merge-base-diff

Evidence is one-sided: I found no request for two-dot as the default. Two-dot is still wanted as an option
(lazygit's own proposal lists `A..B`, `B..A`, `A...B`, `B...A`, #2659 below).

### A.1 JetBrains "Compare with Current"

Source: https://www.jetbrains.com/help/idea/manage-branches.html
- Branches pane or VCS widget → pick branch → **Compare with Current**. A new Git tool-window tab lists "all commits that
  exist in the selected branch and do not exist in the current branch". `Ctrl+A` on the commits shows all differing
  files. Separate action **Show Diff with Working Tree** diffs the branch against your files.
- Two-dot or three-dot for the file diff: the page does not say **(?)**. The open requests above (IJPL-106178,
  IDEA-190498) suggest the file diff is not merge-base-based, but I could not read either ticket body **(?)**.
- Any two refs: requested as "Allow to compare any two branches" — https://youtrack.jetbrains.com/issue/IJPL-105207 ;
  status not read **(?)**.
- **"What will my PR contain?"**: checkout-free if you are on your branch: open branch widget (1), pick `main` (2),
  Compare with Current (3) — but that lists commits on **main** not on yours, the reverse of the question. The
  swap/other direction is not documented on that page **(?)**. Then `Ctrl+A` for files (4). **~4, direction (?)**.

### A.2 GitLens "Search & Compare" and Graph compare mode

Source: https://help.gitkraken.com/gitlens/side-bar/ ; https://help.gitkraken.com/gitlens/gitlens-features/
- A "hidden by default" view to "compare branches, tags, commits, and more". Results split into **Behind** ("commits
  missing from the target branch") and **Ahead** ("commits unique to the target branch"), with commands
  `compareWithUpstream`, `compareWithWorking`, `compareWithHead`, `compareWithSelected`.
- Commit Graph **Compare mode**: "Select two references to see Ahead, Behind, and All views inline."
- Two-dot or three-dot for the files list: not stated on either page **(?)**.
- **"What will my PR contain?"**: right-click branch (1) → compare with… (2) → pick `main` (3) → expand Ahead (4)
  **(?)** — menu wording not confirmed on the pages read.
- Praise/complaints: none found with a primary link in the time spent **(?)**.

### A.3 Sublime Merge

Source: https://www.sublimemerge.com/docs/getting_started
- "If multiple commits are selected, a diff of the first and last commit will also be displayed in the Details
  section." Branch comparison = select both branch tips in the graph. Tip-to-tip is two-dot semantics **(?)**, not
  confirmed by docs.
- No "commits on mine, not on main" list documented.
- **Complaints:** can't compare by picking two branches in the sidebar; the requester notes Fork already does it —
  https://github.com/sublimehq/sublime_merge/issues/1546 (open). Forum threads asking how at all:
  https://forum.sublimetext.com/t/how-do-i-diff-two-branches-in-sublime-merge/75095 ,
  https://forum.sublimetext.com/t/can-sublime-merge-show-diffs-between-branches/40733 . Request for a "Diff with another
  Branch" command — https://github.com/sublimehq/sublime_merge/issues/1831 (read via search summary only **(?)**).
- **"What will my PR contain?"**: find your tip in graph (1+ scroll), click (2), find main's tip (scroll, often far), cmd-click
  (3). **~3 plus scrolling**, and the diff is two-dot **(?)**.

### A.4 lazygit diffing mode

Source: https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md
- `W` / `Ctrl+E`: "View diffing options … diffing against selected ref, entering ref to diff against, and reversing the
  diff direction." Any ref can be typed. Files and diff then reflect the pair.
- Semantics: two-dot only — "lazygit only supports `A..B`", per the maintainer's redesign proposal
  https://github.com/jesseduffield/lazygit/issues/2659 (open), which proposes offering `A...B`, `B...A`, `A..B`, `B..A`
  once two refs are picked, and making "select for diff" do nothing else ("that's ALL it does") because the current mode
  surprises users.
- Commit list between two branches: requested, not built — https://github.com/jesseduffield/lazygit/issues/3792 ; also
  https://github.com/jesseduffield/lazygit/discussions/4338 , https://github.com/jesseduffield/lazygit/issues/311 .
- Divergence from base branch can be shown in the branches list (`showDivergenceFromBaseBranch`: none / arrow / counts) —
  https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md . A search summary claimed a `u b` key for "compare
  with base branch"; not found in the keybindings page **(?)**.
- **"What will my PR contain?"**: select `main` in branches (1–2), `W` (3), pick "diff against selected" (4), select your
  branch (5). **~5**, and it answers with two-dot, which is the wrong answer once main has moved.

### A.5 GitHub compare

Source: GitHub docs above.
- `/compare/base...head` page: commit list (head not in base) + three-dot file diff. Default for PRs.
- **"What will my PR contain?"**: the Compare & pull request banner or the compare URL: **~2–3** (open repo, pick base,
  pick head). The benchmark every complaint above points at.

### A.6 Magit

Source: https://docs.magit.vc/magit/Diffing.html
- `d r` (`magit-diff-range`): "RANGE should be a range (A..B or A…B) but can also be a single commit." Omitted side
  defaults to HEAD. `D r` toggles the range type between `revA..revB` and `revB...revA`.
- Commit lists: `l` log with a range argument; status buffer has "Unpulled from"/"Unpushed to" sections against upstream
  (not against main) **(?)** — sections page not read.
- **"What will my PR contain?"**: `d` (1) `r` (2) type `main...` + Enter (3). **3**, fully keyboard, but you must know
  the dot syntax.

### A.7 Fork

- Only secondhand: the Sublime Merge #1546 requester says Fork shows a diff summary when two branch names are selected
  in the sidebar. Fork's own docs not checked **(?)**. Two-dot or three-dot **(?)**.

### A.8 Summary

| Tool | Commits mine/theirs | Combined diff | Any two refs | PR-contents steps |
|---|---|---|---|---|
| GitHub | yes | three-dot | yes (URL) | ~2–3 |
| Magit | via log range | both, toggle | yes (typed) | 3 |
| JetBrains | one direction per action | (?) | requested | ~4 (?) |
| GitLens | Ahead + Behind | (?) | yes | ~4 (?) |
| Sublime Merge | no | tip-to-tip (?) | select 2 commits | ~3 + scroll |
| lazygit | no (requested) | two-dot only | yes (typed) | ~5 |

---

## B. Submodule display (read-only)

### Git's own states (what any display must map)

`git status` short form: `M` = "the submodule has a different HEAD than recorded in the index"; `m` = "has modified
content"; `?` = "has untracked files". Porcelain v2 `<sub>` field `S<c><m><u>`: commit changed / tracked changes /
untracked. `--ignore-submodules=none|untracked|dirty|all` — https://git-scm.com/docs/git-status

`git diff --submodule=short|log|diff`: `short` (default) shows only the two commit names; `log` "lists the commits in the
range like `git submodule summary`"; `diff` inlines the content diff — https://git-scm.com/docs/git-diff

So there are three separate facts: **pointer moved** (pinned ≠ checked out), **dirty inside**, **untracked inside** —
plus "not initialised".

### B.1 lazygit

Source: Keybindings page above. A Submodules panel (in the Files tab **(?)**): `Enter` enters the submodule as its own
repo, `Esc` returns; plus init/update/remove/bulk. How pinned vs checked-out is drawn in the panel: not in docs **(?)**.
- **Complaints:** a submodule with both a new commit and dirty content can't be unstaged cleanly —
  https://github.com/jesseduffield/lazygit/issues/3641 ; status slow with many submodules (~4 s per add on Windows with
  20+) — https://github.com/jesseduffield/lazygit/issues/4993
- **Praise:** `Enter`/`Esc` in and out is the model others lack (inference from docs, no user quote found **(?)**).

### B.2 Sublime Merge

Submodules listed in the Locations column, but can't be revealed in the graph ("excluding submodules") —
https://www.sublimemerge.com/docs/getting_started . How the pointer diff is drawn: not in docs **(?)**.
- **Complaints (the dominant theme is wrong state):** a submodule-only change is invisible —
  https://github.com/sublimehq/sublime_merge/issues/1084 (open); phantom or missing submodule changes git doesn't show —
  https://github.com/sublimehq/sublime_merge/issues/1932 ; unmodified submodules marked unstaged —
  https://github.com/SublimeTextIssues/Merge/issues/18 ; stage-and-reappear loop, fixed build 1070 —
  https://forum.sublimetext.com/t/bug-submodule-updated-in-sublime-merge-no-changes-in-git-bash/39161

### B.3 JetBrains

Submodules are registered as separate VCS roots; changes are grouped per repository. Pointer changes land **under the
submodule instead of the parent**, so they're hard to commit —
https://intellij-support.jetbrains.com/hc/en-us/community/posts/360008085940-Changes-to-git-submodule-commit-fall-under-submodule-instead-of-main-project .
Long-running requests: https://youtrack.jetbrains.com/issue/IDEA-64024 ("Git submodules should be supported"),
https://youtrack.jetbrains.com/issue/IDEA-202509 , https://youtrack.jetbrains.com/issue/IDEA-234159 ("Changes of
submodule objects do not appear in Local Changes"). Ticket bodies not read **(?)**.

### B.4 GitKraken Desktop

Source: https://help.gitkraken.com/gitkraken-desktop/submodules/ . Left-panel Submodules pane with named states: **"Out
of sync"** ("The pointer commit changed"), **"Added but not initialized"**, **"Added and initialized but not
committed"**; returning from a submodule prompts to save the new pointer. Complaints are about automatic sync, not
display — https://feedback.gitkraken.com/suggestions/193513/option-to-disable-submodule-synchronization

### B.5 Fork

Complaints only, all about actions: can't discard submodule changes (freeze) —
https://github.com/fork-dev/Tracker/issues/2308 ; stash fails when only submodules changed —
https://github.com/fork-dev/Tracker/issues/658 ; re-inits de-initialised submodules on pull —
https://github.com/fork-dev/TrackerWin/issues/2060 . Display itself not documented **(?)**.

### B.6 Magit

`magit-list-submodules` (RET on the "Modules" section) opens a table; columns configurable via
`magit-submodule-list-columns`; `magit-insert-modules` adds a Modules section to status —
https://docs.magit.vc/magit/Submodules.html . The module sections hook includes an overview (path, branch, version) and
Unpulled/Unpushed counts per module, per the older manual —
https://magit.vc/manual/2.13.0/magit/Listing-Submodules.html (current wording not rechecked **(?)**). User request for
more state ("git super status") — https://github.com/magit/magit/issues/2705

---

## Graft today (as far as checked)

- Two-commit compare exists as a key: `x` marks a compare base / completes a compare —
  `docs/design/02-keymap.md:196`. Whether it diffs two-dot or three-dot: not checked **(?)**.
- A merge-base read exists: `history_reads::merge_base`, used at `crates/graft-ops/src/preview/kinds.rs:41`.
- A `submodule` field appears in selection code (`crates/graft/src/update/selection.rs:193`, set `false`); what sets it
  and what draws it not checked **(?)**. The detail pane names a "type changed (file, symlink, submodule)" status —
  `crates/graft-ui/src/detail.rs:136`.
- The sidebar spec lists Submodules and "can be opened" (PROMPT.md:188, :278).

---

## Takeaways for Graft

1. **Take — three-dot as the default combined diff** (GitHub). Every complaint found asks for it; none ask for two-dot as
   default. Label it in words ("changes on `feature` since it left `main`"), never as dots.
2. **Take — two commit lists, both directions, side by side or as tabs** (GitLens Ahead/Behind). Answers "what's mine"
   and "what have I missed" in one screen; JetBrains shows one direction per action.
3. **Take — two-dot as an explicit toggle, not default** (Magit `D r`, lazygit #2659). Needed for "tip against tip".
4. **Invent — one key from a branch row: compare with its base.** Nobody reaches "what will my PR contain?" in under ~3
   steps locally. Target: cursor on branch (or HEAD) → 1 key → the compare screen against `main`/upstream base.
   **Question:** how the base is chosen (default branch, upstream, or the branch it was created from) — his dependent
   branches make "main" often wrong (memory `works-in-a-team-on-dependent-branches`). Decide in the design note.
5. **Take — any two refs by picking, not typing** (Sublime #1546, Fork): mark one ref, pick the other in the branch
   list; reuse the existing `x` mark (`02-keymap.md:196`) rather than a new key. Typed ref as fallback (lazygit, Magit).
6. **Reject — lazygit's global "diffing mode"** that silently changes what other panes show; its own maintainer calls it
   confusing (#2659).
7. **Take — git's three submodule facts drawn separately**: pointer moved (`pinned abc123 · checked out def456`), dirty,
   untracked, plus not initialised (GitKraken's named states). Status by state (CLAUDE.md status rule): a clean submodule
   shows nothing extra.
8. **Take — pointer change in a diff shown as `--submodule=log`**: old → new short SHAs plus the commit subjects in
   between, not a bare hash pair. Read-only, cheap, and answers "what did this bump pull in?". Content diff (`=diff`)
   rejected for M7: large and slow.
9. **Reject — per-submodule VCS roots in the main status** (JetBrains): pointer changes end up filed under the wrong
   repo.
10. **Invent/guard — correctness over richness.** Sublime's and JetBrains' complaints are about **wrong or missing**
    submodule state, not missing features. Read state from `git status --porcelain=v2` `<sub>` field; test a
    submodule-only change (Sublime #1084) and a moved+dirty submodule (lazygit #3641). Watch status cost with many
    submodules (lazygit #4993).
11. **Defer — enter submodule as its own repo** (lazygit `Enter`/`Esc`). PROMPT.md:188 says "can be opened"; multi-repo
    tabs are deferred (PROMPT.md:278). Question for design: does "open" mean a nested view with `Esc` back?

## Could not read / unconfirmed

- JetBrains `using-git-integration.html` and `work-with-branches.html`: empty / 404. YouTrack IDEA-190498 rendered empty.
- magit.vc `/manual/magit/Diffing.html` 404 (used docs.magit.vc instead).
- No Fork documentation read; Fork claims are secondhand.
- No user praise with a primary link found for GitLens compare or lazygit submodules.
- Whether Graft's `x` compare is two- or three-dot.
