# Big-repo history scrolling — prior art

Question: how do the best-loved Git tools keep history scrolling smooth on big repos (~20k+ commits, 1k+ refs), and how do they stop held keys or scroll events from piling up?

Trigger: field note 2026-09 (commit 68214f28), scroll lag on a 17.7k-commit repo.

Rule: facts carry a link. **Guess** / **Question** marks anything not confirmed from a primary source. Researched 2026-10-01.

## Summary

- **No tool's public material describes input coalescing for held keys.** Nothing found from Sublime Merge, lazygit, gitui, GitUp, Fork, JetBrains or Git Graph. That half of the question is unanswered from primary sources; what's below on it comes from TUI frameworks and tig's loop.
- The fast GUIs read the repository themselves rather than spawning `git` per view: Sublime Merge (own reader), GitUp (libgit2 subset + own code).
- The terminal tools stream or page the work: tig reads the `git` pipe incrementally while staying responsive; gitui walks in 3000-commit batches on a background thread; lazygit loads 300 at a time.
- Complaints cluster on **startup / first load**, `--topo-order` cost, and unmaintained repos (no commit-graph), not on frame drawing.

## Per tool

### Sublime Merge (first, the user asked)
- **Reader:** "We have a custom implementation of Git for reading repositories, which drives a lot of our high performance functionality … we defer to Git itself for operations that mutate the repository." — [Sublime Merge launch post](https://www.sublimetext.com/blog/articles/sublime-merge)
- **UI:** built on the Sublime Text UI engine. The post credits that engine for fast, flicker-free, non-blocking UI. Same source.
- **Optimistic UI for slow writes:** "Sublime Merge predicts the outcome of staging and presents that immediately" (ben-schaaf, Sublime HQ, [HN 40862144](https://news.ycombinator.com/item?id=40862144)).
- **Lazy / virtualised list, commit-graph file use, per-frame work, key-repeat handling:** **not documented publicly.** **Question:** does it virtualise rows, and does it read `.git/objects/info/commit-graph`? Closed source; nothing found to confirm either.
- **Complaints:** slowness drawing the tree on a repo with thousands of committers and ~6 years of history. Reporter asks for a "recent commits only" mode. No maintainer reply. — [sublime_merge#1937](https://github.com/sublimehq/sublime_merge/issues/1937). Freeze on large commits (diff loading, not scroll) — [#138](https://github.com/sublimehq/sublime_merge/issues/138). Write ops slow in a 60k-file repo — [#1890](https://github.com/sublimehq/sublime_merge/issues/1890).
- **Praise:** "totally snappy … only time I've seen it slow down was opening Unreal Engine" (tehnub, [HN 40862144](https://news.ycombinator.com/item?id=40862144)).

### lazygit
- **Loading:** spawns `git log HEAD --topo-order … -300`, so pages of 300 — quoted in [lazygit#2397](https://github.com/jesseduffield/lazygit/issues/2397).
- **Cost:** `--topo-order` took ~15 s to fill the commits view on the Linux kernel. Removing it fixed that for the reporter. Same issue.
- **Complaints:** lag of 1–2 s per key in a repo with 300k commits; fixed by `git maintenance run` (gc, commit-graph) — [discussion #3965](https://github.com/jesseduffield/lazygit/discussions/3965). Slow startup with many annotated tags (a refs cost) — [#4770](https://github.com/jesseduffield/lazygit/issues/4770). Very slow on large repos — [#2875](https://github.com/jesseduffield/lazygit/issues/2875).
- **Input coalescing:** **Question.** Not confirmed from source.

### tig
- **Incremental read:** the main loop's `update_views()` reads from each view's `git` pipe. When data arrives it sets the input timeout to 0, then calls `doupdate()` and `wgetch()`. Rows show while `git` is still writing, and keys are handled between reads — [tig src/display.c](https://raw.githubusercontent.com/jonas/tig/master/src/display.c) (`get_input`, `update_views`).
- One `doupdate()` per loop turn. **Guess:** with ncurses, keys queued during a slow turn are then read one per turn, not batched. Not verified.

### gitui (Rust, ratatui-family)
- **Own reader:** libgit2 via `asyncgit`. `AsyncLog` walks revisions on a rayon thread in batches of `LIMIT_COUNT = 3000`. It sleeps 2 ms between batches in the foreground and 1 s in the background — [asyncgit/src/revlog.rs](https://raw.githubusercontent.com/gitui-org/gitui/master/asyncgit/src/revlog.rs).
- **Lesson:** a cache key held a millisecond timestamp, so the cache missed on every UI tick and status took 5+ min on a 1.7M-file repo — [gitui#2823](https://github.com/gitui-org/gitui/issues/2823). Per-tick work must hash stable inputs.
- **Complaints:** slow start-up — [#1239](https://github.com/gitui-org/gitui/issues/1239); slow file loading in the log — [#976](https://github.com/gitui-org/gitui/issues/976).

### GitUp
- "Interact[s] directly with the Git database on disk". Uses "a minimal subset of libgit2 and reimplements everything else". Can "load the entire repo history in memory for fast access, including all commits, branches and tags" — [GitUp README](https://github.com/git-up/GitUp).
- So: load once, all in memory, no paging. Per-frame cost and large-repo limits are not documented.

### Fork
- Release notes list "performance improvements for large repositories" (1.0.57, 1.0.9) without detail — [Fork release notes](https://git-fork.com/releasenotes).
- **Complaint:** freezes on almost every click and memory grows, on a repo with long history and 10k-file commits — [TrackerWin#122](https://github.com/fork-dev/TrackerWin/issues/122).
- Mechanism not public.

### JetBrains (IntelliJ)
- **"Git log indexing"** (a persistent index) is optional. It gives fast log filtering, precise history, and history search — [IntelliJ Log tab docs](https://www.jetbrains.com/help/idea/log-tab.html).
- Paging and graph layout internals: **Question**, not read. The source is open (intellij-community `vcs-log`); not inspected for this note.

### VS Code Git Graph / GitLens
- **Git Graph:** settings for "Initial Load" commit count and "Load More". "Load More Automatically" fetches more when scrolled to the bottom — [vscode-git-graph README](https://github.com/mhutchie/vscode-git-graph).
- **GitLens:** complaint that the commit graph is slow versus the CLI on large repos; no maintainer reply yet — [vscode-gitlens#4585](https://github.com/gitkraken/vscode-gitlens/issues/4585).

## Git itself: commit-graph
- The commit-graph file stores OID, parents, date, root tree and generation number. Walks use it instead of inflating commit objects. Generation numbers cut ancestry and topo-order work. On big repos the doc says these operations go from minutes to fast — [git commit-graph docs](https://git-scm.com/docs/commit-graph).
- lazygit #3965 above shows the user-visible effect of a stale commit-graph and stale pack state.

## Terminal techniques
- **Decouple render from input.** Ratatui's recipe runs separate `tick_rate` (4/s) and `frame_rate` (60/s) intervals. Input events update state; `Event::Render` draws at most once per frame interval. A burst of keys becomes one draw — [ratatui terminal-and-event-handler recipe](https://ratatui.rs/recipes/apps/terminal-and-event-handler/).
- **You own the draw cycle.** Ratatui does not throttle; the app decides when `terminal.draw()` runs — [ratatui FAQ](https://ratatui.rs/faq/).
- **Drain before draw.** Loop `crossterm::event::poll(Duration::ZERO)` + `read()` until empty, apply all events, then draw once. **Guess** from the poll/read API — [ratatui event handling (DeepWiki)](https://deepwiki.com/ratatui/ratatui/4.3-handling-events) is secondary. A search snippet claimed `poll(ZERO)` skips crossterm's parser buffer and that a 1 ms poll is needed. **Question:** unverified, check crossterm source before relying on it.
- **Stream, don't block:** tig's `update_views` → zero-timeout → `doupdate` loop (above).
- **Background walk in batches:** gitui `AsyncLog` (above).

## What fits a TUI (Graft)
1. Draw only visible rows. Keep layout/graph lanes computed once per load, not per frame. Supported by: gitui and lazygit keep the list in memory and page the walk. **Guess** that Graft's lag is per-frame recompute — **Question** for the scout; not read.
2. Drain all pending input, apply it to state (latest scroll position wins), then draw once. Optionally cap draws at ~60 fps (ratatui recipe).
3. Walk history on a background thread in batches (gitui: 3000), showing the first screen before the walk ends (tig).
4. Read commit-graph when present; avoid `--topo-order`-style full sorts on the UI path (lazygit#2397, git docs).
5. Never put a time value in a per-tick cache key (gitui#2823).
6. Refs: load once, not per frame. lazygit#4770 shows tag count alone can cost startup.

## Not confirmed
- Sublime Merge: virtualisation, commit-graph use, input coalescing — closed source, nothing public.
- Any GUI tool's held-key handling — no primary source found.
- JetBrains internals — source not read.
- crossterm `poll(ZERO)` buffer behaviour.
