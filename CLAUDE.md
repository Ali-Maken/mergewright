# Graft: rules for Claude

"Graft" is only a **codename**: the name is taken by another Git GUI. The real name gets chosen in M8. Don't publish, register or tag anything public under this name.

## Where things live

| File | Purpose |
|---|---|
| `PROMPT.md` | The product spec. The note above its first `---` is for humans. |
| `DESIGN.md` | The approved design (exists after step 2). |
| `CONTEXT.md` | The glossary from `/grill-with-docs`. Use its terms everywhere. |
| ADRs | Decision records, in the folder `docs/agents/domain.md` names (default `docs/adr/`). **Wherever PROMPT.md says `DECISIONS.md`, write an ADR instead.** |
| `PROGRESS.md` | The plan and the current step. |
| Tickets | Local markdown, wherever `docs/agents/issue-tracker.md` says. |
| `KNOWN_GAPS.md` | Anything unfinished or stubbed. |
| `docs/reviews/` | Milestone reviews. |

## How work runs

- The user runs `/conduct` in a main session on **Opus**. The conductor hands every piece of work to a fresh helper in `.claude/agents/` and stops at checkpoints.
- **Who builds what, ticket size, context and checkpoints:** see **Standing rules** below (set 2026-09-13, every remaining milestone). They replace the 2026-09-12 cost-control routing.
  `xhigh` is not automatic: the conductor dispatches `builder-opus-xhigh` only when the user asks for it on a specific ticket.
- **The one exception is step 1** (`/grill-with-docs`), which runs in the main session because it needs the user.
- Do only the next unchecked step. Never start a new milestone without the user's OK at a checkpoint.

## Standing rules (every remaining milestone, set 2026-09-13)

### Tickets

- **Size:** each ticket finishes inside one context, target **under 80k tokens**. Compaction means the ticket was too big: the helper says so in its report, the conductor splits the remainder into new tickets.
- **Each ticket states:** scope in/out; acceptance criteria; the **exact sections** to read (never a whole document); the **exact signatures** the work must expose and call.
- **Interface-first grouping:** the first ticket of a group defines the traits and stubs; the rest fill them in.

### Models

**Fable decides; Opus plans; Opus and Sonnet build.**

| Work | Who | Model, effort |
|---|---|---|
| **Design pass** at the start of every milestone, before tickets | `planner-fable` | Fable, high |
| Drafting tickets from the design note | the conductor | Opus |
| Concurrency, terminal protocols, git writes, undo, conflicts | `builder-opus` | Opus, high |
| Everything mechanical: wiring, config, docs, plain widgets, tests, renames | `builder-sonnet` | Sonnet, medium |
| A ticket that passes all three Fable tests (below) | `builder-fable` | Fable, high |
| Reviews | `reviewer` | Opus; **Fable for M3 and M5** |
| Debugging | `debugger` | Fable |

- **Design pass:** `planner-fable` reads only the relevant PROMPT.md and DESIGN.md sections and the crate MAP.md files — never source — and writes `docs/design-notes/<milestone>.md`, two pages max. It covers:
  - the interfaces and data shapes the milestone introduces
  - the two or three hard-to-reverse choices, each with a recommendation
  - where it would differ from DESIGN.md, and why
  - which tickets are genuinely risky

  No code. The conductor drafts tickets from the note; each ticket's "Read first" lists the note's relevant section, and builders follow it.
- **Fable builds a ticket only when all three hold:**
  1. the ticket doesn't decide the approach;
  2. a wrong choice becomes structural for later tickets;
  3. correctness can't be judged from the diff or the tests.

  Expect one or two per milestone. If a plan wants three or more, the conductor says so: the design is usually underspecified and needs another design pass, not more Fable builds.
- **Escalation beats prediction:**
  - Sonnet ticket fails twice → Opus.
  - Opus ticket stalls twice (`failed` or `partial` without progress), or a review finds its design wrong → redo that ticket on `builder-fable`.
  - Fable fails too → `debugger`.
- Each ticket's `Build with` line names its model; the conductor follows it.

### Context

- **Scouts:** builders delegate exploration to a Sonnet or Haiku scout (Agent tool, `Explore` with `model: haiku` or `sonnet`) that returns a summary of one page or less, instead of reading source into their own context.
- **MAP.md:** every crate keeps `crates/<crate>/MAP.md`, one line per public item. Any ticket changing a public item updates it. Builders read MAP.md before source. A crate without one gets it from the next ticket that touches the crate.

### Checkpoint policy

- **Read-only milestones (M1, M2, M7, M8):** build straight through, no ticket approval. Stop only if a Critical survives review, or at the end.
- **Write milestones (M3, M4, M5, M6):** full gates and ticket approval.
- **Checkpoint reports:** 15 lines max, telegraphic; say which model ran each ticket and whether anything compacted.

## Rules for every agent

