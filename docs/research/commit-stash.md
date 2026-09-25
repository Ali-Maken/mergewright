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

---

# Addendum (2026-09-25): conflicted stash pop, and auto-stash around pull and switch

For M6 picks 15–16 (`.scratch/m6-conflicts/user-picks-2026-09-24.md`): a stash pop that clashes lands on the conflict
screen; pull-with-rebase and branch switch set unsaved edits aside and put them back, a clash landing on the conflict
screen. §1–5 above cover save/pop/drop keys only; `push-pull.md` has no auto-stash coverage (grep: no hit for
"autostash"/"stash"). Not repeated here.

## A1. Git's facts that bind the design

Verified on git 2.54.0 (Apple Git-157) in a scratch repo, 2026-09-25, and against git source (master) and docs.

1. **A conflicted `pop` keeps the stash; you drop it yourself.** Docs: "Applying the state can fail with conflicts; in
   this case, it is not removed from the stash list. You need to resolve the conflicts by hand and call `git stash drop`
   manually afterwards." — https://git-scm.com/docs/git-stash. Source prints "The stash entry is kept in case you need
   it again." and only drops on success (`builtin/stash.c:902-907`, https://github.com/git/git/blob/master/builtin/stash.c).
   Observed: exit 1, `stash@{0}` still listed.
2. **Index after a conflicted apply:** clashing paths are unmerged (stages 1/2/3; `UU`); **the stash's clean paths
   are left staged** even if they were unstaged when stashed (observed: `g` stashed unstaged, came back `M ` staged).
   Anything the user had staged beforehand stays staged and is now indistinguishable from the stash's clean paths
   (observed: pre-staged `h` shown `M ` next to them). `--index` refuses outright when it can't reinstate the index:
   "this can fail, when you have conflicts" (git-stash docs); Magit hit this as "conflicts in index. Try without
   --index" — https://github.com/magit/magit/issues/4993.
3. **No `MERGE_HEAD`, no operation state.** Observed: only `HEAD`, `ORIG_HEAD` in `.git` after the conflicted pop.
   There is nothing to `--continue` or `--abort`; "done" is "no unmerged paths left", then `git stash drop`. A reader
   that detects "in a conflict" by `MERGE_HEAD`/`rebase-merge/` will **not** see this state. Conflict markers are
   labelled `Updated upstream` / `Stashed changes` (observed); VS Code keys on that label (below).
4. **Pop refuses before touching anything if a dirty file overlaps:** "Your local changes to the following files would
   be overwritten by merge … Aborting", exit 1, stash kept (observed). That is a refusal, not a conflict.
