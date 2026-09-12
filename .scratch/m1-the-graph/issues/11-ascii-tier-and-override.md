# m1-the-graph/11 — The ASCII tier and the renderer override

Status: ready
Build with: Opus
Blocked by: 08, 10

Why: PROMPT.md §8 says all three renderers. ASCII is the honest last resort (`TERM=dumb`, no truecolor, output not a TTY), and the config override plus its escape hatch are what keep a wrong detection from being a dead end — which matters more than usual, because no real terminal had answered a capability probe when these tickets were written.

## Read first
- PROMPT.md §1.1 Tier 3; DESIGN.md §4.2's last row (config override wins)
- Ticket 03's ADR (tier order)
- Ticket 08's renderer — ASCII shares its structure; do not fork it

## Goal
A graph that still reads deliberately with nothing but ASCII, and a user who can always force or drop a tier.

## Scope
In:
- The ASCII glyph set (`* | / \ - +` and a bounded character ramp where density is shown), sharing ticket 08's text-cell renderer rather than duplicating it.
- Lane identity without colour at all: shape and pattern only.
- `graph.renderer = auto | pixel | unicode | ascii` applied at the app layer over what `detect` returned (M0's `detect` deliberately does not apply config).
- The one-key escape hatch that drops a tier when the output looks garbled, and says what it did.
- Piped/redirected output (not a TTY) lands on ASCII without complaint.

Out:
- Pixel encoding (14).
- Changing detection itself.

## Acceptance
- [ ] Snapshots per fixture at 80×24 in ASCII, with and without colour.
- [ ] Forcing each tier from config is snapshot-tested, including forcing `pixel` on a terminal that never acked (it attempts, and the escape hatch works).
- [ ] The escape hatch is reachable from the graph screen and leaves the terminal in a sane state.
- [ ] `graft … | cat` produces ASCII and no escape soup.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
