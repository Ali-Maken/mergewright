# Push and pull on a team with dependent branches: prior art

Pipeline step 2, done late: the feature shipped in M4 without a study (`docs/reviews/m6-usability.md`, M6 walk).

**The need.** He works in a team; other people build branches on his, and shared branches move under him
(memory note `works-in-a-team-on-dependent-branches.md`; ADR-0030 kept fetch/pull/push in scope for this reason).
Daily: share commits, take teammates' commits, and recover when a push is refused because someone pushed first.
After he rebases a branch others build on, a force push is routine, not an accident.

Sources are primary (source code at `main`/`master`, vendor docs, issue trackers), fetched 2026-09-24.
`(?)` = not confirmed. Step counts are mine, derived from the cited code/docs, counted per
`docs/research/ux-principles.md` §1.3 (one key or click = one step; a confirm = one step).

---

## 1. Per tool

### 1.1 lazygit (terminal)

- **Keys:** `P` push, `p` pull (config `pushFiles: P`, `pullFiles: p`) — https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md
- **Plain push has no confirm.** `push()` calls `pushAux` directly unless the branch is behind its upstream —
  `pkg/gui/controllers/sync_controller.go:89-98` (master).
- **Behind (known locally) → force prompt, with lease.** `requestToForcePush` shows a confirm and sets
  `forceWithLease = true` — `sync_controller.go:237-252`. Prompt text: "Your branch has diverged from the remote
  branch. Press {{.cancelKey}} to cancel, or {{.confirmKey}} to force push." — `pkg/i18n/english.go:1401`. Keys are
  the universal return/confirm (`esc`/`enter`) — `sync_controller.go:255-262`.
- **Rejected by the server, remote not fetched → offers plain `--force`, not lease.** On "Updates were rejected",
  if the remote branch is not stored locally it confirms and sets `newOpts.force = true` — `sync_controller.go:209-226`;
  `--force` vs `--force-with-lease` chosen in `pkg/commands/git_commands/sync.go:37-38`. If the remote *is* stored
  locally it says "Updates were rejected. Please fetch and examine the remote changes before pushing again."
  (`english.go:1403`).
- **Lease without an expected value.** lazygit passes bare `--force-with-lease` (`sync.go:38`), which leases against
  the last-fetched tracking ref: commits a teammate pushed and you already fetched get overwritten. This is the
  complaint in #4077 (below). Reading of git semantics, not tested here.
- **Pull merge vs rebase:** not a lazygit option in Config.md (the page has no rebase-on-pull setting); presumably
  follows `git config pull.rebase` `(?)`.
- **Dependent branches:** PR #6045 (opened 2026-09-20): "After rebasing a stack of branches, every branch of the stack
  has to be force-pushed, and so far the only way ... was to check out each branch in turn and push it." Proposes a menu
  listing the stack's branches and how far each diverged — https://github.com/jesseduffield/lazygit/pull/6045
- **Disable switch:** `git.disableForcePushing` (default false) — Config.md.

Steps: push `P` = **1**. Rejected (teammate pushed, fetched) → prompt shows → `esc`, `p`, `P` = **4** (the prompt
offers force first). Force after own rebase: `P`, `enter` = **2**.

Complaints (confirmed, issue bodies read):
- Enter-to-force is muscle memory: "I ended up pressing `enter` out of habit too many times" — asks for typing
  "force" — https://github.com/jesseduffield/lazygit/issues/3468
- Lease overwrote a teammate's fetched commit; asks to change default force strategy —
  https://github.com/jesseduffield/lazygit/issues/4077
- Opposite side: force push is the normal GitLab flow; "I very often forget to press enter for the force push
  confirmation prompt" — https://github.com/jesseduffield/lazygit/issues/5966
- Should `disableForcePushing` default to true? (24 comments) — https://github.com/jesseduffield/lazygit/issues/1668
- Confirm button should be configurable (9 comments) — https://github.com/jesseduffield/lazygit/issues/2009 (title only read)

Praise: `(?)` — no primary praise source read for push/pull specifically.

### 1.2 VS Code (built-in Git) and GitLens

