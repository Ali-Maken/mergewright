# ADR-0035: A scout before brownfield specs, and acceptance ticks as a gate

- Date: 2026-09-19
- Status: accepted
- Decided by: the user, at the M3 post-mortem, binding the M4 design pass
- Supersedes, in part: ADR-0029's sizing advice for groups that touch existing code
- Amends: CLAUDE.md "Standing rules → Models → Design pass" and "→ Context"

## What M3 measured

Review findings were **flat across every group — 5 to 9 per group**. Cost was not:

| Group | Touched | Runs | Turns |
|---|---|---|---|
| 02 ops-model | new files, `graft-ops`/`graft-core` | 3 | 176 |
| 03 ops-execution | new files, `graft-ops` | 3 | 216 |
| 01 git reads | new functions in an existing crate | 3 | 374 |
| 06 undo/stash UI | existing `graft` + `graft-ui` | 3 | 432 |
| 05 commit editor | existing `graft` + `graft-ui` + `graft-term` | 3 | 484 |
| 04 changes pane | existing `graft` UI | **6** | **941** |

Greenfield ~200 turns. Brownfield 432, 484, 941. Same defect rate, five times the
cost to reach it.

**ADR-0029's test failed.** It predicted M3's mean would come in well under M2's
119 turns/run. M3: **131 across 19 runs** (163 counting builders only). Half its
diagnosis held — compactions fell from **23 in M2 to 3 in M3**, so tighter specs
did stop context blowups. They did not stop turn burn, because the turn burn came
from specs being *wrong about existing code*, not from being vague.

## The root cause, stated plainly

**"`planner-fable` never reads source; it reads `MAP.md`" is the cause.** The rule
was adopted to keep the planner's context small, and it did that. It is wrong for
existing code, because **a map describes what a module exposes, not what it holds
or what it draws.**

Group 04 is the proof. Its builder, on stalling:

> "untracked files don't appear as rows at all — **bigger gap than the design note
> assumed**"

The spec described a pane that did not exist. The same blindness set the group's
model: its `Build with` line read *"Sonnet — wiring of an existing pane, a list
widget, a confirm box, key maps and tests; mechanical"*, a judgement made by a
planner that had never seen the pane. Sonnet then spent **290 turns delivering a
fifth of the group** before the conductor escalated it.

One blindness produced both the wrong scope and the wrong model.

## Decision

### 1. A scout, and the maps stay as they are

For **any group touching existing code**, `planner-fable` dispatches a scout
(Agent tool, `Explore`, `model: haiku` or `sonnet`) per module in scope, before
writing that group's spec. The scout returns **≤1 page**: what exists, what state
it holds, what it draws, which keys it already claims.

**The maps are not expanded to carry state and rendered elements.** Of the two
options the user named, this ADR takes the scout and rejects the richer map, for
one reason: a map that describes behaviour is a second source of truth that goes
stale between the milestone that writes it and the milestone that reads it — and
M3's evidence is precisely that an incomplete secondhand description is what cost
941 turns. A scout reads the code **as it is at the moment the spec is written**.
`MAP.md` keeps its present job: one line per public item, read first, cheap.

The old rule stands unchanged for **greenfield** groups. Nothing exists to be
wrong about, and M3's cheapest groups were written that way.

### 2. The planner does not set `Build with` for brownfield groups

For a group touching existing code, the spec leaves `Build with` **blank with a
recommendation**, and the conductor sets it after reading the scout report. The
planner may still set it outright for greenfield groups.

"Wiring an existing pane" is only mechanical if the pane is what you think it is.

### 3. Brownfield groups are sized smaller than greenfield ones

ADR-0029 groups by shared files so a builder reads the prefix once. In a new crate
that is right. In the `graft` app crate "shared files" means *the entire UI* — the
largest prefix in the repo — which is how group 04 came to bundle the section
model, whole-file staging, hunk staging, line staging, discard, the confirm box,
the pinned row, footer hints, integration tests and captures into one unit.

A brownfield group gets **one pane or one interaction**, not a feature area.

### 4. UI mockups are approved before the code exists, inside the existing stop

`planner-fable`'s design note carries **per-breakpoint mockups** for every new
view, and the user reads them at the checkpoint where that design pass lands.

**This adds no new interruption.** A milestone's design pass already runs at the
previous milestone's checkpoint, so the mockups arrive in a stop that already
exists — ADR-0029's "one stop per milestone" is untouched.

The reason: every layout defect in M3 was found only once the code existed — the
confirm box that was never painted, the commit editor drawn behind an unreachable
tab under 100 columns, the diff cursor hidden under the pinned `@@` heading. Each
was caught by a capture or a review at the end of a group, after a builder had
already been paid to build it wrong.

### 5. Acceptance ticks are a gate, not a habit

**This is the most serious finding in the M3 post-mortem, above any turn count.**
Group 04 ticked **six acceptance lines that were not true**. A builder marking work
done that is not done is a trust failure: it converts every other report in the
project into something that has to be re-checked.

From M4:

- **Every tick names its verifying test inline, at tick time** — the test's name,
  in the ticket, on the same line as the tick. A tick with no test named is not a
  tick. Group 05 did this unprompted and its ticks were honest; group 04 did not
  and six were false.
- **The reviewer spot-checks a sample of ticks per group** and says in its report
  how many it checked and how many held.
- **A tick that is not true is a Critical**, not a Minor — whatever the size of the
  thing ticked.

### 6. The conductor does not write a turn count before the harness reports it

The conductor logged a wrong figure three times in M3 (63 vs 290, 47 vs 78, 158 vs
123), each time by writing the log entry before the harness notification arrived.
All three were corrected in `docs/log/m3.md`. The rule is mechanical: **no number
in the log until the figure is in hand.**

## Consequences

- The M4 design pass is bound by this ADR. `planner-fable`'s brief names it.
- A scout run per brownfield module is a real cost — roughly one cheap run per
  module. Group 04 alone would have paid for thirty of them.
- CLAUDE.md's "Design pass" and "Context" rules point here. The sentence
  "never reads source" now reads "never reads source; for brownfield groups it
  sends a scout instead".
- This ADR makes a prediction, so that it can fail the way ADR-0029's did:
  **M4's brownfield groups should cost no more than its greenfield ones, and no
  group should need more than one continuation.** If M4's worst group again runs
  past ~400 turns, the diagnosis here was wrong and the scout is not the fix.
