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
| Sonnet | g01 secret-field (done, merged `fe2c000`, 7/7 ticks each naming its test, turn count not recovered — conductor compacted before logging); g02 git-history-reads (done, merged `2826aa5`, turn count not recovered, same reason); g02 fix round (done, merged `4415c5e`, 60 tool uses, all 5 fixes at cause); g03 hint bar (done, COMPACTED once mid-ticket, merged `ee855e4`, 315 tool uses); g04 ui-ghost-banner-conflict — first dispatch killed by a session rate limit mid-flight (nothing committed, nothing lost), resumed and closed **partial** (`blocked: needs graft` at `fad9f9b`, 84 tool uses) — its gates were run `-p graft-ui` only, which is exactly what hid the crate-seam problem it left for its successor; ticket 27 short-form hint table (done, 87 turns, merged); ticket 30 border-glyph invariant (done, COMPACTED once, 231 tool uses, merged) — two point-fixes-that-caused-the-next replaced by one rule; ticket 36 review's-8-minors group (done, merged `b4840ca`); ticket 37 acceptance-walk refs/names (done, merged, 7/7 scenarios `ok`, no findings); **ticket 44 the selection token** (done, merged `1502d9d`, filed off the user's own by-eye report an hour after M5's build closed — the cursor row's fill went 1.21:1 to 3.04:1 in the dark theme; ticked one box `[~]` rather than `[x]` because the conductor's ticket set two contrast floors that are arithmetically incompatible in that theme, and it derived the empty window and said so instead of silently choosing one) |
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

## M6 Conflict resolver

