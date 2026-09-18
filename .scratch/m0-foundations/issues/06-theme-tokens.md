# m0-foundations/06 — Theme tokens and themes

Status: done (2026-09-12)
Build with: Sonnet
Blocked by: 04

Why: M0 must pick and apply a theme (from the OSC 11 hint ticket 04 provides) so the first screen isn't hard-coded colours; M1's lanes build on the token system.

## Read first
- PROMPT.md §5 (tokens, themes, colour depth, NO_COLOR, lane-palette independence)
- docs/design/04-terminal-capability-detection.md §4.6
- ADR-0003 (OSC 11 is a light/dark hint, never a paint colour)

## Goal
`graft-ui`: the token set, three themes, theme selection from hint + override, and the perceptual colour-depth mapping.

## Scope
In:
- Tokens exactly per PROMPT.md §5: `bg, surface, surface-raised, border, text, text-muted, accent, added, removed, warning, info, lane-1…12`.
- Dark, light and high-contrast themes.
- Lane palette: one fixed, theme-independent, colourblind-safe 12-hue set (start from a published colourblind-safe palette; document the source and choice in a comment). Themes restyle chrome only; a theme cannot touch lane hues.
- `Theme::select(override: Option<...>, hint: core theme hint) -> Theme`: override wins; unknown hint → dark.
- `NO_COLOR` (from `Capabilities`): chrome tokens collapse to no-colour per docs/design/04-terminal-capability-detection.md §4.6. (Graph shape/pattern redundancy is M1.)
- Colour-depth mapping: pure fn truecolor → nearest 256/16 palette entry by perceptual distance (Lab/OKLab), not per-channel rounding (PROMPT.md §5).

Out:
- Rendering/widgets (ticket 09 draws with defaults until 11 wires the theme).
- Theme switching at runtime, OSC 11 re-query on focus (M1+, §4.5).
- Validating lane distinguishability at 16 colours — M1's visual pass; if the mapping collides lanes at 16 colours, note it in KNOWN_GAPS.md (expiry M1), don't solve it here.

## Acceptance
- [x] All 11+12 tokens present in all three themes; snapshot test (`insta`) of the resolved values.
- [x] Selection tests: override beats hint; unknown hint → dark.
- [x] Depth mapping: unit tests on known colours; the 12 lane hues map to 12 distinct entries at 256 colours.
- [x] `NO_COLOR` theme has no ANSI colour in chrome tokens.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Notes (build, 2026-09-12)

- **Token count.** Acceptance said "15+12"; PROMPT.md §5 lists **11** chrome tokens
  (`bg, surface, surface-raised, border, text, text-muted, accent, added, removed,
  warning, info`) plus 12 lanes = 23. Built to PROMPT.md §5, and the ticket text
  is corrected above.
- **Lanes are not tokens.** `Token` has no lane variant and `Theme` has no lane
  field, so "a theme cannot touch lane hues" is structural, not a convention.
  `no_theme_can_change_a_lane_hue` asserts it anyway.
- **Lane palette source:** union of Paul Tol's published CVD-safe schemes
  (SRON/EPS/TN/09-002), filtered to an OKLab lightness band (0.50–0.86, so a lane
  stays legible over ADR-0003's translucent backdrop) and ordered by max-min
  worst-case distance under Machado 2009 protan/deutan/tritan simulation.
  Reasoning and the measured numbers are in `theme/lanes.rs`.
- **Depth mapping weights chroma 2×.** Plain OKLab distance sent near-neutral dark
  chrome to saturated 16-colour entries (`bg #14161B` → navy, `border #333944` →
  maroon: at ΔE ≈ 0.19 a lightness match can outrank a hue match). `CHROMA_WEIGHT
  = 2.0` fixes it; regression test
  `near_neutral_chrome_does_not_pick_up_a_hue_at_sixteen_colours`.
- **256 search skips 0–15** — those indices are user-redefinable, so their colour
  is not something Graft can predict.
- **Gaps filed** (KNOWN_GAPS.md, all expiry M1): lanes collapse 12 → 6 at 16
  colours; dark chrome flattens to one background at 16 colours; the palette is
  verified numerically and as flat swatches, never as thin lines in a terminal
  and never by a CVD user.
