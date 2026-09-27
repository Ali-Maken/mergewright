# 20k — Blame bar while a read is in flight (split from 20i)

Status: ready
Milestone: M7 (m7-power-tools)
Split from: 20i (compacted mid-ticket; remainder along the builder's named seam, 2026-09-27). Scope, signatures, model unchanged from 20i.
Blocked by: 20i (merged 8d98c9a9).
Crates: graft-ui (`HintContext::Blame`), graft (`update/place/keys.rs`, the hint sweeps; Q1 below: `update/place/trail.rs`, `update/detail/landing.rs`, `update/place/tests/esc_back.rs`).
Build with: Opus (unchanged from 20i).
Scout: none — 20i's NOTES name the lines.

## What is wrong

While blame is reading (including the re-read after `i`), the bar still draws `⏎` and `^`, but `update/place/keys.rs:219` and `:273` don't answer them (pre-existing; found by 20i). So `every_key_the_bar_draws_is_answered`, given a reading-after-`i` state, goes red. 20i left that sweep row out.

## Seam (20i's)

`HintContext::Blame { read: bool }` in graft-ui. While a read is in flight the bar leaves out the keys the place does not answer. Decide nothing else. If the right fix looks like "answer the keys" rather than "don't draw them", stop and report it: that is a spec question for planner-fable, not a build decision.

## Added 2026-09-27 (planner, from review m7-32-20i Q1, gap M7/32i-q1): a `⏎` landing after `Esc` goes back

**A question, not a defect (rule 7): settle it with a test first.** `show_again` (`place/trail.rs:80-86`, at `cb065156`) sets `trail.shown` and leaves `state.line_landing` alone; `leave` drops it (`trail.rs:46`); `detail::landing::advance` (`landing.rs:141-166`) runs after every event with no trail check. So after `B ⏎ Esc` onto a commit the walk hasn't reached, the landing may still select it, open the pane, or say "missed" under blame.

**Ruling:** going back to blame abandons the `⏎` he left. `show_again` drops `state.line_landing` as `leave` does — one rule for both ways off the landing. Not a trail check in `advance` (that makes `advance` know about places; the landing's owner is whoever ends it).

- [ ] **Red first** `esc_back_drops_a_landing_still_seeking` (in `esc_back.rs`): `B`, `⏎` onto a commit not yet loaded, `Esc` (blame shown again), then deliver the rows. Assert: the graph selection and the pane are where they were before `⏎`, the status says no "missed", blame still shown; then leave blame: the origin is restored. **If it is green before the fix**, the question closes "no": quote the line that already stops it in NOTES and change nothing.
- [ ] Green after the fix; the other 8 `esc_back` rows unchanged and green.
- [ ] MAP.md graft: `show_again` "drops a landing still seeking, as `leave` does".
- [ ] Touches: every `line_landing` writer/reader (grep); 34 (queued) moves `escape_key` but not `show_again` — no clash.

## Acceptance

- [ ] Red first: add the reading-after-`i` state (and plain reading) to the `every_key_the_bar_draws_is_answered` sweep; quote the red.
- [ ] Green with the `read` field; the sweep includes the state (no row left out).
- [ ] `every_context_reaches_every_shape` (or its successor) covers `Blame { read: true }`.
- [ ] Captures that show a blame bar re-checked (`check_shot` green); any regenerated one listed.
- [ ] MAP.md graft-ui (`HintContext::Blame`) updated.
- [ ] The Q1 boxes above.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches

Every `HintContext::Blame` construction (grep + compiler), the hint catalogue, the bar sweeps, blame captures under `docs/shots/m7/`. No keymap change, no journal/undo. Q1: `line_landing` sites (grep); for the conductor, closes M7/32i-q1.
