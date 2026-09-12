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
- **Who builds what** (cost control, set 2026-09-12). The routing table in PROGRESS.md is authoritative; the rule is:
  - **Fable** only for the M1 graph layout engine tickets and the `debugger` agent. Nothing else.
  - **Opus** builds everything else, including M1's other tickets. Sonnet is no longer used for building.
  - **Reviews** run on Opus, except M1, M3 and M5, which stay Fable.
  - **Effort:** `builder-opus` is `high`. The conductor dispatches `builder-opus-xhigh` only for undo (M3), conflicts (M6), interactive rebase (M7) and the M1 graph layout engine.
- **The one exception is step 1** (`/grill-with-docs`), which runs in the main session because it needs the user.
- Do only the next unchecked step. Never start a new milestone without the user's OK at a checkpoint.

## Rules for every agent

- **Never mark stubbed or half-done work as done.** List it in `KNOWN_GAPS.md`.
- **Rust code is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
- **Commit** after each ticket. Never push or force.
- **Ask the user** (through the conductor) before changing the spec or cutting scope. Record agreed changes as ADRs.
- **Architecture and readability are hard rules** (PROMPT.md 6.1 and 6.2). Read `ARCHITECTURE.md` before adding or moving modules, use the `codebase-design` skill when you do, and update `ARCHITECTURE.md` if the structure changes. Never add a dependency that breaks the crate rules.
- **Keep your context lean:** read what you need, not everything.
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
