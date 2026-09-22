# Failure recurrence, M0–M6

A check of the record against itself: what went wrong, what was changed in response, and whether the same failure happened again after the change. Written 2026-09-22 by a researcher helper from `docs/rules-rationale.md`, `docs/log/m0–m6.md`, a sample of `docs/reviews/`, `docs/field-notes.md` and the conductor's memory notes. **Not every review was read.** Recurrence is only claimed where a later log line names it. Every row cites a line. Where no line could be found, the item is written as a question.

Sources are abbreviated: `RR` = `docs/rules-rationale.md`, `FN` = `docs/field-notes.md`, `mN` = `docs/log/mN.md`, `MEM:x` = `~/.claude/projects/-Users-devuser-dev-sandbox-gitter/memory/x.md`.

## 1. Failures, fixes, recurrence

| # | What happened (source) | Class | Reasoning at the time | Rule / change added | Recurred after the fix? |
|---|---|---|---|---|---|
| 1 | Builders ticked acceptance lines that were not true. M3 group 04: four found by review, on top of two the builder corrected itself (m3:45). RR:112 gives the total as "six". | false tick | "A trust failure: it converts every other report into something that has to be re-checked" (RR:113-114). Group 05 named its tests and its ticks were honest (RR:114-115). | Each tick must name its test; a false tick is Critical (ADR-0035, RR:110-116; CLAUDE.md "The gate"). First used in M3 g06's fix round (m3:62). | **Yes, repeatedly, but now caught by review.** M4 g07: the conductor's own tick (m4:30). M5 g02: 1 false, with a test that "exists nowhere" (m5:19). M5 g04: 3 false (m5:34). M5 g09: 1 false capture tick (m5:154,156). M5 12–13: 1 false (m5:198). M6 g01: 1 overstated (m6:22). M6 g02: 1 false (m6:47). Counter-evidence: M4 groups 02, 04, 05 and 06 held 9/9, 17/17, 13/13 and 26/26 (m4:7,15,22,25), and M5 had a 22/22 spot-check (m5:532). |
| 2 | Agents self-reported tool and turn counts that were wrong low: "by 8x and by 3x" (RR:89). M2/11 reported under the stop; the harness counted 218 calls (m2:60). M2/14 reported 42; the harness counted 58 (m2:67). | unverified claim (self-report) | The 40-call / 100k stop was expected to make builders stop themselves (RR:83). | Every self-counting rule deleted on 2026-09-19. The harness figure is logged instead (RR:143-160). | **No** once the rule was deleted: M4–M6 lines cite "(harness)" figures (e.g. m4:5, m6:22). Before that, the rule was ignored 33 of 33 times (RR:151). |
| 3 | The conductor logged a turn count before the harness reported it, three times in M3 (RR:90-91). | unverified claim | — | "Must not write a number before the harness reports it" (RR:90), now carried by the harness-only logging. | Did not find a later instance. Is that absence real, or did nobody look? Question. |
| 4 | The conductor wrote a specific it had not checked into a brief, ticket or gap row. M5: wrong KNOWN_GAPS row number (m5:78); `PartWay { done, total, error }` restated from memory (m5:82); a gap row marked **broken** that described nothing broken (m5:93); an interim keymap table promised and never added (m5:130, "sixth this milestone"). | unverified claim (conductor) | "I restated a signature from memory of a report instead of from the code" (m5:82). | Rule 7 added 2026-09-21: a claim carries its line or is filed as a question (m5:345). Second half, "could not confirm is a stop", added after the conductor missed a clamp at `detail_pane/layout.rs:34-36` (m5:696-698). | **Yes, after both halves were added.** MEM:check-the-specific… lists five more cases: tickets 34, 40, 41, 42, 44. Ticket 34 is dated 2026-09-22 (FN:42). Ticket 44 set floors that cannot both be met, 13.5 needed against 13.01 (m5:1452-1456). m5:1080 calls ticket 40 "the tenth instance of the signature failure". M6: "the conductor's own fix biting back", a line-number rule ordered without reading `conflict.rs:249-252` (m6:50). Builders caught every one of these. |
| 5 | The user filed defects from output, not from the call site. Defect 2 was withdrawn and wrong, and 5b was wrong (m5:316-340). | unverified claim (user) | "Both from reading output instead of the call site" (m5:340). | Rule 7 binds the user as well (m5:341-347). | Too early to tell. His later reports are visual, and the conductor measured them before filing (FN:48). |
| 6 | The first cursor-row report was filed without theme or terminal. Ticket 35 fixed the fallback marker, and the real cause (a 1.21:1 fill) shipped untouched (FN:44, 48; MEM:ask-theme…). | colour unverifiable / wrong fix | The ticket diagnosed "the same six-percent step in its own code comment" and still fixed the fallback (FN:48). | A memory note: ask theme and terminal first (MEM:ask-theme…). No CLAUDE.md rule. | Too early to tell. Fixed and confirmed by eye 2026-09-22 (FN:50). |
| 7 | Brownfield specs were wrong about existing code, and turn burn followed. Groups ran 432, 484 and 941 turns, against about 200 for greenfield (RR:62-65; m3:47). | spec defect → wasted work | "The burn came from specs being wrong about existing code" (RR:58). | ADR-0035: a scout before brownfield specs; `Build with` set after the scout (m3:55). | **The failure recurs; the fix catches it.** M6 G2's crate set could not be built, and the scout found it (m6:11). A stale keymap gate was found by the g05 scout (m6:80). The g04 scout found three facts that changed the brief (m6:43). |
| 8 | Scouts could not write their own reports, so the conductor transcribed them (m6:10, 42, 80-81; "same gap as M5", m6:10). | other (tooling) | — | New `scout` definition with a write tool, 2026-09-22 (m6:85). | Too early to tell. |
| 9 | Captures taken in the wrong state, with captions claiming screens the capture did not show: mid-load (FN:40), a caption true at one width only ("second instance", FN:41), a banner absent at 80x24 (m5:156), and captures "lying" in ticket 40 (m5:1080). | unverified claim (captions) | "A caption is prose" (m5:659). | Rule 6 (user judges by eye) and the contact sheet (ADR-0037). Did not find a rule on capture state. | **Yes**, within M5 (FN:41 says "second instance"; m5:1080). Is there an M6 instance? Not found. |
| 10 | The green-gate sweep did not reach what ships. Ticket 30's border sweep passed; the user found a missing border in five minutes (FN:42). | two things that must agree, untested | "The invariant is right; the sweep does not reach what ships" (FN:42). | Ticket 34. Did not find a rule. | Unknown. Question. |
| 11 | Parts that must agree but were never tested together: the same fact drawn twice (FN:36); the text and pixel tiers folding different graphs (m5:269); a MAP claim made false by a clean auto-merge (m6:74); `e`/`M` re-read asymmetry (m6:71). | two things that must agree, untested | "Neither is wrong; together they waste a scarce screen" (FN:36). | Fixed case by case. No rule. | **Yes**, across M5 and M6 (rows cited). |
| 12 | Helpers compacted mid-ticket: 21 in M1 and 22–23 in M2 (RR:57; m2:94 says 22, RR says 23). | oversized agent work | Tickets too big (m2:17). | ADR-0029 groups; context raised to 200k (RR:20-24). | **Reduced, not gone**: 3 in M3 (RR:57). M4 had 3 (m4:19, 24, 28). M6 had g01 once, g08 once and g04 twice (m6:12, 54, 67). |
| 13 | Worktree and index collisions: a housekeeping commit "swept into a conductor commit by mistake" (m2:86); a helper's worktree inside the repo committed by `git add -A` (m5:85). | other (concurrency) | "`git add -A` from the conductor is no longer safe by default" (m5:85). | ADR-0036 worktrees; "never stage while another agent works"; `.worktrees/` gitignored (m5:85). | Did not find a later instance. |
| 14 | Standalone research dispatched without a model inherited Opus: the naming research (m3:31, "general-purpose"; RR:132 "went to Opus by mistake"). | wrong model / cost | "No warm context to ride, so the cheaper model is simply cheaper" (RR:130). | CLAUDE.md: never dispatch `general-purpose`/`Explore` without `model:`. | Did not find one. M6 scouts are logged as "sonnet" (m6:10, 42, 80). |
| 15 | A builder fixed with a 20/10 ms polling accept (m4:18). | symptom fix | — | Sent back under rule 2 (m4:18). | Did not find one. |
| 16 | A review round missed at the time, then run late (m4:33). | process skip | — | ? No rule found. Question. | Unknown. |
| 17 | KNOWN_GAPS rows had gone false: "`graft-ops` is empty", "no test checks the 300 MB gate" (m3:57). The capability ledger cited 4 of 108 tests that no longer existed (m5:74). | stale claim | Tests renamed or deleted by later work in the same milestone (m5:74). | Mechanical check of the ledger (m5:74). | ? Unknown. |
| 18 | A mutation claim was half true: the named test passes under mutation B (m5:87). Two reviewer mutations exposed tests "that cannot go red" (m5:154). | unverified claim (tests) | — | Reviewers re-run mutations (m5:87). The builder re-measured the review's experiments (m6:32). | Did not find a later instance. |
| 19 | The builder's git model was disproved by the reviewer's experiment, twice in M6 g01 (m6:23) and twice more in g02/g03 (m6:48). | unverified claim (external behaviour) | — | Nothing new. Reviews already run git. | **Yes** within M6 (m6:23 → m6:48). |
| 20 | CLAUDE.md grew 12.4x in eight days: 53 rules, about 13M tokens over M3 (RR:136-139). | cost | "Every helper re-reads it on every turn" (RR:137). | Cut to about 6 KB, rationale moved out, no ADR per incident (RR:141, 175-178). | ? CLAUDE.md size now not measured here. Question. |

