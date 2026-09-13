---
name: conduct
description: Run the Graft build as the conductor. Hand each step or ticket to a fresh helper agent and stop at checkpoints. Only when the user types /conduct or says "continue".
disable-model-invocation: true
effort: high
---

# Conductor

You coordinate and the helpers do the work. **Your context must stay small**, because it has to last the whole project.

**The rules live in CLAUDE.md, "Standing rules"** (tickets, models, escalation, context, checkpoint policy). This file says only how to operate: whom to dispatch, when to stop, what to log. If the two ever disagree, CLAUDE.md wins.

## Rules

- **Don't read source code, diffs or large files yourself.** Read only:
  - CLAUDE.md and PROGRESS.md
  - the ticket list
  - design notes and review files
  - `git log --oneline -10` and `git status --short`
- **Model check:** this session should be Opus. If it isn't, tell the user to run `/model opus` and stop.
- **Every piece of work goes to a fresh helper** via the Agent tool, one ticket per helper. Tell it the milestone, the ticket ID and path, and "Read CLAUDE.md first."
- **Whom to dispatch:**
  - `planner-fable` — the milestone's design pass
  - `builder-sonnet` / `builder-opus` / `builder-fable` — whatever the ticket's `Build with` line says
  - `reviewer` — Opus by default; pass `model: "fable"` in the Agent call for **M3 and M5**
  - `debugger` — Fable, at the end of the escalation chain
- **All your messages to the user** are telegraphic (CLAUDE.md, "Talking to the user").
- **Helper replies** are 150 words or fewer. Don't ask them for more.
- **One helper at a time.** Never run helpers in parallel: builds and tests would compete for the laptop's memory.
- **After each helper returns:**
  - tick the box in PROGRESS.md
  - update **Current step**
  - add a one-line Session log entry (format at the top of PROGRESS.md's Session log). If the helper compacted, end the line with `· COMPACTED`.
  - commit: `git commit -am "progress: <what>"`
- **Never** push, force anything or skip a failing gate.
- If the conversation was auto-compacted, that's fine: everything lives in files. Re-read PROGRESS.md and carry on.

## Loop

1. Read PROGRESS.md and find the first unchecked item.
2. **It's a "You" or CHECKPOINT item:** if the next milestone has no tickets yet, do 4.1 first. Ticket approval at this stop only if CLAUDE.md's checkpoint policy says so for that milestone. Then stop, tell the user exactly what to do in a few lines, and wait. If the user says they've already done it, check it's really done (for example, the files it should have produced exist), then tick it, commit and carry on.
3. **It's step 1 (the decisions interview):** if `CONTEXT.md` and ADRs already exist, the interview has probably happened but wasn't ticked. Ask the user whether it's finished; if it is, tick it, commit and carry on. Otherwise, tell the user to run the `/grill-with-docs` line from PROGRESS.md in this session, then stop.
4. **It's a milestone:**
   1. **Design pass, then tickets** (normally done at the checkpoint before the milestone):
      - No `docs/design-notes/<milestone>.md` → send `planner-fable` the milestone slug and its PROMPT.md §8 entry; commit the note as `design: <milestone>`.
      - No tickets → run `/to-tickets` from that note, following CLAUDE.md "Standing rules → Tickets" and "→ Models".
      - **Stop and tell the user** if the plan wants three or more Fable tickets, or the note would differ from DESIGN.md (a Proposed change; tickets follow DESIGN.md until approved).
      - Ticket approval: only where the checkpoint policy requires it.
   2. **Build each ticket in dependency order,** dispatching per its `Build with` line whenever it's unblocked:
      - `done` → next ticket.
      - **Compacted** → log `· COMPACTED`. If work remains, split the remainder into new tickets and build those, not the oversized ticket. Write milestones: the user approves the split tickets at the next stop.
      - `partial` → a fresh builder on the same model continues from the handoff note. **At most 2 continuations per ticket**; a third `partial` → stop and ask the user whether to split.
      - `failed` / stalled → escalate per CLAUDE.md "Standing rules → Models → Escalation". Log the escalation.
      - The debugger fails too → stop and ask the user.
   3. **Review:** once every ticket is done, send the milestone to `reviewer`.
      - Critical or Major issues → send them to a builder (grouped sensibly), then run `reviewer` again. **At most 2 rounds.**
      - Still open after that → stop or carry on per the checkpoint policy; anything left that doesn't stop the milestone goes to KNOWN_GAPS.md.
   4. **Milestone checkpoint:** do 4.1 for the next milestone, then stop and report (limits: CLAUDE.md "Checkpoint policy"):
      ```
      M<n> done. <one-line result>
      Built: <3–5 one-line bullets>
      Models: Sonnet 01,03,04 · Opus 02,05 · Fable 06 · escalated: <ids or none>
      Compacted: <ticket ids → split into ids, or none>
      Try: <exact commands>
      Gaps: <bullets or "none">
      Review: <n> critical / <n> major fixed, <n> minor left
      Next: M<n+1>, <count> tickets (<path>). Approve? yes / change   (read-only next milestone: "continue?")
      ```
5. Keep going until something makes you stop.

When the user says **"continue"**, start again at step 1 of the loop.
