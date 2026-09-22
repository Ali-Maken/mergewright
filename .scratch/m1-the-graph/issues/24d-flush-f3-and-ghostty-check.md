# m1-the-graph/24d — Cache flush, F3 from pixels, real-Ghostty check

Status: done — by-eye check pending (user)
Build with: **Opus** (high) — terminal graphics protocol
Blocked by: 24c
Leaves runnable: yes — pixel graph survives resize/theme changes; F3 recovers; user has a command to judge by eye.

Split from 24. 24's "Handoff" section (bullet **24d**) is the spec. Closes 24's remaining acceptance boxes.

## Read first
- Ticket 24, "Acceptance" and "Handoff" sections
- ADR-0012, including its Correction
- CLAUDE.md, "Visual results are judged by eye"
- `crates/graft-term/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft/MAP.md`

## Scope
In:
- `StripCache::flush` from `Session::apply` (CellSize/Theme) and resize; resize also `forget_uploads`.
- F3 from Pixel drops to Unicode (today always Ascii, `update.rs` `is_tier_drop`); `update` needs the settled tier in `AppState`.
- `graph.renderer = pixel` with no protocol ack → attempt Kitty; F3 recovers.
- Automated real-Kitty-protocol check: transmit + place at `q=1`, DA1 fence, count APC errors, write a report file. A pty capture is not acceptance.
- Raw escape capture + decoded summary under `docs/shots/`; exact Ghostty command for the user written into this ticket.

Out:
- iTerm2 and resize frame cost (25). tmux/SSH (29).

## Acceptance
- [x] Theme / cell-size / resize flush the cache (tests); resize retransmits.
- [x] Forcing `pixel` on a terminal that never acked attempts it, and F3 recovers to Unicode.
- [x] Automated real-terminal run reads back `q=1` replies with zero errors — or, if no Kitty-protocol terminal is reachable from the builder, say so plainly and leave the box unticked.
- [x] Capture + decoded summary committed; Ghostty command in this ticket.
- [ ] **By eye (user):** the pixel graph looks right in Ghostty. Unticked until the user looks.
- [x] When all boxes are met, mark 24 done too (all but by-eye).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. Scouts for source. If you compact, say so; stop at a clean seam with a handoff note.

## Done (2026-09-13, builder-opus, no compaction)

Built:
- `AppState::settled_tier` (set in `run` from `Session::settle`) + `AppState::tier()`. `History::started` pads to `STRIP_ROWS` at Pixel. F3 (`drop_tier`): Pixel→Unicode, else→Ascii; clears `strip_rows` and re-asks rows if needed.
- `draws_pixels`: Kitty ack, or no protocol acked at all (forced `pixel` attempts Kitty). iTerm2/Sixel still not drawn (25).
- `Graphics::flush(Flush)`: strip cache flush; `Resize` also `forget_uploads` (ratatui's resize clear frees Ghostty's image data, ADR-0012). Called from `frames` on a changed `TerminalChange` (CellSize/Theme) and before `update` on `Event::Resize`.
- `placed_columns`: a strip is placed across its own lane columns, clamped to the graph column — no sideways stretch.
- `crates/graft/examples/kitty_check.rs`: real-terminal protocol check (below).

Verified:
- Tests: resize → 1 flush, next frame retransmits; CellSize + Theme change → 2 flushes, unchanged theme → none; forced pixel with `pixel_protocol: None` transmits, F3 → `a=d,d=i`, "forced to unicode", no gfx; update: Started at Pixel pads to 32, F3 Pixel→Unicode→Ascii and `strip_rows` None; `placed_columns`. Renderer-Pixel snapshot re-accepted (blank column + `gfx ok`, intended).
- **Real Ghostty 1.3.1**, launched by the builder (`open -na Ghostty.app --args -e …/kitty_check`): first frame (2 transmits), scroll crop, clear+retransmit, hide, free_all — every step DA1-fenced, **0 error replies**. Control step (place missing id 999999) heard exactly 1 `ENOENT`, so the fence does hear errors. Report `docs/shots/24d-kitty-check-report.txt`; raw bytes `docs/shots/24d-kitty-check.esc`; decoded `docs/shots/24d-kitty-check.esc.txt`.
- Not verified: the *app's* history strips in a real terminal (the check uses synthetic strips through the same encoder); how anything looks. Resize frame cost.

## Ghostty command (user, by eye)

```
cd <home____________>/dev/sandbox/gitter && cargo build -p graft && open -na /Applications/Ghostty.app --args -e <home____________>/dev/sandbox/gitter/target/debug/graft <home____________>/dev/sandbox/gitter
```

Look for: header `gfx ok`; lanes drawn as pixels in the graph column, lined up with the rows, not stretched; `j`/`k`/`PgDn` scroll keeps them aligned across strip seams; resize the window → graph comes back; `F3` → Unicode graph + footer `renderer forced to unicode (F3)`, no leftover images; `F2` and back; `q` leaves no images behind.

Re-run the protocol check: `cargo build -p graft --example kitty_check && open -na /Applications/Ghostty.app --args -e $PWD/target/debug/examples/kitty_check /tmp/report.txt /tmp/capture.esc` then `cat /tmp/report.txt`.

