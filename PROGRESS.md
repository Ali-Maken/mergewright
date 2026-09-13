# Graft: Progress

**What's next?** The first unchecked box below.

Start Claude Code in this folder in auto mode with `claude --permission-mode auto` (the status bar should show `⏵⏵ auto mode on`), switch to **Opus** with `/model`, and run `/conduct`. Don't use plan mode: it blocks the file writes both the interview and the builders need. It works through the boxes on its own, handing each job to a fresh helper agent, and **stops at every box marked You or CHECKPOINT**. When you've done your part, type `continue`.

**Current step:** **M1 The graph** — ticket 33 (31, 07–26, 32 done; user Ghostty check: pixels ok, found wheel-scroll wrap → 32; user chose F3 as a cycle → 33; tooltip fix + pin on Enter + author/date (user decisions) → 34; then 27; 24 split into 24b/24c/24d; 37 tickets in `.scratch/m1-the-graph/`). M0 done and approved by the user 2026-09-12. M1 follows the read-only checkpoint policy (CLAUDE.md).

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
| [ ] | **M1 The graph** | per ticket's `Build with` line | Opus | `git/git` and the merge-mistake fixtures: is the history shape obvious at a glance? |
| [ ] | M2 Details and diffs (read-only) | per ticket; Fable: Mistake Radar detection | Fable | Diffs look right; bad merges get flagged |
| [ ] | M3 Working tree, undo and stash (write) | per ticket; Fable: undo per-kind inverse | Fable | Stage a hunk, commit, undo it, stash and unstash, then try to lose work |
| [ ] | M4 Remotes (write) | per ticket | Fable | Fetch, pull and push on a real remote; wrong password |
| [ ] | M5 Merge, rebase, reset, Ghost Preview (write) | per ticket; Fable: Ghost Preview planning | Fable | Every history change is previewed and can be undone |
| [ ] | M6 Conflict resolver (write) | per ticket; Fable: conflict and operation state machine | Fable | Resolve a real conflict; kill mid-rebase, relaunch |
| [ ] | M7 Power tools (read-only) | per ticket | Fable | Interactive rebase, blame, line history, reflog |
| [ ] | M8 Polish, **final name**, ship (read-only) | per ticket; Fable: architecture pass over MAP.md files | Fable | Pick the name (below), then a fresh install and the first-run tour |

**Who designs, specifies, builds and reviews; ticket size, context, checkpoints:** CLAUDE.md, "Standing rules". Not restated here. Fable's share was raised for M2 onward on 2026-09-13; M1 keeps its tickets and its Opus review. Record of who did what: `docs/AUTHORSHIP.md`.

