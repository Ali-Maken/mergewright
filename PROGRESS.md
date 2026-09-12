# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** **M1 The graph** — ticket 05 (17 tickets in `.scratch/m1-the-graph/`). M0 done and approved by the user 2026-09-12. **M1 runs straight through without stopping for ticket approval** — see "How M1 runs" below.

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
| [x] | M0 Foundations | Opus | Opus | Opens a repo; quitting leaves the terminal clean |
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

## M0 checkpoint: approved 2026-09-12

The user ran M0 in a real terminal and approved it. **Observed by eye**, which
closes the standing "no emulator has ever answered a probe, no human has seen
the screen" caveat:

- Status screen correct: repo, HEAD on `main`.
- **Pixel tier via the Kitty protocol**, cell size **17x37**.
- **Dark** theme, taken from the terminal's own background.
- **Truecolor**.
- Capability probe answered in **367 µs** — against a 200 ms budget.
- `q` exits clean.

**ADR-0014 confirmed**: all six DESIGN.md divergences stand. Keep the code,
the amended DESIGN.md is correct.

**Raised by the user, logged not fixed:** sanitising strips the emoji joiner
(U+200D), so a multi-part emoji breaks into its pieces in a commit subject.
Accepted as the right safety trade; in `KNOWN_GAPS.md` as revisit-only-if-it-annoys-him.

Still unseen: emulators other than his daily one, tmux, SSH, and the 16-colour
and ASCII fallback paths.

---

## How M1 runs (set by the user, 2026-09-12)

**M1 builds straight through. Do not stop for ticket approval.** The 17
tickets in `.scratch/m1-the-graph/` are approved as drafted — the user did not
want to review them one by one.

- Build every ticket in dependency order, dispatching per each ticket's
  `Build with` line (Fable for 05, 06, 07 — the graph-crate algorithm; Opus for
  the rest).
- Run `reviewer` on the milestone as usual, and fix what it finds.
- **Stop only if something Critical survives the review rounds.** Majors and
  Minors get fixed and the milestone carries on.
