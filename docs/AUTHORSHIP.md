# Authorship

Which model designed, specified, built and reviewed each part of Graft. **Generated from the session logs under `docs/log/<milestone>.md`** (the conductor regenerates it at every milestone checkpoint). Nothing here goes beyond what the log records; where the log is silent, this file says so.

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

## M5 History, Ghost Preview, merge, rebase, reset

Started under ADR-0029's groups model (11 groups across two `planner-fable`
design passes). Group 11 (acceptance) did not run to a single finish: it split,
after the eye-check and review rounds it kept surfacing, into a long chain of
numbered defect/fix tickets (14–43) closed by an acceptance-walk split (37, 38,
39, plus 32 and 43) — the same "split along the ticket's own handoff seams"
pattern M1 and M4 used for an oversized acceptance ticket. The walk finished at
49/49, `b3b176a` (git log: "g39 merged - the walk is whole, 49 of 49").

| Role | Model | Detail |
|---|---|---|
| Designed | Fable (`planner-fable`) | Design pass 1 of 2: note + 7 group specs (11 groups planned, 4 left for pass 2). Design pass 2 of 2, after two scouts: 4 more specs (app-preview-and-keys, app-conflict-interim, app-drag-drop-and-stash-branch, acceptance-history). `planner-fable`'s own definition carries no Agent tool, so it could not dispatch the ADR-0035 scout itself — logged as a process gap and surfaced to the user, not silently worked around. |
| Specified | Fable (`planner-fable`); scouts dispatched by conductor | Conductor ran the two scouts planner-fable couldn't (Explore, sonnet: app scout, ops/graph scout) between the two design passes; both scouts were read-only and could not write their own files, so the conductor wrote `.scratch/m5-history-and-ghost-preview/scout-{app,ops}.md` from their reports. Conductor then filed and sequenced groups 01–11 and set `Build with` from the planner's post-scout recommendations: Sonnet 01–04, Opus 05, 06, 08, 09, 10, Fable 07 (signature) and 11 (acceptance). |
| Specified (defect/fix tickets 14–43) | Conductor | Filed off review findings and the user's own by-eye checks, not further `planner-fable` passes — the M5 counterpart to M2/M4's conductor-filed ranges. Includes the acceptance-walk split into 37 (refs/names), 38 (previews/conflicts), 39 (undo/push/secrets), each pre-assigned a model and crate scope by the conductor before dispatch. |
| Built | see table below | |
| Reviewed | Fable (reviewer agent) | One round per group for 01–09 and the user's own 12/13 (context-menu overlay), plus two combined catch-up rounds: tickets 14–22 and 15 together (nine tickets the review step had quietly skipped — a third of the milestone, caught by the conductor's own audit, not by process); and tickets 10, 30 and 31 together. The defect-ticket chain past 13 (16 through 43) ran its fix/review cycle per-ticket rather than as further "group" reviews; several of those tickets state their own findings were reproduced/verified before being trusted (rule 7 in practice). |

### M5 builds

