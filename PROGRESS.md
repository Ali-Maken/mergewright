# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** M3 building, group 01 of 7 (2026-09-19). Design pass and specs are done (`.scratch/m3-working-tree-undo-and-stash/issues/`); the M3 start stop is answered — ADR-0034 records the six implementation deltas and the three scope cuts the user kept. M2 is closed; its checkpoint answers are ADR-0030 to ADR-0033.

**M3 must be daily-usable the day it lands** (ADR-0030) — the user starts using Graft for real work then, and those findings shape M5 and M6 while they are still being designed. `docs/field-notes.md` and TESTING.md's "Times I left" start collecting from that day.


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

1. **Design and group specs:** `planner-fable` writes `docs/design-notes/<milestone>.md` (interfaces, hard-to-reverse choices, risky work, new UI; no code) **and every group spec** (scope, the pieces inside, signatures, acceptance, Read first, `Build with`), in two passes past ~8 groups. The conductor numbers and sequences them; it doesn't re-decide them. `/to-tickets` was used for M0 and M1 only.
2. **Build:** each **group** goes to one fresh builder on its `Build with` model, which builds its pieces test-first in one context, reviews and commits. Groups cluster by shared files (ADR-0029).
3. **Stuck:** escalation per CLAUDE.md "Standing rules → Models".
4. **Review:** `reviewer` checks each group from its diff and the MAP.md files, **one round**; what's left goes to KNOWN_GAPS with an expiry, marked broken or unverified.
5. **CHECKPOINT:** one stop per milestone (ADR-0029). You test it and type `continue`. **No ticket approval on any milestone** — the conductor stops unasked only for a visual result, unprovable-inverse work, a product preference, or a spec/scope change.

| ✓ | Milestone | Build with (per ticket, CLAUDE.md standing rules) | Review | CHECKPOINT: what you test |
|---|---|---|---|---|
| [x] | M0 Foundations | Opus | Opus | Opens a repo; quitting leaves the terminal clean |
| [x] | **M1 The graph** | per ticket's `Build with` line | Opus | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [x] | M2 Details and diffs (read-only) | per ticket; Fable: Mistake Radar detection | Fable | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash (write) | per group; Fable: undo per-kind inverse | Fable | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes (write) | per group | Fable | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview (write) | per group; Fable: Ghost Preview planning | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver (write) | per group; Fable: conflict and operation state machine | Fable | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools (read-only) — **conditional, judged on real use** (ADR-0030) | per group | Fable | Interactive rebase, blame, line history, reflog, **branch-dependency + chain rebase** (ADR-0032) |
| [ ] | M8 Polish, **final name**, ship (read-only) — **conditional** (ADR-0030) | per group; Fable: architecture pass over MAP.md files | Fable | Pick the name (below), then a fresh install and the first-run tour |

**Who designs, specifies, builds and reviews; ticket size, context, checkpoints:** CLAUDE.md, "Standing rules". Not restated here. Fable's share was raised for M2 onward on 2026-09-13; M1 keeps its tickets and its Opus review. Record of who did what: `docs/AUTHORSHIP.md`.

**M2 design pass** (`planner-fable`, note and ticket specs) runs when M1 finishes. Its New UI section must cover: the detail pane, diff views, the Mistake Radar Explain panel, and the Map view and Overview strip deferred from M1.

## M3 design pass brief — ADRs the user approves before any M3 ticket is built (user, 2026-09-17)

**Answered at the M2 checkpoint, 2026-09-19.** Points 3, 4 and 5 are now decided and written up:
**ADR-0030** (order is M3 → M4 → M5 → M6; remotes are daily, not "after"; M7/M8 conditional on real use; the user's daily use starts the day M3 lands),
**ADR-0031** (the journal answers whole-repo-state-at-a-time from day one; the time machine appears in M5 behind Ghost Preview),
**ADR-0032** (branch dependency + chain rebase, not Graphite, M7, conditional),
**ADR-0033** ("Times I left" — the promise is measured).
Points 1 and 2 (the four-property operation shape, and the `ops`/`app` seam) are the conductor's to decide with `planner-fable`, per the user's 2026-09-19 instruction that implementation is not his to approve.
The original text stands below as the brief's reasoning.

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

Live at [docs/log/m3.md](docs/log/m3.md) — the current milestone's log
(moved out, housekeeping/01 continuation 1, 2026-09-18, to keep this file
under 10 KB). Finished milestones: [docs/log/m0.md](docs/log/m0.md),
[docs/log/m1.md](docs/log/m1.md), [docs/log/m2.md](docs/log/m2.md).
