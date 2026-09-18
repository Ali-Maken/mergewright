# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** PAUSED at the user's request, 2026-09-18, after M2 review fix round 1c (`ca120c0`).

**Queue on resume, in order:**
1. `housekeeping/02-heap-slope` (opus) — the red gate. M2 is not done over it (user, 2026-09-18). Measure where the bytes go, then get under the 50 B/commit bar or bring the user an ADR arguing the bar with evidence; a bar change needs the user's approval before it lands.
2. M2 review fix round 1d (sonnet) — M7 (ARCHITECTURE.md not updated for M2, PROMPT 6.1), M8 (files far over the ~400-line rule, PROMPT 6.2), and review Minor 15 (TESTING.md and gallery stop 4's caption still describe the old always-on word tint).
3. `reviewer` round 2 (fable) from `183c6ca` — **the second and last of the two allowed rounds.**
4. M2 checkpoint: regenerate `docs/AUTHORSHIP.md` from `docs/log/m2.md`, report the per-ticket tool-call and token figures with causes (user asked for the figures, not an impression), then the M3 design pass — `planner-fable` writes the M3 brief's five points as ADRs, **the user approves them before any M3 ticket is built**, and proposes the rewritten milestone promises and where each of the four product gaps lands.

**For the checkpoint, unresolved and needing the user:**
- `W` (word tint) and the unfocused diff body are fixed and unit-tested but **visually unverified**: `pty-gallery-m2.py` captures are pyte character grids with no colour attributes, so no capture in this repo can prove a colour-only change. The project's way of proving visual work has a hole in it.
- CLAUDE.md reached 15,841 B, not the 10 KB the diet ticket targeted; going lower means dropping requirements, which is the user's call.

---

Finished milestones' logs archived to `docs/log/` (housekeeping/01,
2026-09-18): [M0](docs/log/m0.md), [M1](docs/log/m1.md). The current
milestone's log lives at [docs/log/m2.md](docs/log/m2.md) (moved there in
housekeeping/01 continuation 1, 2026-09-18, to hit this file's size target —
see that file's header). This file keeps the plan and the current step only.

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

1. **Design and ticket specs:** `planner-fable` writes `docs/design-notes/<milestone>.md` (interfaces, hard-to-reverse choices, risky tickets, new UI; no code) **and every ticket spec** (scope, signatures, acceptance, Read first, `Build with`), in two passes past ~12 tickets. The conductor numbers and sequences them; it doesn't re-decide them. `/to-tickets` was used for M0 and M1 only.
2. **Build:** each ticket goes to a fresh builder on its `Build with` model, which builds it test-first, reviews it and commits.
3. **Stuck:** escalation per CLAUDE.md "Standing rules → Models".
4. **Review:** `reviewer` checks the milestone from its diff and the MAP.md files (model in the table) and builders fix what it finds (at most 2 rounds).
5. **CHECKPOINT:** you test the milestone, approve the next milestone's tickets (write milestones only) and type `continue`.

| ✓ | Milestone | Build with (per ticket, CLAUDE.md standing rules) | Review | CHECKPOINT: what you test |
|---|---|---|---|---|
| [x] | M0 Foundations | Opus | Opus | Opens a repo; quitting leaves the terminal clean |
| [x] | **M1 The graph** | per ticket's `Build with` line | Opus | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [ ] | M2 Details and diffs (read-only) | per ticket; Fable: Mistake Radar detection | Fable | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash (write) | per ticket; Fable: undo per-kind inverse | Fable | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes (write) | per ticket | Fable | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview (write) | per ticket; Fable: Ghost Preview planning | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver (write) | per ticket; Fable: conflict and operation state machine | Fable | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools (read-only) | per ticket | Fable | Interactive rebase, blame, line history, reflog |
| [ ] | M8 Polish, **final name**, ship (read-only) | per ticket; Fable: architecture pass over MAP.md files | Fable | Pick the name (below), then a fresh install and the first-run tour |

**Who designs, specifies, builds and reviews; ticket size, context, checkpoints:** CLAUDE.md, "Standing rules". Not restated here. Fable's share was raised for M2 onward on 2026-09-13; M1 keeps its tickets and its Opus review. Record of who did what: `docs/AUTHORSHIP.md`.

**M2 design pass** (`planner-fable`, note and ticket specs) runs when M1 finishes. Its New UI section must cover: the detail pane, diff views, the Mistake Radar Explain panel, and the Map view and Overview strip deferred from M1.

