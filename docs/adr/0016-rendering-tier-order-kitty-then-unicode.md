# The rendering-tier order: Kitty, then Unicode

Decision: the preference order is **Kitty → Unicode → iTerm2 → Sixel →
ASCII**. Unicode is not a fallback; it is the tier Graft picks for every
terminal that is not running the Kitty graphics protocol. iTerm2 and Sixel
sit *below* Unicode and are chosen only where they demonstrably beat it —
today that means only where the alternative is ASCII.

This overrides the approved DESIGN.md §4.2, which said Kitty > iTerm2 >
Sixel and treated Unicode as what you get when no image protocol answers.
`KNOWN_GAPS.md` recorded the conflict at the step-3 fix and required an ADR
before anything was built on either order; this is it.

## Why

**The user's daily terminals have no image protocol.** He works in the Zed
and VS Code terminals. Neither acks a Kitty graphics query, so under the old
order the graph he sees every day was the one the design called the
downgrade path. A tier that is chosen for the terminals nobody uses is not a
preference, it is a decoration.

That reverses the usual reasoning. The question is not "what is the richest
thing this terminal could draw?" but "which tier is worth the effort of
making excellent, and which one does the user actually look at?". The answer
is the same tier: Unicode. So the order follows the answer, and M1 ticket 08
builds Unicode as a first-class renderer, judged by eye to the same standard
as the pixel tier.

**Kitty is the one exception, and it earns it.** The M0 checkpoint ran the
real probe in Ghostty: the terminal answered, reported 17×37 px cells and
truecolor, and the spike (ADR-0012) drew an aligned, scrolling pixel graph
through the Kitty protocol at `q=1` with zero errors. It is the only pixel
path verified end to end, and the gap between a real image and box-drawing
characters is large enough to be worth a second renderer. Where Kitty
answers, Graft uses it.

**iTerm2 and Sixel are ranked below Unicode because nothing shows they beat
it.** iTerm2's inline images work — the user ran the spike in a real iTerm2
on 2026-09-12 and the graph drew correctly — but "works" is not "better than
a Unicode graph that does not exist yet". Sixel has never been drawn at all
(`KNOWN_GAPS.md`: "Sixel and plain-text rendering tiers unproven"). Ranking
an unproven tier above the daily one is how the step-3 failure happened.

## What "demonstrably beats Unicode" means

It means one specific thing, not a judgement call: **a manual-checklist row,
run by the user, in that terminal, with both tiers on screen.**

The procedure is in `TESTING.md`, "Does a pixel tier beat Unicode here?":
run Graft twice on the same repository at the same window size, once with
`graph.renderer = "unicode"` and once with `graph.renderer = "pixel"`, and
answer yes/no with a reason. A yes is only a yes if the pixel tier is better
*to look at* and does not lose anything — alignment at row boundaries,
survival of a resize, no stray sequences, no scrollback damage.

Until a row says yes for a protocol, that protocol does not take the tier
from Unicode. Nobody promotes a protocol from a benchmark, from a spec
sheet, or from the fact that it emitted the right bytes: emission is not
rendering, which is exactly what ADR-0012's Correction had to record.

## When more than one protocol answers

One terminal can ack Kitty, list Sixel in `DA1`, and be iTerm2, all at once.
Two separate questions, answered separately:

1. **Which protocol is reachable?** Highest first: Kitty, then iTerm2, then
   Sixel. This is reported in `Capabilities::pixel_protocol` whatever the
   tier ends up being, because it is a fact about the terminal, and because
   `graph.renderer = "pixel"` needs to know which protocol to force.
2. **Does it take the tier?** Only Kitty does, by this ADR. iTerm2 and Sixel
   take the tier only when the alternative is ASCII — a terminal with an
   image protocol and no truecolor is better served by pixels than by
   `|`/`*`, and that comparison is not close enough to need a checklist row.

So an iTerm2 window reports `Unicode` as its tier and `Iterm2` as its
reachable protocol, and the config override turns the second into the first
for anyone who wants it. Nothing is hidden, and nothing is chosen on a hope.

## The escape hatch stays in both directions

`graph.renderer` still wins over detection (DESIGN.md §4.2, last row).
A user who prefers iTerm2 pixels sets `renderer = "pixel"` and gets them;
a Kitty user who dislikes the pixel graph sets `renderer = "unicode"`. The
order decided here is the *default*, which is the only thing a detection
rule can ever be.

## Consequences

- DESIGN.md §4.2's table is rewritten around this order, and §4.3's "falls
  back to the Unicode tier" becomes "settles on": inside tmux without
  passthrough, Unicode is the ordinary answer, not a consolation.
- `detect::tier` picks the pixel tier for Kitty, for iTerm2/Sixel only over
  ASCII, and reports the reachable protocol either way.
- M1 ticket 08 builds the Unicode renderer as the primary one. Ticket 14
  builds Kitty and iTerm2 transports; Sixel stays unbuilt — this ADR does
  not keep it, so per ticket 14 it goes to `KNOWN_GAPS.md` rather than being
  written on spec.
- No document may call the Unicode tier a fallback or a downgrade.

## What would reopen this

This order is the user's stated preference, recorded before the Unicode
tier existed to compare against. Two answers could contradict it, and if
either does, **this ADR is superseded by a new one, not quietly edited**:

- **VS Code's `terminal.integrated.enableImages`.** If it gives a usable
  Sixel tier, one of the two daily terminals is not image-less and the whole
  premise moves. `TESTING.md` carries the question; only the user can answer
  it.
- **The manual checklist itself.** If the pixel tier in iTerm2 turns out to
  be plainly better than the finished Unicode graph, the order says so:
  promote it, with the checklist row as the evidence.
