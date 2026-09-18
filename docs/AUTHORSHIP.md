# Authorship

Which model designed, specified, built and reviewed each part of Graft. **Generated from PROGRESS.md's session log** (the conductor regenerates it at every milestone checkpoint). Nothing here goes beyond what the log records; where the log is silent, this file says so.

Roles:
- **Designed:** the design documents the work follows (DESIGN.md, ADRs from design work, design notes).
- **Specified:** who wrote the tickets.
- **Built:** the model of each builder run, by ticket.
- **Reviewed:** the milestone review.

"Conductor" is the main Opus session that dispatches helpers.

## Phase A: setup and decisions

| Step | Model (from the log) | What the log records |
|---|---|---|
| 1. Decisions interview | — | Not in the session log. |
| 2. DESIGN.md + ARCHITECTURE.md | Fable | Written in two runs (one partial), revised after checkpoint feedback. |
| 2. Design review | Fable | Two rounds; round 1: 4 Critical, 5 Major, 4 Minor; fixes by Fable. |
| 3. Graph-drawing prototype | Fable, then Opus | Fable built it and fixed the blank-graph bug; Opus fixed the perf misattribution and the row cap. |

## M0 Foundations

| Role | Model | Detail |
|---|---|---|
| Designed | Fable | DESIGN.md (step 2). No per-milestone design note (the rule came later). |
| Specified | Fable | "M0 ticket split · fable" — 12 tickets. |
| Built | Opus | Tickets 01–12 all Opus; 11 went partial (limit) and a fresh Opus finished it. |
| Reviewed | Opus | Two rounds: 0 Critical, 3 Major, 3 Minor → all fixed by Opus → 3 new Minor. |

## M1 The graph

M1 is grandfathered under CLAUDE.md (existing tickets, `Build with` lines and Opus review stand). It ran long enough that mid-milestone design/spec passes moved to `planner-fable` in practice (M2's routing arrived early, on this milestone's own findings).

| Role | Model | Detail |
|---|---|---|
| Designed | Fable | M1 graph design pass, `docs/design-notes/m1-the-graph.md`: ADR-0022 (fold stubs + rail), ADR-0023 (no hover tooltip, footer reveal + peek + SHA copy), ADR-0024 (pixel thread by alpha). Memory-at-1M design pass: ADR-0026 (handles/commit-graph reads/hydrate-on-demand), amended twice more (split slope guard; no-cg heap floor) in later Fable spec passes. |
| Designed (spec amendments) | Fable / Sonnet | PROMPT.md and DESIGN.md edits for ADR-0022–0025 (folded into ticket 28); ADR-0027 (cold first-paint convention) added to 28's scope by user decision. |
| Specified | Opus, then Fable (mid-milestone) | Opus wrote "M1 ticket split" (17 tickets, later replanned to 31; conductor filed 18–31 as the plan grew). From ticket 36 on, `planner-fable` ran repeated spec-only passes reacting to builder findings (design note above, plus: fold-pinning-vs-lane-cap → ADR-0025, filing 47 and amending 44; `next_links` amendment to 49/52; merge-lane-identity amendment splitting 52/52b; merge_reads-bound amendment to 52b; spec pass 2 filing 57/58/59/60; spec pass 3 filing 58b; slope-guard-remainder pass filing 61 and wontfix-ing 62; no-cg-heap-floor pass 4 filing 63). |
| Specified (conductor-filed ranges) | — | 18–31 (plan growth); 32–34 (user-found bugs/decisions); 36; 46; 48 remainder; 51b–51d; 52b; 56c–56d; 59b; 64; 65–79 (review-round fixes and file splits). |
| Specified (remainder splits) | — | Conductor split partial tickets along their own handoff seams (not a spec pass): 09→continuation; 12→12b/12c; 15→continuation; 24→24b/24c/24d; 27→27b→27c/27d; 30→30b/30c/30d→(30c seams a/b, 30e, 30f seams 1–4, 30g, 30h)→30d part 2; 35→35b/35c; 49 seam a→b; 51→51b (then 51c, 51d); 52→52b→continuation; 55 phases; 58b phases; 59→59b; 70/71 (compacted, too big) → 72a/72b/72c/73a/73b/74a/74b/75a/75b/75c/76a/76b. |
| Built | see table below | |
| Reviewed | Opus | Two rounds (below). |

### M1 builds

