# Interactive rebase: how the best-loved tools do it

Researcher, 2026-09-26. Feature pipeline step 2 for M7. Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I have not run any of these tools. Step counts are
derived from each tool's docs; where the docs leave a step out, the count is marked **(?)**. Anything I could not
confirm is marked **(?)** or listed in §5. Claims about Graft carry a repo `file:line`.

**Counting rule** (`docs/research/ux-principles.md` §1.3): one step = one key chord or one click that the user must
decide on; cursor moves to a known row count as one step per keypress. Typing a message or editing a file is counted
as "(edit)", not in the number.

**Who needs it** (`memory: graft-daily-workflows`): interactive rebase is **weekly-ish**, not daily. The user works in
a team on dependent branches. `docs/field-notes.md` has no line on rebase -i, squash, fixup or reword yet (grep,
2026-09-26); the only rebase line is pull-with-rebase with unsaved edits (`docs/field-notes.md:67`).

**The three tasks:**
- **T1** squash the last 3 commits into one;
- **T2** fix a typo in the commit 4 back (the fix is already made in the file);
- **T3** swap two adjacent commits.

---

## 1. The tools

### 1.1 lazygit (nearest peer: keyboard TUI)

Sources: https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md ,
https://github.com/jesseduffield/lazygit/wiki/Interactive-Rebasing

- **Model: act on the commit in place, from the ordinary commits list.** No separate todo editor needed for single
  operations. Keys in the commits panel: `s` squash into the commit below, `f` fixup, `r` reword (`R` in `$EDITOR`),
  `d` drop, `e` edit ("start an interactive rebase from the selected commit"), `Ctrl+j`/`Ctrl+k` (or `Alt+↓/↑`) move
  down/up, `A` amend staged changes into the selected commit, `F` create a `fixup!` commit for the selected commit,
  `S` apply fixup commits (autosquash) "either above the selected commit, or all in current branch", `B` mark a base
  commit for the next rebase, `i` start an interactive rebase for the branch.
- **Batch mode:** press `e` (or `i`), then mark several commits with `p/s/f/d/e`, then `m` → *continue* runs them
  (wiki page above). Commits are shown newest-first, the reverse of git's todo file (same page).
- **Split:** no dedicated split key found in the keybindings doc **(?)**; the custom-patch menu (`Ctrl+p`) moves
  selected lines out of a commit (keybindings doc lists "View custom patch options").
- **T1:** `j` to 2nd commit, `v` range-select, `j`, `s`, confirm = **~5** **(?: range-select and confirm prompt not in
  the doc I read)**.
- **T2:** stage the fix (`Space`), switch to commits panel, `j`×3, `A`, confirm = **~6** **(?: confirm prompt)**. Or
  `F` then `S` (autosquash) for a reviewable two-step path.
- **T3:** move to commit, `Ctrl+k` = **1–2**.
- **When it stops:** on conflict lazygit asks whether to abort or resolve; after resolving it says all conflicts are
  resolved and asks whether to continue. The commits list shows **"YOU ARE HERE"** at HEAD (wiki page above).
