# acceptance-captures-and-sheet: ticket 21's remainder — captures, contact sheet, TESTING.md

Status: ready
Blocked by: 42 (its fixes change frames 43 captures).
Was: split by the conductor 2026-09-24 from ticket 21, which compacted once after part 1 (merged `36e89da1`). Scope, model and rules unchanged from 21; the seam is 21's own ("parser and journeys first; TESTING.md and captures second").
Build with: **Fable**. It is the acceptance ticket's second half.
Needs: 42.
Crates: graft (tests), scripts, docs.

## Read first
- Ticket 21 in full: its Scope 6, 7, 8 and its NOTES, **"Remaining (exact next steps)" 1, 2, 4** and "Gotchas". Those are this ticket's steps; follow them as written there.
- ADR-0037 (contact sheet). `.scratch/m6-conflicts/captions-fix-draft.tsv`.

## Scope
In: 21's Remaining steps:
1. The 10 capture stems.
2. The contact sheet, split into parts of 200 KB or less, with every `fix-*` capture in ticket order under its caption, including the captures from tickets 37–42.
3. TESTING.md "Conflict journeys": the Zed `Ctrl+T` line is marked **unverified**. Confirm "Times I left" :108 is already closed (`shell_begun::c_enter_finishes_a_shell_begun_merge`).

Out: perf (ticket 44). The M6/32 bar-words facts (conductor → researcher). Any behaviour fix: file it with a line, or as a question.

## Acceptance (each tick names its test, at tick time)
- [ ] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with the new frames. (21's box.)
- [ ] TESTING.md "Conflict journeys" written, every line concrete (no "looks right"), with the Zed `Ctrl+T` line marked **unverified**. (21's box.)
- [ ] The captures are listed by path in the report. The user judges them by eye (rule 6). (21's box.)
- [ ] Contact sheet part paths in the report; each ≤ 200 KB.
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests.

## NOTES
