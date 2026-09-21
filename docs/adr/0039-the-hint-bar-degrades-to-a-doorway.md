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

Restated 2026-09-21 (m5/27) to read straight: the rule as the three
amendments below leave it, stated once rather than in the sequence that
arrived at it. The history section keeps the trail.

Edited in place 2026-09-21 (m5/28), not appended to, with the three things
the third amendment left open or unsaid: where the doorway sits, what
happens below the floor, and why the bare-key rung stays. A fourth
amendment on top of a clean restatement is the autopsy growing back.

Split the bar into two parts with **different promises**.

1. **The doorway.** One short, fixed marker — `. more`, about seven columns —
   drawn at **every** width, in **every** pane the doorway has work to do in
   (a modal pane is the exception, below). This, and only this, is what
   ADR-0037 §4's "never all hidden" protects. The rule now costs seven
   columns instead of fifty, so it holds everywhere without a special case.

   **Fixed 2026-09-21 (m5/29), below 100 columns as everywhere else.** The
   detail pane taking the whole screen behind a `[Graph] Detail` tab strip
   (`update::detail::effective_layout` → `DetailLayout::Tab`) used to leave
   the doorway with nowhere to draw, since the graph screen — and its
   footer — is not on the tab layout at all. The tab layout now draws its
   own bottom row for the pane's own keys
   (`update::detail::tab_hint_rows`, `view::frame::draw_tab_hints`), through
   the same `hints_for` ladder and floor every other bar uses. The test that
   used to fail on purpose, `crates/graft/tests/it/hint_bar.rs`, is renamed
   to `the_tab_layout_draws_the_panes_own_keys` and passes.

   **Where it sits: at the head of the bare keys — which is the end of the
   row when there are none.** One sentence, two layouts, no special case.
   A graph row ends in a run of bare keys and `. more` leads that run, so
   everything after it reads as *keys the menu will spell out*; the changes
   pane and the journal browser draw every hint whole, so the doorway stays
   last. The doorway's promise is that it is **reliable** — never dropped,
   never clipped — and reliable is not the same as last. It was last until
   2026-09-21 (m5/28) and `ends_with(". more")` was pinned by three tests
   and several captions; the promise those pinned is unchanged, only where
   it sits.

2. **The shortcuts.** The context's own keys, shown when there is room,
   dropped when there is not. Dropping them is safe *because* the doorway is
   not droppable.

Discoverability no longer depends on the shortcuts being visible. It depends on
the doorway being reliable — which is a far cheaper thing to guarantee.

Four rules govern how the shortcuts fill the room they have:

- **The bar fills from the most relevant end.** Each hint takes the best rung
  that still leaves room for the hints ahead of it; a hint that will not fit
  even at its bare key is dropped. This is decided **per hint, never as a
  step the whole list takes together** — the list-wide version was built,
  measured and reverted (third amendment, below).
- **The floor, and what is below it.** The three most relevant hints never
  fall below their floor rung — their short rung where the user's table
  gives one, their whole verb where it does not. A floor that cannot be met
  is not a floor, so the sequence is stated to the bottom:

  ```
  floor  →  `. more`  →  `.`  →  nothing
  ```

  The bare-key rung applies to the doorway like anything else: one column
  still says a key exists. Below that, empty. **The bar never draws a
  fragment of itself at any width** — a clipped `. mo` on the one element
  meant to be reliable is worse than an empty bar, and worse than a squeeze,
  because nobody chose where it cut. This composes with the doorway's
  position: as width shrinks the tail goes, then the labels, and `. more` is
  what remains; its position stops mattering exactly where it starts being
  the only thing there. **Design completeness, not a live defect** — the
  narrowest bar anything reachable actually has is 48 columns
  (`graft_ui::detail_pane::layout`'s `RIGHT_MIN`; below 100 the pane is a
  full-width tab, which since m5/29 draws its own bar through this same
  ladder — point 1), and the tightest context's floor fits in 38.

- **The bare-key rung stays.** Letting the ladder stop at the short rung and
  dropping bare keys entirely was proposed and rejected: the tail's
  membership is itself information. On another branch's tip it carries `m`
  and `r`; on an ordinary commit it does not. That is context the menu only
  gives you after a keypress. **The menu covers discovery; the bar covers
  recall.**
