# Graft: rules for Claude

Why behind these rules: [`docs/rules-rationale.md`](docs/rules-rationale.md)
(not auto-loaded). Every rule here keeps its normative text, date and
attribution — only the "why" moved out.

"Graft" is only a **codename**: the name is taken by another Git GUI. Real
name chosen in M8. Don't publish, register or tag anything public under it.

## Where things live

| File | Purpose |
|---|---|
| `PROMPT.md` | Product spec (note above first `---`: for humans). |
| `DESIGN.md` | Approved design; index into `docs/design/`. |
| `CONTEXT.md` | Glossary from `/grill-with-docs`. Use its terms everywhere. |
| ADRs | Decision records, folder `docs/agents/domain.md` names (default `docs/adr/`). **PROMPT.md's `DECISIONS.md` → write an ADR instead.** |
| `PROGRESS.md` | Plan + current step. Current milestone's log: `docs/log/<milestone>.md`, linked from here — don't inline it back. |
| Tickets | Local markdown, per `docs/agents/issue-tracker.md`. |
| `KNOWN_GAPS.md` | Open items. Closed rows: `docs/gaps/closed.md`. |
| `docs/reviews/` | Milestone reviews. |
| `docs/design-notes/` | One note per milestone, by `planner-fable`. |
| `docs/AUTHORSHIP.md` | Who designed/specified/built/reviewed each milestone. From the session logs; conductor regenerates every checkpoint; facts from the log only. |

## How work runs

- User runs `/conduct` in a main session on **Opus**. Conductor hands every piece of work to a fresh helper in `.claude/agents/`, stops at checkpoints.
- **Who designs/specifies/builds/reviews; ticket size, context, checkpoints:** see **Standing rules** below (set 2026-09-13, every remaining milestone). Replaces the 2026-09-12 cost-control routing.
  `xhigh` not automatic: conductor dispatches `builder-opus-xhigh` only when the user asks for it on a specific ticket.
- **Exception: step 1** (`/grill-with-docs`) runs in the main session — needs the user.
- Do only the next unchecked step. Never start a new milestone without the user's OK at a checkpoint.

## Standing rules (every remaining milestone, set 2026-09-13; Fable's share raised for M2 onward)

**M1 is grandfathered:** its tickets, `Build with` lines and Opus review stay as-is. Everything marked "M2 onward" applies from M2.

### Tickets

