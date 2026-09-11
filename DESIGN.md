# Graft: Design

This is the design produced by PROMPT.md section 10, step one: view mockups, the
full keymap, the graph layout algorithm, terminal-capability detection, and the
undo journal. No code — this document is reviewed by the user before M0 starts.

It uses the vocabulary fixed in `CONTEXT.md` (default branch, lane, rendering
tier, Ghost Preview, Mistake Radar, marker, ref badge, operation, undo journal,
Command Log, job) and follows the decisions in `docs/adr/0001`–`0005`. Where a
choice here narrows something PROMPT.md left open, it's called out as a design
decision so the checkpoint review can push back on it specifically.

---

## Contents

1. [View mockups](#1-view-mockups)
2. [Keymap](#2-keymap)
3. [Graph layout algorithm](#3-graph-layout-algorithm)
4. [Terminal-capability detection](#4-terminal-capability-detection)
5. [Undo journal design](#5-undo-journal-design)
6. [Open design questions](#6-open-design-questions)
7. [Architecture](#7-architecture)
8. [Proposed changes](#8-proposed-changes)

---

## 1. View mockups

Sixteen views. The first three show the same graph view at the three
responsive breakpoints from PROMPT.md §2; the rest are shown once, at whatever
width best fits. Colour, weight and italics don't render in ASCII — where they
matter, a legend line says what would be a colour or style in the real UI.
Lane colour is noted with a letter suffix on the node (`●a`, `●b`, …) standing
in for a distinct hue; ref badges are `[name]`; Mistake Radar markers are `⚠`
(warning) or `ⓘ` (info).

### 1.1 Main graph view — wide (160+ columns, three-column layout)

```
╭─ ◆ graft ─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ─────────────────────────────────────── ⌃P Commands ─╮
│ BRANCHES         ⌕ │ GRAPH   REF            SUBJECT                          AUTHOR  WHEN   SHA      │ COMMIT a1b2c3d                  │
│ ● main       ↑2    │ ◌                      Uncommitted changes (3 staged)                           │ Rui M · 2 min ago              │
│ ▾ feature/         │ ●a      [main]         Fix token refresh race            RM      2m     a1b2c3d │ parents 9f8e7d6                 │
│    auth            │ │ ●b    [feature/auth] Add OAuth provider                AK      1h     7f6e5d4 │ Contained in: main              │
│    billing         │ ◉a──╯b  ⚠ [main]       Merge 'feature/login'             RM      3h     3c2b1a0 │                                 │
│ REMOTES            │ ●a                     Bump deps                        RM      1d     0a9b8c7 │ Fix token refresh race          │
│ ▸ origin           │ ●a      ⓘ              Merge branch 'main' of origin     RM      2d     9f8e7d6 │                                 │
│ TAGS               │ ●a      [v2.3.0]       Release 2.3.0                     AK      4d     88ee11c │ ▾ src/auth/token.rs   +12 −4    │
│ STASHES (2)        │ ⋮                                                                                │   @@ -40,7 +40,15 @@ fn refresh()│
│ WORKTREES          │                                                                                  │  40   let token = self.cache;   │
│ SUBMODULES         │                                                                                  │  41 - if token.is_expired() {   │
│                    │                                                                                  │  41 + if token.is_expired()     │
│                    │                                                                                  │  42 +     && !refreshing {      │
╰─ ↑↓ move  ⏎ open  space stage  c commit  m merge  r rebase  ⚠ foxtrot merge — e Explain  ? help ───────╯
```

- Left: sidebar tree, filter box at the top (`⌕`).
- Centre: the graph — one row per commit, ref badges inline, Mistake Radar
  marker column between the graph and the ref badge.
- Right: detail pane — commit metadata, containing branches, file tree and
  diff with a pinned hunk header.
- Top bar: repo name, path, current branch with ahead/behind, dirty count,
  and the command-palette hint. A background-task spinner (not shown) lives
  just left of `⌃P Commands` while a job is running.

### 1.2 Main graph view — medium (100–159 columns)

Sidebar collapses to icons; detail pane moves under the graph instead of
beside it.

```
╭─ ◆ graft ─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ──────────────────────── ⌃P ─╮
│ ⌕│ GRAPH  REF            SUBJECT                         AUTHOR  WHEN   SHA  │
│ ●│ ◌                     Uncommitted changes (3 staged)                     │
│ ▾│ ●a     [main]         Fix token refresh race           RM      2m    a1b2c│
│ ▸│ │ ●b   [feature/auth] Add OAuth provider                AK      1h    7f6e5│
│ ▸│ ◉a──╯b ⚠ [main]       Merge 'feature/login'             RM      3h    3c2b1│
│ ▸│ ●a                    Bump deps                         RM      1d    0a9b8│
├──┴────────────────────────────────────────────────────────────────────────────┤
│ COMMIT a1b2c3d · Rui M · 2 min ago · parents 9f8e7d6 · Contained in: main    │
│ Fix token refresh race                                                       │
│ ▾ src/auth/token.rs +12 −4    @@ -40,7 +40,15 @@ fn refresh()                │
│  40   let token = self.cache;      41 - if token.is_expired() {              │
╰─ ↑↓ move  ⏎ open  space stage  c commit  m merge  r rebase  ? help ──────────╯
```

### 1.3 Main graph view — narrow (under 100 columns, exactly 80×24)

A single pane with a tab strip. `Tab`/`Shift-Tab` still switch panes; here
that means switching tabs.

```
╭◆ graft ~/code/api ⎇ main↑2↓0 ●3 ─────────────────────────────────── ⌃P ─╮
│[Graph]  Changes  Detail                                                │
│ GRAPH   SUBJECT                              WHEN   SHA                │
│ ◌       Uncommitted changes (3 staged)                                 │
│ ●a      Fix token refresh race               2m     a1b2c3d            │
│ │●b     Add OAuth provider                   1h     7f6e5d4            │
│ ◉a─╯b ⚠ Merge 'feature/login'                3h     3c2b1a0            │
│ ●a      Bump deps                            1d     0a9b8c7            │
│ ●a      Merge branch 'main' of origin  ⓘ     2d     9f8e7d6            │
│ ●a      Release 2.3.0                        4d     88ee11c            │
│ ⋮                                                                       │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
│                                                                          │
╰ ↑↓ move ⏎ open ⇥ next tab space stage c commit ? help ──────────────────╯
```

### 1.4 Working tree / staging view (detail pane, "Changes" tab)

```
╭ CHANGES ── ⎇ main ── 2 staged, 1 unstaged, 1 untracked ────────────────────╮
│ STAGED (2)                          │ src/auth/token.rs            +12 −4  │
│  [x] src/auth/token.rs      +12 −4  │ ───────────────────────────────────  │
│  [x] src/auth/session.rs     +3 −0  │  38   fn refresh(&mut self) {        │
│ UNSTAGED (1)                        │  39       let token = self.cache;   │
│  [ ] src/auth/mod.rs          +1 −1 │  40 - if token.is_expired() {       │
│ UNTRACKED (1)                       │  40 + if token.is_expired()         │
│  [ ] src/auth/refresh_test.rs (new) │  41 +     && !refreshing {          │
│                                      │  42           self.do_refresh();   │
│                                      │  43       }                        │
│                                      │                                    │
╰ space stage/unstage  s stage  ⌃A stage all  d discard  e edit  c commit ──╯
```

Hunks and lines expand under a file with the same `[x]`/`[ ]` checkboxes,
staged at hunk or line granularity; `Space` toggles whatever is under the
cursor (file, hunk or single line).

### 1.5 Conflict resolver (three-pane)

```
╭ RESOLVING CONFLICTS ── rebase 3/7 ── 2 files ── src/auth/token.rs (1/2) ───╮
│ OURS (HEAD)            │ RESULT (edit here)      │ THEIRS (feature/auth)  │
│ fn refresh(&mut self){ │ fn refresh(&mut self){  │ fn refresh(&mut self){ │
│   if expired() {       │<<<<<<< ours ────────── │   if is_expired() {    │
│     self.renew();      │   if expired() {        │     self.do_refresh(); │
│   }                    │     self.renew();       │   }                    │
│ }                      │   }                      │                        │
│                         │======================= │                        │
│                         │   if is_expired() {      │                        │
│                         │     self.do_refresh();   │                        │
│                         │   }                      │                        │
│                         │>>>>>>> theirs ────────── │                        │
├──────────────────────────────────────────────────────────────────────────┤
│ CONFLICTED FILES: ⚠ src/auth/token.rs   ⚠ src/auth/mod.rs                │
╰ o take ours  t take theirs  b take both  e edit  n next hunk  ⏎ resolved ─╯
```

Base is available on demand (a fourth column or a toggle) when a merge base
exists; "Launch external mergetool" (`M`) hands the file to the user's
configured `git mergetool`.

### 1.6 Command palette

```
        ╭ ⌃P  rebase                                                ╮
        │ ACTIONS                                                   │
        │  Rebase current branch onto…                    r         │
        │  Rebase (interactive) current branch onto…                │
        │ BRANCHES                                                   │
        │  feature/rebase-cleanup                                    │
        │ COMMANDS RUN RECENTLY                                      │
        │  git rebase main                                           │
        ╰ ↑↓ move  ⏎ run  esc close ─────────────────────────────────╯
```

Fuzzy search over actions, branches, tags, files and commits at once, grouped
by kind, ranked by match quality then recency.

### 1.7 Ghost Preview (overlaid on the graph, before confirming)

```
│ ●a      [main]          Fix token refresh race            RM   2m   a1b2c│
│ ┊●a┈    [main] (ghost)  Rebase feature/auth onto main      —    —    —   │
│ ┊│┊●b┈  (ghost)         Add OAuth provider                 AK   1h   —   │
│ │ ●b    [feature/auth]  Add OAuth provider                 AK   1h  7f6e5│
│ ◉a──╯b                  Merge 'feature/login'               RM   3h  3c2b1│
╰ Preview: rebase feature/auth onto main (2 commits replayed) ──────────────╮
│ git rebase main                                                          │
│                                              [ Confirm ]   [ Cancel ]    │
╰────────────────────────────────────────────────────────────────────────╯
```

Ghost nodes/edges are drawn dashed and dimmed (`┊`/`┈` stand in for a lower
alpha, dashed stroke in the real renderer); real nodes stay solid so the
before/after reads as one continuous picture, not two separate graphs.

### 1.8 Mistake Radar marker + Explain

```
│ ◉a──╯b  ⚠ [main]  Merge 'feature/login'                    RM   3h  3c2b1│
├─────────────────────────────────────────────────────────────────────────┤
│ ⚠ FOXTROT MERGE                                                          │
│ This merge's first parent (9f8e7d6) is not main's previous tip           │
│ (0a9b8c7). main's first-parent line was rewritten — usually by pulling   │
│ on main and pushing the result, rather than merging in the normal way.   │
│                                                              [ Explain ] │
│ [main]: 0a9b8c7 ──╮                                                     │
│                    ◉ 3c2b1a0  (this merge)                              │
│         9f8e7d6 ──╯                                                     │
│                                                                [ Fix… ] │
╰───────────────────────────────────────────────────────────────────────╯
```

`Fix…` opens the options from PROMPT.md §1.4 (revert with `-m 1`, reset with
a backup ref, or rebase instead), each behind its own Ghost Preview.

### 1.9 Interactive rebase editor

```
╭ INTERACTIVE REBASE onto main (4 commits) ── drag to reorder, Alt+↑/↓ ─────╮
│  pick    a1b2c3d  Fix token refresh race                                 │
│  fixup   7f6e5d4  fixup! Fix token refresh race                          │
│  reword  3c2b1a0  Add OAuth provider                                     │
│  squash  0a9b8c7  Add OAuth provider tests                               │
│                                                                            │
│  fixup! commits detected — autosquash applied automatically              │
╰ p pick  r reword  e edit  s squash  f fixup  d drop  ⏎ start  esc cancel ─╯
```

### 1.10 Undo journal browser

```
╭ UNDO JOURNAL ── ~/code/api ──────────────────────────────────────────────╮
│ ▸ 2m ago    Rebase feature/auth onto main         3 refs · backup saved  │
│   14m ago   Discard src/auth/refresh_test.rs       1 file · backup saved │
│   1h ago    Commit "Add OAuth provider"            HEAD moved            │
│   3h ago    Merge 'feature/login' into main        1 ref · backup saved  │
│   1d ago    Push main → origin/main                not locally undoable │
╰ ⏎ revert to before this  (Ghost Preview shown first)  esc close ─────────╯
```

### 1.11 Stash list + diff

```
╭ STASHES (2) ──────────────────────────────────────────────────────────────╮
│  stash@{0}  "WIP on main: token refresh"        2h ago   3 files          │
│  stash@{1}  "quick fix before pull"              1d ago   1 file           │
├──────────────────────────────────────────────────────────────────────────┤
│ stash@{0}  src/auth/token.rs  +8 −2                                       │
│  @@ -12,6 +12,12 @@ struct TokenCache                                    │
│  +   retry_count: u8,                                                    │
╰ ⏎ apply  p pop  d drop (undoable)  b branch from stash  space preview ────╯
```

### 1.12 Blame view

```
╭ BLAME src/auth/token.rs ── age heatmap ─────────────────────────────────╮
│ a1b2c3d  RM  2m    40   let token = self.cache;                        │
│ a1b2c3d  RM  2m    41   if token.is_expired() && !refreshing {         │
│ 7f6e5d4  AK  1h    42       self.do_refresh();                         │
│ 0a9b8c7  RM  1d    43   }                                               │
│ 88ee11c  AK  6mo   44   self.notify_listeners();                       │
╰ ⏎ open commit  n/N older/newer change  ^ blame parent  space heatmap ───╯
```

Colour (not shown) ages from a bright "recent" hue to a dim "old" hue, redundant
with the printed relative age so it never depends on colour alone.

### 1.13 Operation banner (mid-rebase, focused)

```
╭ ◆ graft ─ ~/code/api ─ ⎇ (rebasing) ─────────────────────────────────────╮
│ ⚠ REBASING  step 3 of 7 — conflict in src/auth/token.rs                 │
│                                    [ Continue ]  [ Skip ]  [ Abort ]     │
╰ c continue  s skip  a abort ─────────────────────────────────────────────╯
```

Shown in place of the ahead/behind indicator in the top bar whenever
`rebase-merge/`, `MERGE_HEAD`, `CHERRY_PICK_HEAD` or `BISECT_LOG` exist; also
the first thing offered on relaunch if Graft (or the machine) died mid-op.

### 1.14 Credential prompt (in-app askpass)

```
        ╭ Credentials for https://github.com ───────────────────╮
        │ Username  git                                          │
        │ Password  ************************                     │
        │                                                         │
        │                          [ Submit ]      [ Cancel ]     │
        ╰ ⇥ next field  ⏎ submit  esc cancel ───────────────────╯
```

`GIT_TERMINAL_PROMPT=0` is set on every git invocation, and `GIT_ASKPASS`
points at a small helper that talks back to Graft over a local socket/pipe so
the prompt renders in-app instead of falling through to a blocked terminal
prompt or failing silently.

### 1.15 First-run / empty state

```
╭ ◆ graft ──────────────────────────────────────────────────────────────────╮
│                                                                            │
│                 This folder isn't a Git repository yet.                  │
│                                                                            │
│                       [ o  Open… ]   [ i  Init ]   [ c  Clone… ]         │
│                                                                            │
│                 Recent repos                                              │
│                   ~/code/api                                              │
│                   ~/code/graft                                            │
│                                                                            │
╰────────────────────────────────────────────────────────────────────────╯
```

A 30-second, skippable first-run tour runs the first time a real repo opens
(not shown here — it's a sequence of four callouts pointing at the hint bar,
the graph, the sidebar and `Ctrl+P`, dismissible with `Esc` or `?` at any
point and never shown again unless replayed from the palette).

### 1.16 Context help overlay (`?`)

```
╭ HELP — Graph view ────────────────────────────────────────────────────────╮
│ MOVE      ↑↓/j k  Enter open   Esc back   Tab next pane                  │
│ ACT       c commit   m merge   r rebase   b branch   t tag   S stash     │
│ REMOTE    f fetch   p pull   P push                                       │
│ UNDO      ⌃Z / u undo   ⌃⇧Z / U redo                                     │
│ FIND      / search   ⌃P command palette                                  │
│ ITEM      . context actions for the selected row                          │
│                                                        remap in config → │
╰ esc close ─────────────────────────────────────────────────────────────╯
```

`?` always shows the keymap for whichever pane has focus — it's the same
table as §2 below, filtered to the current context, so the hint bar and this
overlay are never out of sync with each other or with reality.

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
  that means something different depending on where the cursor is. Three
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
- **Bulk variants use a modifier, not a different letter.** `Ctrl+A` stages
  everything; plain `a` stays "amend" everywhere. This avoids growing the
  letter bank for "the same verb, but all of them."
- **Rare verbs live behind `.` (context actions) or the palette, not a
  dedicated key.** Rename, delete branch/tag, cherry-pick, reset to here,
  create branch/tag from an arbitrary commit, copy SHA and open-in-browser
  are all reachable in at most two steps without owning a letter.
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
- Hover shows the same tooltip content that's reachable without a mouse
  (PROMPT.md §1.3); nothing is mouse-only.
- Mouse capture never sends `XTSHIFTESCAPE` with `n=1` — `Shift`+drag stays
  the terminal's native text selection, never captured by Graft (ADR-worthy
  rule already fixed by PROMPT.md §3; noted here because it constrains how
  the `term` module configures crossterm's mouse capture).

---

## 3. Graph layout algorithm

Lives entirely in the `graph` crate: a pure library with no terminal code
(PROMPT.md §1.2), taking a commit stream in and producing rows with lane
assignments and edges out. Three renderers (pixel/unicode/ascii, §4) consume
its output; none of them feed back into it.

### 3.1 Data model

```
Commit         { id, parents: [CommitId; ≤N], author, committer, date, message }
BranchIdentity { stable hash of a branch name — the source of lane colour,
                 independent of which column the branch currently occupies }
Lane           { index: usize,                 // the column
                 identity: BranchIdentity,      // for colour, stays fixed for the lane's life
                 waiting_for: CommitId,          // the next commit that continues this lane
                 reserved: bool }                // true for lane 0 and integration-branch lanes:
                                                  // never handed to the free pool
Row            { commit: Commit,
                 lane: usize,                    // this commit's own column
                 incoming: [usize],              // lanes that converge on this row (closing edges)
                 outgoing: [(usize, CommitId)],   // lanes leaving this row, one per parent
                 marker: Option<MistakeRadarKind> }
LayoutCursor    { walker_state, active: [Option<Lane>], waiting: Map<CommitId,[usize]>,
                  free_pool: [usize], rows_emitted: usize }
                                                  // opaque, resumable snapshot of everything below
```

### 3.2 Algorithm overview (prose)

**1. Resolve the default branch and any integration branches**, per
ADR-0001's ordered precedence. This runs once per repo, off-thread, and is
cached. The default branch's tip seeds lane 0; each integration branch's tip
seeds its own reserved lane adjacent to lane 0. Both kinds of lane are marked
`reserved`, so the general allocator in step 3 never hands them to another
branch even when they're temporarily idle.

**2. Walk commits in topological order** (date order and first-parent-only
are alternate walk strategies selected by the same interface, per PROMPT.md
§1.2). "Topological" means: a commit is only emitted once every one of its
children that's part of the visible walk has already been emitted — the
standard in-degree-counting topological sort git itself uses for
`--topo-order`. Multiple commits can be ready to emit at once (parallel
branches); break the tie by commit date (most recent first) and, if dates
tie exactly, by commit id, so the order is fully deterministic.

**3. For each commit, as it's emitted, assign it a lane:**

- Look up every lane whose `waiting_for` is this commit's id (`waiting` map).
  There's normally exactly one; there's more than one exactly where two or
  more branches share this commit as an ancestor before diverging further
  back — a fork point, read backwards. There's zero for a new root reached by
  a fresh tip that hasn't merged into anything seen yet (open a new lane).
- The commit's own lane is the lowest-indexed lane among those found, with
  one override: if the default branch is among them, its lane (0) always
  wins, keeping lane 0 straight for its entire life as PROMPT.md §1.2
  requires. Every other lane found here **closes into** this row — drawn as
  a converging edge, then not carried further forward under its own column.
- Assign outgoing edges, one per parent:
  - The **first parent** continues in the commit's own lane. This is what
    keeps a first-parent chain in one column for its whole life.
  - Each **additional parent** (a merge) gets a lane: reuse the
    lowest-indexed column in `free_pool`, or open a new column (current
    max + 1) if the pool is empty. Never a `reserved` lane.
  - Update `waiting_for` on each of these lanes to the corresponding parent
    id (or close the lane immediately if the parent was already emitted
    earlier in the walk — happens when a lane's target turns out to be an
    already-visible commit, e.g. two merges into the same ancestor).
- Any lane with no more work (a root commit, or a parent already satisfied)
  is freed — but placed in a **pending** free list, not `free_pool` itself.

**4. End of row: promote pending frees into `free_pool`.** This is what makes
lane reuse never happen on the same row that frees it (PROMPT.md §1.2): a
lane vacated while processing row N only becomes available starting row
N+1's allocation step.

**5. Compaction.** If the number of concurrently open lanes exceeds
`graph.max_lanes` (default 8), fold the least-recently-touched non-reserved
lanes behind a single `+N` column (§3.5) rather than growing the graph
wider.

### 3.3 Pseudocode

```
fn resolve_lanes(default_tip, integration_tips, tips_to_show) -> initial state:
    active[0] = Lane { identity: default_branch_identity, waiting_for: default_tip, reserved: true }
    for (i, tip) in integration_tips.enumerate():
        active[1 + i] = Lane { identity: hash(tip.branch_name), waiting_for: tip, reserved: true }
    for tip in tips_to_show where tip not in (default_tip, integration_tips):
        push_to_heap(tip)   # opened lazily, on first emission, in step below
    waiting = reverse_index(active)   # commit id -> [lane indices]
    return LayoutCursor { active, waiting, free_pool: [], rows_emitted: 0 }

fn layout_chunk(cursor, walker, chunk_size) -> [Row]:
    rows = []
    pending_free = []
    for _ in 0..chunk_size:
        commit = walker.next_topological()     # None when history is exhausted
        if commit is None: break

        incoming = waiting.remove(commit.id) or []
        lane = min(incoming) if incoming else open_new_lane(commit)
        if default_branch_lane in incoming:
            lane = 0                             # lane 0 always wins

        for l in incoming where l != lane:
            pending_free.push(l)                 # these edges converge into `lane` this row

        outgoing = []
        first, rest = commit.parents[0], commit.parents[1..]
        active[lane].waiting_for = first
        outgoing.push((lane, first))
        if first is None:
            pending_free.push(lane)              # root commit: lane ends here

        for parent in rest:                       # merge parents
            l = pop_lowest(free_pool) or allocate_new_column()
            active[l] = Lane { identity: branch_identity_for(commit, parent), waiting_for: parent, reserved: false }
            waiting[parent].push(l)
            outgoing.push((l, parent))

        marker = mistake_radar.classify(commit, lane, incoming, outgoing)  # see PROMPT.md §1.4

        rows.push(Row { commit, lane, incoming, outgoing, marker })
        cursor.rows_emitted += 1

    free_pool.extend(pending_free)                 # promote frees — not usable until the *next* chunk
    return rows
```

Streaming and virtualization build on this directly (§3.6): `layout_chunk` is
the unit of incremental work, and a `LayoutCursor` is the thing checkpointed
and resumed.

### 3.4 Merge commits and branch-name recovery

- The **second-parent curve** is drawn in the merged branch's `identity`
  colour (from the lane opened for that parent in step 3), not the primary
  lane's colour, so a merge visually reads as "this colour flowed into that
  one."
- **Labelling a deleted branch:** the merge commit's own message is the only
  surviving source once the branch ref is gone, so the label is recovered by
  matching, in order: `Merge branch '<name>'`, `Merge branch '<name>' into
  <target>`, `Merge pull request #<n> from <name>`, `Merge remote-tracking
  branch '<name>'`. If none match, the label falls back to the merged
  parent's short SHA and the row is not treated as a labelling failure —
  it's just less specific.
- **Evil-merge, accidental-pull-merge, backward-merge, foxtrot-merge and
  force-pushed-remote** detection all run as a classification step over the
  `Row` a commit already produced (comparing `incoming`/`outgoing` lanes,
  parent order, and — for evil merges — a lazy `--remerge-diff` computed only
  once a merge is selected, per PROMPT.md §1.4). They annotate `Row.marker`;
  they never change lane assignment. Duplicate-commit detection (patch-id
  matching) runs as a separate, bounded, off-thread pass over the visible
  window and attaches cross-references rather than a marker on layout itself.

### 3.5 Compaction (`+N`)

When `active.len()` (lanes open and not `reserved`) exceeds `graph.max_lanes`:

- Rank open, non-reserved lanes by how long it's been since a row last used
  them (least-recently-touched first).
- Fold enough of the least-recently-touched lanes into one `+N` marker column
  to bring the row's width back under the limit, but never fold: lane 0, any
  reserved integration lane, or a lane on the ancestry path of the currently
  selected commit (so selecting a commit never causes its own lane to
  disappear behind a `+N`).
- The `+N` column renders as a single glyph carrying a count; `Enter` (or a
  click) on it temporarily re-expands the folded lanes for the rows where
  they're relevant, without changing anyone else's column assignment.
- This is a rendering-time decision, not a layout-time one: the underlying
  `Row` data keeps every lane's real column so expanding `+N` is instant and
  never re-runs layout.

### 3.6 Incremental, streaming and virtualized layout

- **Streaming:** `layout_chunk` (chunk size ~500 commits) is called
  repeatedly, appending to a `rows: Vec<Row>` buffer that backs the view. The
  first chunk is sized to hit the 150 ms first-paint budget; later chunks run
  in the background between frames.
- **Rendering virtualization:** only rows in `[viewport_top − buffer,
  viewport_bottom + buffer]` are rasterized/drawn, independent of how much of
  `rows` has been computed. But since lane occupancy is stateful, `rows` up
  to `viewport_bottom` must already exist — scrolling far into history can't
  skip layout for what it skips over.
- **Checkpoints** make deep jumps (search hit at row 900,000, `G` to the very
  end) cheap without laying out everything first: every N rows (e.g. 2,000),
  snapshot the full `LayoutCursor`. Jumping to a distant row resumes from the
  nearest earlier checkpoint rather than row 0. Checkpoints are pure
  functions of the commit sequence up to that point, so they're cache-only
  (bounded, LRU-evicted) and always rebuildable — nothing about them needs to
  be durable. This is what keeps memory bounded by viewport-plus-checkpoints
  rather than by history size (PROMPT.md §1.2's "memory grows with the
  viewport, not with history size").
- **Folding a merged branch** (PROMPT.md §1.3) and **first-parent view** are
  both expressed as a different *inclusion predicate* over which commits the
  walker in step 2 emits at all, re-using the same `layout_chunk` machinery
  rather than being a separate code path — first-parent view is simply "only
  follow `parents[0]`," which trivially keeps everything in lane 0.

### 3.7 Invariants → tests

Each invariant in PROMPT.md §1.2 maps to a specific property the pseudocode
above is designed to satisfy, so the M1 property tests assert directly
against it:

| Invariant | Where it's enforced |
|---|---|
| Every parent edge is drawn | Every element of `commit.parents` produces exactly one `outgoing` entry |
| No edge passes through a node it isn't connected to | Only lanes in a row's `incoming`/`outgoing` bend toward that row; every other open lane draws a straight pass-through segment |
| Deterministic output | Heap tie-break by date then commit id; no unordered iteration in lane allocation |
| A lane never changes column mid-life | A lane's column is fixed at open time; only `waiting_for` mutates until the lane closes |

---

## 4. Terminal-capability detection

Lives in the `term` module (PROMPT.md §6). Runs once at startup, before the
first frame, within a fixed time budget so a terminal that answers nothing
never stalls launch.

### 4.1 Probe sequence

1. **Read cheap environment hints first** (never trusted alone, per
   PROMPT.md §1.1): `$TERM`, `$TERM_PROGRAM`, `$TERM_PROGRAM_VERSION`,
   `$COLORTERM`, `$KITTY_WINDOW_ID`, `$TMUX`, `$SSH_TTY`/`$SSH_CONNECTION`,
   `$NO_COLOR`.
2. **Enter raw mode and send active queries together**, then read whatever
   arrives before a deadline (200 ms locally; §4.4 raises this over SSH):
   - Kitty graphics protocol: an APC query action (`a=q`); look for the
     terminal's acknowledgement.
   - Sixel: `DA1` (`CSI c`); look for parameter `4` in the reply.
   - Cell size in pixels: `CSI 16 t`; parse `CSI 6 ; height ; width t`.
   - Kitty keyboard protocol: `CSI ? u`; a reply means support.
   - Synchronized output (DEC 2026): `CSI ? 2026 $ p` (DECRQM); read the
     reported mode, purely for diagnostics — begin/end sync sequences are
     emitted unconditionally either way since an unsupported terminal just
     ignores the unknown CSI harmlessly.
   - `OSC 11`: background colour, for the light/dark theme hint only (ADR-0003
     — never used as a paint colour).
3. **Fall back per-query, not globally**, on silence or a malformed reply:
   - Cell size: fall back to `TIOCGWINSZ` (character rows/cols plus pixel
     width/height, when the terminal fills those fields); if that's also
     zero, assume 8×16 px and mark the value "approximate" (visible in a
     diagnostics view, never silently treated as exact).
   - `OSC 11` silence: default to the dark theme, flagged "unknown" so a
     manual override (`ui.theme`) is easy to find.
   - Kitty keyboard / synchronized output silence: assume unsupported;
     both are enhancements, never required for correctness.
4. **iTerm2 is the one tier allowed to rely on an environment hint alone**
   (`$TERM_PROGRAM == "iTerm.app"` plus a minimum `$TERM_PROGRAM_VERSION`),
   because iTerm2's inline-image support has no query protocol — PROMPT.md
   §1.1 explicitly permits this one exception.

### 4.2 Tier decision

| Condition | Tier |
|---|---|
| Kitty graphics query acknowledged, iTerm2 hint matched, or Sixel indicated by `DA1` | **Pixel** (prefer Kitty > iTerm2 > Sixel when more than one answers) |
| Truecolor indicated (`COLORTERM=truecolor`/`24bit`, or a known-truecolor terminal) | **Unicode** |
| Neither of the above, or `TERM=dumb`, or output isn't a TTY | **ASCII** |
| `graph.renderer` set in config | Always wins over detection; forcing `pixel` on a terminal that didn't ack the query still attempts it, with a one-key escape hatch to drop a tier if the output looks garbled |

### 4.3 tmux

- Detected via `$TMUX`.
- Passthrough for the Kitty protocol needs tmux ≥ 3.3 with
  `allow-passthrough` set, and depends on the *outer* terminal too. Graft
  checks the tmux version and then re-sends the capability probe wrapped in
  tmux's passthrough sequence (`\ePtmux;...\e\\`), applying the same
  per-query fallback as §4.1.
- If the wrapped probe fails, times out, or the tmux version is too old,
  Graft falls back to the Unicode tier — deliberately, not by hoping
  passthrough happens to work. This is what PROMPT.md §7.1's manual-checklist
  pass condition ("a correct pixel graph *or* a clean fallback, never a
  broken screen") requires, and it's a decision made once at startup, not a
  per-frame guess.

### 4.4 SSH

- Detected via `$SSH_TTY`/`$SSH_CONNECTION`.
- No special-cased tier logic: a locally Kitty-capable terminal SSH'd into a
  remote host still relays escape sequences correctly in the common case, and
  the same probe-timeout fallback in §4.1 degrades gracefully when it
  doesn't. The one adjustment is a longer probe deadline (500 ms instead of
  200 ms) to absorb one extra network round trip before giving up on a query.

### 4.5 Re-detection triggers

- **Tier** is decided once per session; it doesn't change mid-session.
- **Cell size** (`CSI 16 t`) is re-queried on resize (`SIGWINCH`), since a
  font-size change or a window drag to a different-DPI display changes
  pixels-per-cell without changing the tier.
- **`OSC 11`** is re-queried on focus regain (piggybacking on the existing
  refresh-on-focus behaviour), so a terminal theme switched while Graft was
  in the background is picked up without a restart.

### 4.6 `NO_COLOR` and colour-depth fallback

- `NO_COLOR` (any non-empty value) disables ANSI colour in the chrome
  outright. For the graph specifically, lane identity falls back to the same
  shape/pattern redundancy already required for colourblind safety (distinct
  node shapes, and — for lines — distinct dash patterns per lane) rather than
  forcing a tier downgrade: `NO_COLOR` is about colour, not about protocol
  capability, so a pixel-tier terminal keeps drawing real graphics, just
  without colour.
- When truecolor isn't available but colour is (`NO_COLOR` unset), Graft
  checks terminfo/`$TERM` capabilities for 256- or 16-colour support and maps
  each lane's truecolor hue to the nearest palette entry using a perceptual
  distance (not naive per-channel rounding), so adjacent lanes stay
  distinguishable at any colour depth — the same colourblind-safe palette
  constraint from PROMPT.md §1.2 applies at every depth, not just truecolor.

---

## 5. Undo journal design

The undo journal is the mechanism behind `Ctrl+Z`/`u` and `Ctrl+Shift+Z`/`U`
(ADR-0002) and lives in the `ops` module (PROMPT.md §6), which is also the
only place allowed to run a write operation.

### 5.1 Storage and data model

- **Append-only log:** `.git/graft/undo-journal.jsonl`, one JSON record per
  line. Repo-local (travels with the repo's `.git`, not with Graft's global
  config), never committed to the repo's own history, human-readable enough
  to inspect by hand if something goes wrong.
- **Backup content:** real git objects under `refs/graft/backup/<timestamp>`,
  created before any operation that can destroy working-tree content. A
  backup ref keeps its objects reachable for ordinary `git gc`, so no
  extra pinning mechanism is needed while the ref exists.

```
JournalEntry {
    id: Ulid,                       // sortable, unique
    timestamp: DateTime,
    kind: OperationKind,            // Commit, Amend, Merge, Rebase, Reset,
                                     // BranchDelete, Discard, StashDrop, Push, …
    description: String,            // human text shown in the browser (§1.10)
    status: Pending | Committed | Aborted,
    refs_before: Map<RefName, Option<Oid>>,   // captured before the op runs
    refs_after: Map<RefName, Option<Oid>>,    // filled in once the op finishes
    index_before: Option<Oid>,       // pre-op index tree, for ops that don't touch the working tree
    backup_ref: Option<RefName>,     // refs/graft/backup/<timestamp>, when working-tree content was at risk
    reverses: Option<Ulid>,          // set on an undo/redo entry: which entry it reverses
}
```

### 5.2 Recording protocol

1. **Before** running the operation: read the current OIDs of every ref it
   might touch, and — for anything that can destroy working-tree content —
   take the combined snapshot (§5.3). Append a `Pending` record.
2. **Run** the operation through `ops` (the same typed-argument git
   invocation that populates the Command Log).
3. **On success:** capture the resulting OIDs and append a `Committed` record
   referencing the same `id`. The log is append-only — this is a second line,
   not an edit to the first — so a crash between steps 1 and 3 leaves a
   `Pending` record with no matching `Committed` one. On next launch, Graft
   treats an unmatched `Pending` entry the same way it treats a stray
   `rebase-merge/` directory (PROMPT.md §6): surfaced explicitly, offered for
   inspection, never silently dropped or silently assumed to have succeeded.
4. **On failure/abort:** append an `Aborted` record. Nothing needs undoing,
   but it stays in the log for the Command Log / troubleshooting view.

### 5.3 Working-tree snapshot (the untracked-file fix)

Per PROMPT.md §3, `git stash create` silently omits untracked files and must
not be used for this. Instead, before an operation that can destroy
working-tree content (discard, forced checkout, `reset --hard`, stash drop,
clean):

1. Start from the pre-op index tree.
2. For every untracked file in scope, `git hash-object -w` its contents into
   a blob and add it to that tree at its path (ignored files too, when the
   operation in question would also remove them).
3. Commit that combined tree under `refs/graft/backup/<timestamp>` — one
   object graph that represents tracked *and* untracked content as it stood
   immediately before the operation.

Undoing such an operation reads that tree back and restores working-tree
files from it (tracked content via the ordinary index/checkout path,
untracked content by writing the recovered blobs back to their recorded
paths). This is exactly what closes the gap PROMPT.md calls out, and it's
covered by the test PROMPT.md names explicitly: create an untracked file,
discard it, undo, assert the bytes come back exactly.

### 5.4 Per-kind inverse

Undo is not one generic "run the reverse command" — each kind has its own
specific inverse, computed from the journal entry:

| Kind | Inverse |
|---|---|
| Commit / Amend | Move ref(s) back to `refs_before`, restore the index from `index_before`. Working tree untouched (neither commit nor amend touches it) |
| Merge / Rebase / Cherry-pick / Revert | Move the branch ref back to `refs_before` using a compare-and-swap `update-ref` (fails loudly rather than clobbering if something else moved it since), restore index and working tree to match |
| Reset | Move ref back to `refs_before` (working tree was already covered by the pre-reset snapshot if it was `--hard`) |
| Branch delete | Recreate the ref at its recorded pre-delete OID |
| Discard | Restore files from the combined snapshot tree (§5.3) |
| Stash drop | Recreate the `refs/stash` reflog entry pointing at the (still-reachable, thanks to the backup ref) stash commit |
| Push | **Not silently undoable** — it changed shared state. Journaled for transparency and shown in the browser, but "undo" here opens a guided, freshly-confirmed re-push of the pre-push ref state rather than a one-key reversal, since it affects other people |

A mid-flight operation (a rebase currently stopped on a conflict) is not a
journal entry yet — its `Pending` record only resolves to `Committed` once
the operation finishes or is cleanly aborted through the operation banner
(§1.13). The banner's Abort is the mid-flight escape hatch; the undo journal
picks up once there's a finished operation to reverse.

### 5.5 Redo

Undo is itself journaled as an ordinary entry (`kind: Undo`, `reverses: Some(id)`),
computed and recorded the same way as any other operation, including its own
`refs_before`/`refs_after`. Redo is therefore not a separate data structure —
`Ctrl+Shift+Z`/`U` looks at the most recent entry; if it's an `Undo`, redoing
re-applies the entry it reversed from its recorded `refs_after`. This keeps
undo and redo symmetric: undo is always undoable too, the same way a text
editor's undo stack works.

Performing a new operation after an undo doesn't erase anything from the
log (it's append-only) — it just means the next `Ctrl+Shift+Z` has nothing to
redo, because the active cursor has moved past that branch of history. The
undo journal browser (§1.10) can still show the abandoned entries; they're
just not reachable through plain redo any more, matching how the browser is
described as fully browsable even for entries off the current undo/redo path.

### 5.6 Browsing and Ghost Preview

Selecting an entry in the browser (§1.10) and confirming reverts state to
just *before* that entry, by applying each entry's specific inverse from the
most recent one back to the target, in order. Before confirming, Graft draws
the predicted resulting ref state as a Ghost Preview (§1.7) — undo gets the
same "see it before you do it" treatment as every other history-changing
operation, per PROMPT.md §1.5.

### 5.7 Safety rails

Before applying an undo, Graft compares the ref/tree it expects to find
(`refs_after` / the state right after the operation being undone) against
what's actually there now. If they differ — the user ran a bare `git` command
outside Graft in the meantime — undo doesn't silently overwrite the
unexpected state. It stops, explains what changed, and offers to snapshot the
current state first (the same combined-tree mechanism as §5.3) before
proceeding, so an undo can never discard work Graft didn't itself put there.

### 5.8 Retention

Backup refs and journal entries are **kept indefinitely by default.** Git
objects are content-addressed and deduplicated, and a ref list entry is
cheap, so "never lose work" wins over tidiness by default. A "Clear backups
older than…" maintenance action is reachable from the command palette, always
explicit and always confirmed — never run automatically, and never as a side
effect of any other operation.

---

## 6. Open design questions

Parameters this document fixed with a reasonable default but that the M0/M1
prototype (PROGRESS.md step 3) or early milestones should validate against a
real terminal and real fixtures, adjusting here (and in an ADR, if the answer
changes something PROMPT.md fixed) rather than in code comments:

- **`graph.max_lanes` default of 8** before compaction kicks in (§3.5) — pick
  the real default from how `git/git`'s history actually looks at typical
  terminal widths, not a guess.
- **Checkpoint interval of 2,000 rows** (§3.6) trades memory against jump
  latency; tune it against the heavy tier's 100k-commit fixture once that
  exists.
- **Capability-probe budgets (200 ms local / 500 ms over SSH)** (§4.1, §4.4)
  are starting points; the manual terminal checklist (PROMPT.md §7.1) is what
  actually validates them across Ghostty/Kitty/WezTerm/iTerm2/tmux.
- **8×16 px fallback cell size** (§4.1) when no geometry query answers at
  all — a placeholder until the graph-drawing prototype shows whether that
  assumption produces a usable picture or needs to be per-platform.
- **`Ctrl+A` for "stage all"** (§2.4) is this document's choice to keep the
  bulk variant off a bare letter; worth confirming it doesn't collide with a
  terminal or OS-level binding on the user's actual setup before it ships.

None of these are stubbed features — they're numeric or ergonomic parameters
with a documented default, not gaps in scope, so they aren't duplicated in
`KNOWN_GAPS.md`.

---

## 7. Architecture

PROMPT.md §6.1 fixes the crate map and the dependency rule. This section fills
in the part §6.1 leaves open: which crate each *type* used elsewhere in this
document actually lives in, and why. It uses the `codebase-design` skill's
vocabulary — **module**, **interface**, **depth**, **seam**, **adapter** — so
each crate can be checked against the same question: is this a small interface
hiding a lot of implementation, or is it a pass-through? `ARCHITECTURE.md` at
the repo root is the short, standalone version of this section for whoever is
about to add or move a module; this section is where its reasoning lives.

### 7.1 Crate map

| Crate | Owns | May depend on |
|---|---|---|
| `core` | Shared domain types with no I/O: commit and ref identity, events, the record shapes other crates write and read, errors for core's own parsing | nothing internal |
| `graph` | The pure layout engine (§3) and the pixel/Unicode/ASCII renderers | `core` |
| `git` | Repository reads (`gix`), the typed git command runner, output and diff parsers | `core` |
| `ops` | Operations, the undo journal (§5), Mistake Radar detection, Ghost Preview planning | `git`, `core` |
| `term` | Terminal capability detection (§4), image-protocol encoding, escape-sequence sanitising | `core` |
| `ui` | Widgets, screen layout, theme, view models | `graph`, `term`, `core` |
| `app` | The binary: event loop, `AppState`, `update`, the job runner, config, wiring | everything |

As a layering picture rather than a table — each row may only reach into the
row(s) above it:

```
core
graph   git   term        (each: + core)
        ops                (+ git, core)
ui                          (+ graph, term, core)
app                          (+ everything above)
```

`graph`, `git` and `term` are siblings: none of them may depend on each other.
That's what forces any type all three need — see §7.2 — up into `core`.

### 7.2 Key types and where they live

Several types this document already defines (§3.1, §5.1) are written without a
crate label, because they read naturally as part of the algorithm they support
rather than as part of an architecture diagram. Placed against the dependency
rule in §7.1, most of them have exactly one legal home. Nothing below changes
the shapes already defined in §3.1 or §5.1 — it says which crate each one
compiles in and, where that isn't obvious, why.

**`core`** — plain data with no behaviour, used across crates that may not
depend on each other:

- `Oid` (raw git object id) and `CommitId` (an `Oid` known to name a commit) —
  needed by `git` (produces them), `graph` (lays them out), `ops` (journals
  and scans them) and `ui` (displays them).
- `Commit` (§3.1: id, parents, author, committer, date, message) — `git`
  parses it from `gix`/porcelain output, `graph` lays it out, `ops` scans it
  for Mistake Radar, `ui` renders it. Four crates, none of which may depend on
  each other, so this can only live in `core` even though §3.1 introduces it
  inside the graph-layout discussion.
- `RefName`, `WalkStrategy` (topological / date-order / first-parent-only,
  §3.2) — chosen by `app`/`ui`, executed by `git`'s rev-walk, shapes `graph`'s
  input order. Same reasoning: `git` and `graph` are siblings.
- `MistakeRadarKind` (CONTEXT.md's seven kinds) — `ops::MistakeRadar` produces
  it, `graph::Row.marker` (§3.1) carries it, `ui` renders the marker and its
  Explain text. Because `graph` may not depend on `ops`, **`graph::layout`
  takes a `Map<CommitId, MistakeRadarKind>` as a plain input parameter and
  never computes it** — `app` runs `ops::MistakeRadar::scan` and `graph::layout`
  separately and passes one into the other. `graph` stays ignorant of `ops`
  the same way it's ignorant of the terminal.
- `RenderingTier`, `Capabilities` (§4) — `term::detect` is the one impure
  function that produces a `Capabilities` (it's the only crate that queries
  the terminal); `graph`'s renderers and `ui`'s diagnostics view both consume
  the result as data, and both are `term`'s siblings or dependents, so the
  type itself has to sit below `term`, in `core`.
- `RasterFrame` — the pixel renderer's output: a premultiplied-alpha pixel
  buffer plus geometry, produced by `graph`'s pixel renderer (§1, ADR-0003)
  and consumed by `term`'s image encoder, which turns it into Kitty/iTerm2/
  Sixel bytes. `graph` and `term` are siblings, so `RasterFrame` lives in
  `core` for the same reason `Capabilities` does. This is the split between
  *drawing* the graph (pure, in `graph`) and *encoding it for this terminal*
  (protocol-specific, in `term`) that §1 and §4 don't spell out separately.
- `JournalEntry`, `OperationKind` (§5.1) — §5.1 says the undo journal "lives
  in the `ops` module," which is true for the journal's *behaviour* (append,
  replay, compute the inverse — all in `ops::UndoJournal`). But the undo
  journal browser (§1.10) is a `ui` view, and `ui` may not depend on `ops`, so
  the record *shape* itself — `JournalEntry` and `OperationKind` — has to be a
  `core` type: `ops` writes it, `app` reads it back to build the browser's
  view model, `ui` renders that view model. Same data-vs-behaviour split as
  `Commit`.
- `JobId`, `Event` — `Event` carries terminal input (key, mouse, resize,
  paste, focus, tick) plus job-completion notices (`JobFinished(JobId)`,
  `JobFailed(JobId, core::Error)`). It stays crate-agnostic on purpose: it
  never names a `git`, `ops` or `graph` type directly, which is what lets
  `core` own it without depending on anything. `app`'s job runner looks up
  what a finished `JobId` actually produced.
- Each crate keeps its own `Error` enum (§6.2 of PROMPT.md); `core::Error`
  only covers `core`'s own fallible constructors (parsing an `Oid` from a
  string, and so on) — it is not a shared error type other crates wrap.

**`graph`** (+ `core`): `Lane`, `Row`, `LayoutCursor`, `BranchIdentity` (§3.1)
— all layout-engine-internal, only ever touched by `graph` itself and by `ui`
(which already depends on `graph`). The public interface is small on purpose:
roughly `layout(commits, strategy, markers) -> impl Iterator<Item = Row>`, one
function per rendering tier taking a `Row` stream and producing either styled
text cells (Unicode/ASCII) or a `core::RasterFrame` (pixel). Everything about
lane assignment, compaction (§3.5) and incremental checkpoints (§3.6) is
internal — a deep module: one call in, a fully laid-out graph out.

**`git`** (+ `core`): `RepoHandle` (an open repository), `GitInvocation` /
`GitArg` (the typed command builder — Technology §"Writing to the repo": never
a joined shell string), `WorktreeStatus`, `DiffHunk`, `WordDiff` (porcelain
and diff parsing output). Internally, reads split into a small `RepoReader`
seam with a `gix` adapter and a `git2` fallback adapter (PROMPT.md
Technology) — two real adapters, so this is a real seam, not a hypothetical
one (codebase-design: "one adapter means a hypothetical seam, two means a
real one"). Writes are a second, separate path: build a `GitInvocation`,
spawn the user's `git`, parse its output — no shared code with the reader
seam beyond the argument and output types.

**`ops`** (+ `git`, `core`): `UndoJournal` (records and replays
`core::JournalEntry`, §5), `MistakeRadar::scan(commits) -> Map<CommitId,
MistakeRadarKind>`, `GhostPreview::plan(operation) -> PreviewPlan` (predicted
ref and commit deltas). `ops` never touches `graph`: to draw a Ghost Preview,
`app` takes the `PreviewPlan`, synthesises the predicted commits into the
walk, and runs the result through the same `graph::layout` used for real
history — dimmed/dashed styling is a `ui` rendering concern, not something
`ops` or `graph` need to know about.

**`term`** (+ `core`): `Capabilities::detect(probe_budget) -> Capabilities`
(§4.1–§4.4, the only impure function in this list — it's the one that talks
to the terminal), and an image encoder, `encode(&RasterFrame, Protocol) ->
Bytes`. Detection and encoding are two different seams for two different
reasons: detection varies by *terminal*, encoding varies by *protocol*
(Kitty/iTerm2/Sixel) — keeping them separate means adding a protocol doesn't
touch detection and vice versa.

**`ui`** (+ `graph`, `term`, `core`): plain view-model structs assembled by
`app` (for example a `GraphViewModel` wrapping a slice of `graph::Row`, or a
`JournalViewModel` wrapping `core::JournalEntry`), plus the ratatui widgets
that render them. `ui` never imports `git` or `ops` — every fact a screen
needs, including undo-journal entries and Mistake Radar explanations, arrives
already resolved into a `core` or `graph` type.

**`app`** (+ everything): `AppState` (the aggregate of every sub-state),
`update(state: &AppState, event: core::Event) -> (AppState, Vec<Command>)`,
and `Command` itself. `Command` is deliberately **not** a `core` type, even
though PROMPT.md §6.1 lists "events, commands" together under `core`'s
domain: a command such as "run this `git` fetch" or "record this journal
entry" has to carry a `git::GitInvocation` or a `core::JournalEntry`, and
`core` may not depend on `git`. Only `app` sees every crate, so `Command` —
and the job runner that executes each one as a cancellable `core::Job` and
reports a `core::Event::JobFinished` back to `update` — lives there. `Event`
stays in `core`; `Command` lives in `app`. That split is the one place this
section adds something PROMPT.md's phrasing doesn't quite say.

### 7.3 Data flow

```
                 ┌─────────────────────────────────────────┐
                 │                   app                    │
Terminal/job  ──▶│  Event ──▶ update(state) ──▶ (state, [Command])
input             │              │                    │      │
                 │              ▼                    ▼      │
                 │           view(state)         JobRunner   │
                 │              │                    │      │
                 └──────────────┼────────────────────┼──────┘
                                ▼                    ▼
                         ui::render(...)      spawns a core::Job per
                         (ratatui frame)      Command, off the UI
                                              thread, cancellable;
                                              result comes back as
                                              an Event
```

`update` never does I/O (PROMPT.md §6.1) — it only turns an `Event` into a new
`AppState` and a list of `Command`s to hand to the job runner. The UI thread
draws from `state` and otherwise only waits on the next `Event`; it never
waits on a `Job` directly, which is what keeps input responsive during a
two-minute fetch (PROMPT.md, non-negotiable principle 3).

### 7.4 Keeping crates deep

Depth is checked the same way for every crate in §7.1: could a caller do
everything they need through the handful of functions in §7.2, without
reaching past them into the crate's internals? `graph::layout` is the
clearest case — one function call replaces lane assignment, compaction and
checkpointing (§3), which is exactly the leverage a deep module is supposed
to give its one caller (`app`). Where a crate needs more than one concrete
implementation behind a function — `git`'s `gix`/`git2` reader split, or
`term`'s per-protocol encoders — that split is an **internal seam**, private
to the crate and invisible to callers, not a reason to expose more surface
area. `pub(crate)` is the default everywhere (PROMPT.md §6.1); a type only
becomes `pub` when another crate in §7.1's dependency graph actually needs it.

### 7.5 Dependency rule and enforcement

The rule from PROMPT.md §6.1: dependencies only point downward (§7.1's
picture), and it's a Cargo workspace so a cycle fails to compile. That
catches *cycles*. It does not, by itself, catch a new *acyclic* dependency
that still breaks the layering — `ui` adding `git` as a path dependency
compiles fine (no cycle) but violates "`ui` never imports `git` or `ops`."
§8 proposes closing that gap with an explicit test rather than relying on
review to catch it every time.

---

## 8. Proposed changes

Where this document would deviate from PROMPT.md, and why (CLAUDE.md,
"Challenge the spec"). One item — everything else in §7 fills in detail
PROMPT.md left open rather than disagreeing with it.

- **Add an automated dependency-edge check, not just cycle-freedom (§7.5).**
  PROMPT.md §6.1 says "the compiler rejects cycles, so the structure can't
  quietly erode." True, but incomplete: cycle-rejection stops `ui` from
  depending on something that depends back on `ui`; it does nothing about a
  direct, acyclic `ui → git` dependency, which is exactly the kind of erosion
  §6.1 is trying to prevent, and which the compiler will happily accept.
  Trade-off: a small test — parse `cargo metadata`'s dependency graph and
  assert it matches §7.1's edge set exactly — costs a few lines now and one
  edit whenever a crate or an allowed edge is added, in exchange for turning
  "a reviewer might notice" into "the fast tier fails." Propose adding it as
  an M0 ticket alongside the workspace scaffold, living in `app`'s test suite
  since `app` is the only crate that already depends on everything it needs
  to inspect.
