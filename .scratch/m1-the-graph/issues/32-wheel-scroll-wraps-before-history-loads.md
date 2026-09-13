# m1-the-graph/32 — Bug: trackpad scroll wraps to the top before history has loaded

Status: ready
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
- [ ] A test that fails before the fix: wheel-scroll repeatedly while the walk has not finished → never returns to the top; reaches the oldest commit.
- [ ] Same check for PgDn during loading.
- [ ] pty reproduction before/after (capture under `docs/shots/32-*`), report says what was verified.
- [ ] **By eye (user):** trackpad in Ghostty right after launch stops at the oldest commit. Unticked until the user looks.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.
