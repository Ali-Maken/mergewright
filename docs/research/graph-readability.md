# Graph readability at ~10–20 lanes: prior art

Pipeline step 2. Researched 2026-10-17. Distinction principle: `ux-principles.md` §3.6–3.7 (not repeated).

**Problem (user, 2026-10-17, work repo, ~10 lanes):** "when there are twenty lines i see at once, how am i able to distinguish which line is which, wont it be a clutter? … i dont know whats happening" / "what about abbreviation of name? i still cant know at glance who did what". Asked for Sublime Merge's good points, not a copy.

## 1. Sublime Merge (main focus)

**Too many lanes**
- **Merge commits folded by default; only first parent shown.** "Commit Folding collapses merge commits down into a single entry in the commit graph", click to show the merged-in branch ([build 1070 blog](https://www.sublimemerge.com/blog/sublime-merge-build-1070)). Default since dev build 1067 ([forum](https://forum.sublimetext.com/t/sublime-merge-dev-build-1067/39651)); opt out: `expand_merge_commits_by_default` ([forum](https://forum.sublimetext.com/t/sublime-merge-tree-view/40808)). Developers' framing: from "a tangle of lines" to clean history (per search summary of the blog; exact quote unconfirmed).
- **Per-ref visibility (eyeball) in the Locations sidebar.** Click eye = show/hide one branch or remote; **Alt+Click = show only this one** ([forum](https://forum.sublimetext.com/t/sublime-merge-tree-view/40808)).
- No lane compaction: "each branch is indented one level more than the last" — issue open since 2018, no maintainer reply ([#19](https://github.com/sublimehq/sublime_merge/issues/19)). Ordering is by date; topo order requested ([forum](https://forum.sublimetext.com/t/feature-request-git-branch-graph-with-topological-order/58382)).

**Which line is which**
- Lane colour = "next color in the allowable commit graph colors" — palette rotation, **not tied to the branch** ([forum](https://forum.sublimetext.com/t/branch-graph-colorization-not-matching/47902), community reply, not staff). Ref badges are coloured **by kind** (branch / remote / tag / stash / HEAD), unrelated to lane colour — users read that as a mismatch (same thread).
- Selecting a Locations item jumps to it in the graph ([docs](https://www.sublimemerge.com/docs/getting_started)).
- Dashed lines: HEAD → working-dir row ([forum](https://forum.sublimetext.com/t/dashed-lines-vs-solid-lines-in-the-commit-graph/68354)).
- **No ancestry highlight on selection.** Open request #2009 (2025): recolour lines from a selected merge to root, tint parent rows; "Hiding all other branches is a possible workaround, but not very comfortable" ([#2009](https://github.com/sublimehq/sublime_merge/issues/2009)). Kernel-repo commenter: a 4-commit merge "requires scrolling".

**Who did what**
- Each row = 2 lines: message, then **full author name + time** ([#18](https://github.com/sublimehq/sublime_merge/issues/18)). No avatars or initials found in docs (unconfirmed absence). `commit_graph_author_date` pref added build 2077 (same issue).
- Complaint: 2 lines/commit wastes height; theme hacks give ~50% more commits ([#18](https://github.com/sublimehq/sublime_merge/issues/18), [#692](https://github.com/sublimehq/sublime_merge/issues/692)).

**Praise:** speed, keyboard nav ([mjtsai 2018](https://mjtsai.com/blog/2018/09/20/new-git-client-sublime-merge)); "user-friendly graph that helps me easily understand what's happening" ([G2](https://www.g2.com/products/sublimerge/reviews), via search summary; page not opened).

## 2. Contrast tools (only lanes / identity / authorship)

| Tool | Too many lanes | Which line is which | Who did what |
|---|---|---|---|
| **GitKraken** | **Solo** (right-click; hides all non-soloed) and **Hide** per branch; *Smart Branch Visibility* auto-shows checked-out branch, its target and upstreams ([docs](https://help.gitkraken.com/start-here/interface), [feedback](https://feedback.gitkraken.com/suggestions/348581/comment/772679)) | **Hover a branch → related commits highlighted, unrelated dimmed** (toggleable) ([docs](https://help.gitkraken.com/start-here/interface)) | Avatars on graph nodes ([feature page](https://gitkraken.com/features/commit-graph)) |
| **Fork** | Collapse/expand merges by clicking tip or ←/→; "collapse all" then expand one; pitched as "make sense of a messy graph" on Swift repo ([blog](https://fork.dev/blog/posts/collapsible-graph/)) | **Branch labels drawn in their lane's colour** ([Win 1.38](https://git-fork.com/blog/posts/forkwin-1.38/), [Mac notes](https://fork.dev/releasenotes)) | Gravatar avatars (same Win post) |
| **VS Code Git Graph** | Branches dropdown (pick / glob); "Only Follow First Parent"; remotes toggle ([README](https://github.com/mhutchie/vscode-git-graph)) | **Mute commits not ancestors of HEAD**; mute merges (README) | Optional avatars; author column toggle (README) |
| **GitLens graph** | Hide branch/tag/remote; **Focus Branch**; fold lanes "to collapse inactive branches and reduce visual clutter"; minimap ([docs](https://help.gitkraken.com/gitlens/gl-commit-graph/)) | Pinned branch pill → bright lane to fork point, others muted ([release notes](https://help.gitkraken.com/gitlens/gitlens-release-notes-current/), via search summary) | **Author column at min width shows avatars instead of text** (docs) |
| **JetBrains log** | **Collapse Linear Branches** → dotted line per branch, per-branch click; Branch filter ([docs](https://www.jetbrains.com/help/phpstorm/log-tab.html)) | Highlight: **current branch light-blue bg**, merges greyed, not-cherry-picked greyed (docs) | **My commits bold**; User filter (docs) |
| **lazygit** | unconfirmed | unconfirmed | **Initials when narrow, full name when wide**; `gui.commitAuthorFormat` forces short ([#3624](https://github.com/jesseduffield/lazygit/issues/3624)); per-author colours via `authorColors` ([discussion](https://github.com/jesseduffield/lazygit/discussions/2307)); complaint: 3-part names get only 2 letters ([#5670](https://github.com/jesseduffield/lazygit/issues/5670)) |
| **tig** | not researched — unconfirmed | — | — |

## 3. Technique → tools → evidence

| Technique | Tools | Users like / dislike |
|---|---|---|
| Fold merges to first parent by default, expand on demand | SM, Fork, (VS Code first-parent load) | Liked: SM devs/Fork blog frame it as the clutter fix. Disliked: kernel user must scroll to see a merge's commits ([#2009](https://github.com/sublimehq/sublime_merge/issues/2009)) |
| Show-only-this (Alt+Click / Solo / Focus) | SM, GitKraken, GitLens | SM user: hiding others is "not very comfortable" as a way to trace ([#2009](https://github.com/sublimehq/sublime_merge/issues/2009)) |
| Auto-scope to current branch + target + upstream | GitKraken Smart Branch Visibility | GitKraken team concedes it's not manual hide ([feedback](https://feedback.gitkraken.com/suggestions/348581/comment/772679)); praise unconfirmed |
| Highlight selected/hovered branch, dim the rest | GitKraken, GitLens, VS Code (non-HEAD mute), JetBrains (current branch bg) | **Requested** in SM ([#2009](https://github.com/sublimehq/sublime_merge/issues/2009)) — strongest gap in SM; praise for others unconfirmed |
| Ref label colour = lane colour | Fork | SM users confused when they don't match ([forum](https://forum.sublimetext.com/t/branch-graph-colorization-not-matching/47902)) |
| Lane compaction / reuse | none found (SourceGit request cites GitExtensions — [#740](https://github.com/sourcegit-scm/sourcegit/issues/740), unconfirmed) | SM width complaint open since 2018 ([#19](https://github.com/sublimehq/sublime_merge/issues/19)) |
| Collapse linear runs to a dotted segment | JetBrains | unconfirmed |
| Avatars | GitKraken, Fork, VS Code, GitLens | none found in a terminal tool; reception unconfirmed |
| Initials + per-author colour | lazygit | Wanted (#3624 closed by config); 2-letter collisions complained (#5670) |
| "My commits" bold | JetBrains | unconfirmed |

## Open questions
- Does SM have any first-parent-only filter beyond folding? Not found.
- Any source showing users praising hover-dim highlighting specifically? Not found.
- tig and GitUp not researched.
