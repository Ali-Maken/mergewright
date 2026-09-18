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
  - the current milestone's session log, `docs/log/<milestone>.md` (linked from PROGRESS.md; housekeeping/01 continuation 1 moved the live log out of PROGRESS.md)
  - the ticket list
  - design notes and review files
  - `git log --oneline -10` and `git status --short`
- **Model check:** this session should be Opus. If it isn't, tell the user to run `/model opus` and stop.
- **Every piece of work goes to a fresh helper** via the Agent tool, one ticket per helper. Tell it the milestone, the ticket ID and path, and "Read CLAUDE.md first."
- **Whom to dispatch:**
  - `planner-fable` — the milestone's design pass **and its ticket specs** (M2 onward); a spec-only pass when a split needs new decisions; the M8 architecture-pass ticket
  - `builder-sonnet` / `builder-opus` / `builder-fable` — whatever the ticket's `Build with` line says (`Fable (signature)` → `builder-fable`)
  - `reviewer` — Fable (its default) for **every milestone from M2**; pass `model: "opus"` in the Agent call for M1 only
  - `debugger` — Fable, at the end of the escalation chain
- **All your messages to the user** are telegraphic (CLAUDE.md, "Talking to the user").
- **Helper replies** are 150 words or fewer. Don't ask them for more.
- **One helper at a time.** Never run helpers in parallel: builds and tests would compete for the laptop's memory.
- **After each helper returns:**
  - tick the box in PROGRESS.md
  - update **Current step**
  - add a one-line entry to the current milestone's live log, `docs/log/<milestone>.md` (format at the top of that file's Session log; not PROGRESS.md, which only points to it). If the helper compacted, end the line with `· COMPACTED`.
  - commit: `git commit -am "progress: <what>"`
- **You file tickets; you don't decide them** (M2 onward). Scope, signatures and `Build with` come from `planner-fable`'s specs. If one looks wrong, send it back to `planner-fable` or ask the user; don't edit it.
- **Never** push, force anything or skip a failing gate.
- If the conversation was auto-compacted, that's fine: everything lives in files. Re-read PROGRESS.md and carry on.

## Loop

1. Read PROGRESS.md and find the first unchecked item.
2. **It's a "You" or CHECKPOINT item:** if the next milestone has no tickets yet, do 4.1 first. Ticket approval at this stop only if CLAUDE.md's checkpoint policy says so for that milestone. Then stop, tell the user exactly what to do in a few lines, and wait. If the user says they've already done it, check it's really done (for example, the files it should have produced exist), then tick it, commit and carry on.
3. **It's step 1 (the decisions interview):** if `CONTEXT.md` and ADRs already exist, the interview has probably happened but wasn't ticked. Ask the user whether it's finished; if it is, tick it, commit and carry on. Otherwise, tell the user to run the `/grill-with-docs` line from PROGRESS.md in this session, then stop.
4. **It's a milestone:**
   1. **Design pass and ticket specs, then filing** (normally done at the checkpoint before the milestone):
      - No `docs/design-notes/<milestone>.md` → send `planner-fable` the milestone slug, its PROMPT.md §8 entry and "write the note and the ticket specs". It writes the note plus unnumbered specs in `.scratch/<milestone>/issues/`. If it expects more than about twelve tickets, it does pass 1 (note + interface-first specs) and says so; send a fresh `planner-fable` for pass 2. Commit as `design: <milestone>`.
      - **File and sequence** the specs: number them `NN-<slug>.md` in dependency order, the acceptance ticket last (if the spec pass didn't write one, send it back to `planner-fable`), set `Blocked by:` from each spec's `Needs:`, set `Status: ready`. Don't change scope, signatures or `Build with`. Commit as `tickets: <milestone>`.
      - **Don't run `/to-tickets`** for any milestone with a Fable spec pass (M2 onward). It was for M0 and M1 only.
      - M8: file the standing `fable-architecture-pass.md` ticket with the rest.
      - **Stop and tell the user** if the plan wants three or more Fable builds (signature tickets count; escalation redos and the acceptance ticket don't), or the note would differ from DESIGN.md (a Proposed change; tickets follow DESIGN.md until approved).
      - Ticket approval: only where the checkpoint policy requires it.
   2. **Build each ticket in dependency order,** dispatching per its `Build with` line whenever it's unblocked. When dispatching the milestone's first ticket, add `M<n> start <hash>` (current HEAD) to the milestone's live log (`docs/log/<milestone>.md`): the reviewer diffs from it.
      - `done` → next ticket.
      - **Compacted** → log `· COMPACTED`. If work remains, split the remainder into new tickets along the handoff note's seams, keeping scope, signatures and model, and build those, not the oversized ticket. A split that needs a new decision goes to `planner-fable` as a spec-only pass. Write milestones: the user approves the split tickets at the next stop.
      - `partial` → a fresh builder on the same model continues from the handoff note. **At most 2 continuations per ticket**; a third `partial` → stop and ask the user whether to split.
      - `failed` / stalled → escalate per CLAUDE.md "Standing rules → Models → Escalation". Log the escalation.
      - The debugger fails too → stop and ask the user.
   3. **Review:** once every ticket is done, send the milestone to `reviewer` with the milestone's start hash from its live log (`docs/log/<milestone>.md`; it reads the diff and MAP.md files first; CLAUDE.md "Reviewer efficiency").
      - Critical or Major issues → send them to a builder (grouped sensibly), then run `reviewer` again. **At most 2 rounds.**
      - Still open after that → stop or carry on per the checkpoint policy; anything left that doesn't stop the milestone goes to KNOWN_GAPS.md.
   4. **Milestone checkpoint:** regenerate `docs/AUTHORSHIP.md` from the session logs (`docs/log/*.md`; facts from the log only) and commit it. Do 4.1 for the next milestone, then stop and report (limits: CLAUDE.md "Checkpoint policy"):
      ```
      M<n> done. <one-line result>
      Built: <3–5 one-line bullets>
      Models: spec Fable · Sonnet 01,03,04 · Opus 02,05 · Fable 06 (signature) · review Fable · escalated: <ids or none>
      Compacted: <ticket ids → split into ids, or none>
      Try: <exact commands>
      Gaps: <bullets or "none">   (open test-plan layers or an unjudged acceptance ticket are gaps: never call the milestone done over them)
      Review: <n> critical / <n> major fixed, <n> minor left
      Next: M<n+1>, <count> tickets (<path>). Approve? yes / change   (read-only next milestone: "continue?")
      ```
5. Keep going until something makes you stop.

When the user says **"continue"**, start again at step 1 of the loop.