5. **Autostash (`rebase --autostash`, `rebase.autoStash`, `pull --autostash`, `merge --autostash`/`merge.autoStash`):**
   "Automatically create a temporary stash entry before the operation begins, and apply it after the operation ends …
   the final stash application after a successful rebase might result in non-trivial conflicts." —
   https://git-scm.com/docs/git-rebase, https://git-scm.com/docs/git-merge (merge records it in ref
   `MERGE_AUTOSTASH`), https://git-scm.com/docs/git-pull. All default false.
   On a clashing put-back git **stores the entry in the stash list as `autostash`** and leaves the conflict in the tree
   (`sequencer.c:4843-4868`, https://github.com/git/git/blob/master/sequencer.c; message "Your changes are safe in the
   stash. You can run "git stash pop" or "git stash drop" at any time.").
   **Observed: `git rebase --autostash` exited 0** ("Successfully rebased") with `UU f` and `stash@{0}: autostash`.
   VS Code's PR confirms the same for `git pull --autostash`: "exits with 0 when re-applying the autostash fails and
   only reports it on stderr" — https://github.com/microsoft/vscode/pull/325762. **Exit code cannot be trusted; the
   index (unmerged entries) must be read after.**
6. **Switch/checkout has no `--autostash`.** `git switch -m` is the nearest: docs (master) say conflicting local changes
   "are automatically stashed before the switch and reapplied afterwards … If reapplying the stash results in
   conflicts, the entry is saved to the stash list" — https://git-scm.com/docs/git-switch. **On 2.54.0 I observed
   otherwise:** `switch -m` exit 0, `UU f`, **no stash entry**. Question: which git version introduced the stash-list
   behaviour? Not confirmed. Either way, the app doing stash → switch → pop itself is under its own control.

## A2. Per tool: conflicted pop/apply

| Tool | What he sees | Where he lands | Stash after | Finish | Source |
|---|---|---|---|---|---|
| lazygit | git's error text in a popup | Files panel refreshed; `UU` files resolved in its merge view | kept (plain `git stash pop`) | resolve, then drop by hand in Stash panel `d` + confirm | `pkg/gui/controllers/stash_controller.go:136-150` (calls `Stash.Pop`, returns git's error) |
| VS Code | warning: "There are merge conflicts while applying the stash. Please resolve them before committing your changes." + **Show Changes** | Source Control view; merge editor titles the incoming side **"Stashed Changes"** | kept | resolve, stage, then *Git: Drop Stash* from palette | `extensions/git/src/commands.ts:5607-5612`, `:862-877`; `git.ts:2694-2695` (https://github.com/microsoft/vscode) |
| Magit | stash kept unless clean | status buffer, conflicts in Unmerged section | pop removes "on complete success" only | resolve, `k` on stash `(?)` | https://docs.magit.vc/magit/Stashing.html; falls back to `git apply --3way` (same page) |
| JetBrains | conflicts "need to be resolved" | its Resolve Conflicts dialog `(?)` | Pop removes, Apply keeps — docs silent on pop-with-conflict `(?)` | `(?)` | https://www.jetbrains.com/help/idea/shelving-and-unshelving-changes.html |
| Sublime Merge, GitUp, Fork, Tower | not read | — | — | — | Fork asks for `--index` option: https://github.com/fork-dev/Tracker/issues/878; SM freeze on apply into big conflicts: https://github.com/sublimehq/sublime_merge/issues/1965 |

**No tool read finishes the job**: every one that I could confirm leaves the stash in the list and the drop to the user.

Step count, "pop clashes → resolved → stash gone" (one file, one hunk), best confirmed:
- lazygit: `g`, `Enter` (confirm) → `Esc` error → `2` files → `Enter` file → pick hunk `Space` → `Esc` → `5` stash →
  `d` → `Enter` = **~10** (`(?)` whether resolving auto-stages; not run).
- VS Code: palette Pop Stash (~3) → **Show Changes** → open file → accept → Complete Merge → palette Drop Stash, pick
  (~3) = **~10** `(?)` counted from code, not walked.

## A3. Per tool: auto-stash around switch and pull

- **lazygit switch:** on "uncommitted changes" error asks **"Autostash?** You must stash and pop your changes to
  bring them across. Do this automatically? (enter/esc)", then `stash push` → checkout → `stash pop`; on a pop clash
  "re-render the UI even if the pop operation failed (e.g. conflict)" and returns the error —
  `pkg/gui/controllers/helpers/refs_helper.go:91-112`, strings `pkg/i18n/english.go:1642-1645`. Stash kept (git pop).
  Steps: `Space`, `Enter` = **2** to switch; clash → Files panel. Same for new branch (`refs_helper.go:404-420`).
- **VS Code switch:** modal "Your local changes would be overwritten by checkout." — **Stash & Checkout**, **Migrate
  Changes** (stash, checkout, pop), **Force Checkout** — `commands.ts:2961-2980`. Clash → the stash-conflict warning
  above. Users ask it to honour `git.autoStash` silently instead of the modal:
  https://github.com/microsoft/vscode/issues/240571 (PR open: https://github.com/microsoft/vscode/pull/305697).
- **VS Code pull:** `git.autoStash` passes `--autostash` (`git.ts:2467-2469`). A clashing put-back is **invisible**:
  https://github.com/microsoft/vscode/issues/262360 "Invisible autostash"; fix open, also because the `autostash`
  entry is filtered out of the stash list (PR #325762 above).
- **Sublime Merge pull --rebase --autostash:** "SM shows a successful command execution, not a failed one" —
  https://github.com/sublimehq/sublime_merge/issues/1199 (open since 2021).
- **JetBrains Smart Checkout:** "will shelve uncommitted changes, check out the selected branch, and then unshelve the
  changes. If a conflict occurs during the unshelve operation, you will be prompted to merge the changes." —
  https://www.jetbrains.com/help/idea/manage-branches.html. Update Project saves changes by **Stash** or **Shelve**
  (setting) — https://www.jetbrains.com/help/idea/settings-version-control-git.html. Whether the shelf is deleted after
  a clashing unshelve `(?)`; docs say shelves "remain available unless explicitly deleted" (shelving page above).
  Steps: branch popup, pick branch, **Smart Checkout** = **3**, clash → merge dialog directly. JetBrains support forum
  not fetched (403 earlier, §5).
- **Magit:** checkout with changes is guarded; a `--merge` checkout option PR was closed unmerged 2026-08-17 —
  https://github.com/magit/magit/pull/5618. Rebase `--autostash` switch `(?)` not read.
- **lazygit, losing edits without autostash:** https://github.com/jesseduffield/lazygit/issues/4823 — staged changes
  lost aborting a rebase; user asks for "a dialog that asks to stash the changes or autostash".
- GitUp: refuses merge-conflict work unless the repo is clean ("the repo must be clean to resolve them") —
  https://github.com/git-up/GitUp/issues/403; user's workaround was the Terminal.

Praise: none found in primary sources (issue trackers only carry complaints). Complaints cluster on one thing: **the
clash is silent or dumped as raw git text** (VS Code #262360, SM #1199, GitUp #403).

## A4. What this means for Graft

- **Take** — JetBrains' landing: a clashing put-back goes straight to the conflict screen (manage-branches doc). It
  matches the user's pick.
- **Take** — keep the stash until the conflict is resolved (git fact A1.1, A1.5). Losing the edits is the one outcome
  "you cannot lose work" forbids.
- **Invent** — finish drops the stash. Every tool I read leaves the drop to the user (A2), and he forgets it. On the
  conflict screen's finish, offer to drop *that* entry, found by its commit id, not `stash@{0}`: the index shifts.
  Reason: nothing found fits.
- **Invent** — the app's own conflict state for "stash put-back". There is no `MERGE_HEAD` (A1.3), so detection can't
  lean on git's operation files. Record which stash id and why (pop / switch / pull) when the app starts it.
  **Question:** how does Graft detect a stash clash the user made from the shell? Only unmerged entries plus the
  `Stashed changes` label (VS Code's trick, `commands.ts:867`) remain.
- **Take** — read the index after every put-back, never the exit code (A1.5: exit 0 with conflicts). This is the bug
  in VS Code and Sublime Merge.
- **Reject** — handing it all to `--autostash`. Its stash is only visible as `autostash`, and switch has no
  equivalent (A1.6). Do stash → op → pop in the app, so one code path covers pull, switch and a plain pop.
- **Reject** — `--index` by default. It refuses on conflict (A1.2, Magit #4993). Say plainly that staged/unstaged
  split is lost on a clash.
- **Question, not decided:** "abort" on a stash clash has no git verb. Is it reset-to-before plus keep the stash? That
  would also throw away what the user had staged before (A1.2). Needs design, and the undo journal must cover it.
- Step targets: switch with edits **2** (lazygit), 3 for JetBrains with a dialog; clashing pop to done **~10** in the
  best two tools. Graft's aim: land on the conflict screen with zero extra steps, and drop in the finish step (0 extra).

Gaps: Sublime Merge, GitUp, Fork, Tower pop/autostash UI not read; JetBrains shelf retention after a clash, Magit
drop key and autostash switch, lazygit auto-stage after resolving — all `(?)`. Step counts in A2 are read from code, not
walked.
