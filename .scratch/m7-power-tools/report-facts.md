# M7 report facts (docs/log/m7.md; :N = log line). Compiled by researcher 2026-10-01, saved by conductor.

## Tool counts (harness; sums by researcher)
- Builders 16 203 / 140 entries → mean ≈116. Uncounted: 04b pre-cut :35, 10b resume :352, 24-a stop :430, 30p-a stalls :464, 14 first stop :282. 14/16/21b finals taken as cumulative (:296 :313 :362; log silent).
- Planners 1614; 9 no count (:40 :132 :284 :320 :338 :351 :377 :378 :530)
- Scouts 316; 11 no count
- Reviewers 2378 / 28 runs; 2 no count (:417; :517 stalled)
- Researchers 245; 2 no count (:380 :418)
- Debugger 525 / 6 runs (:62 :155 :272 :468 :484 :503)
- Total ≈21 281

## Compacted → split
02a→02b :30 · 12A→12b,12c :67 · 12c→12d,12e :80 · 22→22b :76 · 20→20e,20f :169 · 20e×2→20g :180 · 30→30b :193 · 20i→20k :221 · 30c→30d :229 · 05→05b :297 · 21×2→21b,21c :350 · 09×2→09b :368 · 07c→07d :412 · 24-a→remainder :438 · 26→30s,t,u,v,26b :492
No split: 02b 18 20b 18e 14 13b 10b 21b 03b 30j 10(run2) 07b 11 08-a 07d 08-b 24-b 30o-b 30q-a 30r 30s

## Escalations
No log line says "escalated". Above default tier: xhigh 30q-a/b :466, 30r :477, 30s :493, 30w :509; 12g → debugger :243.
Spec model ≠ log model, no reason given: 06f, 33 specced Sonnet → builder-opus (:235 :236); 16b sonnet :318 → opus :344.

## Models
builder-opus Opus medium; builder-sonnet Opus low; builder-fable Opus high; builder-opus-xhigh (30q-a/b 30r 30s 30w); planner-fable Opus high; scout, researcher Opus low; reviewer/debugger Opus high per conduct SKILL.md (log silent).

## Waste / slips / deviations
:32 shared CARGO_TARGET_DIR · :35 network cut, 3 runs
:49/:51 17 sent back +10 · :65 merged with an unread grep hit · :154 gates run mid-merge
:199 researcher dispatched to edit KNOWN_GAPS · :222 merge broke the main build; ticket-number clash
:343 reviewer committed on main · :344 16b edit without a grant · :353 30i skipped red-first
:203 06d reverted +4 · :312 01b sent back +11
:383/:387 30j, 09b stalled · :462/:464 30p-a stalled twice (n/a)
:450 merged 30o-a before reading its ticks · :455/:459 published_repo failed on text
:486 30q-a/b file lists missed sites; 16 perf run wasted; 30r 216; full gates on docs-only change ~5 min
:512 30w + 30w-b merged with no gates between · :517 reviewer stalled in fuzz
:518 tickets misnumbered · :519 specs on stale line refs → 22 · :529 second spec-correction pass
:532 40 file list missed 9 test files → ruling 25 · :535 researcher dispatched to edit KNOWN_GAPS again (refused, 1) · :538/:539 exit code misread
Builder deviations (no cost): :54 :120 :147 :204 :209 :252 :267 :360 :362 :392 :419 :472 :536
Waste total (costs logged): 216+25+22+11+10+4+1 = 289 tool uses + ~5 min gates. Stalls uncosted.
