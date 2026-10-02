# Graft: Design — Terminal-capability detection

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Graph layout algorithm](03-graph-layout-algorithm.md) · Next: [Visualization](05-visualization.md)

---

## 4. Terminal-capability detection

Lives in the `term` module (PROMPT.md §6). Runs once at startup, before the
first frame, within a fixed time budget so a terminal that answers nothing
never stalls launch.

### 4.1 Probe sequence

1. **Read cheap environment hints first** (never trusted alone, per
   PROMPT.md §1.1): `$TERM`, `$TERM_PROGRAM`, `$TERM_PROGRAM_VERSION`,
   `$COLORTERM`, `$KITTY_WINDOW_ID`, `$TMUX`, `$SSH_TTY`/`$SSH_CONNECTION`,
   `$NO_COLOR`.
2. **Enter raw mode and send active queries together**, then read whatever
   arrives before a deadline (200 ms locally; §4.4 raises this over SSH):
   - Kitty graphics protocol: an APC query action (`a=q`); look for the
     terminal's acknowledgement.
   - Sixel: `DA1` (`CSI c`); look for parameter `4` in the reply.
   - Cell size in pixels: `CSI 16 t`; parse `CSI 6 ; height ; width t`.
   - Kitty keyboard protocol: `CSI ? u`; a reply means support.
   - Synchronized output (DEC 2026): `CSI ? 2026 $ p` (DECRQM); read the
     reported mode, purely for diagnostics — begin/end sync sequences are
     emitted unconditionally either way since an unsupported terminal just
     ignores the unknown CSI harmlessly. Each frame's text **and** image
     commands sit inside one begin/end pair (ADR-0054).
   - Device status (DSR): `CSI 5 n`; a `CSI 0 n` reply means the terminal can
     answer the per-frame fence (§4.7, ADR-0054). Silence: no fence this run.
   - `OSC 11`: background colour, for the light/dark theme hint only (ADR-0003
     — never used as a paint colour).
3. **Fall back per-query, not globally**, on silence or a malformed reply:
   - Cell size: fall back to `TIOCGWINSZ` (character rows/cols plus pixel
     width/height, when the terminal fills those fields); if that's also
     zero, assume 8×16 px and mark the value "approximate" (visible in a
     diagnostics view, never silently treated as exact).
   - `OSC 11` silence: default to the dark theme, flagged "unknown" so a
     manual override (`ui.theme`) is easy to find.
   - Kitty keyboard / synchronized output silence: assume unsupported;
     both are enhancements, never required for correctness.
4. **iTerm2 is the one tier allowed to rely on an environment hint alone**
   (`$TERM_PROGRAM == "iTerm.app"` plus a minimum `$TERM_PROGRAM_VERSION`),
   because iTerm2's inline-image support has no query protocol — PROMPT.md
   §1.1 explicitly permits this one exception.

### 4.2 Tier decision

The order is **Kitty → Unicode → iTerm2 → Sixel → ASCII** (ADR-0016). Unicode
is not a fallback: it is what Graft picks for every terminal that is not
running the Kitty graphics protocol, because the terminals the graph is read
in most days — Zed's and VS Code's — have no image protocol at all. Kitty is
the one protocol that outranks it, because it is the one verified end to end
(ADR-0012, and the M0 checkpoint's real Ghostty run).

Two separate questions, decided separately:

**Which pixel protocol is reachable**, highest first. Reported in
`Capabilities::pixel_protocol` whatever tier is chosen, because it is a fact
about the terminal and because `graph.renderer = "pixel"` needs to know what
to force.

| Condition | Protocol |
|---|---|
| Kitty graphics query acknowledged | **Kitty** |
| iTerm2 hint matched (§4.1 step 4) and image bytes reach the terminal (§4.3) | **iTerm2** |
| Sixel indicated by `DA1` | **Sixel** |
| None of the above | none |

**Which tier gets drawn.** First row that matches wins.

| Condition | Tier |
|---|---|
| `graph.renderer` set in config | Whatever it says — always wins over detection. Forcing `pixel` on a terminal that didn't ack the query still attempts it, using the reachable protocol above, with a one-key escape hatch to drop a tier if the output looks garbled |
| Kitty protocol reachable | **Pixel**, via Kitty |
| Truecolor indicated (`COLORTERM=truecolor`/`24bit`, or a known-truecolor terminal) | **Unicode** |
| Inside tmux, with no graphics protocol through the passthrough (§4.3) | **Unicode** — not ASCII: tmux itself draws Unicode fine, and the missing capability is the image transport, not the character set (ADR-0014) |
| iTerm2 or Sixel reachable, and neither of the two rows above matched | **Pixel**, via that protocol — an image protocol with no truecolor beats ASCII, which is the only comparison it has to win (ADR-0016) |
| Anything left, or `TERM=dumb`, or output isn't a TTY | **ASCII** |

iTerm2 and Sixel move above Unicode only when a `TESTING.md` manual-checklist
row, run by the user in that terminal with both tiers on screen, says the
pixel graph is better to look at and loses nothing. Until then they stay
below it, and ADR-0016 is superseded — not edited — if the answer changes.

### 4.3 tmux

- Detected via `$TMUX`.
- Passthrough for the Kitty protocol needs tmux ≥ 3.3 with
  `allow-passthrough` set, and depends on the *outer* terminal too. Graft
  checks the tmux version and then re-sends the capability probe wrapped in
  tmux's passthrough sequence (`\ePtmux;...\e\\`), applying the same
  per-query fallback as §4.1.
- **Every pixel protocol fails closed inside tmux, iTerm2 included.** §4.1's
  env-hint exception describes the *outer* terminal, and says nothing about
  whether tmux will carry the image bytes to it, so under `$TMUX` the iTerm2
  hint alone is not enough: the tier is pixel only if the passthrough re-probe
  answered (ADR-0014).
- If the wrapped probe fails, times out, or the tmux version is too old,
  Graft settles on the Unicode tier — deliberately, not by hoping passthrough
  happens to work. That is the ordinary tier for a terminal without the Kitty
  protocol (§4.2, ADR-0016), not a consolation for a failed probe. It is also
  what PROMPT.md §7.1's manual-checklist pass condition ("a correct pixel
  graph *or* a clean fallback, never a broken screen") requires, and it's a
  decision made once at startup, not a per-frame guess.

### 4.4 SSH

- Detected via `$SSH_TTY`/`$SSH_CONNECTION`.
- No special-cased tier logic: a locally Kitty-capable terminal SSH'd into a
  remote host still relays escape sequences correctly in the common case, and
  the same probe-timeout fallback in §4.1 degrades gracefully when it
  doesn't. The one adjustment is a longer probe deadline (500 ms instead of
  200 ms) to absorb one extra network round trip before giving up on a query.
