# Graft: rules for Claude

Why behind every rule here: [`docs/rules-rationale.md`](docs/rules-rationale.md) — not auto-loaded, read it when a rule seems wrong.

"Graft" is a **codename**. The name is taken by another Git GUI; the real one is chosen in M8. Never publish, register or tag anything public under it.

## The seven that pay

Every rule below earned its place by catching a real failure. They are dispositions, checkable in a report.

1. **Green gates are not evidence.** Say what you actually verified and how, and what you couldn't. Never claim behaviour you didn't observe.
2. **Fix the cause, not the symptom.** No workaround, retry loop, sleep, hardcoded special case or disabled test without the user's OK. Root cause out of scope → stop, say what it is, ask. Every fix report says which it is — cause or symptom — and what it costs the user.
3. **Never swallow an error.** No ignored `Result`, `let _ =`, `unwrap_or_default()` standing in for a failure, empty `Err(_) => {}`, or catch-all that logs and carries on. Must genuinely ignore one → comment why it's safe.
4. **Never mark stubbed or half-done work done.** It goes in `KNOWN_GAPS.md`, where **broken and unverified are marked apart**. Unverified is not broken.
5. **Captures can't prove colour.** The pty pipeline records character grids with no colour attributes. Colour ships **unverified** and says so.
6. **Visual results are judged by eye, by the user.** Any visual ticket ends with a capture under `docs/shots/`. No builder or reviewer ticks "looks right".
7. **A behavioural claim carries its line, at filing time.** A finding that asserts wrong logic, wrong order or wrong behaviour names the file and line that proves it when it is filed. No line, not a defect — it is filed as a **question**. This is rule 1 at the filing step: a capture shows output, and output alone cannot tell wrong logic from correct logic under an input you assumed. It binds both sides — **the user files with the line or as a question, and the conductor does not open a ticket on a behavioural claim that arrives without one, the user's included.** Textual findings still stand on a capture alone: wording drift, a stale caption, a number disagreeing with the picture, two panes with no gap between them.

   **It binds the reader as much as the filer.** A report that says *could not confirm* is a **stop**, not a premise to reason forward from. Reasoning conditionally — "*if* X, then the cause is Y" — and then instructing unconditionally is how an unverified premise gets promoted with nobody deciding to promote it. Verify it, or ask. Twice now the repo was open and neither side looked.

**The gate that makes them stick:** every acceptance tick **names its verifying test inline, at tick time**. A tick with no test named is not a tick. The reviewer spot-checks a sample per group and says how many it checked and how many held. **A false tick is Critical**, whatever its size.

**A status field must never read as an error when nothing is wrong.** Render status by its state (`gfx ok` / `gfx: <error>`), or show it only when there's something to report.

## Where things live

| File | Purpose |
|---|---|
| `PROMPT.md` | Product spec (note above first `---`: for humans). |
| `DESIGN.md` | Approved design; index into `docs/design/`. |
| `CONTEXT.md` | Glossary. Use its terms everywhere. |
| `docs/adr/` | Decision records. **A decision, not an autopsy** — no ADR per incident. |
| `PROGRESS.md` | Plan + current step. Milestone log: `docs/log/<milestone>.md`. |
| Tickets | `.scratch/<milestone>/issues/NN-<slug>.md`. See `docs/agents/issue-tracker.md`. |
| `KNOWN_GAPS.md` | Open items. Closed rows: `docs/gaps/closed.md`. |
| `docs/field-notes.md` | What the user hit in real use. One line, dated. Feeds the next design pass. |
| `TESTING.md` "Times I left" | Every reach for the shell (ADR-0033). The only measurement of the promise. |
| `docs/reviews/`, `docs/design-notes/` | Milestone reviews; one design note per milestone. |
| `docs/AUTHORSHIP.md` | Regenerated each checkpoint from `docs/log/*.md`. Facts from the log only. |

## Which model, and who decides

Your ticket's `Build with` line names your model; follow it. Model routing,
escalation and checkpoint policy are the conductor's — they live in
`.claude/skills/conduct/SKILL.md`. Two rules bind everyone:

- **An agent with no definition file inherits the conductor's model (Opus).** Never dispatch `general-purpose` or `Explore` without passing `model:` explicitly. Research, standalone docs, summarising and scouting → Sonnet or Haiku.
- **Docs follow the context, not the model table.** Docs a ticket produces — MAP.md, ARCHITECTURE.md, ticket NOTES — are written by that ticket's own builder, whatever its model. **Standalone** doc work goes to Sonnet.

## Gates