- **A modal pane draws its own keys, not the doorway.** Where a pane answers
  every key itself, `. more` would advertise a key the pane swallows, and the
  pane's own list is already complete, so nothing needs recovering.
- **Widths are counted from the bar's own width, never the screen's.** A pane
  can leave the bar narrower than it is with no pane open at all; keying the
  count to the screen gives less help on the bigger terminal, which is wrong
  in the direction that matters.

**No abbreviation scheme.** The ladder's bare-key rung is already the
mechanism for "no room" — a verb the short-form table does not name simply
draws whole or not at all, never invented or squeezed to fit (m5/27,
`crates/graft-ui/src/graph_screen/hints.rs`'s `short_form`).

## Three conditions, without which this is a lie

- **`.` must list everything valid in the current context, including every key
  the width rule dropped.** Today it lists none of them (group 03 review,
  Major 1). A doorway that does not lead to what was dropped makes dropping
  unsafe, and that is precisely why the old design had to choose between showing
  everything and showing nothing. This is the real work of this ADR.
- **Below 100 columns, one row is spent — reused where it can be.** m5/29
  found the doorway had nowhere to ride below 100 columns (point 1 above)
  and gave the tab layout a bottom row of its own
  (`update::detail::tab_hint_rows`). That row is skipped, not doubled, when
  the operation banner already draws its own key-hint row at the focused
  pane's foot (`tab_banner_rows(state) < 2`) — so the cost is one line, only
  in tab mode, only when nothing else is already spending it.
- **Shrink before dropping.** When space tightens, lose padding first, then the
  verb, and keep the key itself longest — `F3 renderer` → `F3 rend` → `F3`.
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
  of content at every width to fix a case that only some widths have. **What
  was taken instead, 2026-09-21 (m5/29):** a narrower version of this same
  idea — one row, not in every pane but only in the tab layout, and only
  below 100 columns where the doorway has no footer to ride at all (point 1
  above). The width-everywhere cost this bullet rejects never happened;
  the row is spent only where the violation actually was.
- **Keys with no verbs** (`space d c`). Fits anywhere, teaches nothing.
- **tig's model — no hints at all, `?` for help.** Coherent, and flatly against
  the user's "a feature nobody can see doesn't exist".
- **Helix's timeout-triggered which-key popup**, where hesitating after a prefix
  pops a vertical menu costing zero width at rest. The better end state, and
  worth revisiting once the doorway's menu is complete: the mechanism here is the
  same one, minus the timeout. Not taken now because it advertises nothing at
  rest unless the trigger is drawn — which is the doorway again.

## History

The decision above is what these three amendments collectively arrive at,
restated once rather than read as an autopsy (CLAUDE.md, m5/27). Kept
unedited, in order, for the trail.

## Amendment, 2026-09-21: a modal pane draws its own keys, not the doorway

M5 ticket 16 put the operation banner above the tab line so a modal overlay is
visible wherever it has focus. The builder then drew **no `. more`** under a
focused banner, and asked for the deviation to be confirmed. Confirmed, and the
reasoning belongs here rather than in a doc comment — a rule that lives only in
code gets "fixed" by the next person who reads this ADR.

**The doorway is drawn at every width in every pane, except where the pane is
modal and its own list is complete.**

Two rules meet here and the older one wins:

- This ADR says the doorway is never dropped. Its **purpose** is that dropping
  shortcuts is safe *because* the doorway recovers them.
- `docs/design/02-keymap.md` §2.1 says a displayed key must work when pressed.

A modal pane answers every key itself, `Ctrl+C` excepted — so it swallows `.`.
Drawing `. more` there advertises a key that does nothing, which is the failure
this project treats as worse than absence. And the doorway has no work to do:
the focused banner's whole list fits at 80 columns, so nothing was dropped for
it to recover.

So the guarantee is unchanged in substance — **the user is never left with no
way to see what is valid here**. Under a modal pane that promise is kept by the
pane showing its own complete list, which is strictly better than a doorway.
Where a list is *not* complete, the doorway returns, modal or not.

This does not loosen the rule for the graph screen, the panes or the tab
layout, where `. more` remains undroppable and is pinned by 20 tests going red
when it is removed.

## Amendment, 2026-09-21: count from the bar's own width, never the screen's

Ticket 19 fixed an inverted footer — 120 columns drew 116 cells of wording and
160 drew 47 — by replacing width tiers with "the widest wording that fits". It
then found the **same miscalibration unfixed in `hints_for`**: three hints at
160 columns against six at 120. It stopped and asked rather than changing it,
on the grounds that the 3/6/all numbers trace to ADR-0037 §4 and are therefore
a spec matter. Correct call, and the answer is that they never were.

**ADR-0037 §4 says the bar shows "the three most relevant keys for the current
context and drops or abbreviates the rest. Never all hidden."** It sets a
floor and a rule. It says nothing about screen width, and the 3/6/all tiers
keyed to the screen are an implementation invented on top of it.

**The rule: the bar shows as many hints as fit in the room the bar actually
has.** Not the screen's width — the bar's own region. A pane can take 42% of a
160-column screen, which leaves the bar narrower than it is at 120 with no
pane; keying the count to the screen is then wrong in the direction that
matters, giving the user *less* help on the bigger terminal.

Unchanged: ADR-0037 §4's floor of three most relevant keys; never all hidden,
which is the doorway's job; and shrink before dropping (`space stage` →
`space stg` → `space`), which applies before any hint is dropped at all.

