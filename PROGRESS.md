# Graft: Progress

**What's next?** The first unchecked box below.

Open this folder in Claude Code on **Opus** and run `/conduct`. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** 0. Setup

---

## Phase A: Setup and decisions

- [ ] **0. Setup** (You)
  - [ ] Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`, then check with `rustc --version`
  - [ ] Claude Code installed and signed in, and **Fable** available on your plan
  - [ ] Ghostty or iTerm2 installed, for the pixel graph
  - [ ] `git --version` shows 2.38 or later (`brew install git` if it's older)
  - [ ] In your terminal: `claude plugins install mattpocock-skills`
  - [ ] Open this folder in Claude Code and run `/setup-matt-pocock-skills`. When asked where issues live, choose **local markdown**. Then commit.
  - [ ] Have 2–3 of your messiest real repos ready for testing

- [ ] **1. Decisions interview** (You, with Opus, in the main session). Run:
  > /grill-with-docs Read PROMPT.md. Interview me one question at a time about decisions only I can make: which workflows I use daily versus rarely, keybinding style, Mac-only or cross-platform, mouse versus keyboard priority, trunk branch names, personal or open-source, and anything in the spec you think is wrong or risky. Already decided: Rust, tickets in local files, the conductor stops at checkpoints, Fable as planned, and the real name is chosen in M8. Don't add features. Propose PROMPT.md edits for my approval.

  From here on, just run `/conduct`.

- [ ] **2. Write DESIGN.md** (conductor → `builder-fable`). Task:
  > Read PROMPT.md, CONTEXT.md and the ADRs. Do the first step of section 10: write DESIGN.md with ASCII mockups of every main view, the full keymap, the graph layout algorithm in prose and pseudocode, the terminal-capability detection plan and the undo journal design. No code.

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
| [ ] | **M1 The graph** | **Fable** | Your messiest repos: is the history shape obvious? |
| [ ] | M2 Details and diffs | Sonnet (Opus for Mistake Radar) | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree and undo | Sonnet (Opus for undo) | Stage a hunk, commit, undo it, then try to lose work |
| [ ] | M4 Merge, rebase, reset, Ghost Preview | Fable | Every history change is previewed and can be undone |
| [ ] | M5 Conflict resolver | Opus | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M6 Remotes | Sonnet | Fetch, pull and push on a real remote; wrong password |
| [ ] | M7 Power tools | Sonnet (Opus for interactive rebase) | Interactive rebase, stash, blame, bisect |
| [ ] | M8 Polish, **final name**, ship | Sonnet | Pick the name (below), then a fresh install and the first-run tour |

**Name shortlist for M8:** Tributary (command `trib`) and Thicket passed the clash check on 2026-09-11. Graft is taken. Names built on "git" (such as "tergit") break Git's trademark policy.

**Every 2–3 milestones** (after M2, M4 and M7), the conductor also runs `/improve-codebase-architecture`.

---

## Session log

<!-- The conductor adds one line per helper run:
- YYYY-MM-DD · step or ticket · model · done, partial or failed · one-line note
-->