- **Sync = pull then push**, confirm dialog on by default: `git.confirmSync` default `true`
  (`extensions/git/package.json:3442-3445`); message "This action will pull and push commits from and to
  "{0}/{1}"." (`extensions/git/src/commands.ts:4288`). Pull mode: `git.rebaseWhenSync` default `false` = merge
  (`package.json:3809-3813`; "Force Git to use rebase when running the sync command.", `package.nls.json:265`).
- **Force push is off by default:** `git.allowForcePush` false; `git.useForcePushWithLease` true;
  `git.useForcePushIfIncludes` true; `git.confirmForcePush` true (`package.json:3845-3864`).
- **Force warning** (modal): "You are about to force push your changes, this can be destructive and could
  inadvertently overwrite changes made by others. Are you sure to continue?" with `OK` / `OK, Don't Ask Again`
  (`commands.ts:4053-4062`).
- **Rejected push:** notification "Can't push refs to remote. Try running 'Pull' first..." offering only "Open Git Log"
  and "Show Command Output"; PR to add a Pull button is open — https://github.com/microsoft/vscode/pull/307635

Steps: Sync click + OK = **2** (1 with confirm off). Rejected path via Sync: same **2** — Sync pulls first. Via Push:
Push, error, Pull, Push = **3+** clicks through menus (menu depth `(?)`). Force: enable setting once, then
menu → Push (Force) → OK = **3** `(?)` on exact menu depth.

Complaints (confirmed):
- Sync silently merged when behind and pushed the merge: "VSCode decided by itself to pull and merge my commits ...
  Once, it made the repo into a bogus state and we needed to force-push to fix it." —
  https://github.com/microsoft/vscode/issues/174198
- Rebase handling on pull/sync — https://github.com/microsoft/vscode/issues/199436 (title only);
  `rebaseWhenSync` rebases with nothing to pull — https://github.com/microsoft/vscode/issues/168045 (title only).
- After a rebase, suggest Push (Force) — https://github.com/microsoft/vscode/issues/193143 (title only).
- GitLens: "Next Steps doesn't offer Force push after a branch has been rebased" —
  https://github.com/gitkraken/vscode-gitlens/issues/5832 ; fix PR https://github.com/gitkraken/vscode-gitlens/pull/5845 (titles only).

### 1.3 Magit (Emacs, keyboard)

- Push transient: `P`, then `p` pushremote / `u` upstream / `e` elsewhere; switches `-f` "Force with lease",
  `-F` "Force" — `lisp/magit-push.el:40-58` (main).
- Pull transient: `F`, then `p`/`u`/`e`; switches `-f` ff-only, `-r` rebase; `r` sets `branch.<name>.rebase` —
  `lisp/magit-pull.el:46-79`.
- Confirm before force: none in the transient; the switch is the deliberate act `(?)` (no confirm found in the file
  excerpt read).

Steps: push `P u` = **2**. Rejected: `F u`, `P u` = **4** (rebase: `F -r u` = 3, total 5 unless `branch.rebase` set).
Force-with-lease: `P -f u` = **3**.

Complaints: `-f` silently not forcing after an update (2019, closed) — https://github.com/magit/magit/issues/3735.
Praise `(?)` — not read.

### 1.4 JetBrains (IntelliJ family)

From https://www.jetbrains.com/help/idea/commit-and-push-changes.html (confirmed):
- Push: `Ctrl+Shift+K` opens Push Commits dialog listing commits since last push; `Push` or `Force push` from the
  button's dropdown.
- Rejected: **Push Rejected dialog** offers **Rebase** or **Merge**; checkbox "Remember the update method choice and
  silently update in the future".
- Force push uses `git push --force-with-lease`; not offered on branches in the **Protected branches** field.

Steps: `Ctrl+Shift+K`, `Push` = **2**. Rejected: dialog → `Rebase`, then push again = **3–4** (whether it re-pushes
automatically after the update `(?)`). Force: shortcut, dropdown, Force push = **3**, plus any confirm `(?)`.

Complaints: `(?)` — none read.

### 1.5 Sublime Merge

