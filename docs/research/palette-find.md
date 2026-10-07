# Prior art: one "find anything" box (palette find)

Pipeline step 2 for M8 palette find. Researched 2026-10-07.

## Already covered — not repeated here

- `docs/research/keymap-help-remap.md` covers the palette as an **action/help** surface: lazygit's ask for a fuzzy contextual `?` (line 19), VS Code / Sublime Merge palette and keymap files (lines 45, 48), "palette rows show their key" as a take (line 83).
- Its citation `docs/design/02-keymap.md:480-487` is **stale**; the palette is now `docs/design/02-keymap.md:528-542`. Current design: `Ctrl+P`/`:` opens on "actions, then branches, tags, files, commits, then recently run commands" (`02-keymap.md:534`); filter by label + hidden words (`:539`).
- This file covers the **navigation** half: jumping to a branch, tag, commit or file.

## Per tool

### VS Code — Quick Open + GitLens
- Quick Open `Ctrl/Cmd+P` fuzzy-opens files by name; prefixes switch mode: `?` help, `>` commands, `@` symbols in file, `#` workspace symbols, `:` line — [tips-and-tricks](https://code.visualstudio.com/docs/getstarted/tips-and-tricks).
- Recent: pressing the shortcut again cycles recently opened files — same page. Empty-box shows recent files first: **Question** (not stated on fetched pages).
- Git objects are **not** in Quick Open. GitLens adds commit search with prefixes: message plain, `@author`, `#sha`, `:path/glob`, `~patch` — [GitLens features](https://help.gitkraken.com/gitlens/gitlens-features/).
- Per-keystroke cancel on big repos: **Question** (not documented on fetched pages).

### JetBrains — Search Everywhere
- Double `Shift`; one entry point for files, actions, classes, symbols, settings "and anything in Git"; opens on **recent files** by default; results in **tabs**; the **Git tab** (branches, commits, tags) must be enabled in Advanced Settings — [searching-everywhere](https://www.jetbrains.com/help/idea/searching-everywhere.html).
- Complaints — latency: slow after 2022.3, workaround is disabling "Wait for all contributors to finish before showing results" — [IJPL-131511](https://youtrack.jetbrains.com/issue/IDEA-308324); 25 s for `file:line` — [IJPL-56316](https://youtrack.jetbrains.com/issue/IDEA-245379); "Searching..." lag even on 10 files — [IDEA-264834](https://youtrack.jetbrains.com/issue/IDEA-264834); UI freeze — [IJPL-132381](https://youtrack.jetbrains.com/issue/IDEA-200312).
- Lesson: streaming per-source results beats waiting for all sources (the workaround above).

### Sublime Merge — command palette
- `Ctrl/Cmd+P` palette; "Navigate to branch" and "Checkout branch" are palette commands, i.e. **verb first, then pick** — [forum: navigate to branch](https://forum.sublimetext.com/t/navigate-to-branch-and-other-newbie-questions/67469); `navigate_to_commit` takes a partial/full SHA — [forum: navigate to commit](https://forum.sublimetext.com/t/navigate-to-commit-command/51352).
- Palette entries are `caption` + `command` + `args` — [command_palette docs](https://www.sublimemerge.com/docs/command_palette).
- Commit-search syntax (author:, path: etc.): **Question** — fetched getting_started page did not show it.

### lazygit — per-panel `/`
- `/` **filters** most views but **searches** (highlight, `n`/`N`) the commits view, "since users typically need to see commits before and after a match"; `Ctrl+S` / `-f path` filters commits by path — [Searching.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Searching.md).
- No global box: you go to the panel first, then `/`.
- Complaints: "Fuzzy filtering is too fuzzy" — 4000 remote branches, `sh-` matched ~800 unrelated; asks substring per space-separated word — [#3373](https://github.com/jesseduffield/lazygit/issues/3373). Wants git-side log search, not only loaded rows — [#3279](https://github.com/jesseduffield/lazygit/issues/3279). Fuzzy file picker for file history — [#1525](https://github.com/jesseduffield/lazygit/issues/1525). Search branches by commit hash — [#3265](https://github.com/jesseduffield/lazygit/issues/3265).

### Magit
- Checkout prompts via Emacs completing-read over refs. Complaint: alphabetical order, asks for `--sort=-committerdate` — [#2872](https://github.com/magit/magit/issues/2872); MRU order asked on the list — [magit group](https://groups.google.com/g/magit/c/FEgncBX59WQ); remote branches not offered by default — [#3264](https://github.com/magit/magit/issues/3264). Manual: [Branch Commands](https://docs.magit.vc/devel/magit/Branch-Commands.html).

### fzf tools — forgit, git-fuzzy
- forgit: one command per object+verb (`gcb` branches to checkout, `gsw` switch, `glo` log, `gco` commit checkout, `gcf` files…) — [forgit](https://github.com/wfxr/forgit).
- git-fuzzy: status, log, branch, diff, reflog, stash; log typing "filters in the usual fzf style" — client-side over pre-fetched lines, no git call per keystroke — [git-fuzzy](https://github.com/bigH/git-fuzzy).

### tig
- `/`, `?` regex search within a view, `n`/`N`; refs view `r`; grep view `g`; **no fuzzy finder** — [tig manual](https://jonas.github.io/tig/doc/manual.html).

### GitUp
- Claims "instant search across the entire repo including diff contents" — [GitUp README](https://github.com/git-up/GitUp). Ranking/grouping: **Question**.

## Journeys (step counts — mine, from the documented behaviour above)

| Task | JetBrains | VS Code | Sublime Merge | lazygit | forgit |
|---|---|---|---|---|---|
| Jump to commit by message words | `⇧⇧`, Git tab, type, `Enter` = 4 (tab must be enabled once) | palette, GitLens search, type, `Enter` = 4 | Q: search syntax unconfirmed | focus commits, `/`, type, `Enter`, `n`… = 4+ | `glo`, type, `Enter` = 3 |
| Open a file's history | `⇧⇧` file, open, history action = 4+ | Quick Open file + GitLens history = 4+ | Q | `Ctrl+S`, type path, `Enter` = 3 | n/a |
| Switch to a branch used yesterday | Git tab, type, `Enter` = 3–4; MRU: **Q** | not in Quick Open | palette, "checkout branch", type, `Enter` = 4 | `b` panel, `/`, type, `Space` = 4 | `gsw`, type, `Enter` = 3 |

## Big repos
- Filter-in-memory (git-fuzzy, lazygit) is instant but only sees what was loaded (lazygit [#3279](https://github.com/jesseduffield/lazygit/issues/3279)).
- Multi-source wait = lag (JetBrains issues above). Per-keystroke cancellation behaviour in any tool: **Question**, no primary source found.

## Take / reject candidates for Graft
- **Take** one box, grouped by kind (JetBrains tabs; design already orders kinds, `02-keymap.md:534`) — show group headers.
- **Take** recent items on an empty box (JetBrains default); branches ordered by recency, not alphabet (Magit #2872 complaint).
- **Take** substring-per-word over loose fuzzy for refs (lazygit #3373); rank prefix/word matches first.
- **Take** stream results per source; never block on the slowest (IJPL-131511). Cancel stale queries per keystroke — Graft's own design, no prior art confirmed.
- **Take** short prefixes to narrow a kind (VS Code `>`/`@`, GitLens `@`/`#`/`:`) — must not collide with `:` opening the palette (`02-keymap.md:534`).
- **Reject** verb-first navigation (Sublime Merge "Navigate to branch", forgit per-verb commands): object first, then the action keys of the place it lands in.
- **Reject** opt-in Git tab (JetBrains): git objects are the point here.
- **Question** commit search over the full history vs loaded rows only — decide in the design note.
