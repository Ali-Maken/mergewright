# m1-the-graph/30h — Idle prefetch of the strips above and below the screen

Status: ready
Build with: **Opus** (high) — event loop, wakes, frame budget (option B from 30g, mandated by 30f seam 3)
Blocked by: none
Leaves runnable: yes — pixel page keys find their strips already rasterised; nothing the user sees changes.

Why: 30f seam 3 (commit 3a0263d). Counted release runs 1–2 passed (max 15.3, 15.7 ms); run 3 missed: Pixel 100k key 226 at 16.8 ms (6.5 ms queued + 10.3 ms own frame, 2 strips drawn inside the key's frame). Per 30g's scope, a miss after option A means option B. Outputs: `target/30f/counted-{1,2,3}.txt`.

## Read first
- Ticket 30f "Seam 3" handoff (exact places: the `Wakes` trait, frames.rs's test wake source, `pixel_rows` for off-screen strips; rows above/below are already fetched, History needs no change)
- Ticket 30g scope, option B paragraph; ADR-0021
- `crates/graft/MAP.md`, `crates/graft-graph/MAP.md`; scouts for source

## Scope
In:
- After a frame, only when no input is waiting: rasterise the strip above and the strip below the viewport on the loop thread, checking for input between strips. Keys still rasterise on the key path when prefetch hasn't reached them.
- Tests: a key arriving mid-prefetch waits for at most one strip; prefetch never runs while input is queued; the prefetched strips are the ones a PgUp/PgDn then uses (cache hit).
- Note key 226's 6.5 ms queue: say whether prefetch work could ever be what a key queues behind, and prove it can't exceed one strip.
- MAP.md for any public change. Gates.
- Report: cause or symptom, and what it costs the user (idle CPU after each scroll stop, in ms and strips).

Out: the counted release runs, proofs, docs (30f seam 4).

## Acceptance
- [ ] Tests above, red before, green after.
- [ ] One uncounted release frames run recorded (target/30h/).
- [ ] Idle cost measured and stated.
- [ ] Gates pass (fmt, clippy default + heavy-tests, test).

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.