Docs do not list push/pull key bindings ("Currently there is no compiled list of all built-in commands" —
https://www.sublimemerge.com/docs/key_bindings). Command palette has a "Push..." entry that prompts for options —
https://www.sublimemerge.com/docs/command_palette. Force/lease behaviour and rejection flow `(?)`.

### 1.6 GitUp, Fork

- **GitUp:** no `--force-with-lease` support requested since 2017, still open —
  https://github.com/git-up/GitUp/issues/333. Pull after someone force-pushed asks "merge or rebase"; buttons did
  nothing in one state — https://github.com/git-up/GitUp/issues/793. Request for one-click force push without the
  confirm — https://github.com/git-up/GitUp/issues/2715.
- **Fork:** release notes: "Use --force-with-lease instead of --force during push." — https://git-fork.com/releasenotes
  (version/date not captured `(?)`).
- **Tower:** not read.

---

## 2. Patterns

1. **Plain push confirms nothing** in lazygit and Magit; VS Code confirms Sync (pull+push), not push. JetBrains shows
   the commits being sent in its push dialog.
2. **Rejection → sync → push** is where tools differ most: VS Code folds it into one Sync (and gets blamed for silent
   merges, #174198); JetBrains asks merge-or-rebase in the rejection dialog; lazygit leads with a force prompt;
   VS Code's plain rejection offers no next action (PR #307635).
3. **Force defaults to lease** in JetBrains, VS Code, Fork, Magit's `-f`. lazygit uses lease only when it knew ahead of
   time; on a server rejection it offers plain `--force` (`sync_controller.go:223`).
4. **Bare lease is weaker than users think:** it guards against what you haven't fetched, not what you haven't looked
   at (#4077). VS Code adds `--force-if-includes` by default.
5. **Enter-to-force is two-sided:** users who force rarely get it by habit (#3468); users who force daily resent the
   extra key (#5966, GitUp #2715). Protected-branch lists (JetBrains) move the guard to where it matters.
6. **Stacks:** pushing every branch of a rebased stack is a 2026 open request in lazygit (#6045). Nobody in this set
   does it built-in `(?)` for JetBrains/GitLens.

---

## 3. What Graft does today (design, not code)

From `docs/design-notes/m4-remotes.md` and `docs/design/02-keymap.md`:
- `f` fetch all + prune, `p` fast-forward pull (undoable), `P` push incl. set-upstream — m4-remotes.md:13; keymap 02-keymap.md:194-196.
- A confirm box before every push, none before pull/fetch; rule "a box iff the write is not undoable or destroys
  uncommitted work" — m4-remotes.md:125-126. Box shows the git command, `[ Cancel ]` under the cursor — :131-140.
- Rejection → sticky status `push rejected: origin/main has commits you don't · p to pull first, or P to force with lease` — :160.
- Force = lease with an **explicit expected oid** (`--force-with-lease=<ref>:<oid>`), shown in a box titled
  `Force-push main → origin/main?` naming the overwritten commit — m4-remotes.md:68, :142-143.
- Stale lease → `push refused: origin/main moved since your last fetch (stale lease) · f to fetch, then look` — :163.
- Diverged pull refused in M4, naming `git pull --rebase`; merge/rebase pull scheduled for M5 — :17, :161.
  **Whether M5 built it: not checked (source not read) — question.**
- Push not undoable; `u` after push undoes locally and says origin still has it — :90-99.
- Stack push ("rebase this branch and everything built on it"): ADR-0032, M7 — memory note; not in these docs.

---

## 4. Best step counts to judge against

| Task | Best found | Tool | Graft (design) |
|---|---|---|---|
| Push commits | 1 (`P`) | lazygit | 2 (`P`, `⏎`) |
| Push, shown what is sent | 2 | JetBrains (`Ctrl+Shift+K`, Push) | 2 |
| Rejected → integrate → push | 2 (Sync, OK) — but merges silently (#174198) | VS Code | `P`,`⏎`, `p`, `P`,`⏎` = 5 if fast-forwardable; diverged: refused in M4 `(?)` M5 |
| Rejected → integrate, choosing merge/rebase | ~3–4 `(?)` | JetBrains dialog | `(?)` pending M5 |
| Force with lease after own rebase | 2 (`P`, `enter`) | lazygit | `P`,`⏎` → reject → `P`,`⏎` = 4 `(?)` whether `P` goes straight to lease box when already known diverged |
| Push a rebased stack | none built in (lazygit #6045 open) | — | M7 (ADR-0032) |

Could not confirm: Sublime Merge push/pull flow, Tower, JetBrains auto-push after rejection-update, praise for any tool.
