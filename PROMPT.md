# Build Prompt: "Mergewright", a graphical terminal Git client

> **Note for humans:** this is the product spec. The working process lives in `PROGRESS.md` and `CLAUDE.md`: open this folder in Claude Code and run `/conduct`. The product is Mergewright (command mwr); "Graft" remains the project's codename (ADR-0057).

---

## Role

You're a senior terminal-UI product designer and a senior Rust systems engineer who has shipped developer tools used by millions. You care about craft: text rows that line up exactly, no flicker, instant response, and never losing a user's work. You design for the task first and then make it look good.

## Mission

Build **Mergewright**, a terminal Git client that works with both keyboard and mouse. Its centrepiece is a **real graphical commit graph**: the kind of visual history Sublime Merge shows, running inside the terminal. It must:

- cover the everyday and advanced Git workflows that Sublime Merge covers
- be the easiest Git tool to learn
- be reliable enough to trust with production repositories

**Platforms:** macOS and Linux. **Windows is an explicit non-goal for v1** and nothing in the plan pays for it: no Windows CI, no Windows Terminal in the manual checklist, no path, PTY or line-ending special cases carried "just in case". Keep the platform-specific code behind the module boundaries in section 6 so Windows stays *possible* later, but do not claim it, test it or block on it.

## Why this exists (every decision follows from this)

Terminal clients such as lazygit, tig and gitui draw history as a thin line of ASCII characters, or leave it out. That hides the most useful thing a Git GUI shows: the *shape* of history. In Sublime Merge you can see a bad merge straight away: main merged into a feature branch, an accidental `git pull` merge, a foxtrot merge that rewrites main's first-parent line, or a merge that slips in unrelated changes. Mergewright must make those mistakes obvious, easy to understand and easy to recover from.

**The graph is the product.** Everything else supports it.

This is inspiration only. Don't use Sublime Merge's name, logo, icons or copy. Give Mergewright its own identity.

Prior art to study (and beat): `serie`, a Rust commit graph drawn with terminal image protocols; `lazygit` and `gitui` for workflow coverage; `tig` for speed.

## Non-negotiable principles

1. **Real graphics, not ASCII art.** Draw the graph with actual pixels where the terminal supports it, and with carefully designed Unicode everywhere else.
2. **Never lose work.** Every destructive operation can be undone or leaves a recoverable backup.
3. **Never block the UI.** All Git work runs off the UI thread and can be cancelled. Input stays responsive even during a 2-minute fetch.
4. **Discoverable.** Every action is in the command palette, the keys that apply right now are always shown, and the mouse works everywhere.
5. **Transparent.** Every write operation shows the exact `git` command it runs and records it in a Command Log.
6. **Correct over clever.** Writes go through the user's own `git` binary, so hooks, config, credentials, signing and LFS all behave exactly as they do on the command line.
7. **Fast at scale.** It stays responsive on the Linux kernel repo (over 1M commits).
8. **Leave the terminal clean.** Restore the terminal state on quit, on error and on panic.

## Technology (decided; don't substitute without recording the reason in an ADR)

- **Language:** Rust (stable).
- **TUI:** `ratatui` + `crossterm`. Turn on:
  - mouse capture, including motion events for hover
  - bracketed paste and focus events
  - the kitty keyboard protocol, where the terminal supports it
  - synchronized output (DEC mode 2026), to prevent flicker