- **Who writes them (M2 onward):** `planner-fable` writes every ticket spec in its design pass (see Models). Conductor **files and sequences**: numbering, dependency order, `Blocked by:`, `Status:`. It does not re-decide scope, signatures or `Build with`. A spec the conductor thinks wrong goes back to `planner-fable` or the user; conductor doesn't edit it.
- **`/to-tickets` not used for any milestone with a Fable spec pass** (M2 onward). Drafted M0 and M1 only.
- **Size:** each ticket finishes inside one context, target **under 80k tokens**. Compaction means the ticket was too big: helper says so in its report, conductor splits the remainder into new tickets.
- **Size from evidence, not intuition** (user, 2026-09-15; rules-rationale has M1's numbers). `planner-fable` reads the M1 session log's compaction/overrun entries first, says what caused them, sizes the milestone's tickets from that. A ticket that can't be made to fit says so when written, not mid-build.
- **Splitting a remainder:** conductor cuts along the seams in the builder's handoff note, keeps original scope/signatures/model. A split needing a new decision (signature, scope change, model) goes to `planner-fable` as a spec-only pass.
- **Each ticket states:** scope in/out; acceptance criteria; **exact sections** to read (never a whole document); **exact signatures** the work must expose/call; its `Build with` line.
- **Interface-first grouping:** first ticket of a group defines traits/stubs; rest fill them in.
- **Every milestone ends with an acceptance ticket** (set 2026-09-13, from M1 ticket 35): builds the means to judge the milestone's **promise**, not just tests — one command walking the scenarios, a `TESTING.md` checklist saying concretely what right looks like per scenario (never "looks correct"), captures under `docs/shots/`. Not done till this ticket is. `builder-fable`; last in order, before review. Captures go straight to files — builder reports paths, not contents.
  - **Promises:** M1 "the shape of history is obvious at a glance"; M3 "you cannot lose work"; M5 "you see the result before you confirm"; M6 "a real conflict is resolvable without leaving the app". `planner-fable` names the rest in their design notes.

### Models

**Fable decides, specifies and reviews; Opus conducts and builds; Sonnet builds mechanical work.**

| Work | Who | Model, effort |
|---|---|---|
| **Design pass + ticket specs**, start of every milestone | `planner-fable` | Fable, high |
| Filing/sequencing specs (numbers, order, blocked-by) | conductor | Opus |
| **Signature tickets** (list below) | `builder-fable` | Fable, high |
| **Acceptance ticket**, last of every milestone | `builder-fable` | Fable, high |
| Ticket passing all three Fable tests (below) | `builder-fable` | Fable, high |
| Concurrency, terminal protocols, git writes, undo, conflicts | `builder-opus` | Opus, high |
| Mechanical: wiring, config, docs, plain widgets, tests, renames | `builder-sonnet` | Sonnet, medium |
| Reviews, **every milestone from M2** (M1: Opus) | `reviewer` | Fable, high |
| **M8 architecture pass** (MAP.md + public interfaces, never source) | `planner-fable` | Fable, high |
| Debugging | `debugger` | Fable |

- **Design pass (M2 onward):** `planner-fable` reads only the relevant PROMPT.md/DESIGN.md sections and crate MAP.md files — never source — and writes:
  1. `docs/design-notes/<milestone>.md`, two pages max: the milestone's promise (one line); interfaces/data shapes introduced; 2–3 hard-to-reverse choices with a recommendation each; where it differs from DESIGN.md and why; genuinely risky tickets; new UI DESIGN.md doesn't already specify (layout, per-breakpoint content, hierarchy, interaction).
  2. **Ticket specs**, straight into `.scratch/<milestone>/issues/` as unnumbered `<ticket-slug>.md` files: scope in/out, exact signatures, acceptance criteria, Read-first sections, `Build with` line, `Needs:` (signatures called from other tickets, for the conductor to derive order).

  **More than ~12 tickets → two passes:** pass 1 writes the note + interface-first specs; pass 2 (fresh `planner-fable`, reads note + pass 1 specs, no source) writes the rest. No code — builders follow the note and spec.
- **Signature tickets — Fable builds them regardless of the three tests**, because the code is the idea:
  - M2: Mistake Radar detection
  - M3: undo journal's per-kind inverse
  - M5: Ghost Preview planning
  - M6: conflict and operation state machine

  Spec marks them `Build with: Fable (signature)`.
- **Any other ticket goes to Fable only when all three hold:** 1) doesn't decide the approach; 2) a wrong choice becomes structural for later tickets; 3) correctness can't be judged from diff or tests.
- **Expect one or two Fable builds per milestone, signature tickets included.** Three-plus → conductor flags it (rules-rationale has why). Escalation redos don't count; acceptance ticket doesn't count either (standing, like the M8 pass).
- **Escalation beats prediction:** Sonnet ticket fails twice → Opus. Opus ticket stalls twice (`failed`/`partial` without progress), or review finds its design wrong → redo on `builder-fable`. Fable fails too → `debugger`.
- Each ticket's `Build with` line names its model; conductor follows it.
- **Reviewer efficiency (M2 onward):** conductor logs `M<n> start <hash>` in the session log at the milestone's first ticket. Reviewer reads, in order: `git diff --stat <start>..HEAD`, crate MAP.md files, then the diff itself (`git diff <start>..HEAD`, crate by crate). Opens a whole source file only to confirm a finding. Never reads the whole codebase; only the PROMPT.md/DESIGN.md sections the milestone covers.
- **M8 architecture pass:** standing ticket `.scratch/m8-polish-and-ship/issues/fable-architecture-pass.md`. `planner-fable` reads every `crates/*/MAP.md` and ARCHITECTURE.md, never source, writes `docs/reviews/m8-architecture.md`. A review, not a build — doesn't count against the Fable build expectation. M8's spec pass files its other tickets around it.

### Context

- **Scouts:** builders delegate exploration to a Sonnet/Haiku scout (Agent tool, `Explore`, `model: haiku`/`sonnet`) returning a summary ≤1 page, instead of reading source into their own context.
- **MAP.md:** every crate keeps `crates/<crate>/MAP.md`, one line per public item. Any ticket changing a public item updates it. Builders read MAP.md before source. A crate without one gets it from the next ticket touching that crate.
- **Quiet gates** (user, 2026-09-15): builders run `cargo test --quiet`, `cargo clippy --all-targets --message-format=short -- -D warnings`, `cargo build --quiet`. Full output only on failure, only the failing part.
- **Measurement never runs in a helper's context** (user, 2026-09-15): perf runs, benchmarks, heavy-tier runs, memory curves run from a script writing to `docs/perf/<ticket>.txt`; helper reads summary lines only.
- **Hard stop at 40 tool calls or ~100k tokens** (user, 2026-09-15): helper commits at the next clean point, writes a handoff note in the ticket, reports `partial`; conductor splits the rest. 80k sizing target stands.
- **Builders never read `PROGRESS.md` or `KNOWN_GAPS.md`** (housekeeping/01, 2026-09-18): those belong to the conductor. A builder's findings go in its ticket.

### Checkpoint policy

