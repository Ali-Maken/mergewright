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

## M3 Working tree, undo and stash

First milestone run fully under ADR-0029's groups model: 7 seams-first
groups instead of numbered tickets, each sized to one builder's context.

| Role | Model | Detail |
|---|---|---|
| Designed | Fable | M3 design pass: note + 7 group specs, seams-first (01 git-reads → sonnet, 02 ops-model → fable signature, 03 ops-execution → opus, 04 changes-pane → sonnet, 05 commit-editor → opus, 06 undo-and-stash-ui → opus, 07 acceptance → fable); 9 Proposed changes vs DESIGN.md. |
| Specified | Fable | Same design pass wrote the 7 group specs, already numbered 01–07 with `Build with` set. The log does not record a separate conductor filing/numbering step for M3 (unlike M2's unnumbered-specs-then-conductor-numbers flow) — not stated, so not claimed here. |
| Specified (scope/deltas) | Conductor, user-approved | M3 start-stop: user kept 3 scope cuts (hunk-staging in untracked → M7; `i` ignore → M7; commit during merge/rebase → M5); conductor took 6 implementation deltas. Both recorded as ADR-0034. |
| Built | see table below | |
| Reviewed | Fable | One review round per group, groups 01–06 (detail below). Group 07 (acceptance) has no separate review recorded in the log. |

### M3 builds

| Model | Groups/runs |
|---|---|
| Sonnet | 01-git-reads-and-patches (partial, pieces 1–3 of 8, 179 turns) → continuation 1 (done, pieces 4–8, 131 turns); 04-changes-pane (partial, whole-file stage/unstage + Ctrl+A + discard only, 290 turns — escalated to Opus by the conductor as a substantive stall, not a spec change); housekeeping: KNOWN_GAPS split/compress (done) |
| Fable | 02-ops-model (signature ticket, done, 59 turns, no compaction); 02-ops-model review fixes (done, 78 turns); 07-acceptance walk (done, 76 turns, COMPACTED once — 22 scenarios, all green) |
| Opus | 01-git-reads-and-patches review fixes (done, 64 turns); 03-ops-execution (done, 87 turns); 03-ops-execution review fixes (done, 95 turns); 04-changes-pane continuation (**failed** — Opus session rate limit, not a code failure; seam 1 already landed clean, `bc4886a`); 04-changes-pane continuation 2 (done, 168 turns); 04-changes-pane review round (done, 148 turns); 05-commit-editor (done, 306 turns, COMPACTED); 05-commit-editor review fixes (done, 123 turns); 06-undo-and-stash-ui (partial, 269 turns, COMPACTED — journal/undo/redo/drift done, stash overlay not); 06 stash-overlay remainder (done, 163 turns) |
| builder-opus (named) | group 06 review fixes (**partial** — C1, C2 and most majors fixed at cause; M4 and M5's 14-site refactor not fixed, filed as gaps instead of a second round, per one-round-per-group; 187 turns) |
| general-purpose, unpinned (out of band) | naming research (user-requested) — ran on **Opus**, not the Sonnet this class of work is routed to: conductor omitted `model:`. Found and logged same milestone as a "model-routing leak" (127,775 tokens / 38 calls); rule added to CLAUDE.md and the conduct skill to always pass `model:` for unnamed agents. |
| Conductor (not a build) | snapshot cost run (`docs/perf/m3-snapshot.txt`); CLAUDE.md trim + ADR-0035/0036 binding; M3 post-mortem → ADR-0035 |

**Group totals** (harness turn figures, ADR-0029):

- Group 01: 3 runs, 374 turns. `.scratch/m3-working-tree-undo-and-stash/as-built.md` written here — every later group reads it.
- Group 02: 3 runs, 176 turns.
- Group 03: 3 runs, 216 turns.
- Group 04: closed once after the build phase — 4 runs (1 rate-limited), 748 turns, escalated sonnet → opus after run 1 — then again after the review round: **6 runs (1 rate-limited), 941 turns total — the milestone's worst by far**.
- Group 05: 3 runs, 484 turns, 1 compaction.
- Group 06: build phase closed at 3 runs, 432 turns, 1 compaction. The review-fix round (builder-opus, partial) has no further "closed" line in the log — remainder filed as gaps rather than a second round.
- Group 07 (acceptance): 1 run, 76 turns, 1 compaction; not tallied as a "group closed" line.

### Review and fix rounds

- **Group 01** — Fable: 1 Critical / 5 Major / 9 Minor. C1 + all majors fixed at cause by Opus; minors untriaged.
- **Group 02** — Fable: 2 Critical / 7 Major / 8 Minor. Both criticals lost work (aborted no-op counted as in force; undo of an empty stash push dropped the previous stash). Fixed by Fable: C1, C2, M1–M4, M6, M7; M5 deferred to group 03; 6 minors untriaged.
- **Group 03** — Fable: 1 Critical / 6 Major / 11 Minor. C1: after-commits hung off no ref, so `gc` could prune an entry's backup and undo failed weeks later — verified by prune in a throwaway repo. Fixed by Opus at cause (C1 + M1–M6 + 11 minors); same bug found and fixed in `recover_interrupted`, which the review had missed.
- **Group 04** — Fable: 1 Critical / 9 Major / 9 Minor. C1: scrolling hid the diff cursor under the pinned `@@` heading, so keys acted on an invisible row. **Four acceptance ticks were false** (on top of two the builder self-corrected) — logged as the honesty rule failing twice in one group. Fixed by Opus: C1 + M1–M8 at cause; `operating` now cleared on job failure/cancel; 4 of 5 over-ticked lines re-earned with tests named inline, the 5th ticked with its deviation written into the line.
- **Group 05** — Fable: 0 Critical / 8 Major / 14 Minor; ticks judged honest (all 39 named tests exist). Top findings: hook-rejection text had nowhere to draw while the editor was open; `e` opened `$EDITOR` on a porcelain-relative path with no `current_dir`. Fixed by Opus: all 8 majors at cause, including the **index.lock verdict** — measured true via a C harness sweeping kill timing across a real 20k-file `git commit` (SIGKILL stranded 43/267 mid-flight locks; SIGTERM 6/288); `exec::kill` now SIGTERM → 2s grace → SIGKILL.
- **Group 06** — Fable: 2 Critical / 8 Major / 13 Minor. C1: `is_undo_key` fired before the stash-message `edit` field consumed the keystroke, so typing "update" in a stash message triggered a journaled undo. C2: a 4-arg call to `badges` in `tests/it/frames.rs` meant the heavy tier and release perf never compiled for this group. Fixed by builder-opus (partial): C1, C2 and most majors at cause; M4 and M5's 14-site refactor filed as gaps, not fixed — no second round, per one-round-per-group.
- **Group 07 acceptance walk** — builder-fable, done: 22 scenarios, all green, exit 0; a deliberately broken expectation observed to fail (exit 1); 18 captures. No separate review round recorded in the log.

**Escalation:** 04-changes-pane, Sonnet → Opus, conductor-driven — 290 turns on whole-file staging alone judged a stall in substance, and hunk/line staging is a git-write path CLAUDE.md routes to Opus regardless. Not a spec change.

**Failed run (infrastructure, not a build):** 04-changes-pane continuation, Opus — session rate limit, not a code failure; already-committed work (seam 1) was clean.

**Compacted:** 05-commit-editor (build), 06-undo-and-stash-ui (build), 07-acceptance walk.

### Attribution corrections (commit titles are wrong; the log is right)

Cause: conductor and a running builder shared one `.git/index` while a helper was mid-run. Fixed going forward by ADR-0036 (no conductor staging while a helper runs). History not rewritten — the corrections below are what AUTHORSHIP.md follows instead of the commit titles.

- **`ace834e`**, titled as a conductor rules-docs commit ("rules: docs follow the context"), also swept up **builder** work left uncommitted in the tree: `crates/graft/src/jobs/undo.rs` (+69), `ARCHITECTURE.md`, and the three `MAP.md` files (`crates/graft`, `crates/graft-core`, `crates/graft-ui`). The log attributes this content to a builder, not the conductor, but does not name which builder run it came from — left unstated here rather than guessed.
- **`6d6098d`**, titled as a builder fix landing in `m3-06-fixes`, also swept in **conductor** work: `scripts/toolchain-bench.sh`. Per the log this file is conductor work, not `builder-opus`'s group 06 review-fixes work that the commit title otherwise describes.

## M4 Remotes

Ran under ADR-0029's groups model: 7 groups, plus fix rounds and a separate
keymap-reconciliation pass.

| Role | Model | Detail |
|---|---|---|
| Designed | Fable (`planner-fable`) | M4 design pass and 7 group specs — the log records the model for this milestone's design/spec work but, unlike M1/M3, carries no separate design-note walkthrough entry. Keymap reconciliation ran as its own `planner-fable` pass (31 turns): `docs/reviews/keymap-reconciliation.md`, 6 drifts beyond the known `d`, worst is `x` (compare-two-commits, live since M2) with no presence in the design doc at all; corrected 3 errors in the conductor's key extraction; M5–M8 allocation survives if M6/M7 are stated modal; awaiting user approval before `02-keymap.md` changes. |
| Specified | Fable (`planner-fable`) | Group specs 01 remote-fixtures, 02 git-remote-and-askpass, 03 ui-credential-prompt-and-hints, 04 ops-remote-operations, 05 remote-keys, 06 credential-prompt-wiring, 07 acceptance-remotes. |
| Built | see table below | |
| Reviewed | Fable | One review round per group, for groups 01, 02, 04, 05, 06, 07 (detail below). Group 03 has no review round recorded in the log. |

### M4 builds

| Model | Groups/runs |
|---|---|
| Opus | 01-remote-fixtures — first dispatch killed by rate limit (wave 1, alongside 02 and keymap reconciliation), no commits/files; redispatched fresh, done (`70c75c3`, ticks `981abc2`, merged `5f6e6fd`, 56 turns, no scout — new code, 3 small files read); signature deviations recorded in the ticket (`LocalRemote::new -> Result<_, CorpusError>`, Oid as 40-hex `String`, no graft-core dependency); touched `Cargo.lock` outside its crate, unavoidable for the new dependency; 6 of 8 ticks each naming its test, first attempt left the ticket unticked and was sent back to tick its own, listener-thread exit left open (filed unverified), docs left for the reviewer · 02-git-remote-and-askpass — first dispatch killed in the same wave-1 rate limit; redispatched fresh, done, 11/11 ticks name tests, merged `807a425`, scout: 1 Sonnet, 102 tool uses/998 s · 04-ops-remote-operations — stopped by rate limit then continued (first run's tool-use count lost to the kill; 30 tool uses on the continuation), 18/18 ticks name tests, one line outside fence (`crates/graft/src/jobs/operate.rs`, `ApplyError::Remote` arm; allowed by ticket gates), merged `46c08b0`, scout: none (diff-driven continuation); builder noted DESIGN §6.4 not changed, awaiting the user's yes on the design note's Proposed change 1 (fetch/push transparent to undo) · 05-remote-keys — done, 198 tool uses/1572 s, COMPACTED once (nothing left over), 14/14 ticks, 2 Sonnet scouts, merged, capture `docs/shots/m4-remote-keys.txt` · 06-credential-prompt-wiring — done, 217 tool uses/1718 s, COMPACTED once (remainder docs only), 14/14 ticks, Sonnet scout, merged, capture `docs/shots/m4-credential-prompt-in-app.txt` · thread-teardown fix for 01+02 Majors — done, 81 tool uses total; first pass used a 20/10 ms polling accept, sent back (rule 2: no sleep/poll loop without user OK); final: blocking accept, stop → wake → join → unlink, failed wake returns `Err`; merged · group 04 review fixes — done, 121 tool uses; first pass kept a poll in askpass, corrected to blocking; merged via side worktree (reviewer held main's index); conductor seam fix: app push-box test + design note lines 68/143 dropped `--force-if-includes` · group 05 review fixes (+ group 06's two) — done, 110 tool uses, Sonnet scout, no compaction; Esc-cancel-for-u/U fix WITHDRAWN after proof it left work half-applied |
| Sonnet | 03-ui-credential-prompt-and-hints — stopped by rate limit then continued (36 tool uses), merged `0ba9203`; main broke afterward (app crate built `FieldView` without new `masked` field → conductor seam fix `d0e7934`, cause, 2 lines); app snapshot then failed (footer hints placed after `u undo` not after `S stash` per spec, pushing `/ search`, `j/k move` off at 80 cols) → sent back to the same builder; fix round, same builder resumed, 49 tool uses total: hints moved after stash per spec, remote hints now fit only at ≥156 cols (visual, for the user); merged. Root cause of both misses: `-p graft-ui` gates never compiled the app crate, so CLAUDE.md gates moved workspace-wide. |
| Fable | 07-acceptance-remotes — done, 118 tool uses/2303 s, COMPACTED once (nothing left), Sonnet scout, 6/7 ticks (the open one is a KNOWN_GAPS filing the conductor reserved), merged; walk `cargo run -p graft --example m4_walk` 19/19, slowest 1.4 s; 16 captures `docs/shots/m4-0{1..8}-*`, 71 KB total (cap 200 KB/file) · acceptance fix round — killed by session rate limit mid-split, continued by a fresh `builder-fable`, 36 tool uses: `acec55f` (hung pull + hung push cancel scenarios, box bodies and capture files under test, rejection line keeps its keys at 80 cols), `b0a0794` (walk split into `tests/it/m4_walk/{mod,fixture,truth,steps,run,vocabulary,fetch_pull,push,credentials,hangs_and_undo,every_scenario}.rs`, largest 364 lines; `coverage.rs` + `repo_truth.rs` shared; examples compile only what they run); merged |
| Conductor (not a build) | keymap-reconciliation extraction check (corrected by the Fable pass above); accepted 02's `classify_failure` deviation ("unable to access" not `Unreachable`, also covers 403/TLS) as a heuristic inside the ticket, not a scope change, reviewer confirmed sound and tested; Critical fix from review group 07 in two halves — filed 11 KNOWN_GAPS rows, retargeted `notify` M4→M7, closed `gallery --matrix` (`da0c1af`) |

**Failed/killed runs (infrastructure, not build stalls):** wave-1 dispatch — groups 01, 02 (both Opus) and keymap reconciliation (Fable) — killed together by a session rate limit within their opening moves; no commits or files, group 02's empty worktree removed; the log notes parallelism spends quota proportionally faster than serial dispatch, a cost of ADR-0036 the ADR does not state; all three redispatched fresh on the new session. Groups 03 and 04 were separately stopped mid-build by a later user-paused rate limit and resumed from their worktrees. The acceptance fix round was killed by a session rate limit mid-split and continued by a fresh `builder-fable`.

**Compacted:** 05-remote-keys (once, nothing left over), 06-credential-prompt-wiring (once, remainder docs only), 07-acceptance-remotes (once, nothing left).

### Review and fix rounds

- **Review group 01** — Fable (concurrent with 02/03): 1 Major (listener-thread teardown unproven), 3 Minor. 6/6 ticks held. `e3d666c`, 30 tool uses.
- **Review group 02** — Fable (concurrent with 03/04): 0 Critical, 1 Major (`AskPassServer::shutdown` leaks its listener thread when the wake-connect fails), 5 Minor. 9/9 ticks held. `56d2862`.
- **Thread-teardown fix (01+02 Majors)** — Opus. See builds table.
- **Review group 04** — Fable: 0 Critical, 4 Major, 9 Minor. 17/17 ticks held. `ce30d46`, 28 tool uses. Conductor decisions on two Majors: (1) lease — `--force-if-includes` was a no-op with an explicit expect; `check_push` must compare `expected` with the fetched tracking tip and refuse `LeaseStale`; (3) an up-to-date pull moves nothing → transparent to undo like fetch, never `next_undo`.
- **Group 04 review fixes** — Opus. See builds table.
- **Review group 05** — Fable: 0 Critical, 4 Major, 11 Minor. 13/13 test-named ticks held. `5bc0966`, 26 tool uses. Conductor decision on M3 (Esc cancels u/U in flight, untested): prove it leaves no half-applied undo, or Esc must not cancel u/U — not self-certifiable (checkpoint rule 2); fix round waited for group 06 to free the graft crate.
- **Review group 06** — Fable: 0 Critical, 1 Major (typed password sits in a plain TextField String, cloned per event, freed unzeroed — the ticket asked for TextField, so a spec limitation), 9 Minor. 26/26 ticks held. `e1e8ade`, 25 tool uses. Confirms: no password in argv, env, journal, log, capture, panic or Debug; the HTTP round trip genuinely proves git got it. Conductor sent 06's two cheap fixes (Esc cancel order, askpass doc overclaim) to the running group-05-fix builder; the memory-remnant Major went to the user at the checkpoint.
- **Group 05 review fixes (+ 06's two)** — Opus. See builds table. Esc-cancel-for-u/U WITHDRAWN: `restore::execute` checks its stop token once, before step 1, so a mid-flight cancel leaves earlier steps applied; u/U now `Stoppable::No`, line "undo cannot be stopped part-way — finishing…".
- **Review group 07 acceptance** — Fable: **1 Critical**, 5 Major, 9 Minor. 6/7 ticks held — the false tick was the conductor's: tick 6 claimed KNOWN_GAPS rows filed when 3 of ~12 were. `30b7c9d`, 26 tool uses. Fixed in two halves: conductor filed 11 rows + retargeted `notify` M4→M7 + closed `gallery --matrix` (`da0c1af`); builder-fable fixed every overclaim.
- **Acceptance fix round** — Fable. See builds table.

Group 03 has no review round recorded in the log.

**Milestone review perf tier:** `cargo test --release --features heavy-tests` — 1855 passed, 0 failed, exit 0 (`docs/perf/m4.txt`). Frame budgets hold under `--release`.

## M5–M8

Not yet in the session log.