## 2. Agent work wasted, or that could have been simpler

- **M3 group 04, 941 turns over 6 runs**, the milestone's worst (m3:47). The first Sonnet run spent 290 turns on whole-file staging alone before escalating to Opus (m3:41). Simpler route: size brownfield groups to one pane, and route git writes to Opus from the start (RR:102-108; m3:41).
- **M3 groups 05 and 06: 484 and 432 turns** (m3:51, 54). The cause was specs wrong about existing code (RR:62-65). Simpler route: a scout first (ADR-0035).
- **M2/06b: 197 calls / 162k plus a compaction**, "no cause in the report", 5x past the stop (m2:48). Simpler route: split the ticket.
- **M2/10b continuation 2 (M2/11): 218 calls / 149k**, reported as under the stop (m2:60).
- **Conductor errors paid for by builders.** Five tickets each cost "a builder spending real turns" (MEM:check-the-specific…); the count per ticket is not logged. M5 ticket 09: the builder wrote the keymap table the conductor owed (m5:130). The `PartWay` brief sent group 08 "running against the wrong shape" (m5:82). Simpler route: open the specific before writing it (MEM:check-the-specific…, "How to apply" 1).
- **Scout transcription.** The conductor rewrote four scout reports from the replies (m6:10, 42, 80, 81), at 47–55 tool uses per scout. Fixed by the `scout` definition (m6:85).
- **Ticket 35 fixed the wrong half** of the cursor complaint, costing a full round (MEM:ask-theme…). Simpler route: one question, "which theme?" (FN:50).
- **M6 g04: 363 tool uses, compacted twice, partial** (m6:67). A continuation of 42 finished it (m6:82). Could it have been split up front? Question.
- **M6 g08: 241 tool uses, compacted, partial**, then 106 more (m6:54, 60).
- **Thread-teardown fix: 81 tool uses** including a polling pass that was sent back (m4:18).
- **Naming research on Opus** by mistake (RR:132). Its cost is not logged (m3:31, "1 run").

