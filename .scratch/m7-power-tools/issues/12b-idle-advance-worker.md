# 12b idle-advance-worker — Part B + Part D of 12, plus the 04b capture fix

Status: ready
Blocked by: 12 Part A (merged 7f64f5c0)
Crates: graft (`history::views`, `history::stream` worker loop, `update::history` apply arm, `event_loop/label.rs`,
`tests/it/heavy.rs`, `tests/it/two_boxes_over_content.rs`), graft-graph (tests only).
Build with: Opus (unchanged from 12).
Split by: conductor, 2026-09-26, along the builder's seam (12 compacted after Part A). No new decision.

## Scope
- **12 "Decided" → Part B and Part D**, exactly as written there. Signatures from 12 "Signatures": `Views::idle_step`,
  `IdleWork`, the `serve` loop, `HistoryUpdate::LaidOut` (+ its arms in `update` apply and `event_loop/label.rs`),
  `History::laid_out`. The 12 builder's NOTES "What remains (for the split)" lists what is ready.
- **12 "Acceptance" boxes for Part B (both) and Part D.** Tick them in 12's file, in place, each naming its test.
- **Carried fix, KNOWN_GAPS M7/04b:** `two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots`
  fails ~50% under load (debugger: 30 of 31 fails were `ⓘ` missing on the tip row). Cause: `ready()`
  (`crates/graft/tests/it/two_boxes_over_content.rs:102-124`) doesn't wait for the radar job. Proposed, untested: end
  `ready` with `crate::captures::radar_answered(app, …)`. Prove it: a loop at 20 parallel that failed before, 0 after;
  say the counts. Add a tick for it under 12's Acceptance, marked "(12b, M7/04b)".

## Not in scope
Part C, the final perf rev, heap slope, heavy peak memory, `frames` additions — those are 12c.

## Rules
12 "Touches", "Rules / Gates". Gates workspace-wide. Release runs only through the script.
