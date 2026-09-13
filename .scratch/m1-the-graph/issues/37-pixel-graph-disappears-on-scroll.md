# m1-the-graph/37 — Pixel graph disappears after scrolling (git/git)

Status: ready
Build with: **Opus** (high) — terminal graphics protocol, strip cache, background history
Blocked by: 30c
Leaves runnable: yes — scrolling git/git in the pixel tier never leaves rows without their graph.

Why: user's gallery walk, 2026-09-13, stop 23/23 `git-git` (pixel tier, "gfx ok", 384 rows so far, tooltip pinned): "this page was buggy as hell, and upon scrolling a bit, the graph disappears." Screenshot: `docs/shots/user-37-git-git-pixel-after-scroll.png` (look at it). Also visible there: the last row reads `### match next` in the message column — find out what that is (search prompt or hint bleeding into a graph row?) and fix it if it's wrong.

Correctness bug in the shipped graph screen, not only the gallery: reproduce in the real binary too (`cargo run -p graft -- ~/.cache/graft/git-git.git`).

## Read first
- Tickets 30b, 30d result notes (strip cache key, strips wait for the cursor's row, eviction), 27c/27d notes (promotion swap), 36 notes (pin keyed by commit)
- KNOWN_GAPS rows: "43 strips re-rasterised after eviction", "30b blank frame on PgDn"
- `crates/graft-term/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
- Reproduce first, under a pty, in the pixel tier: git/git, scroll with j, PgDn, wheel; (the pinned-tooltip case is void: ADR-0023 removes it); during and after background loading and the promotion swap. Record which input and which state loses the graph.
- Find the cause (strip deleted/evicted but not redrawn? placement ids reused? tooltip overlay clearing images? swap invalidating strips?). Fix the cause; no redraw-everything workaround.
- Test red before, green after, at the layer where the bug lives.
- The `### match next` row: explain; fix if wrong.
- Update KNOWN_GAPS rows it resolves.

Out: new rendering features; Unicode/ASCII tiers unless the same bug shows there.

## Acceptance
- [ ] Repro recorded (input + state) in the ticket notes.
- [ ] Cause named; test red → green.
- [ ] `### match next` explained / fixed.
- [ ] pty capture under `docs/shots/37-*` after the scroll that used to lose the graph; user judges by eye.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
