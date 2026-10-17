# 05c — merge-review rest: the live clash bar, Q7 pause, Q17 landing, relaunch file states

Status: ready (conductor split, 2026-10-18: 05b compacted 3× and stopped at its split point with 8/15)
Blocked by: 05b-merge-review-at-y (merged 4ed45107)
Build with: Fable — 05b's own tier; the remainder (landing every clash stop, relaunch classification) keeps 05b's three Fable reasons.
Spec: **`05b-merge-review-at-y.md` is the spec.** Its Decided, Scope, Signatures and Touches hold unchanged. Read its NOTES and Handoff first; they say where each open piece starts.

## Scope = 05b's open acceptance lines
Tick them **in 05b's file**, each naming its test:
- `:63` merge all-solved bar (the live resolver bar must pick `HintContext::Clash`; today it still draws `⏎ done … d start over`). **First**: in a merge `⏎` marks nothing, so today's bar misleads.
- `:64` fixture merge, one file, 2 clashes, landing + `o o` → `Solved`, `ls-files -u` still lists it.
- `:68` journey `m y o y y` lands on the graph at the merge commit (needs Q17's landing).
- `:69` Q7 `esc` pause / banner `⏎` back to the first open clash, picks kept.
- `:71` relaunch `file_state` (binary and delete/modify read `Open`).
- `:74` Q17 a: rebase and cherry-pick clash stops land on the resolver.
- `:75` `edit` stop lands on the graph; `u` mid-merge refuses with §2.6a's words.
- Also (05b report): a solved merge file's row still reads `⏎ marks done` — make it follow 05b's Decided.

Out: 11's clash-screen look, 12's review look.
