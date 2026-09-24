# Staging, committing and stash: prior art

Pipeline step 2, done **late**: these features shipped in M3 without a study (`docs/reviews/m6-usability.md` flagged it).
Scope: staging whole files, hunks and lines; writing the message, amend, and which key commits vs moves to the body;
stash save, apply/pop and drop.

Sources are primary (vendor docs, source code, issue trackers). `(?)` = not confirmed. Counts use the Graft rule
(`docs/research/ux-principles.md:29-36`): one chord = 1, a typed message = 1, movement not counted when the target is
under the cursor. Access failures and gaps: §5.

---

## 1. Per tool: keys and counted journeys

Journey A = "I've edited files, want them all in one commit, subject only". Start: tool open, focus on its changes
list. Journey B = stash round trip: save everything, later pop it back. Line-level staging is noted per tool, not
counted (it depends on selection size).

### 1.1 lazygit (terminal)

- Files panel: `Space` "Toggle staged for selected file", `a` stage all, `c` commit, `C` commit with git editor, `A`
  amend last commit, `Enter` opens the staging view for hunks/lines. Staging view: `a` "Toggle line-by-line vs. hunk
  selection mode", `Space` toggle staged, `v` range select. Stash: files panel `s` "Stash all changes", `S` stash
  options (all / staged / unstaged); stash panel `Space` apply, `g` pop, `d` drop.
  https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md (confirmed)
- **Confirm key, confirmed from source:** in the **summary** field the confirm key is `SubmitEditorText` =
  `<enter>` (`pkg/config/user_config.go:1071`, bound at `pkg/gui/controllers/commit_message_controller.go:~33`). `Tab`
  (`TogglePanel`) moves to the **description**, where Enter is a newline and commit is `ConfirmInEditor` =
  `<ctrl+enter>`, `<meta+enter>` on macOS, alt `<ctrl+s>` (`user_config.go:1039-1040`,
  `commit_description_controller.go:~37`). So: **Enter commits from the subject; Tab reaches the body.**
