# Scroll smoothness (wheel, trackpad, held key) — prior art

Builds on [`big-repo-scroll.md`](big-repo-scroll.md) (2026-10-01: loading/paging, commit-graph, drain-then-draw). Not repeated here.

Need (field notes 2026-10-02): 17,694 commits, 1,323 refs, Ghostty full screen, Kitty graphics tier. Lag on held `j` **and** wheel/trackpad. "VS Code never lagged, neither did Sublime Merge."

Rule: facts carry a link. **Guess** / **Question** = not confirmed. Researched 2026-10-02.

## Summary

- The smooth GUIs separate three things: **data** (paged), **layout** (row positions known up front), **paint** (only visible rows, at most once per display frame). VS Code's list widget is the clearest public example.
- **No terminal tool found coalesces wheel events.** lazygit and zellij handle each wheel event on its own; they scroll N rows per event instead. Coalescing would be ours (**invent**, from the ratatui frame-rate recipe already cited in big-repo-scroll).
- Ghostty already scales events: trackpad (precision) ×1, wheel (discrete) ×3. So a trackpad sends many 1-step events.
- Kitty graphics: moving a placement by re-sending the same image id + placement id is flicker-free and needs no retransmission. Unicode placeholders make images move with text for free.
- Synchronized output (DEC mode 2026) makes a frame paint atomically; Ghostty and Kitty support it.

## 1. GUIs: VS Code and Sublime Merge

### VS Code list widget (used by its trees and lists)
- `ListView` renders only the visible range: `rangeMap` holds every item's position; on scroll, `render()` computes the range from `renderTop` + `renderHeight` and inserts/removes only the changed rows. DOM rows are pooled in `rowCache` and reused — [listView.ts](https://github.com/microsoft/vscode/blob/main/src/vs/base/browser/ui/list/listView.ts).
- Scroll is applied via a `Scrollable` scheduled with `scheduleAtNextAnimationFrame` — so many scroll events in one frame give one render. Same file (summary via fetch; exact lines not quoted — **Question:** confirm line numbers before citing in a spec).
- **Question:** whether the SCM Graph view uses this `ListView` (via the workbench tree) — likely, not read.

### VS Code Source Control Graph
- Paged: `scm.graph.pageSize` sets commits loaded initially and per "load more" — [VS Code docs: source control history](https://code.visualstudio.com/docs/sourcecontrol/history).
- **Complaint:** "the source control panel becomes unresponsive" with thousands of commits — [vscode#227475](https://github.com/microsoft/vscode/issues/227475). Fix not visible on the page fetched. **Question:** what fixed it.

### Git Graph / GitLens
- Git Graph pages ("Initial Load", "Load More Automatically") — see big-repo-scroll. Whether its webview virtualises rows: **Question**, not read.
- GitLens Commit Graph slowness complaint — see big-repo-scroll (#4585).

### Sublime Merge
- Closed source. Nothing new beyond big-repo-scroll (own Git reader, Sublime Text UI engine). Virtualisation / frame throttling: **Question**, nothing public.

## 2. TUIs and wheel/trackpad bursts

- **lazygit:** `HandleScrollUp/Down` on `MouseWheelUp/Down` scrolls by `gui.scrollHeight` (default 2) per event; no batching seen — [list_controller.go](https://github.com/jesseduffield/lazygit/blob/master/pkg/gui/controllers/list_controller.go), [Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md).
- **zellij:** each mouse event dispatched immediately, no accumulation or throttling — [input_handler.rs](https://github.com/zellij-org/zellij/blob/main/zellij-client/src/input_handler.rs).
- **Ghostty:** `mouse-scroll-multiplier` with `precision:` / `discrete:` prefixes; defaults precision 1, discrete 3 — [Ghostty config reference](https://ghostty.org/docs/config/reference).
- **crossterm:** `poll(Duration::ZERO)` "must return immediately"; `Ok(true)` guarantees the next `read` won't block — [crossterm event.rs](https://github.com/crossterm-rs/crossterm/blob/master/src/event.rs). Whether it checks the internal buffer first: docs don't say. **Question** (still open from big-repo-scroll; read `src/event/read.rs`).
- gitui, tig, helix, bottom/btop wheel handling: **not read this pass.** **Question.** (helix `scroll_lines` option: fetch did not surface it.)

## 3. Terminal graphics while scrolling

From the [Kitty graphics protocol spec](https://sw.kovidgoyal.net/kitty/graphics-protocol/):
- Same image id + placement id sent again "will replace the first. This can be used to resize or move placements around the screen, without flicker." Image data stays; no retransmit.
- `d=p` (lower case) deletes placements, keeps image data. Upper case also frees data once unreferenced.
- **Unicode placeholders** (`U+10EEEE`, virtual placement `U=1`, kitty ≥0.28): image cells are text, so images "move around as needed when they redraw their screens". Yazi uses them on Kitty and Ghostty — [superfile#1665](https://github.com/yorukot/superfile/pull/1665) (note: WezTerm accepts graphics but does not render placeholders).
- Local clients should use file / shared-memory transmission rather than inline base64. Same spec.
- Image-heavy TUIs' complaints are about **slow first display**, not scroll: yazi + kitty + tmux 15–20 s — [yazi#3102](https://github.com/sxyazi/yazi/discussions/3102); yazi `image_delay` — [yazi#2363](https://github.com/sxyazi/yazi/issues/2363).
- **Synchronized output** (`CSI ?2026h` … `l`): terminal holds paint until the frame ends; query with `CSI ?2026$p` — [spec gist](https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036). Supported in Kitty, Ghostty, WezTerm, iTerm2, Alacritty — [otty terminal comparison](https://docs.otty.sh/vt/terminal-comparison) (secondary). VS Code's terminal: requested — [vscode#284592](https://github.com/microsoft/vscode/issues/284592). **Question:** does Graft already wrap frames in 2026?
- timg, chafa, termpdf: not read. **Question.**

## 4. Praise / complaints

- VS Code SCM graph unresponsive at thousands of commits — [#227475](https://github.com/microsoft/vscode/issues/227475).
- Sublime Merge praise / tree slowness — see big-repo-scroll.
- No scroll-smoothness complaints found for lazygit/zellij specifically this pass. **Question.**

## Take / reject / invent candidates

- **Take** (VS Code ListView): row positions precomputed once; paint only the visible window; reuse row buffers/strips across frames.
- **Take** (VS Code `scheduleAtNextAnimationFrame`): scroll events only move an offset; one paint per frame.
- **Invent** (none of lazygit/zellij do it): sum all wheel deltas pending in one drain into one offset change. Keep N-rows-per-event (lazygit) as the step; Ghostty already multiplies discrete wheels ×3.
- **Take** (Kitty spec): move graphics by re-placing same id/placement id; never retransmit on scroll; `d=p` not data delete. Consider Unicode placeholders so images scroll as text (**Question**: fits Graft's strip design?).
- **Take**: DEC 2026 synchronized output around each frame (Ghostty/Kitty support).
- **Reject**: per-event immediate redraw (zellij-style) for the history pane.
