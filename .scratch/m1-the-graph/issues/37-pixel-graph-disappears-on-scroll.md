# m1-the-graph/37 — Pixel graph disappears after scrolling (git/git)

Status: done
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
- [x] Repro recorded (input + state) in the ticket notes.
- [x] Cause named; test red → green.
- [x] `### match next` explained / fixed.
- [x] pty capture under `docs/shots/37-*` after the scroll that used to lose the graph; user judges by eye.
- [x] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Done (2026-09-13, builder-opus)

**Repro (real release binary, pty, git/git, forced pixel, 200x45):** `docs/shots/37-pty-drive.py` sends keys and records the output; `docs/shots/37-pty-frames.py` replays the Kitty placements frame by frame (`?2026l`) and reports frames where body rows had no image. **Input: the wheel (trackpad).** State: any, loading or loaded; it needs only the viewport to scroll away from the cursor. Before the fix, wheel down 120 notches then 6 s idle: graph blank from the moment the cursor's row left the window on hand, **never recovered** until `j`/`k` moved the cursor. `j`, `k`, PageDown: no lasting blank. Report: `docs/shots/37-wheel-scroll-placements.txt`.

**Cause:** 30b's rule in `view::strips::pixel_rows`: "every strip waits while the cursor's row has not arrived", on the premise "the cursor is always on screen". The wheel moves the viewport and leaves the selection alone (`update.rs` `WHEEL_LINES`, docs/design/02-keymap.md §2.15), so the cursor goes off screen, its row leaves the window, `thread` is `None`, and every strip waited for a row nobody asked for. Placements were deleted (`KittyImages::frame` drops unlisted ones) and the text layer leaves the graph column empty at pixel tier.

**Fix (cause):** wait only while the cursor is in the viewport and its row is missing (`cursor_row_on_its_way`). A cursor off screen: strips draw unlit, the same as the text tiers' graph. 30b's no-double-send still holds for page keys. Cost to user: none new; after wheel-scrolling back to the cursor, its strips are re-rasterised lit (legitimate, one per strip).
Test: `view::strips::tests::strips_draw_when_the_wheel_has_left_the_cursor_row_behind`, red `left: []`, green after.
After the fix, same runs: no blank frames except startup (before the first answer) and one single frame (the 30b page-key frame, KNOWN_GAPS row kept).

**`### match next`:** not a bug. It is a real commit subject in git/git (`32178142bd62`, one of 3 such commits on `seen`, Junio's integration placeholders). The shipped binary draws what the repo holds.

**Not verified:** by eye in Ghostty — the user judges. Try: `cargo run --release -p graft -- ~/.cache/graft/git-git.git`, trackpad-scroll away from the selection, stop. The capture is a placement timeline, not pixels (a pty can't hold images). KNOWN_GAPS: no row resolved (the 30b one-frame blank and 43 re-rasterises are different causes).

## NOTES
No compaction.
