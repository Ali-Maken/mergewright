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
| `KNOWN_GAPS.md` | Open items, **broken and unverified marked apart**. Closed rows: `docs/gaps/closed.md`. |
| `docs/field-notes.md` | What the user hit in real use. One line, dated, no ceremony. Feeds the next design pass. |
| `TESTING.md` "Times I left" | Every reach for the shell (ADR-0033). The only measurement of the promise. Conductor groups repeats at every checkpoint. |
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
- **Groups, not tickets** (ADR-0029, 2026-09-18). `planner-fable` writes **groups**: the unit one builder takes in one context, made of work that **shares files**, so the prefix is read once and later pieces ride a warm cache. Fine-grained steps live inside the group. **Decompose the work; don't decompose the context.** Splitting into more units does not save money — it duplicates the startup and saves nothing on turns.
- **The metric is turns, not tokens** (ADR-0029; the 80k token target is withdrawn). Cost ≈ turns × prefix. Rationale and the M2/M3 figures: `docs/rules-rationale.md`.
- **Size:** a group must fit in one context. A piece past **60 turns** commits and says **what made it long** (not the count — see Context). Compaction means the group was too big: helper says so, conductor splits the remainder.
- **What cuts turns:** a spec precise enough to shorten the write → test → fail → fix loop, and a scout instead of reading source into the builder's context. `planner-fable` sizes from the previous milestone's logged turn counts and causes.
- **Splitting a remainder:** conductor cuts along the seams in the builder's handoff note, keeps original scope/signatures/model. A split needing a new decision (signature, scope change, model) goes to `planner-fable` as a spec-only pass.
- **Each ticket states:** scope in/out; acceptance criteria; **exact sections** to read (never a whole document); **exact signatures** to expose/call; its `Build with` line.
- **Brownfield groups** (ADR-0035): `Build with` left blank with a recommendation, set by the conductor after the scout; sized to **one pane or one interaction**, not a feature area.
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
| Research, docs, summarising, scouting | unnamed agent — **pass `model:` explicitly** | Sonnet or Haiku |

**An agent with no definition file inherits the conductor's model (Opus).**

**Docs follow the context, not the model table.** Docs a ticket produces — MAP.md, ARCHITECTURE.md, ticket NOTES — are written by that ticket's own builder, whatever its model: the diff is already in its context and a handoff describing what changed costs more than the lines themselves. **Standalone** doc work with no warm context to ride — gap splits, TESTING.md, AUTHORSHIP.md, research write-ups, log transcription — goes to Sonnet, the conductor's own included. Never dispatch `general-purpose` or `Explore` without naming the model; an unpinned docs or research job costs Opus for Sonnet work.

- **Design pass (M2 onward):** `planner-fable` reads only the named PROMPT.md/DESIGN.md sections and crate MAP.md files — never source. **Brownfield group (touches existing code) → send a scout first** (ADR-0035: `Explore`, haiku/sonnet, ≤1 page per module: what exists, what state it holds, what it draws, which keys it claims). Greenfield keeps the old rule. Writes:
  1. `docs/design-notes/<milestone>.md`, two pages max: the milestone's promise (one line); interfaces/data shapes introduced; 2–3 hard-to-reverse choices with a recommendation each; where it differs from DESIGN.md and why; genuinely risky tickets; new UI DESIGN.md doesn't already specify (layout, per-breakpoint content, hierarchy, interaction), **as per-breakpoint mockups the user reads at the checkpoint where the design pass lands** (ADR-0035; no new stop — that checkpoint already exists).
  2. **Group specs**, straight into `.scratch/<milestone>/issues/` as unnumbered `<group-slug>.md` files: scope in/out, the pieces inside the group in order, exact signatures, acceptance criteria, Read-first sections, `Build with` line, `Needs:` (signatures called from other groups, for the conductor to derive order). Group by **shared files** (ADR-0029), so one builder reads the prefix once.

  **More than ~8 groups → two passes:** pass 1 writes the note + interface-first specs; pass 2 (fresh `planner-fable`, reads note + pass 1 specs, no source) writes the rest. No code — builders follow the note and spec.
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

- **Scouts:** builders delegate exploration to a Sonnet/Haiku scout (Agent tool, `Explore`, `model: haiku`/`sonnet`, summary ≤1 page) instead of reading source into their own context. **Every builder report says whether it used one, and if not, why** (ADR-0036 — the rule was ignored in 19 of 20 M3 runs; a report line is the gate).
- **MAP.md:** every crate keeps `crates/<crate>/MAP.md`, one line per public item. Any ticket changing a public item updates it. Builders read MAP.md before source. A crate without one gets it from the next ticket touching that crate.
- **Quiet gates** (user, 2026-09-15): builders run `cargo test --quiet`, `cargo clippy --all-targets --message-format=short -- -D warnings`, `cargo build --quiet`. Full output only on failure, only the failing part.
- **No turn count in the log before the harness reports it** (ADR-0035). The conductor logged a wrong figure three times in M3, each time by writing the entry before the notification arrived.
- **Measurement never runs in a helper's context** (user, 2026-09-15): perf runs, benchmarks, heavy-tier runs, memory curves run from a script writing to `docs/perf/<ticket>.txt`; helper reads summary lines only.
- **Turn budget** (ADR-0029; tightened by ADR-0035 after 14 of 20 M3 runs passed 60): past **60 turns on one piece**, commit at the next clean point and say **what made it long**. Past **150 turns on one group**, stop, write a handoff note and report `partial` — that is a gate, not advice. **Never report the count itself**; the conductor takes it from the harness.
- **Prefix is paid on every turn.** 60k of junk in the opening context is not read once, it is re-read once per turn. Read the named sections, not the documents.
- **Builders never read `PROGRESS.md` or `KNOWN_GAPS.md`** (housekeeping/01, 2026-09-18): those belong to the conductor. A builder's findings go in its ticket.

