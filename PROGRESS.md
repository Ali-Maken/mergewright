# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** **M1 The graph** — ticket 12 (31, 07–11 done; 31 tickets in `.scratch/m1-the-graph/`). M0 done and approved by the user 2026-09-12. M1 follows the read-only checkpoint policy (CLAUDE.md).

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

1. **Design, then tickets:** `planner-fable` writes `docs/design-notes/<milestone>.md` (interfaces, hard-to-reverse choices, risky tickets; no code). The conductor splits the milestone into tickets from it with `/to-tickets`.
2. **Build:** each ticket goes to a fresh builder on its `Build with` model, which builds it test-first, reviews it and commits.
3. **Stuck:** escalation per CLAUDE.md "Standing rules → Models".
4. **Review:** `reviewer` checks the whole milestone (model in the table) and builders fix what it finds (at most 2 rounds).
5. **CHECKPOINT:** you test the milestone, approve the next milestone's tickets (write milestones only) and type `continue`.

| ✓ | Milestone | Build with (per ticket, CLAUDE.md standing rules) | Review | CHECKPOINT: what you test |
|---|---|---|---|---|
| [x] | M0 Foundations | Opus | Opus | Opens a repo; quitting leaves the terminal clean |
| [ ] | **M1 The graph** | per ticket's `Build with` line | Opus | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [ ] | M2 Details and diffs (read-only) | per ticket | Opus | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash (write) | per ticket | Fable | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes (write) | per ticket | Opus | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview (write) | per ticket | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver (write) | per ticket | Opus | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools (read-only) | per ticket | Opus | Interactive rebase, blame, line history, reflog |
| [ ] | M8 Polish, **final name**, ship (read-only) | per ticket | Opus | Pick the name (below), then a fresh install and the first-run tour |

**Who builds what, ticket size, context, checkpoints:** CLAUDE.md, "Standing rules". Not restated here.

**M2 design pass** (`planner-fable`) runs when M1 finishes. Its New UI section must cover: the detail pane, diff views, the Mistake Radar Explain panel, and the Map view and Overview strip deferred from M1.

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

## How M1 runs

M1 follows the read-only checkpoint policy (CLAUDE.md, "Standing rules → Checkpoint policy"); its tickets were approved as drafted on 2026-09-12.

---

## Session log

<!-- The conductor adds one line per helper run:
- YYYY-MM-DD · step or ticket · model · done, partial or failed · one-line note
Append `· COMPACTED` when the helper compacted mid-ticket: that ticket was too big.
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

- 2026-09-12 · M1/05 layout engine · fable · done · resumed after interrupt (3c30a91, 8a2aa1b kept, nothing rewritten); added 4 §1.2 invariant proptests, outside-in replay, random chunking, 18 corpus snapshots read row by row; proptests proved themselves by catching 2 deliberate layout mutations. 5 gaps logged. commits bbb2fd0, 7b07d2a, b07a588, e7db822

