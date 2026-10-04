# Prior art: screen chrome and the empty state

Pipeline step 2 for `docs/design-notes/m8-polish-and-ship.md:51` (Screen chrome) and `:65` (Empty state), answering review M1 (`docs/reviews/m8-06-08-look.md:75`). Researched 2026-10-04.

**Method and limits.** Web search on primary sources (official docs, project issue trackers). Most claims are from search-result summaries of the linked page, not a full read of it; a row marked **?** is a question, not a fact. No site refused access. Nothing found for GitUp, gitui or tig on either topic: those are open questions below, not findings.

## 1. Chrome: header, bottom bar, gutter/cursor marks

| Tool | What it does | Praise / complaint | Source |
|---|---|---|---|
| VS Code SCM | Repo state lives in the bottom-left **status bar**: branch, dirty mark, incoming/outgoing counts; clicking the branch switches it. | Complaint: "No display or easy way to see what Repo you are in" (multi-repo). | [branches-worktrees](https://code.visualstudio.com/docs/sourcecontrol/branches-worktrees), [SCM overview](https://code.visualstudio.com/docs/sourcecontrol/overview), [vscode#306315](https://github.com/microsoft/vscode/issues/306315) |
| Magit | No separate chrome: the status buffer's **first lines** are headers: `Head:` (current branch), `Merge:`/`Rebase:` (upstream), `Push:` (push target). Configurable via `magit-status-headers-hook`. | ? no user feedback found. | [Magit manual: Status Buffer](https://docs.magit.vc/magit/Status-Buffer.html) |
| helix | One bottom **statusline**, three areas (left/centre/right); defaults put `mode` left, `file-name` centre; every element configurable. | Users ask for more elements/options (discussion). | [helix editor config](https://docs.helix-editor.com/editor.html), [helix#12924](https://github.com/helix-editor/helix/discussions/12924), [PR #2434](https://github.com/helix-editor/helix/pull/2434) |
| zellij | Bottom **status bar of key hints** by mode; `compact-bar` alternative. 0.43 added tooltip hints on a toggle key. | Complaint, quoted by zellij itself: "a much requested feature" was to see hints "without having them constantly take up valuable screen space". Bug: custom normal-mode bindings made the hints disappear. | [zellij 0.43 news](https://zellij.dev/news/web-client-multiple-pane-actions/), [zellij#4119](https://github.com/zellij-org/zellij/issues/4119), [discussion #1270](https://github.com/zellij-org/zellij/discussions/1270) |
| gitui | Bottom key-hint bar per focused panel. Cited by another TUI as the model to copy ("similar to GitUI"). | Praise (indirect): bottom's request to copy it. | [ClementTsang/bottom#1710](https://github.com/ClementTsang/bottom/issues/1710) |
| lazygit | ? Bottom options line and repo/branch in the status panel are commonly described; not confirmed from a primary source here. `@` opens command-log options. | ? | [lazygit keybindings](https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md) |
| Sublime Merge, tig, GitUp | ? No primary source found on header/bar/gutter layout. | ? | — |

Cross-cutting complaint (third-party TUIs, for the "hints from the live table" choice): hints hardcoded drift from bindings ([zo-ll/grove#17](https://github.com/zo-ll/grove/issues/17), [pane#1](https://github.com/lucasilverentand/pane/issues/1)). Weak sources: small projects.

**Gutter / cursor mark:** no primary source found on any of these tools using a gutter glyph for the cursor row. The note's `:52` row cites lazygit #4306 for reverse video; not re-checked here.

## 2. Launched outside a repo

| Tool | What it does | Praise / complaint | Source |
|---|---|---|---|
| lazygit | `notARepository`: `prompt` (init here, or open a recent repo), `create`, `skip` (open most recent), `quit`. Recent-repos list also inside the app. | Complaints when it broke: v0.43.1 exited with "must be run inside a git repository" — users wanted the repo picker ([#3795](https://github.com/jesseduffield/lazygit/issues/3795), [#3740](https://github.com/jesseduffield/lazygit/issues/3740), [#3849](https://github.com/jesseduffield/lazygit/issues/3849), [#3895](https://github.com/jesseduffield/lazygit/issues/3895)). Crash opening a recent repo that no longer exists ([#1609](https://github.com/jesseduffield/lazygit/issues/1609)). PR to skip the recent prompt ([#4667](https://github.com/jesseduffield/lazygit/pull/4667)). Old ask: better "not a git repository" handling ([#447](https://github.com/jesseduffield/lazygit/issues/447)). | [Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md) |
| GitHub Desktop | No repos → "Let's get started!": tutorial repo, clone, create new, add existing local. | ? no feedback found. | [Getting started](https://docs.github.com/en/desktop/overview/getting-started-with-github-desktop), [first repository](https://docs.github.com/en/desktop/overview/creating-your-first-repository-using-github-desktop) |
| Sublime Merge | First run opens the **clone page**; a **New Repository** button inits a folder; both open the repo when done. | Ask: recursive clone option on the new-tab dialog ([sublime_merge#1015](https://github.com/sublimehq/sublime_merge/issues/1015)). | [Getting Started](https://www.sublimemerge.com/docs/getting_started) |
| JetBrains | Welcome screen: recent projects list on the left with a search field; Clone / Get from VCS. | ? no feedback found. | [Open projects](https://www.jetbrains.com/help/pycharm/open-projects.html), [Set up a Git repository](https://www.jetbrains.com/help/idea/set-up-a-git-repository.html) |
| VS Code | Clone via `Git: Clone` or the SCM view's Clone button; recents via File › Open Recent. ? The Welcome page's own recent list was not confirmed from an official page. | Complaint (third-party): the big clone button disappears after the first clone, making a second clone hard to find. | [repos-remotes](https://code.visualstudio.com/docs/sourcecontrol/repos-remotes), [quickstart](https://code.visualstudio.com/docs/sourcecontrol/quickstart), [Heinrichs](https://jaredheinrichs.substack.com/p/how-to-clone-a-github-repo-in-vscode) (not primary) |
| GitUp | ? Not found. | ? | — |

## Verdicts

**Row `:51` — Screen chrome: T, but cite the right tools.** Repo/branch on top: take **Magit** (`Head:` / `Push:` as the first lines of the status screen). Words in the bottom bar: take **gitui's / zellij's** key-hint bar, with zellij's lesson that a full-time bar costs space users complain about. The note's "Sublime Merge / lazygit" attribution is **not backed** by anything found here; replace it or keep it as a question. The gutter `▸` stays **I** (`:52`), nothing found.

**Row `:65` — Empty state: T (lazygit `prompt` + JetBrains recent list + Sublime Merge clone/new).** lazygit's own users say what they want when launched outside a repo: the recent-repo picker, not an exit (#3795 et al.); Sublime Merge and GitHub Desktop show clone/new/open; JetBrains puts recents first with a filter. Two lessons: handle a recent repo that has since been deleted (lazygit #1609), and keep clone findable after first use (VS Code complaint). The note's "VS Code welcome recent list" is **unconfirmed**; prefer JetBrains or lazygit as the source.
