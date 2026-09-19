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
- **Every piece of work goes to a fresh helper** via the Agent tool, **one group per helper** (ADR-0029) — the group's pieces run inside that one context, so the prefix is read once. Tell it the milestone, the group ID and path, and "Read CLAUDE.md first."
- **Whom to dispatch:**
  - `planner-fable` — the milestone's design pass **and its ticket specs** (M2 onward); a spec-only pass when a split needs new decisions; the M8 architecture-pass ticket
  - `builder-sonnet` / `builder-opus` / `builder-fable` — whatever the ticket's `Build with` line says (`Fable (signature)` → `builder-fable`)
  - `reviewer` — Fable (its default) for **every milestone from M2**; pass `model: "opus"` in the Agent call for M1 only
  - `debugger` — Fable, at the end of the escalation chain
- **All your messages to the user** are telegraphic (CLAUDE.md, "Talking to the user").
- **Helper replies** are 150 words or fewer. Don't ask them for more.
- **Builders run in parallel, isolated** (ADR-0036): start at **two**, measure, then try three. Each gets its own worktree (Agent `isolation: "worktree"`), its own `CARGO_TARGET_DIR`, and a **disjoint crate set** derived from its spec's file list — never run two groups whose crate sets intersect. **Reviews stay serial.** You merge each branch as it lands and resolve conflicts yourself; a conflict you can't resolve from the two specs goes back to the owning builder. A `blocked: needs <crate>` report means merge the holder's branch and resume, or serialise the remainder — and log it, because a cross-crate need is a spec defect.
- **After each helper returns**, add a one-line entry to the current milestone's live log, `docs/log/<milestone>.md` (format at the top of that file's Session log; not PROGRESS.md, which only points to it), ending with **the helper's turn count** — the harness figure, never the helper's own claim, which has been wrong by 8x. If it compacted, end with `· COMPACTED`.
- **Batch the ceremony per group, not per helper** (ADR-0029): tick the box, update **Current step** and `git commit -am "progress: <what>"` once the group is done. Your own turns cost the same as anyone's.
- **You file tickets; you don't decide them** (M2 onward). Scope, signatures and `Build with` come from `planner-fable`'s specs. If one looks wrong, send it back to `planner-fable` or ask the user; don't edit it.
- **Never** push, force anything or skip a failing gate.
- If the conversation was auto-compacted, that's fine: everything lives in files. Re-read PROGRESS.md and carry on.

## Loop

1. Read PROGRESS.md and find the first unchecked item.
2. **It's a "You" or CHECKPOINT item:** if the next milestone has no groups yet, do 4.1 first. **No ticket approval at any milestone** (ADR-0029) — one stop per milestone, and the four things you stop unasked for are in CLAUDE.md's checkpoint policy. Then stop, tell the user exactly what to do in a few lines, and wait. If the user says they've already done it, check it's really done (for example, the files it should have produced exist), then tick it, commit and carry on.
3. **It's step 1 (the decisions interview):** if `CONTEXT.md` and ADRs already exist, the interview has probably happened but wasn't ticked. Ask the user whether it's finished; if it is, tick it, commit and carry on. Otherwise, tell the user to run the `/grill-with-docs` line from PROGRESS.md in this session, then stop.
4. **It's a milestone:**
   1. **Design pass and ticket specs, then filing** (normally done at the checkpoint before the milestone):
      - No `docs/design-notes/<milestone>.md` → send `planner-fable` the milestone slug, its PROMPT.md §8 entry and "write the note and the ticket specs". It writes the note plus unnumbered specs in `.scratch/<milestone>/issues/`. If it expects more than about eight groups, it does pass 1 (note + interface-first specs) and says so; send a fresh `planner-fable` for pass 2. Commit as `design: <milestone>`.
      - **File and sequence** the specs: number them `NN-<slug>.md` in dependency order, the acceptance ticket last (if the spec pass didn't write one, send it back to `planner-fable`), set `Blocked by:` from each spec's `Needs:`, set `Status: ready`. Don't change scope, signatures or `Build with`. Commit as `tickets: <milestone>`.
      - **Don't run `/to-tickets`** for any milestone with a Fable spec pass (M2 onward). It was for M0 and M1 only.
      - M8: file the standing `fable-architecture-pass.md` ticket with the rest.
      - **Stop and tell the user** if the plan wants three or more Fable builds (signature tickets count; escalation redos and the acceptance ticket don't), or the note would differ from DESIGN.md (a Proposed change; tickets follow DESIGN.md until approved).
   2. **Build each group in dependency order,** dispatching per its `Build with` line whenever it's unblocked. When dispatching the milestone's first group, add `M<n> start <hash>` (current HEAD) to the milestone's live log (`docs/log/<milestone>.md`): the reviewer diffs from it.
      - `done` → review the group (3), then the next group.
      - **Compacted** → log `· COMPACTED`. If work remains, split the remainder into new tickets along the handoff note's seams, keeping scope, signatures and model, and build those, not the oversized ticket. A split that needs a new decision goes to `planner-fable` as a spec-only pass. You decide the split; the user sees it at the milestone stop.
      - `partial` → a fresh builder on the same model continues from the handoff note. **At most 2 continuations per ticket**; a third `partial` → stop and ask the user whether to split.
      - `failed` / stalled → escalate per CLAUDE.md "Standing rules → Models → Escalation". Log the escalation.
      - The debugger fails too → stop and ask the user.
   3. **Review, one round per group** (ADR-0029): when a group is done, send `reviewer` that group's start hash from the live log (it reads the diff and MAP.md files first; CLAUDE.md "Reviewer efficiency").
      - Critical or Major → one builder fixes them, then move on. **Don't re-review.** A second round only for a surviving Critical.
      - Everything left goes to `KNOWN_GAPS.md` with an expiry milestone — marked **broken** or **unverified**, which are not the same thing.
   4. **Milestone checkpoint:** regenerate `docs/AUTHORSHIP.md` from the session logs (`docs/log/*.md`; facts from the log only) and commit it. Do 4.1 for the next milestone, then stop and report (limits: CLAUDE.md "Checkpoint policy"):
      ```
      M<n> done. <one-line result>
      Built: <3–5 one-line bullets>
      Models: spec Fable · Sonnet g1,g3 · Opus g2 · Fable g4 (signature) · review Fable · escalated: <ids or none>
      Turns: g1 <n> · g2 <n> · … · total <n>, mean <n>/group   (ADR-0029's test: well under M2's 119)
      Compacted: <group ids → split into ids, or none>
      Try: <exact commands>
      Gaps: <bullets or "none">   broken and unverified marked apart; an open test-plan layer or an unjudged acceptance group is a gap — never call the milestone done over one
      Judge by eye: <captures the user must look at, or "none">
      Review: <n> critical / <n> major fixed, <n> left in gaps
      Next: M<n+1>, <count> groups (<path>). continue?
      ```
5. Keep going until something makes you stop.

When the user says **"continue"**, start again at step 1 of the loop.
