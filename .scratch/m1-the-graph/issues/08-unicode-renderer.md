# m1-the-graph/08 — The Unicode renderer (the tier the user lives in)

Status: ready
Build with: Opus
Blocked by: 03, 05

Why: the user works in the Zed and VS Code terminals, which have no image protocol. **This is the graph he sees most days.** KNOWN_GAPS.md is explicit: build it as a first-class tier, designed rather than degraded, judged by eye to the same standard as the pixel tier. Nothing in the code, the docs or the UI may describe it as a fallback. It is also where the 16-colour lane collapse has to be solved.

## Read first
- PROMPT.md §1.1 Tier 2 and §1.2's colour rules; §5 (visual design system)
- DESIGN.md §1.1–§1.3 mockups (the graph column in all three widths) and §4.6
- Ticket 03's ADR (tier order)
- KNOWN_GAPS.md M0 rows: "Twelve lanes collapse to six at 16 colours", "The lane palette is argued, not seen", "Colour depth is inferred from environment strings"
- `crates/graft-ui/src/theme/` (tokens, lane palette, depth mapping) as it stands

## Goal
A Unicode graph that looks like a designed diagram, not like `git log --graph`, at truecolor, 256 and 16 colours, with and without `NO_COLOR`.

## Scope
In:
- Rounded box-drawing (`╭ ╮ ╰ ╯ │ ─ ├ ┤`) and node glyphs (`● ○ ◉ ◆`), with fork and merge elbows that read as curves, drawn from `Row` data only.
- Per-lane colour from branch identity (stable across scrolls and refreshes), mapped perceptually at each colour depth.
- **Lane identity at 16 colours must come from shape, not hue** — six distinct colours cannot carry twelve lanes. Decide the second channel (dash pattern, glyph variation, or a repeating hue cycle plus a distinguishing mark) and judge it by eye. The same redundancy carries `NO_COLOR` (DESIGN.md §4.6), so it is one mechanism, not two.
- Special commits (merge, root, HEAD, stash, uncommitted) differ in **shape**, never in colour alone.
- The `+N` compaction column and folded-branch indicator from ticket 07.
- Width-aware: the graph column must stay legible in the 80×24 narrow layout.
- **Decide the colour-depth question** KNOWN_GAPS.md defers to "once M1 knows what the renderers need": either add a terminfo lookup (`max_colors`, `RGB`) or record a deliberate decision that the env strings are enough. Whichever you choose, write it down — an under-reported depth costs a tier.

Out:
- Ref badges, subject, author, date columns (ticket 09) — this ticket owns the graph column.
- ASCII (11), pixel (13/14).
- Focus-mode dimming (10) — but leave the per-row paint attribute the renderer will need.

## Acceptance
- [ ] `TestBackend` + `insta` snapshots for every fixture shape at 80×24, 120×40 and 200×60, dark and light, at truecolor / 256 / 16 / `NO_COLOR`. Snapshots must be readable as pictures by a human reviewer.
- [ ] At 16 colours, two lanes that share a hue are still told apart — demonstrate it in a snapshot, not by a distance number.
- [ ] **Judged by eye by the user** in the Zed and VS Code terminals against the criss-cross and foxtrot fixtures. This acceptance box stays unticked until that happens; a snapshot is not evidence of legibility (CLAUDE.md: green gates are not evidence). Prepare the exact command for the checkpoint.
- [ ] The colour-depth decision is recorded (ADR or a stated note) with its consequence for the tier boundary.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