| Model | Groups/tickets |
|---|---|
| Sonnet | g01 secret-field (done, merged `fe2c000`, 7/7 ticks each naming its test, turn count not recovered — conductor compacted before logging); g02 git-history-reads (done, merged `2826aa5`, turn count not recovered, same reason); g02 fix round (done, merged `4415c5e`, 60 tool uses, all 5 fixes at cause); g03 hint bar (done, COMPACTED once mid-ticket, merged `ee855e4`, 315 tool uses); g04 ui-ghost-banner-conflict — first dispatch killed by a session rate limit mid-flight (nothing committed, nothing lost), resumed and closed **partial** (`blocked: needs graft` at `fad9f9b`, 84 tool uses) — its gates were run `-p graft-ui` only, which is exactly what hid the crate-seam problem it left for its successor; ticket 27 short-form hint table (done, 87 turns, merged); ticket 30 border-glyph invariant (done, COMPACTED once, 231 tool uses, merged) — two point-fixes-that-caused-the-next replaced by one rule; ticket 36 review's-8-minors group (done, merged `b4840ca`); ticket 37 acceptance-walk refs/names (done, merged, 7/7 scenarios `ok`, no findings) |
| Fable | g07 ops-history-operations, **pieces 1–2 only** (`OperationKind`, the operation shapes; committed `667ad8e`) — ran out of credits mid-build; remainder substituted to Opus (detail below). g11 acceptance — did not run to completion; split into the defect-ticket chain (14–43) instead of one Fable run to the end. |
| Opus | g04 seam+pulse fix (done, merged `190a4ed`, 123 tool uses — restored the pulse ADR-0038 §2 needs on both halves after the review round's fix bought length-freedom by deleting it); g05 refresh race (done, merged `6c8f7c1`, 137 tool uses, scout used, 7 tests run red-before-green, no sleep/poll/retry); g05 refresh-race fix (folded into g08, see below); g06 undo-cancel-rollback (done, merged `9ff1d28`, 83 tool uses, scout used) and its fix round (done, merged, 84 tool uses — C1 fixed with the mutation named correctly the second time); g07 pieces 3–9 (**Fable-substitution**, detail below; compacted once; `done` in its own crates, `blocked: needs graft` — not merged standalone) then g07 seam+leftovers (done, merged `7180f07`, 90 tool uses, scout used); g07 fix round dispatched to **`builder-opus-xhigh`** (escalation, detail below) — partial, `blocked: needs graft`, 255 tool uses, all 3 Criticals fixed at cause; g08 app-preview-and-keys, pieces 1–5 (partial, COMPACTED once, 265 tool uses, scout used), continuation 1, pieces 7–10 (partial, 160 tool uses, `blocked: needs graft-ui` on piece 6), piece 6 (done, 91 tool uses, scout used), piece 6 label ruling resumed (done, 16 tool uses); g09 app-conflict-interim, pieces 1–4 (partial, COMPACTED, merged `4c000e3`); ticket 14 journal live-vs-interrupted, group 09's seam A (done); ticket 15 undo-rules+push (done, 191 turns/57 min — the mutation-check-cost ticket); g10 name-prompt+drag (done, COMPACTED **twice**, 307 tool uses); g12 context-menu overlay, the user's own filed ticket (done, scout used, 91 tool uses); g13 menu-keys-and-overlay-clears (done); ticket 16 banner-visible-and-honest (done, **resumed after a stream-watchdog stall**, detail below); ticket 17 preview-shows-your-screen (done, COMPACTED once, 12 mutations A–L); ticket 18 right-click-wrong-row (done); ticket 19 pane-boundaries-as-a-class (done — built from the user's own test design); ticket 20 say-it-once-name-the-outcome (done); ticket 21 unmerged-files-invisible (done); ticket 22 diff-parser-swallows-unknown (done, 63 turns); ticket 23 every-capture-reproducible (done, 117 turns/17 min); ticket 24 the ladder (done across **three separate Opus passes**, 75+59 turns after the first — first built the rule as written and reverted it on measurement, second built a corrected rule and stopped at the `graft` fence, third closed the seam); ticket 25 review's-five-majors+clock-seam (done, COMPACTED once, 254 turns/46 min); ticket 26 (done, 77 turns); ticket 28 (done, 212 turns); ticket 29 (done, 131 turns); ticket 31 (done, no compaction, 215 tool uses); ticket 33 (done, COMPACTED once); tickets 34+35 combined (done, one merge `e1cacf2`); ticket 38 previews/conflicts — **built and deliberately not merged**: 7 of 8 scenarios `ok`, 1 left failing as a filed finding, same handling as ticket 11→32; ticket 39 undo/push/secrets (done, 287 turns, merged `b3b176a` — the milestone's closing walk, 49/49); ticket 40 stale-captures class (done, 8/8 ticks); ticket 41 home-path/scratch-path redaction, two rounds (done); ticket 42 dead-key-overlay-bar (done); ticket 43 rebase-instead fix, merged together with 38 (done) |
| **Not recorded** | Ticket 32 — referenced repeatedly as merged and as having closed ticket 11's five deliberately-failing scenarios ("ticket 11's five deliberate failures are genuinely closed by ticket 32"; "ticket 32 made the preview tell the truth about files"), but the log carries no dedicated build entry naming its builder, model or merge hash. Left unstated here rather than guessed from the surrounding Opus run of tickets. |
| Conductor (not a build) | Eye-check before forwarding captures to the user (found the empty-parens `Uncommitted changes ()` defect → ticket 21); review triage with no helper (tickets 30/31 review close-out; ticket 36's Minors/Questions sort); acceptance-walk split into 37/38/39; the capability ledger regeneration and its mechanical re-check (six agents reading the tickets, ~137 capabilities, `docs/capabilities/`) |

**Fable substitution, group 07 (`ops-history-operations`, signature ticket).**
Fable ran out of credits mid-build; the conductor routed the remainder to Opus
rather than stall groups 08–10, which all wait on 07. Recorded as a
substitution, not a silent swap: pieces 1–2 are Fable's (`667ad8e`); pieces
3–9 — including both items the ticket was routed to Fable for, the journal
shape of a conflicted operation and `PreviewPlan` — are Opus's. The group 07
review later found a Critical (C1: a resolved conflict is unreachable by `u`)
and said plainly the structural question **"wanted Fable"** — the ticket's own
`Build with` line was right and the conductor's substitution was the wrong
call, made to keep the milestone moving. The log records this as a cost paid
on the milestone's central promise, found by review rather than by the user.

**Escalation to `builder-opus-xhigh`, group 07 fix round.** CLAUDE.md reserves
that agent for undo, conflicts, interactive rebase and the layout engine; C1
(undo-across-a-conflict) is exactly that class, so the fix round was dispatched
there rather than to plain `builder-opus`. It also pushed back on the
reviewer's own suggested fix (keying the live span off `ORIG_HEAD`) with
evidence, and won.

**Stalled/killed runs, infrastructure not code (recorded distinctly from
`failed` builds):**
- A session rate limit killed three in-flight agents at once (g01 fix round,
  g04 build, g07 build). None had committed; all three worktrees held complete
  uncommitted diffs, so nothing was lost; all three resumed in the same
  worktrees, told explicitly to verify the inherited diff rather than assume it
  finished.
- Ticket 16's builder and the groups-12/13 reviewer were both killed by the
  stream watchdog (600 s no progress) within moments of each other. The
  builder's uncommitted diff survived whole and was treated as unverified, not
  done, by its resumed successor (which found one mutation — C — had been
  inherited green with no test covering it). The reviewer's findings did **not**
  survive (nothing on disk, nothing committed) and were explicitly not carried
  forward; re-dispatched fresh with a process change (write findings
  incrementally, not once at the end).

**Compacted:** g03 hint bar (once, mid-ticket), g08 pieces 1–5 (once), g09
pieces 1–4, g10 (**twice**), g07 pieces 3–9 (once), g33 (once, near the end),
tickets 17, 23, 25 (each once).

### Review rounds

One round per group unless noted; `docs/reviews/m5-*.md` under each hash.

- **g01 secret-field** — 1 Critical (in-place `Zeroizing<String>` reallocs
  leaked unwiped prefixes on any password over 8 characters), 4 Major, 11
  Minor. 7/7 ticks held as worded, though 2 held against the tick's own text
  while failing the user's binding condition.
- **g02 git-history-reads** — 1 Critical (`merge-tree --write-tree` does write
  objects, contradicting the ticket's "nothing writes" claim), 4 Major, 15
  Minor. 4 ticks checked, 3 held, **1 false** (cited a nonexistent test).
- **g04 ui-ghost-banner-conflict** — 1 Critical (`preview_box.rs` always
  reported "too small" at its own configured height, hidden by padded test
  fixtures and a capture pipeline that filters blank rows), 7 Major, 12 Minor.
  11 ticks total, 10 checked, **only 7 held** — three false ticks in one group.
- **g03 hint bar** — 2 Critical (an unreachable `ChangesPane` branch meant
  `space stage`/`d discard` never appeared at all; a guard test with an
  exemption list that let unbound keys through), 6 Major, 10 Minor. 13 ticks
  checked, all 13 name real tests, 11 hold, 2 true of the code only.
- **g05 refresh race** — 1 Critical (a test built to prove a race provably
  cannot fail — it passed even with the race reverted), 4 Major, 3 Minor.
  4 ticks / 11 named claims checked, 10 held.
- **g06 undo-cancel-rollback** — 1 Critical (a mutation-check claim was half
  true: one mutation genuinely caught, the other's catching test passed for an
  unrelated reason, caught by the reviewer re-running it), 5 Major, 6 Minor. 3
  boxes held at box level; of box 1's 7 named tests, 4 held, 1 half, 1 flaky, 1
  Critical.
- **g07 ops-history-operations** — 3 Critical (C1: a journaled conflict
  resolution is unreachable by `u`; C2: `plan_revert_to` re-creates
  deliberately-undone work; C3: a "13 kinds" mutation-check claim was actually
  11), 11 Major, 15 Minor. 5 ticks checked, 4 held, 1 partly. Verified both of
  the builder's own spec corrections against real git 2.54.
- **g08 app-preview-and-keys** — 0 Critical as filed, **1 Major escalated to
  Critical by the conductor** (the preview drew the slice's first row rather
  than the viewport's, invisible in every test and capture because all of them
  have `top == 0`), 6 Major, 11 Minor total. 14/33 ticks checked, 13 held, 1
  held by inspection only.
- **g09 app-conflict-interim** — 1 Critical (a false capture tick: at 80x24
  the conflict banner is entirely absent, and below 100 columns generally a
  **modal** overlay travels off-screen with the tab layout while still holding
  the keyboard), 5 Major, 9 Minor. 10 ticks checked, 9 held, 1 false. Folded
  into ticket 16 rather than filed separately.
- **Review of tickets 14–22 and 15 (combined, catch-up round)** — 0 Critical,
  6 Major, 9 Minor. 22 acceptance ticks spot-checked, **all 22 held**. Found a
  fourth instance of the milestone's "two things that must agree" fault
  class, vertical rather than horizontal (a banner row reserved before layout
  ran); a rule-3 violation shipped hours earlier (`.ok()?` on a failed remote
  read silently downgrading a push-aware undo to a plain one); one tick
  narrowed-but-not-false; ran the release perf tier early as the milestone
  review run (13/13 budgets, p99 7.7–9.1 ms).
- **Review of tickets 10, 30, 31 (combined)** — 0 Critical, 6 Major, 8 Minor, 2
  Questions. T10 27/27 ticks exist, T31 6/6, **T30 none — all seven boxes left
  blank** (nothing false shipped, but the tick list — the artefact a reviewer
  spot-checks — was never filled in; added as a pre-merge check to the conduct
  skill). One Major escalated to Critical scope by the conductor's own account:
  badge-geometry hit-testing and drawing used two different icon/lane-cap
  sources, so a drag lands on the wrong badge under Nerd icons — the icon set
  this user actually runs.
- **Groups 12/13 (context menu + overlay clears)** — 1 Critical, 3 Major, 5
  Minor, re-run after the lost attempt above. 12 ticks checked, 11 held, 1
  false. Critical escalated by the conductor from the review's own Major:
  right-click opens the menu on the clicked row without moving the cursor, so
  a fall-through verb (`y`, silently) can act on the wrong commit with no
  feedback at all.

### Escalations (severity, not model)

Beyond the two model-routing escalations above (Fable→Opus substitution on g07,
Opus→`builder-opus-xhigh` on g07's fix round), the conductor escalated a
reviewer-scored **Major to Critical** three times in this milestone, each time
reasoning from the milestone's own promise ("you see the result before you
confirm") rather than diff size: g08's preview-window bug (drawn from the wrong
row of the slice), the tickets-10/30/31 review's badge-geometry bug (hit-test
and draw used two different icon/lane sources), and the groups-12/13 review's
right-click-wrong-row bug (a silent verb acting on a commit the user didn't
click). Each escalation states the reviewer's original score and the reasoning
for raising it, rather than silently overriding it.