- **Praise:** autosquash `S` for "a lot of fixup after a review"; the maintainer renamed the command because users
  could not find it — a discoverability lesson
  (https://github.com/jesseduffield/lazygit/discussions/1160). Request for "squash all fixups in current branch"
  without choosing a start commit (https://github.com/jesseduffield/lazygit/issues/3263).
- **Complaints:** "CRITICAL: unwanted squash during rebase" — pressing `A` while stopped on a conflict melded two
  commits in a long patch series (https://github.com/jesseduffield/lazygit/issues/4208). Autosquash on `main` fails
  silently (https://github.com/jesseduffield/lazygit/issues/5087). A commit whose message starts `fixup!` was treated
  as a fixup unasked (https://github.com/jesseduffield/lazygit/discussions/4664).

### 1.2 Sublime Merge

Source: https://www.sublimemerge.com/blog/sublime-merge-tips-editing-git-commits

- **Model: context menu "Edit Commit" on a commit in the graph.** Edit message (`Ctrl/Cmd+Enter` saves), drop selected
  commits, squash selected commits (multi-select with `Ctrl/Cmd+click`). Move up/down exists as a `move_commit`
  command; users asked for a key binding for it (https://forum.sublimetext.com/t/key-binding-for-move-commit-up-down/65499 ,
  https://viteinfinite.com/2022/12/rebase-key-bindings-for-sublime-merge/). Fixup, edit-contents and split: not in the
  article **(?)**.
- **T1:** click, Cmd+click ×2, right-click, Edit Commit → Squash = **~5**. **T2:** no amend-into-older-commit found
  **(?)**; presumably edit/rebase **(?)**. **T3:** right-click → move = **~3** **(?)**.
- **When it stops:** "you may run into a merge conflict … you will have to manually resolve the conflicts and select
  Continue Rebase" (same article).
- **Complaints:** squash against the initial commit leaves an errant rebase state
  (https://github.com/sublimehq/sublime_merge/issues/594); a failed squash corrupted a repository
  (https://github.com/sublimehq/sublime_merge/issues/687).

### 1.3 JetBrains IDEs

Source: https://www.jetbrains.com/help/idea/edit-project-history.html

- **Model: both.** In-place actions from the Log context menu (Squash Commits, Fixup, Drop, reword, "Extract changes"
  to split files out of a commit), plus **Interactively Rebase from Here**, a dialog listing commits with
  pick/edit/squash/fixup/drop and reordering (https://youtrack.jetbrains.com/issue/IDEA-305851).
- **Fix into an older commit:** stage changes, right-click the target, fixup/squash "append staged changes to the
  selected commit" (docs above).
- **T1:** select 3 (click, shift-click), right-click, Squash Commits, (edit message), OK = **~4**. **T2:** right-click
  target, Fixup = **~2 after staging** (+ commit-tool-window steps **(?)**). **T3:** Interactively Rebase from Here,
  move row, Start Rebasing = **~4** **(?: move gesture not documented on the page read)**.
- **When it stops:** the page read does not describe stops **(?)**; the Conflicts dialog from `conflict-resolution.md`
  §1.1 is the likely landing **(?)**.
- **Complaints:** the action is greyed out with no reason given, on protected branches
  (https://youtrack.jetbrains.com/issue/PY-36338 , https://youtrack.jetbrains.com/issue/IJPL-75136); did not honour
  `fixup!`/`squash!` until an autosquash option was added (https://youtrack.jetbrains.com/issue/IJPL-73981); aborts on
  an empty commit (https://youtrack.jetbrains.com/issue/IJPL-110272); no `--update-refs` for stacked branches
  (https://youtrack.jetbrains.com/issue/IDEA-305851) — relevant to a user on dependent branches.

### 1.4 GitUp

Sources: https://github.com/git-up/GitUp , https://github.com/git-up/GitUp/wiki/Splitting-Commits-with-GitUp ,
https://github.com/git-up/GitUp/wiki/Rewriting-Commits-with-GitUp

- **Model: direct manipulation on the graph, every operation undoable.** "Live and interactive repo graph (edit,
  reorder, fixup, merge commits…)", "Unlimited undo / redo of almost all operations (even rebases and merges)"
  (README). Squash with parent, swap with parent/child (`U` for swap with child, per
  https://github.com/git-up/GitUp/issues/209); other keys not confirmed **(?)**.
- **Rewrite (`W`):** edit a commit's contents and message in place; `Opt-Return` confirms, `Esc` cancels (wiki).
- **Split:** a **visual splitter**: move files or lines between an "Old Commit" and "New Commit" list; "does not modify
  your repository or working directory until you finish"; then Continue, two messages, confirm (wiki).
- **T1:** squash-with-parent twice = **~4** **(?)**. **T2:** select commit, `W`, (edit), `Opt-Return` = **~3** **(?:
  whether the already-made working-tree fix can be pulled in)**. **T3:** select, swap = **~2**.
- **When it stops:** conflict handling on rewrite is not described in the pages read **(?)**.
- **Complaints:** swap-with-child sometimes fails after Quick View (https://github.com/git-up/GitUp/issues/209).

### 1.5 jj (Jujutsu)

Source: https://docs.jj-vcs.dev/latest/cli-reference/

- **Model: no todo list, no stop.** Each operation is one command; descendants are rebased automatically.
  - `jj squash` (`-r` source, `--into` target, `-i` pick hunks) moves changes between revisions.
  - `jj split` divides a revision in two (interactive without paths).
  - `jj absorb` moves each change "to the closest mutable ancestor where the corresponding lines were modified last".
  - `jj rebase -r X -A/-B Y` reorders ("insert after/before"). `jj describe` rewords, `jj abandon` drops.
  - Conflicts: descendants "will be rebased on top as usual, which may result in conflicts" — recorded in the commit,
    the command does not stop (same page). Undo: `jj undo`.
- **T1:** `jj squash --from X::Y --into Z` or two `jj squash` = **1–2 commands** **(?: exact revset form)**. **T2:**
  `jj absorb` = **1**, or `jj squash --into <rev>` = 1. **T3:** `jj rebase -r X -B Y` = **1**.
- **Praise:** "You can just say 'gosh I really wish I'd made some changes here' and either move changes in, or just
  jump in and live-edit it directly" (steveklabnik); rebases so cheap that commands do many "without even mentioning
  it" (https://news.ycombinator.com/item?id=42505475). Absorb on megamerges:
  https://v5.chriskrycho.com/journal/jujutsu-megamerges-and-jj-absorb/ .
- **Counterpoint:** "git rebase -i has an 'edit' command that pauses the rebase at that commit to let you do whatever"
  (Izkata, same thread) — the stop is a feature to some.

### 1.6 git-absorb

Source: https://github.com/tummychow/git-absorb

- Stage fixes, `git absorb --and-rebase`: creates `fixup!` commits matched per hunk and autosquashes. Looks at the last
  10 commits by default (`--base` to change). A hunk it cannot place gets a warning and **stays staged**, never guessed.
- **T2:** stage + 1 command = **2**. Not for T1/T3.

### 1.7 Magit

Source: https://docs.magit.vc/magit/Rebasing.html

- Single-commit shortcuts from the log: `r m` edit an older commit, `r w` reword, `r k` remove, `r f` autosquash.
  `r i` opens the todo buffer: `p r e s f k` set the action, `M-p`/`M-n` move a line.
- **T1:** `r i`, pick base, `s` on 2 lines, `C-c C-c` = **~5** **(?)**. **T2:** commit with fixup (`c F` instant fixup
  **(?: not in the page read)**) then `r f` **(?)**. **T3:** `r i`, `M-p`, `C-c C-c` = **~4**.
- **When it stops:** the status buffer shows a rebase section split at HEAD — done commits above, pending below, the
  stopped commit highlighted; "point is placed on the commit that separates the two groups, i.e., on HEAD" (page above).
  That is a *where am I* map during the stop.
- Praise/complaint links: search did not surface primary threads **(?)**.

### 1.8 GitKraken

Source: https://help.gitkraken.com/gitkraken-desktop/interactive-rebase/

- Start by dragging a branch onto another or right-click → Interactive Rebase. Per-row keys `P S R D`
  (pick/squash/reword/drop); Reset to undo setup. No merge commits allowed; "If you start the rebase in GitKraken
  Desktop, you must complete it there". Edit, fixup and split not listed **(?)**; conflict handling not on the page **(?)**.
- **T1:** open, `S` on 2 rows, start = **~4**. **T3:** drag a row **(?)**.

### 1.9 Fork

Source: https://git-fork.com/ — "Edit, reorder and squash your commits using visual interactive rebase." Details not
on the page **(?)**.
- **Complaints:** the entry point is backwards — you select the *target* branch to rebase the checked-out one
  (https://github.com/fork-dev/Tracker/issues/223); asks for a shortcut to "interactive rebase to here"
  (https://github.com/fork-dev/TrackerWin/issues/1105) and drag-to-reorder in history
  (https://github.com/fork-dev/TrackerWin/issues/905); a freeze during interactive rebase
  (https://github.com/fork-dev/Tracker/issues/2164); squash acted like fixup
  (https://github.com/fork-dev/Tracker/issues/1824).

### 1.10 Tower

Source: https://www.git-tower.com/help/guides/commit-history/interactive-rebase/mac

- Drag a commit between two others to reorder; select several → "Squash N Revisions", or drag onto another; `Option`
  while dragging = fixup; delete, Edit Commit Message; **Edit** stops with the commit's changes in the Working Copy
  view, where it can also be split. Conflict landing not on the page **(?)**.
- **T1:** select 3, right-click, Squash = **~3**. **T2:** Edit, (fix), continue = **~3** **(?)**. **T3:** 1 drag.

---

## 2. Summary table

| Tool | T1 squash 3 | T2 typo 4 back | T3 swap | Model | Stops? |
|---|---|---|---|---|---|
| lazygit | ~5 (?) | ~6 (?) / `F`+`S` | 1–2 | in place + batch todo | yes, "YOU ARE HERE" |
| Sublime Merge | ~5 | (?) | ~3 (?) | context menu | yes, Continue Rebase |
| JetBrains | ~4 | ~2 after staging | ~4 (?) | in place + dialog | yes (?) |
| GitUp | ~4 (?) | ~3 (?) | ~2 | graph, undo | (?) |
| jj | 1–2 | 1 (absorb) | 1 | commands, auto-rebase | never |
| git-absorb | — | 2 | — | CLI | on conflict (git's) |
| Magit | ~5 (?) | (?) | ~4 | shortcuts + todo buffer | yes, status map |
| Tower | ~3 | ~3 (?) | 1 drag | drag and drop | (?) |

---

## 3. Graft today

- Keymap §2.7 already allocates a **modal todo editor**: `p r e s f d`, `Alt+↑/↓` reorder, `Enter` start, `Esc` cancel
  (`docs/design/02-keymap.md:355-365`), plus a note that `r` means reword inside it (`docs/design/02-keymap.md:75-78`).
- `r` in the graph is "Rebase current onto selection" (`docs/design/02-keymap.md:191`); `y` confirms every preview
  (`docs/design/02-keymap.md:228`); a rebase begun in Graft continues with `c` (`docs/design/02-keymap.md:248`), `s`
  skips (`docs/design/02-keymap.md:306`), and "shell-begun rebase … stay M7" (`docs/design/02-keymap.md:305`).
- Not checked: whether any §2.7 code exists **(?)**; how the editor is opened (no key found in the grep) **(?)**.

---

## 4. Takeaways for Graft

1. **Take: in-place single operations from the graph** (lazygit, JetBrains, Sublime Merge, GitUp). T3 and T2 drop to
   1–3 steps; a todo editor for one squash is 5+. Needs keys — spec change through the conductor, since §2.7 only
   binds inside the modal editor.
2. **Take: keep the modal todo editor for batches**, with git's `p r e s f d` letters (already §2.7). Users know them.
3. **Take: amend staged changes into any commit** (lazygit `A`, JetBrains fixup-onto). This is T2 in ~3 steps.
4. **Take: absorb** (jj, git-absorb), including git-absorb's rule: an unplaceable hunk **stays staged with a warning**,
   never guessed. Best T2 count found (1–2). Show it in a Ghost Preview (which hunk → which commit) before `y`.
5. **Take: autosquash from one key, whole branch** (lazygit `S`, #3263) — and name it so it is findable (#1160).
6. **Take: a where-am-I map while stopped** (Magit's done/stopped/pending split, lazygit's "YOU ARE HERE"). Land on the
   conflict screen for a clash (M6 rule), on the map for an `edit`.
7. **Take: `--update-refs` for dependent branches.** JetBrains users ask for it (IDEA-305851); the user works on
   dependent branches (`memory: works-in-a-team-on-dependent-branches`). Whether Graft's rebase uses it today **(?)**.
8. **Take: GitUp's split model** — two lists, nothing touches the repo until confirm.
9. **Reject: jj's never-stop model.** Graft sits on git; committed conflict states are not a git concept.
10. **Reject: amend refused/unsafe while stopped on a conflict must be explicit** — lazygit #4208 lost a patch series
    this way. Graft's `a` already refuses in words during an operation (`docs/design/02-keymap.md:248`); keep that.
11. **Reject: silent disables and silent failures** (JetBrains PY-36338, lazygit #5087). Refuse in words.
12. **Invent: journal undo of a whole interactive rebase as one step** — GitUp's undo is the praise-worthy piece, and
    it lets the in-place operations skip confirm boxes (`ux-principles.md` §3.5). Invent only if the M3 journal does not
    already cover rebase **(?)**.

---

## 5. Could not confirm / access notes

- 404: `sublimemerge.com/docs/editing_commits`, `magit.vc/manual/magit/Rewriting-History.html`,
  `magit.vc/manual/magit/Rebasing.html` (used docs.magit.vc instead).
- GitUp wiki index failed to load its pages ("error while loading"); individual pages worked.
- No primary praise/complaint threads found for Magit's rebase, GitKraken or Tower.
- All step counts are from docs, not from running the tools; confirm prompts are unverified throughout.
- Graft's current rebase code, journal coverage of rebase and use of `--update-refs`: not read.