- **The deadline is per probe round, not per startup** (ADR-0014). §4.3's
  tmux re-probe is a second round and gets a full budget of its own, so a slow
  first round cannot starve it into misreporting passthrough. Two silent
  probe rounds cost 400 ms locally, 1 s over SSH, and only inside tmux.
- **Inside tmux, `tmux -V` is the third bound**, and the largest. Graft runs
  it when `$TMUX` is set and tmux has not published its version in
  `$TERM_PROGRAM_VERSION`, and waits at most 200 ms — the same scale as a
  probe round, because tmux is a local binary printing one short line, and a
  startup budget measured in seconds is not a budget. So the true worst case
  before the first frame is 200 ms (`tmux -V` wedged) + two silent rounds:
  **600 ms locally, 1.2 s over SSH**, and only inside tmux. Outside it,
  200 ms / 500 ms.

### 4.5 Re-detection triggers

- **Tier** is decided once per session; it doesn't change mid-session.
- **Cell size** (`CSI 16 t`) is re-queried on resize (`SIGWINCH`), since a
  font-size change or a window drag to a different-DPI display changes
  pixels-per-cell without changing the tier.
- **`OSC 11`** is re-queried on focus regain (piggybacking on the existing
  refresh-on-focus behaviour), so a terminal theme switched while Graft was
  in the background is picked up without a restart.

### 4.6 `NO_COLOR` and colour-depth fallback

- `NO_COLOR` (any non-empty value) disables ANSI colour in the chrome
  outright. For the graph specifically, lane identity falls back to the same
  shape/pattern redundancy already required for colourblind safety (distinct
  node shapes, and — for lines — distinct dash patterns per lane) rather than
  changing the tier: `NO_COLOR` is about colour, not about protocol
  capability, so a pixel-tier terminal keeps drawing real graphics, just
  without colour.
- When truecolor isn't available but colour is (`NO_COLOR` unset), Graft
  checks terminfo/`$TERM` capabilities for 256- or 16-colour support and maps
  each lane's truecolor hue to the nearest palette entry using a perceptual
  distance (not naive per-channel rounding), so adjacent lanes stay
  distinguishable at any colour depth — the same colourblind-safe palette
  constraint from PROMPT.md §1.2 applies at every depth, not just truecolor.

### 4.7 Frame pacing (M8, ADR-0054)

The loop never writes ahead of the terminal.

- Every frame is `SYNC_BEGIN` · text · image commands · `SYNC_END` · `CSI 5 n`.
- The next frame waits for that frame's `CSI 0 n`. Wakes that arrive meanwhile are applied to state, not drawn; the
  answer releases one frame showing the latest state.
- A fence unanswered after 500 ms draws anyway; three in a row stop fencing for the run, and frames are then capped
  at one per 16 ms. A terminal that did not answer the startup DSR (§4.1) never fences.
- Wheel events applied in one turn sum into one scroll.
- Inside tmux, tmux answers the DSR itself: the fence paces Graft against tmux, which paces its own output.

---
