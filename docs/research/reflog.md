# Reflog view: how the best-loved tools do it, and what Graft should take

Researcher, 2026-09-26. Feature pipeline step 2 for M7. Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I have not run any of these tools. Step counts
follow Graft's rule, `steps = keys + decisions` (`docs/research/ux-principles.md:29-38`), and are **derived from the
docs, not measured**. Where the docs leave a step out I say so. Anything I could not confirm is marked **(?)**. Sites
that refused or 404'd are listed at the end.

**The need, in the user's words and data.** Reflog is **rare** for him: "Rarer: … revert/reset and file history
(monthly), reflog (rare)" (memory `graft-daily-workflows`). `docs/field-notes.md` has no reflog entry to date. So this
is an *emergency* screen: used seldom, under stress, when he does not remember its keys. That weighs toward
recognition over recall (Nielsen 6, `ux-principles.md` §2) and toward few decisions (UX 1.1: "M dominates").

**What Graft already has.** The M3 undo journal records operations Graft itself ran. ADR-0010: the journal is
per-worktree, backup refs are `refs/graft/backup/<worktree>/<timestamp>` (`docs/adr/0010-journal-per-worktree-backup-refs-namespaced.md:8-16`).
ADR-0031: the journal must be able to answer "what did the whole repo look like at 14:20", and the time machine goes
through Ghost Preview, never bypassing it (`docs/adr/0031-the-journal-answers-repo-state-at-a-time.md:8-13`, `:30-35`).
The keymap doc has no reflog entry (`grep -i reflog docs/design/02-keymap.md` returns nothing), so **no key is
allocated yet** — that is a spec item, not free space (CLAUDE.md, "Shared namespaces").

