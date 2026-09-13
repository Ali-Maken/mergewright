# m1-the-graph/13 — Lanes at 16 colours and `NO_COLOR`: identity from shape

Status: ready
Build with: **Opus** (high) — design judgment; one mechanism for two constraints
Blocked by: 12b, 12c
Leaves runnable: yes — the graph stays readable in 16-colour terminals and with `NO_COLOR`.

Why: six hues cannot carry twelve lanes, and `NO_COLOR` carries none. One second channel must solve both.

## Read first
- PROMPT.md §1.2 (colour rules only) and §5
- DESIGN.md §4.6
- KNOWN_GAPS.md M0 rows "Twelve lanes collapse to six at 16 colours", "The lane palette is argued, not seen", "Colour depth is inferred from environment strings, not terminfo"
- Code: `crates/graft-ui/src/theme/`, 08's widget

## Scope
In:
- A second channel for lane identity when hue can't carry it: dash pattern, glyph variation, or a hue cycle plus a mark. Pick one; say why the others lost.
- The same mechanism carries `NO_COLOR`.
- 08's paint attribute (dimmed / highlighted) still distinguishable at 16 colours and `NO_COLOR`.
- Settle colour depth: a terminfo lookup (`max_colors`, `RGB`), or an ADR that environment strings are enough — with its consequence for the tier boundary.

Out:
- ASCII (19).

## Acceptance
- [ ] Snapshots of criss-cross, foxtrot, octopus, the widest fixture and a linear history at 256, 16 and `NO_COLOR`, 80×24.
- [ ] A snapshot where two adjacent lanes share a hue at 16 colours and are still told apart.
- [ ] Colour-depth decision recorded as an ADR; the three M0 rows closed or re-owned with a reason.
- [ ] Exact commands prepared for the user to judge in Zed and VS Code on criss-cross and foxtrot. **The by-eye box stays unticked until the user looks.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.
