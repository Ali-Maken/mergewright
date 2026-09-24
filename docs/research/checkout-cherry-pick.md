# Switching branches and cherry-picking: prior art

Pipeline step 2, done **late**: the feature shipped in M5 (`.` menu verbs) before this study existed. Trigger:
`docs/reviews/m6-usability.md:52-53` — check out a branch = `. j ⏎ y` 4 keys; cherry-pick = `. j×6 ⏎ y` 9 presses;
both daily (`docs/reviews/m6-usability.md:7-8`), neither has a key.

**Need:** daily Git user in a team on dependent branches (memory `works-in-a-team-on-dependent-branches`): hop between
his branch and colleagues', often with uncommitted work; pull a fix commit from another branch onto his.

Sources are primary (vendor docs, source, issue trackers). `(?)` = not confirmed. Step counts are **mine**, counted from
the documented keys per `docs/research/ux-principles.md` §1.3; they are not measured.

---

## 1. Switching branch

| Tool | Steps, A → B (find among many) | Dirty tree | Key / where | Source |
|---|---|---|---|---|
| **lazygit** | Branches panel, `/` filter, type, `space`: **3** (+ typing, + reaching the panel). Or `c`, type name, `⏎`: **3**; `c` then `-` = previous branch | Offers to autostash (stash, switch, pop) with one `⏎` confirm → **+1** | Dedicated key `space`, shown in the panel's hint bar | Keys: https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md ; autostash exists on checkout, absent on new-branch: https://github.com/jesseduffield/lazygit/issues/3866 |
| **VS Code** | Click branch name in status bar (or `⇧⌘P` "Git: Checkout to"), type to filter, `⏎`: **3** | Docs: "Git might prevent the switch"; commit or stash first. A `git.autoStash`-style setting is not on that page `(?)` | Status bar + palette | https://code.visualstudio.com/docs/sourcecontrol/branches-worktrees |
| **JetBrains** | Branches widget/popup, type to search (`Ctrl+F` refocuses search), select branch, **Checkout**: **3–4**. Dedicated shortcut not documented `(?)` | **Smart Checkout** shelves, switches, unshelves; Force Checkout overwrites ("you will lose them"). Stash instead of shelve is a setting | Popup menu | https://www.jetbrains.com/help/idea/manage-branches.html |
| **Magit** | `b b`, completing-read with the name, `⏎`: **3** | Plain git refusal; auto-stash-on-switch was requested (closed, no linked change) — see §3 | Transient menu `b`, itself a visible key menu | `b b`: magit issue threads (manual page 404'd, §5) https://github.com/magit/magit/issues/2906 ; https://github.com/magit/magit/issues/4013 |
| **Sublime Merge** | Palette "Checkout Branch…" (`checkout_branch` command), type, `⏎`: **3**. Sidebar gesture `(?)` | `(?)` | Palette; release notes: "Combined all checkout commands into a single command" | https://www.sublimemerge.com/docs/command_palette ; https://www.sublimemerge.com/download (changelog) |
| **jj** | `jj new <rev>` / `jj edit <rev>`: **1** command | **No dirty-tree problem by design**: "the working-copy commit gets automatically amended by the next `jj` command" | CLI | https://docs.jj-vcs.dev/latest/tutorial/ |
| **GitHub Desktop** (complaint only) | — | "Bring my changes" / "Leave my changes" prompt; users report changes lost when it fails | — | https://github.com/desktop/desktop/issues/8579 |
| GitUp, Fork, Tower | not researched `(?)` | | | |

## 2. Cherry-picking onto the current branch

| Tool | Steps, "see the commit" → "on my branch" | Several commits | Key / where | Source |
|---|---|---|---|---|
| **lazygit** | `C` on commit, go to own branch's commits view, `V`: **2 keys + navigation** (copy/paste model — works across branches without switching views of HEAD) | Mark several with `C`, one `V` pastes all; `Esc` / `Ctrl+R` clears | Dedicated keys `C`/`V` | Keybindings link above; `resetCherryPick: <ctrl+r>`: https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md |
| **Magit** | `A A` on commit at point (prompt defaults to it), `⏎`: **2–3** | "If the region selects multiple commits, then those are copied, without prompting." `A a` = apply without committing | Transient `A` | https://docs.magit.vc/magit/Cherry-Picking.html |
| **JetBrains** | Git tool window `Alt+9`, Log tab, select commit, toolbar **Cherry-pick**: **2** once in the Log. "will apply and commit" | Multi-select in log `(?)` (docs silent) | Toolbar button | https://www.jetbrains.com/help/idea/apply-changes-from-one-branch-to-another.html |
| **Sublime Merge** | Context menu on commit → Cherry Pick: **2** | "Cherry Pick multiple commits via the commit graph and context menu" (Build 2059, 23 Jul 2021); later "skip empty commits" option | Context menu | https://www.sublimemerge.com/download ; before: "only one commit to be able to be cherry picked at a time" https://github.com/sublimehq/sublime_merge/issues/480 |
| **jj** | `jj duplicate <rev>` (docs as summarised by fetch tool; exact destination flag `(?)`) | revset | CLI | https://docs.jj-vcs.dev/latest/tutorial/ |
| **VS Code / GitLens** | not confirmed `(?)` — core docs page does not cover cherry-pick | | | https://code.visualstudio.com/docs/sourcecontrol/branches-worktrees |

## 3. Praise and complaints (sourced only)

- **Dirty-tree switch is the pain point everywhere.** Requested for Magit (https://github.com/magit/magit/issues/4013),
  for lazygit's new-branch path (https://github.com/jesseduffield/lazygit/issues/3866); lost work in GitHub Desktop's
  auto-carry (https://github.com/desktop/desktop/issues/8579). JetBrains ships it as the default "Smart Checkout".
- **Multi-commit cherry-pick was a top request** in Sublime Merge until 2021 (#480 above; also merge-commit picks
  https://github.com/sublimehq/sublime_merge/issues/168, drag-and-drop https://github.com/sublimehq/sublime_merge/issues/1194).
- Praise quotes for any tool: **not collected** `(?)` — only feature requests were read.

## 4. Pattern

- Switch = **pick from a filtered list, one confirm: 3 steps** in every tool read. The branch name is typed, not scrolled to.
- Dirty tree: the good ones **stash-switch-pop behind one prompt** (lazygit, JetBrains); the failure mode is losing
  changes silently when the pop/carry fails (GitHub Desktop).
- Cherry-pick: **a dedicated key or top-level menu item, 2 steps**; multi-select by marking or region.

## 5. Access failures

- `magit.vc/manual/magit/Branch-Commands.html`, `…/Cherry-Picking.html`, `docs.magit.vc/magit/The-Branch-Transient.html`: 404. `b b` therefore rests on issue threads, not the manual.
- Sublime Merge key-bindings page lists no checkout/cherry-pick default key.

## 6. What Graft does today (design docs only; source not read)

- Rule: cherry-pick is a "rare verb" behind `.`, no dedicated key — `docs/design/02-keymap.md:145-148`. The usability
  review calls it daily (`docs/reviews/m6-usability.md:8`), contradicting that rule.
- Checkout: `Enter` on a sidebar branch/tag/stash — `docs/design/02-keymap.md:199`; from the graph, `.` menu
  `Checkout <branch>` — `docs/design-notes/m5-history-and-ghost-preview.md:246`; behind a Ghost Preview (`:9-10`).
- Dirty tree on switch: refused with `DirtyTree { count }`, "S stashes N changed files first" —
  `docs/design-notes/m5-history-and-ghost-preview.md:88`. So stash is a separate step, not a stash-switch-pop.
- Runs `switch <branch>` / `switch --detach` (`:93`); cherry-pick runs `cherry-pick --no-edit <oid>…` (`:91`) — the
  `…` suggests several oids; whether the UI can select several `(?)`.
- Finding a branch among many: whether the sidebar `/` filter reaches branches `(?)` — not in the sections read.
- Measured counts: checkout `. j ⏎ y` = 4 (+ finding the row); cherry-pick `. j×6 ⏎ y` = 9 presses —
  `docs/reviews/m6-usability.md:52-53`.

## 7. Counts to judge against

| Task | Best found | Tool | Graft today |
|---|---|---|---|
| Switch to a named branch among many | 3 (filter, type, confirm) | lazygit, VS Code, Magit, SM | 4 + finding the row (`?` filter) |
| Switch with a dirty tree | 4 (3 + one autostash confirm) | lazygit, JetBrains | refused; stash separately (`S` …) then switch — count `(?)` |
| Back to previous branch | 3 (`c` `-` `⏎`) | lazygit | `(?)` |
| Cherry-pick one commit | 2 | JetBrains, SM, Magit | 9 presses |
| Cherry-pick several | marks + 1 | lazygit, Magit, SM | `(?)` |