- **Rust is done only when** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` and `cargo test --doc` all pass. Run them quiet; full output only on the failing part. **Run them workspace-wide, never `-p`** — a crate-scoped gate hid two app-crate breaks in M4 group 03. (nextest does not run doctests — hence the second command. Measured warm-vs-warm 2026-09-19, 1636 tests: 70.1 s → 50.5 s.)
- **Tests must be laptop-safe** (PROMPT.md 7.1). Builders run the fast tier only. Heavy tier, fuzzing and benchmarks run in milestone reviews. **Every milestone review includes the release perf run** — frame budgets fail only under `--release` (ADR-0021).
- **Measurement never runs in a helper's context.** Perf runs, benchmarks, memory curves run from a script writing to `docs/perf/<ticket>.txt`; the helper reads summary lines.
- **Captures stay small:** `docs/shots/` trimmed to the point, **hard cap 200 KB per file**. Raw terminal streams never committed. The repo may be published.
- **Commit** after each ticket. **Never push or force.**

## Running work

- User runs `/conduct` on **Opus**. Every piece of work goes to a fresh helper in `.claude/agents/`. Step 1 (`/grill-with-docs`) is the exception — it runs in the main session.
- **Groups, not tickets** (ADR-0029). A group is what one builder takes in one context, made of work that **shares files**. Decompose the work; don't decompose the context.
- **Brownfield groups** (ADR-0035) — anything touching existing code — get a **scout first** (`Explore`, haiku/sonnet, ≤1 page: what exists, what state it holds, what it draws, which keys it claims). Sized to one pane or one interaction. `Build with` left blank, set by the conductor after the scout.
- **Every builder report says whether it used a scout, and if not, why.**
- **Builders run concurrently, isolated** (ADR-0036). Each gets its own git worktree, its own `CARGO_TARGET_DIR` outside the shared `target/`, and a **disjoint crate set**. Never two cargo commands in one worktree.
- **A builder never edits outside its crate set.** Needs a crate another holds → commit at the seam, report `blocked: needs <crate>` naming the signature, stop. Never wait in place.
- **Reviews run concurrently with the next group's build.** The reviewer is read-only against a fixed hash. Same rounds, overlapped — never fewer.
- **Never stage or commit while another agent works the same worktree.** `git add` writes the shared index; the next commit from either side takes it. The conductor commits between helpers.
- **If you compact mid-ticket, say so.** The conductor splits the remainder.
- **Builders never read `PROGRESS.md` or `KNOWN_GAPS.md`.** A builder's findings go in its ticket.
- **A milestone's captures ship as one contact sheet** (ADR-0037): one file concatenating every capture in ticket order, each under its "what right looks like" line. The per-scenario files stay; the sheet is assembled from them. The user reviews by scrolling once.
- **Every milestone ends with an acceptance ticket:** one command walking the scenarios, a `TESTING.md` checklist saying concretely what right looks like per scenario, captures under `docs/shots/`. Promises — M1 "the shape of history is obvious at a glance"; M3 "you cannot lose work"; M5 "you see the result before you confirm"; M6 "a real conflict is resolvable without leaving the app".
- **The user's daily use is a test tier, not a fallback**, from the day M3 lands. What it finds goes in `docs/field-notes.md` the same day.
- **Architecture and readability are hard rules** (PROMPT.md 6.1, 6.2). Read `ARCHITECTURE.md` before adding or moving modules; use `codebase-design` when you do. Never add a dependency that breaks the crate rules.
- **Shared namespaces are decided once, in the design, not per milestone.** Any group that binds a key reads `docs/design/02-keymap.md` first; it allocates keys for milestones not yet built. A key it does not name is **not free** — the code showing it unbound means nothing. Wanting a key the doc doesn't give you is a spec change, through the conductor, not a build decision. Same for what counts as an operation and what is undoable.
- **A scout reports code, not intent.** "Nothing uses this" is not "this is available" — check the design doc before treating anything as unclaimed.
- **Keep your context lean:** read the named sections, not the documents.

## Skills

Matt Pocock plugin skills are **banned by default**. Allowed, only for what the row says:

| Skill | Allowed for |
|---|---|
| `implement` | Building a ticket. |
| `tdd` | Test-first work inside a ticket. |
| `code-review` | Reviewing a finished ticket or milestone. |
| `diagnosing-bugs` | **`debugger` only.** Not builders. |
| `codebase-design` | Adding or moving modules. |
| `handoff` | Mid-ticket handover. |
| `improve-codebase-architecture` | After M2, M5 and M7 only. |

**Every other skill in that plugin is banned**, `prototype` and `wayfinder` included. Think you need one: **stop and ask the user**. Don't work around it silently. Keep `prototype`'s principles — throwaway, minimal, findings as an ADR — not the skill.

Some allowed skills aren't in the Skill tool's listing. Read the SKILL.md directly:
`~/.claude/plugins/cache/claude-plugins-official/mattpocock-skills/1.2.3/skills/engineering/<name>/SKILL.md`

## Talking to the user

Clear and concise over grammar. Telegraphic.

- Fragments fine. No filler, preamble, recap or politeness padding.
- Lead with the result or decision. Then what the user must do.
- Bullets, one line each. Exact numbers, paths, commands.
- Risks and unknowns stated plainly, not softened.
- Questions: one line + short options.

```
M1 done. Graph renders, 3 tiers. Pixel ok in Ghostty.
Try: cargo run -- ~/code/api   (j/k scroll, / search)
Gaps: hover tooltip → M2
Next: M2, 9 tickets (.scratch/m2/). Approve? yes / change
```

Design docs and code comments: write for a human who wants to understand it quickly. No filler.

## Challenge the spec

PROMPT.md is the brief, not gospel. See a better approach, or think part of it's wrong: say so, in a **Proposed changes** section with reasons and trade-offs. Don't silently comply, don't silently deviate. Approved changes become ADRs and spec edits.
