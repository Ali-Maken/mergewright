# ADR-0054: The loop never writes ahead of the terminal

Status: accepted (user, 2026-10-02: the M8 note's Proposed change 4, "§4.1 gains the per-frame DSR fence").
Amends DESIGN.md §4.1. Design note: `docs/design-notes/m8-polish-and-ship.md`, Hard-to-reverse choice 1.

## Context

His work repo (17 694 commits, 1 323 refs), Ghostty full screen: "i stopped pressing and it kept going" (field notes
2026-10-01); after M7 41–48, "still a bit laggy" (2026-10-02). M7/43 made a turn's wakes share one frame, so keys no
longer queue in the loop. They can still queue **in the pipe**: the loop writes a frame as soon as it has one, and a
terminal that draws slower than the loop writes (Kitty without deflate, SSH, a Pixel frame of several MB) holds a
backlog that keeps playing after the key is released. No studied TUI paces its output (`docs/research/scroll-smoothness.md`,
`big-repo-scroll.md`); VS Code paces on the display frame, which a TUI cannot see.

## Decision

1. **At most one unanswered frame in the pipe.** Every frame ends with DSR `CSI 5 n`. The next frame is not written
   until the terminal answers `CSI 0 n` or the fence times out. A terminal answers in order, so the answer means it has
   read every byte before it.
2. **Waiting is not idling.** While a frame is unanswered, wakes are still applied to state; nothing is drawn. When
   the answer comes, one frame shows the latest state. Held `j` on a slow terminal becomes fewer, newer frames.
3. **Every frame is atomic.** Text and image commands of one frame sit inside one DEC 2026 bracket
   (`CSI ? 2026 h` … `CSI ? 2026 l`), emitted unconditionally as §4.1 already says.
4. **Fallback.** The startup probe adds `CSI 5 n` to its batch; a terminal that does not answer it never fences. A
   fence that times out (500 ms, the SSH probe budget) draws anyway; three timeouts in a row stop fencing for the run,
   and frames are then capped at one per 16 ms.
5. **Wheel events in one turn sum into one scroll.** The per-event step is unchanged until he has tried the
   prototype's scene 1.

## Why not the alternatives

- **Status quo (write when ready):** the backlog is bounded only by the pipe's size; that is the lag he reported.
- **Fixed 60 fps cap:** bounds the write rate, not the terminal's read rate. A terminal that takes 40 ms per Pixel
  frame still falls behind.
- **Measure throughput and guess:** a guess per terminal, wrong whenever load changes (battery, Low Power Mode, SSH).

## Consequences

- `Wake` gains `Fence`; the input decoder must hand `CSI 0 n` to the reply path, never to keys.
- Every later frame-timing test runs against a sink that answers the fence when it has drained the bytes before it.
- Over SSH, frame rate becomes 1/RTT. That is the honest rate; anything faster was backlog.
- A terminal multiplexer (tmux) answers DSR itself, so behind tmux the fence measures tmux, not the outer terminal.
  Accepted: tmux already paces its own output.