- **Reading the repo:** `gix` (gitoxide) for fast rev-walks, refs, objects and status. `git2` is an acceptable fallback for anything `gix` lacks.
- **Writing to the repo:** run the system `git` (minimum 2.38; detect the version and warn if it's older).
  - For parsing, use `--porcelain=v2 -z`, `-c core.quotepath=off` and `LC_ALL=C`.
  - Set `GIT_TERMINAL_PROMPT=0` and handle credentials with an in-app askpass.
  - Build commands from typed arguments, never by joining shell strings.
- **Diffs:** parse Git's own unified diff output (`--no-color --no-ext-diff`) so results follow the user's diff config. Compute word-level highlighting inside changed hunks with `imara-diff` or `similar`.
- **Syntax highlighting:** `syntect` or `tree-sitter-highlight`, run off-thread and cached.
- **Pixel graph:** rasterise with `tiny-skia`. Encode for the Kitty graphics protocol (use Unicode-placeholder mode so images scroll with the text and survive tmux), iTerm2 inline images and Sixel. Evaluate `ratatui-image`, and write your own encoder if it doesn't fit.
- **Auto-refresh:** `notify` file watching (debounced), plus a refresh when the terminal regains focus.
- **Concurrency:** a worker pool for Git jobs with cancellation tokens. The UI thread only handles input and draws.
- **Config:** TOML via `serde` at `~/.config/mergewright/config.toml`.
- **Testing:** `insta`, `proptest`, ratatui `TestBackend`, `tempfile`, `criterion`, `cargo-fuzz`.

---

## 1. The commit graph (the most important section)

### 1.1 Rendering tiers

Mergewright detects which tier to use automatically. The user can override it with `graph.renderer = "auto" | "pixel" | "unicode" | "ascii"`.

**Tier 1: Pixel.** For terminals with the Kitty graphics protocol (Kitty, Ghostty, WezTerm), iTerm2 inline images, or Sixel.

- **Drawing:** anti-aliased lines, smooth Bézier curves where branches fork and merge, and circular commit nodes with distinct ring styles.
- **Transparency:** render strips with a real alpha channel and never paint the background into them. Many users run a translucent or blurred terminal background, and `OSC 11` reports an opaque colour that ignores alpha, so a baked-in background would punch opaque rectangles through their translucency. Let the terminal composite.
- **Cell size:** draw at the terminal's real cell size in pixels. Query it with `CSI 16 t` or `TIOCGWINSZ`, and fall back sensibly if neither answers.
- **Alignment:** each image row must line up exactly with its text row, with no tearing or lag while scrolling.
- **Caching:** render in strips the size of the viewport. Cache them by row range, layout hash, theme and cell size, and redraw only when one of those changes.
- **Detecting support:** query the terminal directly: a Kitty graphics query, DA1 for Sixel, and `TERM_PROGRAM`/env hints for iTerm2. Never rely on `$TERM` alone.
- **tmux and SSH:** handle tmux passthrough, and fall back gracefully over SSH.

**Tier 2: Unicode.** For Terminal.app, Alacritty, the VS Code terminal and similar.

- Use rounded box-drawing characters (`╭ ╮ ╰ ╯ │ ─`) and node glyphs (`● ○ ◉ ◆`), with a truecolor colour per lane.
- It must look like a designed diagram, not like `git log --graph`. It's a first-class experience, not a second-rate fallback.

**Tier 3: ASCII.** For dumb terminals and limited fonts.

### 1.2 Layout algorithm

The layout engine is a **pure library crate** with no terminal code, so it can be tested and benchmarked in isolation.

- **One commit per row.** Topological order by default, with date order and first-parent-only as options.
- **Stable lanes.** The first-parent chain of each branch stays in one straight column for its whole life. The **default branch** sits in lane 0 and is always straight.
  - The default branch is a repo's main line of history: the branch everything else grows out of and merges back into. Detect it by this **ordered** precedence, resolved once per repo and cached: (1) `origin/HEAD` if it resolves, (2) `init.defaultBranch` from git config, (3) the first existing of `main`, `master`, (4) otherwise the branch with the most first-parent merge targets. Always overridable per repo in config.
  - An **integration branch** is a long-lived branch that collects work but is not the default branch, such as `develop` or a `release/*` line. It gets a stable lane adjacent to lane 0 and never competes for lane 0.
- **Lane reuse.** A lane freed when a branch ends can be reused, but never on the same row that frees it, so the picture stays unambiguous.
- **Compact output.** Keep line crossings and total graph width to a minimum. When too many branches run in parallel, fold the lowest-priority lanes behind a rail column; the header names the exact count (`+N lanes`) and `+` expands every fold on demand (ADR-0022, ADR-0025).
- **Invariants,** enforced with property tests:
  - every parent edge is drawn
  - no edge passes through a node it isn't connected to
  - the output is deterministic
  - a lane never changes column in the middle of its life
- **Colour.** Each lane's colour comes from the branch identity (a stable hash of the branch name), not from its column. That way a branch keeps its colour across scrolls and refreshes.
  - Use a colourblind-safe palette of 10–12 hues, tested for contrast between neighbours.
  - Special commits also differ in *shape*, never in colour alone.
- **Incremental and virtualized.**
  - Stream commits from the rev-walk and lay them out in chunks, so layout can resume where it stopped.
  - Paint the first screen within 150 ms.
  - Scroll at 60 fps on 1M+ commits.
  - Memory for rendering grows with the viewport, not with history size.

### 1.3 What each row shows

**Columns:** graph · ref badges · subject · author (initials chip, coloured per author, respecting `.mailmap`) · relative date · short SHA. Lower-priority columns hide as the terminal gets narrower.

**Ref badges** cover local branches, remote branches, tags, `HEAD` and stashes.

- When a local branch and its remote are in sync, show one merged badge (`main ⇄ origin`).
- Otherwise, show ahead and behind counts (`↑2 ↓1`).

**Uncommitted changes** appear as a virtual row at the top, attached to HEAD, with counts of staged, unstaged and untracked files.

**Merge commits**

- Use a distinct node shape.
- Draw the second-parent curve in the merged branch's colour.
- Label the curve with the merged branch's name even if that branch has since been deleted. Work the name out from messages such as `Merge branch 'x'`, `Merge pull request #n from y` and `Merge remote-tracking branch`.

**Selection**

- Selecting a commit highlights its ancestry path and dims unrelated lanes.
- The detail pane shows which branches and tags contain it ("Contained in: main, release/2.1").

**Hover** (no tooltip, ADR-0023) reveals a row's full subject in the footer when the graph column has truncated it; `Enter` on the cursor row gives the same reveal without a mouse. The full message, parents and containing branches live in the detail pane (M2), and `y` copies the selected commit's SHA.

**Folding**

- Collapse a merged branch's commits into its merge commit, and expand it again.
- Merged branches open **folded** by default (`▸ N commits from <branch>`); `Space` opens and closes one (ADR-0063, user approved 2026-10-17).
- A "first-parent view" gives a clean default-branch-only history.
- Stashes can be shown in or hidden from the graph.

### 1.4 Mistake Radar (signature feature)

Detect the following problems and mark each with a small, calm marker in the graph. The detail pane explains what the marker means.

| Pattern | Detection | Default severity |
|---|---|---|
| **Backward merge** | The default branch merged *into* a feature branch | info (configurable, since some teams do this on purpose) |
| **Foxtrot merge** | A merge on the default branch whose first parent is not the previous tip of that branch, so its first-parent history was rewritten (usually by pulling on the default branch and then pushing) | warning |
| **Accidental pull merge** | `Merge branch 'x' of <remote>`: a branch merged with its own remote copy | info |
| **Evil merge** | A merge that introduces changes found in neither parent. Show them with `git show --remerge-diff`. **Computed lazily, only for the selected merge** — re-merging every merge in a repo costs minutes of CPU. Until computed, the marker reads "unchecked" | warning |
| **Duplicate commits** | The same patch-id on several branches, left behind by cherry-picks or rebases. Link the copies together. **Bounded:** computed off-thread, only across visible refs and within a capped commit window — patch-id over full history diffs every commit | info |
| **Force-pushed remote** | The remote-tracking tip is not a descendant of its previous value (check the remote ref's reflog). The reflog is empty in a fresh clone, so when there is no history to check, say **"not checkable in this repo"** — never report absence as if it were verified | warning |
| **Risky state** | Detached HEAD, unpushed commits on the default branch, a branch with no upstream | info |

Each marker has two actions:

- **Explain:** a plain-English account of what happened, with the relevant part of the graph highlighted.
- **Fix…:** safe options, such as reverting the merge with `-m 1`, resetting the branch to before the merge (with a backup ref), or rebasing instead. Each option shows a **Ghost Preview** first (see 1.5).

### 1.5 Ghost Preview

Before a merge, rebase, reset, cherry-pick, revert or branch move, draw the predicted result in the graph with dashed or dimmed "ghost" nodes. The user confirms against that picture. This is the main way Mergewright makes history editing safe and easy to understand.

---

## 2. Screen layout and information architecture

```
╭─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ──────────── ⌃P Commands ─╮
│ BRANCHES      ⌕ │  GRAPH       SUBJECT                        AUTHOR   WHEN   │
│ ● main     ↑2   │  ◌           Uncommitted changes (3)                        │
│ ▾ feature/      │  ●           Fix token refresh race  [main] RM      2m      │
│     auth        │  │ ●         Add OAuth provider [feature/auth] AK   1h      │
│     billing     │  ◉─╯         Merge 'feature/login'          RM      3h      │
│ REMOTES         │  ●           Bump deps                      RM      1d      │
│ ▸ origin        ├─────────────────────────────────────────────────────────────┤
│ TAGS            │ COMMIT a1b2c3d · Rui M · 2 min ago · parents 9f8e7d6       │
│ STASHES (2)     │ Fix token refresh race                                       │
│ WORKTREES       │ ▾ src/auth/token.rs  +12 −4                                  │
│ SUBMODULES      │   @@ -40,7 +40,15 @@ fn refresh()                            │
╰─ ↑↓ move  ⏎ open  space stage  c commit  m merge  r rebase  ? help ─────────╯
```

**Panes**

- **Top bar:** repo, path, current branch, ahead/behind counts, dirty indicator and background-task spinner. While a merge, rebase, cherry-pick or bisect is in progress, it shows an **operation banner** (for example `REBASING 3/7`) with Continue / Skip / Abort buttons.
- **Left sidebar** (collapsible): branches shown as a tree split on `/`, plus Remotes, Tags, Stashes, Worktrees and Submodules, with filter-as-you-type.
- **Centre:** the graph, focused by default.
- **Detail pane:** changes with context. It can show:
  - a commit (metadata, message, file tree, diff)
  - working changes (unstaged and staged lists, plus a diff with hunk/line staging)
  - the conflict resolver
  - blame
  - file history

  It sits to the right on wide terminals and below the graph on medium ones.
- **Hint bar:** the keys that work in the current context, the last command's result, and notifications.

**Responsive breakpoints**

| Width | Layout |
|---|---|
| 160+ columns | Three columns |
| 100–159 | Sidebar shrinks to icons; detail pane moves below the graph |
| Under 100 | A single pane with tabs |

It must stay usable at 80×24. Split sizes can be changed with the mouse or keyboard and are remembered per repo.

## 3. Interaction design (ease of use comes first)

**Mouse**

Keyboard-first, mouse fully supported: every action is reachable from the keyboard and no feature is mouse-only, but every mouse affordance below is first-class.

- **Never send `XTSHIFTESCAPE` with `n=1`.** Terminals reserve `Shift`+drag to bypass mouse capture and do native text selection; requesting that `Shift` be delivered to the application instead would take the user's ability to select and copy text out of Mergewright.
- click to select; double-click to open; right-click for a context menu
- the scroll wheel works on every scrollable area; drag dividers to resize
- **Drag and drop:** drag a branch badge onto a commit or another branch. A menu offers Merge into…, Rebase onto…, Reset to here and Cherry-pick, each with a Ghost Preview.

**Keyboard**

- **Mnemonic-first.** Single-letter actions are mnemonic (`c` commit, `m` merge, `s` stage) and always listed in the hint bar. This is the organising principle: the hint bar teaches the tool, which is what "the easiest Git tool to learn" requires.
- Arrows navigate. Vim navigation keys (`j/k/h/l`, `g/G`, `/` to search) are **additional** bindings for movement only, never the primary grammar. Where a vim key and a mnemonic collide, the mnemonic wins.
- `Tab`/`Shift-Tab` move between panes, `Enter` drills in and `Esc` backs out.
- `?` opens context help.
- Every binding can be remapped.

**Command palette:** `Ctrl+P` and `:` open fuzzy search over every action, branch, tag, file and commit.

**Ease-of-use rules**

- Every action is at most two steps away through the palette.
- Verbs mean the same thing on every screen, and no hidden mode changes what keys do.
- **Destructive actions** open a confirmation that:
  - explains the consequence in plain language
  - shows the exact git command
  - has the safe option selected by default

**Undo and redo** (`Ctrl+Z` / `u`, redo with `Ctrl+Shift+Z` / `U`)

This is Mergewright's headline safety feature, not plumbing. The tools it competes with cannot undo a discard, cannot undo a pull, and leave users unable to tell what is undoable at all. Mergewright's answer is an explicit, browsable journal — treat it as a differentiator and design it accordingly.

`Ctrl+Z` is `SIGTSTP` (suspend) in a terminal. Mergewright captures it for undo by default, because undo is where every user reaches for that key. Restore suspend with `keys.suspend_on_ctrl_z = true`.

- Undo reverses the last Mergewright operation: commit, amend, merge, rebase, reset, branch delete, discard, stash drop and so on.
- Before each operation, Mergewright records ref states in an operation journal.
- Before any operation that destroys working-tree content, it also saves a snapshot under `refs/mergewright/backup/<timestamp>`.
  - **`git stash create` is not sufficient on its own: it silently omits untracked files.** It has no `-u` flag, and passing one is accepted as the stash *message*, so it exits 0 and returns a valid commit containing none of the untracked work. Discarding a new file this way is unrecoverable.
  - Back untracked content up explicitly (write blobs with `git hash-object -w` and record them in the backup tree) so the snapshot covers tracked *and* untracked files.
  - Cover this with a test: create an untracked file, discard it, undo, and assert the contents come back byte-for-byte.
- The journal is browsable, and each entry can be reverted.

**Long operations** (fetch, push, clone, rebase)

- Show progress parsed from git's `--progress` output.
- Can be cancelled.
- Never freeze input.

**Feedback**

- Successes appear as short-lived toasts.
- Errors stay until dismissed. They show git's stderr plus a plain-language explanation of common failures: non-fast-forward push, authentication failure, a dirty tree blocking checkout, a rejected hook.

**Commit message editor**

- Built into the app: a subject field, a body field, and guides at 50 and 72 characters.
- Helpers for sign-off and co-authors; commit templates are respected; message history.
- An option to open `$EDITOR` instead.

## 4. Feature scope (equal or better than Sublime Merge)

Sublime Merge is the **floor**, not the target. Match its core workflows and beat it on safety and discoverability — the undo journal, Ghost Preview, Mistake Radar and the always-visible hint bar are where Mergewright is meant to be better. Do not chase parity on things Sublime Merge itself does not do well; see *Deferred* at the end of this section.

**Repositories**

- Open from the current directory or with `mwr [path]`; init; clone with progress; recent repos.
- Submodules appear in the sidebar and can be opened. *(Deferred: multiple repos in tabs; the worktree switcher; submodule update/sync.)*
- Bare repos open read-only; shallow clones are handled.

**Working changes**

- Status grouped into staged, unstaged, untracked and conflicted.
- Stage, unstage or discard a whole file, a hunk or selected lines, including inside new untracked files.
- Stage all; add to `.gitignore`; rename detection.
- Open in `$EDITOR` at the right line.
- Discards are backed up and undoable.

**Committing**

- Commit and amend (message and/or content); sign-off.
- GPG and SSH signing according to the user's git config.
- Hook output is shown.
- A clearly labelled `--no-verify` toggle; empty commits.

**History**

- The graph, with search by message, author, committer, SHA, path, content (pickaxe `-S`/`-G`) or date range.
- Filter to one or more branches.
- File history that follows renames; line-range history (`git log -L`).
- Blame with a per-line age heatmap and jump-to-commit.
- A reflog view; a signature-verification badge.

**Comparing**

- Diff any two commits: select two rows with `Shift` or `Ctrl`+click.
- Branch comparison: commits in A that aren't in B, and the reverse.
- Diff against the working tree.
- Toggles for side-by-side or unified view, ignore-whitespace, word diff and context size.
- Rename and copy detection.
- Image diffs in the pixel tier; binary files handled.

**Branches and tags**

- Create a branch from any commit; rename.
- Checkout, with a "stash and switch" option when the tree is dirty.
- Delete local or remote branches, with a warning about unmerged work.
- Set or unset upstream; track a remote branch; fast-forward without checking out.
- Create, delete and push lightweight and annotated tags.

**Merging and rewriting**

- Merge (fast-forward / no-ff / squash) with a message editor.
- Rebase onto another branch.
- **Visual interactive rebase:**
  - reorder by dragging or with `Alt+↑/↓`
  - pick, reword, edit, squash, fixup or drop
  - autosquash `fixup!` commits
- Cherry-pick a single commit or a range.
- Revert, including merges, with a choice of parent.
- Reset (soft, mixed or hard) to any commit.
- Create a fixup commit for the selected commit; reword or split older commits.

**Conflicts**

- List the conflicted files.
- A three-pane resolver (ours | result | theirs, with base on demand):
  - per-hunk Take ours / Take theirs / Take both
  - editable result
  - mark as resolved
- Launch an external mergetool; show which commits are being combined.

**Remotes**

- Fetch (all remotes or one, with prune).
- Pull: fast-forward-only by default, with merge or rebase configurable. Warn before a pull would create a merge commit.
- Push, including setting the upstream. **A force push always uses `--force-with-lease --force-if-includes` and needs explicit confirmation.** A plain `--force` never runs.
- Add, rename or remove remotes.
- Credential prompts appear in the app with masked input; the SSH agent works.

**Stash**

- Stash everything, staged changes only, including untracked files, or selected files, with a message.
- List, view the diff, apply, pop, drop (undoable), and create a branch from a stash.

**Also**

- Co-author display.
- Open a commit or PR in the browser, with the URL worked out from the remote. This makes no network calls of its own.

**Deferred (stretch goals, not v1 promises)**

These are recorded in `KNOWN_GAPS.md` from day one rather than carried as promises. Each is either something Sublime Merge does not do either, or something the primary user does not use. Do not build them without asking; do not let a milestone claim "done" while implying they exist.

- Visual bisect
- Worktree switcher (opening a worktree directly still works: it looks like a normal repo)
- LFS awareness beyond what the user's own git filters already do
- Sparse-checkout awareness
- Several repos open at once in tabs

## 5. Visual design system

- **Identity:** original, calm and high-signal. The interface chrome stays in the background; the graph and diffs stand out. Dense, but with room to breathe. No decorative labels, subtitles or headings in the UI: every label must help the user act.
- **Colour tokens:** `bg`, `surface`, `surface-raised`, `selection`, `border`, `text`, `text-muted`, `accent`, `added`, `removed`, `warning`, `info`, `ours`, `theirs`, `open`, `ours-tint`, `theirs-tint`, `open-tint`, `lane-1…12`. (`selection` added in m5-history-and-ghost-preview/44: `surface-raised` was doing both the raised-panel job and the "you are here" job, and the second one needs far more contrast than the first.) (The six conflict tokens were added in M6, ADR-0044: `ours` and `theirs` are the conflict resolver's left and right sides by position — git's stages 2 and 3 — and are never red and green; `open` marks a block still to decide; each `-tint` is a background mixed 30 % of its hue into `bg`. Whether a block is open or taken is feedback, so the roles ship now; M8 may retune the values.)
  - Ship dark, light and high-contrast themes.
  - Pick the theme automatically from the terminal background (query it with OSC 11), with a manual override. Note `OSC 11` reports an opaque colour even on a translucent background, so treat the result as a light/dark hint, not as a colour to paint with.
  - **The lane palette is independent of the UI theme.** Themes restyle chrome; lanes keep their own validated, colourblind-safe hues in every theme. A theme must never be able to make two adjacent lanes indistinguishable.
  - Honour `NO_COLOR`.
- **Colour depth:** use truecolor, and fall back to 256 and then 16 colours using perceptual nearest-colour mapping (not naive rounding). **Exception, the six conflict tokens:** each carries a fixed 256-colour entry, because the nearest entry to a dark tint lies on the grey ramp and loses its hue. At 16 colours, in the high-contrast theme and under `NO_COLOR` the three tints are dropped (ANSI backgrounds are full-strength blocks); the glyphs and the hues of titles and separators carry the state.
- **Hierarchy in monospace:** use weight, dim, italic (where supported), case, spacing and strict column alignment, not colour alone.
- **Borders:** thin and rounded. The focused pane gets an accent border and a stronger title; unfocused panes dim slightly.
- **Icons:** plain Unicode by default. Nerd Font icons are opt-in (`ui.icons = "nerd"`); a patched font is never required. The **shipped default must be designed and snapshot-tested on an unpatched font**, not built with Nerd Font glyphs and then downgraded. Nerd Font detection is unreliable (there is no query for it), so never auto-enable it: the plain tier is the one every user sees first, and it has to look deliberate on its own.
- **Diffs**
  - soft tints on added and removed lines, a stronger tint on changed words
  - a line-number gutter; hunk headers that stay pinned while scrolling
  - an optional view of whitespace and line endings
  - image previews in the pixel tier (after v1, ADR-0057)
- **Motion:** no decorative animation. Optional smooth scrolling. Zero flicker, through synchronized output and diff-based redraws.
- **Accessibility**
  - lane colours safe for colourblind users
  - every piece of information available without colour and without hover
  - full keyboard operation
- **Empty and first-run states:** helpful. If the folder isn't a repo, offer Init, Clone or Recent repos. A practice tour (a throwaway repo, opt-in from the palette or t on the empty state) can be left with Esc at any step (ADR-0056).

## 6. Engineering and architecture

### 6.1 Architecture: a pure core with a thin shell

- **The shape:** the parts that *decide* are pure functions: data in, data out, with no I/O, no clock and no terminal. Only a thin outer layer (the "shell") touches the terminal, the filesystem, git and threads. The pure parts are:
  - graph layout
  - diff and porcelain parsing
  - Mistake Radar
  - undo planning
  - `update(state, event)`

  So most of the code can be tested without a terminal or a real repo.
- **One-way data flow:** `Event → update(state) → (new state, commands) → view(state)`.
  - `update` never does I/O. Instead it returns *commands*, such as "run git fetch".
  - The shell runs each command as a Job, and the result comes back as an Event.
  - The UI never waits on git.
- **Crate boundaries enforce the architecture.** It's a Cargo workspace, and dependencies only point downward. The compiler rejects cycles, so the structure can't quietly erode.

  | Crate | Responsibility | May depend on |
  |---|---|---|
  | `core` | Shared domain types, in CONTEXT.md's terms: commit IDs, refs, events, commands, errors | nothing internal |
  | `graph` | The pure layout engine and the pixel, Unicode and ASCII renderers | `core` |
  | `git` | Repository reads (`gix`), the typed git command runner, output parsers | `core` |
  | `ops` | High-level operations, the undo journal, Mistake Radar, Ghost Preview planning | `git`, `core` |
  | `term` | Terminal capability detection, image protocols, escape-sequence sanitising | `core` |
  | `ui` | Widgets, screen layout, theme, view models | `graph`, `term`, `core` |
  | `app` | The binary: event loop, state, `update`, job runner, config, wiring | everything |

  `ui` never imports `git` or `ops`. It receives plain view models, which keeps screens easy to test and to change.
- **Deep modules:** each crate and module offers a small, well-named public API that hides a lot of work (use the `codebase-design` skill). Keep internals `pub(crate)`.
- **`ARCHITECTURE.md`** sits at the repo root. It's a one-page map in the style of rust-analyzer's: the crates, what each one owns, the dependency rules and where to start reading. It's written in step 2 alongside DESIGN.md, and updated by any ticket that changes the structure.

### 6.2 Readable code (hard rules)

Anyone who knows basic Rust should be able to follow any file without help.

- **Names:**
  - Use CONTEXT.md's terms exactly: a "lane" is always a lane.
  - Use full words. The only abbreviations allowed are universal ones (`id`, `oid`, `ui`).
  - Functions are verbs and types are nouns. Booleans read as questions (`is_merge`, `has_upstream`).
- **Small pieces:**
  - Functions stay under about 60 lines, files under about 400 lines, and nesting at most 3 levels deep. Split a function once it needs comments to separate its sections.
  - Enforce this with Clippy's `too_many_lines` and `cognitive_complexity` lints, thresholds set in `clippy.toml`.
- **Plain over clever:**
  - no macros where a function would do
  - no traits or generics without at least two real uses
  - owned data and simple lifetimes are preferred
  - early returns instead of deep nesting
- **Explicit errors:**
  - Each library crate has its own error enum (`thiserror`) whose messages a user can understand. `anyhow` is allowed only in `app`.
  - Never swallow an error.
- **Comments explain why, not what:**
  - Every public item has a doc comment (the `missing_docs` lint is on).
  - Every module opens with a short `//!` paragraph saying what it's for.
  - Tricky algorithms (lane layout, undo, Mistake Radar) get a prose explanation with a small worked example.
- **No `unsafe`:** put `#![forbid(unsafe_code)]` in every crate. Any exception needs an ADR.
- **Tests read as documentation:**
  - Test names describe behaviour, such as `merge_of_trunk_into_feature_is_flagged_as_backward_merge`.
  - Each fixture script says in a comment what history it builds.
- **Enforced, not hoped for:**
  - `rustfmt.toml`, plus workspace-level lints (`[workspace.lints]`) in the root `Cargo.toml`
  - Clippy's `pedantic` group on, with any noisy lint allowed in one place, with a reason
  - `cargo clippy -- -D warnings` is a gate
- **Reviewed:** unclear names, oversized functions, missing docs and broken dependency rules are Major review findings.

### 6.3 Runtime behaviour

- **Awareness of in-progress operations:** read git's state files (`MERGE_HEAD`, `rebase-merge/`, `CHERRY_PICK_HEAD`, `BISECT_LOG` and so on) and show the operation banner. Relaunching mid-operation offers Continue or Abort.
- **Refresh:** watch the worktree and `.git` (refs, HEAD, index), debounced. Never refresh in the middle of an operation.

**Robustness**

- No `unwrap()` or `expect()` outside tests (enforce this with clippy lints).
- A panic hook restores the terminal and writes a crash report.
- Handle all of the following:
  - file paths that aren't valid UTF-8
  - huge files (truncate the diff, with a "Load more" option)
  - binary files, symlinks and CRLF line endings
  - empty repos, shallow clones, detached HEAD
  - worktrees and submodules
  - repos with 50k+ refs

**Security**

- **Clean every string before drawing it:** commit messages, author names, file names, file contents and branch names. This stops terminal escape-sequence injection from a malicious repo.
- Run nothing from the repo except through git itself.

**Performance budgets**

- first paint: at most 150 ms on a typical repo and 500 ms on the Linux kernel
- input to frame: under 16 ms
- benchmark the layout engine with `criterion`, and fail CI if it regresses by more than 10%

## 7. Testing and quality gates ("bug-free" in practice)

### 7.1 Laptop-safe testing (hard rules)

Tests run on the developer's laptop while they're using it. **The suite must never make the machine swap, freeze or crash.**

| Tier | Command | When | Budget |
|---|---|---|---|
| **Fast** (default) | `cargo test` | After every change; the gate for every ticket | Under 3 minutes and about 2 GB of RAM in total |
| **Heavy** | `cargo test --features heavy-tests -- --test-threads=2` | Milestone reviews only | Under 10 minutes and about 3 GB |
| **Manual** | Steps in `TESTING.md` | Only when the user chooses (the M1 and M8 checkpoints) | — |

**What each tier contains**

- **Fast tier:**
  - small fixtures only (500 commits or fewer each)
  - `proptest` capped at 64 cases, set in code (`graft_fixtures::proptest_cases`). The environment cannot change it — see ADR-0017
  - no network, no fuzzing, no benchmarks
- **Heavy tier:**
  - the 100k-commit synthetic repo and the performance budgets
  - `proptest` at 1,000 cases
  - a **peak-memory test** that fails if loading and scrolling the 100k repo uses more than 300 MB (measured with `getrusage`)
  - put these tests behind the `heavy-tests` feature only — no `#[ignore]`, so the one command above runs the whole tier (ADR-0018)
- **Manual tier:** the Linux kernel repo. Never downloaded or run automatically. `TESTING.md` explains how to make a commits-only clone (`git clone --filter=tree:0`) if the user wants to try it.

**Limits that keep the machine usable**

- **Parallelism:** `.cargo/config.toml` caps builds at 4 jobs. Tests run on at most 4 threads (`RUST_TEST_THREADS=4`), and heavy tests on 2.
- **One integration-test binary:** put integration tests in `tests/it/`, with a single `main.rs` that includes the other files as modules. Every separate file in `tests/` becomes its own binary to compile and link, which is slow and uses a lot of memory.
- **Lighter debug info:** use `debug = "line-tables-only"` for dev and test builds. It cuts the memory needed for linking and the disk space used.
- **Fixtures:**
  - generate them once with a script using `git fast-import`, not thousands of `git commit` calls
  - cache them in `target/fixtures/` and share them read-only
  - tests that write copy a small fixture into a temp directory, which is deleted afterwards
- **Isolated git:** every test that runs git:
  - has a 30-second timeout
  - sets `GIT_TERMINAL_PROMPT=0`, `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`
  - never uses the network

  No test may ever hang waiting for input.
- **One heavy job at a time:** never run two cargo commands, two test suites or two helper agents at the same moment.
- **Fuzzing:** only at milestone reviews. Limit each target to 60 seconds with `-max_total_time=60 -rss_limit_mb=1024`. `cargo-fuzz` needs nightly Rust, so install that only when it's needed.
- **Benchmarks:** only at milestone reviews, with short runs (`--warm-up-time 1 --measurement-time 3`).
- **Disk:** the reviewer reports the size of `target/`. Run `cargo clean` when it passes 20 GB.
- **Slow tests are bugs.** A fast-tier test that takes over 10 seconds, or a fast suite over its budget, is a Major review finding.

### 7.2 What to test

1. **Graph layout:** unit tests and property tests (`proptest`) for every invariant in 1.2, plus snapshot tests of the layout output.
2. **Fixture repos from scripts:**
   - linear history, feature branches, octopus merges, criss-cross merges
   - foxtrot, backward, evil and accidental pull merges
   - orphan branches and cherry-pick duplicates
   - a synthetic history of 100k commits (heavy tier only)
   - Unicode and non-UTF-8 names
   - conflicting rebases, submodules, worktrees, LFS pointers

   Mistake Radar must flag exactly the fixtures it should, and nothing else.

   **The test corpus is named, scripted and reproducible.** It is not "the developer's own repos": those are not reproducible, not shareable and, in this project's case, contain no merges and no tags at all, which would make the graph and Mistake Radar checkpoints unfalsifiable. The corpus is:

   | Corpus | What it proves | Tier |
   |---|---|---|
   | Scripted synthetic fixtures (`git fast-import`) | **Correctness.** Every graph invariant and every Mistake Radar pattern, with a known-right answer to assert against | Fast |
   | `git/git`, cloned with `--filter=tree:0` | **Human proof.** Real, messy, merge-heavy history that a person can eyeball. Roughly 80k commits with thousands of merges and tags | Heavy / manual |
   | A small real repo with linear history and no merges | **The trivial case.** Proves the graph still looks deliberate when there is nothing to draw | Fast |

   The Linux kernel repo stays where 7.1 puts it: manual tier only, never downloaded automatically.
3. **Screen snapshots** with `TestBackend` for every main view at 80×24, 120×40 and 200×60, in dark and light themes.
   - For the pixel graph, snapshot the **rasterised RGBA buffer at a fixed synthetic cell size** (say 8×16), not a PNG of a real terminal. Real cell sizes vary by font, font size and display scaling, so a PNG taken on one machine fails everywhere else for reasons that have nothing to do with the code. Assert on the buffer's bytes (or a perceptual hash of them) with the cell size pinned by the test.
   - Cover terminal-reported cell geometry separately, as a unit test of the `CSI 16 t` / `TIOCGWINSZ` parsing and the fallbacks, including terminals that report nothing.
4. **Every write operation:**
   - run it through `ops` on a temporary repo
   - check that the resulting refs, index and worktree match what the equivalent git CLI produces
   - check that **undo restores exactly the previous state**
5. **Fuzzing** of the diff, porcelain and blame parsers with `cargo-fuzz`, time-limited as described in 7.1.
6. **CI on GitHub Actions** (once the project has a GitHub repo). Heavier runs belong here, not on the laptop.
   - checks: `cargo fmt --check`, `clippy -D warnings`, the fast and heavy test tiers, benchmarks
   - platforms: macOS and Linux
   - git versions: the oldest supported and the latest
7. **Manual terminal checklist** in `TESTING.md`, to be run before each release:
   - **Pixel tier must pass:** Ghostty, Kitty, WezTerm, iTerm2
   - **Unicode tier must look deliberate:** Terminal.app, Alacritty, the VS Code terminal
   - **Must degrade, not break:** inside tmux, and over SSH. tmux's graphics passthrough depends on the tmux version, `allow-passthrough` and the outer terminal, so the pass condition inside tmux is *either* a correct pixel graph *or* a clean fall back to the Unicode tier — never a broken screen or stray escape sequences. Detect tmux and pick the tier deliberately rather than hoping passthrough works.
   - Windows terminals are out of scope (see Mission).

## 8. Delivery plan (milestones)

At the end of each milestone:

- every quality gate passes
- README and CHANGELOG are updated
- there are short demo steps
- `KNOWN_GAPS.md` is up to date

Don't start the next milestone while any test is failing.

| # | Milestone | Contents |
|---|---|---|
| **M0** | Foundations | Workspace with the crate map from ARCHITECTURE.md, lint and format config, terminal setup and teardown, panic hook, capability detection, theme tokens, config, event loop, job runner, open repo |
| **M1** | **The graph** (read-only, best-in-class) | Build the fixtures first, then the layout engine and all three renderers, ref badges, selection, ancestry highlighting, folding, hover, search, virtualized scrolling, performance budgets. *Put most of the design effort here.* |
| **M2** | Details and diffs | Commit detail pane, file tree, unified and side-by-side diffs, word diffs, syntax highlighting, compare any two commits, Mistake Radar detection and Explain |
| **M3** | Working tree, undo and **stash** | Status, file/hunk/line staging, discard with backup, commit and amend, the commit editor, **undo journal**, stash save/apply/pop/drop |
| **M4** | **Remotes** | Fetch, pull and push with progress, in-app authentication, clone, managing remotes |
| **M5** | Branching and rewriting | Branches, tags, checkout, merge, rebase, cherry-pick, revert, reset, drag and drop, **Ghost Preview**, Mistake Radar Fix actions |
| **M6** | Conflicts | Three-pane resolver, operation banners, recovery on relaunch |
| **M7** | Power tools | Interactive-rebase editor, blame, file and line history, reflog, submodule display, branch comparison |
| **M8** | Polish, **final name** and ship | Themes, keybinding help and remapping, first-run tour, docs, man page, `cargo install`, Homebrew tap, release binaries via `cargo-dist` |

**Why this order.** The milestones are ordered by *how often the primary user does the thing*, not by how hard it is to build. Fetch, pull, push, merge, conflict resolution, stash and cherry-pick are all daily work, so none of them may sit behind six milestones of machinery. Stash moves next to staging because they are the same reflex (put this aside), and remotes move ahead of history rewriting because fetching and pushing are more frequent, and lower risk, than rebasing.

**M5 ships without a conflict resolver.** Merge and rebase can produce conflicts one milestone before M6 builds the three-pane resolver. M5 must therefore ship an honest interim: detect the conflicted state, show which files conflict, and hand off to the user's configured `git mergetool` or editor, with the operation banner and Abort always available. **Record this in `KNOWN_GAPS.md` with M6 named as its expiry**, and never let M5 claim conflict resolution is done.

## 9. Definition of done

- The heavy tier's 100k-commit repo meets the performance and memory budgets. In the manual check (7.1), opening the Linux kernel repo paints the first screen in 500 ms or less, and scrolling to the bottom stays smooth.
- On `git/git` (the named human-proof corpus in 7.2), the history shape is legible without zooming, panning or squinting: lanes stay distinguishable, the default branch is straight, and merges read as merges.
- In Ghostty, Kitty, WezTerm and iTerm2, the graph renders as anti-aliased curves lined up with the text rows, with no smearing while scrolling. In Terminal.app, the Unicode tier looks deliberately designed. Inside tmux, the chosen tier renders cleanly, whichever tier that is.
- On a translucent terminal background, the graph's own background stays translucent: no opaque rectangles behind the lanes.
- The foxtrot-merge fixture is flagged in the graph, and Explain highlights the rewritten first-parent line.
- Every write operation can be undone. Killing Mergewright with `kill -9` mid-rebase leaves a state that git recognises, and on relaunch Mergewright offers Continue or Abort.
- A new user can do all of the following without reading docs, using only the hint bar and the command palette:
  - stage a single hunk
  - commit
  - stash the rest and bring it back
  - create a branch
  - merge it with a Ghost Preview
  - resolve a conflict
  - push
- No stray characters remain in the terminal after quitting or crashing.
- Every deferred item in section 4 is listed in `KNOWN_GAPS.md`, and nothing in the README or the first-run tour implies it exists.
- Zero clippy warnings, and all tests pass on macOS and Linux.

## 10. How to work with me

- **Before writing code,** produce `DESIGN.md` containing:
  - ASCII mockups of every main view
  - the full keymap table
  - the graph layout algorithm, in prose and pseudocode
  - the terminal-capability detection plan
  - the undo journal design
  - the crate map, key types and dependency rules (section 6.1), also written up as `ARCHITECTURE.md`

  **Then stop for my review.** Stop again after M1 so I can try the graph in my terminal.
- Record every significant choice as an ADR (see CLAUDE.md).
- Challenge this spec. If you see a better approach, or think something here is wrong, propose it with reasons in a "Proposed changes" section. Don't silently comply or silently deviate. I decide at the next checkpoint.
- Never quietly stub a feature and call it finished. Say it plainly and list it in `KNOWN_GAPS.md`. Ask me before cutting scope.
- Work in small, verified steps: run the tests and the app yourself, and check the snapshots after any visual change.
