# Blame, file history and line history: how the best-loved tools do it

Researcher, 2026-09-26. Feature pipeline step 2 for M7. Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I ran none of these tools. Step counts come from
their docs; where the docs leave a step out I say so and mark it **(?)**. Claims about Graft carry a repo `file:line`.
Sites that refused or 404'd are listed at the end.

**Counting rule:** one step = one key chord, click or menu pick (`docs/research/ux-principles.md` §1.3). Counts start
with the file already open on screen and the cursor on the line.

**Why this matters to the user.** Blame and file history are weekly-ish for him, not daily (memory:
graft-daily-workflows). `docs/field-notes.md` has no entry on blame or file history yet, so there is no first-hand pain
to design against. **Question for step 1:** what does he reach for today when he asks "why is this line like this"?

**What Graft has already fixed:** `docs/design/02-keymap.md:367-374` allocates the blame keys: `Enter` jumps to the
commit in the graph, `n`/`N` older/newer change to this line, `^` blames the parent revision, `Space` toggles the age
heatmap. The spec asks for "File history that follows renames; line-range history (`git log -L`)" and "Blame with a
per-line age heatmap and jump-to-commit" (`PROMPT.md:300-301`).

---

## 0. The git layer everyone builds on

Source: https://git-scm.com/docs/git-blame, https://git-scm.com/docs/git-log

- `--ignore-rev` / `--ignore-revs-file` / `blame.ignoreRevsFile`: "Ignore changes made by the revision when assigning
  blame, as if the change never happened." Lines are passed to "the previous commit that changed that line or nearby
  lines". `blame.markIgnoredLines` marks re-attributed lines with `?`, and `blame.markUnblamableLines` marks lines it
  could not attribute with `*`.
- `-M` finds lines moved within a file and `-C` finds lines moved across files (repeat it for wider search). These are
  what make "whitespace/move" commits drop out without an ignore file.
- `git blame -L :funcname` limits blame to one function, found the same way diff hunk headers are.
- `git log --follow`: follows renames, "works only for a single file".
- `git log -L <start>,<end>:<file>` / `-L :<funcname>:<file>`: "Trace the evolution of the line range". It
  "Implies `--patch`", allows no pathspec, allows at most one starting revision, and the range "must exist in the
  starting revision". Diff-stat formats are "not currently implemented".

## 1. The tools

### 1.1 JetBrains IDEs: Annotate with Git Blame, Show History for Selection

Source: https://www.jetbrains.com/help/idea/investigate-changes.html

- **Open blame:** right-click the gutter, then *Annotate with Git Blame*. It has no default shortcut; you assign one in
  Keymap.
- **Commit:** click the annotation. That jumps to the commit in the Log tab, and a popup shows the commit details.
  *Show Diff* is `Ctrl+D` from the annotation.
- **Before that commit:** right-click the annotation, then *Annotate Previous Revision*. The docs name its use: when
  "all that was changed is code formatting".
- **Ignore noise:** annotation *Options*: Ignore Whitespaces (`-w`), Detect Movements Within File (`-M`), Across Files
  (`-C`). The page does not mention `.git-blame-ignore-revs` **(?)**.
- **Line/function history:** select the code, then *Git | Current File | Show History for Selection*. With nothing
  selected, it shows the current line.
- **Counts:** Task 1: 2 (open blame) + 1 (click) + 1 (`Ctrl+D`) = **4**. Task 2: 2 + 2 = **4**. Task 3: select the
  function (1–2) + menu (3) = **~5**.

### 1.2 GitLens (VS Code)

Source: https://help.gitkraken.com/gitlens/gitlens-features/

- **Blame is on all the time:** "Current Line Blame adds a subtle ... annotation at the end of the active line". Hovers
  offer commands. There are also file blame annotations with a heatmap, and a file history view.
- **Before that commit:** the *Open Blame Prior to Change* command (`gitlens.openBlamePriorToChange`) is named in a
  third-party mirror (https://gitlens.amod.io/). I could not confirm it on an official page **(?)**.
- **Counts:** Task 1: 0 (already inline) + hover + 1 click (open changes) = **~2** **(?)** because the hover
  commands' names were not confirmed. Task 2: **~2** via the hover **(?)**. Task 3: the Line History view (selection
  driven) = **~2** **(?)**.
