# m1-the-graph/03 — Settle the rendering-tier order (ADR) and clear M0's three Minors

Status: done
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
- [x] ADR written, numbered, linked from DESIGN.md §4.2.
- [x] DESIGN.md §4.2 and §4.4 read consistently with the code and with each other; no sentence still calls Unicode a fallback or a downgrade anywhere in DESIGN.md.
- [x] Minor 2's edge has a failing-then-passing test.
- [x] `TESTING.md` carries the VS Code Sixel question and the tier-order pass conditions.
- [x] Report to the conductor: the order is the **user's** stated preference; if the VS Code answer or the M0 checkpoint probe runs contradict it, the ADR gets superseded, not quietly edited.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Done (2026-09-12)

- **ADR-0016** `docs/adr/0016-rendering-tier-order-kitty-then-unicode.md` — order
  Kitty → Unicode → iTerm2 → Sixel → ASCII, the reasons, what "demonstrably
  beats Unicode" means (a `TESTING.md` checklist row run by the user with both
  tiers on screen), the two questions when several protocols answer, and a
  "What would reopen this" section: contradicting evidence **supersedes** the
  ADR, it is not edited into it.
- **DESIGN.md** §4.2 rewritten as two tables (protocol reachability, then
  tier); §4.3 "falls back to" → "settles on"; §4.4 gained the `tmux -V` bullet
  with the real worst case; §4.6 and §7 reworded. No sentence in DESIGN.md
  calls the Unicode tier a fallback or a downgrade.
- **Minor 1:** ADR-0013's false claim replaced — a statement-level
  `#[allow(unsafe_code)]` does compile; function scope is the chosen
  granularity because each `SAFETY:` argument spans the whole function.
- **Minor 2:** `detect::probe`'s outcome now folds both rounds (a failure in
  either wins; otherwise a fenced tmux re-probe counts as an answer). Proved
  red-then-green: with the fold reverted,
  `a_tmux_reprobe_that_answers_is_not_reported_as_silence` fails
  `left: TimedOut, right: Answered`; restored, 73 pass in `graft-term`.
- **Minor 3:** `TMUX_VERSION_DEADLINE` is now `super::LOCAL_PROBE_BUDGET`
  (200 ms) rather than 2 s — structural, not a coincidence of numbers — and
  DESIGN.md §4.4 / ADR-0014 §6 state the resulting worst case: 600 ms locally,
  1.2 s over SSH, inside tmux only. The `try_wait` `Err` arm kills and reaps
  the child before returning.
- **TESTING.md:** two new sections — "Does a pixel tier beat Unicode here?"
  (the procedure that decides a promotion, 4 terminals, all *not run*) and
  "Does VS Code give a usable Sixel tier?" (3 questions, *not run*).

### Deliberately past the "Out" list
`detect::tier` was changed. The ticket says "Out: any renderer code", but
acceptance requires §4.2 to "read consistently with the code", and leaving
`tier()` preferring iTerm2/Sixel over Unicode would contradict the ADR being
written in the same commit. `tier()` is detection, not a renderer and not the
probe sequence. `pixel_protocol` is still reported independently, so
`graph.renderer = "pixel"` can still force iTerm2/Sixel.

### Gaps logged
`KNOWN_GAPS.md`, "Rendering tiers: order and the Unicode tier": VS Code Sixel
unverified; "demonstrably beats Unicode" never measured for any protocol; and
a new one — **no Sixel encoder is planned**, yet a Sixel-only terminal without
truecolor now selects the pixel tier. Owned by M1 ticket 14.