Groups model (ADR-0029): 8 groups (01–08; 08 added mid-milestone), fix rounds,
`planner-fable` rulings. Start `47b55c2`; final fixes merged `902a742` at 2674
passed, 1 skipped (conductor's own gate run). The M6 stop came at m6:152, followed by an
**amendment stretch** (tickets 09–46, start `2184642`, m6:165), covered below. Its last gate line is
3047 passed, 1 skipped (m6:318), and 46 merged at `981fc58e` (m6:320). A second stretch
(look-and-finish, usability fixes, stash put-back; tickets 47–86, m6:321–487) follows it, below; its last
gate line is 3486 passed, 1 skipped (m6:480, m6:482), and 85 merged at `513adf7c` (m6:487). Tool-use counts are the harness's,
as logged in `docs/log/m6.md` (cited `m6:<line>`); "no count" means the log gives none.

**Model changes during M6** (both recorded, not silent): resumed 2026-09-22 with
the conductor on Opus 5.5 and the Fable tier running on Opus at high effort
(`19ac7fe`, m6:79); same day the user switched Sonnet → Opus low everywhere —
`builder-sonnet` keeps its name as a tier label, and new `scout`/`researcher`
definitions (Opus low) close the "scouts cannot write" gap (m6:85). So
`builder-fable` after m6:79 is Opus high, and `builder-sonnet` after m6:85 is Opus low.

| Role | Model | Detail |
|---|---|---|
| Designed | Fable (`planner-fable`) | Note + 7 group specs + ADR-0042, `3d90471`, turn count logged with M5's close (m6:9). Group 08 spec later, 20 tool uses (m6:35). |
| Specified | `planner-fable`; conductor | Rulings passes: g05 box 74/no-base notice, 12 tool uses (m6:98); g06 relaunch/fetch, 4 (m6:105); g06 box 550 re-ruling, 5 (m6:117); g06 repush question, 10 (m6:121, count m6:141); g07 F3/F5 rulings, 10 (m6:124, m6:141); g07 design rulings 07b, 13 (m6:134, m6:141). Conductor widened g02's crate set from the scout (m6:11), set g04 → Opus (m6:45), g05 Sonnet / g06 Opus (m6:80–81). |
| Scouts | Explore, sonnet (read-only; conductor wrote their files) | ui 47 (m6:10); app g04 50 (m6:42); app g05 54 (m6:80); app g06 55 (m6:81). |
| Built | see table below | |
| Reviewed | Fable, then Opus high (reviewer) | g01 (m6:22), g02+g03 (m6:47), g05 (m6:102), g06 (m6:110), g07 + milestone runs (m6:135). |
| Researcher (Opus low) | — | history audit (m6:86), false-tick sort (m6:87), KNOWN_GAPS ×2 (m6:114, m6:137; m6-07 gaps 6, m6:141), AUTHORSHIP regen 7 (m6:141), gap triage of 67 rows (m6:143), g04/g08 minors 14 rows (m6:152). |

### M6 builds

| Group | Agent (model) | Runs, as logged | Tool uses |
|---|---|---|---|
| 01 git-conflict-model | builder-fable (signature) | build, **COMPACTED** once, `partial` → `blocked: needs graft, graft-ops` (m6:12); review fix round, builder-opus, merged `421b4ec`, no scout (m6:31) | 136 + 113 = **249** |
| 02 ui-conflict-resolver | builder-sonnet | build + continuation 1 (m6:15); continuation 2 (m6:18); continuation 3, golden files (m6:19); review fix round (m6:52) | 155 + 32 + ? + ? + 50 = **237** (cont. 2, 3 no count) |
| 03 ops-conflict-operations | builder-opus | build `partial`, no scout (m6:26); continuation, merged (m6:39) | ? + 32 = **32** (build no count) |
| 08 ops-undo-an-unmerged-index | builder-opus | piece A, **COMPACTED** once, `partial`, `blocked: needs graft` (m6:54); Scope 5(c) continuation, `done` (m6:60); seam closed by conductor (m6:65); review fix M1+M3, builder-opus-xhigh, `partial`, **COMPACTED**, scout used, no count (m6:148); continuation, builder-opus (medium), merged `31fbaaf` (m6:150) | 241 + 106 + ? + 54 = **401** (xhigh fix no count; m6:152 gives 541) |
| 04 app-conflict-resolver | builder-opus | build, **COMPACTED twice**, `partial`, scout used (m6:67); continuation 1 (medium), `done`, merged `421593c` (m6:82); review fix M2 (medium), merged `1cda3fb` (m6:146) | 363 + 42 + 34 = **439** |
| 05 app-resolver-whole-files-and-base | builder-sonnet (**Sonnet**, dispatched before the switch) | build, **COMPACTED** once, reported `done` but all 9 boxes blank — not merged (m6:88–89); continuation 1, Opus low, `partial` 5/9 (m6:92); continuation 2, Opus low, `partial` 8/9 (m6:94); fix round after ruling, Opus low, `done` (m6:100), merged `e6c92cd` (m6:101); review fix round, **escalated to builder-opus** (medium) because Major 3 is two concurrent git writes, COMPACTED (reply vs ticket disagree; logged compacted) (m6:109, m6:111) | 271 + 34 + 57 + 26 + 138 = **526** |
| 06 answers-beat-rereads | builder-opus (medium) | build `partial` 5/8, no compaction (m6:103); continuation 1, `done` (m6:107), merged `e2b6f56`; review fix round, no compaction (m6:115); (iii) round after re-ruling (m6:118), merged `3662e79` | 183 + 32 + 71 + 27 = **313** |
| 07 acceptance | builder-fable (Opus high) | build, **COMPACTED ×2**, `partial` 4/6 (m6:122); F1/F4/F6/F7/F8 fixes, builder-opus medium (m6:125); F2 fix, **builder-opus-xhigh** (undo) (m6:123, m6:127); continuation 1 `partial` 5/6 (m6:129); continuation 2 `done` 6/6 (m6:131), merged `c309b45` | 388 + 90 + 57 + 156 + 143 = **834** (fixes and cont. 1 added at m6:141) |
| M6 final fix round (g07 review Majors + rulings 07b) | builder-opus (medium) | `done`, **COMPACTED** once, no scout (m6:138), merged `902a742` | **261** |

Builder total to the final fix round: **3204** as corrected at m6:141 (first pass 2958 missed five counts). The log's
own tally at the M6 stop (m6:152) is ≈ **3432**, with the g04/g08 review fixes included. Still not counted: g02 continuations 2 and 3,
the g03 build and the g08 xhigh review fix.

**Compacted:** g01 build (once), g04 build (twice), g05 build (once), g05
review fix round (once, disputed by its own ticket), g07 build (twice), g08
piece A (once), final fix round (once), g08 review fix (once, m6:148).

**Escalations (model):** g05 review fix round Sonnet-tier → builder-opus
(m6:109); g07 F2 → builder-opus-xhigh, undo class (m6:123).

**Conductor's own errors, as logged:** g02 line-number fix ordered wrong
(m6:50); g05 brief said hints "already draw" when the scout said `planned`
(m6:90); planner's `checkout --merge` reading wrong until the conductor ran it (m6:36).

### Review rounds

- **g01** — reviewer (Fable), 27 tool uses: 1 Critical (overstated tick), 5 Major, 11 Minor, 1 Question; 14 checked / 13 held (m6:22).
- **g02 + g03** — reviewer (Fable), 45 tool uses: g02 1 Critical, 5 Major, 4 Minor, 13 checked / 11 held, 1 false; g03 0 Critical, 2 Major, 4 Minor, 16/16 held (m6:47).
- **g05** — reviewer (Opus high), 56 tool uses: 0 Critical, 6 Major, 9 Minor; 9/9 held (m6:102).
- **g06** — reviewer (Opus high), 84 tool uses: 0 Critical, 2 Major, 9 Minor; 8/8 held (m6:110).
- **g07** — reviewer (Opus high), 97 tool uses (m6:141): 0 Critical, 5 Major, 8 Minor; 9/9 held. Milestone runs: heavy 2701/0, release 13/13 frame p99 ≤ 8.9 ms, fuzz 4×60 s 0 crashes (m6:135).
- **g04 + g08** (missed round, found by the AUTHORSHIP pass, m6:142) — reviewer (Opus high), no count: g04 0 Critical, 1 Major, 5 Minor; g08 0 Critical, 2 Major, 6 Minor; 17/17 + 18/18 held. g08 M1 can lose work. It was fixed by the xhigh + opus rounds above (m6:144–150).

### M6 amendment (tickets 09–46)

This stretch came after the M6 stop. The trigger was the user's stuck-after-merge report and the feature pipeline added to CLAUDE.md (`f65c14f`, m6:153). Tool-use counts come from the harness as logged; "no count" means the log gives none.

| Role | Model | Detail |
|---|---|---|
| Prior art / research | researcher (Opus low) | UX principles `6b9c806`; conflict-resolution prior art, 55 (`84a745d`); layout audit, 14 (m6:153, m6:160–161). |
| Prototype | builder-sonnet (Opus low) | v1 14 (`e7cdd0f`), v2 15 (`1f74ec8`), finish summary 9 (`87cd71b`) (m6:158–162). The conductor's hotfix is `c4ac7c6`. |
| Designed | planner-fable (Opus high) | Amendment 39 (m6:157) and amendment 2, 77: ADR-0043 supersedes 0042, P1–P9 (m6:163). The user approved P1–P5. Amendment 3 (J11/J12) came with the 28 spec (m6:222). |
| Specified | conductor; planner-fable (Opus high) | The conductor filed 09–21 from the `fix-*` specs (m6:164) and later 22–46. Spec-only passes, mostly after scout contradictions: 10 17, 11+12 plus review-09 Q1/Q2 53 total, 13 22, 25 17+3, 14 20, 15 24, 28 31+5, 16 16, 19 25, 17 26, 31 11, 20 32, 39 18, 40 28, 41 35+11. |
| Scouts | scout (Opus low) | For 09, 10, 11+12, 13, 14, 25, 15+18, 28, 16, 19, 17, 31, 20, plus the stuck-start repro (m6:155). All have counts except 16's. |
| Doc edits, gap rows | researcher (Opus low) | Keymap and §1.5 rows, design-note edits, and gap rows after each review (their definition forbids editing KNOWN_GAPS, so the conductor appended the drafts, m6:183). |
| Reviewed | reviewer (Opus high) | 15 reviews, listed below. |

| Ticket | Agent (model) | As logged | Tool uses |
|---|---|---|---|
| 09 | builder-opus (medium) | `done`, merged `08a3656`, scout used (m6:171) | 116 |
| 10 | builder-sonnet (Opus low) | `done`, scout used (m6:179) | 136 |
| 11 | builder-sonnet (Opus low) | `done`, merged `c6d26ab` (m6:181) | 45 |
| 12 | builder-opus (medium) | `done`, **COMPACTED**, merged `dd2a758` (m6:185) | 304 |
| 13 | builder-opus (medium) | `done`, **COMPACTED**, merged `b0e08fd` (m6:199) | 198 |
| 14 | builder-opus (medium) | `done`, **COMPACTED**, merged `7bef4b7a` (m6:209) | 167 |
| 15 | builder-opus (medium) | `done`, merged `06788a5` (m6:223) | 113 |
| 16 | builder-sonnet (Opus low) | `done`, merged (m6:249) | 93 |
| 17 | builder-sonnet (Opus low) | `partial`, then resumed `done`, merged `20bcd77` (m6:257–258) | 79 + 5 |
| 18 | builder-sonnet (Opus low) | `partial`, then continuation `done`, merged `0a1cb92` (m6:224, m6:228) | 40 + 57 |
| 19 | builder-opus (medium) | `partial` 0/21, **COMPACTED**, split into 34 and 35 (m6:267) | 141 |
| 20 | builder-fable (Opus high) | Part A `blocked: needs graft`, **COMPACTED**; follow-up; Part B `partial` (m6:288–291) | 180 + 2 + 86 |
| 21 | builder-fable (Opus high) | part 1 `partial`, **COMPACTED**, split into 42, 43 and 44 (m6:309) | 197 |
| 22 | builder-opus (medium) | `done` (m6:178) | 44 |
| 23 | builder-opus (medium) | `done`, merged `4395547` (m6:189) | 144 |
| 24 | builder-opus (medium) | `done`, merged `4b5b43a` (m6:197) | 122 |
| 25 | builder-opus (medium) | `done`, **COMPACTED**, merged `798a176` (m6:217) | 269 |
| 26 | builder-opus (medium) | `done`, merged `1d9a769` (m6:207) | 121 |
| 27 | builder-sonnet (Opus low) | `done`, merged `1bb925a` (m6:218) | 57 |
| 28 | builder-opus (medium) | `done`, **COMPACTED**, merged `6429b2a` (m6:234) | 162 |
| 29 | builder-sonnet (Opus low) | `done`, merged `4ec8fd7` (m6:239) | 34 |
| 30 | builder-sonnet (Opus low) | `done`, merged `c08bb8a` (m6:244) | 33 |
| 31 | builder-opus (medium) | `done`, **COMPACTED**, merged `12e8ac2` (m6:284) | no count |
| 32 | builder-sonnet (Opus low) | `partial` by design, merged `4a632c1` (m6:263) | 70 |
| 33 | builder-sonnet (Opus low) | `done`, merged `18b11e1` (m6:265) | 50 |
| 34 | builder-opus (medium) | `done`, **COMPACTED ×2** (m6:269) | no count |
| 35 | builder-opus (medium) | `done`, merged `37045c8` (m6:273) | 93 |
| 36 | builder-sonnet (Opus low) | `done`, merged `ba7d76f` (m6:279) | no count |
| 37 | builder-sonnet (Opus low) | `done`, merged `ce69b5d5` (m6:290) | 41 |
| 38 | builder-opus (medium) | `done`, merged `13886c74` (m6:297) | 81 |
| 39 | builder-sonnet (Opus low) | `partial`, merged `27c133ce` (m6:294) | 47 |
| 40 | builder-sonnet (Opus low) | `done`, merged `0f03dbcb` (m6:299) | 64 |
| 41 | builder-opus (medium) A; builder-sonnet (Opus low) B | A `done`, merged `c01403fd`; B `done`, merged `1bd9217d` (m6:306–307) | 136 + 37 |
| 42 | builder-sonnet (Opus low) | `done`, merged `b1504477` (m6:313) | 23 |
| 43 | builder-fable (Opus high) | `done`, **COMPACTED**, merged `237d674f` (m6:315) | 199 |
| 44 | builder-sonnet (Opus low) | perf, merged (m6:311) | 11 |
| 45 | builder-opus (medium) | `done`, merged `e6d3d43b` (m6:317) | 90 |
| 46 | builder-sonnet (Opus low) | `done`, merged `981fc58e` (m6:320) | 63 |
| review fix 10+22 M1 | builder-opus (medium) | `done`, merged `28aa0c2` (m6:184) | 18 |

Builder total, counted entries only: **3968**, of which tickets 16–46 account for 2871. Tickets 31, 34 and 36 are not counted because the log gives no number for them.

**Amendment reviews** (reviewer, Opus high). All figures are Critical / Major / Minor.
- 09: 32 tool uses, 0 / 0 / 9 (m6:174).
- 10+22: 64 tool uses, **1** / 3 / 8. The Critical is a false tick (m6:182).
- 11+12: 74 tool uses, 0 / 3 / 11 (m6:190).
- 13: 77 tool uses, 0 / 2 / 13 (m6:201).
- 14: 74 tool uses, 0 / 3 / 7 (m6:215).
- 25: 60 tool uses, 0 / 3 / 8 (m6:220).
- 15: no count, 0 / 3 / 8 (m6:226).
- 18: 76 tool uses, **1** / 2 / 7. The Critical is a false tick (m6:229).
- 28: 47 tool uses, 0 / 1 / 7 (m6:236).
- 16: 43 tool uses, 0 / 3 / 7 (m6:254).
- 17: 43 tool uses, **1** / 1 / 8. The Critical is a false tick (m6:261).
- 19+34: 52 tool uses, 0 / 3 / 9 (m6:271).
- 35: 35 tool uses, 0 / 2 / 9 (m6:275).
- 31: 35 tool uses, 0 / 2 / 7 (m6:286).
- 20: 95 tool uses, 0 / 2 / 9 (m6:301).

Totals: **3 Critical, 33 Major, 127 Minor**, across 807 counted tool uses. Majors went to fix tickets 23, 24, 26, 27, 28, 29, 30, 32, 33, 36, 37 and 41. Fix tickets 37 and 41 got no re-review (m6:290, m6:308).

### M6 look-and-finish, usability fixes, stash put-back (tickets 47–86)

This stretch follows the 46 checkpoint (m6:321), starts from the first whole-app usability walk (m6:325), and ends with ticket 85 merged (m6:487). Ticket 86 waited for the user's yes on the stash preview (m6:485); it was built after the checkpoint, below (m6:489–499). Ticket 62 was not built either: the user chose the built behaviour (m6:393). Counts are as logged. "Cumulative" or "total" in the log means one count across that ticket's runs; this file does not add those runs together a second time.

| Role | Model | Detail |
|---|---|---|
| Prior art / research | researcher (Opus low) | Conflict visuals + finish, no count (m6:322); follow-up 61 cumulative (m6:323); checkout + cherry-pick 26 (m6:327); push/pull 19 (m6:328); commit + stash 44 (m6:329); stash-clash addendum A1–A4 31 (m6:410). |
| Designed / specified | planner-fable (Opus high) | Look+finish amendment 76 (m6:324); usability fix specs 47–52 59 (m6:330); spec-only 47–49 62 (m6:337); picks, ADR-0044, filed 54–63, 107 (m6:348); spec-only 54–58 51 (m6:352); final-walk plan 46 (m6:408); stash put-back note + ADR-0045, 79 (m6:414); 85/86 spec 47 (m6:483). Spec passes and fixes after scouts: 59 9, 60 8, 52 15, 52 decision 2, 50/51/61 24, 68+69 20, 72 14, 77 36 + 3, 78 34, 82 28, 84 35; 71 and 70 spec fixes no count (m6:417, m6:421). |
| Specified (decisions) | conductor | Review triage for 54–58 (m6:372) and wave 2 (m6:401). Mid-ticket decisions on 49, 50, 61 and 68+69 (m6:342, m6:388, m6:396, m6:428). 62 closed on the user's choice (m6:393). |
| Scouts | scout (Opus low) | 49 11, 48 11, 47 12, lane O 20, lane U 19, 59 23, 60 25, 52 14, 50/51/61 11/11/15, 67 12, 71 9, 68+69 18, 77 12, 78 13, 73 13, 74 23, 84 8, 85 15. 70 and 72: no count (m6:420, m6:423). |
| Doc edits, gap rows | researcher (Opus low) | User batch 8 (m6:319); checkpoint regen, no count (m6:321); usability gap rows 6 (m6:326); design-note pointer edits 7 (m6:338); spec-pass doc edits 10 (m6:349); keymap rows 7 (m6:415). |
| Reviewed | reviewer (Opus high) | 12 reviews and walks, listed below. |

| Ticket | Agent (model) | As logged | Tool uses |
|---|---|---|---|
| Prototype look+finish | builder-opus (medium) | merged `edaf8164` (m6:332) | 106 |
| Prototype scenes 11–13 | builder-sonnet (Opus low) | merged `ddda6fd6` (m6:336) | 18 |
| 47 | builder-sonnet (Opus low) | `done`, merged `9dae37a8` (m6:339) | 62 |
| 48 | builder-sonnet (Opus low) | `done`, 1 continuation, merged `2c794e05` (m6:340) | 56 (incl. continuation) |
| 49 | builder-opus (medium) | `partial` at the stop case, then continuation `done`, **COMPACTED**, merged `da071a1b` (m6:342–343) | 187 cumulative |
| 53 (review fix 47–49) | builder-sonnet (Opus low) | `done`, merged `eb4c6a68` (m6:346) | 54 |
| 54 | builder-sonnet (Opus low) | `done`, merged `fac64a78` (m6:354) | 33 |
| 57 | builder-opus-xhigh | part A `partial`, **COMPACTED** (m6:356); app half (separate agent after the pause) `done`, **COMPACTED**, merged `6e514378` (m6:361) | 197 + 135 |
| 55 | builder-opus (medium) | `done`, merged `ed60e73b` (m6:357) | 149 |
| 56 | builder-sonnet (Opus low) | `partial`, `blocked: needs graft`, then `done` over 3 runs, merged `4d8f687e` (m6:360, m6:363) | 53 cumulative |
| Contact-sheet split | builder-sonnet (Opus low) | `done`, merged `e1c5995b` (m6:364) | 5 |
| 58 | builder-opus | `done`, merged `59de71c7` (m6:366) | 132 |
| 59 | builder-opus | `done`, **COMPACTED**, merged `9a5187b1` (m6:373) | 168 |
| 64 (review fix 54–58) | builder-opus | `done`, merged `8b8d0826` (m6:377) | 106 |
| 60 | builder-opus | `done`, **COMPACTED**, merged `0a3a1a53` (m6:383) | 223 |
| 52 | builder-opus | `partial` at find-and-pin 2, then resumed `done`, **COMPACTED**, merged `6f8c033a` (m6:385, m6:387) | 124 cumulative |
| 50 | builder-opus | `partial` at find-and-pin 1, then `done` over 2 runs, merged `6e9007dc` (m6:388, m6:390) | 170 cumulative |
| 51 | builder-opus | `done`, merged `7661ef67` (m6:394) | 129 |
| 61 | builder-opus | 4 runs, 3 stops at its own find-and-pins, `done`, **COMPACTED**, merged `fb7a967f` (m6:396–397) | 205 cumulative |
| 63 (acceptance) | builder-fable (Opus high) | `done`, **COMPACTED**, merged `87d00090` (m6:402) | 278 |
| 65 (review fix, wave 2) | builder-opus | `done`, merged `ccb8cd8e` (m6:404) | 123 |
| 66 | builder-sonnet (Opus low) | 8/8 ticks, merged `f09d8e47` (m6:411) | 97 |
| 67 | builder-sonnet (Opus low) | 7/7 ticks, merged `78c0fa04` (m6:422) | 120 |
| 71 | builder-sonnet (Opus low) | 7/7 ticks, merged `79553aab` (m6:426) | 98 |
| 68+69 | builder-opus | run 1: 68 done, 69 `partial` 4/13 (m6:428); run 2: 11/11 + 13/13, **COMPACTED**, merged `9d7a5011` (m6:430) | 364 total |
| 76 (review fix 66/67/71) | builder-sonnet (Opus low) | 10/10 ticks, merged `1094e830` (m6:435) | 105 |
| 77 | builder-opus-xhigh | 6/6 ticks, **COMPACTED ×2**, merged `1bef5053` (m6:440) | 359 |
| 79 | builder-opus | run 1 held unmerged (m6:442); run 2 after a network error, merged `18e88d8c` (m6:445) | 30 + no count |
| 78 (review fix 68+69) | builder-opus | 18/18 ticks, **COMPACTED**, merged `ec98c71e` (m6:443) | 352 |
| 70 | builder-sonnet (Opus low) | 9/10 ticks, n1 stopped by its rule → 81, **COMPACTED**, merged `c6129790` (m6:448) | 163 |
| 81 | builder-sonnet (Opus low) | run 1 3/4, blocked on graft-ui, then runs 2–3 4/4, merged `4c16ec00` (m6:450–451) | 73 total |
| 80 (review fix 77+79) | builder-opus | run 1 died on an API error (no count), resumed 13/13, **COMPACTED**, merged `7ad92863` (m6:453) | no count + 5 |
| 72 (ops half) | builder-opus | 4/4 ticks, merged (m6:455) | 120 |
| 72b (app half) | builder-opus | 7/7 ticks, **COMPACTED**, merged `d572438f` (m6:457) | 255 |
| 73 | builder-fable (Opus high) | run 1 **COMPACTED** (m6:463); run 2 **COMPACTED**, merged `b98fc040` (m6:464) | 235 total |
| 82 (review fix 72+72b) | builder-opus | 12/12 ticks, **COMPACTED**, merged `1fb1577c` (m6:466) | 217 |
| 83 (review fix 73) | builder-opus | 9/9 ticks, merged (m6:470) | 86 |
| 74 | builder-opus | 5/5 ticks, **COMPACTED**, merged `47eea813` (m6:472) | 294 |
| 75 (acceptance, stash put-back) | builder-fable (Opus high) | 7/7 ticks, **COMPACTED**, merged `fb60034c` (m6:477) | 217 |
| 84 (review fix 74) | builder-opus | 17/17 ticks, **COMPACTED**, merged `6b5c1d1c` (m6:479) | 314 |
| 85 | builder-opus | run 1 `partial` 13/18, **COMPACTED** (m6:486); continuation 1 19/19, merged `513adf7c` (m6:487) | 220 + 150 |

Builder total, counted entries only: **6587**. Not counted: 79 run 2 and 80 run 1, because the log gives no number for them.

**Compacted in this stretch:** 49, 57 part A, 57 app half, 59, 60, 52, 61, 63, 68+69 run 2, 77 (×2), 78, 70, 80, 72b, 73 run 1, 73 run 2, 82, 74, 75, 84, 85 run 1.

**Process slips, as logged:** the 76 builder edited main's `verbs.rs`, then restored it (m6:431, m6:435). A reviewer's first gate run was in the 68+69 builder's worktree; its numbers were discarded (m6:429). Two gate failures came from text only: a temp path in 50's NOTES (m6:391) and a home-path pattern in 82's NOTES (m6:468). The conductor's uncommitted fence rows failed a gate and were reverted (m6:473).

**Reviews and walks** (reviewer, Opus high). All figures are Critical / Major / Minor.
- Usability walk (first): 99 tool uses, 0 / 6 / 11, plus 2 questions (m6:325).
- 47–49: 95 tool uses, 0 / 2 / 8; ticks 30/30 held (m6:345).
- 54–58: 65 tool uses, 0 / 2 / 9; 14/14 held (m6:371).
- Wave 2 (59–61, 50–52, 64): 108 tool uses, 0 / 3 / 9; 22/22 held (m6:400).
- Usability walk (final): 126 tool uses, 0 / 7 / 6, plus 3 questions (m6:407).
- 66/67/71: 124 tool uses, 0 / 1 / 10, plus 3 questions; 22/22 held (m6:429).
- 68+69: 77 tool uses, 0 / 3 / 7. Ticks are logged as "68 9/11 checked held, 69 12/13 held" (m6:437).
- 77+79: 76 tool uses, 0 / 2 / 13; 9/9 held (m6:447).
- 72+72b: 69 tool uses, 0 / 4 / 8; 3/3 and 6/6 held (m6:461).
- 73: 69 tool uses, **1** / 2 / 9; 4 checked, 3 held. The Critical is a false tick, fixed by 83 (m6:469–470).
- 74: 79 tool uses, 0 / 4 / 8; 22/22 held (m6:474).
- Usability walk 2: 139 tool uses, 0 / 4 / 15, plus 3 questions. One Major was already known (m6:481).

Totals: **1 Critical, 40 Major, 113 Minor**, across 1126 tool uses.

Other counted helpers in this stretch: planner-fable **755** (71 and 70 spec fixes not counted); scouts **295** (70 and 72 not counted); researcher **219** (two runs not counted).

**M6 whole milestone, counted builder tool uses as logged:** groups ≈ 3432 (m6:152) + tickets 09–46 3968 + tickets 47–85 6587 ≈ **13 987**. This sum is this file's, not the log's; the uncounted runs listed in each section are not included.

### M6 close (tickets 86–87)

After the M6 checkpoint the user answered "86 yes · looks fine · M7 go" (m6:489).

| Ticket | Agent (model) | As logged | Tool uses |
|---|---|---|---|
| 86 scout | scout (Opus low) | 4 mismatches, resolved in the builder prompt (m6:492) | 17 |
| 86 | builder-opus | 8/8 ticks named, **COMPACTED**, merged `f68f0857` (m6:494) | 205 |
| 86 review | reviewer | 0 / 2 / 7; 25/27 ticks checked, 25 held; release perf 13/13 (m6:497) | 65 |
| 87 (review fix) | builder-opus | 5/5 ticks named, merged `9034b792` (m6:498) | 131 |

Last M6 gate line: 3529 passed, 1 skipped; "M6 closed" (m6:499). The AUTHORSHIP regen at the checkpoint was a researcher run of 9 tool uses (m6:488).

## M7 Power tools

Start `942c0a5a` (m7:23). Gate line at the acceptance merge: 5061 passed, 7 skipped (m7:541). After the checkpoint, field use found scroll lag on the user's work repo (m7:543) and tickets 41–49, 46b and 46c followed; last gate line 5111 passed, 8 skipped, after 47 (m7:585); 48 reported 5120/5120 (m7:588). Tool-use counts are the harness's, as logged in `docs/log/m7.md` (cited `m7:<line>`). Where the log gives a figure "cumulative" or "total" across runs, this file uses it once. "No count" means the log gives none.

**Models.** The M7 log names agents, not models. Their models are the ones set during M6 (m6:79, m6:85): `builder-opus` is Opus medium, `builder-sonnet` is Opus low (a tier label), `builder-fable` and `planner-fable` are Opus high, `builder-opus-xhigh` is Opus xhigh, `scout` and `researcher` are Opus low. The log does not name the reviewer's or debugger's model for M7.

| Role | Agent | Detail |
|---|---|---|
| Prior art | researcher | 5 studies: blame/line history 25, stacked branches 33, reflog 29, interactive rebase 36, branch compare/submodules 35 (m7:8–12). |
| Prototype | builder-opus | Scenes 1–5, 44 (m7:16); rev 2, 50, merged `ca3f6e7a` (m7:21). The user walked both (m7:17, m7:21). |
| Designed | planner-fable | Pass 1 50, note `ab516e96`, ADR-0046/0047 (m7:14); pass 1b 34 (m7:19); pass 2a 121, 17 specs, design `942c0a5a` (m7:22); pass 2b 31 after resume, 9 specs 18–26 (m7:42). |
| Specified | planner-fable; conductor | The conductor filed 01–17 (m7:22). planner-fable wrote every respec, ruling and fix-ticket spec after that: 12 runs with no count (3 added after the checkpoint: m7:546, m7:552, m7:556), the rest **1750** counted in all (1614 to the checkpoint + 136 for the scroll-lag specs, corrections and rulings, m7:544–584) (this file's sum over the log's lines). ADRs drafted or amended by these passes: 0048 (m7:100, m7:104), 0049 (m7:133), 0050 (m7:141), 0051 (m7:277), 0052 (m7:466), 0053 (m7:544). |
| Scouts | scout | 384 counted (316 to the checkpoint + 68: 41, 43, 44, 45, 42, 46, 48; m7:545, :551, :562, :583); 11 runs with no count (17, 05 second run, 16b, 10b, 11 first run, 09, 21, 07, 24, 07d, rebase-keeps-drops). Many found the ticket's premises wrong, which sent the ticket back to the planner (e.g. m7:37, m7:95, m7:98, m7:247, m7:519, m7:529). |
| Researcher (docs, rows) | researcher | 286 counted (245 to the checkpoint + AUTHORSHIP regen 18, m7:542, + big-repo-scroll study 23, m7:543); 3 runs with no count (m7:380, m7:418, kitty-layering-deflate m7:547). It refused KNOWN_GAPS edits twice, by its definition (m7:199, m7:535). |
| Debugger | debugger | 8 runs, 699: M7/02b + M7/04b 92 (m7:62); M7/ⓘ 79 (m7:155); 12g blocked 68 (m7:272); oracle-flake 56, not reproduced (m7:468); 12g 46 (m7:484); 30v 184 (m7:503); scroll-lag diagnosis 97 (m7:543); 47 misses 77, no product change (m7:580). |
| Built | builder-opus, builder-sonnet, builder-fable, builder-opus-xhigh | 153 counted builder entries, **17 194** tool uses, mean ≈ 112 per entry (this file's sum: 140 / 16 203 to the checkpoint + 13 / 991 after it: 41 48, 45 97, 44 126, 42 73+13, 43 77 + 16 + 18, 46 101, 46c 55, 46b 87, 47 203, 49 57, 48 20; m7:549–588). Fable built 02a, 02b, 22, 22b, 26, 26b, the post-fix acceptance and caption passes, 44 and 47. Opus xhigh built 30q-a, 30q-b, 30r, 30s and 30w. Sonnet-tier: 17, 25, 25b, 10a, 20a, 20c, 20d, 22c, 18c, 18d, 06d, 32b, 12f, 33b, 14a, 14b, 20l, 33c, 34, 30d, 14, 20j, 10c, 01b, 30f, 20o, 30g, 30i, 21c, 30h, 30e, 07c, 30l, 24-0, 30m, 30p-c, 30q-c, 30t, 30t-b, 30u, 30w-b, 36, 39, 40, 41, 42, 43 (3 runs), 45, 46b, 49. Opus (builder-opus) after the checkpoint: 46, 46c, 48. The rest are builder-opus. |
| Reviewed | reviewer | 30 reviews and walks (usability walk included), listed below; 28 counted, **2378**. Two have no count: 10..07c (m7:417) and 30p–30w, which stalled in fuzz (m7:517). |

**Not counted among builders:** 04b before the network cut (m7:35), the 10b resumed run (m7:352), 24-a's verify-first stop (m7:430), 30p-a's two stalled runs ("tools n/a", m7:464), 14's first stop of 11 (m7:282), 44's lit_hash check after resume (33 tools at the network death, m7:557; no final count, m7:558) and 48's first run ("n/a", m7:588). For 14 (m7:296), 16 (m7:313) and 21b (m7:362), the log does not say whether the final figure covers earlier runs; this file takes it as cumulative.

**Model differences between spec and log:** 06f and 33 were specified Sonnet (m7:222, m7:213) and logged as builder-opus (m7:235, m7:236). 16b was dispatched as builder-sonnet (m7:318) and logged done as builder-opus (m7:344). The log gives no reason for any of the three.

**Compacted:** 02a, 02b, 18, 12 part A, 22, 12c, 20b, 20 (in orientation), 20e (×2), 30, 18e, 20i, 30c, 14, 05, 13b, 10b, 21 (×2), 21b, 09 (×2), 03b, 30j, 10 (second run), 07b, 11, 07c, 08-a, 07d, 08-b, 24-a, 24-b, 30o-b, 30q-a, 30r, 26, 30s, 44 (m7:555), 47 (m7:579): 41 in all. Splits after a compaction: 02a → 02b; 12 A → 12b, 12c; 12c → 12d, 12e; 22 → 22b; 20 → 20e, 20f; 20e → 20g; 30 → 30b; 20i → 20k; 30c → 30d; 05 → 05b; 21 → 21b, 21c; 09 → 09b; 07c → 07d; 24-a → remainder; 26 → 30s, 30t, 30u, 30v, 26b (m7:30–493). 44 and 47 compacted with nothing left, per their builders; no split.

**Reviews** (reviewer). Figures are Critical / Major / Minor.
- 02: 106 tool uses, 0 / 3 / 11; 48/48 held (m7:59). 04: 53, 0 / 3 / 11; 9/9 (m7:64).
- 22-23: 89, **1** / 4 / 9; 30/30 (m7:118). 18+19+27+10a: 95, 0 / 4 / 14; 30/30 (m7:129).
- 06 absorb: 76, **1** / 8 / 11; 3/3 (m7:150). 28a+28b: 70, 0 / 3 / 6; 13/13 (m7:174).
- 06b: 56, **2** / 3 / 6; 5/5 (m7:178). 06c: 78, 0 / 3 / 6; 6/6 (m7:190).
- 28c+31: 61, 0 / 0 / 6; 12/12 (m7:198). 20e+20g: 78, 0 / 4 / 9; 11/11 (m7:206).
- 18b+18c+18d: 85, 0 / 1 / 7; 20/22 checked, 20 held (m7:208). 30+30b+06d: 92, 0 / 1 / 8; 16/16 (m7:215).
- 06e+20h: 64, 0 / 1 / 4; 13/14 checked held (m7:219). 18e+20f: 78, 0 / 2 / 8; 12/12 (m7:224).
- 32+20i+merge fix: 62, 0 / 1 / 5; 27/27 (m7:226). 20a+20b+23b: 82, 0 / 1 / 3; 13/13 (m7:228).
- 12 A–D: 94, 0 / 3 / 5; 23/23 (m7:241). 18f/06f/33/32b: 74, 0 / 2 / 6; 26/26 (m7:250).
- 20k..20l: 70, 0 / 2 / 7; 39/39 (m7:263). 33c..34: 95, 0 / 1 / 5; 19/19 (m7:283).
- 30c..10c: 106, 0 / 2 / 7; 29/29 (m7:306). 01a..20o: 97, 0 / 3 / 11; 12/12 by assertion (m7:343).
- 16b..21b: 98, 0 / 3 / 12; 27/27 (m7:369). 09..21c: 131, 0 / 2 / 15; 29/29 (m7:395).
- 10..07c: no count, 0 / 4 / 9; 17/17 (m7:417). 08..rebase-keeps-drops: 107, 0 / 3 / 11; 28/28 (m7:436).
- 24..30n: 99, **1** / 4 / 14; 21 checked, 20 held. The Critical is a false tick on 24-b's walks (m7:447).
- 30m..30o: 69, 0 / 3 / 7; 18/18 (m7:456).
- Usability walk: 113, 0 / 6 / 9, plus 2 questions (m7:516). 30p–30w: no count, 0 / 3 / 6; 27/27 (m7:517).

Totals (this file's sums): **5 Critical, 83 Major, 248 Minor** across 30 reviews and walks. Release perf run at the milestone review: 17/17 passed, worst p99 10.6 ms (m7:490).

**Acceptance:** 26 (builder-fable) partial, 243, COMPACTED (m7:492); 26b partial 88 (m7:508), finished 24, `scripts/m7-accept.sh` rc 0, 29/29 journeys within count (m7:514). After review fixes 36–40, the re-run drifted on 9 of 13 pty runs; every changed line was traced to 36, 37 or 40 and re-taken, 51 + 81 (m7:538–540).

**After the checkpoint (scroll lag).** Model per ticket is the spec's where the log gives it: 41, 42, 43, 45, 46b Sonnet; 44, 47 Fable; 46, 46c, 48 Opus (m7:546, :563, :573, :581). No reviewer run is logged for 41–49. 47 left 2 boxes open by planner ruling (idle timing on battery + low power; Pixel RSS) as gaps (m7:581–582). User after 48: "still a bit laggy but improved for sure, lets move on" (m7:589).

M7 total after the scroll work ≈ **22 691** tool uses (≈21 281 at the checkpoint, m7:542, + 1410 logged since). Waste the log names after the checkpoint: third unchecked spec pass (m7:545), two background-wait stalls (m7:557), 45's missed heavy-only sites (m7:559).

## M8 Polish, final name, ship (m8-polish-and-ship)

Go 2026-10-02 at `f7a9a8eb` (m8:7); ticket start `f5080f50` (m8:27). Last gate line in the log: 6349/6349 after the close debug merge (m8:509); debugger 2 merged after it, gates "running" at the log's end (m8:510). Tool-use counts are the harness's, cited `m8:<line>`. "No count" means the log gives none.

**Models.** The M8 log names agents, not models. Per the M6 mapping (m6:79, m6:85): `builder-opus` Opus medium, `builder-sonnet` Opus low (tier label), `builder-fable`, `planner-fable`, `reviewer` and `debugger` Opus high, `builder-opus-xhigh` Opus xhigh, `scout` and `researcher` Opus low. The log itself never states a model for reviewer or debugger.

| Role | Agent | Detail |
|---|---|---|
| Prior art | researcher | 5 studies: scroll-smoothness 16, install-docs 23 + 37 (pass 2, harness figure after resume), first-run 27, keymap-help-remap 21, themes 19 (m8:8–13). Later passes: 03-M2 addendum 18 (m8:109), quit-while-git-runs 24 (m8:144), chrome+empty-state 16 (m8:160), palette-find 22 (m8:208), look-polish post hoc 22 (m8:300). 11 runs, 245 counted (this file's sum). |
| Prototype | builder-opus; builder-sonnet | Prototype scenes 1–4, 38 (m8:17). Phone/real-app shots by builder-sonnet: 11, 48, 65 (m8:18, :20, :21). |
| Designed | planner-fable | Pass 1, 78, COMPACTED (m8:15); pass 2a, 97, ADRs 0054–0058 (m8:24); pass 2b, 95, 20 specs + acceptance (m8:28). ADRs later: 0059 (m8:65, accepted m8:79), 0060 and 0061 (m8:237), 0062 (m8:343). Architecture pass 25 (MAPs), 70, 0 / 4 / 8, filed 33–42 (m8:421). |
| Specified | conductor; planner-fable | Conductor filed 01–28 (m8:29). planner-fable wrote every respec, split, ruling and fix spec after that: 74 lines with a count, **1897** (this file's script sum, loose match, not checked line by line over planner-fable lines); 8 lines with no count (m8:90, :225, :335, :337, :363, :365, :376, :379). |
| Scouts | scout | 37 counted, **426** (this file's script sum, loose match, not checked line by line); no count on m8:43, :48, :49, :70, :81, :352, :354. |
| Debugger | debugger | 12 runs, **1358**: 13g flake 106 (m8:235), 13h 94 (:251), http_remote 33 (:264), 13k 128 (:294), 13m 123 (:308), pull flake 94 (:339), task_4 43 (:369), 29 write_shots 50 (:401), 30 linux 341 (:411), tour/open-recent 114 (:444), close-run red 114 (:508), debugger 2 118 (:510). |
| Built | builder-opus, builder-sonnet, builder-fable, builder-opus-xhigh | Counts per run are in the log line for each ticket; this file does not total them (log lines mix cumulative, post-resume and lost counts). Fable: 02 pace, 02 review fixes, 09 key table, 18e A1, 18e A2+A3, 26 Part B, 28 acceptance. Opus xhigh: 13c, 13d, 21c A, 21c B, 19d-a, 19d-a2, 19d-a3, 19e. Sonnet-tier: 22, 22 remainder, 21 review-fix, 10b (stop + done), 11b, 09e (stop + done), 13f, 17, 10f, 10g, 10j, 10h, 10i, 10k, 22c, 11d, 11c (blocked), 12f, 10l, 20, 20 remainder, 13l A, 18d, 18c, 11f A/B/C, 19c-a, 21b-1, 21b-2, 21b-4 additions, 21b-5, 21b-6, 21b-7b, 19d-b, 19d-b2, 18e B, 20b A, 20b B, 20c, 20d, 20d-b, 24-b, 20e, 31, 32, 33–37, 37 Addition, 43 (+ clippy fix), 26r B, 27a, 27b, 27c, 27d, 38 (pass 1, pass 2, cont 2), 41, 42, 42b. The rest are builder-opus (incl. 01, 03, 04, 05, 06, 07, 08, 10, 10c, 10e, 11, 12–12e, 13, 13b, 13e, 13g, 13j, 13l B, 14–14e, 15, 15b, 16, 17b, 17c, 18a, 18b, 19a, 19b, 19c-b, 19c-c, 19d-c, 21, 21b-3, 21b-4, 21b-7a, 23, 24, 26 A, 26r A, 39, 40, 44, 45). |
| Reviewed | reviewer | 28 reviews and walks (usability walk included) + 2 quiet-run passes; 24 counted, **1955** (this file's sum). No count: 21+22 (m8:51), 13 (m8:61), 03 (paused at 48+, final not given, m8:96/:107), palette+theme (m8:261). |

**Lost or partial counts the log names:** 13b review-fix first run (lost to the network drop, m8:179); 09d planner first run (m8:180); 16 first run (m8:186); 22 remainder and 04 review fixes pre-drop legs (m8:136–137); 05 A pre-stop (m8:497); 45 pre-stop (m8:502); 28's SKILLS line (lost to compaction, m8:504); 18d merge "tools n/a" (m8:312).

**Compacted** (as logged): planner pass 1, 02, 14, 04 pass 1, 13b, 14b, 14 fixes, 07, 14c remainder, 08, 09b, 10 (×2), 10c, 09c, 13b fixes (×2), 09d, 13c, 11, 13d (×2), 11b, 10e, 09f, 12, 12b, 11e, 18b, 18c, 19b (×2), 18e A2+A3, 21c A, 21c B, 19d-a (×2), 19d-a2, 19d-a3, 19e, 26 A, 26 B, 05 Part 2, 05 A, 28.

**Reviews** (reviewer). Critical / Major / Minor.
- 01: 73, 0 / 2 / 9; 6/6 (m8:41). 21+22: no count, 0 / 2 / 10; 17/17 (m8:51).
- 13: no count, 0 / 3 / 10; 5/5 (m8:61). 02: 87, 0 / 3 / 7; 10/10 (m8:83).
- 14: 84, 0 / 5 / 9; 4/5 held (m8:86). 03: no final count, **1** / 3 / 8; 9/10 — C1 false tick on J1 (m8:107).
- 04: 50, 0 / 5 / 5; 2/2 (m8:133). 13b: 84, 0 / 4 / 7; 10/10 (m8:141).
- 14b+14c: 57, 0 / 3 / 9; 19/19 (m8:145). Look 06+07+08: 115, 0 / 2 / 11; 23/23 (m8:158).
- 09+09b: 85, 0 / 5 / 12; 14/14 (m8:162). 14d: 40, 0 / 2 / 7; 15/15 (m8:166).
- 22: 54, **1** / 3 / 4; 5 checked, 4 held — C1 false tick (m8:169). Keymap sweeps: 76, 0 / 4 / 9; 26/26 (m8:220).
- Palette+theme: no count, 0 / 4 / 11; 24/24 (m8:261). 13b-fix/13c/13d: 84, 0 / 3 / 9; 15/15 (m8:203).
- Stop+look: 91, 0 / 3 / 11; 30/30 (m8:299). Doorway+words: 119, 0 / 3 / 9; 42/42 (m8:314).
- Clone: 77, **1** / 3 / 7; 30/33 checked, 29 held — C1 false tick on 18c box 2 (m8:327).
- Palette/landing/stop: 68, **1** / 3 / 7; 26 checked, 25 held — C1 false tick on 12f D3 (m8:340).
- Bars+hygiene: 112, 0 / 1 / 8; 25/25 (m8:350). Tour/setaside/fetch: 20 after resume, 0 / 2 / 5; 15/15 (m8:378).
- Words-focus: 94, 0 / 4 / 9; 38/38 (m8:410). Hygiene-release: 63, 0 / 1 / 8; 39/39 (m8:414).
- Health-1: 95, 0 / 2 / 7; no false tick (m8:434). 26 (A+B): 90, 0 / 3 / 11; 17/17 (m8:454).
- Health-rename-2: 112, 0 / 4 / 10; 31/31 (m8:488). Usability walk: 125, 0 / 2 / 9; journeys within note counts (m8:501).
- Quiet run: 8, ran nothing (battery, m8:506); rerun no count, RED, `docs/reviews/m8-close-run.md` (m8:507).

Totals (this file's sums): **4 Critical, 84 Major, 238 Minor** across 28 reviews and walks. All four Criticals are false ticks.

**Acceptance:** 28 by builder-fable, partial 7/8, 227, COMPACTED; perf box open for a quiet run (m8:504). Close run RED (m8:507) → debugger: frames harness ran unfenced since 02, release perf 21/21 after fix, product code unchanged (m8:508); debugger 2: held_key red was the harness, trace draw miss = CPU clock state, needs a measurement-method ruling (m8:510).

**Conductor errors the log records:** 12f merged without checking ticks (m8:280); 11f A merge sealed with conflict markers (m8:319); 18c false tick merged with the gap filed (m8:327); home-path leaks in conductor commits (m8:238, :330, :333, :351); 20d-b's claim filed as broken without a line (m8:418); 24 builder left registered running (m8:429).

**User decisions logged:** max 2 helpers (m8:94); pause/resume (m8:95, :101); name Tributary/trib as placeholder, naming discussion owed (m8:22, :456); 05 deferred then built (m8:284, :490); after 05 in Ghostty: "now it felt solid" (m8:500).
