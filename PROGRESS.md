# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** 2. Write DESIGN.md

---

## Phase A: Setup and decisions

- [x] **0. Setup** (You)
  - [x] Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`, then check with `rustc --version`
  - [x] Claude Code installed and signed in, and **Fable** available on your plan
  - [x] Ghostty or iTerm2 installed, for the pixel graph
  - [x] `git --version` shows 2.38 or later (`brew install git` if it's older)
  - [x] In your terminal: `claude plugins install mattpocock-skills`
  - [x] Open this folder in Claude Code and run `/setup-matt-pocock-skills`. When asked where issues live, choose **local markdown**. Then commit.
  - [x] Nothing else. The test corpus is scripted, not personal: M1's first ticket builds the synthetic fixtures and clones `git/git` with `--filter=tree:0` (PROMPT.md 7.2). You don't need to find repos.

- [x] **1. Decisions interview** (You, with Opus, in the main session). Run:
  > /grill-with-docs Read PROMPT.md. Interview me one question at a time about decisions only I can make: which workflows I use daily versus rarely, keybinding style, Mac-only or cross-platform, mouse versus keyboard priority, trunk branch names, personal or open-source, and anything in the spec you think is wrong or risky. Already decided: Rust, tickets in local files, the conductor stops at checkpoints, Fable as planned, and the real name is chosen in M8. Don't add features. Propose PROMPT.md edits for my approval. When we're finished and I've approved the edits, tick step 1 in PROGRESS.md and commit.

  From here on, just run `/conduct`.

  **Done 2026-09-11.** Nine decisions settled and applied to PROMPT.md: macOS + Linux only, mnemonic-first keymap, `Ctrl+Z` as undo, ordered default-branch detection, milestones reordered by daily use, a named scripted test corpus, and ten spec risks fixed. Recorded in `CONTEXT.md` and `docs/adr/0001`–`0005`.

- [ ] **2. Write DESIGN.md** (conductor → `builder-fable`). Task:
  > Read PROMPT.md, CONTEXT.md and the ADRs. Do the first step of section 10: write DESIGN.md with ASCII mockups of every main view, the full keymap, the graph layout algorithm in prose and pseudocode, the terminal-capability detection plan, the undo journal design, and the architecture from PROMPT.md 6.1 (crate map, key types, dependency rules). Also write ARCHITECTURE.md. Write them for a human who wants to understand them quickly. Where you see a better approach than the spec, propose it under "Proposed changes" with your reasons (CLAUDE.md, "Challenge the spec"). No code.

- [ ] **CHECKPOINT: review DESIGN.md** (You). Read it and ask for changes. For a second opinion, ask the conductor to run `reviewer` on it.

- [ ] **3. Graph-drawing prototype** (conductor → `builder-fable`). Task:
  > Build a throwaway prototype in prototypes/graph-spike/: a tiny Rust app that draws a hard-coded 30-commit graph with branches and merges using the Kitty graphics protocol and iTerm2 inline images, lined up with text rows and scrollable with j/k. The goal is to prove that alignment and scrolling work. Record the findings as an ADR.

- [ ] **CHECKPOINT: try the prototype and approve the M0 tickets** (You)
  - Run `cargo run --manifest-path prototypes/graph-spike/Cargo.toml` in Ghostty or iTerm2.
  - If it doesn't work, decide the next approach with the conductor before M0.

---

## Phase B: Milestones (all through `/conduct`)

**What the conductor does for every milestone:**

1. **Tickets:** splits the milestone into small tickets with `/to-tickets`. You approve them at the checkpoint before the milestone starts.
2. **Build:** each ticket goes to a fresh builder on the model below, which builds it test-first, reviews it and commits.
3. **Stuck:** if a ticket fails twice, `debugger` (Fable) finds the root cause.
4. **Review:** `reviewer` (Fable) checks the whole milestone and builders fix what it finds (at most 2 rounds).
5. **CHECKPOINT:** you test the milestone, approve the next milestone's tickets and type `continue`.

| ✓ | Milestone | Build with | CHECKPOINT: what you test |
|---|---|---|---|
| [ ] | M0 Foundations | Sonnet | Opens a repo; quitting leaves the terminal clean |
| [ ] | **M1 The graph** | **Fable** | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [ ] | M2 Details and diffs | Sonnet (Opus for Mistake Radar) | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash | Sonnet (Opus for undo) | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes | Sonnet | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver | Opus | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools | Sonnet (Opus for interactive rebase) | Interactive rebase, blame, line history, reflog |
| [ ] | M8 Polish, **final name**, ship | Sonnet | Pick the name (below), then a fresh install and the first-run tour |

**Naming (an M8 job for Sonnet):** pick a name in the style of Sublime Merge or Araxis Merge (a naming-pattern reference only: Araxis Merge is a diff/merge tool, not a Git client, and is not prior art for this project), with no "git" in it (Git's trademark policy forbids "git" inside names like "tergit"). Starting shortlist: Tributary (command `trib`) and Thicket passed the clash check on 2026-09-11. Graft is taken.

**Every 2–3 milestones** (after M2, M5 and M7), the conductor also runs `/improve-codebase-architecture`.

---

## Session log

<!-- The conductor adds one line per helper run:
- YYYY-MM-DD · step or ticket · model · done, partial or failed · one-line note
-->