## M3 design pass brief — ADRs the user approves before any M3 ticket is built (user, 2026-09-17)

Filed during M2; **does not change M2's scope**. At the M2 checkpoint, before M3's design note and tickets, `planner-fable` writes these as ADRs; the user approves them; only then do M3 tickets get written.

1. **Every write operation defined by four properties.** In `ops`, one uniform shape per operation (merge, rebase, reset, discard, amend, stash drop, checkout, push): preconditions and what it refuses; the typed git invocation(s); what it changes — refs, index, working tree — which determines the snapshot; and its inverse. Undo follows from the definition instead of being added alongside it; every operation testable without a screen. Use-case idea from clean architecture, **without its ceremony**: the crate graph already enforces direction — no abstract interface per action, no cross-layer mappers. Same treatment for screens: data rendered, keys accepted, focus/empty/error states, commands emitted.
2. **The write seam: what is `ops`, what stays `app`.** Decide before M3's tickets what moves into `ops` (operations, undo journal, Ghost Preview planning) and the rule that keeps `app` wiring rather than deciding. Also: how failure paths get tested — rejected hooks, auth failures, interrupted rebase, an error mid-write — since temp repos cover the happy path and those are where work is lost. (Conductor, 2026-09-17: user's figures were app 24,315 / ops 1,940 and "ops has no MAP.md"; measured now, `src` only: graft 19,378, graft-ops 1,590, and **graft-ops/MAP.md exists** since m2/04. The imbalance stands; the MAP.md line is closed.)
3. **Milestones judged by daily loops, not feature lists.** Rewrite each remaining milestone's promise as a loop: "what changed since yesterday", "prepare and land a commit", "get my branch up to date", "tidy up before a PR", "something went wrong — put it back". The acceptance ticket proves the loop end to end without leaving Graft. The spec is organised by Git's nouns, which is how Git's manual is organised, not how a day is.
4. **Best-in-class study per operation, one ADR per milestone naming what we take and what we reject:** delta (diff presentation), JetBrains 3-way merge (per-conflict navigation, accept left/right/both, auto-resolve trivial), **jj operation log** (closest prior art to the time machine), GitUp (direct-manipulation rebase, instant undo), GitLens (blame annotation, age heatmap), lazygit (keyboard economy), Graphite/Sapling (stacked work — **decide explicitly whether we skip it**).
5. **Four product gaps not in the spec:**
   - **Escape hatch (M4 or M7):** run a git command from inside Graft, output shown, graph refreshed after. For the tail only (`bisect run`, odd flag combinations); never a replacement for the menus, which stay the discoverable path. Pairs with the Command Log, so the tool teaches Git in use.
   - **Git integration hooks (M8):** register Graft as git's difftool, mergetool and pager, so `git diff`, `git mergetool`, `git log` open Graft. The spec currently only launches the user's external mergetool — the opposite direction.
   - **Time machine (M3 or M5):** "put the repo back to how it was N minutes ago", Ghost Preview first. Same undo-journal machinery, framed as the product's identity rather than a feature.
   - **M6 conflict scope additions:** move between conflicted files inside the resolver; undo a resolution already made; re-run the merge with a different strategy.
6. **Already covered, skip:** hooks, signing, credentials, LFS (principle 6). **Deferred on purpose:** sparse-checkout, deeper LFS, multi-repo tabs.

**M7 design pass must carry** (user, 2026-09-17): a ticket for background layout advance while idle (KNOWN_GAPS "Layout advances only as far as the screen asks") whose scope includes **finding the cause** of the unexplained 17–18 ms release key. Not a documented limitation; don't push it a fifth time.

**Naming (an M8 job):** pick a name in the style of Sublime Merge or Araxis Merge (a naming-pattern reference only: Araxis Merge is a diff/merge tool, not a Git client, and is not prior art for this project), with no "git" in it (Git's trademark policy forbids "git" inside names like "tergit"). Starting shortlist: Tributary (command `trib`) and Thicket passed the clash check on 2026-09-11. Graft is taken.

**Every 2–3 milestones** (after M2, M5 and M7), the conductor also runs `/improve-codebase-architecture`.

## Session log

Live at [docs/log/m2.md](docs/log/m2.md) — the current milestone's log
(moved out, housekeeping/01 continuation 1, 2026-09-18, to keep this file
under 10 KB). Finished milestones: [docs/log/m0.md](docs/log/m0.md),
[docs/log/m1.md](docs/log/m1.md).