- Pasting multi-line text into the summary: the first newline used to commit; now a paste's first newline moves to the
  description (https://github.com/jesseduffield/lazygit/pull/4234, confirmed from the PR title/summary).
- Stash apply, pop and drop each ask for confirmation: `ConfirmIf(!SkipStashWarning …SureApplyStashEntry)`
  (`pkg/gui/controllers/stash_controller.go:119-122`), `SurePopStashEntry` (`:153-155`), `SureDropStashEntry`
  (`:165-167`). Apply's prompt can be switched off (`gui.skipStashWarning`); pop's and drop's are unconditional in
  this code.
- **Journey A:** `a` (stage all) → `c` → type subject → `Enter` = **4**.
- **Journey B:** `s` → type name `(?)`: whether `s` prompts for a name not read → `Enter`; later `5`/click to the stash
  panel `(?)` → `g` → `Enter` (confirm) = **~6**.
- **Complaints:**
  - Commit-from-body key clashes with the terminal: Alt+Enter toggles fullscreen in Windows Terminal
    (https://github.com/jesseduffield/lazygit/issues/1700, /issues/4530; Ctrl+Enter proposed as "a common UI pattern
    on the web"). Now default Ctrl/Cmd+Enter per source above.
  - Multi-line description cut after the first line break (https://github.com/jesseduffield/lazygit/issues/2597, closed
    as bug).
  - Description lost after a failed hook (https://github.com/jesseduffield/lazygit/issues/2679, title only `(?)`).
  - Dropped a stash instead of popping; wants `z` to undo the drop (https://github.com/jesseduffield/lazygit/issues/4491,
    open).
  - "Stash staged changes" makes two stashes, sometimes errors, leaves files indexed
    (https://github.com/jesseduffield/lazygit/issues/3333).
- **Praise:** no primary source read `(?)`.

### 1.2 Magit (Emacs)

- `s` stage / `u` unstage "the change at point" — a file, a hunk, or the **region** (selected lines) with the same key;
  `S` stage all modified, `U` unstage all. https://docs.magit.vc/magit/Staging-and-Unstaging.html (confirmed). The
  manual calls this an advantage over git's modal `git add --interactive` (vendor claim).
- Commit: `c c` commit, `c a` amend, `c e` extend (amend, no message edit), `c w` reword. The message is a buffer:
  **Enter is always a newline**; `C-c C-c` finishes, `C-c C-k` cancels; `M-p`/`M-n` cycle the commit message ring
  (saved drafts). https://docs.magit.vc/magit/Committing.html (confirmed by grep of the page, lines ~405-438).
- Stash: `z` prefix; `z z` stash index and worktree, `z i` index only, `z a` apply, `z p` pop, `z k` drop.
  https://docs.magit.vc/magit/Stashing.html (confirmed). Pop removes the stash only "on complete success". Whether
  single drop asks to confirm: not stated `(?)`.
- **Journey A:** `S` → `c` → `c` → type → `C-c C-c` = **5**.
- **Journey B:** `z` → `z` → type message `(?)` whether prompted → `Enter` … later `z` → `p` → pick stash (`Enter`,
  default top `(?)`) = **~6**.
- **Praise/complaints:** none read from a primary source `(?)`.

### 1.3 Sublime Merge

- Stage file / hunk: buttons "Stage file", "Stage hunk"; lines: select them and "Stage lines".
  https://www.sublimemerge.com/docs/getting_started (confirmed). Keyboard: `Tab` moves between parts, `Space`
  expands, `Enter` stages/unstages hunks — seen in a search snippet only `(?)`.
- Commit: message box at top, then the Commit button; `Ctrl+Enter` commits — search snippet only, the default keymap
  file was not read `(?)`. Amend: dropdown next to Commit → "Amend Previous Commit".
  https://www.sublimemerge.com/docs/getting_started (confirmed).
- Stash: docs name stashes in the Locations column but give no steps (confirmed absent from the getting-started page).
- **Journey A (mouse):** click "Stage all" `(?)` → click message → type → `Ctrl+Enter` = **~4**, plus H.
- **Complaint:** no amend shortcut; the answer is a hand-written keymap entry
  `"ctrl+shift+enter" … "commit --amend"` (https://forum.sublimetext.com/t/amend-commit-keyboard-shortcut/50337,
  2020-03-29).

### 1.4 JetBrains (IntelliJ IDEA)

- `Ctrl+K` opens the Commit tool window with the active changelist selected; `Ctrl+Enter` commits from the message
  field; `Ctrl+Alt+K` commit and push; **Amend** is a checkbox. Partial commits by **gutter checkboxes** per chunk and
  line — no staging area by default (changelists instead).
  https://www.jetbrains.com/help/idea/commit-and-push-changes.html (confirmed, 2026.2 docs).
- Stash: Commit window → Git | Stash Changes → dialog: message, "Keep index" → Create Stash. Unstash dialog: Apply or
  Pop; Drop / Clear from the list. Also "Shelve" (IDE-only patches, `Ctrl+Shift+H` shelve silently).
  https://www.jetbrains.com/help/idea/shelving-and-unshelving-changes.html (confirmed).
- **Journey A:** `Ctrl+K` → type → `Ctrl+Enter` = **3** (all changes are ticked by default in the active changelist).
- **Journey B:** menu Git → Uncommitted Changes → Stash (≥3 clicks `(?)` exact path) → type → Create = **~5**; unstash
  via menu → pick → Pop → Apply = **~5**. Stash is the weak journey here.
- **Complaints:** stash vs shelve confusion when the staging area is enabled
  (https://intellij-support.jetbrains.com/hc/en-us/community/posts/360010269140, title only; not fetched `(?)`).

### 1.5 VS Code (built-in Git; GitLens)

- Stage file: hover `+`, or right-click; stage all: `+` on the Changes header. Lines: select in the diff editor →
  gutter Stage button or "Stage Selected Ranges" (`Ctrl+K Ctrl+Alt+S`, `extensions/git/package.json` keybindings).
  Amend: Commit dropdown → "Commit (Amend)". Long messages: `git.useEditorAsCommitInput`.
  https://code.visualstudio.com/docs/sourcecontrol/staging-commits (confirmed).
- **Confirm key, from source:** `scm.acceptInput` is `Ctrl/Cmd+Enter`
  (`src/vs/workbench/contrib/scm/browser/scm.contribution.ts:420-425`, main). **Enter in the box is a newline.**
- Smart Commit: with nothing staged, commit stages all and commits (docs, confirmed).
- Stash: palette only — `git.stash`, `stashIncludeUntracked`, `stashStaged`, `stashPop`, `stashPopLatest`,
  `stashApply`, `stashDrop`, `stashDropAll`, `stashView` (`extensions/git/package.json`, confirmed); no default keys.
- **Journey A:** `Ctrl+Shift+G` → type → `Ctrl+Enter` (smart commit on) = **3**.
- **Journey B:** palette → "Git: Stash" → `Enter` → type → `Enter`; later palette → "Pop Latest" → `Enter` = **~6**.
- **Complaint:** staged a selected range, commit took the whole file
  (https://github.com/microsoft/vscode/issues/52367, 2018, closed needing info) — the smart-commit/partial-stage
  interaction is a trust issue.
- GitLens: not studied `(?)`.

### 1.6 GitUp (macOS)

- Advanced Commit view: `S` stage all, `Return` stage/unstage the selected file(s) **or line(s)** (select lines by
  clicking the line-number margin), `U` unstage all, `Tab` to the message, **`Return` in the message is a newline,
  `Opt-Return` commits**, `A` toggles amend, `Cmd-Z` undoes a commit.
  https://github.com/git-up/GitUp/wiki/Using-GitUp-Advanced-Commit-View (confirmed).
- Stashes view: `S` save, `Return` apply, `Delete` drop; **no pop** ("apply the stash first then delete it"); a drop is
  undone with `Cmd-Z`. https://github.com/git-up/GitUp/wiki/Using-GitUp-Stashes-View (confirmed).
- **Journey A:** `S` → `Tab` → type → `Opt-Return` = **4**.
- **Journey B:** `S` → type `(?)` → `Return`; later `Return` (apply) → `Delete` = **~5** (view switch not counted `(?)`).

### 1.7 jj (Jujutsu)

- "There's no index (staging area). Because the working copy is automatically committed…"; partial staging becomes
  `jj split`; amend becomes `jj squash` / `jj squash -i`. https://docs.jj-vcs.dev/latest/git-comparison/ (confirmed;
  the old jj-vcs.github.io URL 301s there). Stash equivalent not on that page `(?)` — commonly `jj new` on another
  parent, unconfirmed.
- **Journey A:** `jj commit -m "…"` = **1** command. The model removes staging, and with it the "did I stage it?" doubt.

### 1.8 Fork, Tower

Not studied `(?)`.

---

## 2. What the tools agree on

- **Enter vs commit.** Two camps, both confirmed from source/docs:
  - *Subject is a one-line field, Enter commits:* lazygit (summary). Body is a separate field reached by `Tab`.
  - *Message is a text area, Enter is a newline, a chord commits:* VS Code and JetBrains `Ctrl+Enter`, GitUp
    `Opt-Return`, Magit `C-c C-c`, Sublime Merge `Ctrl+Enter` `(?)`.
  - lazygit's own history shows the chord camp's cost in terminals: the Alt+Enter chord was eaten by the terminal
    (#1700, #4530). Ctrl+Enter has the same risk without the kitty protocol (Graft already records this,
    `docs/design-notes/m3-working-tree-undo-and-stash.md:119-121`).
- **One key toggles, the unit follows the cursor/selection.** Magit `s`/`u`, lazygit `Space`, GitUp `Return`: file,
  hunk or lines by what is under the cursor or selected.
- **Stage-all is one key.** lazygit `a`, Magit `S`, GitUp `S`; VS Code/JetBrains skip it (smart commit / all ticked).
- **Pop is not universal.** GitUp has none (apply + delete); Magit's pop keeps the stash if apply wasn't clean.
- **Drop is guarded one of two ways:** confirm (lazygit, `stash_controller.go:165`) or undo (GitUp `Cmd-Z`). The
  lazygit user in #4491 asks for undo instead.

---

## 3. What Graft does today (from design docs only; source not read)

- Staging lives in the detail pane of the Uncommitted row: `Space` stages/unstages the file, hunk or line under the
  cursor, `s` alias, `Ctrl+A` stage all, `d` discard with confirm and undoable backup (`docs/design/02-keymap.md:225-240`;
  `docs/design-notes/m3-working-tree-undo-and-stash.md:142-150`). Hunk staging in untracked files deferred
  (`m3…:131`).
- `c` / `a` commit / amend, global (`02-keymap.md:183-184`). Commit editor: Subject, Body, options row with
  `[ Commit ]` button; **Enter in Subject moves to Body**; `Ctrl+Enter` commits; `Tab` subject → body → options;
  `Ctrl+Enter` unreadable in Zed/VS Code terminals, so the fallback is `Tab` to `[ Commit ]` + `Enter`
  (`m3…:119-121`, `:159-166`; `02-keymap.md:361-370`).
- **Inconsistency, as a question:** the M6 finish screen says `Enter` commits "in the file list and the subject; in the
  body it is a newline" (`02-keymap.md:321`), while the M3 editor says Enter in Subject moves to Body
  (`m3…:165`). Are these one editor or two? Which rule is current? Not checked in source.
- Stash overlay `S`: `s` save (prompt: message, `Tab` toggles all-incl-untracked / staged-only, `Enter` saves),
  `Enter` apply, `p` pop, `d` drop (confirm, and undoable), `Space` preview, `Esc` close
  (`02-keymap.md:350-359`; `m3…:168-175`).
- Counted from the docs (start: graph, cursor on the Uncommitted row). Questions marked:
  - **A:** `Enter` (open pane) → `Ctrl+A` → `c` → type → `Enter` (to body) → `Tab` ×N `(?)` → `Enter` on
    `[ Commit ]` = **≥7** in Zed/VS Code. With a kitty-protocol terminal: `…type → Ctrl+Enter` = **5**. Whether `c`
    works on the collapsed row without `Enter` first `(?)`: `02-keymap.md:189-190` says `d`/`s` need the pane open,
    silent on `c`.
  - **B:** `S` → `s` → type (optional) → `Enter` → `Esc` `(?)` whether save closes the overlay; later `S` → `p` →
    `Esc` `(?)` = **~6-7**. Drop costs a confirm and is also undoable — both guards at once.

---

## 4. Best counts to judge against

| Journey | Best found | Tool (source) | Graft today (docs) |
|---|---|---|---|
| Stage all + commit, subject only | 3 (1 for CLI) | JetBrains `Ctrl+K`, type, `Ctrl+Enter`; VS Code smart commit (§1.4, §1.5); jj 1 command (§1.7) | ≥7 in Zed/VS Code; 5 with kitty keys |
| Same, terminal TUI | 4 | lazygit `a c` type `Enter` (§1.1) | ≥7 |
| Commit with a body | 5 | lazygit `c` type `Tab` type `Ctrl+Enter` (§1.1) | not counted `(?)` |
| Amend, no message change | 2 `(?)` | Magit `c e` (§1.2) | `a` + editor steps `(?)` |
| Stage one hunk | 1 key at cursor | Magit `s`, lazygit `Space` in staging view, GitUp `Return` | 1 at cursor, after opening the pane |
| Stash save | 2-3 | GitUp `S`; lazygit `s` (+ name `(?)`) | 3-4 |
| Stash pop | 2 | lazygit `g` + confirm; GitUp has no pop | 2 + open/close overlay |
| Stash drop guard | undo, no confirm | GitUp `Cmd-Z` (§1.6); asked for in lazygit #4491 | confirm **and** undo |

---

## 5. Gaps and access

- Not read: Sublime Merge default keymap file (docs page has no list); GitLens; Fork; Tower; GitUp Simple Commit view;
  Magit and JetBrains user praise/complaints (no primary pages fetched; the JetBrains support forum returned 403 on
  earlier research, not retried).
- Praise is thin across the board: vendor pages describe, issue trackers complain. No user-praise source was read.
- Line numbers marked `~` in lazygit were derived from an offset of a `sed` range, not read directly.
