# First-run and onboarding — prior art (M8, pipeline step 2)

Topic: what a fresh Graft install shows, and a first-run tour. Researched 2026-10-02.
Every fact links its source. **Guess** / **Question** = not confirmed. Sites that gave nothing: lazygit's Config.md via web (HTTP 503; read through `gh api` instead), GitKraken help pages (no first-launch description found), VS Code docs pages fetched (no Welcome-page text found), Fork site (no onboarding text).

## Per tool

### lazygit (closest peer: keyboard TUI Git client)
- **First launch:** one read-only popup: thanks, a link to a features video, a link to release notes, a donation note, "Press <confirm> to get started." Source: [`pkg/i18n/english.go` `englishIntroPopupMessage`](https://github.com/jesseduffield/lazygit/blob/master/pkg/i18n/english.go).
- **Shown once per popup version:** shown if stored `StartupPopupVersion` < the constant (5); else shows breaking-change notes instead. [`pkg/gui/layout.go` `onInitialViewsCreation`](https://github.com/jesseduffield/lazygit/blob/master/pkg/gui/layout.go), [`pkg/gui/gui.go`](https://github.com/jesseduffield/lazygit/blob/master/pkg/gui/gui.go).
- **Skip:** `disableStartupPopups: false` default; one key dismisses. **Replay:** none found — **Question**.
- **Ongoing:** `showRandomTip: true` — "show a random tip in the command log when Lazygit starts." [docs/Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md).
- **Real repo:** yes, popup sits over the user's repo; no sandbox.
- **Steps install→first useful action:** run `lazygit` in repo (1), dismiss popup (1) = 2. Teaching is outsourced to a video.
- **Complaints/praise:** not found in issue search — **Question**.

### helix (`:tutor`)
- **First launch:** nothing; tutor is opt-in via `hx --tutor` or `:tutor`. [docs.helix-editor.com/usage](https://docs.helix-editor.com/usage.html).
- **Form:** an editable text file, interactive — "Press the j key until you reach the introduction." [runtime/tutor](https://github.com/helix-editor/helix/blob/master/runtime/tutor). Sandbox (a copy), not user files. Replayable any time.
- **Complaints:** newcomers aren't led to it — user took 4 hops through site/wiki before finding `--tutor` ([#16032 "Unclear learning path for new users"](https://github.com/helix-editor/helix/issues/16032)); tutor shows default keys, not the user's remapped ones ([#13483](https://github.com/helix-editor/helix/issues/13483)).

### vimtutor
- ~30 min; a **copy** of the file to practise on; "you need to execute the commands to learn them properly. If you only read the text, you will forget." [runtime/tutor/tutor1](https://github.com/vim/vim/blob/master/runtime/tutor/tutor1). Separate command, never auto-shown. Praise: **Question** (widely cited as loved; no primary source fetched).

### zellij
- **Startup tips** on by default (`show_startup_tips`), reachable later via the about plugin; **release notes** on first run of a new version (`show_release_notes`). [Options docs](https://zellij.dev/documentation/options.html).
- **Onboarding wizard** asks for unlock mode (e.g. `C-g`), writes config; welcome screen is a plugin layout. [#5064 comment](https://github.com/zellij-org/zellij/issues/5064), [welcome.kdl](https://github.com/zellij-org/zellij/blob/main/zellij-utils/assets/layouts/welcome.kdl).
- **Complaints:** status-bar tips "great for newbies but at some point … useless information and wasting vertical screen space" ([#1730](https://github.com/zellij-org/zellij/issues/1730)); welcome screen hard to exit — Ctrl-C/Esc do nothing, help text doesn't show quit key ([#5064](https://github.com/zellij-org/zellij/issues/5064)).

### Sublime Merge
- Opens the **clone/new repository page** on first run. [Getting started](https://www.sublimemerge.com/docs/getting_started). No tour found — **Question**. Steps: launch, open/clone repo = 2+.

### GitUp
- No tour in README; help via Help > GitUp Help and the [wiki](https://github.com/git-up/GitUp/wiki). [README](https://github.com/git-up/GitUp). Notable: GitUp also leads with undo/snapshots as a selling point — same story Graft tells. Whether the app teaches undo on first run: **Question**.

### GitKraken
- Help center routes new users to Interface Basics and "New to Git". [Help home](https://help.gitkraken.com/gitkraken-desktop/gitkraken-desktop-home/). Sign-in on first launch and in-app onboarding: **Question** (not stated on pages fetched).

### Fork
- Site mentions only "free evaluation". [fork.dev](https://fork.dev/). Onboarding: **Question**.

### VS Code
- Extension **walkthroughs** live on the Getting Started page; steps auto-tick via `completionEvents` (`onCommand`, `onSettingChanged`, `onContext`, `onView`…); "Walkthroughs are automatically opened on install of your extension." [Contribution points](https://code.visualstudio.com/api/references/contribution-points). Welcome page on first launch and `workbench.startupEditor` to disable: **Question** (not on pages fetched).

### JetBrains
- **Features Trainer** (Learn tool window): interactive lessons; "generates a learning project" — sandbox; pick lessons, resume, reset. Opened from Welcome screen or Help | Learn IDE Features. [Docs](https://www.jetbrains.com/help/idea/feature-trainer.html).
- **Tip of the Day:** dialog with "Don't show tips on startup" checkbox; reopen via Help | Tip of the Day. [Getting help](https://www.jetbrains.com/help/idea/getting-help.html). Users ask how to kill it: [ReSharper forum](https://resharper-support.jetbrains.com/hc/en-us/community/posts/206058339-How-to-Disable-Tips-of-the-day).

### fish
- Prints `$fish_greeting` on every interactive start; disable with `set -U fish_greeting`. [fish_greeting](https://fishshell.com/docs/current/cmds/fish_greeting.html). Default text: **Question**.

### btop
- No tour. Config "auto generated if not found"; every option changeable in-UI; help and options menus; highlighted keys clickable. [README](https://github.com/aristocratos/btop). Steps to useful: 1 (launch = useful).

## Patterns

| Pattern | Who | Interactive | Sandbox | Skip/replay |
|---|---|---|---|---|
| One-shot popup, versioned | lazygit | no | real repo | 1 key / Question |
| Opt-in tutor file | helix, vimtutor | yes | copy | always replayable |
| Sandbox lesson project | JetBrains Trainer | yes | generated project | resume/reset |
| Self-ticking checklist | VS Code walkthroughs | yes (real actions) | real workspace | — |
| Rotating tips | lazygit, zellij, JetBrains, fish | no | — | config off |
| Setup wizard | zellij | yes | — | Question |
| Nothing; UI self-explains | btop, GitUp, Sublime Merge | — | — | — |

## Take / reject / invent candidates

- **Take:** lazygit's version-gated one-shot (shown once, re-shown only for breaking changes). Cite: layout.go.
- **Take:** tutor in a **sandbox repo** (helix/vimtutor copy, JetBrains learning project) — teaching undo/Ghost Preview on the user's real repo is risky; a throwaway repo makes "try a reset, then Ctrl+Z" safe.
- **Take:** VS Code-style completion by doing — a step ticks when the user performs the action, not when he reads.
- **Take:** tutor shows the user's **actual** bindings (helix #13483 complaint).
- **Take:** quit/skip key shown on the tour itself; Esc always leaves (zellij #5064).
- **Reject:** rotating tips taking a permanent row (zellij #1730) and Tip-of-the-Day dialogs (JetBrains forum).
- **Reject:** teaching via external video/links (lazygit) — leaves the terminal.
- **Reject:** setup wizard before first use — btop proves sane defaults + in-app options suffice. **Guess.**
- **Invent?** Mistake Radar / undo as the tour's hook: one staged "mistake" in the sandbox, user undoes it. Nothing found does this; GitUp sells undo but no tour found (Question).
- **Question for planner:** does a fresh install with no repo land on a repo picker (Sublime Merge) or an error? Check Graft's current behaviour in code before deciding.
