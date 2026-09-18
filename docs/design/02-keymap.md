# Graft: Design — Keymap

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [View mockups](01-view-mockups.md) · Next: [Graph layout algorithm](03-graph-layout-algorithm.md)

---

## 2. Keymap

### 2.1 Principles

- **Mnemonic first, one meaning per verb.** A letter is assigned to one verb
  (commit, merge, rebase, branch, tag, stash, fetch, pull, push, discard,
  edit, ignore) and that verb is never reassigned to something else anywhere
  the letter is live. Vim movement (`j k h l`, `g G`, `/`) is additional and
  never collides with a mnemonic, per PROMPT.md §3.
- **Focus-scoped panes may reuse a letter for a *different* vocabulary, never
  a conflicting one.** The hint bar always shows what's active, so this isn't
  a hidden mode: it's the same rule modal editors use for a command palette
  that means something different depending on where the cursor is. Four
  places do this deliberately:
  - The **conflict resolver** binds `o`/`t`/`b` to ours/theirs/both — `t`
    (normally "new tag") doesn't apply mid-resolve, so it's free.
  - The **interactive rebase editor** binds `p r e s f d` to
    pick/reword/edit/squash/fixup/drop — git's own convention for `rebase -i`
    todo lines, which every Git user already knows. `r` here is "reword," not
    "rebase," because you're already inside a rebase.
  - The **operation banner**, when focused, binds `c s a` to
    Continue/Skip/Abort — "commit," "stage" and "amend" aren't valid while an
    operation is in progress, so the letters are free and match the buttons'
    initials exactly.
  - The **first-run / empty state** (§2.13) binds `o`/`i`/`c` to
    Open/Init/Clone — no repo is open yet, so the whole repo vocabulary
    (`c` commit included) is off duty.
- **Bulk variants use a modifier, not a different letter.** `Ctrl+A` stages
  everything; plain `a` stays "amend" everywhere. This avoids growing the
  letter bank for "the same verb, but all of them."
- **Rare verbs live behind `.` (context actions) or the palette, not a
  dedicated key.** Rename, delete branch/tag, cherry-pick, reset to here,
  explain/fix a Mistake Radar marker, create branch/tag from an arbitrary
  commit, copy SHA and open-in-browser are all reachable in at most two
  steps without owning a letter.
- **Every binding is remappable** in `[keys]` in `config.toml`; Graft detects
  and warns about remap collisions at startup rather than picking silently.

### 2.2 Global (every pane)

| Key | Action |
|---|---|
| `Ctrl+P`, `:` | Command palette |
| `?` | Context help for the focused pane |
| `Tab` / `Shift+Tab` | Next / previous pane |
| `Esc` | Back out / close overlay / cancel |
| `Ctrl+Z`, `u` | Undo (ADR-0002) |
| `Ctrl+Shift+Z`, `U` | Redo |
| `Ctrl+R` | Manual refresh |
| `q`, `Ctrl+C` | Quit (confirms if an operation is in progress) |
| `↑ ↓ ← →` | Move selection / scroll |
| `j k h l` | Move down/up/left/right (vim, movement only) |
| `g` / `G` | Jump to top / bottom (vim) |
| `/` | Find in the focused list |
| `Enter` | Open / drill in / activate |
| `.` | Context actions menu for the selected item |
| `Space` | Toggle stage where staging applies, else expand/collapse |

`Ctrl+Shift+Z` is not a reliable binding: many terminals collapse it to the
same byte sequence as `Ctrl+Z`, so Graft cannot tell the two apart on every
terminal it targets. `U` is therefore not just a convenience alias — it's
the binding that actually works everywhere, and the one the help overlay
and any redo prompt should lead with.

### 2.3 Graph / history pane

| Key | Action |
|---|---|
| `c` | Commit |
| `a` | Amend |
| `m` | Merge selected branch into current |
| `r` | Rebase current onto selection |
| `b` | New branch from selected commit |
| `t` | New tag at selected commit |
| `d` | Discard (only on the Uncommitted-changes row) |
| `s` | Stage (alias for `Space` in this context) |
| `S` | Stash… |
| `f` | Fetch |
| `p` | Pull |
| `P` | Push |
| `i` | Add to `.gitignore` |
| `e` | Open in `$EDITOR` |
| `Enter` (on a sidebar branch/tag/stash) | Checkout / open |
| `M` | Toggle Map view (§5.1) |
| `F` | Toggle Focus mode on the selected commit (§5.2) |
| `O` | Toggle the Overview strip (§5.3) |
| `Space` (on a merge) | Fold its merged branch into it / unfold (PROMPT.md §1.3; m1/15) |
| `1` | Toggle first-parent view (m1/15) |
| `H` | Hide / show the stash in the graph (m1/15) |
| `Enter` (on a graph row) | Open the detail pane on that commit (peek (M1) → open details (M2), ADR-0023, m2-details-and-diffs/10) |
| `y` | Copy the selected commit's SHA to the clipboard (OSC 52) (ADR-0023, m1/45) |

