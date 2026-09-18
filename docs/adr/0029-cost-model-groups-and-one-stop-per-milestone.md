# ADR-0029: The cost model, groups instead of tickets, and one stop per milestone

**Date:** 2026-09-18
**Status:** accepted (user, 2026-09-18)
**Supersedes:** the 80k ticket-size target and the write-milestone ticket approval
in CLAUDE.md "Standing rules" (set 2026-09-13).

## Why

M2 melted a 5x Max plan. The figures, from `docs/log/m2.md`, harness only:

```
47 helper dispatches (29 sonnet, 8 opus, 10 fable)
33 with logged figures: 3,774k tokens · extrapolating the other 14 ≈ 5.4M
3,936 tool calls, mean 119 per run, against a 40-call hard stop
20 of 69 log entries COMPACTED
```

Six milestones remain, three of them write milestones. At this rate the project
does not finish.

## What actually costs

A helper's context is built once and re-sent every turn. Per run, at M2's mean
(119 turns, ~110k prefix), in base-equivalent tokens:

| Term | Arithmetic | Share |
|---|---|---|
| turn reads | 119 × 110k × 0.1 (cached read) | ~57% |
| output | 119 × ~1.5k × 5 | ~40% |
| startup write | 60k × 2 (1h TTL cache write) | ~3–8% |

**Turns dominate. Startups are under a tenth.** This was got wrong twice before
it was measured: first by sizing tickets in tokens (the log figure is context
size, not what is billed), then by blaming cold starts (they are noise next to
the turn term). Both mistakes pointed at a real cost and named the wrong cause.

Three consequences:

1. **The metric is turns per unit of work.** Not tokens per ticket, not ticket
   count, not helper count.
2. **Splitting work into more tickets does not save money.** It duplicates the
   startup (small) and saves nothing on turns. M2's 80k rule pushed
   `planner-fable` to 28 tickets and the mean still came in at 114k on 33 of 33.
3. **Cutting the prefix pays on every turn.** `housekeeping/01` (CLAUDE.md
   trimmed, DESIGN.md split, PROGRESS.md archived, KNOWN_GAPS split) is the
   largest proven win in this project, because 60k of startup junk was being
   re-read 119 times per run, not once.

What actually reduces turns is a spec precise enough to cut the
write → test → fail → fix → test loop, and delegating exploration to a scout
instead of reading source into the builder's own context.

## Decision

1. **`planner-fable` writes groups, not tickets.** A group is the unit one
   builder takes in one context: work that shares files, so the prefix is read
   once and the second and third pieces ride a warm cache. Fine-grained steps
   live *inside* the group. Decompose the work; do not decompose the context.
2. **The 80k token target is withdrawn.** Replaced by: a group must fit in one
   context, and each piece inside it reports its turn count at its boundary. A
   piece past 60 turns commits, says why, and continues deliberately.
3. **One stop per milestone, for every milestone.** Write-milestone ticket
   approval is withdrawn (user, 2026-09-18: "for me to check after every 2 or 3
   turns shouldn't be there, of course when I need to see, I can ask"). The
   gates do not change — they are the conductor's to run, not the user's to
   watch.
4. **The conductor stops unasked for exactly four things:** a visual result; work
   that cannot be put back and whose inverse is unproven; a product preference it
   cannot infer (batched into one list, not trickled); a spec change or scope cut.
5. **One review round per group.** The reviewer reports to the conductor.
   Remaining findings go to `KNOWN_GAPS.md` with an expiry milestone. A second
   round happens only for a surviving Critical.
6. **The conductor stays on Opus and does less.** Logging and commits batch per
   group, not per ticket. Its own context is long-lived and re-read every turn,
   so its turn count is a cost like any other.
7. **The user's daily use is a test tier, not a fallback.** The fast tier covers
   logic on fixtures we invented; heavy and perf cover scale at review; the
   acceptance walk covers the promise along a path someone scripted. Only real
   use reaches how it looks, repo shapes nobody made a fixture for, and sequences
   nobody thought to try. `docs/field-notes.md` catches what it finds, so a
   real-use bug survives the session it was noticed in.
8. **Unverified is not the same as broken.** `KNOWN_GAPS.md` marks work that
   shipped with nothing able to prove it. M2's capture pipeline is pyte character
   grids with no colour attributes, so no capture in this repo can prove a
   colour-only change; two M2 fixes shipped that way.

## Costs

- A wrong direction now surfaces at the milestone end, not at group two, so the
  rework is larger when it happens. Mitigated by the per-group review, which
  still runs — it reports to the conductor instead of stopping for the user.
- Groups are cost-neutral, not a saving. They were adopted for continuity —
  fewer handbacks, less knowledge dying at each return — after the arithmetic
  showed the startup saving is noise. Recorded here so the next person does not
  re-derive it as a cost argument.

## What was considered and rejected

- **A manager layer** (conductor → managers → workers). Adds a tier of cold
  starts that writes no code, and its one real advantage — parallelism — is
  unavailable: "one helper at a time" stands, because concurrent cargo runs
  thrash the laptop.
- **Specs detailed enough that no bug is found.** Sorting M2's 1 Critical and 11
  Major: 5 spec-preventable, 5 found only by contact with reality (git's index
  stat cache lying to plumbing; two colour bugs no check here can see), 2 rules
  already written and ignored. ~40% is the ceiling, and closing it means
  `planner-fable` reading source and effectively writing the code, at Fable
  rates — moving spend from build to plan rather than removing it.
- **Raising `CLAUDE_CODE_AUTO_COMPACT_WINDOW` as a saving.** It is a wash: one
  compaction ≈ 0.35M base-equivalent, the extra prefix over the same span ≈ 0.4M.
  Left at 300k (user, 2026-09-18) because a mid-group compaction is the worse
  failure, not because it saves anything.

## How we will know

Per-group turn counts and the milestone total go in the checkpoint report. If
M3's mean turns per unit of work is not well below M2's 119, this diagnosis was
wrong and gets revisited rather than reinforced.
