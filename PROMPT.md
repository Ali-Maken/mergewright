# Build Prompt: "Graft", a graphical terminal Git client

> **Note for humans:** this is the product spec. The working process lives in `PROGRESS.md` and `CLAUDE.md`: open this folder in Claude Code and run `/conduct`. "Graft" is a codename; the real name is chosen in M8.

---

## Role

You're a senior terminal-UI product designer and a senior Rust systems engineer who has shipped developer tools used by millions. You care about craft: text rows that line up exactly, no flicker, instant response, and never losing a user's work. You design for the task first and then make it look good.

## Mission

Build **Graft**, a terminal Git client that works with both keyboard and mouse. Its centrepiece is a **real graphical commit graph**: the kind of visual history Sublime Merge shows, running inside the terminal. It must:

- cover the everyday and advanced Git workflows that Sublime Merge covers
- be the easiest Git tool to learn
- be reliable enough to trust with production repositories

## Why this exists (every decision follows from this)

Terminal clients such as lazygit, tig and gitui draw history as a thin line of ASCII characters, or leave it out. That hides the most useful thing a Git GUI shows: the *shape* of history. In Sublime Merge you can see a bad merge straight away: main merged into a feature branch, an accidental `git pull` merge, a foxtrot merge that rewrites main's first-parent line, or a merge that slips in unrelated changes. Graft must make those mistakes obvious, easy to understand and easy to recover from.

**The graph is the product.** Everything else supports it.

This is inspiration only. Don't use Sublime Merge's name, logo, icons or copy. Give Graft its own identity.

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
- **Config:** TOML via `serde` at `~/.config/graft/config.toml`.
- **Testing:** `insta`, `proptest`, ratatui `TestBackend`, `tempfile`, `criterion`, `cargo-fuzz`.

---

## 1. The commit graph (the most important section)

### 1.1 Rendering tiers

Graft detects which tier to use automatically. The user can override it with `graph.renderer = "auto" | "pixel" | "unicode" | "ascii"`.

**Tier 1: Pixel.** For terminals with the Kitty graphics protocol (Kitty, Ghostty, WezTerm), iTerm2 inline images, or Sixel.

- **Drawing:** anti-aliased lines, smooth Bézier curves where branches fork and merge, and circular commit nodes with distinct ring styles.
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
- **Stable lanes.** The first-parent chain of each branch stays in one straight column for its whole life. The trunk sits in lane 0 and is always straight. Trunk means `main`, `master`, `trunk` or `develop` by default, configurable.
- **Lane reuse.** A lane freed when a branch ends can be reused, but never on the same row that frees it, so the picture stays unambiguous.
- **Compact output.** Keep line crossings and total graph width to a minimum. When too many branches run in parallel, compress them behind a `+N` indicator that expands on demand.
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

**Hover** shows a tooltip with the full message, parents and containing branches. Everything in a tooltip must also be reachable without hover.

**Folding**

- Collapse a merged branch's commits into its merge commit, and expand it again.
- A "first-parent view" gives a clean trunk-only history.
- Stashes can be shown in or hidden from the graph.

### 1.4 Mistake Radar (signature feature)

Detect the following problems and mark each with a small, calm marker in the graph. The detail pane explains what the marker means.

