# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** M0 Foundations — ticket 03 (12 tickets in `.scratch/m0-foundations/`, approved 2026-09-12)

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

- [x] **2. Write DESIGN.md** (conductor → `builder-fable`). Task:
  > Read PROMPT.md, CONTEXT.md and the ADRs. Do the first step of section 10: write DESIGN.md with ASCII mockups of every main view, the full keymap, the graph layout algorithm in prose and pseudocode, the terminal-capability detection plan, the undo journal design, and the architecture from PROMPT.md 6.1 (crate map, key types, dependency rules). Also write ARCHITECTURE.md. Write them for a human who wants to understand them quickly. Where you see a better approach than the spec, propose it under "Proposed changes" with your reasons (CLAUDE.md, "Challenge the spec"). No code.

- [x] **CHECKPOINT: review DESIGN.md** (You). Read it and ask for changes. For a second opinion, ask the conductor to run `reviewer` on it.

- [x] **3. Graph-drawing prototype** (conductor → `builder-fable`). Task:
  > Build a throwaway prototype in prototypes/graph-spike/: a tiny Rust app that draws a hard-coded 30-commit graph with branches and merges using the Kitty graphics protocol and iTerm2 inline images, lined up with text rows and scrollable with j/k. The goal is to prove that alignment and scrolling work. Record the findings as an ADR.

- [x] **CHECKPOINT: try the prototype and approve the M0 tickets** (You)
  - Run `cargo run --manifest-path prototypes/graph-spike/Cargo.toml` in Ghostty or iTerm2.
  - If it doesn't work, decide the next approach with the conductor before M0.

---

## Phase B: Milestones (all through `/conduct`)

**What the conductor does for every milestone:**

1. **Tickets:** splits the milestone into small tickets with `/to-tickets`. You approve them at the checkpoint before the milestone starts.
2. **Build:** each ticket goes to a fresh builder on the model below, which builds it test-first, reviews it and commits.
3. **Stuck:** if a ticket fails twice, `debugger` (Fable) finds the root cause.
4. **Review:** `reviewer` checks the whole milestone and builders fix what it finds (at most 2 rounds). Opus, except M1, M3 and M5, which review on Fable.
5. **CHECKPOINT:** you test the milestone, approve the next milestone's tickets and type `continue`.

| ✓ | Milestone | Build with | Review | CHECKPOINT: what you test |
|---|---|---|---|---|
| [ ] | M0 Foundations | Opus | Opus | Opens a repo; quitting leaves the terminal clean |
| [ ] | **M1 The graph** | **Fable** for the graph layout engine tickets, **Opus** for the rest | Fable | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [ ] | M2 Details and diffs | Opus | Opus | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash | Opus (xhigh for undo) | Fable | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes | Opus | Opus | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview | Opus | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver | Opus (xhigh) | Opus | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools | Opus (xhigh for interactive rebase) | Opus | Interactive rebase, blame, line history, reflog |
| [ ] | M8 Polish, **final name**, ship | Opus | Opus | Pick the name (below), then a fresh install and the first-run tour |

**Who builds what.** Cost control, set 2026-09-12:
- **Fable** only for the M1 graph layout engine tickets, and the `debugger` agent. Nothing else.
- **Opus** builds everything else, M1's other tickets included.
- **Sonnet** is no longer used for building.
- **Reviews** run on Opus, except M1, M3 and M5, which stay Fable.
- **Effort:** `builder-opus` is `high` by default. The conductor uses `builder-opus-xhigh` only for undo (M3), conflicts (M6), interactive rebase (M7) and the M1 graph layout engine.

**Naming (an M8 job):** pick a name in the style of Sublime Merge or Araxis Merge (a naming-pattern reference only: Araxis Merge is a diff/merge tool, not a Git client, and is not prior art for this project), with no "git" in it (Git's trademark policy forbids "git" inside names like "tergit"). Starting shortlist: Tributary (command `trib`) and Thicket passed the clash check on 2026-09-11. Graft is taken.

**Every 2–3 milestones** (after M2, M5 and M7), the conductor also runs `/improve-codebase-architecture`.

---

## Session log

<!-- The conductor adds one line per helper run:
- YYYY-MM-DD · step or ticket · model · done, partial or failed · one-line note
-->

- 2026-09-11 · step 2 DESIGN.md (sections 1–6) · fable · partial · stopped mid-revision, draft kept
- 2026-09-11 · step 2 architecture + ARCHITECTURE.md · fable · done · DESIGN.md §7–8, ARCHITECTURE.md, commit bbc9c99
- 2026-09-11 · step 2 checkpoint feedback · fable · done · Visualization §5, ADR-0006/0007, mockup + keymap fixes, commit a014578
- 2026-09-11 · step 2 design review · fable · done · 4 Critical, 5 Major, 4 Minor in docs/reviews/step-2-design.md, commit c1f1223
- 2026-09-11 · step 2 Critical fixes C1–C4 · fable · done · commit-graph first paint, row-cache eviction, unconditional snapshots, per-worktree journal; ADR-0008/0009/0010, commit f82df1f
- 2026-09-11 · step 2 Major+Minor fixes · fable · done · M1–M5, N1–N2 fixed, N3/N4 → KNOWN_GAPS, ADR-0011, commit 15d2f10
- 2026-09-11 · step 2 design review round 2 · fable · done · C1–C4 all resolved; 2 new Majors (R2-1, R2-2) in docs/reviews/step-2-design-round-2.md, commit fee1768
- 2026-09-12 · step 2 round-2 fixes · fable · done · backup refs by ULID (R2-2); R2-1/3/4 → KNOWN_GAPS, commit f1af260
- 2026-09-12 · step 3 graph-drawing prototype · fable · done · Kitty crop-scroll + iTerm2 re-send both verified; ADR-0012, prototypes/graph-spike/
- 2026-09-12 · M0 ticket split · fable · done · 12 tickets in .scratch/m0-foundations/, commit ed862e1
- 2026-09-12 · step 3 reopened · — · — · blank graph in Ghostty; pty captures proved emission, not rendering. M0 tickets NOT approved.
- 2026-09-12 · step 3 fix · fable · done · root cause: Ghostty frees kitty image data on `ESC[2J` and `q=2` hid every ENOENT. Fixed: no steady-state clear, retransmit on resize, replace placement `p=1`, `q=1` errors shown in status line, `GRAPH_SPIKE_DEBUG=1` dump. Real-Ghostty run: zero graphics errors. ADR-0012 Correction. Pixels await user's eyes at the checkpoint.
- 2026-09-12 · step 3 perf + row cap · opus · done · 23ms "upload" was a status-line misattribution, not a re-upload; upload-once was intact. Removed MAX_VIEW=24 cap, added `tx N`. commit 78a73a0
- 2026-09-12 · step 3 APPROVED by user · — · — · Ghostty and iTerm2 both verified by eye. M0 tickets approved as drafted, ticket 05 included.
- 2026-09-12 · M0/01 workspace scaffold · opus · done · 7 crates, edition 2024, dep edges match ARCHITECTURE.md; commit 662f108
- 2026-09-12 · M0/02 dependency-edge test · opus · done · cargo metadata --no-deps, exact edge-set equality; proved it fails on a forbidden edge; commit 0dadc65
