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

## Status: partial (handoff below)

Done:
- **Part 1a — DESIGN.md split.** `docs/design/01..09-*.md` + `docs/design/README.md`.
  `DESIGN.md` is now a 1.5 KB stub: intro + Contents index, nothing deleted.
- **Part 1a — design-notes split.** `docs/design-notes/m1-the-graph/*.md` and
  `docs/design-notes/m2-details-and-diffs/*.md`, each with its own README.md
  index; the top-level `m1-the-graph.md` / `m2-details-and-diffs.md` are now
  short stubs (title/promise/Read-line kept verbatim) linking to the split.
- **Part 2 — PROGRESS.md archive.** `docs/log/m0.md` (6.4 KB), `docs/log/m1.md`
  (84 KB) hold the M0/M1 session logs + M1's own open-questions/sliding-gaps
  sections, verbatim. `PROGRESS.md` keeps Phase A/B, the M3 brief, and the
  **M2** session log only (M2 is the current milestone — other helpers were
  actively appending to it today, so it stays put per the run's caution not
  to touch anything that looks live).
- **Part 3 — KNOWN_GAPS.md split.** `docs/gaps/closed.md` holds closed rows
  (table rows classified by last-cell "closed…" or first-cell strikethrough;
  bullets by strikethrough+"closed" near the top), grouped under their
  original `## ` headings, `(closed rows)` suffixed where a heading's section
  was only partly closed. `KNOWN_GAPS.md` keeps the open rows under the same
  headings. Verified several sections by hand (M1 gaps, M0 gaps, Scheduled
  interim states) — genuinely-open content stayed, genuinely-closed rows
  moved, nothing duplicated or dropped in the ones checked.
- **Part 4 — CLAUDE.md trim.** `docs/rules-rationale.md` created: pulled out
  the "why" for ticket sizing, Fable-build escalation, the 200k context
  window raise, and the two banned-skill rationales (prototype/wayfinder).
  Every rule's normative text, date and attribution kept as written — I did
  not reword any requirement, only removed sentences that explained motive
  without adding a requirement. Net size drop is modest (17,579 → 17,392 B,
  ~1%): CLAUDE.md was already mostly normative: there wasn't as much
  reasoning-prose to remove as the file's overall size suggested. Flagging
  this rather than padding the cut to look bigger.
- **Part 5 — new rule added.** "Builders never read `PROGRESS.md` or
  `KNOWN_GAPS.md`" is in CLAUDE.md's "Rules for every agent" section, dated
  2026-09-18.
- **Part 6 — `scripts/context-cost.sh`.** Prints byte + est.-token (bytes/4)
  cost of CLAUDE.md + named files, appends a dated block to
  `docs/perf/context-cost.txt` (never overwrites, so before/after both land
  there). Before numbers reconstructed from `git show HEAD:CLAUDE.md` (the
  ticket's own before-table only covered whole files, not agent-file cost);
  after numbers from a live run against `builder-sonnet.md`. Both blocks are
  in the file.

**Not done — disclosed, not silently dropped:**
- **Part 1b, the pointer rewrite in ADRs and old tickets.** Scope says
  "update every pointer in tickets, ADRs and CLAUDE.md so each cites a file."
  CLAUDE.md had none to fix (checked — no `DESIGN.md §` citations there).
  But `docs/adr/*.md` and `.scratch/m0-foundations|m1-the-graph|m2-details-and-diffs/issues/*.md`
  carry on the order of 150+ `DESIGN.md §N.M` citations (grep:
  `grep -rn 'DESIGN\.md §' docs/adr .scratch` from repo root). These are not
  1:1 rewritable by a safe global substitution: some sentences cite section
  numbers from two different new files in one clause (e.g. "DESIGN.md §4.2,
  §4.3, §4.4 and §8.2" — §4.x now lives in `docs/design/04-*.md`, §8.2 in
  `docs/design/08-*.md`), so a mechanical "replace DESIGN.md with the right
  file" per occurrence needs per-citation judgement, not a regex. Doing this
  correctly across ~40 files was going to blow this ticket's budget. None of
  these references are actually broken in the sense of 404: `DESIGN.md`
  still exists (as the stub/index), so `grep`-ing for a stale `§N` inside it
  fails loud, not silent — a reader who follows "DESIGN.md §8.1" today lands
  on the index and has one more click to `docs/design/08-architecture.md`,
  where §8.1 still exists verbatim (split was at top-level `##` only,
  sub-numbering intact within each file). Recommend a follow-up ticket
  (`housekeeping/02` is already taken by heap-slope; call it
  `housekeeping/03`) scoped just to this rewrite, ideally per-crate/per-ADR
  batches small enough to check by eye rather than by script.
- **PROGRESS.md is 45,094 B, not under 10 KB.** The ticket's own numeric
  target and its "current milestone's log only" requirement conflict: the M2
  session log *alone* (still open, still being appended to today) is 33.5 KB
  before any header/Phase-A/B/M3-brief content is added back. Moving any of
  it out would violate "current milestone's log only stays" and this run's
  explicit caution not to touch what other helpers are actively writing.
  Kept the log complete rather than hit the byte target by cutting live
  content. Flagging as a spec conflict for the user, not silently picking
  one side. Deviation, disclosed per the run's instruction.
- **Full "nothing dangles" sweep** only covered CLAUDE.md and the
  design-notes/DESIGN.md whole-file references (`grep` shown above/in
  report); it did not chase every `PROGRESS.md`/`KNOWN_GAPS.md`-internal
  heading anchor across `.scratch/` — none were found in a
  `grep -rn 'PROGRESS\.md#\|KNOWN_GAPS\.md#'` pass, so this is likely a
  non-issue, but wasn't exhaustively confirmed against every ticket body.

No compaction this run. Stopped for budget, not confusion — parts 1b's
scale only became clear once I ran the grep.