### 2.4 Working tree / staging (detail pane)

| Key | Action |
|---|---|
| `Space` | Stage/unstage the file, hunk or line under the cursor |
| `s` | Stage (alias) |
| `Ctrl+A` | Stage all |
| `d` | Discard, with confirmation and an undoable backup |
| `e` | Open in `$EDITOR` at the current line |
| `i` | Add to `.gitignore` |
| `c` / `a` | Commit / amend (as above — same verbs, same keys) |

### 2.5 Diff pane (view options; active wherever a diff is shown)

| Key | Action |
|---|---|
| `w` | Toggle ignore-whitespace |
| `W` | Toggle word-diff granularity |
| `v` | Toggle unified / side-by-side |
| `[` / `]` | Decrease / increase context lines |
| `<` / `>` | Shrink / grow the detail pane on the focused divider, 4 cells a step; no-op with no divider shown (ADR-0028, m2-details-and-diffs/10b) |

### 2.6 Conflict resolver

| Key | Action |
|---|---|
| `o` | Take ours (this hunk) |
| `t` | Take theirs (this hunk) |
| `b` | Take both |
| `e` | Edit the result inline |
| `n` / `N` | Next / previous conflicted hunk or file |
| `Enter` | Mark file resolved |
| `M` | Launch the configured external mergetool |
| (banner focused) `c` `s` `a` | Continue / Skip / Abort |

### 2.7 Interactive rebase editor

| Key | Action |
|---|---|
| `p` `r` `e` `s` `f` `d` | pick / reword / edit / squash / fixup / drop |
| `Alt+↑` / `Alt+↓` | Reorder (also: drag with the mouse) |
| `Enter` | Start the rebase |
| `Esc` | Cancel |

### 2.8 Blame

| Key | Action |
|---|---|
| `Enter` | Jump to the commit in the graph |
| `n` / `N` | Older / newer change to this line |
| `^` | Blame the parent revision (walk up, like `git blame` at a prior commit) |
| `Space` | Toggle the age heatmap |

### 2.9 Stash list

| Key | Action |
|---|---|
| `Enter` | Apply |
| `p` | Pop (apply, then drop) |
| `d` | Drop (undoable) |
| `b` | Create a branch from this stash |
| `Space` | Preview the diff |

### 2.10 Commit message editor

| Key | Action |
|---|---|
| `Ctrl+Enter` | Commit |
| `Tab` | Move between subject / body / options |
| `Ctrl+E` | Open `$EDITOR` instead |
| `Ctrl+S` | Insert a sign-off trailer |
| `Ctrl+Shift+C` | Insert a co-author trailer |
| `Esc` | Cancel |

### 2.11 Undo journal browser

| Key | Action |
|---|---|
| `↑ ↓`, `j k` | Move |
| `Enter` | Revert to just before this entry (Ghost Preview first) |
| `Esc` | Close |

### 2.12 Command palette

| Key | Action |
|---|---|
| type | Fuzzy-filter actions, branches, tags, files and commits |
| `↑ ↓` | Move |
| `Enter` | Run |
| `Esc` | Close |

### 2.13 First-run / empty state

A separate screen (no repo is open yet), with its own small vocabulary:

| Key | Action |
|---|---|
| `o` | Open… |
| `i` | Init |
| `c` | Clone… |
| `Enter` (on a recent-repo row) | Open that repo |
| `?` | Replay/skip the first-run tour |

### 2.14 Credential prompt

| Key | Action |
|---|---|
| `Tab` | Move between fields |
| `Enter` | Submit |
| `Esc` | Cancel (fails the operation cleanly, nothing left half-authenticated) |
| `Ctrl+U` | Clear the focused field |

### 2.15 Mouse (every pane, always first-class — PROMPT.md §3)

- Click selects; double-click opens; right-click opens the same context menu
  as `.`.
- The scroll wheel works in every scrollable area.
- Divider drags resize panes; sizes are remembered per repo.
- Drag a branch badge onto a commit or another branch to open Merge
  into…/Rebase onto…/Reset to here/Cherry-pick, each behind a Ghost Preview.
- In the graph pane, hovering a row whose subject is truncated reveals the
  full subject in the footer, the same reveal `Enter` gives without a mouse
  (§2.3); moving off the row restores the hint bar. There is no tooltip
  (ADR-0023 — supersedes this section's "same tooltip content" and PROMPT.md
  §1.3's hover line): the full message, parents and containing branches live
  in the detail pane, M2.
- Mouse capture never sends `XTSHIFTESCAPE` with `n=1` — `Shift`+drag stays
  the terminal's native text selection, never captured by Graft (ADR-worthy
  rule already fixed by PROMPT.md §3; noted here because it constrains how
  the `term` module configures crossterm's mouse capture).

---
