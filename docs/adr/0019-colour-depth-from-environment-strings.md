# Colour depth comes from environment strings, not terminfo

Decision: **Graft keeps reading colour depth from `COLORTERM`, `TERM` and a
short list of known truecolor terminals. It does not look up terminfo
(`max_colors`, `RGB`, `Tc`).** Closes the M0 gap "Colour depth is inferred
from environment strings, not terminfo".

## Why

- **Terminfo does not answer the question that costs a tier.** DESIGN.md
  §4.2 puts the Unicode/ASCII boundary at *truecolor*. The terminfo
  capability for that, `RGB`, is present almost only in `*-direct` entries,
  which `detect` already recognises from `TERM` alone. `Tc` is a tmux
  extension, not in the shipped database. `max_colors` separates 256 from
  16, and that never moves the tier.
- **`TERM` lies where terminfo would be read.** Over SSH and inside tmux,
  `TERM` is `xterm-256color` or `tmux-256color` whatever the real terminal
  is, so terminfo describes the name, not the terminal. `COLORTERM` is the
  one signal terminals set on purpose, and Zed and VS Code — where the graph
  is read most days — set it.
- **It would cost a dependency and file reads at startup** for no case the
  env strings get wrong today.
- **Lanes no longer need the exact depth to stay readable.** At 16 colours
  and with `NO_COLOR`, lanes in odd columns draw dashed (m1/13), so a
  depth reported one step low loses hue, not lane identity.

## Consequences

- A truecolor terminal that sets no `COLORTERM`, no `-direct` `TERM`, and is
  not on the list is under-reported, and outside tmux it gets the ASCII tier
  (once m1/19 lands). The fixes are the user's: `export COLORTERM=truecolor`,
  or `graph.renderer = "unicode"`. The status screen (F2) shows the depth,
  so an under-report is visible, not silent.
- A 16-colour Unicode graph is reached only inside tmux or through the
  `graph.renderer` override; that is where the dashed lanes matter most.
- Revisit if a terminal the user relies on is under-reported. Add it to the
  list first; terminfo only if the list stops scaling.
