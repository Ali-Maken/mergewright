# 20k — Blame bar while a read is in flight (split from 20i)

Status: done
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

- [x] **Red first** `esc_back_drops_a_landing_still_seeking` (in `esc_back.rs`): `B`, `⏎` onto a commit not yet loaded, `Esc` (blame shown again), then deliver the rows. Assert: the graph selection and the pane are where they were before `⏎`, the status says no "missed", blame still shown; then leave blame: the origin is restored. **If it is green before the fix**, the question closes "no": quote the line that already stops it in NOTES and change nothing.
- [x] Green after the fix (`esc_back_drops_a_landing_still_seeking`); the other 8 `esc_back` rows unchanged and green.
- [x] MAP.md graft: `show_again` "drops a landing still seeking, as `leave` does".
- [x] Touches: every `line_landing` writer/reader (grep); 34 (queued) moves `escape_key` but not `show_again` — no clash.

## Acceptance

- [x] Red first (`every_key_the_bar_draws_is_answered`): add the reading-after-`i` state (and plain reading) to the `every_key_the_bar_draws_is_answered` sweep; quote the red.
- [x] Green with the `read` field (`every_key_the_bar_draws_is_answered`, `every_swept_state_is_the_context_it_claims`); the sweep includes the state (no row left out).
- [x] `every_context_reaches_every_shape` (slots 81–84) (or its successor) covers `Blame { read: true }`.
- [x] Captures that show a blame bar re-checked (`check_shot` green); any regenerated one listed.
- [x] MAP.md graft-ui (`HintContext::Blame`) updated.
- [x] The Q1 boxes above.
- [x] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches

Every `HintContext::Blame` construction (grep + compiler), the hint catalogue, the bar sweeps, blame captures under `docs/shots/m7/`. No keymap change, no journal/undo. Q1: `line_landing` sites (grep); for the conductor, closes M7/32i-q1.

## NOTES (20k builder, 2026-09-27)
- Not compacted. No scout: the ticket named the lines (20i's NOTES); read `trail.rs`, `landing.rs`, `history/scroll/{query,apply}.rs` directly.
- **Q1 red, quoted** (before any fix): `assertion left == right failed: reached: graph selection; left: Some(Oid(a1a1…)) right: Some(Oid(0202…))` — the landing selected Lina's commit under blame. Q1 closes **yes**.
- **Deviation from the ruling, cause not symptom:** dropping `state.line_landing` alone (the ruling) left the same red: the walk's find answer moves the cursor itself (`history/scroll/apply.rs:54`, `seeking.take()`; corrected from a stale `:92-99` per review m7-20k-to-20l Minor 7 → `cursor = …`), and the pane follows (`pane spec` then went red too). So `show_again` now calls new `detail::abandon(state, commands)`: drops the landing and, if it was `Seeking(c)` with `history.seeking == Some(c)`, `withdraw_jump` + `ShowRows` for the viewport — the same move as `escape_key`'s `G` rung (`update/keys.rs:511-519`). `show_again` gained `commands` (callers `detail/keys.rs:84`, `update/keys.rs:558`). `leave` calls `abandon` too (one rule for both ways off). Not a trail check in `advance`, per the ruling.
- **Bar red, quoted:** 180 dead keys: `⏎ that commit` ×144, `^ before it` ×144, `n N match` ×72 (counted by hint line), e.g. `Blame { finding: false, ignore_file: false } (Hidden) at 100 columns … draws ⏎ that commit`. `n N` was dead too while reading (no lines to step); left out with `⏎`/`^` — "don't draw", nothing answered.
- Sweep states for `read: true`: re-read after `w`, after `i` (with the file), and the first read after `B` (neither flag): `place::tests::place_reading`.
- Captures: none shows a reading place (grep `Reading` in `docs/shots/m7/*blame*` empty); the 33 blame/esc_back `it` tests incl. check_shot green; none regenerated.
- Touches: `line_landing` sites grepped — `landing.rs` (show_line :69/:103, advance, cannot_show, LandingWatch :348-361 — sees the landing already gone, unaffected), `trail.rs` leave/show_again. `HintContext::Blame` constructions: `view/rows.rs:595`, catalogue, graft-ui tests; compiler-checked. 34 (queued) moves `escape_key`, not `show_again`; `show_again`'s new `commands` arg is at `update/keys.rs:558`, which 34 may move — trivial clash at most.
- **Question (not verified):** with the pane closed and a seek live, graph `Esc` hits `escape_key`'s `G` rung (`update/keys.rs:511-519`) before `show_again` (`:558`), while `esc_goes_back` (`trail.rs`) ignores seeks, so the bar may lead `esc back to blame` while the first `Esc` withdraws the seek. Not in the `the_bar_leads…` table.
- **Question:** a `Failed` place also leaves `⏎`/`^` unanswered (`place/keys.rs` `enter`/`caret` return on non-`Ready`) and draws them; no failed state is swept. Out of 20k's scope (`read` is in-flight only).
