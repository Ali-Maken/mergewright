# m0-foundations/06 — Theme tokens and themes

Status: todo
Build with: Sonnet
Blocked by: 04

Why: M0 must pick and apply a theme (from the OSC 11 hint ticket 04 provides) so the first screen isn't hard-coded colours; M1's lanes build on the token system.

## Read first
- PROMPT.md §5 (tokens, themes, colour depth, NO_COLOR, lane-palette independence)
- DESIGN.md §4.6
- ADR-0003 (OSC 11 is a light/dark hint, never a paint colour)

## Goal
`graft-ui`: the token set, three themes, theme selection from hint + override, and the perceptual colour-depth mapping.

## Scope
In:
- Tokens exactly per PROMPT.md §5: `bg, surface, surface-raised, border, text, text-muted, accent, added, removed, warning, info, lane-1…12`.
- Dark, light and high-contrast themes.
- Lane palette: one fixed, theme-independent, colourblind-safe 12-hue set (start from a published colourblind-safe palette; document the source and choice in a comment). Themes restyle chrome only; a theme cannot touch lane hues.
- `Theme::select(override: Option<...>, hint: core theme hint) -> Theme`: override wins; unknown hint → dark.
- `NO_COLOR` (from `Capabilities`): chrome tokens collapse to no-colour per DESIGN.md §4.6. (Graph shape/pattern redundancy is M1.)
- Colour-depth mapping: pure fn truecolor → nearest 256/16 palette entry by perceptual distance (Lab/OKLab), not per-channel rounding (PROMPT.md §5).

Out:
- Rendering/widgets (ticket 09 draws with defaults until 11 wires the theme).
- Theme switching at runtime, OSC 11 re-query on focus (M1+, §4.5).
- Validating lane distinguishability at 16 colours — M1's visual pass; if the mapping collides lanes at 16 colours, note it in KNOWN_GAPS.md (expiry M1), don't solve it here.

## Acceptance
- [ ] All 15+12 tokens present in all three themes; snapshot test (`insta`) of the resolved values.
- [ ] Selection tests: override beats hint; unknown hint → dark.
- [ ] Depth mapping: unit tests on known colours; the 12 lane hues map to 12 distinct entries at 256 colours.
- [ ] `NO_COLOR` theme has no ANSI colour in chrome tokens.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