- 2026-09-12 · M1/06 streaming layout · fable · done · chunks, checkpoints, LRU eviction, promotion; R2-1 closed (checkpoint 1,578 B @10k / 1,694 B @40k, identical on both paths). Observed on 100k in-memory rows: 16 chunks/8,000 rows resident, 49 checkpoints = 90 kB, EmittedOrder 9.0 MB; jump to row 15,000 replayed 300 rows, 0 walk pulls; evicted chunk rebuilt equal. Real 100k fixture, getrusage/300 MB and the 150 ms budget NOT measured → m1-16. 421 tests. commits 4061028, 695db1b, 06cc3d2, 33d0f71
- 2026-09-12 · proptest counts moved into code · opus (conductor) · done · ADR-0017: PROPTEST_CASES removed from .claude/settings.json and from graft-fixtures; 64 fast / 1000 heavy set by feature flag; PROMPT.md §7.1, TESTING.md, KNOWN_GAPS and the checkpoint question corrected (the variable was this repo's, not the user's shell); commits ecad6dd, 8105193
- 2026-09-13 · M1/31 heavy tests feature-only · sonnet · done · #[ignore] removed; default --list 0 heavy::, with feature 4; clippy clean both ways; heavy tier not run; commit d606cb3
- 2026-09-13 · M1/07 history streams into layout · opus · done · WalkSource/seeds_for feed StreamingLayout via jobs; status screen shows rows + streaming/complete/cancelled; config graph.default_branch; pty: 20k in steps of 500, 100k Esc at 1.0s → cancelled at 22,000 rows; 432 tests; 3 gaps (AppState keeps every row, first advance not sized to 150 ms, Esc drops the in-flight chunk); MAP.md for graft, graft-ui; first try stopped by user with nothing written; commit 34dfc28 · COMPACTED (once, after pty runs; nothing left to split)
- 2026-09-13 · M1/08 Unicode graph column · opus · done · Row.lanes per column + BranchIdentity::slot; graph_column/ in graft-ui; murmur hue hash after octopus shared main's hue; capture docs/shots/08-unicode-graph-column.txt for user's eye; 452 tests; 3 gaps (adjacent hue collisions, HEAD glyph hides merge, ambiguous-width glyphs); 164k tokens, no compaction; commit 8689b5d
- 2026-09-13 · M1/09 graph screen · opus · partial · history job answers row requests with a bounded window; cursor + j/k/PgUp/PgDn/g/G; Esc drops pending G; no graph screen yet (app still opens on status); 456 tests; 3 gaps (history job holds 1 of 4 workers, layout advances on demand only vs DESIGN.md §3.6, nothing draws history); 160k tokens, no compaction; commit 47f624b
- 2026-09-13 · M1/09 graph screen (continuation 1) · opus · done · app opens on the graph, F2 status; pty-only bug fixed (0x0 first state drew 1 row); expect under pty: j/k, q exits 0; git/git first graph frame 567 ms debug build, one run (budget checked in 27/30); capture docs/shots/09-graph-screen.txt; background advance stays with 27; gaps: HEAD node unmarked, git/git ~28 lanes fill the column; 468 tests; 140k tokens, no compaction; commit bf71a25
- 2026-09-13 · M1/10 row text columns · sonnet · done · columns + .mailmap via gix (mailmap.rs); insta snapshots 80x24/120x40/200x60; capture docs/shots/10-row-columns.txt (3 sizes x dark/light) for user's eye; no new gaps; commit d1c2aa1 · COMPACTED (once, finished in its continuation; nothing left to split)
- 2026-09-13 · M1/11 ref badges · opus · done · badges + upstream ahead/behind; HEAD node ◆, newest stash ■ (09 gap closed); pty + pyte read-back on this repo, ref-states, stash, detached-head, all exit 0; capture docs/shots/11-ref-badges.txt (colour/bold/Nerd glyphs unseen in text dump); 506 tests; 5 gaps (node glyphs ignore ui.icons, only newest stash badged, ahead/behind trusts commit dates, 50k refs unmeasured, ref-states fixture ends on unborn orphan); commit 2bbe8e8 · COMPACTED (once; nothing left to split)

## Open questions for the M1 checkpoint

- **Node glyphs ignore `ui.icons`** (m1/11): ◆ HEAD / ■ stash draw even with icons off. Keep, or follow the setting?
- ~~**Heavy tier is two commands, not one.**~~ **Decided 2026-09-13:** drop
  `#[ignore]`, gate on the feature alone (ADR-0018). Code change in m1/31, built before 07.
  Nothing for the checkpoint to decide.
- ~~**`PROPTEST_CASES=64` is exported in the user's shell env**~~ — **wrong,
  and now fixed.** It was set in this repository's own `.claude/settings.json`,
  not the user's shell; M1/02 reported it as external and PROGRESS.md and
  KNOWN_GAPS.md repeated that for a day. Settled 2026-09-12: the variable is
  removed and the case counts live in code (64 fast, 1,000 heavy), ADR-0017.
  Nothing for the checkpoint to decide.
