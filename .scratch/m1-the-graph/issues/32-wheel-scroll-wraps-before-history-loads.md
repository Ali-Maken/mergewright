# m1-the-graph/32 — Bug: trackpad scroll wraps to the top before history has loaded

Status: done
Build with: **Opus** (high) — history window / streaming walk interaction
Blocked by: none
Leaves runnable: yes — wheel scroll stops at the end instead of wrapping.

Found by the user in Ghostty (2026-09-13), debug build at 1508cdd, repo = this repo (186 commits).

## Repro (user's words, condensed)
1. Launch: `cargo build -p graft && open -na /Applications/Ghostty.app --args -e .../target/debug/graft /Users/devuser/dev/sandbox/gitter`
2. **Immediately** scroll down with the trackpad and keep going.
3. The view scrolls to a commit from "1d ago" (not the oldest: the oldest commits are author `RE`), then **jumps back to the top** and keeps going — endless loop.

Contrast:
- Launch, press `j` repeatedly → reaches the true oldest commit and stops. Correct.
- After that, trackpad scroll → stops at the end, no loop.

So the wrap happens only while the history window has not reached the end of history. Suspect: wheel scroll (17) computes against the partially loaded window / row count, or the window re-request resets the offset. **Find the cause; don't clamp over it.**

## Read first
- Ticket 17 (mouse and tooltip), "Scope" section; ticket 09 and 18 notes on the history window
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`
- Scouts for source.

## Scope
In:
- Root cause of the wrap, and the fix. Wheel scroll past loaded rows must request more (like `j` does) and stop at the true end; never wrap.
- Check PgDn / G / search jumps for the same fault while loading.

Out:
- Anything else about the mouse or tooltip.

## Acceptance
- [x] A test that fails before the fix: wheel-scroll repeatedly while the walk has not finished → never returns to the top; reaches the oldest commit.
- [x] Same check for PgDn during loading.
- [x] pty reproduction before/after (capture under `docs/shots/32-*`), report says what was verified.
- [x] **By eye (user, 2026-09-13: "trackpad stops at the end now"):** trackpad in Ghostty right after launch stops at the oldest commit. Unticked until the user looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.

## Notes (builder, 2026-09-13)
- Cause: `History::apply` ran `top = top.min(cursor)` on every window with `done` set. The wheel moves only `top` and leaves the cursor on row 0, so once the walk had finished, every answer snapped the view back to row 0: the wrap. After `j` to the end the cursor is on the last row, so the clamp did nothing, which explains the contrast. It isn't about unloaded rows: wheel requests already ask past the frontier.
- Fix: `keep_within_rows` clamps `top` to the last row on its own, and follows only a cursor that had to move. `restore` (Esc on a running search) had the same `top.min(cursor)` and now uses it too.
- Tests: `history::tests::the_wheel_from_launch_...` failed before the fix ("went back from row 189 to 0"). `page_down_from_launch_...` passes before and after; PgDn keeps `top <= cursor`. G and search jumps set both cursor and top, so they don't have this fault.
- pty: `.scratch/m1-the-graph/pty-wheel-check.py` sends 100 wheel notches from the first frame. `docs/shots/32-wheel-before.txt` wraps at notch 26; `32-wheel-after.txt` never wraps and stops on the root commit.
- No compaction.