- **Loved:** "The Current Line Blame feature has been highly valued" (see #3066 below). Having zero steps to "who
  and when" is what people praise.
- **Complaints:** the inline blame is noisy, and hard to fully turn off:
  https://github.com/gitkraken/vscode-gitlens/discussions/1513,
  https://github.com/gitkraken/vscode-gitlens/issues/1320 ("No way to disable hover"),
  https://github.com/gitkraken/vscode-gitlens/issues/2234 (CodeLens blame shows even when disabled). It is also wrong
  after edits: https://github.com/gitkraken/vscode-gitlens/issues/1965. Large files:
  https://github.com/gitkraken/vscode-gitlens/issues/3066 ("Dirtying Large File Indefinitely Suspends Git Line Blame").

### 1.3 Sublime Merge

Source: https://www.sublimemerge.com/docs/command_line (the docs page on file history returned 404, see end)

- `smerge blame file [line]` and `smerge log file`. The home page says File History and Hunk History "follow any file
  moves or renames" (https://www.sublimemerge.com/). The only source for this was a search summary; I did not read the
  page myself **(?)**.
- There is a line history of the selected lines **(?)**, per the search summary only.
- **Counts:** not confirmed from primary docs **(?)**.

### 1.4 tig

Source: https://raw.githubusercontent.com/jonas/tig/master/doc/manual.adoc

- `b` opens blame from any view, or `tig blame <file>` from the shell. `-L` is supported.
- `,` "In the blame view it will load blame for the parent commit": this is blame-before-this-commit in one key.
- `q` goes back through the view stack.
- **Counts:** Task 1: `b` + `Enter` on the line (opens the commit/diff view) = **2** (the `Enter` step is **(?)**, as
  the fetched summary did not state it). Task 2: `b` + `,` = **2**. Task 3: there is no `log -L` view **(?)**; you
  leave for the shell.

### 1.5 lazygit

Source: https://github.com/jesseduffield/lazygit/discussions/1690

- There is **no built-in blame view**: "the wiki points to setting a custom command that launches tig". lazygit does
  call `git blame` internally, for finding a fixup base
  (https://github.com/jesseduffield/lazygit/blob/master/docs/dev/Find_Base_Commit_For_Fixup_Design.md).
- The commit log can be filtered (`<ctrl+s>` "View filter options",
  https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md). I did not confirm whether
  the path filter follows renames **(?)**.
- **Counts:** Tasks 1–3 all leave the app (or go through a user-configured custom command). The complaint is the
  discussion itself.

### 1.6 Magit (`magit-blame`)

Source: https://docs.magit.vc/magit/Blaming.html (magit.vc/manual returned 404)

- `magit-blame-addition` (`C-c M-g b`) blames. `magit-blame-removal` shows which revision removed a line.
  `magit-blame-reverse` shows "the last revision in which a line still existed".
- Inside blame: `RET` "shows the commit that last touched the line at point"; `b` blames the parent commit; `n`/`p`
  move to the next/previous chunk; `N`/`P` move to the same commit's chunks; `c` cycles the display style; `q` quits.
- **Counts:** Task 1: `C-c M-g b` (1 chord sequence, 3 keys) + `RET` = **2 steps / 4 keys**. Task 2: blame + `b` =
  **2**. Task 3: `magit-log-trace-definition` or `L` in the log popup **(?)**, not confirmed on the fetched page.
- Magit is the only tool here that offers **removal** and **reverse** blame ("where did this line go?").

### 1.7 GitHub web blame

Source: https://docs.github.com/en/repositories/working-with-files/using-files/viewing-and-understanding-files

- Open the file and click **Blame**. A "versions" icon on each hunk shows the file "before a specific commit was
  applied". This is the blame-prior button, 1 click, and it is repeatable.
- It reads `.git-blame-ignore-revs` from the repo root **automatically**, and shows a banner: "Ignoring revisions in
  .git-blame-ignore-revs". You bypass it with `~` on the SHA in the URL. Caveat: "If the commit was the last to
  modify a line, it will still appear in blame."
- **Counts:** Task 1: Blame (1) + commit message (1) = **2**, and the full diff is on that page. Task 2: **2**.
  Task 3: no line-range history **(?)**.
- GitLab users ask for the same "re-blame" button: https://gitlab.com/gitlab-org/gitlab/issues/34713.

### 1.8 delta

Source: https://dandavison.github.io/delta/git-blame.html

- delta is a **pager** for `git blame`. It formats the output and, with hyperlinks on, makes commit hashes links to the
  forge. It has no navigation. It is a model for **how blame looks**, not for the journey.

### 1.9 Fork

Source: https://git-fork.com/ (fetched only through a search summary)

- "With blame view you can find the last commit which changed a particular file line. With history view you can find
  all commits where a particular file or directory was changed." The primary site gives no key or step detail
  **(?)**.

## 2. Step-count table

| Tool | 1. line → message + diff | 2. line before that commit | 3. every change to this function |
|---|---|---|---|
| JetBrains | 4 | 4 | ~5 |
| GitLens | ~2 (?) | ~2 (?) | ~2 (?) |
| tig | 2 (?) | 2 | leaves the app (?) |
| Magit | 2 | 2 | (?) |
| GitHub | 2 | 2 | none (?) |
| lazygit | leaves the app | leaves the app | leaves the app |
| Sublime Merge, Fork | (?) | (?) | (?) |

The best confirmed counts are **2 / 2**, from tig, Magit and GitHub. Nobody documents Task 3 in fewer than ~2 steps
(GitLens, unconfirmed), and several tools lack it entirely.

## 3. Performance on large files and repos

- Blame cost is git's cost: "slow for large files with extensive commit history, even when limiting the line range"
  (https://gitlab.com/gitlab-org/gitlab/-/issues/586593). Another report is on a 30k-line, 25-year file:
  https://public-inbox.org/git/CABXAcUzoNJ6s3=2xZfWYQUZ_AUefwP=5UVUgMnafKHHtufzbSA@mail.gmail.com/T/.
- Always-on inline blame multiplies that cost. In Zed, opening a 1.8 MB file with ~52k commits "immediately starts
  creating Git processes which use a lot of CPU"; turning inline blame off fixed it
  (https://github.com/zed-industries/zed/issues/13547). Gitpod has an OOM kill case:
  https://github.com/gitpod-io/gitpod/issues/5110.
- libgit2 blame is reported as far slower than CLI git: https://github.com/libgit2/libgit2/issues/3027. Git
  Extensions has a similar report: https://github.com/gitextensions/gitextensions/issues/1180. Visual Studio too:
  https://developercommunity.visualstudio.com/t/Git-Blame-Annotate-is-very-slow-and-ca/10678023.

## 4. Takeaways for Graft

1. **Take: parent-blame as one key**, as tig `,`, Magit `b`, GitHub's versions icon and JetBrains' *Annotate Previous
   Revision* all do. It is already allocated as `^` (`02-keymap.md:373`). Keep the cursor on the same line content
   after the walk (**(?)** how each tool maps the line; to be decided in the design).
2. **Take: read `.git-blame-ignore-revs` / `blame.ignoreRevsFile` automatically, with a visible banner**, as GitHub
   does. Render the banner by its state (CLAUDE.md status rule), and give one key to show blame without it. Show git's
   `?`/`*` marks as text, not colour (rule 5).
3. **Take: `-w -M` on by default** (JetBrains exposes these as options). Whether `-C` should be the default too is a
   question: it costs more on large repos (§3).
4. **Take: `Enter` → the commit in the graph with its diff**, 2 steps total, matching tig, Magit and GitHub
   (`02-keymap.md:371`).
5. **Reject: always-on inline blame** in the diff/file views. The GitLens noise complaints and the Zed CPU runaway (§3)
   are the evidence. Blame runs only when asked.
6. **Take: line/function history from a selection** (JetBrains *Show History for Selection*), backed by `git log -L`.
   Respect its limits: the range must exist at the start revision, and there is no pathspec. Say so in the UI rather
   than failing silently (rule 3).
7. **Take: file history follows renames by default** (`--follow`, as Sublime Merge claims). It works for a single
   file only, which matches a per-file view.
8. **Invent (candidate): "every change to this function" in 2 steps from blame.** No confirmed tool does Task 3 in
   ≤2 steps inside a keyboard TUI. tig and lazygit leave the app, and Magit is unconfirmed. `git blame -L :funcname`
   and `log -L :funcname` give the base. The key must come from `02-keymap.md` through the conductor, not be picked in
   the build.
9. **Take (low priority): Magit's reverse blame** ("when did this line disappear"). It is unique to Magit, and whether
   the user needs it is a question for step 1.
10. **Reject: libgit2 blame** unless it is measured on a large repo first. The reports say CLI git is much faster
    (§3). Blame must run async and be cancellable. The measurement goes in `docs/perf/`, not in a helper's context.

## 5. Could not confirm / refused

- 404: https://magit.vc/manual/magit/Blaming.html (docs.magit.vc used instead), https://www.sublimemerge.com/docs/file_history.
- Sublime Merge and Fork details came through search summaries only, not pages I read.
- GitLens hover command names and the Line History view steps come from a third-party mirror, not official docs.
- tig: that `Enter` on a blame line opens the commit diff was not in the fetched summary.
- No field note exists on blame or history, so the user's own pain is unknown.
