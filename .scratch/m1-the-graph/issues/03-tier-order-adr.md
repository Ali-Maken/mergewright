# m1-the-graph/03 — Settle the rendering-tier order (ADR) and clear M0's three Minors

Status: ready
Build with: Opus
Blocked by: —

Why: KNOWN_GAPS.md says M1 opens by settling this, and it changes what every renderer ticket is for: the user works in the Zed and VS Code terminals, so **Unicode is the tier he sees most days**, not a downgrade. DESIGN.md §4.2 currently says the opposite. Nothing may be built on the old order. The three loose Minors from the M0 round-2 review are folded in here because they are all small documentation/diagnostic fixes in the same area.

## Read first
- KNOWN_GAPS.md, "Rendering tiers: order and the Unicode tier" (all three rows)
- DESIGN.md §4.2–§4.5, ADR-0014
- `docs/reviews/m0-foundations-round-2.md`, "New findings → Minor" (all three)
- `docs/agents/domain.md` for the ADR format

## Goal
The tier order is a written decision with reasons, DESIGN.md agrees with it, and the M0 review leaves nothing dangling.

## Scope
In:
- **The ADR** (this is not optional — it overrides an approved DESIGN.md section). The user's stated order is **Kitty → Unicode → iTerm2 → Sixel**, with iTerm2 and Sixel used only where they demonstrably beat Unicode. Record the order, the reason (daily terminals have no image protocol; a tier the user never sees is not worth preferring), and what "demonstrably beats" means in practice — the manual checklist row that decides it.
- Amend DESIGN.md §4.2's table to match, including what happens when several protocols answer at once.
- **Minor 1:** fix ADR-0013's false claim that an `unsafe` block cannot carry the allowance. Keep the decision; replace the justification with the real one (whole-function SAFETY argument is the chosen granularity).
- **Minor 2 (code):** `graft-term/src/detect/mod.rs` can report `pixel, assumed (terminal silent)` when round 1 timed out and the tmux re-probe answered. Fold the second round into the reported outcome, or derive the tier source from the round that decided the tier. Add the test for that edge.
- **Minor 3:** DESIGN.md §4.4 and ADR-0014 §6 still say "400 ms locally, 1 s over SSH"; the `tmux -V` 2 s deadline makes the true worst case ~2.4 s / ~3 s. Either state the real number or shrink `TMUX_VERSION_DEADLINE` to probe-budget scale and say so. Also add the one-line `kill` before the `try_wait` `Err` return the reviewer noted.
- Add a row to `TESTING.md`'s manual checklist: **does VS Code's `terminal.integrated.enableImages` give a usable Sixel tier?** It changes what the order should be, and only the user can answer it. Write the question so the answer is a yes/no with evidence.

Out:
- Any renderer code. This ticket decides and documents; 08/11/13/14 build.
- Changing the probe sequence itself.

## Acceptance
- [ ] ADR written, numbered, linked from DESIGN.md §4.2.
- [ ] DESIGN.md §4.2 and §4.4 read consistently with the code and with each other; no sentence still calls Unicode a fallback or a downgrade anywhere in DESIGN.md.
- [ ] Minor 2's edge has a failing-then-passing test.
- [ ] `TESTING.md` carries the VS Code Sixel question and the tier-order pass conditions.
- [ ] Report to the conductor: the order is the **user's** stated preference; if the VS Code answer or the M0 checkpoint probe runs contradict it, the ADR gets superseded, not quietly edited.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
