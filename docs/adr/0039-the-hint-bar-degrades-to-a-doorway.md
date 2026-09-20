# ADR-0039 — The hint bar degrades to a doorway, not to nothing

Date: 2026-09-20. Decided by the user mid-M5, after the group 03 fix round found
the forbidden case. Status: accepted. Refines ADR-0037 §4; changes DESIGN §1.1.

## The problem, stated properly

The hint bar is a horizontal strip, so its width cost grows with the number of
keys it shows: N keys × (key + verb + separator) is 40–60 columns for a useful
bar. Width is the scarce axis in a narrow terminal — an editor side panel, a
split, a panel squeezed by a file tree.

ADR-0037 §4 says the bar must never hide every key. Group 03 satisfied that by
dropping keys from the right until the rest fit. The group 03 **review** then
found the case that breaks: below 100 columns the layout switches to tabs
(DESIGN §1.1), a pane takes the whole screen, the graph screen is not drawn, and
the bar goes with it. Every key hidden — the one outcome the rule forbids — at a
width the user works at.

Spending a row on the bar in tab mode would fix it and cost a line of diff in
every pane at every width. That trade is what the user was asked to make.

**Neither horn was the answer.** Dropping keys trades a width problem for a
discoverability problem; reserving a row pays width everywhere to fix a case that
only arises sometimes.

## The decision

Split the bar into two parts with **different promises**.

1. **The doorway.** One short, fixed marker — `. more`, about seven columns —
   drawn at **every** width, in **every** pane, never dropped. This, and only
   this, is what ADR-0037 §4's "never all hidden" protects. The rule now costs
   seven columns instead of fifty, so it holds everywhere without a special case.

2. **The shortcuts.** The three-to-six most relevant keys for the context, shown
   when there is room, dropped when there is not. Dropping them is safe *because*
   the doorway is not droppable.

Discoverability no longer depends on the shortcuts being visible. It depends on
the doorway being reliable — which is a far cheaper thing to guarantee.

## Three conditions, without which this is a lie

- **`.` must list everything valid in the current context, including every key
  the width rule dropped.** Today it lists none of them (group 03 review,
  Major 1). A doorway that does not lead to what was dropped makes dropping
  unsafe, and that is precisely why the old design had to choose between showing
  everything and showing nothing. This is the real work of this ADR.
- **No row is spent in tab mode.** The doorway rides a line already drawn — the
  `Graph  [Detail]` tab line has dead space on the right. A pane loses no
  content at any width.
- **Shrink before dropping.** When space tightens, lose padding first, then the
  verb, and keep the key itself longest — `space stage` → `space stg` → `space`.
  Zellij degrades in these stages and it reads well; dropping whole items while
  keeping fat separators does not.

## What this changes

- DESIGN §1.1: the sub-100-column tab layout now carries the doorway on its tab
  line. The bar is no longer the only place keys are advertised.
- ADR-0037 §4 still stands, with its guarantee attached to the doorway rather
  than to the whole bar. "Never all hidden" is unchanged as a promise; what
  satisfies it is now affordable.
- `KNOWN_GAPS.md`'s "below 100 columns with a pane open, no hint bar at all"
  closes when this lands, together with the `#[ignore]` on
  `hint_bar::the_tab_layout_still_hides_every_key`.

## Considered and not taken

- **A row reserved for the bar in every pane.** Honest and simple; costs a line
  of content at every width to fix a case that only some widths have.
- **Keys with no verbs** (`space d c`). Fits anywhere, teaches nothing.
- **tig's model — no hints at all, `?` for help.** Coherent, and flatly against
  the user's "a feature nobody can see doesn't exist".
- **Helix's timeout-triggered which-key popup**, where hesitating after a prefix
  pops a vertical menu costing zero width at rest. The better end state, and
  worth revisiting once the doorway's menu is complete: the mechanism here is the
  same one, minus the timeout. Not taken now because it advertises nothing at
  rest unless the trigger is drawn — which is the doorway again.
