# m1-the-graph/12c — Pane separation at 16 colours

Status: done — `Theme::color_depth()` added (stored in `Theme::new`); `graph_screen::draw_row` skips the raised-background step at `Palette16` and draws an accent left-edge rule (`▏`) in the marker slot instead. Truecolor/256 unchanged (tests + capture confirm). Capture: `docs/shots/12c-palette16-chrome.txt` (raw ANSI, forced 16 colours) — judge by eye. KNOWN_GAPS.md M0 row closed. MAP.md updated. Gates green.
Build with: **Sonnet** (medium) — a theme accessor and a separator rule
Blocked by: 12b
Leaves runnable: yes — at 16 colours the panes are visibly separated.

Split from 12 after it compacted. Closes KNOWN_GAPS.md M0 row "Dark chrome flattens at 16 colours".

## Read first
- Ticket 12: Scope (the Palette16 bullet), Notes steps **5–6** only
- docs/design/04-terminal-capability-detection.md §4.6
- KNOWN_GAPS.md M0 row "Dark chrome flattens at 16 colours"
- `crates/graft-ui/MAP.md` before source; then `theme/mod.rs` and `graph_screen/mod.rs` `draw_row`

## Scope
In:
- `Theme::color_depth() -> ColorDepth` (stored in `Theme::new`), or thread the depth into `GraphScreen::new`: pick one, say why.
- At `ColorDepth::Palette16`, pane separation and the highlighted row come from a rule or box-drawing edge, not a background step. Truecolor/256 unchanged.
Out:
- Lane identity at 16 colours (13). The uncommitted row (12b).

## Acceptance
- [ ] Snapshot at 16 colours shows panes visibly separated and the highlighted row marked; truecolor/256 snapshots unchanged.
- [ ] KNOWN_GAPS.md M0 row closed.
- [ ] MAP.md updated.
- [ ] pty capture under `docs/shots/12c-palette16-chrome.txt` forced to 16 colours, with ANSI kept (`cat` it); report says the user judges it by eye.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so.
