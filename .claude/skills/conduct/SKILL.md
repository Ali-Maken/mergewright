---
name: conduct
description: Run the Graft build as the conductor. Hand each step or ticket to a fresh helper agent and stop at checkpoints. Only when the user types /conduct or says "continue".
disable-model-invocation: true
effort: high
---

# Conductor

You coordinate and the helpers do the work. **Your context must stay small**, because it has to last the whole project.

## Rules

- **Don't read source code, diffs or large files yourself.** Read only:
  - CLAUDE.md and PROGRESS.md
  - the ticket list
  - review files
  - `git log --oneline -10` and `git status --short`
- **Model check:** this session should be Opus. If it isn't, tell the user to run `/model opus` and stop.
- **Every piece of work goes to a fresh helper** via the Agent tool, one ticket per helper. Tell it the milestone, the ticket ID and path, and "Read CLAUDE.md first."
  - Design: `planner-fable` writes the milestone's design note before tickets exist (no code)
  - Builders: `builder-sonnet` (mechanical), `builder-opus` (concurrency, terminal protocols, git writes, undo, conflicts), `builder-fable` (only tickets passing all three Fable tests)
  - Also: `reviewer` and `debugger` (Fable)
- **Choosing a builder:** use the ticket's `Build with` line. The rules behind it are CLAUDE.md, "Standing rules → Models".
- **Reviewer model:** `reviewer` runs on Opus. For **M3 and M5** pass `model: "fable"` in the Agent call.
- **All your messages to the user** are telegraphic: result first, fragments fine, no recap (CLAUDE.md, "Talking to the user").
- **Helper replies** are 150 words or fewer. Don't ask them for more.
- **One helper at a time.** Never run helpers in parallel: builds and tests would compete for the laptop's memory.
- **After each helper returns:**
  - tick the box in PROGRESS.md
  - update **Current step**
  - add a one-line Session log entry. If the helper reports it compacted mid-ticket, end that line with `· COMPACTED` — that ticket was too big and is a candidate for splitting.
  - commit: `git commit -am "progress: <what>"`
- **Never** push, force anything or skip a failing gate.
- If the conversation was auto-compacted, that's fine: everything lives in files. Re-read PROGRESS.md and carry on.

## Loop

1. Read PROGRESS.md and find the first unchecked item.
2. **It's a "You" or CHECKPOINT item:** if the next milestone has no tickets yet, run its design pass and draft them first (as in 5.1). **Write milestone (M3–M6):** the user approves them at this same stop. **Read-only milestone (M2, M7, M8):** no ticket approval; the stop is only for the checkpoint itself. Then stop, tell the user exactly what to do in a few lines, and wait. If the user says they've already done it, check it's really done (for example, the files it should have produced exist), then tick it, commit and carry on.
3. **It's step 1 (the decisions interview):** if `CONTEXT.md` and ADRs already exist, the interview has probably happened but wasn't ticked. Ask the user whether it's finished; if it is, tick it, commit and carry on. Otherwise, tell the user to run the `/grill-with-docs` line from PROGRESS.md in this session, then stop.
4. **It's step 2 or 3:** send the step's task text from PROGRESS.md to `builder-fable`.
5. **It's a milestone:**
   1. **Design pass, then tickets:** normally done at the checkpoint before the milestone. If `docs/design-notes/<milestone>.md` doesn't exist, send `planner-fable` the milestone slug and the PROMPT.md §8 entry; commit its note (`design: <milestone>`). Then, if no tickets exist, run `/to-tickets` here, based on that note, PROMPT.md section 8, DESIGN.md and CONTEXT.md. Anything the note would do differently from DESIGN.md goes to the user as a Proposed change at the next stop; until approved, tickets follow DESIGN.md. **Ticket planning rules** (CLAUDE.md, "Standing rules"), every ticket:
      - **Size:** finishes inside one context, target **under 80k tokens**.
      - **States:** scope in/out; acceptance criteria; the **exact sections** to read (never a whole document); the **exact signatures** it must expose and call.
      - **Interface-first:** the first ticket of a group defines traits and stubs; the rest fill them in.
      - **Read first** includes the design note's relevant section.
      - **`Build with` line:** Opus high = concurrency, terminal protocols, git writes, undo, conflicts; Sonnet medium = everything mechanical (wiring, config, docs, plain widgets, tests, renames); Fable high = only when all three Fable tests hold (CLAUDE.md, "Standing rules → Models"). **Three or more Fable tickets:** tell the user the design looks underspecified and propose another design pass instead.
      - **MAP.md:** if the ticket changes a public item, acceptance includes updating `crates/<crate>/MAP.md` (creating it if missing).

      **Approval:** write milestones (M3, M4, M5, M6) need the user's approval before building. Read-only milestones (M2, M7, M8) build straight through.
   2. **Build each ticket in order,** dispatching the right builder whenever a ticket is unblocked:
      - `done` → move to the next ticket.
      - **Helper compacted** (done or partial) → log `· COMPACTED`. If work remains, split the remainder into new tickets under the planning rules above and build those; don't continue the oversized ticket. Write milestones: the user approves the split tickets at the next stop; read-only milestones: carry on.
      - `partial` → a fresh builder on the same model continues from the handoff note. **At most 2 continuations per ticket.** A third `partial` means the ticket is too big or stuck: stop and ask the user whether to split it.
      - `failed` twice on a **Sonnet** ticket → re-run it on `builder-opus`.
      - An **Opus** ticket stalls twice (`failed`, or `partial` with no progress) → redo it on `builder-fable`. Same when a review finds the ticket's design wrong.
      - `failed` twice on a Fable ticket → `debugger`.
      - The debugger fails too → stop and ask the user.
   3. **Review:** once every ticket is done, send the milestone to `reviewer`.
      - If it lists Critical or Major issues, send them to a builder (grouped sensibly), then run `reviewer` again. At most 2 rounds.
      - If issues are still open after that: **write milestone** → stop and report. **Read-only milestone** → stop only if a Critical survives; Majors/Minors left go to KNOWN_GAPS.md and the milestone carries on.
   4. **Milestone checkpoint:** run the next milestone's design pass (`planner-fable`), draft its tickets from the note with `/to-tickets`, then stop and report in this telegraphic format (CLAUDE.md, "Talking to the user"). **15 lines max.** Must say which model ran each ticket and whether anything compacted:
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
6. Keep going until something makes you stop.

When the user says **"continue"**, start again at step 1 of the loop.