**M2 design pass** (`planner-fable`, note and ticket specs) runs when M1 finishes. Its New UI section must cover: the detail pane, diff views, the Mistake Radar Explain panel, and the Map view and Overview strip deferred from M1.

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
- 2026-09-13 · M1/12 uncommitted row + chrome · sonnet · partial · graft_git::status::counts + 8 fixture tests; app wiring, virtual row, 16-colour separator, capture not started; handoff in ticket; split into 12b (wiring + row) and 12c (16-colour chrome), 13 now blocked by both; commit 40163c3 · COMPACTED (once)
- 2026-09-13 · M1/12b uncommitted row wiring · sonnet · done · working_tree read job on open + Ctrl+R; virtual row via sentinel Row (zero Oid, drawing only), GraphLine.row now Cow<Row>; shows when HEAD is topmost visible row or branch unborn, hidden while refs load; 3 pty captures (dirty, unborn, clean) docs/shots/12b-uncommitted-row.txt; 522 tests; commit b1320c5 · COMPACTED (once; nothing left to split)
- 2026-09-13 · M1/12c 16-colour chrome · sonnet · done · Theme::color_depth(); at Palette16 highlighted row gets an accent ▏ rule instead of raised bg, truecolor snapshot unchanged; pty capture TERM=xterm docs/shots/12c-palette16-chrome.txt; KNOWN_GAPS M0 "dark chrome flattens" closed; 125k tokens, no compaction; commit 14fa94e
- 2026-09-13 · M1/13 lanes without hue · opus · done · at 16 colours and NO_COLOR odd-column lanes draw dashed (╎ ╌); truecolor/256 unchanged; NO_COLOR highlighted row gets ▏; ADR-0019; corpus test: same-colour neighbours never share a glyph; pyte replay both modes; captures docs/shots/13-{foxtrot-merge,criss-cross-merge}-{16,no-color}.txt; 530 tests; 2 gaps closed, 08 hue gap partial (truecolor neighbours told apart by position only), new: 14 must keep dash pattern, corners undashed; 120k tokens, no compaction; commit ee8b594
- 2026-09-13 · M1/14 compaction · fable · done · fold.rs: per window fold untouched then least-recently-touched lanes (never lane 0/reserved/pinned/node), +N marker, `+` expands, graph.max_lanes; proptests + real binary on git/git (80x24: 7 lanes + `+17`); captures docs/shots/14-*.txt (5); closed 09 + 13 gaps; new gaps: layout incoming column absent from Row.lanes, no corpus shape for compaction, node-heavy windows exceed limit (15); ARCHITECTURE.md updated; commit a811bad · COMPACTED (once; finished, nothing left to split)
- 2026-09-13 · M1/15 folding + first-parent · opus · partial · graft-graph only: inclusion.rs predicate (first-parent, stashes, folded merges) before layout, stream truncate re-lays-out only invalidated rows; 4 acceptance tests; no keys/app wiring/captures; 557 tests; gaps: no wiring, reinclude re-reads from walk top (27), trimmed chunk loses fingerprint (review); 153k tokens, stopped early, no compaction; commit 6355c75
- 2026-09-13 · M1/15 folding + first-parent (continuation 1) · opus · done · keys Space fold/unfold merge, 1 first-parent, H hide stash (added to DESIGN.md §2.3); one layout per view, switching back rebuilds no chunks; release binary in pty, unfold and 1-1 byte-identical to full; captures docs/shots/15-*.txt (6); closed 14 many-node-lanes + wiring gaps; logged: tied commit dates put parent above child (pre-existing), folded merge has no glyph, `1` resets to top, keys not in footer; 564 tests; 140k tokens, no compaction; commit 031de51
- 2026-09-13 · M1/16 selection and focus · opus · done · ancestry.rs; selection draws thread lanes bold, others dim; F toggles Focus (dims unrelated rows), Esc leaves Focus first; 1/Space/H re-find the selection; thread lanes never fold; 4 criss-cross snapshots truecolor + 16; release-binary captures docs/shots/16-*.txt (5); `1` gap narrowed (only within first 2 screens); new: thread limited to rows on hand, bold/dim only; 576 tests; 149k tokens, no compaction; commit 14f54ed
- 2026-09-13 · M1/17 mouse and tooltip · sonnet · done · click selects row, wheel scrolls viewport not cursor, Enter opens tooltip, Esc closes it first; tooltip sanitises escapes; release binary in pty with SGR mouse bytes via pyte; captures docs/shots/17-{mouse-select-and-tooltip,tooltip-sanitizes-escape}.txt; "Contained in" → M2 in KNOWN_GAPS; workspace green; 132k tokens · COMPACTED (once; finished, nothing left to split)
- 2026-09-13 · M1/18 promotion and clean startup · opus · done · provisional → topo promotion keeps selected commit and screen offset (SkewedDates fixture, row 12→11), superseded sort dropped, Esc keeps provisional order and says so; non-repo exits 1 without alt screen; deviated: WalkSource::in_order under Included (from_order skipped view filter); captures docs/shots/18-{not-a-repo,promotion-skewed-dates}.txt; Esc mid-sort unseen in pty (sort takes ms); `1` >2 screens gap still open; 5 gaps closed, new: sort ignores stop, walk silent while promotion catches up, long path hides notice; 590 tests; commit d4a2fa2 · COMPACTED (once; finished, nothing left to split)
- 2026-09-13 · M1/19 ASCII tier and override · sonnet · done · ASCII glyph table; F3 drops the tier; headless path when stdout isn't a tty (piped_stdout test); pty script .scratch/m1-the-graph/pty-tier-drop-check.py, termios identical before/after; captures docs/shots/19-ascii-tier-{drop-key,crisscross}.txt; workspace green; commit a51c702 · COMPACTED (twice, before code was written; finished, nothing left to split)
- 2026-09-13 · M1/20 input mux · opus · done · graft-term input/ own decoder (keys, SGR/X10 mouse, paste, focus) replaces crossterm::event::read; resize from SIGWINCH; ADR-0013 + ARCHITECTURE.md; pty: F2 typeahead during 200 ms probe lands right, resize widens to 110, F3 + M0 termios checks pass; key order proven by socket-pair unit test only, not on screen; capture docs/shots/20-typeahead-f2-during-probe.txt; closed probe-eats-typeahead; logged: 50 ms timeout may split slow SSH replies, on-screen order check, urxvt 1015 mouse; 612 tests; 155k tokens, no compaction; commit 43f0d4a
- 2026-09-13 · M1/21 resize and redetection · opus · done · graft-term detect/redetect.rs + event_loop/redetect.rs; resize re-queries cell size (silence keeps old size marked assumed), focus re-queries theme (silent focus unchanged, forced theme wins); frames and queries share one stdout lock; pty script pty-redetect-check.py; launch at 0x0/1x1 no panic, termios restored; captures docs/shots/21-*.txt (3); 50 ms SSH gap now worse: split replies mid-session arrive as keys (logged, owner 29), plus rare late-reply mix-up; 628 tests; 132k tokens, no compaction; commit 9a371e1
- 2026-09-13 · M1/22 rasteriser · opus · done · graft-graph raster.rs (+node, label), graft-core RasterFrame; new dep tiny-skia 0.12 (simd off for stable snapshot bytes; no ADR — check at review); pixel tests: untouched pixels 0, anti-aliased fork curve, 6 ring styles, +N label, dim/highlight emphasis built now; images always RGB so 13's dashing n/a; 32x5 lanes at 17x37: 1.38 ms release; PNGs docs/shots/22-raster-*.png (7); gaps: node style + CELLS_PER_LANE defined twice, 17x37 only; 635 tests; 136k tokens, no compaction; commit 6233a2a
- 2026-09-13 · M1/23 strip cache · opus · done · raster/cache.rs: key = start row, row count, per-row layout hash (incl. emphasis, thread, edges above, +N), theme id, cell size; bounded LRU, flush on theme/cell size/redetection/resize, scrolling never flushes; 10 tests, 4 broke under deliberate mutations; strip 32 rows at 17x37 x 12 lanes: 1.36 ms, 1.8 MiB; 5 strips ≈ 9.2 MiB; flush not yet wired to 21's events (→ 24); 645 tests; 90k tokens, no compaction; commit 7b407a7
- 2026-09-13 · M1/24 kitty output · opus · partial · graft-term::kitty encoder + q=1 reply parser: transmit-once strips, p=1 crop replacement, ENOENT retransmit, free_all; 18 byte-stream tests; no app wiring, no real-Ghostty run, no capture; 105k tokens, no compaction; remainder too big for one context → split along handoff seams into 24b (aligned strip rows), 24c (draw + replies + gfx status), 24d (flush, F3, Ghostty check); 25 → blocked by 24d, 29 → 24d, 25; 660 tests; commit f313456
- 2026-09-13 · M1/24b aligned strip rows · opus · done · padded the request (History::strip_rows), not an offset key; view/strips.rs pixel_rows/StripRows/StripPaint; text tiers unchanged (tested); uncommitted row no longer counts toward strip height; nothing sets strip_rows yet (→ 24d); tests only, no pixels; 671 tests; 142k tokens, no compaction; commit 4714e89
- 2026-09-13 · M1/24c kitty draw + replies · opus · done · event_loop/graphics.rs: frame inside hold_output, replies drained, gfx ok / gfx: <msg>, free_all on exit, hide on F2; redetect ignores graphics replies (tested); byte-stream tests only, no real terminal; gaps: column width from text lane count (stretch?), replies read only on other wake, panic leaves images, fixed cache 16; resize has no forget_uploads + ratatui clears on resize (→ 24d); 680 tests; 156k tokens, no compaction; commit d6312b3
- 2026-09-13 · M1/24d flush + F3 + Ghostty check · opus · done · cache flush on theme/cell size/resize + forget_uploads, F3 Pixel→Unicode, strip_rows set from settled tier, forced pixel attempts Kitty; examples/kitty_check.rs run in real Ghostty 1.3.1: transmit/crop/clear+retransmit/hide/free DA1-fenced, 0 errors, control ENOENT heard (synthetic images via real encoder, not the app); captures docs/shots/24d-kitty-check*; user command in ticket, by-eye unticked; 24 marked done except by-eye; gaps: retransmit after clear leaks old ids, wide strip squeezed, resize redraws all strips; 686 tests; 136k tokens, no compaction; commit 3e28dc7
- 2026-09-13 · M1/25 iterm2 + resize frames · opus · done · graft-term::iterm2 (png 0.18 dep); resize retransmit was ratatui's full clear, not resize → row-by-row clear (row_clear.rs), no strip flush on resize, closes 3 gaps from 24d; real Ghostty: row erase then place 0 errors, full clear 2/2 ENOENT; iTerm2 pty capture byte-identical across resize; resize frame release in-memory Kitty 0.30 ms / iTerm2 0.25 ms; not verified: real iTerm2 by eye, real Ghostty window resize; ADR-0012 addendum; proposes DESIGN §3.8 drop resize flush; 698 tests; 156k tokens, no compaction; commit f683354
- 2026-09-13 · M1/26 search · opus · done · search.rs own job re-walking history; pty+pyte release 100k fixture: far hit 641 ms Enter→row, 401 F2 toggles during search median 3.5 ms max 9.2 ms none missed; captures docs/shots/26-search-far.txt, 26-search-typing.txt (by eye unticked); ESC in subject drawn inert (tested); gaps: only current hit marked, hidden hits say "(not in this view)", n before first hit no-op; 719 tests; commit 8548e22 · COMPACTED (once; finished, nothing left to split)
- 2026-09-13 · M1/32 wheel scroll wrap (user bug) · opus · done · cause: History::apply ran top = top.min(cursor) on every answer, wheel leaves cursor at 0 so view snapped to top once walk finished (not unloaded rows); fix keep_within_rows, search Esc restore had same fault; wheel test failed before (row 189→0), PgDn fine; pty 100 notches: before wraps at 26, after stops on root; captures docs/shots/32-wheel-*; by eye unticked; 721 tests; 73k tokens, no compaction; commit 27a4886

## Open questions for the M1 checkpoint
- **Proposed change (m1/25): DESIGN §3.8 drop the strip flush on resize.** Resize only needs row-by-row clearing; uploads survive. Approve / keep flush. Also: png 0.18 added to graft-term for iTerm2 (reviewer to check crate rules).

- **New keys from m1/15** (DESIGN.md had none): `Space` fold/unfold merge, `1` first-parent, `H` hide stash; m1/17 adds `Enter` opens tooltip, wheel scrolls view not cursor; m1/19 adds `F3` drops to the ASCII tier. Keep, or rebind?
- **Uncommitted row only shows when HEAD is the top visible row** (m1/12b). If another branch has newer commits above HEAD, or you scroll, the dirty-tree row is hidden. Keep, or pin it to the top?
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