### Checkpoint policy

**One stop per milestone, every milestone** (ADR-0029, user 2026-09-18: "for me to check after every 2 or 3 turns shouldn't be there, of course when I need to see, I can ask"). Write-milestone ticket approval is withdrawn. The gates don't change — they're the conductor's to run, not the user's to watch. State lives in files; the user reads `docs/log/<milestone>.md` whenever they want in.

**The conductor stops unasked for exactly four things:**

1. **A visual result.** Not deference — incapacity. No builder or reviewer ticks "looks right", and the capture pipeline can't prove colour (below).
2. **Work that can't be put back** whose inverse is unproven. Never self-certified.
3. **A product preference it can't infer** — batched into one list at the checkpoint, not trickled.
4. **A spec change or scope cut.**

Everything else the conductor decides: grouping, order, models, splits, escalation, what counts as done against PROMPT.md and these rules.

- **Reviews: one round per group.** Reviewer reports to the conductor; remaining findings go to `KNOWN_GAPS.md` with an expiry milestone. A second round only for a surviving Critical.
- **Checkpoint reports:** 15 lines max, telegraphic; which model ran each group, what compacted, and **turns per group plus the milestone total** (ADR-0029's test: M3's mean must be well under M2's 119, or the diagnosis was wrong).

## Rules for every agent

- **Never mark stubbed or half-done work as done.** List it in `KNOWN_GAPS.md`.
- **Acceptance ticks are a gate** (ADR-0035): every tick **names its verifying test inline, at tick time** — a tick with no test named is not a tick. The reviewer spot-checks a sample per group and reports how many it checked and how many held. **A false tick is a Critical**, whatever its size.
- **Rust code is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass.
- **Commit** after each ticket. Never push or force.
- **Never stage or commit while another agent is working in the same worktree.** `git add` writes the shared `.git/index`, so whoever commits next takes whatever is staged there — in both directions. Observed twice on 2026-09-19: a conductor `git add -A` swept a builder's `undo.rs` into a rules commit, then a builder's commit swept a conductor script into `m3-06-fixes`. The conductor queues its edits and commits **between** helpers (it already batches ceremony per group, ADR-0029); concurrent builders each get their own worktree (ADR-0036) — the conductor is a concurrent writer too.
- **Ask the user** (through the conductor) before changing the spec or cutting scope. Record agreed changes as ADRs.
- **Architecture and readability are hard rules** (PROMPT.md 6.1, 6.2). Read `ARCHITECTURE.md` before adding/moving modules, use `codebase-design` when you do, update it if structure changes. Never add a dependency that breaks the crate rules.
- **Keep your context lean:** read what you need, not everything.
- **Context window:** `CLAUDE_CODE_AUTO_COMPACT_WINDOW` is `200000` in `.claude/settings.json`. The cap is headroom, not an allowance.
- **If you compact mid-ticket, say so in your report.** Conductor logs it in the session log and splits the remainder into new tickets.
- **Fix the cause, not the symptom.** No workaround, retry loop, sleep, hardcoded special case or disabled test without the user's OK. Root cause out of scope → stop, say what it is, ask. **Every fix report says which it is — cause or symptom — and what it costs the user.**
- **Never swallow an error.** No ignored `Result`, `let _ =`, `unwrap_or_default()` standing in for a failure, empty `Err(_) => {}`, or catch-all that logs and carries on. Handle, propagate or surface every error. Must genuinely ignore one → say in a comment why it's safe.
- **A status field must never read as an error when nothing is wrong.** Name/render status by its state (`gfx ok` / `gfx: <error>`), or show the field only when there's something to report.
- **Green gates are not evidence** (rules-rationale has why). Say what you actually verified and how, and what you couldn't. Never claim behaviour you didn't observe.
- **Visual results are judged by eye, by the user.** Green gates aren't evidence for how something looks. Any ticket with a visual result ends with a pty capture committed under `docs/shots/`; report names the capture, says the user judges it by eye. No builder or reviewer ticks "looks right".
- **Captures can't prove colour** (ADR-0029): the pty pipeline records character grids with no colour attributes. Colour work ships **unverified** and says so — never claim a capture verified it.
- **Unverified is not broken.** `KNOWN_GAPS.md` marks both, apart.
- **The user's daily use is a test tier, not a fallback** (ADR-0029), starting the day M3 lands (ADR-0030). What it finds goes in `docs/field-notes.md` the day it's noticed. Why it's a tier: `docs/rules-rationale.md`.
- **Captures stay small**: `docs/shots/` trimmed to the point, **hard cap 200 KB per file**. Raw terminal streams never committed — summarise, keep the raw one untracked under `target/`. Repo may be published.
- **Tests must be laptop-safe** (PROMPT.md 7.1). Builders run the fast tier only. Heavy tier, fuzzing and benchmarks run in milestone reviews. **Every milestone review includes the release perf run** — frame budgets fail only under `--release`; debug records without failing (ADR-0021).
- **Concurrent builders are isolated, not serialised** (ADR-0036, replaces "one heavy job at a time"). Each concurrent builder gets its own git worktree, its own `CARGO_TARGET_DIR` outside the shared `target/`, and a **disjoint crate set**. Never run two cargo commands inside one worktree. Never generate large fixtures outside the heavy tier.
- **A builder never edits outside its crate set.** Needs a crate another holds → commit at the seam, report `blocked: needs <crate>` naming the signature it wants, stop. The conductor merges the holder's branch and resumes it, or serialises the remainder. Never wait in place.

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