Also unchanged: where a modal pane draws its own complete list, the doorway
does not appear — the amendment above this one.

The general form, and the reason both of these were wrong the same way: **ask
how much room this thing has, not how big the screen is.** Any place that
branches on screen width to decide how much to say is suspect; there is at
least one more in the tree, and tickets should look for it rather than wait
for a capture to expose it.

## Amendment, 2026-09-21 (third): the ladder degrades per hint, from the least relevant end — correcting the amendment above

**The amendment above this one is wrong in one clause, and it is mine.** It
says shrink before dropping "applies before any hint is dropped at all". Read
list-wide — every hint steps down a rung together, and only when all of them
are on their key does anything drop — that clause **empties the bar**.

Ticket 24 built exactly that clause, measured it, and reverted rather than ship
it. The numbers: `drawable()` appends `TAIL`'s screen-wide keys to every row
context (`graph_screen/hints.rs:224-230`), so a row context is **22 hints**.
Whole list plus `. more`: **227 cells full, 190 short, 81 key alone.** Below 190
columns no rung but "key alone" fits, so the list-wide ladder goes straight
there and draws this at 80, 120 **and** 160:

```
 ⏎  c  y  u  S  J  /  q  F2  F3  x  +  j/k  space  1  F  H  f  p  P  M  O  . more
```

The 160-column terminal teaches exactly as much as the 80-column one: nothing.
That is this ADR's own rejected option — *"Keys with no verbs (`space d c`).
Fits anywhere, teaches nothing."*

**Why the clause was wrong.** This ADR's worked example is the changes pane at
**7** hints, where `space stage → space stg → space` shrinks sensibly. `TAIL`
landed afterwards and made every row context 22. "Shrink everything before
dropping anything" then means "shrink 22 hints to bare keys rather than drop the
22nd", which inverts the purpose. A rule written for a short list was applied to
a long one — the same mistake shape as the width tiers two amendments ago, in a
different dimension.

**The rule, corrected.** The bar fills **from the most relevant end**. Each hint
takes the best rung that still leaves room for the hints ahead of it. A hint that
does not fit even at its key alone is **dropped**. Shrinking is per hint, against
that hint's own place in the order — never a step the whole list takes together.

**Floor, unchanged:** ADR-0037 §4's three most relevant keys never degrade below
their **short** rung, and the bar is never all hidden.

This keeps every property the bar already has. It is monotone in the bar's own
width — more room can only improve a hint's rung or add one, never the reverse
— so `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` and
`no_footer_line_gets_terser_as_the_room_for_it_grows` still hold. And the
leading verbs stay full at 80 columns, which is what
`eighty_columns_leads_with_three_and_fills_the_row` has always pinned.

**Open, and the user's by eye (rule 6):** whether a *tail* of bare keys after
readable leading verbs earns its place, or whether those hints should simply be
dropped. The rejected option above was the whole bar as bare keys; a tail after
`⏎ open  c commit  P push` is a different picture, and a capture is the only
honest way to answer it. Built with the key-alone rung in, for him to judge.