- **Read-only milestones (M1, M2, M7, M8):** build straight through, no ticket approval. Stop only if a Critical survives review, or at the end.
- **Write milestones (M3, M4, M5, M6):** full gates and ticket approval.
- **Checkpoint reports:** 15 lines max, telegraphic; say which model ran each ticket and whether anything compacted.

## Rules for every agent

- **Never mark stubbed or half-done work as done.** List it in `KNOWN_GAPS.md`.
- **Rust code is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
- **Commit** after each ticket. Never push or force.
- **Ask the user** (through the conductor) before changing the spec or cutting scope. Record agreed changes as ADRs.
- **Architecture and readability are hard rules** (PROMPT.md 6.1, 6.2). Read `ARCHITECTURE.md` before adding/moving modules, use the `codebase-design` skill when you do, update `ARCHITECTURE.md` if structure changes. Never add a dependency that breaks the crate rules.
- **Keep your context lean:** read what you need, not everything.
- **Context window:** `CLAUDE_CODE_AUTO_COMPACT_WINDOW` is `200000` in `.claude/settings.json`, raised from 150000 on 2026-09-12 (rules-rationale has why). Lean context still matters — the cap is headroom, not an allowance.
- **If you compact mid-ticket, say so in your report.** Conductor logs it in the session log and splits the remainder into new tickets.
- **Fix the cause, not the symptom.** Find out why before changing anything. No workaround, retry loop, sleep, hardcoded special case or disabled test without the user's OK. Root cause out of scope → stop, say what it is, ask; don't route around it and log a gap. **Every fix report says which it is — cause or symptom — and what it costs the user** (user, 2026-09-13).
- **Never swallow an error.** No ignored `Result`, no `let _ =`, no `unwrap_or_default()` standing in for a real failure, no empty `Err(_) => {}` arm, no catch-all that logs and carries on. Every error handled, propagated or surfaced to the user. Must ignore one genuinely → say in a comment why it's safe.
- **A status field must never read as an error when nothing is wrong.** Name/render status by its state (`gfx ok` / `gfx: <error>`), or show the field only when there's something to report.
- **Green gates are not evidence** (rules-rationale has why). Say what you actually verified and how, and what you couldn't. Never claim behaviour you didn't observe.
- **Visual results are judged by eye, by the user.** Green gates aren't evidence for how something looks. Any ticket with a visual result ends with a pty capture committed under `docs/shots/`; report names the capture, says the user judges it by eye. No builder or reviewer ticks "looks right".
- **Captures stay small** (user, 2026-09-15): anything under `docs/shots/` trimmed to what shows the point, **hard cap 200 KB per file**. Raw terminal streams never committed: summarise in the committed file, keep the raw one untracked under `target/`. Repo may be published.
- **Tests must be laptop-safe** (PROMPT.md 7.1). Builders run only the fast tier (`cargo test`). Heavy tier, fuzzing, benchmarks run only in milestone reviews. **Every milestone review includes the release perf run** (frame budgets fail only under `--release`; debug records without failing — user, 2026-09-13, ADR-0021).
- **One heavy job at a time:** never run two cargo commands or test suites at once. Never generate large fixtures outside the heavy tier.

## Agent skills

### Which skills to use

Matt Pocock plugin skills are **banned by default**. Only these allowed, only for what "When" says:

| Skill | Allowed for |
|---|---|
| `to-tickets` | M0 and M1 only (done). Not used from M2: `planner-fable` writes ticket specs. |
| `implement` | Building a ticket. |
| `tdd` | Test-first work inside a ticket. |
| `code-review` | Reviewing a finished ticket or milestone. |
| `diagnosing-bugs` | **`debugger` agent only.** Not builders. |
| `codebase-design` | Module changes only (adding/moving modules). |
| `handoff` | Mid-ticket handover only. |
| `improve-codebase-architecture` | After M2, M5 and M7 only. |

**Every other skill in that plugin is banned**, `prototype` and `wayfinder` included.

Think you need a banned skill: **stop and ask the user** (through the conductor). Don't use it, don't work around it silently.

`prototype` and `wayfinder` keep getting reached for anyway and stay banned (rules-rationale has why). Keep `prototype`'s principles — throwaway, minimal, findings as an ADR — not the skill.

Some allowed skills aren't in the Skill tool's listing. They're installed; read the SKILL.md directly:
`~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/<name>/SKILL.md`

### Issue tracker

Local markdown: tickets live as files under `.scratch/<milestone-slug>/issues/NN-<slug>.md`. No remote. See `docs/agents/issue-tracker.md`.

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

PROMPT.md is the brief, not gospel. See a better approach, or think part of it's wrong: say so. Put it in a **Proposed changes** section (DESIGN.md, or your report) with reasons and trade-offs. Don't silently comply, don't silently deviate. User approves/rejects at the next checkpoint; approved changes become ADRs and spec edits.
