# context-diet: split the big docs so a helper doesn't pay for a book to read a page

Status: ready
Blocked by: none (runs before the next build ticket)
Build with: Sonnet — file moves, index writing and pointer updates; every target is named below. No code, no behaviour change.

Filed by the conductor from the user's instruction, 2026-09-18. **Scope is the user's, verbatim below; don't widen it.**

## Why

Helpers start at 50–70k tokens before doing any work. Measured causes (bytes, today):

| File | Size | What happens |
|---|---|---|
| `CLAUDE.md` | 17,579 | auto-loaded into **every** helper, including the cheap ones |
| `DESIGN.md` | 105,844 | read whole to reach one section |
| `PROGRESS.md` | 133,474 | whole-file reads for one log line |
| `KNOWN_GAPS.md` | 94,580 | mostly closed rows |
| `docs/design-notes/m2-details-and-diffs.md` | 37,210 | read whole for one bullet |

Fix the files, not the tickets — the tickets already name their sections.

## Scope

1. **Split `DESIGN.md`** into one file per section under `docs/design/`, and split the per-milestone design notes (`docs/design-notes/*.md`) the same way. Update every pointer in tickets, ADRs and `CLAUDE.md` so each cites a **file**, not a section of a book.
2. **Archive `PROGRESS.md`:** finished milestones move to `docs/log/m0.md` and `docs/log/m1.md`. `PROGRESS.md` keeps the plan, the current step and the **current milestone's log only**. Target: under 10 KB.
3. **Split `KNOWN_GAPS.md`:** closed rows to `docs/gaps/closed.md`; open rows stay in the live file.
4. **Trim `CLAUDE.md` to the rules themselves.** The reasoning moves to `docs/rules-rationale.md`, which nothing auto-loads. Every rule keeps its normative content and its date/attribution; what leaves is the explanation of why. This is the one paid on every helper start.
5. **Add a rule to `CLAUDE.md`:** builders never read `PROGRESS.md` or `KNOWN_GAPS.md` — those belong to the conductor. A builder's findings go in its ticket.

**Splitting must not lose the cross-references a whole-file read gave for free:**
- every split file gets a short **"see also"** header naming its neighbours;
- `docs/design/README.md` lists all sections in order (same for the design-notes split and `docs/log/`);
- **nothing is deleted** — content moves, and the index says where.

6. **Measurement, so we can tell whether it worked:** add `scripts/context-cost.sh`, which prints the byte and estimated-token cost of what a helper pays before doing any work — `CLAUDE.md` plus the agent definition in `.claude/agents/`, plus any file named on the command line — and writes the result to `docs/perf/context-cost.txt`. Run it before and after your change (the "before" numbers are the table above) and put both in the file.

Out:
- Rewording any rule's meaning. Moving reasoning out is not licence to change what a rule requires.
- `TESTING.md` (57 KB) and `ARCHITECTURE.md` (14 KB) — leave them; ARCHITECTURE.md has a pending review finding (M7) owned by another ticket.
- Any source change.

## Acceptance

- `wc -c CLAUDE.md PROGRESS.md KNOWN_GAPS.md` all materially smaller; PROGRESS.md under 10 KB.
- No pointer anywhere in `.scratch/`, `docs/adr/`, `CLAUDE.md` or the agent definitions resolves to a file or heading that no longer exists. Prove it: grep for the old paths and show nothing dangling.
- `docs/design/README.md` (and the sibling indexes) list every split file, in order.
- `cargo test --quiet` still green (docs shouldn't break it; if something reads these paths, that's the proof).
- `docs/perf/context-cost.txt` holds before and after numbers.