| Model | Tickets |
|---|---|
| Opus | 01, 02, 03, 04, 07 (COMPACTED), 08, 09 (partial → continuation done), 11 (COMPACTED), 13, 15 (partial → continuation done), 16, 18 (COMPACTED), 20, 21, 22, 23, 24 (partial → split 24b/24c/24d, all done), 25, 26 (COMPACTED), 27 (partial, COMPACTED → split 27b), 27b (partial ×2 parts) → 27c (done), 27d (done), 29, 30 (partial → split 30b/30c/30d), 30b (partial → part 2 done), 30c (partial, 4 seams/parts across the ticket's life, last closed in 30f seam 4), 30d (partial → part 2 done), 30e (done), 30f (failed once — API session limit, not an escalation, redispatched fresh; then seam 1 partial, seam 2 partial, seam 3 partial, seam 4 partial → continuation done), 30g (done), 30h (failed once — infra watchdog, not an escalation, redispatched fresh → done), 32, 33, 36 (partial, stopped by conductor pending a design pass), 37, 39 (done, user by-eye check pending), 40 (done, user by-eye check pending), 45, 46, 49 (partial seam a → continuation seam b done), 50, 52 (partial, blocked on a spec question → done once spec landed, COMPACTED), 52b (partial → continuation done), 54 (done, COMPACTED), 55 (partial phase A → phase B fix 1 partial → fix 2 done), 58b (partial phase A → phase B done, stop rule fired), 59 (partial → split 59b), 59b (done, stop rule fired), 67, 68, 69 |
| Fable | 05, 06, 14 (COMPACTED) — signature-style builds; 35 (partial, COMPACTED) → 35b (partial) → 35c (done) — acceptance ticket (graph gallery); 38 (done); 56 (partial) → 56d (done) — acceptance ticket (memory curve) |
| Sonnet | 10 (COMPACTED), 12 (partial, split, COMPACTED) → 12b (done, COMPACTED), 12c, 17 (COMPACTED), 19 (COMPACTED twice), 31, 34, 41 (COMPACTED), 42, 43 (done, no bug found), 44, 47 (done, COMPACTED, user by-eye check pending), 48 (partial, conductor-run, COMPACTED → follow-up spec), 51 (partial, COMPACTED → split), 51b, 51c (done, stopped over its byte bar on purpose), 51d, 53, 56c, 57, 58 (partial → residual to spec pass 3), 60, 61, 63, 64, 65, 66, 70 (done, COMPACTED — split update.rs), 71 (done, COMPACTED — split event_loop+jobs), 72a, 72b, 72c, 73a, 73b, 74a, 74b, 75a, 75b, 75c, 76a, 76b, 77, 78, 79 |
| Opus (conductor, not a ticket) | proptest counts moved into code (ADR-0017) |

Escalations: none recorded. The two `failed` runs (30f's first attempt, 30h's first attempt) were infrastructure failures — API session limit and a stream watchdog — not build stalls, so the log explicitly says they don't count; both were redispatched fresh on the same model (Opus) rather than escalated.

**Compacted** (log line carries `· COMPACTED`): 07, 10, 11, 12, 12b, 14, 17, 18, 19, 26, 27, 35 (part a), 41, 47, 48, 51, 52 (layout on links), 54, 70, 71.

### Reviewed

- **Round 1** — Opus. 0 Critical / 4 Major / 8 Minor. Majors: files over 400 lines, `AppState` clone per event, wheel scroll strands search, three past-due KNOWN_GAPS rows (tied dates, dangling edge, piped stdout).
- **Round 2** — Opus. 0 Critical / 1 Major / 5 Minor. Major: the update-module purity guard scanned only `mod.rs` after the split. Round-2 minors on target-directory size, `GRAFT_1M` guard behaviour, and two past-due gaps were left to the user at the checkpoint rather than fixed.
- **Fixes for round 1**, all closed before round 2: 65 wheel-scroll-strands-search (Sonnet), 66 piped-stdout (Sonnet), 67 tied-dates (Opus), 68 incoming-edge-outside-lanes (Opus), 69 AppState-clone (Opus), and the over-400-line splits 70, 71, 72a, 72b, 72c, 73a, 73b, 74a, 74b, 75a, 75b, 75c, 76a, 76b, 77 (all Sonnet).
- **Fixes for round 2**: 78 update-purity-guard (Sonnet), 79 split-kitty_check-example (Sonnet).

## M2 Details and diffs

M2 is the first milestone under the 2026-09-13 standing rules (`planner-fable`
designs and specs; conductor files/sequences; models routed per ticket; Fable
reviews).

| Role | Model | Detail |
|---|---|---|
| Designed | Fable | `docs/design-notes/m2-details-and-diffs.md`, three passes (1, 2a, 2b, 2c) plus later spec-only fixes: resize keys (`<`/`>`), the working-tree row (filed as 06b), and deferring `?` help to M8. 23 tickets total. |
| Specified | Fable, filed by conductor | Ticket specs from the design passes above, unnumbered in `.scratch/m2-details-and-diffs/issues/`; conductor numbered 01–23 and set `Blocked by` from `Needs`. Conductor-filed splits/remainders (not spec passes): 06b (from a spec-only pass), 10b, 13b, 13c, and ticket 24 (found by 23's acceptance walk). |
| Built | see table below | |
| Reviewed | Fable | M1 was the Opus exception; M2 review and both rounds ran on Fable. |

### M2 builds

| Model | Tickets |
|---|---|
| Opus | 01 (done), 06 (partial → continuation done), 14 (partial → continuation done), 15 (done), housekeeping/02 heap-slope (done) |
| Fable | 04 (signature, done, COMPACTED once), 23 (partial at its own planned seam, part a → continuation part b done) |
| Sonnet | 02 (done, COMPACTED — self-described, ticket finished), 03 (done), 05 (done, COMPACTED), 06b (done, COMPACTED), 07 (done, COMPACTED), 08 (done, COMPACTED), 09 (done), 10 (partial → split 10b), 10b (partial → continuation done), 11 (done), 12 (done, COMPACTED), 13 (partial → split 13b/13c), 13b (done, COMPACTED), 13c (done, COMPACTED), 16 (done, COMPACTED), 17 (done, COMPACTED), 18 (partial → continuation done, COMPACTED), 19 (done, COMPACTED twice), 20 (partial → continuation done), 21 (done), 22 (done, docs only), 24 (done); also 2a cause + gap re-date, m1/80 (1M-tests behind `heavy-1m` feature), housekeeping/01 context-diet (done, two legs) |
| Conductor (not a build) | user-37 PNG shrink, keys collision check, filing/numbering, ticket-24 discovery routing |

**Hard-stop overruns:** the user decided (2026-09-17) not to move overrunning Sonnet tickets to Opus — the work finished correctly — but to log the harness's own tool-call/token figures on every line rather than a builder's self-report. The log records repeated cases where a builder's self-reported call count understated the harness figure (06b, 09, 11, 12, 18, 20, 21, and round-1d's fix, the worst at self-reported "~90+" against a harness 286). No escalations were made for this reason; it is a logging-accuracy finding, not a model-choice one.

**Compacted:** 02 (self-described only, ticket finished), 03 (no compaction despite being over the stop), 04, 05, 06b, 07, 08, 12, 13, 13b, 13c, 16, 17, 18, 19 (twice), housekeeping/01 (no compaction in its continuation), and review round 1a, 1d.

### Review and fix rounds

- **Review round 1** — Fable. **1 Critical, 11 Major, 16 Minor.** C1: Mistake Radar cost scaled with ref count, per frame and per update; heavy tier and release perf both stalled and were killed. Promise judged to hold on fixtures/small repos once M1–M3 fixed; C1 blocked the milestone.
- **Fix round 1a** — Opus. Fixed C1 at the cause (ref-findings cache + bounded force-push probes, 32/session) and M3, partly covered M9; found a pre-existing heap-slope regression (76 B/commit vs the 50 B bar) not caused by this fix, filed as `housekeeping/02`.
- **`housekeeping/01` context-diet** — Sonnet. Not a review fix; cut DESIGN.md/KNOWN_GAPS/PROGRESS.md/CLAUDE.md size (context-cost work), between fix rounds.
- **Fix round 1b** — Sonnet. Fixed M1 (`W` word-tint did nothing) and M2 (diff body flattened to one colour when unfocused) at the cause; added M9's remaining frame tests.
- **`housekeeping/02` heap slope** — Opus. Fixed the round-1a-found regression at the cause (`RadarContext.default_chain` `HashSet<Oid>` → `HandleSet`, restoring ADR-0026): slope 69.9 → 31.5 B/commit; filed two new out-of-scope findings (`map.rs`'s own `HashSet<Oid>`, and the frame gate asserting on max rather than p99) rather than fixing them.
- **Fix round 1c** — Opus. Fixed M4 (working-tree listing read plumbing, not porcelain, to avoid the index write) and M6 (narrowed evil-merge false positives to files both sides actually changed) at the cause.
- **Fix round 1d** — Sonnet, COMPACTED. Synced ARCHITECTURE.md, split 8 over-400-line files across three crates, fixed Minor 15's stale caption, and found/fixed a real regression in the `update/` purity guard (its directory scan had gone non-recursive after an earlier split, so it stopped checking `update/detail/*.rs`).
- **Review round 2** — Fable. **0 Critical, 3 Major, 17 Minor. Promise holds.** **Reverses round 1c's M4 fix and that round's own report of it**: `git diff --no-optional-locks` still rewrites `.git/index` on this git (only `git status` honours the optional-locks switch), verified in throwaway repos — the "costs the user nothing" claim was wrong. Two further Majors: M6's rename/modify conflicts still false-positive as evil (the both-sides-touched invariant fails under renames), and the M3 evil-marker fix was never observed — TESTING.md and the gallery caption assert the opposite of what the code now does.
- **Fix round 1e** — Opus. Fixed all three round-2 Majors at the cause, each verified against a throwaway git-2.54 repo before coding: M4 back to `diff-index` plumbing with a stepped raw/numstat join (asserts `.git/index` bytes unchanged, not just a claim about flags); M6 rename-aware sets (`diff-tree -r -M -C --name-status`, both rename endpoints carried into the intersection); M3's TESTING.md/gallery text corrected and a new pane-closed marker test added, though the capture itself stays stale (pyte not importable in this environment).

Five fix rounds total (1a–1e) plus the `housekeeping/01` and `housekeeping/02` tickets, interleaved between them.

## M3–M8

M3: design pass next (planner-fable).
