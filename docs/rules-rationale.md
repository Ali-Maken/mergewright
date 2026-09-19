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

## The metric is turns, not tokens (ADR-0029)

A helper's context is re-sent every turn, so cost is turns x prefix, not context size.
M2: mean 119 turns/run; turn reads ~57% of a run's cost, output ~40%, startup under a
tenth. The 80k token target was withdrawn because it measured the wrong thing and pushed
M2 to 28 tickets while the mean still came in at 114k on 33 of 33.

**M3 outcome (ADR-0035):** the prediction failed. M3's mean was 131 turns/run across 19
runs, 163 counting builders only -- worse than M2's 119. What did improve was compaction:
21 in M1 and 23 in M2, down to 3 in M3. Tighter specs stopped context blowups and did not
stop turn burn, because the burn came from specs being wrong about existing code.

## What actually cuts turns

M3's evidence: bug counts were flat at 5-9 findings per group regardless of cost, while
cost varied fivefold. Greenfield groups in a new crate ran ~200 turns; brownfield groups
in the existing UI ran 432, 484 and 941. The difference was not vagueness but specs being
wrong about code the planner had never seen. Hence ADR-0035's scout.

## The user's daily use is a test tier

The fast tier covers logic on fixtures we invented; heavy and perf cover scale at review;
the acceptance walk covers the promise along a path someone scripted. Only real use reaches
how it looks, repo shapes nobody made a fixture for, and sequences nobody thought to try.
Writing findings down the day they're noticed is what makes them survive the session.

## Captures can't prove colour

The pty pipeline records pyte character grids with no colour attributes, so no capture in
this repo can show a colour-only change. Every colour decision across M3's changes pane,
commit editor, stash overlay and journal browser therefore shipped judged by nobody but
the user's eye.

## Turn budget: why 60 is soft and 150 is hard

The 40-call/100k stop of 2026-09-15 was ignored 33 times out of 33, so ADR-0029 replaced
it with the 60-turn report point. In M3, 14 of 20 runs passed 60 -- a threshold that fires
on two runs in three is not a signal. ADR-0035 keeps 60 as the point where a builder says
what made it long, and adds 150 as a hard stop, because M3's cost came from single runs
grinding to 290 and 306 turns rather than from many small overruns.

Self-reported counts have been wrong low every time, by 8x and by 3x. The conductor also
must not write a number before the harness reports it: it logged a wrong figure three times
in M3 by writing the entry before the notification arrived.

## Why mockups are approved before the code exists

Every M3 layout defect was found only once the code existed: a confirm box that was never
painted, the commit editor drawn behind an unreachable tab under 100 columns, the diff
cursor hidden under the pinned @@ heading. Each was caught by a capture or a review at the
end of a group, after a builder had already been paid to build it wrong. A milestone's
design pass already runs at the previous milestone's checkpoint, so mockups cost no extra
interruption.

## Why brownfield groups are sized smaller

ADR-0029 groups by shared files so a builder reads the prefix once. In a new crate that is
right. In the graft app crate "shared files" means the entire UI -- the largest prefix in
the repo -- which is how M3's group 04 came to bundle the section model, whole-file
staging, hunk staging, line staging, discard, the confirm box, the pinned row, footer
hints, integration tests and captures into one unit: 6 runs, 941 turns.

## Why a false tick outranks any turn count

M3 group 04 ticked six acceptance lines that were not true. A builder marking work done
that is not done is a trust failure: it converts every other report in the project into
something that has to be re-checked. Group 05 named a test beside each tick unprompted and
its ticks were honest; group 04 did not and six were false. That is the whole argument for
making it a gate rather than a habit.
