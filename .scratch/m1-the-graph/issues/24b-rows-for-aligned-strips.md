# m1-the-graph/24b — Rows for aligned pixel strips

Status: ready
Build with: **Opus** (high) — history window and strip cache contract
Blocked by: 24 (partial: `graft-term::kitty` encoder done, commit f313456)
Leaves runnable: yes — nothing visible changes; the pixel tier can build full strips.

Split from 24 (no compaction; 105k tokens, remainder too big). 24's "Handoff" section is the spec: use its findings directly, don't re-derive them.

## Read first
- Ticket 24, "Handoff" section only
- `crates/graft-graph/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-term/MAP.md`
- ADR-0016 (tier order)

## Scope
In:
- `StripCache::get` needs whole 32-row aligned strips; `History.window` holds only what the walk sent (~viewport). Make the pixel tier obtain rows padded to strip boundaries, carrying uncommitted virtual row, fold marker, thread emphasis and theme palette into `Strip`.
- Choose: pad the request in `History::show`, or rasterise short strips with an offset key. Pick the one that keeps the cache key honest; record the choice in the ticket and, if it changes a public item, MAP.md.
- Unicode/ASCII tiers must not pay for the padding.

Out:
- Drawing, replies, status (24c). Flush wiring, F3, real-terminal check (24d).

## Acceptance
- [ ] Tests: a viewport starting mid-strip yields a full aligned strip; the last partial strip at end of history is handled; fold/emphasis/uncommitted row reach `Strip`.
- [ ] Unicode tier requests unchanged (test or assertion).
- [ ] Say what you verified and how.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.
