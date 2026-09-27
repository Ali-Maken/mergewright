# 20k — Blame bar while a read is in flight (split from 20i)

Status: ready
Milestone: M7 (m7-power-tools)
Split from: 20i (compacted mid-ticket; remainder along the builder's named seam, 2026-09-27). Scope, signatures, model unchanged from 20i.
Blocked by: 20i (merged 8d98c9a9).
Crates: graft-ui (`HintContext::Blame`), graft (`update/place/keys.rs`, the hint sweeps).
Build with: Opus (unchanged from 20i).
Scout: none — 20i's NOTES name the lines.

## What is wrong

While blame is reading (including the re-read after `i`), the bar still draws `⏎` and `^`, but `update/place/keys.rs:219` and `:273` don't answer them (pre-existing; found by 20i). So `every_key_the_bar_draws_is_answered`, given a reading-after-`i` state, goes red. 20i left that sweep row out.

## Seam (20i's)

`HintContext::Blame { read: bool }` in graft-ui. While a read is in flight the bar leaves out the keys the place does not answer. Decide nothing else. If the right fix looks like "answer the keys" rather than "don't draw them", stop and report it: that is a spec question for planner-fable, not a build decision.

## Acceptance

- [ ] Red first: add the reading-after-`i` state (and plain reading) to the `every_key_the_bar_draws_is_answered` sweep; quote the red.
- [ ] Green with the `read` field; the sweep includes the state (no row left out).
- [ ] `every_context_reaches_every_shape` (or its successor) covers `Blame { read: true }`.
- [ ] Captures that show a blame bar re-checked (`check_shot` green); any regenerated one listed.
- [ ] MAP.md graft-ui (`HintContext::Blame`) updated.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches

Every `HintContext::Blame` construction (grep + compiler), the hint catalogue, the bar sweeps, blame captures under `docs/shots/m7/`. No keymap change, no journal/undo.