- **Never mark stubbed or half-done work as done.** List it in `KNOWN_GAPS.md`.
- **Rust code is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
- **Commit** after each ticket. Never push or force.
- **Ask the user** (through the conductor) before changing the spec or cutting scope. Record agreed changes as ADRs.
- **Architecture and readability are hard rules** (PROMPT.md 6.1 and 6.2). Read `ARCHITECTURE.md` before adding or moving modules, use the `codebase-design` skill when you do, and update `ARCHITECTURE.md` if the structure changes. Never add a dependency that breaks the crate rules.
- **Keep your context lean:** read what you need, not everything.
- **Context window:** `CLAUDE_CODE_AUTO_COMPACT_WINDOW` is `200000` in `.claude/settings.json`, raised from 150000 on 2026-09-12. Builders were compacting mid-ticket; losing the prompt cache and re-reading the same files costs more than the larger window does. Keeping your context lean still matters — the cap is headroom, not an allowance.
- **If you compact mid-ticket, say so in your report.** The conductor logs it in PROGRESS.md's session log and splits the remainder into new tickets.
- **Fix the cause, not the symptom.** When something fails, find out why before changing anything. No workaround, retry loop, sleep, hardcoded special case or disabled test without the user's OK. If the root cause is out of scope: stop, say what it is, and ask — don't route around it and log a gap.
- **Never swallow an error.** A hidden failure doesn't go away; it ships. No ignored `Result`, no `let _ =`, no `unwrap_or_default()` standing in for a real failure, no empty `Err(_) => {}` arm, no catch-all that logs and carries on. Every error is handled, propagated, or surfaced to the user. If you genuinely must ignore one, say in a comment why it's safe.
- **A status field must never read as an error when nothing is wrong.** Name and render status by its state: `gfx ok` / `gfx: <error>`, or show the field only when there is something to report. A permanently-visible `GFX_ERROR:` label trains the user to ignore it, which is how a real error gets missed.
- **Green gates are not evidence.** `fmt`/`clippy`/`test` passing means the code compiles and your own tests agree with you. It does not mean the thing works. Say what you actually verified and how, and what you couldn't. Never claim behaviour you didn't observe.
- **Tests must be laptop-safe** (PROMPT.md 7.1). Builders run only the fast tier (`cargo test`). The heavy tier, fuzzing and benchmarks run only in milestone reviews.
- **One heavy job at a time:** never run two cargo commands or test suites at once. Never generate large fixtures outside the heavy tier.

## Agent skills

### Which skills to use

The Matt Pocock plugin's skills are **banned by default**. Only these are allowed, and only for what the "When" column says:

| Skill | Allowed for |
|---|---|
| `to-tickets` | Splitting a milestone into tickets. Once per milestone. |
| `implement` | Building a ticket. |
| `tdd` | Test-first work inside a ticket. |
| `code-review` | Reviewing a finished ticket or milestone. |
| `diagnosing-bugs` | **`debugger` agent only.** Not builders. |
| `codebase-design` | Module changes only (adding or moving modules). |
| `handoff` | Mid-ticket handover only. |
| `improve-codebase-architecture` | After M2, M5 and M7 only. |

**Every other skill in that plugin is banned**, `prototype` and `wayfinder` included.

If you think you need a banned skill: **stop and ask the user** (through the conductor). Don't use it, and don't work around it silently.

Why the two that keep getting reached for:
- `prototype` builds HTML prototypes. Questions about terminal rendering are answered only by a binary running in a real terminal. Keep its principles — throwaway, minimal, findings recorded as an ADR — not the skill.
- `wayfinder` (and `to-spec`) would create a second plan. The plan already exists: `PROMPT.md` is the spec, `DESIGN.md` the design, `PROGRESS.md` the plan.

Some allowed skills aren't in the Skill tool's listing. They are installed; read the SKILL.md directly:
`~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/<name>/SKILL.md`

### Issue tracker

Local markdown: tickets live as files under `.scratch/<milestone-slug>/issues/NN-<slug>.md`. There is no remote. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Talking to the user (all reports, checkpoints, questions, review files)

User wants clear + concise over grammar. Telegraphic style.
- Fragments fine. No filler, no preamble, no recap, no politeness padding.
- Lead with result or decision. Then what user must do.
- Bullets, one line each. Exact numbers, paths, commands.
- Risks/unknowns stated plainly, not softened.
- Questions: one line + short options.
- Code and doc comments: readable, per PROMPT.md 6.2.

Example:
```
M1 done. Graph renders, 3 tiers. Pixel ok in Ghostty.
Try: cargo run -- ~/code/api   (j/k scroll, / search)
Gaps: hover tooltip → M2
Next: M2, 9 tickets (.scratch/m2/). Approve? yes / change
```

## Design docs (DESIGN.md, ARCHITECTURE.md, ADRs)

Write for a human who wants to understand and use it quickly. Structure it however serves that. No filler.

## Challenge the spec

PROMPT.md is the brief, not gospel. If you see a better approach, or think part of it is wrong, say so. Put it in a **Proposed changes** section (in DESIGN.md, or in your report) with the reasons and trade-offs. Don't silently comply and don't silently deviate. The user approves or rejects at the next checkpoint; approved changes become ADRs and spec edits.
