# m1-the-graph/24 — Pixels on Kitty-protocol terminals

Status: partial — encoder done; remainder split into 24b, 24c, 24d
Build with: **Opus** (high) — terminal graphics protocol
Blocked by: 19, 23
Leaves runnable: yes — Ghostty and other Kitty-protocol terminals show the pixel graph; everything else unchanged.

Why: the user's daily pixel terminal is Ghostty. The spike proved it works — and proved how easily it silently doesn't.

## Read first
- ADR-0012, including its Correction
- DESIGN.md §4.2
- CLAUDE.md, the status-field rule
- Code: the Kitty code in `prototypes/graph-spike/` (reference only), 20's reply-waiter API, 23's cache API, 19's renderer override

## Scope
In:
- Transmit strips once with ids; place with a source crop; replace one placement id per frame (`p=1`); free everything on exit.
- Never `ESC[2J` between transmit and placement. Never `q=2`: use `q=1`, route replies through the mux.
- Status field named by state: `gfx ok` / `gfx: <error>`.
- A placement that returns `ENOENT` re-transmits that strip rather than trusting uploads survive.
- Place by cell rectangle, not by pixel-exact cell-size arithmetic.
- `graph.renderer = pixel` added to 19's override.

Out:
- iTerm2 and resize frame cost (25). tmux and SSH (29).

## Acceptance
- [ ] Byte-stream unit tests: chunking, ids, crop per scroll step, placement replacement, free on exit.
- [ ] A simulated `ENOENT` reply causes a re-transmit and a visible `gfx:` status.
- [ ] An automated run against a real Kitty-protocol terminal reads back `q=1` replies with zero errors. A pty capture is not acceptance.
- [ ] Forcing `pixel` on a terminal that never acked attempts it, and 19's tier-drop key recovers.
- [ ] Exact command prepared for the user in Ghostty. **The by-eye box stays unticked until the user looks.**

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Read only what "Read first" names. If you compact, say so in your report; if a second compaction is coming, stop at a clean seam with a handoff note.

## Handoff (2026-09-13, builder-opus, no compaction)

Done (committed): `graft-term::kitty` — pure encoder + reply parser, 18 byte-stream tests.
- Transmit once per strip id (ids from 1000), base64 chunks ≤4096, straight alpha (RasterFrame is premultiplied; f=32 is not).
- One `p=1` placement per image, replaced each frame; crop `y = rows.start × cell_h`; cell rect `c`/`r`; cursor saved (`ESC 7/8`), `C=1`.
- Strip scrolled out → `a=d,d=i,…,p=1` (data kept). Changed pixels for a slot → `d=I` old id, new id. LRU past capacity frees unplaced only.
- `q=1` everywhere; no `ESC[2J`, no `q=2` (asserted). `free_all` on exit (`d=I` per id, not `d=A`).
- `replies()` + `KittyImages::answer`: `ENOENT` forgets the upload → next frame retransmits. `forget_uploads()` for clear/resize.

Not done — suggested split:
- **24b rows for aligned strips.** `StripCache::get` needs whole 32-row aligned strips; `History.window` holds only what the walk sent (~viewport). Pixel tier must request rows padded to strip boundaries (plus the uncommitted virtual row, fold marker, thread emphasis, theme palette → `Strip`). Decide: request padding in `History::show`, or rasterise short strips with an offset key.
- **24c draw + replies.** `graft-ui`: blank graph column at Pixel tier (keep width); export the graph rect (`GraphScreen::body`: `x = area.x`, `width = min(wanted, area.width/2)`). `event_loop::draw`: after `terminal.draw`, inside `hold_output()` and before END_SYNCHRONIZED_UPDATE, `KittyImages::frame`. Open a `ReplyWaiter`, drain non-blocking each wake, `answer` each, status `gfx ok` / `gfx: <msg>` (sanitize_line). `free_all` on exit, `hide` when F3 leaves pixels.
- **24d flush + F3 + verification.** `StripCache::flush` from `Session::apply` (CellSize/Theme) and resize; resize also `forget_uploads`. F3 today always drops to Ascii (`update.rs` `is_tier_drop`); from Pixel it should drop to Unicode — `update` needs the settled tier in `AppState`. `graph.renderer = pixel` with no protocol → attempt Kitty. Automated real-Ghostty check (example writing transmit+place at q=1, DA1 fence, count APC errors, report file); raw escape capture + decoded summary under `docs/shots/`; Ghostty command for the user.

Gotchas: redetect worker's `ReplyWaiter` also receives graphics replies — confirm it ignores them. Crop math uses `frame.height / frame.rows` (strip pixel space only), never cell pixel size for placement.