## 3. Failure classes with no rule

- **Parts that must agree but are never tested together** (rows 10, 11). Nothing requires a test that two correct halves agree, whether that is two panes, two tiers, or MAP against code after a merge.
- **Capture state.** Nothing requires a capture to be taken in the state the question is about (FN:35, 40). Rule 6 covers who judges, not what is captured.
- **Environment in a user report** (row 6). This exists only as conductor memory, not in CLAUDE.md.
- **Claims about git behaviour** (row 19). Builders are not required to run git before asserting how it behaves; m2:92 and m6:32 show builders doing it anyway.
- **Stale documentation inside a milestone** (row 17): comments, MAP.md and ledger citations that go false as later code lands (m5:271, m6:74).
- **Skipped review rounds** (row 16).

## 4. Inference: which rules the record shows working

*This section is inference from the rows above, not a finding.*

- **Working.**
  - Harness-only counting (row 2).
  - The scout (row 7): it now catches spec defects before the build.
  - "Say if you compacted" and "say if you used a scout" (RR:163-165; m6:31).
  - Builders verifying before building: the record credits them with catching every conductor error in row 4.
- **Working as detection, not prevention.**
  - The named-test tick gate. False ticks continued in M4, M5 and M6 (row 1), but review spot-checks caught each one. Rule 1's premise, that a builder will not overclaim, still does not hold on its own.
- **Not working yet.**
  - Rule 7 on the conductor's side. The conductor's own memory counts five instances after the rule was added, and m5:1080 counts ten instances of the signature failure overall. The rule catches the failure at the builder, one round late; it has not stopped it at filing.
  - Compaction. It was reduced, but M6 g04 compacted twice (m6:67).

**Open questions:** row 3's absence of recurrence; row 10's recurrence; row 16 (why the M4 g03 review was missed); row 20 (current CLAUDE.md size); whether the M0/M1 reviews (only grepped here) hold false-tick or unverified-claim findings this table misses. The origin incident of the `GFX_ERROR` status rule (RR:28) was not found in the logs.
