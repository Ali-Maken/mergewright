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

## M1 The graph (in progress)

| Role | Model | Detail |
|---|---|---|
| Designed | Fable | DESIGN.md (step 2). No per-milestone design note. |
| Specified | Opus | "M1 ticket split · opus" — 17 tickets, later replanned to 31. Ticket 12's remainder split into 12b/12c by the conductor (Opus). |
| Built | see below | |
| Reviewed | — | Not yet run. Planned on Opus (M1 keeps its routing). |

M1 builds so far:

| Model | Tickets |
|---|---|
| Opus | 01, 02, 03, 04, 07, 08, 09 (+1 continuation), 11, 13, 15 (+1 continuation), 16, 18 |
| Fable | 05, 06, 14 |
| Sonnet | 10, 12 (partial, split), 12b, 12c, 17, 31 |
| Opus (conductor, not a ticket) | proptest counts moved into code (ADR-0017) |

Escalations: none recorded. Compacted runs: 07, 10, 11, 12, 12b, 14, 17, 18.

## M2–M8

Not started. From M2 (rules changed 2026-09-13): Fable writes the design note and ticket specs, Fable builds the signature tickets, Fable reviews.
