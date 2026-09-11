---
name: conduct
description: Run the Graft build as the conductor. Hand each step or ticket to a fresh helper agent and stop at checkpoints. Only when the user types /conduct or says "continue".
disable-model-invocation: true
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
  - Builders: `builder-sonnet`, `builder-opus`, `builder-fable`
  - Also: `reviewer` and `debugger`
- **Choosing a builder:** use the "Build with" column in PROGRESS.md. A note in brackets, such as "(Opus for undo)", applies only to tickets about that part.
- **Helper replies** are 150 words or fewer. Don't ask them for more.
- **After each helper returns:**
  - tick the box in PROGRESS.md
  - update **Current step**
  - add a one-line Session log entry
  - commit: `git commit -am "progress: <what>"`
- **Never** push, force anything or skip a failing gate.
- If the conversation was auto-compacted, that's fine: everything lives in files. Re-read PROGRESS.md and carry on.

## Loop

1. Read PROGRESS.md and find the first unchecked item.
2. **It's a "You" or CHECKPOINT item:** stop. Tell the user exactly what to do, in a few lines, and wait.
3. **It's step 1 (the decisions interview):** tell the user to run the `/grill-with-docs` line from PROGRESS.md in this session, then stop.
4. **It's step 2 or 3:** send the step's task text from PROGRESS.md to `builder-fable`.
5. **It's a milestone:**
   1. **No tickets for it yet:** run `/to-tickets` for this milestone here, based on PROMPT.md section 8, DESIGN.md and CONTEXT.md. Size each ticket so a helper can finish it using well under 100k tokens. Put the ticket list in front of the user at the next checkpoint for approval.
   2. **Build each ticket in order,** dispatching the right builder whenever a ticket is unblocked:
      - `done` → move to the next ticket.
      - `partial` → a fresh builder on the same model continues from the handoff note.
      - `failed` twice on the same ticket → `debugger`.
      - The debugger fails too → stop and ask the user.
   3. **Review:** once every ticket is done, send the milestone to `reviewer`.
      - If it lists Critical or Major issues, send them to a builder (grouped sensibly), then run `reviewer` again. At most 2 rounds.
      - If issues are still open after that, stop and report to the user.
   4. **Milestone checkpoint:** stop and give the user:
      - what was built (3–5 bullets)
      - how to try it (the README demo steps)
      - known gaps
      - the drafted ticket list for the next milestone, for approval
6. Keep going until something makes you stop.

When the user says **"continue"**, start again at step 1 of the loop.