- The M1 checkpoint itself still stands: the user tests `git/git` and the
  merge-mistake fixtures and judges whether the history shape is obvious at a
  glance. That is his call, not a reviewer's.

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
- 2026-09-12 · M0/03 terminal session + panic hook · opus · done · RAII restore verified under a pty on quit and panic (termios byte-identical); signal gap moved to ticket 09; commit 32a6268
- 2026-09-12 · M0/04 capability detection · opus · done · 6 queries + DA1 sentinel, 200ms budget; only a bare pty seen so far — no emulator has answered. User probe run queued for the M0 checkpoint; commit 7b9bef9
- 2026-09-12 · M0/05 sanitizer · opus · done · 22 tests incl. proptest; unsanitised construction banned at the type level; 2 spec deviations queued for the M0 checkpoint; commit 5081758
- 2026-09-12 · M0/06 theme tokens · opus · done · 3 themes x 23 tokens x 3 depths, checked by eye on a swatch grid; fixed OKLab hue drift (dark bg -> navy). 16-colour lane collapse 12->6 is real, 3 gaps logged, expiry M1; commit 169e21a
- 2026-09-12 · M0/07 config · opus · done · half-valid file errors with file+line+column, never silent defaults; spellings locked to ticket 04/06 types by exhaustive match
- 2026-09-12 · M0/08 open repo · opus · done · bare/worktree/submodule/detached/unborn/not-a-repo all exercised against real git 2.50.1; non-UTF-8 paths untestable on APFS, gap logged; commit e85ca86
- 2026-09-12 · M0/09 event loop + signals · opus · done · SIGINT/SIGTERM/SIGHUP each leave termios byte-identical, measured under a pty; ticket 03's signal gap closed. MSRV 1.85->1.88. Real-terminal run still unwitnessed; commit 50c3ecd
- 2026-09-12 · M0/10 job runner · opus · done · no-sleep deterministic tests, bounded shutdown reports Stalled rather than hanging; Event::JobFailed now carries panic/git detail; commit a423927
- 2026-09-12 · M0/11 open-repo screen · opus · partial (limit) then done by a fresh opus · all of M0 wired; binary run in a pty: opens repo, q exits 0 clean, non-repo exits 1 clean, NO_COLOR honoured; commit 7bd7ba7
- 2026-09-12 · M0/12 wrap · opus · done · README/CHANGELOG, gaps swept honestly (closed nothing, added 4); commit ceb0712
- 2026-09-12 · M0 review round 1 · opus · done · 0 Critical, 3 Major, 3 Minor; docs/reviews/m0-foundations.md, commit 3592226
- 2026-09-12 · M0 review fixes · opus · done · all 6 fixed: honest TierSource, workspace lints + ADR-0013 (unsafe), detect/ split, ADR-0014 + DESIGN.md edits; commits 1691ea7, 33cb445, cc60d4f
- 2026-09-12 · M0 review round 2 · opus · done · all 6 genuinely resolved; 0 Critical, 0 Major, 3 new Minor; docs/reviews/m0-foundations-round-2.md, commit 9c14cbc
- 2026-09-12 · M1 ticket split · opus · cancelled · user asked to stop after M0; no M1 files written
- 2026-09-12 · M1 ticket split · opus · done · 17 tickets in .scratch/m1-the-graph/; Fable = 05/06/07 (layout engine) only; Unicode tier ordered before pixel; Map view, Overview strip and hover's "Contained in" deferred to M2
- 2026-09-12 · M1/01 fixture corpus · opus · done · new dev-only crate graft-fixtures, git fast-import, 13 shapes, deterministic (two independent builds compared), 303 tests, build 1.15s/3.1MiB; non-UTF-8 ref-name fixture unbuilt on APFS (gap); commits 14b725a, ebdfa47
- 2026-09-12 · M1/02 heavy tier · opus · done · heavy-tests feature + #[ignore]; 100k fixture built for real (2.1s, 35MiB); getrusage peak-RSS helper (ADR-0015, amends ADR-0013); proptest 64/1000; scripts/clone-git-git.sh run (85,649 commits, 42MB); TESTING.md with the manual checklist. Fast tier 307 tests/4.3s, 0 ignored-and-run. Two open items for the user: the heavy tier needs two commands not PROMPT.md §7.1's one, and PROPTEST_CASES=64 in this machine's env silently overrides the tier; commit 90c3bb0
- 2026-09-12 · M1/02 heavy tier harness · opus · done · 100k fixture really built (2.1s, 35MiB); git/git cloned 85,649 commits/1,017 refs/42MB/15s; TESTING.md, ADR-0015; fast 307/307, heavy 4/4; commits 90c3bb0, 8ab260d

- 2026-09-12 · M1/03 tier order + M0 minors · opus · done · ADR-0016: Kitty > Unicode > iTerm2 > Sixel > ASCII (Unicode is first-class, not fallback); DESIGN.md §4.2-4.6 amended, detect::tier changed to match, M0's 3 Minors cleared; new gap: Sixel can win the tier with no encoder (owner M1/14); commit 3915275

- 2026-09-12 · M1/04 reading history · opus · done · graft-git/history/ (simple, topo, date orders) checked against git rev-list on the corpus; branches/, repos.rs, cancellable jobs (Esc); 364 tests. Nothing on screen yet - no widget reads AppState::history; 6 gaps logged; commit 331971b

## Open questions for the M1 checkpoint

- **Heavy tier is two commands, not one.** `cargo test -- --ignored` runs *only*
  ignored tests, so PROMPT.md §7.1's single heavy-tier command never reaches the
  proptests. Fix either way: amend §7.1 to two commands, or drop `--ignored` and
  gate the heavy tier on the feature flag alone. Raised by M1/02.
- **`PROPTEST_CASES=64` is exported in the user's shell env** (not from this
  repo). It silently overrides the heavy tier's case count. Heavy runs need
  `env -u PROPTEST_CASES`, or the export should go.