**A git fact that shapes task 2.** "If the branch currently has a reflog then the reflog will also be deleted"
(https://git-scm.com/docs/git-branch). A deleted branch's own history is gone; its tip survives in the reflog only via
`HEAD`'s log, i.e. only if it was ever checked out, or if the tip is otherwise reachable. A branch created and deleted
without checkout cannot be found by any reflog view. Only a tool that recorded the ref itself (a journal/snapshot) can
bring it back.

---

## 1. The tools

### 1.1 lazygit: Reflog tab beside Commits

Sources: https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md ,
https://github.com/jesseduffield/lazygit/blob/master/docs/Undoing.md

- **Presentation:** a "Reflog" panel listing reflog entries. Keys there mirror the Commits panel: `space` checkout
  (detached), `g` reset menu (soft/mixed/hard), `n` new branch off the entry, `N` move commits (keybindings doc).
  The entry text is git's own reflog subject (e.g. "reset: moving to …") **(?)** — the docs do not show a sample.
- **Undo is built on the reflog:** "lazygit can read through your reflog for you and walk back action by action so
  that you don't even need to read the reflog" (Undoing.md). It maps entries back to intent: a checkout restores the
  branch, a rebase returns to the pre-rebase state. Because it reads git's reflog, `z` should also walk back a reset
  done in the shell **(?)**, not stated.
- **Limits (Undoing.md):** "changes to your working tree or stash aren't covered"; branch creation isn't undone
  ("aren't stored in the reflog"); "If you are mid-rebase, undo/redo is not supported".
- **Task 1, commit lost to `reset --hard`:** focus the commits panel (1 key), switch to the Reflog tab (1 key **(?)**,
  tab key not in the fetched doc), move to the entry before the reset (1 motion + 1 decision), `g` (1), pick hard
  (1 key + 1 decision). **≈ 7.** Alternative: `z` (1) + confirm (? — not stated) ≈ **2–3**, if the reset is the last
  reflog step.
- **Task 2, deleted branch:** reflog tab (2), find the "checkout: moving from <branch>" entry (1 motion + 1 decision —
  the user must recognise the old tip), `n` (1), type name (1), Enter (1, anticipated). **≈ 7.** Works only if the
  branch was checked out (git fact above).
- **Praise:** the Undoing doc itself sells it as "you don't even need to read the reflog" — the undo, not the tab, is
  the praised feature. I found no third-party praise thread to link **(?)**.
- **Complaints (all bugs/perf, not concept):** reflog tab slow to populate, ~15 s on the Linux kernel
  (https://github.com/jesseduffield/lazygit/issues/2397); wrong timezone in reflog
  (https://github.com/jesseduffield/lazygit/issues/3153); `>` doesn't go to bottom in the reflog panel
  (https://github.com/jesseduffield/lazygit/issues/1584); highlight hardcoded to "master"
  (https://github.com/jesseduffield/lazygit/issues/1675).

### 1.2 GitUp: Snapshots **and** a unified reflog browser — two views

Sources: https://github.com/git-up/GitUp , https://github.com/git-up/GitUp/wiki/Using-GitUp-Reflog ,
https://github.com/git-up/GitUp/wiki/Using-GitUp-Snapshots

- **Two features, two panes.** README: "Unlimited undo / redo of almost all operations", "Time Machine like snapshots
  for 1-click rollbacks to previous repo states", and "a unified reflog browser".
- **Reflog pane** (Repository → "Browse Reflog…"): one reflog merged across all branches, deduplicated; arrow keys or
  mouse move through entries; the commit graph ("map") shows where each entry lands; **unreachable commits are drawn
  in darker text**; a **Restore** button "create[s] a new local branch pointing to the commit". "If you restored the
  wrong commit, you can undo with Cmd-Z".
- **Snapshots pane** (Repository → "Browse Snapshots…"): list of whole-repo states; selecting one previews it in the
  map with "all editing operations … disabled"; **Restore** puts back HEAD, tags, local branches and their config,
  not remote branches or stash. For changes made outside GitUp the snapshot is labelled **"Changed outside GitUp"**,
  and snapshots are only taken while GitUp is running.
- **Task 1:** Snapshots: menu (1 + 1 decision), pick the snapshot before the reset (1 motion + 1 decision), Restore (1).
  **≈ 5.** Or Cmd-Z if the reset was done in GitUp: **1**. Via reflog pane: menu (2), find darker entry (2),
  Restore (1) ≈ **5**, but it makes a new branch rather than moving the old one back.
- **Task 2:** Snapshots ≈ **5** (restores the branch under its old name, even if never checked out, provided GitUp was
  running). Reflog pane ≈ **5 + naming (?)** — docs don't say whether Restore asks for a name.
- **Praise / complaints:** I did not find a user thread on these two panes specifically **(?)**. Rule 7: none claimed.

### 1.3 jj (Jujutsu): `op log`, `undo`, `op restore`

Sources: https://docs.jj-vcs.dev/latest/operation-log/ , https://docs.jj-vcs.dev/latest/git-compatibility/

- **One log, jj's own.** Every jj command records an operation holding a "view" (bookmarks, tags, git refs, heads,
  working-copy commits). `jj undo` reverses the latest; `jj op revert` a specific older one; `jj op restore` puts the
  whole repo back to an earlier operation; `--at-op` inspects the repo as of one.
- **Git done outside jj** appears in the same op log as an **"import git refs"** operation (git-compatibility page);
  in a colocated repo every jj command imports first. So outside work is *one lump entry*, not per-command — the
  analogue of GitUp's "Changed outside GitUp".
- **jj does not show git's reflog** in `op log` (not mentioned on either page **(?)**).
- **Task 1:** if the reset was a `git reset --hard` in the shell: `jj op log` (1), read to find the "import git refs"
  entry (1 decision), `jj undo` or `jj op restore <id>` (1 + typed id 1). **≈ 3–4.**
- **Task 2:** same shape, **≈ 3–4**, and it works whether or not the branch was ever checked out — provided a jj command
  ran while the branch existed (?).
- **Complaints:** the auto-import makes undo surprising: "Apparently a no-op. `jj op log` shows: undo operation…
  import git refs…" (https://github.com/jj-vcs/jj/issues/4625); deleting a git tag/branch with git "deletes ancestors
  and breaks `jj undo`" (https://github.com/jj-vcs/jj/issues/6325, title only, not read **(?)**).

### 1.4 git-branchless: `git undo` time-travel UI, smartlog

Source: https://github.com/arxanas/git-branchless/wiki/Command:-git-undo

- An event log of operations that affect the commit graph; `git undo` opens an interactive program showing previous
  repo states, arrow keys to move, Enter to choose, then "a list of operations to carry out, which you can either
  accept or reject". Working-copy changes are covered only if a branchless command snapshotted them.
- Whether ref changes made by plain `git` in the shell are logged (via hooks) is not stated on the page **(?)**.
- **Task 1 / Task 2:** `git undo` (1), arrows to the state (1 motion + 1 decision), Enter (1), accept (1 + 1 decision).
  **≈ 6** each.
- **Praise / complaints:** none read **(?)**.

### 1.5 Magit: reflog buffers

Source: https://docs.magit.vc/magit/Logging.html

- `l r` reflog of the current branch, `l O` reflog of another ref, `l H` HEAD reflog. Shown as a log buffer with a
  configurable margin (age or date, author). Entries are git's own subjects **(?)**.
- **Task 1:** `l H` (2), move to entry (1 + 1 decision), reset from the buffer (`X` then hard? **(?)**, ~2 + 1).
  **≈ 7.**
- **Task 2:** `l H` (2), find the checkout entry (2), create branch at point (`b c`, name, Enter ≈ 4 **(?)**). **≈ 8.**
- **Praise / complaints:** none read **(?)**.

### 1.6 Sublime Merge: no first-class reflog

- Reflog only as a custom command whose raw text output appears behind an info-bar icon. A 2018 request asks for "a
  first-class view so it was easier to parse and understand", citing Tower as the model; no developer reply visible
  (https://forum.sublimetext.com/t/feature-request-first-class-view-for-reflog/39389). Users ask "Are my commits lost
  forever?" (https://forum.sublimetext.com/t/are-my-commits-lost-forever/54311, title only **(?)**).
- A "Repository Undo/Redo" menu item exists per the search summary; not confirmed on a primary page **(?)**.
- **Task 1 / 2:** not possible in-app beyond undo; shell otherwise.

### 1.7 JetBrains IDEs: a notification, not a view

Source: https://www.jetbrains.com/help/idea/manage-branches.html (via search summary; page not fetched directly **(?)**)

- Deleting a branch shows a notification with a **Restore** link; after closing it, the link stays "in the
  Notifications tool window until you restart IntelliJ IDEA". No reflog view on the undo page
  (https://www.jetbrains.com/help/idea/undo-changes.html, fetched: no reflog content).
- **Task 2 (deleted in the IDE, same session):** click Restore: **1–2.** Deleted in the shell or after restart: not
  possible in-app. **Task 1:** no in-app route found **(?)**.

### 1.8 GitKraken: no reflog view

- Feedback item "Reflog support" asks to see lost commits in the tree and restore or prune them
  (https://feedback.gitkraken.com/suggestions/190836/reflog-support — **403**, content from search snippet only **(?)**).
  GitKraken's own blog routes users to the terminal: run `git reflog` there
  (https://www.gitkraken.com/blog/6-underused-git-commands-that-solve-real-developer-problems, not fetched **(?)**).

### 1.9 Fork: reflog, details not documented

- FAQ: "I accidentally lost some commits" → "Use reflog to see lost commits"
  (https://github.com/fork-dev/Docs/blob/master/faq.md). How to open it and what it shows: not documented **(?)**.

### 1.10 Tower (added: named as the model by Sublime Merge users)

- Cited as the reference reflog UI in the Sublime thread above. Its help page 404'd; not studied **(?)**.

---

## 2. Summary table (derived, not measured)

| Tool | Task 1: lost to `reset --hard` | Task 2: deleted branch | Outside-app work visible? |
|---|---|---|---|
| lazygit | ≈7 (tab); ≈2–3 via `z` (?) | ≈7, only if once checked out | yes (reads git reflog) |
| GitUp | ≈5 snapshot; 1 if Cmd-Z | ≈5 snapshot | yes: "Changed outside GitUp", only while running |
| jj | ≈3–4 | ≈3–4 | yes: one "import git refs" entry |
| git-branchless | ≈6 | ≈6 | (?) |
| Magit | ≈7 (?) | ≈8 (?) | yes (git reflog) |
| JetBrains | none found | 1–2, in-IDE, same session | no |
| Sublime Merge | raw text only | raw text only | raw text |
| GitKraken | terminal | terminal | — |
| Fork | (?) | (?) | yes (?) |

---

## 3. One view or two?

**What the two tools that have both do:**

- **GitUp: two views.** Snapshots (whole-repo states, its own record, undo-able) and the Reflog browser (git's
  per-ref history, unified across branches) are separate menu items and separate panes
  (wiki pages above). Outside work shows up in *both*: as "Changed outside GitUp" in Snapshots, and as real entries
  in the reflog.
- **jj: one view.** `op log` is the only history; git activity outside jj is folded into it as "import git refs"
  entries. jj does not surface git's reflog. The cost shows in #4625: an automatic import entry makes `undo` do
  something the user did not expect.

**Recommendation: one list, two sources, visibly marked — but one screen, not two.** Reasons:

1. **The user asks one question in both cases:** "where was my stuff before it went wrong?" He does not know, and
   should not need to know, whether Graft or the shell did the damage (Nielsen 2, match the real world). Two screens
   make him pick one first — a decision (M) at the worst moment, and the wrong pick costs a round-trip.
2. **The journal is richer, the reflog is wider.** The journal knows intent, index and working tree
   (ADR-0031) and can bring back a branch never checked out; the reflog sees the shell. Interleaving them by time
   gives both. Where a journal entry and a reflog entry describe the same ref move, show the journal one (it has the
   human name) — GitUp already dedupes across branches.
3. **Actions differ by source, and must say so.** A journal row offers *undo/restore to this state* (Ghost Preview,
   ADR-0031). A reflog-only row offers *restore as branch* / *reset current branch here* — never "undo", because
   Graft doesn't know what the shell did (lazygit's doc lists exactly these gaps). The row's source tag and the key
   bar make that visible.
4. **Counter-argument, stated plainly:** jj's #4625 shows a merged log can confuse undo. Guard: `u`/Ctrl-Z keep
   meaning "undo Graft's last operation" (ADR-0002) and never walk into reflog-only rows.

This is a hard-to-reverse call touching the keymap and the journal's reader — it wants an ADR at step 3, and the
prototype at step 5 should test whether the mixed list reads clearly.

---

## 4. Takeaways for Graft

| # | Aspect | Take / reject / invent | Why |
|---|---|---|---|
| 1 | Separate reflog and snapshot screens (GitUp) | **Reject** | Forces a "which log?" decision under stress; one question, one place (§3). |
| 2 | One time-ordered list, journal + git reflog, rows tagged by source | **Invent** (jj merges, but hides git's reflog; GitUp shows both, separately) | Shell damage and Graft damage found in the same place; nothing studied does both in one list. |
| 3 | Unified reflog across all refs, deduplicated (GitUp) | **Take** | Per-ref reflogs (Magit `l r`/`l O`/`l H`) make the user know which ref to ask about. |
| 4 | Dim unreachable commits (GitUp) | **Take** — ships colour-unverified (rule 5); needs a glyph cue too | "Lost" is the thing he is looking for; must be visible at a glance. |
| 5 | Restore = new branch at the entry (GitUp Restore, lazygit `n`) | **Take** | Non-destructive; nothing current moves. Default action for reflog-only rows. |
| 6 | Reset-hard from the reflog (lazygit `g`, Magit) | **Take, behind Ghost Preview** | Destructive; ADR-0031 requires preview before any "put back". |
| 7 | Whole-repo restore to a point (GitUp Snapshots, `jj op restore`) | **Take** for journal rows only | Already decided (ADR-0031); reflog rows lack index/worktree state. |
| 8 | Human entry text | **Invent** a mapping of git reflog subjects to plain phrases ("reset main to 3 commits back", "switched from feature-x"), git's raw subject on the detail line | Sublime users call raw reflog hard to parse (forum link); lazygit's undo already maps entries to intent. Exact wording is a design-step item. |
| 9 | "Changed outside Graft" lump entry (GitUp, jj import) | **Reject** as the only record | Git's reflog already has the per-command detail; use it instead of a lump. |
| 10 | Undo key walking the reflog (lazygit `z`) | **Reject** | Undo that can't restore the index/worktree breaks "you cannot lose work"; ADR-0002's Ctrl-Z stays journal-only (jj #4625 shows the confusion). |
| 11 | Notification-only restore (JetBrains) | **Reject** as the mechanism, **take** the idea as a hint | Gone after restart and blind to the shell; but a status hint "branch deleted · restore" after Graft's own delete is cheap. |
| 12 | Branch deleted without ever being checked out | **Invent**: say so plainly in the view when the journal has nothing | No reflog can find it (git-branch doc); an honest empty state beats a silent miss (Nielsen 9). |
| 13 | Target step counts | Task 1 ≤ 4, Task 2 ≤ 5 (open, move, decide, restore/name) | Beats lazygit/Magit (≈7), matches GitUp/jj (≈3–5); rare task, so fewest *decisions* matters more than keys (UX 1.1). |
| 14 | Key to open it | **Question for the conductor** | `02-keymap.md` allocates none; a key must come through the spec, not the build. |

---

## 5. Could not confirm / refused

- **403:** feedback.gitkraken.com. **404:** magit.vc/manual/magit/Reflog.html (used docs.magit.vc instead), Tower help
  page. **GitHub wiki index** failed to render; the two GitUp wiki pages loaded directly.
- Not fetched, cited from search results only: JetBrains manage-branches page, GitKraken blog, jj #6325, Sublime
  "Are my commits lost forever?".
- No user praise found for any reflog *view*; praise exists only for undo (lazygit's own doc). Treat "users love X"
  claims about these screens as unsupported.
- Unknown: whether lazygit `z` undoes a shell reset; whether git-branchless logs plain-git ref changes; Magit's reset
  and branch keys from a reflog buffer; Fork's and Tower's UI.