| Pattern | Detection | Default severity |
|---|---|---|
| **Backward merge** | Trunk merged *into* a feature branch | info (configurable, since some teams do this on purpose) |
| **Foxtrot merge** | A merge on trunk whose first parent is not the previous trunk tip, so trunk's first-parent history was rewritten (usually by pulling on trunk and then pushing) | warning |
| **Accidental pull merge** | `Merge branch 'x' of <remote>`: a branch merged with its own remote copy | info |
| **Evil merge** | A merge that introduces changes found in neither parent. Show them with `git show --remerge-diff` | warning |
| **Duplicate commits** | The same patch-id on several branches, left behind by cherry-picks or rebases. Link the copies together | info |
| **Force-pushed remote** | The remote-tracking tip is not a descendant of its previous value (check the remote ref's reflog) | warning |
| **Risky state** | Detached HEAD, unpushed commits on trunk, a branch with no upstream | info |

Each marker has two actions:

- **Explain:** a plain-English account of what happened, with the relevant part of the graph highlighted.
- **Fix…:** safe options, such as reverting the merge with `-m 1`, resetting the branch to before the merge (with a backup ref), or rebasing instead. Each option shows a **Ghost Preview** first (see 1.5).

### 1.5 Ghost Preview

Before a merge, rebase, reset, cherry-pick, revert or branch move, draw the predicted result in the graph with dashed or dimmed "ghost" nodes. The user confirms against that picture. This is the main way Graft makes history editing safe and easy to understand.

---

## 2. Screen layout and information architecture

```
╭─ ◆ graft ─ ~/code/api ─ ⎇ main ↑2 ↓0 ● 3 changes ──────────── ⌃P Commands ─╮
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

- click to select; double-click to open; right-click for a context menu
- the scroll wheel works on every scrollable area; drag dividers to resize
- **Drag and drop:** drag a branch badge onto a commit or another branch. A menu offers Merge into…, Rebase onto…, Reset to here and Cherry-pick, each with a Ghost Preview.

**Keyboard**

- Arrow keys and vim keys (`j/k/h/l`, `g/G`, `/` to search).
- `Tab`/`Shift-Tab` move between panes, `Enter` drills in and `Esc` backs out.
- Single-letter actions are mnemonic and listed in the hint bar.
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

- Undo reverses the last Graft operation: commit, amend, merge, rebase, reset, branch delete, discard, stash drop and so on.
- Before each operation, Graft records ref states in an operation journal.
- Before any operation that destroys working-tree content, it also saves a snapshot with `git stash create` under `refs/graft/backup/<timestamp>`.
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

## 4. Feature scope (match Sublime Merge)

**Repositories**

- Open from the current directory or with `graft [path]`; init; clone with progress; recent repos.
- Several repos open at once in tabs.
- A worktree switcher; submodules (view, update, open as a tab).
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

- Visual bisect: mark commits good or bad in the graph.
- LFS awareness (pointer files, locks); sparse-checkout awareness; co-author display.
- Open a commit or PR in the browser, with the URL worked out from the remote. This makes no network calls of its own.

## 5. Visual design system

- **Identity:** original, calm and high-signal. The interface chrome stays in the background; the graph and diffs stand out. Dense, but with room to breathe.
- **Colour tokens:** `bg`, `surface`, `surface-raised`, `border`, `text`, `text-muted`, `accent`, `added`, `removed`, `warning`, `info`, `lane-1…12`.
  - Ship dark, light and high-contrast themes.
  - Pick the theme automatically from the terminal background (query it with OSC 11), with a manual override.
  - Honour `NO_COLOR`.
- **Colour depth:** use truecolor, and fall back to 256 and then 16 colours using perceptual nearest-colour mapping (not naive rounding).
- **Hierarchy in monospace:** use weight, dim, italic (where supported), case, spacing and strict column alignment, not colour alone.
- **Borders:** thin and rounded. The focused pane gets an accent border and a stronger title; unfocused panes dim slightly.
- **Icons:** plain Unicode by default. Nerd Font icons are opt-in (`ui.icons = "nerd"`); a patched font is never required.
- **Diffs**
  - soft tints on added and removed lines, a stronger tint on changed words
  - a line-number gutter; hunk headers that stay pinned while scrolling
  - an optional view of whitespace and line endings
  - image previews in the pixel tier
- **Motion:** no decorative animation. Optional smooth scrolling. Zero flicker, through synchronized output and diff-based redraws.
- **Accessibility**
  - lane colours safe for colourblind users
  - every piece of information available without colour and without hover
  - full keyboard operation
- **Empty and first-run states:** helpful. If the folder isn't a repo, offer Init, Clone or Recent repos. A 30-second first-run tour can be skipped.

## 6. Engineering and architecture

- **One-way data flow:** `Event → update(state) → view(state)`. Git work runs as Jobs whose results come back as Events, and the UI never waits on git.
- **Modules** (a Cargo workspace is fine):

  | Module | Responsibility |
  |---|---|
  | `app` | Event loop and state |
  | `ui` | Components and widgets |
  | `graph` | Pure layout, plus the pixel, Unicode and ASCII renderers |
  | `git` | Reads via `gix`; runs the typed git CLI and parses its output |
  | `ops` | High-level operations and the undo journal |
  | `term` | Capability detection and image protocols |
  | `config` | Settings |
  | `theme` | Colours and styles |

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

1. **Graph layout:** unit tests and property tests (`proptest`) for every invariant in 1.2, plus snapshot tests of the layout output.
2. **Fixture repos from scripts:**
   - linear history, feature branches, octopus merges, criss-cross merges
   - foxtrot, backward, evil and accidental pull merges
   - orphan branches and cherry-pick duplicates
   - a synthetic history of 100k commits
   - Unicode and non-UTF-8 names
   - conflicting rebases, submodules, worktrees, LFS pointers

   Mistake Radar must flag exactly the fixtures it should, and nothing else.
3. **Screen snapshots** with `TestBackend` for every main view at 80×24, 120×40 and 200×60, in dark and light themes. Also keep PNG snapshots of the pixel graph.
4. **Every write operation:**
   - run it through `ops` on a temporary repo
   - check that the resulting refs, index and worktree match what the equivalent git CLI produces
   - check that **undo restores exactly the previous state**
5. **Fuzzing** of the diff, porcelain and blame parsers with `cargo-fuzz`.
6. **CI on GitHub Actions:**
   - checks: `cargo fmt --check`, `clippy -D warnings`, tests, benchmarks
   - platforms: macOS, Linux and Windows
   - git versions: the oldest supported and the latest
7. **Manual terminal checklist** in `TESTING.md`, to be run before each release:
   - Ghostty, Kitty, WezTerm, iTerm2
   - Terminal.app, Alacritty, the VS Code terminal, Windows Terminal
   - inside tmux, and over SSH

## 8. Delivery plan (milestones)

At the end of each milestone:

- every quality gate passes
- README and CHANGELOG are updated
- there are short demo steps
- `KNOWN_GAPS.md` is up to date

Don't start the next milestone while any test is failing.

| # | Milestone | Contents |
|---|---|---|
| **M0** | Foundations | Workspace, terminal setup and teardown, panic hook, capability detection, theme tokens, config, event loop, job runner, open repo |
| **M1** | **The graph** (read-only, best-in-class) | Build the fixtures first, then the layout engine and all three renderers, ref badges, selection, ancestry highlighting, folding, hover, search, virtualized scrolling, performance budgets. *Put most of the design effort here.* |
| **M2** | Details and diffs | Commit detail pane, file tree, unified and side-by-side diffs, word diffs, syntax highlighting, compare any two commits, Mistake Radar detection and Explain |
| **M3** | Working tree | Status, file/hunk/line staging, discard with backup, commit and amend, the commit editor, **undo journal** |
| **M4** | Branching and rewriting | Branches, tags, checkout, merge, rebase, cherry-pick, revert, reset, drag and drop, **Ghost Preview**, Mistake Radar Fix actions |
| **M5** | Conflicts | Three-pane resolver, operation banners, recovery on relaunch |
| **M6** | Remotes | Fetch, pull and push with progress, in-app authentication, clone, managing remotes |
| **M7** | Power tools | Interactive-rebase editor, stash, blame, file and line history, reflog, bisect, worktrees, submodules, branch comparison |
| **M8** | Polish and ship | Themes, keybinding help and remapping, first-run tour, docs, man page, `cargo install`, Homebrew tap, release binaries via `cargo-dist` |

## 9. Definition of done

- Opening the Linux kernel repo paints the first screen in 500 ms or less, and scrolling to the bottom stays smooth.
- In Ghostty, Kitty, WezTerm and iTerm2, the graph renders as anti-aliased curves lined up with the text rows, with no smearing while scrolling. In Terminal.app, the Unicode tier looks deliberately designed.
- The foxtrot-merge fixture is flagged in the graph, and Explain highlights the rewritten first-parent line.
- Every write operation can be undone. Killing Graft with `kill -9` mid-rebase leaves a state that git recognises, and on relaunch Graft offers Continue or Abort.
- A new user can do all of the following without reading docs, using only the hint bar and the command palette:
  - stage a single hunk
  - commit
  - create a branch
  - merge it with a Ghost Preview
  - push
- No stray characters remain in the terminal after quitting or crashing.
- Zero clippy warnings, and all tests pass on all three operating systems.

## 10. How to work with me

- **Before writing code,** produce `DESIGN.md` containing:
  - ASCII mockups of every main view
  - the full keymap table
  - the graph layout algorithm, in prose and pseudocode
  - the terminal-capability detection plan
  - the undo journal design

  **Then stop for my review.** Stop again after M1 so I can try the graph in my terminal.
- Record every significant choice as an ADR (see CLAUDE.md).
- Never quietly stub a feature and call it finished. Say it plainly and list it in `KNOWN_GAPS.md`. Ask me before cutting scope.
- Work in small, verified steps: run the tests and the app yourself, and check the snapshots after any visual change.
