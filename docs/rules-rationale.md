# Rules rationale

The "why" behind CLAUDE.md's rules, split out (housekeeping/01, 2026-09-18) so
CLAUDE.md stays lean — this file is not auto-loaded by any helper. Nothing here
changes what a rule requires; each entry just explains why the rule in
CLAUDE.md reads the way it does. Cite CLAUDE.md for the rule itself, this file
only for the reasoning.

## Ticket size: 80k target

M1's session log records its compactions and tickets that ran 93k–200k against
the 80k target — that's the evidence `planner-fable` sizes from (user,
2026-09-15).

## Expect one or two Fable builds per milestone

If a plan wants three or more, the design is usually underspecified and needs
another design pass, not more Fable builds.

## Context window raised to 200000

Builders were compacting mid-ticket; losing the prompt cache and re-reading
the same files costs more than the larger window does. Keeping context lean
still matters — the cap is headroom, not an allowance.

## Never swallow an error / status fields that read as errors

A permanently-visible `GFX_ERROR:`-style label trains the user to ignore it,
which is how a real error gets missed — hence naming/rendering status by its
actual state instead of a fixed label.

## Green gates are not evidence

`fmt`/`clippy`/`test` passing only means the code compiles and the helper's
own tests agree with it. It says nothing about whether the thing actually
works, so builders must say what they verified and how.

## Why two banned skills keep getting reached for

- `prototype` builds HTML prototypes. Questions about terminal rendering are
  answered only by a binary running in a real terminal — HTML tells you
  nothing about how a TUI actually paints. Keep its principles (throwaway,
  minimal, findings recorded as an ADR), not the skill itself.
- `wayfinder` (and `to-spec`) would create a second plan. The plan already
  exists: `PROMPT.md` is the spec, `DESIGN.md` the design, `PROGRESS.md` the
  plan. A second plan would drift from the first.
