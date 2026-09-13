# m1-the-graph/19 — The ASCII tier and the renderer override

Status: ready
Build with: **Sonnet** (medium) — a glyph table over an existing renderer, a config key, a key binding
Blocked by: 13
Leaves runnable: yes — `graph.renderer = ascii` works, and a key drops a tier when output looks garbled.

Why: a user must always be able to force or drop a tier, and piped output must never be escape soup.

## Read first
- PROMPT.md §1.1 (the ASCII tier paragraph only)
- DESIGN.md §4.2 and §4.6
- ADR-0016
- Code: 08's widget and 13's shape channel, `crates/graft-term/src/detect/` (tier decision, public API only), the config module

## Scope
In:
- ASCII glyphs (`* | / \ - +`) as a glyph table in 08's renderer, not a second renderer.
- Lane identity from 13's shape channel; `+N` drawn in ASCII.
- `graph.renderer = auto | unicode | ascii`, applied at the app layer over what `detect` returned. (`pixel` is added by 24.)
- One key drops one tier and says what it did in the status line.
- Non-TTY stdout lands on ASCII without complaint.

Out:
- Pixel tiers (22–26). Changing detection.

## Acceptance
- [ ] Snapshots of criss-cross, foxtrot, octopus and the widest fixture in ASCII at 80×24, with and without colour.
- [ ] Each `graph.renderer` value forced from config is snapshot-tested.
- [ ] The tier-drop key works from the graph screen and leaves the terminal sane (pty).
- [ ] `graft <fixture> | cat` output contains no escape bytes (asserted).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
